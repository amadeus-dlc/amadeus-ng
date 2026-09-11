#!/usr/bin/env bun
/** 固定本家2.7.1のnext入口を、状態のない独立workspaceで実行する。 */
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { UPSTREAM, verifySource } from "./upstream-source";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
const inputs = [
  ["intent"], ["intent", "list"], ["intent", "list", "--status"],
  ["intent", "list", "--json"], ["intent", "--json"],
  ["intent", "help"], ["intent", "-h"], ["intent", "switch"],
  ["intent", "switch", "create"], ["intent", "sample name"],
  ["intent", "create", "--scope", "bugfix", "--arguments", "repair user's intent"],
  ["space"], ["space", "list", "--json"], ["space", "teamB"],
  ["space", "switch", "create"], ["space", "create", "team A"],
  ["space", "create"], ["space-create", "team A"], ["space-create"],
  ["intent", "archive"], ["space", "rename"],
];
const observations = inputs.map((args, index) => {
  const parent = mkdtempSync(join(tmpdir(), "aidlc-next-input-"));
  const workspace = join(parent, "workspace");
  const home = join(parent, "home");
  mkdirSync(workspace);
  mkdirSync(home);
  try {
    const result = Bun.spawnSync([process.execPath, join(resolve(dist), ".claude/tools/aidlc-orchestrate.ts"), "next", ...args], {
      cwd: workspace,
      env: { PATH: "/usr/bin:/bin", HOME: home },
    });
    assert.equal(result.exitCode, 0);
    assert.equal(result.stderr.length, 0);
    JSON.parse(result.stdout.toString());
    assert.deepEqual(readdirSync(workspace), [], "終端指示がworkspaceを変更しないこと");
    return { id: `workspace-${index + 1}`, args, exit_code: result.exitCode,
      stdout: result.stdout.toString(), stderr: result.stderr.toString() };
  } finally { rmSync(parent, { recursive: true, force: true }); }
});
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM,
  source_file: ".claude/tools/aidlc-orchestrate.ts", initial_files: {},
  capture_method: "各入力を空の独立workspaceから実行。標準出力・標準エラー・終了値を無変換で保存。",
  normalization: [], observations }, null, 2) + "\n");
assert.equal(JSON.parse(readFileSync(destination, "utf8")).observations.length, inputs.length);
