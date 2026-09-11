#!/usr/bin/env bun
/**
 * 裁定 Q2 の実走行採取（決定版）。
 *
 * ゲートのある `requirements-analysis` を「承認 → compile ×2 → 再跳躍 → 再承認 → compile」
 * と一周させ、`MEMORY_EMPTY` の件数を段ごとに採る。初期化段 `state-init` の経路
 * （`capture-q2-memory-empty.ts`）は本 build の `advance` 遷移が未配線で再承認へ戻せず、
 * 鍵そのものを切り分けられなかったため、ゲート段でやり直す。
 *
 * 本家の鍵は `(slug, completed_at)`：既存行の時刻がその承認の `completed_at` 以上なら
 * 抑止する（`aidlc-runtime.ts:786-804`）。本 build は承認時刻を持たず、「承認へ倒れたこと」で
 * 印を落とす。同じ履歴で件数が一致するかが「観測等価」の答えである。
 *
 * 変更は mkdtemp の一時 workspace だけに限定する。
 *
 *   bun capture-q2b-gated-reapproval.ts <dist> <rust-binary> <out.json>
 */
import assert from "node:assert/strict";
import {
  appendFileSync, copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync,
  readdirSync, readFileSync, realpathSync, rmSync, writeFileSync,
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

const STAGE = "requirements-analysis";
const REVIEWER = "aidlc-product-lead-agent";
const EMPTY_JOURNAL = "# Stage Memory\n\n## Interpretations\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n";
const FILLED_JOURNAL = "# Stage Memory\n\n## Interpretations\n\n- 2026-09-09T00:00:00Z — 解釈; 文脈\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n";
const QUESTIONS = "# Questions\n\n## Q1\n修正対象は何か。\nA. 小さな不具合\nX. Other (please specify)\n[Answer]: A\n\n## Consolidated Summary Confirmation\nLooks correct / Request changes\n[Answer]: Looks correct\n";
const REQUIREMENTS = "# Requirements\n\n## Functional Requirements\nFR1: 小さな不具合を修正する。\n\n## Sources\n[Q1] 承認済み回答。\n\n## Assumptions & Open Questions\nNone.\n";
const appendix = (iteration: number) =>
  `\n## Review\n\n**Reviewer:** ${REVIEWER}\n**Verdict:** READY\n**Iteration:** ${iteration}\n\n### Findings\nNone.\n`;
const ENVELOPE = JSON.stringify({
  session_id: "11111111-2222-4333-8444-555555555555",
  tool_name: "Bash",
  tool_input: { command: "bun .claude/tools/aidlc-orchestrate.ts report --result completed" },
  tool_response: { stdout: "" },
});

type Impl = "upstream" | "rust";

function harness(impl: Impl) {
  // macOS の `/var` は `/private/var` への symlink なので、実体パスへ寄せる。
  // 寄せないと本家の「記録の内側か」の判定が偽になる。
  const parent = realpathSync(mkdtempSync(join(tmpdir(), `aidlc-q2b-${impl}-`)));
  const root = join(parent, "workspace");
  mkdirSync(root);
  cpSync(join(dist, ".claude"), join(root, ".claude"), { recursive: true });
  writeFileSync(join(root, "source.rs"), "fn main() {}\n");
  const env = { PATH: process.env.PATH!, HOME: join(parent, "home") };
  const bin = join(parent, "bin");
  mkdirSync(bin);
  for (const n of ["aidlc", "aidlc-utility", "aidlc-jump", "aidlc-orchestrate", "aidlc-state", "aidlc-log"]) {
    copyFileSync(rustBin, join(bin, n));
  }
  const opts = { cwd: root, env, encoding: "utf8" as const, input: "", timeout: 30_000 };
  const tool = (name: string, args: string[]) =>
    impl === "upstream"
      ? spawnSync(process.execPath, [join(root, ".claude/tools", `${name}.ts`), ...args], opts)
      : spawnSync(join(bin, name), args, opts);
  const hook = (name: string, stdin: string) =>
    impl === "upstream"
      ? spawnSync(process.execPath, [join(root, ".claude/hooks", `aidlc-${name}.ts`)], { ...opts, input: stdin })
      : spawnSync(join(bin, "aidlc"), ["hook", name], { ...opts, input: stdin });

  const created = tool("aidlc-utility", ["intent-create", "--scope", "bugfix", "--label", "gate", "--arguments", "Fix a small bug"]);
  assert.equal(created.status, 0, `${impl} intent-create: ${created.stderr}`);
  const intents = join(root, "aidlc/spaces/default/intents");
  const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());
  return { impl, root, parent, record, tool, hook, dispose: () => rmSync(parent, { recursive: true, force: true }) };
}
type Harness = ReturnType<typeof harness>;

const normalise = (h: Harness, t: string) =>
  t.replaceAll(h.record, "<RECORD>").replaceAll(h.root, "<ROOT>").replaceAll(h.parent, "<PARENT>")
    .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g, "<TS>")
    .replace(/sha256:[0-9a-f]{64}/g, "sha256:<HASH>")
    .replace(/\b[0-9a-f]{64}\b/g, "<HASH>")
    .replace(/\b\d{6}-gate\b/g, "<INTENT>");

function drive(impl: Impl) {
  const h = harness(impl);
  const steps: any[] = [];
  const stageDir = join(h.record, "inception", STAGE);
  const auditDir = join(h.record, "audit");

  const memoryEmptyRows = () =>
    (existsSync(auditDir)
      ? readdirSync(auditDir).sort().map((p) => readFileSync(join(auditDir, p), "utf8")).join("")
      : "")
      .split("\n---\n").filter((b) => b.includes("**Event**: MEMORY_EMPTY"))
      .map((b) => normalise(h, b).trim());

  const checkbox = () => {
    try {
      const state = readFileSync(join(h.record, "aidlc-state.md"), "utf8");
      return (state.split("\n").find((l) => l.includes(`${STAGE} —`)) ?? "?").trim();
    } catch { return "?"; }
  };

  const graphRow = () => {
    const p = join(h.record, "runtime-graph.json");
    if (!existsSync(p)) return null;
    try {
      const row = JSON.parse(readFileSync(p, "utf8"))?.stages?.find((s: any) => s.stage_slug === STAGE);
      return row ? { outcome: row.outcome, memory_entries: row.memory_entries, completed_at: row.completed_at } : null;
    } catch { return null; }
  };

  const record = (id: string, note: string, r?: any) => {
    const rows = memoryEmptyRows();
    steps.push({
      step: id, note,
      exit: r ? r.status : null,
      stdout: r ? normalise(h, r.stdout ?? "").trim().slice(0, 500) : null,
      stderr: r ? normalise(h, r.stderr ?? "").trim().slice(0, 500) : null,
      checkbox: checkbox(),
      graph_row: graphRow(),
      memory_empty_total: rows.length,
      memory_empty_rows: rows,
    });
  };

  /** 1 周ぶんの承認（要約確認 → レビュー受領証 → ゲート提示 → 承認）。 */
  const approveOnce = (label: string, iteration: number) => {
    mkdirSync(stageDir, { recursive: true });
    writeFileSync(join(stageDir, "requirements.md"), REQUIREMENTS);
    writeFileSync(join(stageDir, "requirements-analysis-questions.md"), QUESTIONS);
    writeFileSync(join(stageDir, "memory.md"), EMPTY_JOURNAL);

    // 本家は「人が答えた要約確認」の監査行を要求し、確認事項ファイルへの束縛も求める。
    record(`${label}-answer`, "要約確認を記録する",
      h.tool("aidlc-log", ["answer", "--checkpoint", "summary-confirmation", "--stage", STAGE,
        "--details", "Looks correct",
        "--questions-file", join(stageDir, "requirements-analysis-questions.md")]));

    const base = ["review", "--stage", STAGE, "--reviewer", REVIEWER, "--iteration", String(iteration)];
    record(`${label}-review-request`, "レビューを起こす", h.tool("aidlc-log", base));
    appendFileSync(join(stageDir, "requirements.md"), appendix(iteration));
    record(`${label}-review-complete`, "READY の受領証を記録する", h.tool("aidlc-log", [...base, "--verdict", "READY"]));

    record(`${label}-awaiting`, "ゲートを提示する",
      h.tool("aidlc-orchestrate", ["report", "--stage", STAGE, "--result", "awaiting-approval"]));
    record(`${label}-approved`, "人の Approve を記録して承認する",
      h.tool("aidlc-orchestrate", ["report", "--stage", STAGE, "--result", "approved", "--user-input", "Approve"]));
    // 承認後も日誌は空のまま（MEMORY_EMPTY の対象）。
    writeFileSync(join(stageDir, "memory.md"), EMPTY_JOURNAL);
  };

  try {
    record("01-next", "初期化を畳む", h.tool("aidlc-orchestrate", ["next", "bugfix"]));
    record("02-jump-forward", `${STAGE} まで進める`,
      h.tool("aidlc-jump", ["execute", "--target", STAGE, "--direction", "forward"]));

    approveOnce("03-first", 1);
    record("04-compile-1", "1 回目の compile — ここで初めて記録されるはず", h.hook("rebuild-stage-graph", ENVELOPE));
    record("05-compile-2", "2 回目の compile — 同じ承認なので増えないはず", h.hook("rebuild-stage-graph", ENVELOPE));

    // 間を置かずに再跳躍して承認し直す（本家の鍵は秒精度の時刻比較である）。
    record("06-jump-redo-immediate", `${STAGE} へ再跳躍（間を置かない）`,
      h.tool("aidlc-jump", ["execute", "--target", STAGE, "--direction", "redo"]));
    approveOnce("07-second", 1);
    record("08-compile-after-immediate", "間を置かない再承認のあとの compile", h.hook("rebuild-stage-graph", ENVELOPE));

    // 秒が確実に進んだ状態で、もう一度再跳躍して承認し直す。
    Bun.sleepSync(2100);
    record("09-jump-redo-after-gap", `${STAGE} へ再跳躍（2 秒あけて）`,
      h.tool("aidlc-jump", ["execute", "--target", STAGE, "--direction", "redo"]));
    approveOnce("10-third", 1);
    record("11-compile-after-gap", "★ 時刻が進んだ再承認のあとの compile — ここが答え", h.hook("rebuild-stage-graph", ENVELOPE));

    writeFileSync(join(stageDir, "memory.md"), FILLED_JOURNAL);
    record("12-compile-filled", "日誌が埋まった位置は対象外", h.hook("rebuild-stage-graph", ENVELOPE));

    // 鍵の検証材料として、正規化していない生の時刻を残す。
    const rawRows = (existsSync(auditDir)
      ? readdirSync(auditDir).sort().map((p) => readFileSync(join(auditDir, p), "utf8")).join("")
      : "").split("\n---\n").filter((b) => b.includes("**Event**: MEMORY_EMPTY"))
      .map((b) => (b.match(/\*\*Timestamp\*\*:\s*(\S+)/) ?? [])[1] ?? "?");
    const graphPath = join(h.record, "runtime-graph.json");
    const rawCompletedAt = existsSync(graphPath)
      ? (JSON.parse(readFileSync(graphPath, "utf8"))?.stages ?? [])
          .filter((s: any) => s.stage_slug === STAGE).map((s: any) => s.completed_at)
      : [];

    return { impl, steps, final_rows: memoryEmptyRows(),
             raw_memory_empty_timestamps: rawRows, raw_final_completed_at: rawCompletedAt };
  } finally {
    h.dispose();
  }
}

const upstream = drive("upstream");
const rust = drive("rust");

const comparison = upstream.steps.map((u: any) => {
  const r = rust.steps.find((s: any) => s.step === u.step);
  return {
    step: u.step, note: u.note,
    upstream: { exit: u.exit, checkbox: u.checkbox, memory_empty_total: u.memory_empty_total },
    rust: r ? { exit: r.exit, checkbox: r.checkbox, memory_empty_total: r.memory_empty_total } : "採取できず",
    same_total: r ? u.memory_empty_total === r.memory_empty_total : null,
  };
});

mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, `${JSON.stringify({
  question: "Q2: MEMORY_EMPTY の重複抑止の鍵（ゲート段での再承認）",
  source: { ...UPSTREAM, verified_manifest: sourceInfo.manifest ?? UPSTREAM.manifest },
  rust_binary: { path: rustBin, sha256: createHash("sha256").update(readFileSync(rustBin)).digest("hex") },
  tool_versions: { bun: Bun.version },
  capture_command: [process.execPath, import.meta.path, dist, rustBin, destination],
  normalization: "一時 workspace の絶対パス・ISO 時刻・sha256・intent 名のみ",
  comparison,
  observations: { upstream, rust },
}, null, 2)}\n`);
console.log(JSON.stringify(comparison.map((c: any) => ({
  step: c.step,
  up: `${c.upstream.exit}/${c.upstream.memory_empty_total}/${c.upstream.checkbox.slice(0, 9)}`,
  rs: c.rust === "採取できず" ? c.rust : `${c.rust.exit}/${c.rust.memory_empty_total}/${c.rust.checkbox.slice(0, 9)}`,
  same: c.same_total,
})), null, 1));
