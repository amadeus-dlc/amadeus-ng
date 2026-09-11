#!/usr/bin/env bun
// 512KiB 境界（本家 `DISPATCH_HOOK_OUTPUT_MAX_BYTES = 512 * 1024`）を実走行で観測する。
//
// 一時ディレクトリに規則ファイルを作り、`AIDLC_RULES_DIR` でそこを読ませる。
// org.md の本文長を二分探索して stdout がちょうど 524288 バイトになる長さを求め、
// その 1 つ手前・ちょうど・1 つ先の 3 点と、`AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK=1`
// を付けた場合を観測する。一時ファイルは終了時に消す。

import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const HOOK = ".claude/hooks/aidlc-deliver-stage-rules.ts";
const STATELESS_PROJECT = `${process.cwd()}/vendor/aidlc-workflows/dist/claude`;
const LIMIT = 512 * 1024;

const dir = mkdtempSync(join(tmpdir(), "aidlc-oversize-"));
mkdirSync(join(dir, "phases"), { recursive: true });
writeFileSync(join(dir, "team.md"), "# Team\n", "utf-8");
writeFileSync(join(dir, "project.md"), "# Project\n", "utf-8");
writeFileSync(join(dir, "phases", "construction.md"), "# Construction\n", "utf-8");

async function measure(bodyLength: number, fallback: boolean) {
  writeFileSync(join(dir, "org.md"), `# Org\n${"x".repeat(bodyLength)}\n`, "utf-8");
  const env: Record<string, string | undefined> = {
    ...process.env,
    AIDLC_RULES_DIR: dir,
    AIDLC_PROJECT_DIR: STATELESS_PROJECT,
  };
  if (fallback) env.AIDLC_DISPATCH_RULES_PRELOAD_FALLBACK = "1";
  const proc = Bun.spawn(["bun", HOOK], {
    stdin: new TextEncoder().encode(
      JSON.stringify({
        tool_name: "Task",
        tool_input: {
          subagent_type: "aidlc-developer-agent",
          prompt: "B /stages/construction/code-generation.md",
        },
      }),
    ),
    stdout: "pipe",
    stderr: "pipe",
    env,
  });
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  const exit = await proc.exited;
  return { exit, bytes: Buffer.byteLength(stdout, "utf-8"), stderr: stderr.trim() };
}

// 出力バイト数は本文長に対して 1 対 1 で単調増加する（"x" は 1 バイトで、
// エスケープも起きない）。まず 1 バイトぶんの傾きと切片を採る。
const a = await measure(1000, false);
const b = await measure(2000, false);
const slope = (b.bytes - a.bytes) / 1000;
const exact = 1000 + Math.round((LIMIT - a.bytes) / slope);

process.stdout.write("case\tbody_len\texit\tstdout_bytes\tstderr\n");
for (const [label, len] of [
  ["under-by-one", exact - 1],
  ["exactly-at-the-limit", exact],
  ["over-by-one", exact + 1],
] as const) {
  const r = await measure(len, false);
  process.stdout.write(
    `${label}\t${len}\t${r.exit}\t${r.bytes}\t${r.stderr.replace(/\s+/g, " ") || "-"}\n`,
  );
}
for (const [label, len] of [
  ["fallback-exactly-at-the-limit", exact],
  ["fallback-over-by-one", exact + 1],
] as const) {
  const r = await measure(len, true);
  process.stdout.write(
    `${label}\t${len}\t${r.exit}\t${r.bytes}\t${r.stderr.replace(/\s+/g, " ") || "-"}\n`,
  );
}
rmSync(dir, { recursive: true, force: true });
