#!/usr/bin/env bun
/**
 * U2 群 A (workflow authority の読取サブコマンド) の実行出力ゴールデンの採取。
 *
 * **固定ピン `a277af21` の配布実バイトを走らせる**。vendor の作業ツリーは fork 側の後続
 * コミット (`801c5700`) を指しているので、そこを直に実行すると「ピンから採った」と言えない
 * (来歴に定数を書くだけでは、走らせた実体がピンである保証にならない)。ここでは
 * `git archive a277af21 dist/claude/.claude` で**ピンの木を一時ディレクトリへ展開**し、その
 * ツールを起動する。展開できなければ**失敗させる** — 作業ツリーへ暗黙に後退しない。
 * 展開物は採取のたびに捨てる (群 B/C の `capture-codekb-authority.ts` と同じ方式)。
 *
 * 入力は固定フィクスチャ (`tests/golden/workflow-authority/fixture/.claude`) で、環境シーム
 * `AIDLC_STAGE_GRAPH` / `AIDLC_SCOPE_GRID` / `AIDLC_SCOPES_DIR` で注入する — Rust の CLI ゴールデン
 * テストは同じフィクスチャを `.claude` として読むので、両者は同一入力に対する upstream との
 * バイト一致を検証する。
 *
 * ゴールデンは `tests/golden/workflow-authority/cli/<verb>/<case>/` の下に
 * `stdout` (生バイト) と `meta.json` ({argv, exit_code}) として書く。**採取結果は手修正しない**。
 *
 *   bun scripts/goldens/capture-workflow-authority.ts
 */

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const REPO_ROOT = join(dirname(new URL(import.meta.url).pathname), "..", "..");
const VENDOR = join(REPO_ROOT, "vendor/aidlc-workflows");
const FIXTURE = join(REPO_ROOT, "tests/golden/workflow-authority/fixture/.claude");
const OUT = join(REPO_ROOT, "tests/golden/workflow-authority/cli");

const UPSTREAM_COMMIT = "a277af218f0df7f325d3b8be7b6d90fce2c5bd40";

type Case = { id: string; tool: string; argv: string[] };

const CASES: Case[] = [
  { id: "lookup/phase-of-slug", tool: "aidlc-state.ts", argv: ["lookup", "phase-of", "intent-capture"] },
  { id: "lookup/phase-of-number", tool: "aidlc-state.ts", argv: ["lookup", "phase-of", "4.2"] },
  { id: "lookup/phase-of-unknown", tool: "aidlc-state.ts", argv: ["lookup", "phase-of", "no-such-stage"] },
  { id: "lookup/agent-for-agent", tool: "aidlc-state.ts", argv: ["lookup", "agent-for", "domain-design"] },
  { id: "lookup/agent-for-orchestrator", tool: "aidlc-state.ts", argv: ["lookup", "agent-for", "state-init"] },
  { id: "lookup/agent-for-unknown", tool: "aidlc-state.ts", argv: ["lookup", "agent-for", "no-such-stage"] },
  { id: "lookup/validate-stage-valid", tool: "aidlc-state.ts", argv: ["lookup", "validate-stage", "domain-design"] },
  { id: "lookup/validate-stage-number", tool: "aidlc-state.ts", argv: ["lookup", "validate-stage", "2.6"] },
  { id: "lookup/validate-stage-invalid", tool: "aidlc-state.ts", argv: ["lookup", "validate-stage", "bogus"] },
  { id: "lookup/next-stage-execute", tool: "aidlc-state.ts", argv: ["lookup", "next-stage", "state-init", "alpha"] },
  { id: "lookup/next-stage-beta", tool: "aidlc-state.ts", argv: ["lookup", "next-stage", "state-init", "beta"] },
  { id: "lookup/next-stage-skip-walk", tool: "aidlc-state.ts", argv: ["lookup", "next-stage", "intent-capture", "alpha"] },
  { id: "lookup/next-stage-none", tool: "aidlc-state.ts", argv: ["lookup", "next-stage", "deployment-execution", "alpha"] },
  { id: "lookup/next-stage-unknown-scope", tool: "aidlc-state.ts", argv: ["lookup", "next-stage", "state-init", "nope"] },
  { id: "scope-table/scope-table", tool: "aidlc-utility.ts", argv: ["scope-table"] },
  { id: "stage-table/stage-table", tool: "aidlc-utility.ts", argv: ["stage-table"] },
];

/** ピンの配布ツリーを一時ディレクトリへ展開し、tools ディレクトリを返す。 */
function materializePinnedDistribution(): { tools: string; root: string } {
  const root = mkdtempSync(join(tmpdir(), "aidlc-pin-"));
  const archive = spawnSync(
    "git",
    ["-C", VENDOR, "archive", UPSTREAM_COMMIT, "dist/claude/.claude"],
    { encoding: "buffer", maxBuffer: 512 * 1024 * 1024 },
  );
  if (archive.status !== 0) {
    rmSync(root, { recursive: true, force: true });
    throw new Error(
      `ピン ${UPSTREAM_COMMIT} の配布物を取り出せない: ${archive.stderr.toString()}`,
    );
  }
  const untar = spawnSync("tar", ["-x", "-C", root], { input: archive.stdout });
  if (untar.status !== 0) {
    rmSync(root, { recursive: true, force: true });
    throw new Error("配布物の展開に失敗した");
  }
  return { tools: join(root, "dist/claude/.claude/tools"), root };
}

const { tools, root: pinRoot } = materializePinnedDistribution();

const env = {
  ...process.env,
  AIDLC_STAGE_GRAPH: join(FIXTURE, "tools/data/stage-graph.json"),
  AIDLC_SCOPE_GRID: join(FIXTURE, "tools/data/scope-grid.json"),
  AIDLC_SCOPES_DIR: join(FIXTURE, "scopes"),
};

rmSync(OUT, { recursive: true, force: true });

try {
  for (const c of CASES) {
    const result = spawnSync("bun", [join(tools, c.tool), ...c.argv], {
      cwd: REPO_ROOT,
      env,
      encoding: "buffer",
    });
    if (result.error) throw result.error;
    const dir = join(OUT, c.id);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "stdout"), result.stdout);
    writeFileSync(
      join(dir, "meta.json"),
      JSON.stringify(
        {
          upstream_commit: UPSTREAM_COMMIT,
          tool: c.tool,
          argv: c.argv,
          exit_code: result.status,
        },
        null,
        2,
      ) + "\n",
    );
    process.stdout.write(`  ${c.id}  exit=${result.status}  ${result.stdout.length}B\n`);
  }
} finally {
  rmSync(pinRoot, { recursive: true, force: true });
}

process.stdout.write(`採取 ${CASES.length} ケース -> ${OUT}\n`);
