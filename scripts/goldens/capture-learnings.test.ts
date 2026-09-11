import { afterAll, beforeAll, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { captureLearnings } from "./capture-learnings";

let fixture: string;
let corpus: ReturnType<typeof captureLearnings>;
beforeAll(() => {
  fixture = mkdtempSync(join(tmpdir(), "learnings-source-"));
  const archive = Bun.spawnSync(["git", "-C", "vendor/aidlc-workflows", "archive", "a277af218f0df7f325d3b8be7b6d90fce2c5bd40", "dist/claude"]);
  expect(archive.exitCode).toBe(0);
  expect(Bun.spawnSync(["tar", "-xf", "-", "-C", fixture], { stdin: archive.stdout }).exitCode).toBe(0);
  corpus = captureLearnings(join(fixture, "dist/claude"));
}, 30000);
afterAll(() => rmSync(fixture, { recursive: true, force: true }));

test("学びの候補表示が選択元のspace/intentを保持して無変更で返す", () => {
  const row = corpus.observations.find(row => row.id === "learnings/surface");
  expect(row).toBeDefined();
  expect(row!.output.exit_code).toBe(0);
  const output = JSON.parse(row!.output.stdout);
  expect(output.space).toBe("default");
  expect(output.intent).toContain("learnings");
  expect(output.candidates).toEqual([]);
  expect(row!.changed_files).toEqual({});
});

test("空の選択は規則も監査も追加しない", () => {
  const row = corpus.observations.find(row => row.id === "learnings/persist-empty");
  expect(row).toBeDefined();
  expect(row!.output.exit_code).toBe(0);
  expect(JSON.parse(row!.output.stdout).rule_learned).toBe(0);
  expect(row!.changed_files).toEqual({});
});

test("学びを1件保存すると規則とRULE_LEARNEDが対応して増える", () => {
  const row = corpus.observations.find(row => row.id === "learnings/persist-one");
  expect(row).toBeDefined();
  expect(row!.output.exit_code).toBe(0);
  expect(JSON.parse(row!.output.stdout).rule_learned).toBe(1);
  const contents = Object.values(row!.changed_files).filter((bytes): bytes is string => bytes !== null).map(bytes => Buffer.from(bytes, "base64").toString()).join("\n");
  expect(contents).toContain("**Event**: RULE_LEARNED");
  expect(contents).toContain("ALWAYS 採取用の検証結果を記録する。");
});

test("同じ選択を再実行しても規則と監査を重複させない", () => {
  const row = corpus.observations.find(row => row.id === "learnings/persist-repeat");
  expect(row).toBeDefined();
  expect(row!.output.exit_code).toBe(0);
  expect(JSON.parse(row!.output.stdout).rule_learned).toBe(0);
  expect(row!.changed_files).toEqual({});
});

test("採取元と違うstageへの保存を拒否し内容を変えない", () => {
  const row = corpus.observations.find(row => row.id === "learnings/wrong-stage");
  expect(row).toBeDefined();
  expect(row!.output.exit_code).toBe(1);
  expect(row!.output.stderr).toContain("slug mismatch");
  expect(row!.changed_files).toEqual({});
});

test("選択元の識別情報が欠けた入力を拒否する", () => {
  const row = corpus.observations.find(row => row.id === "learnings/malformed-selection");
  expect(row).toBeDefined();
  expect(row!.output.exit_code).toBe(1);
  expect(row!.output.stderr).toContain("missing or non-string space");
  expect(row!.changed_files).toEqual({});
});
