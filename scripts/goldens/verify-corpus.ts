#!/usr/bin/env bun
import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { UPSTREAM, digest, listFiles } from "./upstream-source";
import { readBindings, normalizeText, comparableObservation, removeUnchangedFiles } from "./corpus-normalization";

function readJson(path: string): any {
  const bytes = readFileSync(path);
  const text = bytes.toString("utf8");
  assert(Buffer.from(text, "utf8").equals(bytes), `不正なUTF-8のJSON: ${path}`);
  return JSON.parse(text);
}

export function sealCorpus(root: string) {
  const files = Object.fromEntries(listFiles(root).filter(path => relative(root, path) !== "corpus-manifest.json")
    .map(path => [relative(root, path), digest(readFileSync(path))]));
  writeFileSync(join(root, "corpus-manifest.json"), JSON.stringify({ version: 1, source: UPSTREAM, files }, null, 2) + "\n");
}

export function verifyCorpus(root: string) {
  const manifest = readJson(join(root, "corpus-manifest.json"));
  assert.equal(manifest.version, 1, "未知の採取形式");
  assert.deepEqual(manifest.source, UPSTREAM, "本家固定元が不一致");
  readBindings(root);
  const actual = listFiles(root).map(path => relative(root, path)).filter(path => path !== "corpus-manifest.json").sort();
  assert.deepEqual(actual, Object.keys(manifest.files).sort(), "採取ファイルの欠落または追加");
  for (const [path, expected] of Object.entries(manifest.files)) {
    assert.equal(digest(readFileSync(join(root, path))), expected, `採取バイトが不一致: ${path}`);
    if (path.endsWith(".json")) readJson(join(root, path));
  }
  return manifest;
}

if (import.meta.main) {
  try {
    const [root, other] = process.argv.slice(2);
    assert(root, "Usage: bun verify-corpus.ts <corpus> [comparison-corpus]");
    const manifest = verifyCorpus(root);
    if (other) {
      const compared = verifyCorpus(other);
      assert.deepEqual(Object.keys(manifest.files), Object.keys(compared.files), "再採取のファイル集合が不一致");
      const left = readBindings(root), right = readBindings(other);
      const identities = (entries: typeof left) => [...new Set(entries.map(e => `${e.kind}:${e.replacement}`))].sort();
      assert.deepEqual(identities(left), identities(right), "非対称の正規化規則");
      for (const path of Object.keys(manifest.files)) {
        if (path === "comparison.json") continue;
        const isObservation = path === "observations.json" || /^(?:supplemental|stage1|doctor|learnings)\/(?:cases|provenance)\.json$/.test(path);
        const read = (dir: string) => {
          const bytes = readFileSync(join(dir, path));
          if (isObservation) return Buffer.from(JSON.stringify(comparableObservation(JSON.parse(bytes.toString("utf8"))))).toString("latin1");
          return bytes.toString("latin1");
        };
        const byteBindings = (entries: typeof left) => entries.map(entry => ({ ...entry, value: Buffer.from(entry.value).toString("latin1") }));
        const normalize = (dir: string, entries: typeof left) => {
          const text = normalizeText(read(dir), byteBindings(entries));
          return isObservation ? JSON.stringify(removeUnchangedFiles(JSON.parse(text))) : text;
        };
        const expected = normalize(root, left), actual = normalize(other, right);
        if (expected !== actual) {
          let index = 0;
          while (expected[index] === actual[index] && index < expected.length) index++;
          throw new Error(`再採取との差: ${path}, byte ${index}\nleft: ${expected.slice(Math.max(0, index - 80), index + 180)}\nright: ${actual.slice(Math.max(0, index - 80), index + 180)}`);
        }
      }
    }
    console.log("採取コーパスの検証成功");
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
