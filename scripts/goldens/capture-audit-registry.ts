#!/usr/bin/env bun
/** 固定本家の監査レジストリ宣言をそのまま評価し、語彙と見出しの全値を保存する。 */
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
const source = readFileSync(join(dist, ".claude/tools/aidlc-audit.ts"), "utf8");
const setStart = source.indexOf("const VALID_EVENT_TYPES = new Set([");
const setEnd = source.indexOf("\n]);", setStart) + 4;
const headingStart = source.indexOf("const EVENT_HEADINGS: Record<string, string> = {");
const headingEnd = source.indexOf("\n};", headingStart) + 3;
assert(setStart >= 0 && setEnd > setStart && headingStart >= setEnd && headingEnd > headingStart);
const declarations = source.slice(setStart, setEnd) + "\n" + source.slice(headingStart, headingEnd);
const temporary = mkdtempSync(join(tmpdir(), "aidlc-audit-registry-"));
try {
  const script = join(temporary, "registry.ts");
  writeFileSync(script, declarations + '\nconsole.log(JSON.stringify([...VALID_EVENT_TYPES].map(event => ({event, heading: EVENT_HEADINGS[event]}))));\n');
  const result = Bun.spawnSync([process.execPath, script], { stdout: "pipe", stderr: "pipe" });
  assert.equal(result.exitCode, 0, result.stderr.toString());
  const entries = JSON.parse(result.stdout.toString());
  assert.equal(entries.length, 91);
  assert(entries.every(entry => typeof entry.heading === "string"));
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, source_file: ".claude/tools/aidlc-audit.ts", source_sha256: createHash("sha256").update(source).digest("hex"), declarations, declarations_sha256: createHash("sha256").update(declarations).digest("hex"), capture_method: "固定本家のSetと見出し宣言を無変更でTypeScriptとして評価。入力なし。正規化なし。", normalization: [], entries }, null, 2) + "\n");
} finally { rmSync(temporary, { recursive: true, force: true }); }
