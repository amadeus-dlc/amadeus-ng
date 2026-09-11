#!/usr/bin/env bun
/**
 * 裁定 Q1 の実走行採取（第 2 版）。
 *
 * per-unit ステージ（`for_each: unit-of-work`）で、本家 review-freeze が
 * ゼロ Unit のときに `produces` への書込みを凍結するかを実測する。
 *
 * 第 1 版からの変更:
 *   - 対照群 A が本家側でも成立するよう、要求分析の確認事項に
 *     `[Answer]: Looks correct` を入れた（本家の要約確認ガードを満たす）。
 *   - Unit 有りの条件 C を `feature` scope（`units-generation` が EXECUTE）にした。
 *     `bugfix` は `units-generation` が SKIP なので、本家自身が per-unit ステージの
 *     成果物を stage 水準のパスへ置く（`usesStageLevelPerUnitArtifacts`）。
 *   - 1 本の workspace へ**両方のフックを撃つ**（相互投入）。記録の作り手が違うことを
 *     差の原因から外すためである。
 *
 * 変更は mkdtemp の一時 workspace だけに限定する。本リポジトリの記録・監査・状態・
 * memory・受領証は一切触らない。
 *
 * 使い方:
 *   bun capture-q1-review-freeze.ts <dist(=fixed commit tree)> <rust-binary> <out.json>
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

type Impl = "upstream" | "rust";
const IMPLS: Impl[] = ["upstream", "rust"];

const REVIEWER = "aidlc-architecture-reviewer-agent";
const PRODUCT_REVIEWER = "aidlc-product-lead-agent";

/** 本家の受領証が求める逐語の付録。 */
const reviewAppendix = (reviewer: string) =>
  `\n## Review\n\n**Reviewer:** ${reviewer}\n**Verdict:** READY\n**Iteration:** 1\n\n### Findings\nNone.\n`;

const artifactBody = (title: string) =>
  `# ${title}\n\n## Sources\n[desc] Initial description: capture fixture\n\n## Assumptions & Open Questions\nNone.\n`;

/** 本家の要約確認ガードを満たす確認事項。 */
const QUESTIONS_BODY =
  "# Questions\n\n## Q1\n修正対象は何か。\nA. 小さな不具合\nX. Other (please specify)\n[Answer]: A\n\n" +
  "## Consolidated Summary Confirmation\nLooks correct / Request changes\n[Answer]: Looks correct\n";

interface Harness {
  builder: Impl;
  root: string;
  parent: string;
  record: string;
  tool(tool: string, args: string[]): { status: number | null; stdout: string; stderr: string };
  /** 指定した実装のフックを、この workspace へ撃つ。 */
  hookBy(impl: Impl, name: string, stdin: string): { status: number | null; stdout: string; stderr: string };
  audit(): string;
  dispose(): void;
}

function harness(builder: Impl, scope: string): Harness {
  // macOS の `/var` は `/private/var` への symlink なので実体パスへ寄せる。
  const parent = realpathSync(mkdtempSync(join(tmpdir(), `aidlc-q1-${builder}-`)));
  const root = join(parent, "workspace");
  mkdirSync(root);
  // 両実装へ同一の `.claude/`（固定コミットの実バイト）を渡す。
  cpSync(join(dist, ".claude"), join(root, ".claude"), { recursive: true });
  writeFileSync(join(root, "source.rs"), "fn main() {}\n");
  const env = { PATH: process.env.PATH!, HOME: join(parent, "home") };

  // 本 build は argv[0] で面を選ぶ multicall バイナリ。
  const bin = join(parent, "bin");
  mkdirSync(bin);
  for (const name of ["aidlc", "aidlc-utility", "aidlc-log", "aidlc-bolt", "aidlc-jump", "aidlc-orchestrate", "aidlc-state"]) {
    copyFileSync(rustBin, join(bin, name));
  }

  const upstreamTool = (name: string, args: string[]) =>
    spawnSync(process.execPath, [join(root, ".claude/tools", `${name}.ts`), ...args],
      { cwd: root, env, encoding: "utf8" }) as never;
  const rustTool = (name: string, args: string[]) =>
    spawnSync(join(bin, name), args, { cwd: root, env, encoding: "utf8" }) as never;
  const tool = builder === "upstream" ? upstreamTool : rustTool;

  const hookBy = (impl: Impl, name: string, stdin: string) =>
    impl === "upstream"
      ? spawnSync(process.execPath, [join(root, ".claude/hooks", `aidlc-${name}.ts`)],
          { cwd: root, env, encoding: "utf8", input: stdin }) as never
      : spawnSync(join(bin, "aidlc"), ["hook", name],
          { cwd: root, env, encoding: "utf8", input: stdin }) as never;

  const created = tool("aidlc-utility",
    ["intent-create", "--scope", scope, "--label", "freeze", "--arguments", "Fix the review guards"]);
  assert.equal(created.status, 0, `${builder} intent-create(${scope}): ${created.stderr}${created.stdout}`);
  const intents = join(root, "aidlc/spaces/default/intents");
  const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());
  const auditDir = join(record, "audit");
  const audit = () => existsSync(auditDir)
    ? readdirSync(auditDir).sort().map((p) => readFileSync(join(auditDir, p), "utf8")).join("")
    : "";
  return { builder, root, parent, record, tool, hookBy, audit, dispose: () => rmSync(parent, { recursive: true, force: true }) };
}

const normalise = (h: Harness, text: string) =>
  text.replaceAll(h.record, "<RECORD>").replaceAll(h.root, "<ROOT>").replaceAll(h.parent, "<PARENT>")
    .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g, "<TS>")
    .replace(/sha256:[0-9a-f]{64}/g, "sha256:<HASH>")
    .replace(/\b[0-9a-f]{64}\b/g, "<HASH>")
    .replace(/\b\d{6}-freeze\b/g, "<INTENT>");

function put(h: Harness, relative: string, text: string) {
  const target = join(h.record, relative);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, text);
}

/** code-generation の produces（`traceability` だけ `.json` である）。 */
const CG_PRODUCES = ["code-generation-plan", "unit-test-instructions", "code-summary", "traceability"] as const;

/** そのステージの必須 produces を一式そろえる。実ファイル名は本家の語彙に従う。 */
function putProduces(h: Harness, prefix: string) {
  for (const n of CG_PRODUCES) {
    if (n === "traceability") {
      put(h, `${prefix}/traceability.json`, `${JSON.stringify({ requirements: [], units: [] }, null, 2)}\n`);
    } else {
      put(h, `${prefix}/${n}.md`, artifactBody(n));
    }
  }
}

function terminalReceipt(h: Harness, stage: string, reviewer: string, unit: string | null,
                         appendixRelative: string, steps: any[]): boolean {
  const base = ["review", "--stage", stage, "--reviewer", reviewer, "--iteration", "1",
                ...(unit ? ["--unit", unit] : [])];
  const req = h.tool("aidlc-log", base);
  steps.push({ step: `review-request${unit ? `/${unit}` : ""}`, argv: base, exit: req.status,
               stdout: normalise(h, req.stdout), stderr: normalise(h, req.stderr) });
  if (req.status !== 0) return false;
  appendFileSync(join(h.record, appendixRelative), reviewAppendix(reviewer));
  const done = h.tool("aidlc-log", [...base, "--verdict", "READY"]);
  steps.push({ step: `review-complete${unit ? `/${unit}` : ""}`, argv: [...base, "--verdict", "READY"],
               exit: done.status, stdout: normalise(h, done.stdout), stderr: normalise(h, done.stderr) });
  return done.status === 0;
}

const writeInput = (h: Harness, relative: string) =>
  JSON.stringify({ tool_name: "Write", tool_input: { file_path: join(h.record, relative) } });

function freezeRows(h: Harness, before: string): string[] {
  return h.audit().slice(before.length).split("\n---\n")
    .filter((b) => b.includes("REVIEW_FREEZE_BLOCKED"))
    .map((b) => normalise(h, b).trim());
}

// --- 採取の条件 ---------------------------------------------------------------

type Case = {
  id: string;
  scope: string;
  /** 記録を組める実装（本 build に配線が無い条件は upstream だけ）。 */
  builders: Impl[];
  description: string;
  setup(h: Harness, steps: any[]): boolean;
  probes: { id: string; relative: string; expectation: string }[];
};

const CASES: Case[] = [
  {
    id: "A-control/non-per-unit-stage",
    scope: "bugfix",
    builders: IMPLS,
    description: "対照群。per-unit でない requirements-analysis に終端受領証を立てる。両者が記録を読めていることを示す陽性対照。",
    setup(h, steps) {
      put(h, "inception/requirements-analysis/requirements.md", artifactBody("Requirements"));
      put(h, "inception/requirements-analysis/requirements-analysis-questions.md", QUESTIONS_BODY);
      return terminalReceipt(h, "requirements-analysis", PRODUCT_REVIEWER, null,
        "inception/requirements-analysis/requirements.md", steps);
    },
    probes: [
      { id: "declared-artifact", relative: "inception/requirements-analysis/requirements.md", expectation: "両者とも凍結するはず" },
      { id: "undeclared-diary", relative: "inception/requirements-analysis/memory.md", expectation: "両者とも通すはず" },
    ],
  },
  {
    id: "B-zero-unit/stage-level-receipt",
    scope: "bugfix",
    builders: IMPLS,
    description: "本題。per-unit ステージ code-generation に Unit を 1 つも作らず、stage 水準の終端受領証を立てた状態で produces を書く。bugfix は units-generation が SKIP なので、本家自身がこのパスを正規の置き場にする。",
    setup(h, steps) {
      putProduces(h, "construction/code-generation");
      return terminalReceipt(h, "code-generation", REVIEWER, null,
        "construction/code-generation/code-generation-plan.md", steps);
    },
    probes: [
      { id: "declared-plan", relative: "construction/code-generation/code-generation-plan.md", expectation: "裁定の対象" },
      { id: "declared-summary", relative: "construction/code-generation/code-summary.md", expectation: "裁定の対象" },
      { id: "undeclared-diary", relative: "construction/code-generation/memory.md", expectation: "両者とも通すはず" },
    ],
  },
  {
    id: "C-unit/unit-receipt",
    scope: "feature",
    builders: ["upstream"],  // 本 build は `aidlc-bolt start` が未配線で同じ前提を組めない
    description: "Unit 有り。units-generation が EXECUTE の feature scope で Bolt u1/u2 を起こし、u1 にだけ終端受領証を立てる。自 Unit・stage 水準・兄弟 Unit の 3 宛先を撃つ。",
    setup(h, steps) {
      for (const name of ["u1", "u2"]) {
        const r = h.tool("aidlc-bolt", ["start", "--name", name, "--batch", "1"]);
        steps.push({ step: `bolt-start/${name}`, exit: r.status,
                     stdout: normalise(h, r.stdout), stderr: normalise(h, r.stderr) });
        if (r.status !== 0) return false;
      }
      // Bolt DAG は runtime-graph.json の bolt_dag から解決される。
      const compiled = h.tool("aidlc-runtime", ["compile"]);
      steps.push({ step: "runtime-compile", exit: compiled.status,
                   stdout: normalise(h, compiled.stdout), stderr: normalise(h, compiled.stderr) });
      for (const u of ["u1", "u2"]) {
        putProduces(h, `construction/${u}/code-generation`);
        // 本家は Unit ごとの受領証に `source-manifest.json` を要求する
        // （`aidlc-lib.ts:13815-13852`）。書込み 0 件の最小形で通す。
        put(h, `construction/${u}/code-generation/source-manifest.json`,
          `${JSON.stringify({ stage: "code-generation", unit: u, version: 1, writes: [] }, null, 2)}\n`);
      }
      return terminalReceipt(h, "code-generation", REVIEWER, "u1",
        "construction/u1/code-generation/code-generation-plan.md", steps);
    },
    probes: [
      { id: "own-unit", relative: "construction/u1/code-generation/code-generation-plan.md", expectation: "受領証を持つ Unit 自身" },
      { id: "stage-level", relative: "construction/code-generation/code-generation-plan.md", expectation: "Unit 受領証があるときの stage 水準" },
      { id: "sibling-unit", relative: "construction/u2/code-generation/code-generation-plan.md", expectation: "受領証を持たない兄弟 Unit" },
    ],
  },
  {
    id: "D-zero-unit/no-receipt",
    scope: "bugfix",
    builders: IMPLS,
    description: "陰性対照。受領証を一切立てずに同じ宛先を撃つ。両者とも通すはず。",
    setup(h) {
      putProduces(h, "construction/code-generation");
      return true;
    },
    probes: [
      { id: "declared-plan", relative: "construction/code-generation/code-generation-plan.md", expectation: "両者とも通すはず" },
    ],
  },
];

// --- 実行 ---------------------------------------------------------------------

const results: any[] = [];
for (const c of CASES) {
  for (const builder of c.builders) {
    const h = harness(builder, c.scope);
    const steps: any[] = [];
    let setupOk = false;
    let setupError: string | null = null;
    try { setupOk = c.setup(h, steps); }
    catch (e) { setupError = e instanceof Error ? e.message : String(e); }
    const probes: any[] = [];
    if (setupOk) {
      for (const p of c.probes) {
        const stdin = writeInput(h, p.relative);
        const byImpl: Record<string, any> = {};
        for (const impl of IMPLS) {
          const before = h.audit();
          const r = h.hookBy(impl, "review-freeze", stdin);
          byImpl[impl] = {
            exit: r.status, blocked: r.status === 2,
            stdout: normalise(h, r.stdout), stderr: normalise(h, r.stderr),
            audit_freeze_rows: freezeRows(h, before),
          };
        }
        probes.push({ probe: p.id, target: `<RECORD>/${p.relative}`, expectation: p.expectation, stdin: normalise(h, stdin), ...byImpl });
      }
    }
    results.push({ case: c.id, scope: c.scope, description: c.description, built_by: builder,
                   setup_ok: setupOk, setup_error: setupError, setup_steps: steps, probes });
    h.dispose();
  }
}

const comparison = results.map((r) => ({
  case: r.case,
  scope: r.scope,
  built_by: r.built_by,
  setup_ok: r.setup_ok,
  rows: r.probes.map((p: any) => ({
    probe: p.probe,
    upstream_hook: { exit: p.upstream.exit, blocked: p.upstream.blocked, freeze_rows: p.upstream.audit_freeze_rows.length },
    rust_hook: { exit: p.rust.exit, blocked: p.rust.blocked, freeze_rows: p.rust.audit_freeze_rows.length },
    same: p.upstream.exit === p.rust.exit,
  })),
}));

mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, `${JSON.stringify({
  question: "Q1: per-unit 受領証を配線するか（review-freeze の凍結範囲）",
  source: { ...UPSTREAM, verified_manifest: sourceInfo.manifest ?? UPSTREAM.manifest },
  rust_binary: { path: rustBin, sha256: createHash("sha256").update(readFileSync(rustBin)).digest("hex") },
  tool_versions: { bun: Bun.version },
  capture_command: [process.execPath, import.meta.path, dist, rustBin, destination],
  normalization: "一時 workspace の絶対パス・ISO 時刻・sha256・intent 名のみ",
  design: "1 本の workspace へ両実装のフックを撃つ相互投入。記録の作り手は built_by が示す。",
  comparison,
  observations: results,
}, null, 2)}\n`);
console.log(JSON.stringify(comparison, null, 2));
