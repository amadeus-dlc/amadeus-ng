#!/usr/bin/env bun
// 512KiB 境界と「規則が読めない」拒否を、本家（固定コミットの展開写し）と本 build の**両方**で
// 実走行して突き合わせる。前担当の boundary-differential.ts と同じ手順だが、本家は
// CLOSEOUT_ROOT/boundary/.claude（`git archive` の写し）から起動し、stderr の一致も列にする。
//
// project dir は CLOSEOUT_ROOT/boundary（closeout-workspaces.sh が .claude と最小の memory を置く）。
// org.md だけをこのスクリプトが書き換える。本リポジトリには何も書かない。
//
// 使い方: CLOSEOUT_ROOT=<dir> AIDLC_BIN=<bin> bun closeout-boundary-differential.ts <out.tsv>

import { createHash } from "node:crypto";
import { rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = process.env.CLOSEOUT_ROOT!;
const BIN = process.env.AIDLC_BIN!;
const PROJECT = join(ROOT, "boundary");
const HOOK = join(PROJECT, ".claude/hooks/aidlc-deliver-stage-rules.ts");
const MEMORY = join(PROJECT, "aidlc/spaces/default/memory");
const LIMIT = 512 * 1024;

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
  const env: Record<string, string> = {
    PATH: process.env.PATH ?? "/usr/bin:/bin",
    HOME: join(ROOT, "home"),
    AIDLC_PROJECT_DIR: PROJECT,
    ...extra,
  };
  const command = side === "upstream" ? [process.execPath, HOOK] : [BIN, "hook", "deliver-stage-rules"];
  const proc = Bun.spawn(command, { stdin: STDIN, stdout: "pipe", stderr: "pipe", env, cwd: PROJECT });
  const stdout = new Uint8Array(await new Response(proc.stdout).arrayBuffer());
  const stderr = new Uint8Array(await new Response(proc.stderr).arrayBuffer());
  return {
    exit: await proc.exited,
    bytes: stdout.length,
    sha: stdout.length === 0 ? "-" : createHash("sha256").update(stdout).digest("hex"),
    stderr: new TextDecoder().decode(stderr),
  };
}

const org = (body: number) =>
  writeFileSync(join(MEMORY, "org.md"), `# Org\n${"x".repeat(body)}\n`, "utf-8");

const lines = ["case\tmatch\tside\texit\tstdout_bytes\tstdout_sha256\tstderr"];
let same = 0;
let stderrOnly = 0;
let differ = 0;
async function pair(label: string, extra: Record<string, string> = {}) {
  const up = await run("upstream", extra);
  const rs = await run("rust", extra);
  const exitAndStdout = up.exit === rs.exit && up.sha === rs.sha;
  const stderrSame = up.stderr === rs.stderr;
  const match = exitAndStdout && stderrSame ? "same" : exitAndStdout ? "stderr-only" : "DIFFER";
  if (match === "same") same++;
  else if (match === "stderr-only") stderrOnly++;
  else differ++;
  for (const [side, r] of [["upstream", up], ["rust", rs]] as const) {
    lines.push(
      `${label}\t${match}\t${side}\t${r.exit}\t${r.bytes}\t${r.sha.slice(0, 16)}\t${r.stderr.trim().replace(/\s+/g, " ") || "-"}`,
    );
  }
}

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
lines.push(`# body length that makes upstream stdout exactly ${LIMIT} bytes: ${exact}`);

for (const [label, body] of [
  ["under-by-one", exact - 1],
  ["exactly-at-the-limit", exact],
  ["over-by-one", exact + 1],
] as const) {
  org(body);
  await pair(label);
}
org(exact + 1);
await pair("fallback-over-by-one", { AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK: "1" });
org(exact);
await pair("fallback-exactly-at-the-limit", { AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK: "1" });

rmSync(join(MEMORY, "org.md"));
await pair("required-rule-missing");

writeFileSync(join(MEMORY, "org.md"), Buffer.from([0x80, 0x81]));
await pair("required-rule-not-utf8");

writeFileSync(join(MEMORY, "org.md"), Buffer.from([0x23, 0x20, 0x4f, 0x72, 0x67, 0x0a, 0x0a, 0x78, 0xed, 0xa0, 0x80, 0x0a]));
await pair("required-rule-with-an-encoded-surrogate");

lines.push(`# same=${same} stderr-only=${stderrOnly} differ=${differ} total=${same + stderrOnly + differ}`);
writeFileSync(process.argv[2], lines.join("\n") + "\n", "utf-8");
process.stdout.write(lines[lines.length - 1] + "\n");
