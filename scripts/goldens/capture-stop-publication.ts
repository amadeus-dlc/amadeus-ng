#!/usr/bin/env bun
/** 固定本家のStop counter書込障害を、実フックの入出力と公開ファイルで保存する。 */
import assert from "node:assert/strict";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
// 既存ケースの生バイトは保持する。再採取による時刻や一時パスの差で置き換えない。
const previous = existsSync(destination) ? JSON.parse(readFileSync(destination, "utf8")) : null;
if (previous) assert.equal(previous.source.commit, UPSTREAM.commit);
const parent = mkdtempSync(join(tmpdir(), "aidlc-stop-publication-"));
const root = join(parent, "workspace");
try {
  cpSync(dist, root, { recursive: true });
  const setup = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "smoke", "--arguments", "Fix one small defect"] });
  assert.equal(setup.output.exit_code, 0, setup.output.stderr);
  const intents = "aidlc/spaces/default/intents";
  const records = readdirSync(join(root, intents), { withFileTypes: true }).filter(entry => entry.isDirectory() && !entry.name.startsWith("."));
  assert.equal(records.length, 1);
  const barrier = join(intents, records[0].name, ".aidlc-stop-hook/block-count.json");
  mkdirSync(join(root, barrier), { recursive: true });
  const observations = [{ id: "setup", fixture_directories: [], ...setup }];
  for (let invocation = 1; invocation <= 2; invocation++) {
    const observed = captureObservation({ root, argv: [process.execPath, join(root, ".claude/hooks/aidlc-continue-workflow.ts")], stdin: '{"stop_hook_active":false}', environment: { AIDLC_DISABLE_USAGE_TRACKING: "1" } });
    assert.equal(observed.output.exit_code, 0, observed.output.stderr);
    assert.equal(JSON.parse(observed.output.stdout).decision, "block");
    observations.push({ id: `counter-directory/${invocation}`, fixture_directories: [barrier], ...observed });
  }

  for (const [prefix, directory, brownfield] of [
    ["published-counter", "previously-published", false],
    ["brownfield-counter", "brownfield-published", true],
  ] as const) {
    const publishedRoot = join(parent, directory);
    cpSync(dist, publishedRoot, { recursive: true });
    if (brownfield) {
      mkdirSync(join(publishedRoot, "src"));
      writeFileSync(join(publishedRoot, "src/lib.rs"), "pub fn smoke() {}\n");
    }
    const publishedSetup = captureObservation({ root: publishedRoot, argv: [process.execPath, join(publishedRoot, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "smoke", "--arguments", "Fix one small defect"] });
    assert.equal(publishedSetup.output.exit_code, 0, publishedSetup.output.stderr);
    observations.push({ id: `${prefix}/setup`, fixture_directories: [], ...publishedSetup });
    const publishedRecords = readdirSync(join(publishedRoot, intents), { withFileTypes: true }).filter(entry => entry.isDirectory() && !entry.name.startsWith("."));
    assert.equal(publishedRecords.length, 1);
    const publishedBarrier = join(intents, publishedRecords[0].name, ".aidlc-stop-hook/block-count.json");
    const invoke = () => captureObservation({ root: publishedRoot, argv: [process.execPath, join(publishedRoot, ".claude/hooks/aidlc-continue-workflow.ts")], stdin: '{"stop_hook_active":false}', environment: { AIDLC_DISABLE_USAGE_TRACKING: "1" } });
    const success = invoke();
    assert.equal(success.output.exit_code, 0, success.output.stderr);
    assert.equal(JSON.parse(success.output.stdout).decision, "block");
    assert.equal(JSON.parse(readFileSync(join(publishedRoot, publishedBarrier), "utf8")).count, 1);
    observations.push({ id: `${prefix}/initial-success`, fixture_directories: [], ...success });

    rmSync(join(publishedRoot, publishedBarrier));
    mkdirSync(join(publishedRoot, publishedBarrier));
    for (let invocation = 1; invocation <= 2; invocation++) {
      const observed = invoke();
      assert.equal(observed.output.exit_code, 0, observed.output.stderr);
      assert.equal(observed.output.stderr, "");
      assert.equal(JSON.parse(observed.output.stdout).decision, "block");
      observations.push({ id: `${prefix}/directory-${invocation}`, fixture_directories: [publishedBarrier], ...observed });
    }
    rmSync(join(publishedRoot, publishedBarrier), { recursive: true });
    const recovered = invoke();
    assert.equal(recovered.output.exit_code, 0, recovered.output.stderr);
    assert.equal(JSON.parse(recovered.output.stdout).decision, "block");
    assert.equal(JSON.parse(readFileSync(join(publishedRoot, publishedBarrier), "utf8")).count, 1);
    observations.push({ id: `${prefix}/recovered`, fixture_directories: [], ...recovered });
  }

  const retained = previous?.observations ?? [];
  const seen = new Set(retained.map((item: { id: string }) => item.id));
  const appended = observations.filter(item => !seen.has(item.id));
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_method: "検証済み固定配布の実フックを別プロセスで実行。最初からcounterがdirectoryの2回に加え、独立workspaceで正常公開→同名directoryへ置換→2回失敗→directory除去後1回を採取。greenfieldとsrc/lib.rsを置いたbrownfieldを別familyで保存する。コード・出力は無変更。既存IDの元観測は保持し新IDのみ追加する。", normalization: [], observations: [...retained, ...appended] }, null, 2) + "\n");
} finally { rmSync(parent, { recursive: true, force: true }); }
