#!/usr/bin/env bun
// 本家 2.7.1 `hooks/aidlc-deliver-stage-rules.ts` と本 build の `aidlc hook
// deliver-stage-rules` を、同じ入力で**両方とも実プロセスとして起動**し、
// exit / stdout / stderr / updatedInput を 1 行 1 件で突き合わせる。
//
// 読取り専用: 本リポジトリのワークスペースをそのまま projectDir として使い、
// `run_in_background: true` は corpus に入れない（inflight 更新を起こさないため）。
//
// 本家は projectDir を `AIDLC_PROJECT_DIR` から取り、本 build は**プロセスの作業
// ディレクトリ**から取る（この build の全フックに共通の既定）。同じ場所を指すよう、
// ケースの `env.AIDLC_PROJECT_DIR` は Rust 側では cwd へ写す。
//
// 使い方: bun <このファイル> <corpus.json> [--rust-only|--upstream-only]

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const HOOK = ".claude/hooks/aidlc-deliver-stage-rules.ts";
const RUST = "target/debug/aidlc";
const REPO = process.cwd();

// `raw` は stdin へそのまま流すバイト列（不正 JSON の観測用）。
// 無ければ `input` を JSON.stringify したものを流す。
type Case = {
  id: string;
  input: unknown;
  raw?: string;
  env?: Record<string, string>;
};

type Observation = {
  exit: number;
  bytes: number;
  sha256: string;
  bundle: string;
  stage: string;
  blocks: number;
  keys: string;
  stderr: string;
};

function observe(stdout: string, stderr: string, exit: number): Observation {
  const marker = stdout.match(
    /AIDLC_DISPATCH_RULES_BEGIN sha256:([0-9a-f]{64}) stage:([a-z0-9-]+)/,
  );
  let keys = "-";
  try {
    const updated = JSON.parse(stdout)?.hookSpecificOutput?.updatedInput;
    if (updated && typeof updated === "object") keys = Object.keys(updated).join("|");
  } catch {
    // 出力なし・非 JSON はそのまま `-`。
  }
  return {
    exit,
    bytes: Buffer.byteLength(stdout, "utf-8"),
    sha256: stdout.length === 0 ? "-" : createHash("sha256").update(stdout, "utf-8").digest("hex"),
    bundle: marker ? marker[1] : "-",
    stage: marker ? marker[2] : "-",
    blocks: (stdout.match(/AIDLC_DISPATCH_RULES_BEGIN/g) ?? []).length,
    keys,
    stderr: stderr.trim().replace(/\s+/g, " "),
  };
}

async function spawn(
  command: string[],
  stdin: Uint8Array,
  env: Record<string, string | undefined>,
  cwd: string,
): Promise<Observation> {
  const proc = Bun.spawn(command, { stdin, stdout: "pipe", stderr: "pipe", env, cwd });
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return observe(stdout, stderr, await proc.exited);
}

async function runCase(entry: Case, sides: "both" | "rust" | "upstream") {
  const stdin = new TextEncoder().encode(entry.raw ?? JSON.stringify(entry.input));
  const projectDir = entry.env?.AIDLC_PROJECT_DIR ?? REPO;

  const upstream =
    sides === "rust"
      ? null
      : await spawn([ "bun", HOOK ], stdin, { ...process.env, ...(entry.env ?? {}) }, REPO);

  // Rust 側は AIDLC_PROJECT_DIR を読まないので、同じ場所を cwd として与える。
  const rustEnv = { ...process.env, ...(entry.env ?? {}) };
  delete rustEnv.AIDLC_PROJECT_DIR;
  const rust =
    sides === "upstream"
      ? null
      : await spawn([`${REPO}/${RUST}`, "hook", "deliver-stage-rules"], stdin, rustEnv, projectDir);

  return { upstream, rust };
}

const corpusPath = process.argv[2];
const flag = process.argv[3] ?? "";
const sides = flag === "--rust-only" ? "rust" : flag === "--upstream-only" ? "upstream" : "both";
if (!corpusPath) {
  process.stderr.write("usage: bun differential-driver.ts <corpus.json> [--rust-only|--upstream-only]\n");
  process.exit(1);
}
const corpus = JSON.parse(readFileSync(corpusPath, "utf-8")) as Case[];

const columns = (o: Observation) =>
  [o.exit, o.bytes, o.sha256, o.bundle, o.stage, o.blocks, o.keys, o.stderr.slice(0, 160) || "-"];

if (sides === "both") {
  process.stdout.write("id\tmatch\tup_exit\trs_exit\tup_bytes\trs_bytes\tup_sha256\trs_sha256\tstage\tblocks\tkeys\tup_stderr\trs_stderr\n");
  let same = 0;
  let differ = 0;
  for (const entry of corpus) {
    const { upstream, rust } = await runCase(entry, sides);
    if (!upstream || !rust) continue;
    // stderr は Node と Rust で綴りが違いうるので、突き合わせは exit と stdout で行う。
    const matched = upstream.exit === rust.exit && upstream.sha256 === rust.sha256;
    matched ? same++ : differ++;
    process.stdout.write(
      [
        entry.id,
        matched ? "same" : "DIFFER",
        upstream.exit,
        rust.exit,
        upstream.bytes,
        rust.bytes,
        upstream.sha256.slice(0, 16),
        rust.sha256.slice(0, 16),
        `${upstream.stage}/${rust.stage}`,
        `${upstream.blocks}/${rust.blocks}`,
        `${upstream.keys}/${rust.keys}`,
        upstream.stderr.slice(0, 120) || "-",
        rust.stderr.slice(0, 120) || "-",
      ].join("\t") + "\n",
    );
  }
  process.stdout.write(`# same=${same} differ=${differ} total=${same + differ}\n`);
} else {
  process.stdout.write("id\texit\tstdout_bytes\tstdout_sha256\tbundle_sha256\tstage\tblocks\tupdated_keys\tstderr\n");
  for (const entry of corpus) {
    const { upstream, rust } = await runCase(entry, sides);
    const side = sides === "rust" ? rust : upstream;
    if (!side) continue;
    process.stdout.write([entry.id, ...columns(side)].join("\t") + "\n");
  }
}
