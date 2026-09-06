#!/usr/bin/env bun
/** 固定ピンの未変更コードへ、明示した合成入力を与えて補完観測を採る。既存コーパスは書き換えない。 */
import { createHash } from "node:crypto";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";
import assert from "node:assert/strict";

const PIN = "3c3146cfd7cef33020d48e8d48d4e80d0f8c2820";
const MANIFEST = "ea223c423bebf32cd240d45b645fcd9649efc0d19592de75fd48565a6ded0b9f";
const digest = (text: string | Uint8Array) => createHash("sha256").update(text).digest("hex");

function files(root: string): string[] {
  return readdirSync(root, { withFileTypes: true }).flatMap(entry => {
    const path = join(root, entry.name);
    assert(!entry.isSymbolicLink(), `配布物のリンクは受け付けない: ${path}`);
    return entry.isDirectory() ? files(path) : [path];
  });
}

export function capture(dist: string) {
  const paths = files(dist).sort((a, b) => Buffer.compare(Buffer.from(relative(dist, a)), Buffer.from(relative(dist, b))));
  assert.equal(paths.length, 262);
  const manifest = paths.map(path => `${digest(readFileSync(path))}  ${relative(dist, path).replaceAll("\\", "/")}\n`).join("");
  assert.equal(digest(manifest), MANIFEST, "固定ピンの配布物マニフェストが不一致");
  const root = mkdtempSync(join(tmpdir(), "aidlc-supplemental-"));
  try {
    cpSync(join(dist, ".claude"), join(root, ".claude"), { recursive: true });
    cpSync(join(dist, "aidlc"), join(root, "aidlc"), { recursive: true });
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith("AIDLC_") && !key.startsWith("AWS_AIDLC_")));
    Object.assign(env, { CLAUDE_PROJECT_DIR: root, AIDLC_DISABLE_USAGE_TRACKING: "1" });
    function run(file: string, args: string[] = [], input?: object) {
      const result = Bun.spawnSync([process.execPath, join(root, ".claude", file), ...args], {
        cwd: root, env, stdout: "pipe", stderr: "pipe",
        ...(input ? { stdin: Buffer.from(JSON.stringify(input)) } : {}),
      });
      return { exit: result.exitCode, stdout: result.stdout.toString(), stderr: result.stderr.toString() };
    }
    const init = run("tools/aidlc-utility.ts", ["intent-create", "--label", "golden", "--scope", "classic", "--project-dir", root]);
    assert.equal(init.exit, 0, init.stderr);
    const intents = join(root, "aidlc/spaces/default/intents");
    const records = readdirSync(intents, { withFileTypes: true }).filter(entry => entry.isDirectory());
    assert.equal(records.length, 1);
    const record = join(intents, records[0].name);
    const rulesPath = join(root, "aidlc/spaces/default/memory/project.md");
    const rules = Array.from({ length: 240 }, (_, i) => `\n## Supplemental rule ${i}\n\nALWAYS keep synthetic rule ${i}: ${"deterministic ".repeat(25)}\n`).join("");
    writeFileSync(rulesPath, readFileSync(rulesPath, "utf8") + rules);
    let result = run("tools/aidlc-orchestrate.ts", ["next", "--project-dir", root]);
    assert.equal(result.exit, 0, result.stderr);
    let directive = JSON.parse(result.stdout);
    const parts: Array<{ part: number; parts: number; content_sha256: string; synthetic_rules: number[] }> = [];
    while (directive.kind === "load-steering") {
      assert(parts.length < 100, "継続配送が完了しない");
      const text = directive.rules_content.map((entry: { text: string }) => entry.text).join("");
      const ids = [...text.matchAll(/^## Supplemental rule (\d+)$/gm)].map(match => Number(match[1]));
      parts.push({ part: directive.part, parts: directive.parts, content_sha256: digest(text), synthetic_rules: ids });
      result = run("tools/aidlc-orchestrate.ts", ["continue", directive.continue_token, "--project-dir", root]);
      assert.equal(result.exit, 0, result.stderr);
      directive = JSON.parse(result.stdout);
    }
    assert(parts.length > 1);
    assert.deepEqual(parts.map(part => part.part), parts.map((_, i) => i + 1));
    assert(parts.every(part => part.parts === parts.length));
    assert.deepEqual(parts.flatMap(part => part.synthetic_rules), Array.from({ length: 240 }, (_, i) => i));
    assert.equal(directive.kind, "run-stage");

    const missingMode = run("tools/aidlc-bolt.ts", ["set-autonomy", "--mode", "gated", "--project-dir", root]);
    assert.equal(missingMode.exit, 1);
    const state = join(record, "aidlc-state.md");
    // ツール単独生成の状態には欠けている行を、契約テンプレートに従う入力として明示する。
    writeFileSync(state, readFileSync(state, "utf8") + "\n- **Construction Autonomy Mode**: autonomous\n");
    const mode = run("tools/aidlc-bolt.ts", ["set-autonomy", "--mode", "gated", "--project-dir", root]);
    assert.equal(mode.exit, 0, mode.stderr);
    const modeOutput = JSON.parse(mode.stdout);
    assert.equal(modeOutput.mode, "gated");
    assert.equal(modeOutput.state_updated, true);

    const transcript = join(root, "synthetic-transcript.jsonl");
    const chat = [
      { type: "user", message: { role: "user", content: "現在の状況を説明してください" } },
      { type: "assistant", message: { role: "assistant", content: [{ type: "text", text: "状況を説明します" }] } },
    ];
    writeFileSync(transcript, chat.map(row => JSON.stringify(row)).join("\n") + "\n");
    const stopInput = { session_id: "supplemental", transcript_path: transcript };
    const conversational = run("hooks/aidlc-continue-workflow.ts", [], stopInput);
    assert.equal(conversational.exit, 0);
    assert.equal(conversational.stdout, "");
    const engaged = { type: "assistant", message: { role: "assistant", content: [{ type: "tool_use", name: "Bash", input: { command: "bun .claude/tools/aidlc-orchestrate.ts next" } }] } };
    writeFileSync(transcript, [...chat, engaged].map(row => JSON.stringify(row)).join("\n") + "\n");
    const blocked = run("hooks/aidlc-continue-workflow.ts", [], stopInput);
    assert.equal(blocked.exit, 0);
    assert.equal(JSON.parse(blocked.stdout).decision, "block");
    return {
      upstream_commit: PIN, tree_manifest_sha256: MANIFEST, tree_file_count: paths.length,
      fixture_kind: "synthetic-preconditions", bun_version: Bun.version,
      cases: [
        { id: "cli/continue/multi-part", setup: "project.mdへ240個の合成ルールを追加。各ルールの本文はdeterministicを25回繰り返す。", parts, final_kind: directive.kind, delivered_rules: 240 },
        { id: "cli/set-autonomy/gated", setup: "契約テンプレートのConstruction Autonomy Mode行をautonomousで明示的に追加。ツール単独生成とは区別。", tool_generated_state_exit: missingMode.exit, exit: mode.exit, output: modeOutput },
        { id: "hooks/stop-forwarding-loop/transcript-carve-out", setup: "Claude JSONL形式の合成ユーザー/応答2行。対照では同じターンへエンジン呼出を追加。", conversational: { exit: conversational.exit, stdout: conversational.stdout }, engine_engaged: { exit: blocked.exit, decision: JSON.parse(blocked.stdout).decision } },
      ],
    };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

if (import.meta.main) {
  const [dist, output] = process.argv.slice(2);
  assert(dist && output, "Usage: bun scripts/goldens/capture-supplemental.ts <verified-dist/claude> <output-dir>");
  assert(!resolve(output).includes("/upstream-3c3146cf"), "元の採取コーパスは上書きしない");
  const observations = capture(resolve(dist));
  mkdirSync(output, { recursive: true });
  writeFileSync(join(output, "cases.json"), JSON.stringify(observations, null, 2) + "\n");
  writeFileSync(join(output, "provenance.json"), JSON.stringify({ captured_at: new Date().toISOString(), command: "bun scripts/goldens/capture-supplemental.ts <verified-dist/claude> <output-dir>", ...observations, cases: observations.cases.map(entry => entry.id) }, null, 2) + "\n");
  console.log(`補完観測${observations.cases.length}件を保存`);
}
