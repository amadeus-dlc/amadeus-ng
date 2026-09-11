#!/usr/bin/env bun
/**
 * 探索: 本 build の review-freeze が、上流形式の記録（状態ファイル + 監査シャード）だけで
 * 判定できるのか、それとも本 build 固有の集約スナップショット `.aidlc-execution` に
 * 依存しているのかを確かめる。
 *
 * 手順: 本 build で記録を組み、凍結が起きることを確かめてから `.aidlc-execution` だけを
 * 外して同じ入力を撃つ。結論が変わるなら、判定はそのファイルに依存している。
 * 一時 workspace のみ。
 *
 *   bun probe-execution-snapshot.ts <dist> <rust-binary>
 */
import { appendFileSync, copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const [dist, rustBin] = process.argv.slice(2);
const REVIEWER = "aidlc-architecture-reviewer-agent";
const PRODUCT = "aidlc-product-lead-agent";
const body = (t: string) => `# ${t}\n\n## Sources\n[desc] x\n\n## Assumptions & Open Questions\nNone.\n`;
const QUESTIONS = "# Questions\n\n## Q1\n何を直すか。\nA. 小さな不具合\nX. Other (please specify)\n[Answer]: A\n\n## Consolidated Summary Confirmation\nLooks correct / Request changes\n[Answer]: Looks correct\n";
const appendix = (r: string) => `\n## Review\n\n**Reviewer:** ${r}\n**Verdict:** READY\n**Iteration:** 1\n\n### Findings\nNone.\n`;

/** 条件ごとに 1 本ずつ本 build で記録を組み、スナップショットの有無で結論を比べる。 */
const CASES = [
  {
    id: "A: requirements-analysis（per-unit でない段）",
    stage: "requirements-analysis",
    reviewer: PRODUCT,
    dir: "inception/requirements-analysis",
    files: { "requirements.md": body("Requirements"), "requirements-analysis-questions.md": QUESTIONS },
    appendixFile: "requirements.md",
    probe: "requirements.md",
  },
  {
    id: "B: code-generation（ゼロ Unit の per-unit 段）",
    stage: "code-generation",
    reviewer: REVIEWER,
    dir: "construction/code-generation",
    files: {
      "code-generation-plan.md": body("Plan"),
      "unit-test-instructions.md": body("Tests"),
      "code-summary.md": body("Summary"),
      "traceability.json": `${JSON.stringify({ requirements: [], units: [] }, null, 2)}\n`,
    },
    appendixFile: "code-generation-plan.md",
    probe: "code-generation-plan.md",
  },
] as const;

for (const c of CASES) {
  const parent = realpathSync(mkdtempSync(join(tmpdir(), "aidlc-snap-")));
  const root = join(parent, "workspace");
  mkdirSync(root);
  cpSync(join(dist, ".claude"), join(root, ".claude"), { recursive: true });
  writeFileSync(join(root, "source.rs"), "fn main() {}\n");
  const env = { PATH: process.env.PATH!, HOME: join(parent, "home") };
  const bin = join(parent, "bin");
  mkdirSync(bin);
  for (const n of ["aidlc", "aidlc-utility", "aidlc-log"]) copyFileSync(rustBin, join(bin, n));
  const opts = { cwd: root, env, encoding: "utf8" as const, input: "", timeout: 30_000 };
  const tool = (name: string, a: string[]) => spawnSync(join(bin, name), a, opts);

  try {
    const created = tool("aidlc-utility", ["intent-create", "--scope", "bugfix", "--label", "snap", "--arguments", "Fix a small bug"]);
    if (created.status !== 0) throw new Error(`intent-create exit=${created.status} out=${created.stdout} err=${created.stderr}`);
    const intents = join(root, "aidlc/spaces/default/intents");
    const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());
    const dir = join(record, c.dir);
    mkdirSync(dir, { recursive: true });
    for (const [name, text] of Object.entries(c.files)) writeFileSync(join(dir, name), text);

    const base = ["review", "--stage", c.stage, "--reviewer", c.reviewer, "--iteration", "1"];
    const req = tool("aidlc-log", base);
    appendFileSync(join(dir, c.appendixFile), appendix(c.reviewer));
    const done = tool("aidlc-log", [...base, "--verdict", "READY"]);

    const stdin = JSON.stringify({ tool_name: "Write", tool_input: { file_path: join(dir, c.probe) } });
    const hookRust = () => spawnSync(join(bin, "aidlc"), ["hook", "review-freeze"], { ...opts, input: stdin });
    const hookUp = () => spawnSync(process.execPath, [join(root, ".claude/hooks/aidlc-review-freeze.ts")], { ...opts, input: stdin });

    const snapshot = join(record, ".aidlc-execution");
    console.log(`\n================ ${c.id} ================`);
    console.log(`  受領証: request exit=${req.status} / complete exit=${done.status}`);
    console.log(`  .aidlc-execution は存在するか: ${existsSync(snapshot)}`);
    const before = { rust: hookRust().status, upstream: hookUp().status };
    console.log(`  [スナップショット有り] rust hook exit=${before.rust}  upstream hook exit=${before.upstream}`);

    if (existsSync(snapshot)) {
      renameSync(snapshot, `${snapshot}.set-aside`);
      const after = { rust: hookRust().status, upstream: hookUp().status };
      console.log(`  [スナップショット無し] rust hook exit=${after.rust}  upstream hook exit=${after.upstream}`);
      console.log(`  ==> 本 build の結論はスナップショットに依存するか: ${before.rust !== after.rust}`);
      console.log(`  ==> 本家の結論はスナップショットに依存するか:     ${before.upstream !== after.upstream}`);
    }
  } catch (e) {
    console.log(`[${c.id}] ERROR:`, e instanceof Error ? e.message : String(e));
  } finally {
    rmSync(parent, { recursive: true, force: true });
  }
}
