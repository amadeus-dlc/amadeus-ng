#!/usr/bin/env bun
/**
 * U2 群 D (`aidlc-utility` の codekb-snapshot / codekb-publish) の実行出力ゴールデンの採取。
 *
 * **固定ピン `a277af21` の配布実バイトを走らせる**。vendor の作業ツリーは fork 側の後続
 * コミットを指しているので、`git archive a277af21 dist/claude/.claude` でピンの木を一時
 * ディレクトリへ展開し、そのツールを起動する。展開できなければ**失敗させる** — 作業ツリーへ
 * 暗黙に後退しない。展開物は採取のたびに捨てる。
 *
 * 入力は `tests/golden/codekb-write/fixture.json` の scenario 宣言で、Rust の CLI ゴールデン
 * テスト (`modules/app/aidlc/tests/codekb_write_golden.rs`) は**同じ宣言から同じツリーを建てる**。
 * git 指紋 (`git write-tree`) も木のハッシュ (sha256) も内容アドレスであり、作業ディレクトリの
 * 絶対パスに依存しないので、両者が同じ内容を書けば同じ値になる。
 *
 * `codekb-publish` は compare-and-swap の合言葉を要るので、`snapshot_paths` を持つケースは
 * 先に同じワークスペースで `codekb-snapshot --json` を走らせ、その `store_generation` /
 * `source_fingerprint` を argv の `{store_generation}` / `{source_fingerprint}` へ差し込む。
 * 採取側と Rust 側が同じ手順を踏むので、合言葉を定数で焼き込まずに済む。
 *
 * ゴールデンは `tests/golden/codekb-write/cli/<id>/` の下に `stdout` (生バイト) と
 * `meta.json` ({argv, exit_code}) として書く。**採取結果は手修正しない**。
 *
 *   bun scripts/goldens/capture-codekb-write.ts
 */

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const REPO_ROOT = join(dirname(new URL(import.meta.url).pathname), "..", "..");
const VENDOR = join(REPO_ROOT, "vendor/aidlc-workflows");
const FIXTURE = join(REPO_ROOT, "tests/golden/codekb-write/fixture.json");
const OUT = join(REPO_ROOT, "tests/golden/codekb-write/cli");

const UPSTREAM_COMMIT = "a277af218f0df7f325d3b8be7b6d90fce2c5bd40";

type Scenario = {
  workspace_name: string;
  git_repos: string[];
  files: Record<string, string>;
  symlinks?: Record<string, string>;
};
type Case = { id: string; scenario: string; argv: string[]; snapshot_paths?: string };
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
    rmSync(root, { recursive: true, force: true });
    throw new Error(`ピン ${UPSTREAM_COMMIT} の配布物を取り出せない: ${archive.stderr.toString()}`);
  }
  const untar = spawnSync("tar", ["-x", "-C", root], { input: archive.stdout });
  if (untar.status !== 0) {
    rmSync(root, { recursive: true, force: true });
    throw new Error("配布物の展開に失敗した");
  }
  return { tools: join(root, "dist/claude/.claude/tools"), root };
}

/** scenario 宣言から使い捨てワークスペースを建てる。 */
function buildWorkspace(scenario: Scenario): string {
  const parent = mkdtempSync(join(tmpdir(), "aidlc-codekb-write-"));
  const workspace = join(parent, scenario.workspace_name);
  mkdirSync(workspace, { recursive: true });
  for (const [relative, body] of Object.entries(scenario.files)) {
    const path = join(workspace, relative);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, body);
  }
  for (const [relative, target] of Object.entries(scenario.symlinks ?? {})) {
    const path = join(workspace, relative);
    mkdirSync(dirname(path), { recursive: true });
    symlinkSync(target, path);
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
const TOOL = join(tools, "aidlc-utility.ts");

function run(workspace: string, argv: string[]) {
  return spawnSync("bun", [TOOL, ...argv, "--project-dir", workspace], {
    cwd: workspace,
    encoding: "buffer",
  });
}

rmSync(OUT, { recursive: true, force: true });

const observed = new Map<string, { stdout: string; exit: number | null }>();

try {
  for (const testCase of fixture.cases) {
    const scenario = fixture.scenarios[testCase.scenario];
    if (!scenario) throw new Error(`未知の scenario: ${testCase.scenario}`);
    const workspace = buildWorkspace(scenario);

    // compare-and-swap の合言葉は、同じワークスペースの snapshot から採る。
    let argv = testCase.argv;
    if (testCase.snapshot_paths !== undefined) {
      const snapshot = run(workspace, ["codekb-snapshot", "--paths", testCase.snapshot_paths, "--json"]);
      if (snapshot.status !== 0) {
        throw new Error(`${testCase.id}: 先行 snapshot が失敗した: ${snapshot.stderr.toString()}`);
      }
      const taken = JSON.parse(snapshot.stdout.toString("utf-8")) as {
        store_generation: string;
        source_fingerprint: string;
      };
      argv = argv.map((arg) =>
        arg
          .replaceAll("{store_generation}", taken.store_generation)
          .replaceAll("{source_fingerprint}", taken.source_fingerprint),
      );
    }

    const result = run(workspace, argv);
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
          // 宣言のままの argv を残す — Rust 側が同じ差し込みを行う。
          argv: testCase.argv,
          ...(testCase.snapshot_paths === undefined
            ? {}
            : { snapshot_paths: testCase.snapshot_paths }),
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
    process.stdout.write(`  ${testCase.id}  exit=${result.status}  ${result.stdout.length}B\n`);
  }
} finally {
  rmSync(pinRoot, { recursive: true, force: true });
}

// --- 採取そのものの健全性検査 -------------------------------------------------
//
// フィクスチャは鮮度印に指紋を literal で焼き込んでいる。開発者の git 設定や git 本体の版差で
// その値がずれると、CURRENT のつもりの候補が黙って CODEKB_CANDIDATE_STALE を採ってしまい、
// 「upstream と一致した」という誤った証拠が残る。採取の場で突き合わせて落とす。
function expect(id: string, predicate: (seen: { stdout: string; exit: number | null }) => boolean, what: string): void {
  const seen = observed.get(id);
  if (!seen) throw new Error(`検査対象のケースが無い: ${id}`);
  if (!predicate(seen)) {
    throw new Error(`${id}: ${what} を満たさない — 採取: exit=${seen.exit} stdout=${JSON.stringify(seen.stdout)}`);
  }
}

expect("snapshot/no-store-plain", (s) =>
  s.exit === 0 && s.stdout === "STORE_GENERATION none\nSOURCE_FINGERPRINT git:3c55f6af3e88b8f5f11eeb1eafeeb84095e998db\nSOURCE_PATHS src/\n",
  "ストア不在の 3 行");
expect("snapshot/with-store-plain", (s) => s.exit === 0 && s.stdout.startsWith("STORE_GENERATION sha256:"), "既存ストアの世代");
expect("snapshot/non-git-json", (s) => s.exit === 0 && s.stdout.includes('"source_fingerprint":"tree:'), "非 git は tree: へ後退");
expect("snapshot/deduplicates-paths", (s) => s.exit === 0 && s.stdout.includes('"paths":["src/"]'), "重複した --paths を畳む");
expect("publish/happy-plain", (s) => s.exit === 0 && /^PUBLISHED \S+ sha256:[0-9a-f]{64}\n$/.test(s.stdout), "公開の 1 行");
expect("publish/over-existing-store", (s) => s.exit === 0 && s.stdout.startsWith("PUBLISHED "), "既存ストアへの上書き公開");
for (const refused of [
  "snapshot/missing-paths", "snapshot/unmatched-path", "snapshot/invalid-repo",
  "publish/store-changed", "publish/source-changed", "publish/candidate-stale",
  "publish/missing-expect", "publish/missing-paths", "publish/missing-staged",
  "publish/staged-absent-dir", "publish/staged-incomplete", "publish/staged-extra-symlink",
  "publish/staged-outside", "publish/scope-not-covered",
]) {
  expect(refused, (s) => s.exit === 1 && s.stdout === "", "拒否は stdout 空・exit 1");
}

process.stdout.write(`採取 ${fixture.cases.length} ケース -> ${OUT}\n`);
