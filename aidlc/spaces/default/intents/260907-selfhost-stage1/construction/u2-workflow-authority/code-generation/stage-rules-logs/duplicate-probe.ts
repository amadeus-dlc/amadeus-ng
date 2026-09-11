#!/usr/bin/env bun
// 本家 `hasExactBundle` の重複抑止を実走行で観測する。
//
// 1 回目の出力から実際に付いたブロックを取り出し、それを含む brief をもう一度食わせて
// 「変化なし」になることを確かめる。1 バイト変えた場合・途中に置いた場合・別 stage の
// ブロックだった場合も同じ条件で観測する。読取り専用。

import { createHash } from "node:crypto";

const HOOK = ".claude/hooks/aidlc-deliver-stage-rules.ts";
const PROJECT = "/Users/j5ik2o/orca/workspaces/amadeus-ng/stage1";

async function call(prompt: string) {
  const proc = Bun.spawn(["bun", HOOK], {
    stdin: new TextEncoder().encode(
      JSON.stringify({
        cwd: PROJECT,
        tool_name: "Task",
        tool_input: { subagent_type: "aidlc-developer-agent", prompt },
      }),
    ),
    stdout: "pipe",
    stderr: "pipe",
  });
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  const exit = await proc.exited;
  let updated: string | null = null;
  try {
    updated = JSON.parse(stdout)?.hookSpecificOutput?.updatedInput?.prompt ?? null;
  } catch {
    updated = null;
  }
  const blocks = (stdout.match(/AIDLC_DISPATCH_RULES_BEGIN/g) ?? []).length;
  return { exit, stdout, stderr, updated, blocks };
}

function report(id: string, r: Awaited<ReturnType<typeof call>>) {
  process.stdout.write(
    [
      id,
      String(r.exit),
      r.stdout.length === 0 ? "unchanged" : "changed",
      String(r.blocks),
      r.stdout.length === 0
        ? "-"
        : createHash("sha256").update(r.stdout, "utf-8").digest("hex").slice(0, 16),
      r.stderr.trim().replace(/\s+/g, " ").slice(0, 120) || "-",
    ].join("\t") + "\n",
  );
}

const base = "carry out the unit brief";
const first = await call(base);
report("1-first-call-appends", first);
if (first.updated === null) throw new Error("first call produced no updatedInput.prompt");

const block = first.updated.slice(base.length);
process.stdout.write(`# appended block bytes: ${Buffer.byteLength(block, "utf-8")}\n`);
process.stdout.write(
  `# block starts with two newlines: ${JSON.stringify(block.slice(0, 4))}\n`,
);
process.stdout.write(`# block ends with: ${JSON.stringify(block.slice(-24))}\n`);

report("2-exact-block-present-suffix", await call(first.updated));
report("3-exact-block-present-middle", await call(`${block}\ntail text`));
report("4-block-with-one-byte-changed", await call(base + block.replace("Active AI-DLC Rule Bundle", "Active AI-DLC Rule bundle")));
report("5-block-truncated-by-one-char", await call(base + block.slice(0, -1)));
report("6-two-copies-of-the-exact-block", await call(base + block + block));
