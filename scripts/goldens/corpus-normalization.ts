import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

export type Binding = { kind: "path" | "time" | "identifier" | "token" | "clone" | "digest" | "record-date" | "fire-id" | "duration" | "mtime" | "learned-day"; value: string; replacement: string };
export function readBindings(root: string): Binding[] {
  const path = join(root, "comparison.json");
  if (!existsSync(path)) return [];
  const parsed = JSON.parse(readFileSync(path, "utf8"));
  assert.deepEqual(Object.keys(parsed).sort(), ["bindings", "version"]);
  assert.equal(parsed.version, 1);
  assert(Array.isArray(parsed.bindings));
  const values = new Set<string>();
  const identities = new Set<string>();
  for (const entry of parsed.bindings as Binding[]) {
    assert.deepEqual(Object.keys(entry).sort(), ["kind", "replacement", "value"]);
    const patterns = { path: /^\//, time: /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z$/, identifier: /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/, token: /^[A-Za-z0-9_-]{200,}$/, clone: /^[a-z0-9.-]+-[a-f0-9]{12}$/, digest: /^(?:sha256:)?[a-f0-9]{64}$/, "record-date": /^\d{6}-(?:golden|stage1|doctor|learnings)$/, "fire-id": /^[a-f0-9]{8}$/ };
    const fieldRules = {
      duration: [/^\*\*Duration ms\*\*: \d+$/, "**Duration ms**: <duration>"],
      mtime: [/^\*\*Artifact Mtime Ms\*\*: \d+(?:\.\d+)?$/, "**Artifact Mtime Ms**: <time>"],
      "learned-day": [/^\(learned \d{4}-\d\d-\d\d\)$/, "(learned <date>)"],
    } as const;
    const fieldRule = fieldRules[entry.kind as keyof typeof fieldRules];
    assert(fieldRule ? fieldRule[0].test(entry.value) : patterns[entry.kind as keyof typeof patterns]?.test(entry.value), "不正な正規化規則");
    assert(fieldRule ? entry.replacement === fieldRule[1] : /^<(?:root|tool|time|id|token|clone|digest|record|fire)(?::\d+)?>$/.test(entry.replacement), "不正な置換先");
    assert(!values.has(entry.value), "同じ値に複数の正規化規則");
    values.add(entry.value);
    if (["identifier", "token", "digest", "fire-id"].includes(entry.kind)) {
      assert(!identities.has(entry.replacement), "識別子対応を潰す正規化");
      identities.add(entry.replacement);
    }
  }
  return parsed.bindings;
}

/** 正本のstate/auditと合成入力を比較する。機械内の署名鍵/lock/pidは生データに保持する。 */
function comparableFile(data: string | null): string | { binary_bytes: number[] } | null {
  if (data === null) return null;
  const bytes = Buffer.from(data, "base64");
  const text = bytes.toString("utf8");
  // 不正UTF-8は文字列の置換対象にせず、元の各バイトを比較する。
  return Buffer.from(text, "utf8").equals(bytes) ? text : { binary_bytes: [...bytes] };
}

export function comparableObservation(value: any): any {
  if (Array.isArray(value)) return value.map(comparableObservation);
  if (value === null || typeof value !== "object") return value;
  const result: Record<string, unknown> = {};
  for (const [key, entry] of Object.entries(value)) {
    if (key === "initial_files" || key === "changed_files" || key === "fixture_changes") {
      result[key] = Object.fromEntries(Object.entries(entry as Record<string, string | null>)
        .filter(([path]) => !path.split("/").some(part => part.startsWith(".aidlc-") || part === ".capture-home"))
        .map(([path, data]) => [path, comparableFile(data)]));
    } else if (key.endsWith("_base64") && typeof entry === "string") {
      result[key] = Buffer.from(entry, "base64").toString("latin1");
    } else result[key] = comparableObservation(entry);
  }
  return result;
}

export function removeUnchangedFiles(value: any): any {
  if (Array.isArray(value)) return value.map(removeUnchangedFiles);
  if (value === null || typeof value !== "object") return value;
  if (value.initial_files && value.changed_files) {
    value.changed_files = Object.fromEntries(Object.entries(value.changed_files).filter(([path, after]) => value.initial_files[path] !== after));
  }
  for (const key of Object.keys(value)) value[key] = removeUnchangedFiles(value[key]);
  return value;
}

export function normalizeText(text: string, bindings: Binding[]): string {
  let result = text;
  for (const binding of [...bindings].sort((a, b) => b.value.length - a.value.length)) {
    result = result.split(binding.value).join(binding.replacement);
  }
  return result;
}
