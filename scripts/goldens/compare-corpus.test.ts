import { test, expect } from "bun:test";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { sealCorpus } from "./verify-corpus";
import { readFileSync } from "node:fs";

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "compare-corpus-"));
  writeFileSync(join(root, "fixed.txt"), "GATE_APPROVED\n");
  sealCorpus(root);
  return root;
}
const verify = (...roots: string[]) => Bun.spawnSync([process.execPath, "scripts/goldens/verify-corpus.ts", ...roots]);

test("生観測のJSONコンテナ自体が不正UTF-8ならseal済みでも拒否する", () => {
  const a = fixture(), b = fixture();
  try {
    for (const [root, byte] of [[a, 255], [b, 254]] as const) {
      writeFileSync(join(root, "observations.json"), Buffer.concat([Buffer.from('[{"note":"'), Buffer.from([byte]), Buffer.from('"}]')]));
      sealCorpus(root);
      expect(verify(root).exitCode).toBe(1);
    }
    expect(verify(a, b).exitCode).toBe(1);
  } finally { for (const root of [a, b]) rmSync(root, { recursive: true, force: true }); }
});

for (const face of ["initial_files", "changed_files", "fixture_changes"]) {
  test(`${face}の不正UTF-8を両辺seal後も区別する`, () => {
    const a = fixture(), b = fixture();
    try {
      for (const [root, byte] of [[a, 255], [b, 254]] as const) {
        writeFileSync(join(root, "observations.json"), JSON.stringify([{ [face]: { "input.bin": Buffer.from([byte]).toString("base64") } }]));
        sealCorpus(root);
      }
      expect(verify(a).exitCode).toBe(0);
      expect(verify(b).exitCode).toBe(0);
      expect(verify(a, b).exitCode).toBe(1);
    } finally { for (const root of [a, b]) rmSync(root, { recursive: true, force: true }); }
  });
}

test("固定文言の1バイト変更を検出する", () => {
  const root = fixture();
  try {
    expect(verify(root).exitCode).toBe(0);
    writeFileSync(join(root, "fixed.txt"), "GATE_APPROVEd\n");
    expect(verify(root).exitCode).toBe(1);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("時刻正規化後の空差分だけを除き状態値の変更は残す", () => {
  const a = fixture(), b = fixture();
  const stamp = (at: string, state = "Running") => Buffer.from(`${state}\n${at}\n`).toString("base64");
  const times = ["2026-09-08T01:00:00Z", "2026-09-08T01:00:01Z"];
  try {
    for (const root of [a, b]) writeFileSync(join(root, "comparison.json"), JSON.stringify({ version: 1, bindings: times.map(value => ({ kind: "time", value, replacement: "<time>" })) }));
    writeFileSync(join(a, "observations.json"), JSON.stringify([{ initial_files: { state: stamp(times[0]) }, changed_files: { state: stamp(times[1]) } }]));
    writeFileSync(join(b, "observations.json"), JSON.stringify([{ initial_files: { state: stamp(times[0]) }, changed_files: {} }]));
    sealCorpus(a); sealCorpus(b);
    expect(verify(a, b).exitCode).toBe(0);
    writeFileSync(join(a, "observations.json"), JSON.stringify([{ initial_files: { state: stamp(times[0]) }, changed_files: { state: stamp(times[1], "Completed") } }]));
    sealCorpus(a);
    expect(verify(a, b).exitCode).toBe(1);
  } finally { for (const root of [a, b]) rmSync(root, { recursive: true, force: true }); }
});

test("出力のIDを後続入力で取り違えたときだけ拒否する", () => {
  const a = fixture(), b = fixture();
  const ids = ["11111111-1111-4111-8111-111111111111", "22222222-2222-4222-8222-222222222222", "33333333-3333-4333-8333-333333333333", "44444444-4444-4444-8444-444444444444"];
  try {
    for (const [root, first, second] of [[a, ids[0], ids[1]], [b, ids[2], ids[3]]]) {
      writeFileSync(join(root, "comparison.json"), JSON.stringify({ version: 1, bindings: [
        { kind: "identifier", value: first, replacement: "<id:1>" },
        { kind: "identifier", value: second, replacement: "<id:2>" },
      ] }));
      writeFileSync(join(root, "fixed.txt"), `${first}\n${second}\n${first}\n`);
      sealCorpus(root);
    }
    expect(verify(a, b).exitCode).toBe(0);
    writeFileSync(join(b, "fixed.txt"), `${ids[2]}\n${ids[3]}\n${ids[3]}\n`);
    sealCorpus(b);
    expect(verify(a, b).exitCode).toBe(1);
  } finally { for (const root of [a, b]) rmSync(root, { recursive: true, force: true }); }
});

test("UTF-8として不正なバイト同士も同一扱いしない", () => {
  const a = fixture(), b = fixture();
  try {
    writeFileSync(join(a, "fixed.txt"), Buffer.from([255]));
    writeFileSync(join(b, "fixed.txt"), Buffer.from([254]));
    sealCorpus(a); sealCorpus(b);
    expect(verify(a, b).exitCode).toBe(1);
  } finally { for (const root of [a, b]) rmSync(root, { recursive: true, force: true }); }
});

test("採取ファイルの欠落を拒否する", () => {
  const root = fixture();
  try { rmSync(join(root, "fixed.txt")); expect(verify(root).exitCode).toBe(1); }
  finally { rmSync(root, { recursive: true, force: true }); }
});

test("未知の正規化規則と片辺だけの規則を拒否する", () => {
  const a = fixture(), b = fixture();
  try {
    writeFileSync(join(a, "comparison.json"), JSON.stringify({ version: 1, bindings: [{ kind: "regex", value: ".*", replacement: "<time>" }] }));
    sealCorpus(a);
    expect(verify(a).exitCode).toBe(1);
    writeFileSync(join(a, "comparison.json"), JSON.stringify({ version: 1, bindings: [{ kind: "path", value: "/tmp/one", replacement: "<root:1>" }] }));
    writeFileSync(join(b, "comparison.json"), JSON.stringify({ version: 1, bindings: [] }));
    sealCorpus(a); sealCorpus(b);
    expect(verify(a, b).exitCode).toBe(1);
  } finally { for (const root of [a, b]) rmSync(root, { recursive: true, force: true }); }
});

test("別々の識別子を同じ値へ潰す正規化を拒否する", () => {
  const root = fixture();
  try {
    writeFileSync(join(root, "comparison.json"), JSON.stringify({ version: 1, bindings: [
      { kind: "identifier", value: "11111111-1111-4111-8111-111111111111", replacement: "<id:1>" },
      { kind: "identifier", value: "22222222-2222-4222-8222-222222222222", replacement: "<id:1>" },
    ] }));
    sealCorpus(root);
    expect(verify(root).exitCode).toBe(1);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("実測パスと時刻に同じ明示規則を両辺へ適用する", () => {
  const a = fixture(), b = fixture();
  try {
    for (const [root, path, at] of [[a, "/tmp/capture-a", "2026-09-08T01:00:00Z"], [b, "/tmp/capture-b", "2026-09-08T02:00:00Z"]]) {
      writeFileSync(join(root, "fixed.txt"), `${path}\n${at}\nGATE_APPROVED\n`);
      writeFileSync(join(root, "comparison.json"), JSON.stringify({ version: 1, bindings: [
        { kind: "path", value: path, replacement: "<root:1>" },
        { kind: "time", value: at, replacement: "<time>" },
      ] }));
      sealCorpus(root);
    }
    expect(verify(a, b).exitCode).toBe(0);
  } finally { for (const root of [a, b]) rmSync(root, { recursive: true, force: true }); }
});

test("独立採取したコーパス間の未知の差を拒否する", () => {
  const a = fixture(), b = fixture();
  try {
    expect(verify(a, b).exitCode).toBe(0);
    writeFileSync(join(b, "fixed.txt"), "GATE_REJECTED\n");
    sealCorpus(b);
    expect(verify(a, b).exitCode).toBe(1);
  } finally { for (const root of [a, b]) rmSync(root, { recursive: true, force: true }); }
});

test("余分なファイルを拒否する", () => {
  const root = fixture();
  try {
    writeFileSync(join(root, "extra.txt"), "extra");
    expect(verify(root).exitCode).toBe(1);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
