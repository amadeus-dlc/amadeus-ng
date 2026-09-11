import { test, expect, beforeAll, afterAll } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync, readFileSync, existsSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { capture } from "./capture-supplemental";

let fixture: string;
beforeAll(() => {
  fixture = mkdtempSync(join(tmpdir(), "source-fixture-"));
  const archive = Bun.spawnSync(["git", "-C", "vendor/aidlc-workflows", "archive", "a277af218f0df7f325d3b8be7b6d90fce2c5bd40", "dist/claude"]);
  expect(archive.exitCode).toBe(0);
  expect(Bun.spawnSync(["tar", "-xf", "-", "-C", fixture], { stdin: archive.stdout }).exitCode).toBe(0);
});

test("hash採取も固定元以外のモジュールを実行前に拒否する", () => {
  const root = mkdtempSync(join(tmpdir(), "hash-source-reject-"));
  try {
    writeFileSync(join(root, "meta.json"), JSON.stringify({ upstream_commit: "a277af218f0df7f325d3b8be7b6d90fce2c5bd40" }));
    writeFileSync(join(root, "snippet.ts"), 'throw new Error("実行された");');
    const result = Bun.spawnSync([process.execPath, "scripts/goldens/capture-hash-canonical.ts", join(root, "snippet.ts"), join(root, "out"), join(root, "meta.json")]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain("採取スニペットが不一致");
    expect(existsSync(join(root, "out"))).toBe(false);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
afterAll(() => rmSync(fixture, { recursive: true, force: true }));

test("CIの固定元取得でdepth1のforkから本家の祖先を採取できる", () => {
  const root = mkdtempSync(join(tmpdir(), "shallow-upstream-"));
  const source = resolve("vendor/aidlc-workflows");
  const repo = join(root, "vendor/aidlc-workflows");
  const pin = "a277af218f0df7f325d3b8be7b6d90fce2c5bd40";
  try {
    mkdirSync(repo, { recursive: true });
    expect(Bun.spawnSync(["git", "init", "-q", repo]).exitCode).toBe(0);
    expect(Bun.spawnSync(["git", "-C", repo, "fetch", "--depth=1", `file://${source}`, "801c570062f67dc8f4952ee5fc601381d09db7ec"]).exitCode).toBe(0);
    expect(Bun.spawnSync(["git", "-C", repo, "rev-parse", "--is-shallow-repository"]).stdout.toString().trim()).toBe("true");
    expect(Bun.spawnSync(["git", "-C", repo, "archive", pin, "dist/claude"]).exitCode).toBe(128);
    const workflow = Bun.YAML.parse(readFileSync(".github/workflows/ci.yml", "utf8")) as any;
    const step = workflow.jobs["aidlc-distribution"].steps.find((entry: any) => entry.name === "Fetch pinned upstream acceptance source");
    // 取得処理が存在しない現在のCIはここで何も準備しない。
    const command = (step?.run ?? "true").replace("https://github.com/awslabs/aidlc-workflows", '"$GOLDEN_TEST_SOURCE"');
    const setup = Bun.spawnSync(["bash", "-c", command], { cwd: root, env: { ...process.env, GOLDEN_TEST_SOURCE: `file://${source}` } });
    expect(setup.exitCode).toBe(0);
    const archive = Bun.spawnSync(["git", "-C", repo, "archive", pin, "dist/claude"]);
    expect(archive.exitCode).toBe(0);
    expect(Bun.spawnSync(["tar", "-tf", "-"], { stdin: archive.stdout }).stdout.toString()).toContain("dist/claude/.claude/tools/aidlc-version.ts");
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 30000);

test("固定した本家2.7.1の配布物で補完採取できる", () => {
  const root = mkdtempSync(join(tmpdir(), "source-test-"));
  try {
    const archive = Bun.spawnSync(["git", "-C", "vendor/aidlc-workflows", "archive", "a277af218f0df7f325d3b8be7b6d90fce2c5bd40", "dist/claude"]);
    expect(archive.exitCode).toBe(0);
    expect(Bun.spawnSync(["tar", "-xf", "-", "-C", root], { stdin: archive.stdout }).exitCode).toBe(0);
    expect(capture(join(root, "dist/claude")).upstream_commit).toBe("a277af218f0df7f325d3b8be7b6d90fce2c5bd40");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}, 30000);

test("採取CLIは来歴コミットの取り違えを出力前に拒否する", () => {
  const root = mkdtempSync(join(tmpdir(), "source-reject-"));
  try {
    writeFileSync(join(root, "meta.json"), JSON.stringify({ upstream_commit: "3c3146cfd7cef33020d48e8d48d4e80d0f8c2820" }));
    const result = Bun.spawnSync([process.execPath, "scripts/goldens/capture-cli.ts", join(fixture, "dist/claude"), join(root, "out"), join(root, "meta.json")]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toString()).toContain("採取元コミットが不一致");
    expect(existsSync(join(root, "out"))).toBe(false);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

for (const mutation of ["改変", "欠落", "追加", "取得先不在", "既存出力保持"]) {
  test(`採取入口が${mutation}を検出して保存先を壊さない`, () => {
    const root = mkdtempSync(join(tmpdir(), "source-invalid-"));
    const target = join(fixture, "dist/claude/.claude/tools/aidlc-version.ts");
    const original = readFileSync(target);
    const extra = join(fixture, "dist/claude/unexpected");
    try {
      const output = join(root, "output");
      mkdirSync(join(output, "cli"), { recursive: true });
      writeFileSync(join(output, "sentinel"), "既存データ");
      writeFileSync(join(output, "cli/previous.json"), '{"result":"preserved"}\n');
      writeFileSync(join(root, "meta.json"), JSON.stringify({ upstream_commit: "a277af218f0df7f325d3b8be7b6d90fce2c5bd40" }));
      if (mutation === "欠落") rmSync(target);
      else if (mutation === "追加") writeFileSync(extra, "追加");
      else if (mutation === "改変") writeFileSync(target, "改変");
      const result = Bun.spawnSync([process.execPath, "scripts/goldens/capture-cli.ts", mutation === "取得先不在" ? join(root, "absent") : join(fixture, "dist/claude"), output, join(root, "meta.json")]);
      expect(result.exitCode).not.toBe(0);
      expect(result.stderr.toString()).toContain(mutation === "取得先不在" ? "ENOENT" : mutation === "既存出力保持" ? "既存の採取結果は上書きしない" : "固定ピン");
      expect(readFileSync(join(output, "sentinel"), "utf8")).toBe("既存データ");
      expect(readFileSync(join(output, "cli/previous.json"), "utf8")).toBe('{"result":"preserved"}\n');
    } finally {
      writeFileSync(target, original);
      rmSync(extra, { force: true });
      rmSync(root, { recursive: true, force: true });
    }
  });
}
