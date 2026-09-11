#!/usr/bin/env bun
// 着地確認（closeout）の差分駆動 — 本家 2.7.1 固定コミット a277af21 の実バイト（closeout-workspaces.sh が
// `git archive` で CLOSEOUT_ROOT/pinned へ展開し、各ワークスペースの `.claude/` へ写したもの）と
// 本 build（AIDLC_BIN）を、**同じ stdin・同じ環境・同じ project dir** で実プロセスとして起動し、
// exit / stdout / stderr を**バイトで**突き合わせる。
//
// 前担当の differential-driver.ts との違い:
//   1. 本家は本リポジトリの `.claude/hooks/` ではなく、固定コミットから展開した写しを走らせる。
//      project dir に `.claude/hooks/aidlc-deliver-stage-rules.ts` が在ればそれを、無ければ
//      CLOSEOUT_ROOT/ws の写しを使う（本家はグラフ・ペルソナをスクリプト相対で読むため、
//      後者は「project dir に配布物が無い」場面の観測になる）。
//   2. stderr も突き合わせる（前担当は exit + stdout だけを照合し、stderr は表示のみだった）。
//   3. project dir は CLOSEOUT_ROOT 配下の一時ワークスペースで、本リポジトリには何も書かない。
//      前担当の corpus が本リポジトリを指していた `cwd` / `AIDLC_PROJECT_DIR` は次のとおり写す:
//        <repo>                                     → ws
//        <repo>/modules                             → norules
//        <repo>/vendor/aidlc-workflows/dist/claude  → stateless
//   4. 環境は PATH / HOME(= CLOSEOUT_ROOT/home) / TMPDIR と case の env だけにし、
//      この会話由来の CLAUDE_* / AIDLC_* を両側とも持ち込まない。両側へ同じ環境を渡す
//      （本 build は AIDLC_PROJECT_DIR を読まず、作業ディレクトリを project dir にする）。
//
// 使い方: CLOSEOUT_ROOT=<dir> AIDLC_BIN=<repo>/target/debug/aidlc \
//         bun closeout-differential.ts <out.tsv> <detail.json> <corpus.json>...

import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = process.env.CLOSEOUT_ROOT;
const BIN = process.env.AIDLC_BIN;
if (!ROOT || !BIN) {
  process.stderr.write("CLOSEOUT_ROOT and AIDLC_BIN are required\n");
  process.exit(1);
}

type Case = {
  id: string;
  project?: string;
  input?: unknown;
  raw?: string;
  env?: Record<string, string>;
};

type Side = {
  exit: number;
  stdout: Uint8Array;
  stderr: Uint8Array;
};

type Observation = {
  exit: number;
  bytes: number;
  sha256: string;
  stderrSha256: string;
  stderr: string;
  bundle: string;
  stage: string;
  blocks: number;
  keys: string;
};

const sha = (bytes: Uint8Array) =>
  bytes.length === 0 ? "-" : createHash("sha256").update(bytes).digest("hex");

function observe(side: Side): Observation {
  const stdout = new TextDecoder().decode(side.stdout);
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
    exit: side.exit,
    bytes: side.stdout.length,
    sha256: sha(side.stdout),
    stderrSha256: sha(side.stderr),
    stderr: new TextDecoder().decode(side.stderr),
    bundle: marker ? marker[1] : "-",
    stage: marker ? marker[2] : "-",
    blocks: (stdout.match(/AIDLC_DISPATCH_RULES_BEGIN/g) ?? []).length,
    keys,
  };
}

async function spawn(
  command: string[],
  stdin: Uint8Array,
  env: Record<string, string>,
  cwd: string,
): Promise<Side> {
  const proc = Bun.spawn(command, { stdin, stdout: "pipe", stderr: "pipe", env, cwd });
  const stdout = new Uint8Array(await new Response(proc.stdout).arrayBuffer());
  const stderr = new Uint8Array(await new Response(proc.stderr).arrayBuffer());
  return { exit: await proc.exited, stdout, stderr };
}

function projectOf(entry: Case): string {
  if (entry.project) return entry.project;
  const dir = entry.env?.AIDLC_PROJECT_DIR;
  if (!dir) return "ws";
  if (dir.endsWith("/modules")) return "norules";
  if (dir.endsWith("/vendor/aidlc-workflows/dist/claude")) return "stateless";
  throw new Error(`unmapped AIDLC_PROJECT_DIR for ${entry.id}: ${dir}`);
}

function hookFor(projectDir: string): string {
  const local = join(projectDir, ".claude/hooks/aidlc-deliver-stage-rules.ts");
  return existsSync(local) ? local : join(ROOT!, "ws/.claude/hooks/aidlc-deliver-stage-rules.ts");
}

function stdinOf(entry: Case, projectDir: string): Uint8Array {
  if (entry.raw !== undefined) return new TextEncoder().encode(entry.raw);
  const input = entry.input;
  if (input && typeof input === "object" && "cwd" in (input as Record<string, unknown>)) {
    return new TextEncoder().encode(JSON.stringify({ ...(input as Record<string, unknown>), cwd: projectDir }));
  }
  return new TextEncoder().encode(JSON.stringify(input));
}

function envOf(entry: Case, projectDir: string): Record<string, string> {
  const env: Record<string, string> = {
    PATH: process.env.PATH ?? "/usr/bin:/bin",
    HOME: join(ROOT!, "home"),
    AIDLC_PROJECT_DIR: projectDir,
  };
  if (process.env.TMPDIR) env.TMPDIR = process.env.TMPDIR;
  for (const [key, value] of Object.entries(entry.env ?? {})) {
    if (key === "AIDLC_PROJECT_DIR") continue;
    env[key] = value.replaceAll("{ROOT}", ROOT!);
  }
  return env;
}

async function runCase(entry: Case) {
  const project = projectOf(entry);
  const projectDir = join(ROOT!, project);
  const stdin = stdinOf(entry, projectDir);
  const env = envOf(entry, projectDir);
  const hook = hookFor(projectDir);
  const upstream = await spawn([process.execPath, hook], stdin, env, projectDir);
  const rust = await spawn([BIN!, "hook", "deliver-stage-rules"], stdin, env, projectDir);
  return { project, projectDir, hook, stdin: new TextDecoder().decode(stdin), env, upstream, rust };
}

const [outTsv, outDetail, ...corpora] = process.argv.slice(2);
if (!outTsv || !outDetail || corpora.length === 0) {
  process.stderr.write("usage: bun closeout-differential.ts <out.tsv> <detail.json> <corpus.json>...\n");
  process.exit(1);
}

const lines: string[] = [
  "id\tproject\tmatch\tup_exit\trs_exit\tup_bytes\trs_bytes\tup_sha256\trs_sha256\tbundle(up/rs)\tstage(up/rs)\tblocks\tkeys\tup_stderr\trs_stderr",
];
const detail: unknown[] = [];
let same = 0;
let stderrOnly = 0;
let differ = 0;
for (const corpus of corpora) {
  const cases = JSON.parse(readFileSync(corpus, "utf-8")) as Case[];
  for (const entry of cases) {
    const run = await runCase(entry);
    const up = observe(run.upstream);
    const rs = observe(run.rust);
    const exitAndStdout = up.exit === rs.exit && up.sha256 === rs.sha256;
    const stderrSame = up.stderrSha256 === rs.stderrSha256;
    const match = exitAndStdout && stderrSame ? "same" : exitAndStdout ? "stderr-only" : "DIFFER";
    if (match === "same") same++;
    else if (match === "stderr-only") stderrOnly++;
    else differ++;
    const squash = (text: string) => text.trim().replace(/\s+/g, " ").slice(0, 120) || "-";
    lines.push(
      [
        entry.id,
        run.project,
        match,
        up.exit,
        rs.exit,
        up.bytes,
        rs.bytes,
        up.sha256.slice(0, 16),
        rs.sha256.slice(0, 16),
        `${up.bundle.slice(0, 16)}/${rs.bundle.slice(0, 16)}`,
        `${up.stage}/${rs.stage}`,
        `${up.blocks}/${rs.blocks}`,
        `${up.keys}/${rs.keys}`,
        squash(up.stderr),
        squash(rs.stderr),
      ].join("\t"),
    );
    if (match !== "same") {
      detail.push({
        id: entry.id,
        match,
        project: run.project,
        projectDir: run.projectDir,
        hook: run.hook,
        env: run.env,
        stdin: run.stdin,
        upstream: { exit: up.exit, stdout_bytes: up.bytes, stdout_sha256: up.sha256, stderr: up.stderr },
        rust: { exit: rs.exit, stdout_bytes: rs.bytes, stdout_sha256: rs.sha256, stderr: rs.stderr },
      });
    }
  }
}
lines.push(`# same=${same} stderr-only=${stderrOnly} differ=${differ} total=${same + stderrOnly + differ}`);
writeFileSync(outTsv, lines.join("\n") + "\n", "utf-8");
writeFileSync(outDetail, JSON.stringify(detail, null, 2) + "\n", "utf-8");
process.stdout.write(lines[lines.length - 1] + "\n");
