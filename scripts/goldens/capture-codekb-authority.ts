#!/usr/bin/env bun
/**
 * U2 群 B/C (`aidlc-utility` の project-description / codekb-path / codekb-scope-diff) の
 * 実行出力ゴールデンの採取。
 *
 * **固定ピン `a277af21` の配布実バイトを走らせる**。vendor の作業ツリーは fork 側の後続
 * コミット (`801c5700`) を指しているので、そこを直に実行すると「ピンから採った」と言えない。
 * ここでは `git archive a277af21 dist/claude/.claude` で**ピンの木を一時ディレクトリへ展開**し、
 * そのツールを起動する。展開物は採取のたびに捨てる。
 *
 * 入力は `tests/golden/codekb-authority/fixture.json` の scenario 宣言で、Rust の CLI ゴールデン
 * テスト (`modules/app/aidlc/tests/codekb_authority_golden.rs`) は**同じ宣言から同じツリーを
 * 建てる**。git 指紋 (`git write-tree`) は内容アドレスであり作業ディレクトリの絶対パスに
 * 依存しないので、両者が同じ内容を書けば同じ値になる。
 *
 * 決定化: git リポジトリを建てたら repo-local な `core.excludesFile=/dev/null` と
 * `core.autocrlf=false` を据える。開発者の global gitignore が `git add -A` の結果を変えると
 * 指紋が環境依存になるためで、Rust 側の組立ても同じ 2 つを据える。
 *
 * ゴールデンは `tests/golden/codekb-authority/cli/<id>/` の下に `stdout` (生バイト) と
 * `meta.json` ({argv, exit_code}) として書く。**採取結果は手修正しない**。
 *
 *   bun scripts/goldens/capture-codekb-authority.ts
 */

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const REPO_ROOT = join(dirname(new URL(import.meta.url).pathname), "..", "..");
const VENDOR = join(REPO_ROOT, "vendor/aidlc-workflows");
const FIXTURE = join(REPO_ROOT, "tests/golden/codekb-authority/fixture.json");
const OUT = join(REPO_ROOT, "tests/golden/codekb-authority/cli");

const UPSTREAM_COMMIT = "a277af218f0df7f325d3b8be7b6d90fce2c5bd40";

type Scenario = {
  workspace_name: string;
  git_repos: string[];
  files: Record<string, string>;
};
type Case = { id: string; scenario: string; argv: string[] };
type Fixture = { scenarios: Record<string, Scenario>; cases: Case[] };

const fixture: Fixture = JSON.parse(readFileSync(FIXTURE, "utf-8"));

/** ピンの配布ツリーを一時ディレクトリへ展開し、tools ディレクトリを返す。 */
function materializePinnedDistribution(): { tools: string; root: string } {
  const root = mkdtempSync(join(tmpdir(), "aidlc-pin-"));
  const archive = spawnSync(
    "git",
    ["-C", VENDOR, "archive", UPSTREAM_COMMIT, "dist/claude/.claude"],
    { encoding: "buffer", maxBuffer: 512 * 1024 * 1024 },
  );
  if (archive.status !== 0) {
    throw new Error(
      `ピン ${UPSTREAM_COMMIT} の配布物を取り出せない: ${archive.stderr.toString()}`,
    );
  }
  const untar = spawnSync("tar", ["-x", "-C", root], { input: archive.stdout });
  if (untar.status !== 0) throw new Error("配布物の展開に失敗した");
  return { tools: join(root, "dist/claude/.claude/tools"), root };
}

/** scenario 宣言から使い捨てワークスペースを建てる。 */
function buildWorkspace(scenario: Scenario): string {
  const parent = mkdtempSync(join(tmpdir(), "aidlc-codekb-"));
  const workspace = join(parent, scenario.workspace_name);
  mkdirSync(workspace, { recursive: true });
  for (const [relative, body] of Object.entries(scenario.files)) {
    const path = join(workspace, relative);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, body);
  }
  for (const repo of scenario.git_repos) {
    const dir = join(workspace, repo);
    for (const args of [
      ["init", "-q", "--initial-branch=main"],
      // 決定化: 開発者の global gitignore と autocrlf を repo-local で無効化する。
      ["config", "core.excludesFile", "/dev/null"],
      ["config", "core.autocrlf", "false"],
      ["add", "-A"],
    ]) {
      const result = spawnSync("git", ["-C", dir, ...args], { encoding: "utf-8" });
      if (result.status !== 0) {
        throw new Error(`git ${args.join(" ")} に失敗した (${dir}): ${result.stderr}`);
      }
    }
  }
  return workspace;
}

const { tools, root: pinRoot } = materializePinnedDistribution();
rmSync(OUT, { recursive: true, force: true });

const observed = new Map<string, { stdout: string; exit: number | null }>();

for (const testCase of fixture.cases) {
  const scenario = fixture.scenarios[testCase.scenario];
  if (!scenario) throw new Error(`未知の scenario: ${testCase.scenario}`);
  const workspace = buildWorkspace(scenario);
  const argv = testCase.argv.map((arg) => arg.replaceAll("{workspace}", workspace));

  const result = spawnSync(
    "bun",
    [join(tools, "aidlc-utility.ts"), ...argv, "--project-dir", workspace],
    { cwd: workspace, encoding: "buffer" },
  );
  if (result.error) throw result.error;

  const dir = join(OUT, testCase.id);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "stdout"), result.stdout);
  writeFileSync(
    join(dir, "meta.json"),
    `${JSON.stringify(
      {
        upstream_commit: UPSTREAM_COMMIT,
        tool: "aidlc-utility.ts",
        scenario: testCase.scenario,
        argv: testCase.argv,
        exit_code: result.status,
      },
      null,
      2,
    )}\n`,
  );
  observed.set(testCase.id, {
    stdout: result.stdout.toString("utf-8"),
    exit: result.status,
  });
  process.stdout.write(
    `  ${testCase.id}  exit=${result.status}  ${result.stdout.length}B\n`,
  );
}

rmSync(pinRoot, { recursive: true, force: true });

// --- 採取そのものの健全性検査 -------------------------------------------------
//
// フィクスチャは CURRENT の scenario に指紋を**literal で**焼き込んでいる。開発者の git 設定や
// git 本体の版差でその値がずれると、CURRENT のつもりの scenario が黙って STALE を採ってしまい、
// 「upstream と一致した」という誤った証拠が残る。採取の場で突き合わせて落とす。
function assertVerdict(id: string, expected: string): void {
  const seen = observed.get(id);
  if (!seen) throw new Error(`検査対象のケースが無い: ${id}`);
  if (!seen.stdout.startsWith(expected)) {
    throw new Error(
      `${id}: ${expected} を採るはずが別の判定になった — フィクスチャの指紋定数が現在の git と合っていない。\n採取した出力: ${seen.stdout}`,
    );
  }
}
assertVerdict("scope-diff/current", "CURRENT");
assertVerdict("scope-diff/stale", "STALE");
assertVerdict("scope-diff/no-store", "NO_STORE");
assertVerdict("scope-diff/unverified-no-fingerprint", "UNVERIFIED");
assertVerdict("scope-diff/unknown-scope-absent", "UNKNOWN_SCOPE");
assertVerdict("scope-diff/compare-covers", "COVERS");
assertVerdict("scope-diff/compare-narrower", "NARROWER");

const mint = observed.get("scope-diff/mint");
if (!mint || !/^[0-9a-f]{40}\n$/.test(mint.stdout)) {
  throw new Error(`scope-diff/mint が 40 桁 hex を返していない: ${mint?.stdout}`);
}

process.stdout.write(`採取 ${fixture.cases.length} ケース -> ${OUT}\n`);
