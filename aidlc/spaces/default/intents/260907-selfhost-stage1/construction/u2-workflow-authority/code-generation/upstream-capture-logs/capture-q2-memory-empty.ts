#!/usr/bin/env bun
/**
 * 裁定 Q2 の実走行採取。
 *
 * 本家 runtime compile は `MEMORY_EMPTY` を `(slug, completed_at)` ごとに 1 件だけ記録し、
 * 再跳躍して承認し直した位置には改めて記録する（`aidlc-runtime.ts:786-804`）。
 * 本 build は承認時刻を持たず「承認へ倒れたこと」を新しい承認の印にしている。
 * 「観測等価」という主張が正しいかを、同じ履歴を両者へ流して件数と内容で確かめる。
 *
 * 変更は mkdtemp の一時 workspace だけに限定する。本リポジトリの記録・監査・状態・
 * memory・受領証は一切触らない。
 *
 * 使い方:
 *   bun capture-q2-memory-empty.ts <dist(=fixed commit tree)> <rust-binary> <out.json>
 */
import assert from "node:assert/strict";
import {
  copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync,
  readdirSync, readFileSync, rmSync, writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { UPSTREAM, verifySource } from "../../../../../../../../../scripts/goldens/upstream-source.ts";

const [dist, rustBin, destination] = process.argv.slice(2);
assert(dist && rustBin && destination, "usage: <dist> <rust-binary> <out.json>");
const sourceInfo = verifySource(dist);
assert(existsSync(rustBin), `本 build のバイナリが無い: ${rustBin}`);

/** 4 見出しだけの日誌 — 記録 0 件（`MEMORY_EMPTY` の対象）。 */
const EMPTY_JOURNAL = "# Stage Memory\n\n## Interpretations\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n";
/** 記録が 1 件ある日誌 — 対象外。 */
const FILLED_JOURNAL = "# Stage Memory\n\n## Interpretations\n\n- 2026-09-09T00:00:00Z — 解釈; 文脈\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n";

/** compile を焚く PostToolUse 封筒（本家は Bash matcher で受ける）。 */
const envelope = (command: string) =>
  JSON.stringify({
    session_id: "11111111-2222-4333-8444-555555555555",
    tool_name: "Bash",
    tool_input: { command },
    tool_response: { stdout: "" },
  });
const REPORT_COMMAND = "bun .claude/tools/aidlc-orchestrate.ts report --result completed";

/** TaskUpdate の同期 1 件（承認ではない進捗）。 */
const taskUpdate = (slug: string) =>
  JSON.stringify({
    session_id: "11111111-2222-4333-8444-555555555555",
    tool_name: "TaskUpdate",
    tool_input: { status: "in_progress", activeForm: `Running [${slug}]` },
  });

interface Harness {
  impl: "upstream" | "rust";
  root: string;
  parent: string;
  record: string;
  tool(tool: string, args: string[]): { status: number | null; stdout: string; stderr: string };
  hook(name: string, stdin: string): { status: number | null; stdout: string; stderr: string };
  audit(): string;
  dispose(): void;
}

function harness(impl: "upstream" | "rust"): Harness {
  const parent = mkdtempSync(join(tmpdir(), `aidlc-q2-${impl}-`));
  const root = join(parent, "workspace");
  mkdirSync(root);
  cpSync(join(dist, ".claude"), join(root, ".claude"), { recursive: true });
  writeFileSync(join(root, "source.rs"), "fn main() {}\n");
  const env = { PATH: process.env.PATH!, HOME: join(parent, "home") };

  let tool: Harness["tool"];
  let hook: Harness["hook"];
  if (impl === "upstream") {
    tool = (name, args) =>
      spawnSync(process.execPath, [join(root, ".claude/tools", `${name}.ts`), ...args],
        { cwd: root, env, encoding: "utf8" }) as never;
    hook = (name, stdin) =>
      spawnSync(process.execPath, [join(root, ".claude/hooks", `aidlc-${name}.ts`)],
        { cwd: root, env, encoding: "utf8", input: stdin }) as never;
  } else {
    const bin = join(parent, "bin");
    mkdirSync(bin);
    for (const name of ["aidlc", "aidlc-utility", "aidlc-log", "aidlc-bolt", "aidlc-jump", "aidlc-orchestrate", "aidlc-state"]) {
      copyFileSync(rustBin, join(bin, name));
    }
    tool = (name, args) => spawnSync(join(bin, name), args, { cwd: root, env, encoding: "utf8" }) as never;
    hook = (name, stdin) =>
      spawnSync(join(bin, "aidlc"), ["hook", name], { cwd: root, env, encoding: "utf8", input: stdin }) as never;
  }

  const created = tool("aidlc-utility",
    ["intent-create", "--scope", "bugfix", "--label", "memempty", "--arguments", "Fix the runtime graph"]);
  assert.equal(created.status, 0, `${impl} intent-create: ${created.stderr}${created.stdout}`);
  const intents = join(root, "aidlc/spaces/default/intents");
  const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());
  const auditDir = join(record, "audit");
  const audit = () => existsSync(auditDir)
    ? readdirSync(auditDir).sort().map((p) => readFileSync(join(auditDir, p), "utf8")).join("")
    : "";
  return { impl, root, parent, record, tool, hook, audit, dispose: () => rmSync(parent, { recursive: true, force: true }) };
}

const normalise = (h: Harness, text: string) =>
  text.replaceAll(h.record, "<RECORD>").replaceAll(h.root, "<ROOT>").replaceAll(h.parent, "<PARENT>")
    .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g, "<TS>")
    .replace(/sha256:[0-9a-f]{64}/g, "sha256:<HASH>")
    .replace(/\b[0-9a-f]{64}\b/g, "<HASH>")
    .replace(/\b\d{6}-memempty\b/g, "<INTENT>");

/** 監査に立っている `MEMORY_EMPTY` 行（正規化済み・全件）。 */
function memoryEmptyRows(h: Harness): string[] {
  return h.audit().split("\n---\n")
    .filter((b) => b.includes("**Event**: MEMORY_EMPTY"))
    .map((b) => normalise(h, b).trim());
}

/** そのステージの日誌を置く。 */
function journal(h: Harness, phase: string, stage: string, bodyText: string) {
  const dir = join(h.record, phase, stage);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "memory.md"), bodyText);
}

// --- 履歴 ---------------------------------------------------------------------
//
// 1 本の workspace に、承認 → compile ×2 → TaskUpdate 同期 → compile →
// 再跳躍 → 再承認 → compile を順に流し、各段の直後に `MEMORY_EMPTY` の全件を採る。
// 件数の増減そのものが答えなので、段ごとの累計と差分の両方を残す。

function drive(impl: "upstream" | "rust") {
  const h = harness(impl);
  const steps: any[] = [];
  /** その時点の runtime-graph の state-init 行（判定の前提が満たされているかの証跡）。 */
  const stateInitRow = () => {
    const graphPath = join(h.record, "runtime-graph.json");
    if (!existsSync(graphPath)) return null;
    try {
      const graph = JSON.parse(readFileSync(graphPath, "utf8"));
      const row = graph?.stages?.find((s: any) => s.stage_slug === "state-init");
      if (!row) return null;
      return {
        outcome: row.outcome,
        memory_entries: row.memory_entries,
        started_at: normalise(h, String(row.started_at)),
        completed_at: row.completed_at === null ? null : normalise(h, String(row.completed_at)),
        completed_at_raw: row.completed_at,
      };
    } catch { return null; }
  };
  const record = (id: string, note: string, r?: { status: number | null; stdout: string; stderr: string }) => {
    const rows = memoryEmptyRows(h);
    steps.push({
      step: id,
      note,
      exit: r ? r.status : null,
      stdout: r ? normalise(h, r.stdout).slice(0, 800) : null,
      stderr: r ? normalise(h, r.stderr).slice(0, 800) : null,
      memory_empty_total: rows.length,
      memory_empty_rows: rows,
      state_init_row: stateInitRow(),
    });
  };
  /** state-init を承認済みへ戻す（`next` は指示を出すだけで完了させない）。 */
  const complete = () => h.tool("aidlc-orchestrate", ["report", "--stage", "state-init", "--result", "completed"]);

  try {
    // 初期化 3 段を畳んで inception へ入る（state-init が [x] になる）。
    const advance = h.tool("aidlc-orchestrate", ["next", "bugfix"]);
    record("01-advance", "初期化を畳んで state-init を承認済みにする", advance);

    // 承認済みかつ日誌が空 → MEMORY_EMPTY の対象。進行中の位置は対象外の対照。
    journal(h, "initialization", "state-init", EMPTY_JOURNAL);
    journal(h, "inception", "reverse-engineering", EMPTY_JOURNAL);
    record("02-journals", "state-init を空日誌に、進行中の reverse-engineering も空日誌に置く");

    const c1 = h.hook("rebuild-stage-graph", envelope(REPORT_COMMAND));
    record("03-compile-1", "1 回目の compile — ここで初めて記録されるはず", c1);

    const c2 = h.hook("rebuild-stage-graph", envelope(REPORT_COMMAND));
    record("04-compile-2", "2 回目の compile — 同じ承認なので増えないはず", c2);

    // 承認以外の進捗（TaskUpdate による同期）。
    const sync = h.hook("sync-workflow-state", taskUpdate("reverse-engineering"));
    record("05-taskupdate-sync", "TaskUpdate 同期（承認ではない進捗）", sync);

    const c3 = h.hook("rebuild-stage-graph", envelope(REPORT_COMMAND));
    record("06-compile-after-sync", "同期のあとの compile — 抑止が解けていないか", c3);

    // --- 同じ秒のうちに再跳躍して承認し直す（本家の鍵は秒精度の時刻比較である） ---
    const jumpA = h.tool("aidlc-jump", ["execute", "--target", "state-init", "--direction", "redo"]);
    record("07-jump-redo-same-second", "state-init へ再跳躍（間を置かない）", jumpA);
    const reapproveA = complete();
    journal(h, "initialization", "state-init", EMPTY_JOURNAL);
    record("08-reapprove-same-second", "同じ秒のうちに承認し直す", reapproveA);
    const c4 = h.hook("rebuild-stage-graph", envelope(REPORT_COMMAND));
    record("09-compile-after-same-second-reapproval", "間を置かない再承認のあとの compile", c4);

    // --- 2 秒あけて再跳躍して承認し直す（時刻が確実に進んだ状態） ---
    Bun.sleepSync(2100);
    const jumpB = h.tool("aidlc-jump", ["execute", "--target", "state-init", "--direction", "redo"]);
    record("10-jump-redo-after-gap", "2 秒あけて state-init へ再跳躍", jumpB);
    const reapproveB = complete();
    journal(h, "initialization", "state-init", EMPTY_JOURNAL);
    record("11-reapprove-after-gap", "時刻が確実に進んだ状態で承認し直す", reapproveB);
    const c5 = h.hook("rebuild-stage-graph", envelope(REPORT_COMMAND));
    record("12-compile-after-gapped-reapproval", "本家はここで改めて記録するはず", c5);

    // 日誌に記録が入った位置は対象外になる（対照）。
    journal(h, "initialization", "state-init", FILLED_JOURNAL);
    const c6 = h.hook("rebuild-stage-graph", envelope(REPORT_COMMAND));
    record("13-compile-filled-journal", "日誌が埋まった位置は対象外", c6);

    return {
      impl, steps,
      runtime_graph_present: existsSync(join(h.record, "runtime-graph.json")),
      final_state_init_row: stateInitRow(),
      final_memory_empty_rows: memoryEmptyRows(h),
    };
  } finally {
    h.dispose();
  }
}

const upstream = drive("upstream");
const rust = drive("rust");

const stepIds = upstream.steps.map((s: any) => s.step);
const comparison = stepIds.map((id: string) => {
  const u = upstream.steps.find((s: any) => s.step === id);
  const r = rust.steps.find((s: any) => s.step === id);
  return {
    step: id,
    note: u.note,
    upstream_total: u.memory_empty_total,
    rust_total: r ? r.memory_empty_total : "採取できず",
    same: r ? u.memory_empty_total === r.memory_empty_total : null,
    upstream_exit: u.exit,
    rust_exit: r ? r.exit : null,
  };
});

mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, `${JSON.stringify({
  question: "Q2: MEMORY_EMPTY の重複抑止の鍵をどう扱うか",
  source: { ...UPSTREAM, verified_manifest: sourceInfo.manifest ?? UPSTREAM.manifest },
  rust_binary: { path: rustBin, sha256: createHash("sha256").update(readFileSync(rustBin)).digest("hex") },
  tool_versions: { bun: Bun.version },
  capture_command: [process.execPath, import.meta.path, dist, rustBin, destination],
  normalization: "一時 workspace の絶対パス・ISO 時刻・sha256・intent 名のみ",
  comparison,
  observations: { upstream, rust },
}, null, 2)}\n`);
console.log(JSON.stringify(comparison, null, 2));
