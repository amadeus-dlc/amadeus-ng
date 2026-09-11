#!/usr/bin/env bun
/**
 * 探索: 条件 B（ゼロ Unit の per-unit ステージ）で、記録の作り手が違うと
 * 本 build のフックの結論が変わる理由を調べる。受領証の監査行を突き合わせる。
 * 一時 workspace のみ。
 *
 *   bun probe-caseB-receipts.ts <dist> <rust-binary>
 */
import { appendFileSync, copyFileSync, cpSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

const [dist, rustBin] = process.argv.slice(2);
const REVIEWER = "aidlc-architecture-reviewer-agent";
const body = (t: string) => `# ${t}\n\n## Sources\n[desc] x\n\n## Assumptions & Open Questions\nNone.\n`;
const APPENDIX = `\n## Review\n\n**Reviewer:** ${REVIEWER}\n**Verdict:** READY\n**Iteration:** 1\n\n### Findings\nNone.\n`;

for (const builder of ["upstream", "rust"] as const) {
  const parent = realpathSync(mkdtempSync(join(tmpdir(), `aidlc-B-${builder}-`)));
  const root = join(parent, "workspace");
  mkdirSync(root);
  cpSync(join(dist, ".claude"), join(root, ".claude"), { recursive: true });
  writeFileSync(join(root, "source.rs"), "fn main() {}\n");
  const env = { PATH: process.env.PATH!, HOME: join(parent, "home") };
  const bin = join(parent, "bin");
  mkdirSync(bin);
  for (const n of ["aidlc", "aidlc-utility", "aidlc-log", "aidlc-orchestrate"]) copyFileSync(rustBin, join(bin, n));
  const opts = { cwd: root, env, encoding: "utf8" as const, input: "", timeout: 30_000 };
  const up = (name: string, a: string[]) => spawnSync(process.execPath, [join(root, ".claude/tools", `${name}.ts`), ...a], opts);
  const rs = (name: string, a: string[]) => spawnSync(join(bin, name), a, opts);
  const tool = builder === "upstream" ? up : rs;

  try {
    const created = tool("aidlc-utility", ["intent-create", "--scope", "bugfix", "--label", "freeze", "--arguments", "Fix the review guards"]);
    if (created.status !== 0) throw new Error(created.stderr);
    const intents = join(root, "aidlc/spaces/default/intents");
    const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());

    const dir = join(record, "construction/code-generation");
    mkdirSync(dir, { recursive: true });
    for (const n of ["code-generation-plan", "unit-test-instructions", "code-summary"]) writeFileSync(join(dir, `${n}.md`), body(n));
    writeFileSync(join(dir, "traceability.json"), `${JSON.stringify({ requirements: [], units: [] }, null, 2)}\n`);

    const base = ["review", "--stage", "code-generation", "--reviewer", REVIEWER, "--iteration", "1"];
    const req = tool("aidlc-log", base);
    appendFileSync(join(dir, "code-generation-plan.md"), APPENDIX);
    const done = tool("aidlc-log", [...base, "--verdict", "READY"]);

    const auditDir = join(record, "audit");
    const audit = readdirSync(auditDir).sort().map((p) => readFileSync(join(auditDir, p), "utf8")).join("");
    const reviewBlocks = audit.split("\n---\n").filter((b) => b.includes("REVIEW_REQUESTED") || b.includes("REVIEW_COMPLETED"));

    console.log(`\n================ built_by=${builder} ================`);
    console.log(`review-request exit=${req.status} ${req.stderr.trim().slice(0, 200)}`);
    console.log(`review-complete exit=${done.status} ${done.stderr.trim().slice(0, 200)}`);
    console.log("--- 監査の受領証行 ---");
    for (const b of reviewBlocks) console.log(b.replace(record, "<RECORD>").trim(), "\n");

    const stdin = JSON.stringify({ tool_name: "Write", tool_input: { file_path: join(dir, "code-generation-plan.md") } });
    const upHook = spawnSync(process.execPath, [join(root, ".claude/hooks/aidlc-review-freeze.ts")], { ...opts, input: stdin });
    const rsHook = spawnSync(join(bin, "aidlc"), ["hook", "review-freeze"], { ...opts, input: stdin });
    console.log(`--- フックの結論 ---`);
    console.log(`  upstream hook: exit=${upHook.status} ${upHook.stderr.trim().slice(0, 160)}`);
    console.log(`  rust hook    : exit=${rsHook.status} ${rsHook.stderr.trim().slice(0, 160)}`);

    console.log("--- 記録の中身（作り手ごとに何が置かれるか）---");
    const walk = (dir: string, prefix = ""): string[] => {
      const out: string[] = [];
      for (const e of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
        const rel = prefix ? `${prefix}/${e.name}` : e.name;
        if (e.isDirectory()) out.push(`${rel}/`, ...walk(join(dir, e.name), rel));
        else out.push(rel);
      }
      return out;
    };
    for (const p of walk(record)) console.log("   ", p);
    const stageLine = readFileSync(join(record, "aidlc-state.md"), "utf8")
      .split("\n").find((l) => l.includes("code-generation —")) ?? "?";
    console.log("   state の code-generation 行:", stageLine.trim());
  } catch (e) {
    console.log(`[${builder}] ERROR:`, e instanceof Error ? e.message : String(e));
  } finally {
    rmSync(parent, { recursive: true, force: true });
  }
}
