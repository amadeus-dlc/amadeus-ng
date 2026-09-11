#!/usr/bin/env bun
// 重複抑止（本家 `hasExactBundle`）を、本家（固定コミットの展開写し）と本 build の**両方**で
// 実走行して突き合わせる。前担当の duplicate-probe.ts は本家だけを本リポジトリ上で走らせていた。
//
// 1 回目の出力から実際に付いたブロックを取り出し、それを含む brief をもう一度食わせて
// 「変化なし」になることを確かめる。1 バイト変えた場合・途中に置いた場合・2 個並べた場合も
// 同じ条件で観測する。project dir は CLOSEOUT_ROOT/duplicate（読取り専用）。
//
// 使い方: CLOSEOUT_ROOT=<dir> AIDLC_BIN=<bin> bun closeout-duplicate-differential.ts <out.tsv>

import { createHash } from "node:crypto";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = process.env.CLOSEOUT_ROOT!;
const BIN = process.env.AIDLC_BIN!;
const PROJECT = join(ROOT, "duplicate");
const HOOK = join(PROJECT, ".claude/hooks/aidlc-deliver-stage-rules.ts");
const ENV: Record<string, string> = {
  PATH: process.env.PATH ?? "/usr/bin:/bin",
  HOME: join(ROOT, "home"),
  AIDLC_PROJECT_DIR: PROJECT,
};

async function call(side: "upstream" | "rust", prompt: string) {
  const stdin = new TextEncoder().encode(
    JSON.stringify({
      cwd: PROJECT,
      tool_name: "Task",
      tool_input: { subagent_type: "aidlc-developer-agent", prompt },
    }),
  );
  const command = side === "upstream" ? [process.execPath, HOOK] : [BIN, "hook", "deliver-stage-rules"];
  const proc = Bun.spawn(command, { stdin, stdout: "pipe", stderr: "pipe", env: ENV, cwd: PROJECT });
  const stdout = new Uint8Array(await new Response(proc.stdout).arrayBuffer());
  const stderr = new Uint8Array(await new Response(proc.stderr).arrayBuffer());
  const exit = await proc.exited;
  const text = new TextDecoder().decode(stdout);
  let updated: string | null = null;
  try {
    updated = JSON.parse(text)?.hookSpecificOutput?.updatedInput?.prompt ?? null;
  } catch {
    updated = null;
  }
  return {
    exit,
    bytes: stdout.length,
    sha: stdout.length === 0 ? "-" : createHash("sha256").update(stdout).digest("hex"),
    stderrSha: stderr.length === 0 ? "-" : createHash("sha256").update(stderr).digest("hex"),
    blocks: (text.match(/AIDLC_DISPATCH_RULES_BEGIN/g) ?? []).length,
    updated,
  };
}

const lines = ["id\tmatch\tup_exit\trs_exit\tup_changed\trs_changed\tblocks(up/rs)\tup_sha256\trs_sha256"];
let same = 0;
let differ = 0;
async function probe(id: string, prompt: string) {
  const up = await call("upstream", prompt);
  const rs = await call("rust", prompt);
  const match = up.exit === rs.exit && up.sha === rs.sha && up.stderrSha === rs.stderrSha;
  match ? same++ : differ++;
  lines.push(
    [
      id,
      match ? "same" : "DIFFER",
      up.exit,
      rs.exit,
      up.bytes === 0 ? "unchanged" : "changed",
      rs.bytes === 0 ? "unchanged" : "changed",
      `${up.blocks}/${rs.blocks}`,
      up.sha.slice(0, 16),
      rs.sha.slice(0, 16),
    ].join("\t"),
  );
  return up;
}

const base = "carry out the unit brief";
const first = await probe("1-first-call-appends", base);
if (first.updated === null) throw new Error("first upstream call produced no updatedInput.prompt");
const block = first.updated.slice(base.length);
lines.push(`# appended block bytes: ${Buffer.byteLength(block, "utf-8")}`);
lines.push(`# block starts with: ${JSON.stringify(block.slice(0, 4))}`);
lines.push(`# block ends with: ${JSON.stringify(block.slice(-24))}`);

await probe("2-exact-block-present-suffix", first.updated);
await probe("3-exact-block-present-middle", `${block}\ntail text`);
await probe("4-block-with-one-byte-changed", base + block.replace("Active AI-DLC Rule Bundle", "Active AI-DLC Rule bundle"));
await probe("5-block-truncated-by-one-char", base + block.slice(0, -1));
await probe("6-two-copies-of-the-exact-block", base + block + block);
await probe("7-block-of-another-stage-is-not-this-bundle", base + block.replace("stage:code-generation", "stage:build-and-test"));
lines.push(`# same=${same} differ=${differ} total=${same + differ}`);
writeFileSync(process.argv[2], lines.join("\n") + "\n", "utf-8");
process.stdout.write(lines[lines.length - 1] + "\n");
