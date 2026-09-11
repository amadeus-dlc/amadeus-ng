#!/usr/bin/env bun
// 512KiB 境界と「規則が読めない」拒否を、本家と本 build の**両方を実プロセスで**走らせて
// 突き合わせる。
//
// 両者が同じ規則層を読むよう、1 つの一時ワークスペースを作り、
//   本家 : AIDLC_PROJECT_DIR / AIDLC_AGENTS_DIR / AIDLC_STAGE_GRAPH をそこへ向ける
//   本build: そこを作業ディレクトリにする（この build は projectDir を cwd から取る）
// とする。一時ワークスペースは終了時に消す。本リポジトリには何も書かない。

import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO = process.cwd();
const HOOK = join(REPO, ".claude/hooks/aidlc-deliver-stage-rules.ts");
const RUST = join(REPO, "target/debug/aidlc");
const LIMIT = 512 * 1024;

const root = mkdtempSync(join(tmpdir(), "aidlc-boundary-"));
const data = join(root, ".claude/tools/data");
const agentsDir = join(root, ".claude/agents");
const memory = join(root, "aidlc/spaces/default/memory");
mkdirSync(data, { recursive: true });
mkdirSync(agentsDir, { recursive: true });
mkdirSync(join(memory, "phases"), { recursive: true });
for (const name of ["stage-graph.json", "harness.json", "scope-grid.json"]) {
  cpSync(join(REPO, "tests/golden/upstream-a277af21/data", name), join(data, name));
}
writeFileSync(join(agentsDir, "aidlc-developer-agent.md"), "persona", "utf-8");
writeFileSync(join(memory, "team.md"), "# Team\n", "utf-8");
writeFileSync(join(memory, "project.md"), "# Project\n", "utf-8");
writeFileSync(join(memory, "phases/construction.md"), "# Construction\n", "utf-8");

const STDIN = new TextEncoder().encode(
  JSON.stringify({
    tool_name: "Task",
    tool_input: {
      subagent_type: "aidlc-developer-agent",
      prompt: "B /stages/construction/code-generation.md",
    },
  }),
);

async function run(side: "upstream" | "rust", extra: Record<string, string> = {}) {
  const env: Record<string, string | undefined> = { ...process.env, ...extra };
  let command: string[];
  let cwd: string;
  if (side === "upstream") {
    env.AIDLC_PROJECT_DIR = root;
    env.AIDLC_AGENTS_DIR = agentsDir;
    env.AIDLC_STAGE_GRAPH = join(data, "stage-graph.json");
    command = ["bun", HOOK];
    cwd = REPO;
  } else {
    delete env.AIDLC_PROJECT_DIR;
    delete env.AIDLC_AGENTS_DIR;
    delete env.AIDLC_STAGE_GRAPH;
    command = [RUST, "hook", "deliver-stage-rules"];
    cwd = root;
  }
  const proc = Bun.spawn(command, { stdin: STDIN, stdout: "pipe", stderr: "pipe", env, cwd });
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exit: await proc.exited, bytes: Buffer.byteLength(stdout, "utf-8"), stderr: stderr.trim() };
}

const org = (body: number) =>
  writeFileSync(join(memory, "org.md"), `# Org\n${"x".repeat(body)}\n`, "utf-8");

process.stdout.write("case\tside\texit\tstdout_bytes\tstderr\n");
const row = (label: string, side: string, r: Awaited<ReturnType<typeof run>>) =>
  process.stdout.write(
    `${label}\t${side}\t${r.exit}\t${r.bytes}\t${r.stderr.replace(/\s+/g, " ") || "-"}\n`,
  );

// 出力バイト数が LIMIT ちょうどになる本文長を、本家側で求める。
org(1000);
const low = (await run("upstream")).bytes;
org(2000);
const high = (await run("upstream")).bytes;
const slope = (high - low) / 1000;
let exact = 1000 + Math.round((LIMIT - low) / slope);
for (;;) {
  org(exact);
  const measured = (await run("upstream")).bytes;
  if (measured === LIMIT) break;
  exact += measured < LIMIT ? 1 : -1;
}

for (const [label, body] of [
  ["under-by-one", exact - 1],
  ["exactly-at-the-limit", exact],
  ["over-by-one", exact + 1],
] as const) {
  org(body);
  row(label, "upstream", await run("upstream"));
  row(label, "rust", await run("rust"));
}

org(exact + 1);
row("fallback-over-by-one", "upstream", await run("upstream", { AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK: "1" }));
row("fallback-over-by-one", "rust", await run("rust", { AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK: "1" }));

rmSync(join(memory, "org.md"));
row("required-rule-missing", "upstream", await run("upstream"));
row("required-rule-missing", "rust", await run("rust"));

writeFileSync(join(memory, "org.md"), Buffer.from([0x80, 0x81]));
row("required-rule-not-utf8", "upstream", await run("upstream"));
row("required-rule-not-utf8", "rust", await run("rust"));

rmSync(root, { recursive: true, force: true });
