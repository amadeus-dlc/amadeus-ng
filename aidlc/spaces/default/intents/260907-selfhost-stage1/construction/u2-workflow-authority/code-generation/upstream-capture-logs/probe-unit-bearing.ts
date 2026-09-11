#!/usr/bin/env bun
/** 探索: units-generation が EXECUTE の scope で Unit 有りの前提が組めるか。一時 workspace のみ。 */
import { cpSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, appendFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

const dist = process.argv[2];
const parent = mkdtempSync(join(tmpdir(), "aidlc-probe3-"));
const root = join(parent, "workspace");
mkdirSync(root);
cpSync(join(dist, ".claude"), join(root, ".claude"), { recursive: true });
writeFileSync(join(root, "source.rs"), "fn main() {}\n");
const env = { PATH: process.env.PATH!, HOME: join(parent, "home") };
const run = (t: string, a: string[]) =>
  spawnSync(process.execPath, [join(root, ".claude/tools", t), ...a], { cwd: root, env, encoding: "utf8" });
const show = (id: string, r: any) =>
  console.log(`### ${id} exit=${r.status}\n  out:${r.stdout.trim().slice(0, 300)}\n  err:${r.stderr.trim().slice(0, 600)}`);

show("intent-create", run("aidlc-utility.ts", ["intent-create", "--scope", "feature", "--label", "freeze", "--arguments", "Add a feature"]));
const intents = join(root, "aidlc/spaces/default/intents");
const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());
show("bolt-u1", run("aidlc-bolt.ts", ["start", "--name", "u1", "--batch", "1"]));
show("bolt-u2", run("aidlc-bolt.ts", ["start", "--name", "u2", "--batch", "1"]));
show("runtime-compile", run("aidlc-runtime.ts", ["compile"]));
const rg = join(record, "runtime-graph.json");
console.log("runtime-graph exists:", existsSync(rg));
if (existsSync(rg)) {
  const g = JSON.parse(readFileSync(rg, "utf8"));
  console.log("bolt_dag:", JSON.stringify(g.bolt_dag));
}

const P = ["code-generation-plan", "unit-test-instructions", "code-summary", "traceability"];
const body = (t: string) => `# ${t}\n\n## Sources\n[desc] x\n\n## Assumptions & Open Questions\nNone.\n`;
for (const u of ["u1", "u2"]) {
  for (const n of P) {
    const f = join(record, `construction/${u}/code-generation/${n}.md`);
    mkdirSync(dirname(f), { recursive: true });
    writeFileSync(f, body(n));
  }
}
const base = ["review", "--stage", "code-generation", "--reviewer", "aidlc-architecture-reviewer-agent", "--iteration", "1", "--unit", "u1"];
show("u1-request", run("aidlc-log.ts", base));
appendFileSync(
  join(record, "construction/u1/code-generation/code-generation-plan.md"),
  "\n## Review\n\n**Reviewer:** aidlc-architecture-reviewer-agent\n**Verdict:** READY\n**Iteration:** 1\n\n### Findings\nNone.\n",
);
show("u1-complete", run("aidlc-log.ts", [...base, "--verdict", "READY"]));

const hook = (rel: string) =>
  spawnSync(process.execPath, [join(root, ".claude/hooks/aidlc-review-freeze.ts")], {
    cwd: root, env, encoding: "utf8",
    input: JSON.stringify({ tool_name: "Write", tool_input: { file_path: join(record, rel) } }),
  });
for (const rel of [
  "construction/u1/code-generation/code-generation-plan.md",
  "construction/code-generation/code-generation-plan.md",
  "construction/u2/code-generation/code-generation-plan.md",
]) {
  const r = hook(rel);
  console.log(`HOOK ${rel}\n  exit=${r.status} stderr=${r.stderr.trim().slice(0, 260)}`);
}
console.log("PARENT:", parent);
