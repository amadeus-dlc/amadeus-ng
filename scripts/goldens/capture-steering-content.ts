#!/usr/bin/env bun
/** 固定本家の規則本文判定を同じ入力で実行し、空見出しと実体の境界を保存する。 */
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination); verifySource(dist);
const source = await import(pathToFileURL(join(resolve(dist), ".claude/tools/aidlc-steering.ts")).href);
const inputs = [
  ["org-heading", "# Org\n"], ["phase-heading", "# Inception\n"],
  ["org-rule", "# Org\n\nALWAYS keep the audit record.\n"],
  ["phase-rule", "# Inception\n\nALWAYS confirm the scope.\n"],
  ["comments-only", "# Org\n<!-- draft rule -->\n---\n"],
  ["authored-quote", "> ALWAYS confirm scope.\n"],
  ["bom-only", "\uFEFF"], ["nel-only", "\u0085"],
  ["bom-heading", "\uFEFF# Heading\n"], ["nel-line", "\u0085\n"],
];
const observations = inputs.map(([id, text]) => ({ id, text, substantive: source.isSubstantiveRuleText(text) }));
assert.equal(observations[0].substantive, false);
assert.equal(observations[2].substantive, true);
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, source_file: ".claude/tools/aidlc-steering.ts", capture_method: "固定本家の公開isSubstantiveRuleTextを直接呼出し。原文は変更しない。", normalization: [], observations }, null, 2) + "\n");
