#!/usr/bin/env bun
/**
 * 探索: ゲートのある requirements-analysis で「承認 → 再跳躍 → 再承認」を一周できるか。
 * 一周できれば `MEMORY_EMPTY` の鍵そのものを切り分けられる。一時 workspace のみ。
 *
 *   bun probe-gated-approval.ts <dist> <rust-binary>
 */
import { appendFileSync, copyFileSync, cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

const [dist, rustBin] = process.argv.slice(2);
const EMPTY_JOURNAL = "# Stage Memory\n\n## Interpretations\n\n## Deviations\n\n## Tradeoffs\n\n## Open questions\n";
const QUESTIONS = "# Questions\n\n## Q1\n修正対象は何か。\nA. 小さな不具合\nX. Other (please specify)\n[Answer]: A\n\n## Consolidated Summary Confirmation\nLooks correct / Request changes\n[Answer]: Looks correct\n";
const REQUIREMENTS = "# Requirements\n\n## Functional Requirements\nFR1: 小さな不具合を修正する。\n\n## Sources\n[Q1] 承認済み回答。\n\n## Assumptions & Open Questions\nNone.\n";
const APPENDIX = "\n## Review\n\n**Reviewer:** aidlc-product-lead-agent\n**Verdict:** READY\n**Iteration:** 1\n\n### Findings\nNone.\n";
const ENVELOPE = JSON.stringify({
  session_id: "11111111-2222-4333-8444-555555555555",
  tool_name: "Bash",
  tool_input: { command: "bun .claude/tools/aidlc-orchestrate.ts report --result completed" },
  tool_response: { stdout: "" },
});

function build(impl: "upstream" | "rust") {
  const parent = mkdtempSync(join(tmpdir(), `aidlc-gate-${impl}-`));
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
  const opts = { cwd: root, env, encoding: "utf8" as const, input: "", timeout: 20_000 };
  const tool = (name: string, args: string[]) =>
    impl === "upstream"
      ? spawnSync(process.execPath, [join(root, ".claude/tools", `${name}.ts`), ...args], opts)
      : spawnSync(join(bin, name), args, opts);
  const hook = (name: string, stdin: string) =>
    impl === "upstream"
      ? spawnSync(process.execPath, [join(root, ".claude/hooks", `aidlc-${name}.ts`)], { ...opts, input: stdin })
      : spawnSync(join(bin, "aidlc"), ["hook", name], { ...opts, input: stdin });
  const created = tool("aidlc-utility", ["intent-create", "--scope", "bugfix", "--label", "gate", "--arguments", "Fix a small bug"]);
  if (created.status !== 0) throw new Error(`${impl} intent-create: ${created.stderr}`);
  const intents = join(root, "aidlc/spaces/default/intents");
  const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());
  return { impl, root, parent, record, tool, hook };
}

const show = (impl: string, id: string, r: any) => {
  const out = (r.stdout || "").trim().replace(/\s+/g, " ").slice(0, 420);
  const err = (r.stderr || "").trim().replace(/\s+/g, " ").slice(0, 300);
  console.log(`  [${impl}] ${id.padEnd(50)} exit=${r.status}`);
  if (out) console.log(`      out: ${out}`);
  if (err) console.log(`      err: ${err}`);
};

for (const impl of ["upstream", "rust"] as const) {
  console.log(`\n===================== ${impl} =====================`);
  const h = build(impl);
  try {
    const cb = () => {
      const state = readFileSync(join(h.record, "aidlc-state.md"), "utf8");
      return (state.split("\n").find((l) => l.includes("requirements-analysis —")) ?? "?").trim();
    };
    const auditDir = join(h.record, "audit");
    const rows = () => readdirSync(auditDir).sort().map((p) => readFileSync(join(auditDir, p), "utf8")).join("")
      .split("\n---\n").filter((b) => b.includes("**Event**: MEMORY_EMPTY")).length;

    show(impl, "next bugfix", h.tool("aidlc-orchestrate", ["next", "bugfix"]));
    show(impl, "jump forward requirements-analysis",
      h.tool("aidlc-jump", ["execute", "--target", "requirements-analysis", "--direction", "forward"]));
    console.log(`      requirements-analysis: ${cb()}`);

    const dir = join(h.record, "inception/requirements-analysis");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "requirements.md"), REQUIREMENTS);
    writeFileSync(join(dir, "requirements-analysis-questions.md"), QUESTIONS);
    writeFileSync(join(dir, "memory.md"), EMPTY_JOURNAL);

    const base = ["review", "--stage", "requirements-analysis", "--reviewer", "aidlc-product-lead-agent", "--iteration", "1"];
    show(impl, "review request", h.tool("aidlc-log", base));
    appendFileSync(join(dir, "requirements.md"), APPENDIX);
    show(impl, "review complete READY", h.tool("aidlc-log", [...base, "--verdict", "READY"]));

    show(impl, "report --result awaiting-approval",
      h.tool("aidlc-orchestrate", ["report", "--stage", "requirements-analysis", "--result", "awaiting-approval"]));
    console.log(`      requirements-analysis: ${cb()}`);

    for (const choice of ["Approve", "Approve and continue", "1"]) {
      const r = h.tool("aidlc-orchestrate", ["report", "--stage", "requirements-analysis", "--result", "approved", "--user-input", choice]);
      show(impl, `report --result approved --user-input "${choice}"`, r);
      console.log(`      requirements-analysis: ${cb()}`);
      if (cb().startsWith("- [x]")) break;
    }

    writeFileSync(join(dir, "memory.md"), EMPTY_JOURNAL);
    show(impl, "compile", h.hook("rebuild-stage-graph", ENVELOPE));
    console.log(`      MEMORY_EMPTY rows: ${rows()}`);
  } catch (e) {
    console.log("  ERROR:", e instanceof Error ? e.message : String(e));
  } finally {
    rmSync(h.parent, { recursive: true, force: true });
  }
}
