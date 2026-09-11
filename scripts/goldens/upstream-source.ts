import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import assert from "node:assert/strict";

export const UPSTREAM = Object.freeze({
  repository: "https://github.com/awslabs/aidlc-workflows",
  commit: "a277af218f0df7f325d3b8be7b6d90fce2c5bd40",
  version: "2.7.1",
  distribution: "dist/claude",
  files: 277,
  manifest: "282b17c53cb82c24755b149f28e01508ce9eaebbdbe3020034a4dd4c8bda4459",
});

export const digest = (value: string | Uint8Array) => createHash("sha256").update(value).digest("hex");

export function listFiles(root: string): string[] {
  return readdirSync(root, { withFileTypes: true }).flatMap(entry => {
    const path = join(root, entry.name);
    assert(!entry.isSymbolicLink(), `配布物のリンクは受け付けない: ${path}`);
    assert(entry.isDirectory() || entry.isFile(), `通常ファイルではない: ${path}`);
    return entry.isDirectory() ? listFiles(path) : [path];
  }).sort((a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b)));
}

export function verifySource(root: string) {
  const paths = listFiles(root);
  assert.equal(paths.length, UPSTREAM.files, "固定ピンの配布ファイル数が不一致");
  const manifest = paths.map(path => `${digest(readFileSync(path))}  ${relative(root, path).replaceAll("\\", "/")}\n`).join("");
  assert.equal(digest(manifest), UPSTREAM.manifest, "固定ピンの配布物マニフェストが不一致");
  return { ...UPSTREAM, manifest_text: manifest };
}
