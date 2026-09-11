import { afterAll, beforeAll, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { captureDoctor } from "./capture-doctor";

let fixture: string;
let corpus: ReturnType<typeof captureDoctor>;
beforeAll(() => {
  fixture = mkdtempSync(join(tmpdir(), "doctor-source-"));
  const archive = Bun.spawnSync(["git", "-C", "vendor/aidlc-workflows", "archive", "a277af218f0df7f325d3b8be7b6d90fce2c5bd40", "dist/claude"]);
  expect(archive.exitCode).toBe(0);
  expect(Bun.spawnSync(["tar", "-xf", "-", "-C", fixture], { stdin: archive.stdout }).exitCode).toBe(0);
  corpus = captureDoctor(join(fixture, "dist/claude"));
}, 60000);
afterAll(() => rmSync(fixture, { recursive: true, force: true }));

test("初回doctorは生出力を保存し作業記録を生成しない", () => {
  const row = corpus.observations.find(row => row.id === "doctor/cold");
  expect(row).toBeDefined();
  expect(row!.output.stdout).toStartWith("AI-DLC Health Check\n");
  expect(row!.output.stderr).toBe("");
  expect(row!.changed_files).toEqual({});
  expect(row!.output.stdout).toContain("Hook heartbeats: not yet fired");
});

test("既存記録のdoctorはHEALTH_CHECKEDを1件記録する", () => {
  const row = corpus.observations.find(row => row.id === "doctor/initialized");
  expect(row).toBeDefined();
  expect(row!.output.stdout).toContain("State Version: 8");
  expect(row!.output.stderr).toBe("");
  const audit = Object.entries(row!.changed_files).filter(([path]) => path.includes("/audit/")).map(([, bytes]) => Buffer.from(bytes!, "base64").toString()).join("\n");
  expect(audit.match(/\*\*Event\*\*: HEALTH_CHECKED/g)).toHaveLength(1);
  expect(audit).toContain("**Request**: /aidlc --doctor");
});

test("Bun不足を本家の失敗行と終了値で採取する", () => {
  const row = corpus.observations.find(row => row.id === "doctor/missing-bun");
  expect(row).toBeDefined();
  expect(row!.output.exit_code).toBe(1);
  expect(row!.output.stdout).toContain("✗  bun installed (required for CLI tools and hooks)");
  expect(row!.output.stderr).toBe("");
});

test("フック欠落・無効化・設定不足を区別して採取する", () => {
  for (const [id, label] of [
    ["missing-hook", "✗  aidlc-record-human-turn.ts present"],
    ["disabled-hooks", "✗  Hooks DISABLED"],
    ["missing-settings", "✗  Hook contract: settings.json unreadable"],
    ["no-hooks", "✗  Hook contract: settings.json wires no aidlc-*.ts hooks"],
    ["invalid-settings", "✗  Hook contract: settings.json wires no aidlc-*.ts hooks"],
  ]) {
    const row = corpus.observations.find(row => row.id === `doctor/${id}`);
    expect(row).toBeDefined();
    expect(row!.output.stdout).toContain(label);
    expect(row!.output.exit_code).toBe(1);
    expect(row!.output.stderr).toBe("");
  }
});

test("状態版の欠損・過去・未来を本家の分類で採取する", () => {
  for (const [id, label] of [["version-missing", "readable"], ["version-past", "current"], ["version-future", "compatible"]]) {
    const row = corpus.observations.find(row => row.id === `doctor/${id}`);
    expect(row).toBeDefined();
    expect(row!.output.stdout).toContain(`✗  state version ${label}`);
    expect(row!.output.exit_code).toBe(1);
    expect(row!.output.stderr).toBe("");
  }
});

test("heartbeatの300000ms境界は許容し超過は失敗する", () => {
  const boundary = corpus.observations.find(row => row.id === "doctor/heartbeat-boundary");
  const stale = corpus.observations.find(row => row.id === "doctor/heartbeat-stale");
  expect(boundary).toBeDefined();
  expect(stale).toBeDefined();
  expect(boundary!.output.stdout).toContain("✓  Hooks last fired:");
  expect(boundary!.output.exit_code).toBe(0);
  expect(stale!.output.stdout).toContain("✗  Hooks last fired ");
  expect(stale!.output.exit_code).toBe(1);
});

test("必要なステージの欠落・不正schema・不正参照を採取する", () => {
  for (const [id, label] of [["missing-stage", "✗  Orphan stage files:"], ["invalid-stage", "✗  Schema validation:"], ["invalid-reference", "✗  Graph references:"], ["cyclic-graph", "✗  Cycle detection:"]]) {
    const row = corpus.observations.find(row => row.id === `doctor/${id}`);
    expect(row).toBeDefined();
    expect(row!.output.stdout).toContain(label);
    expect(row!.output.exit_code).toBe(1);
    expect(Object.keys(row!.fixture_changes).length).toBeGreaterThan(0);
  }
});

test("管理設定によるhook制限も実測した環境とともに採取する", () => {
  const row = corpus.observations.find(row => row.id === "doctor/managed-only");
  expect(row).toBeDefined();
  expect(row!.output.stdout).toContain("✗  Claude managed hook policy: allowManagedHooksOnly=true");
  expect(row!.output.exit_code).toBe(1);
  expect(row!.input.environment.AIDLC_MANAGED_SETTINGS_PATH).toBeDefined();
});

test("読めないheartbeatを初回の未発火と混同しない", () => {
  const row = corpus.observations.find(row => row.id === "doctor/heartbeat-unreadable");
  expect(row).toBeDefined();
  expect(row!.output.stdout).toContain("✗  Hook heartbeat data");
  expect(row!.output.stdout).toContain("heartbeat files are unreadable");
  expect(row!.output.exit_code).toBe(1);
});

test("診断の監査保存に失敗した結果を成功として保存しない", () => {
  const row = corpus.observations.find(row => row.id === "doctor/audit-locked");
  expect(row).toBeDefined();
  expect(row!.output.exit_code).toBe(1);
  expect(row!.output.stderr).toContain("Failed to acquire audit lock");
  const changes = Object.entries(row!.changed_files).filter(([path]) => path.includes("/audit/"));
  expect(changes).toHaveLength(0);
});
