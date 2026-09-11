#!/usr/bin/env bun
/** 固定2.7.1の内容確認パーサを追加観測する。U2所有。 */
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const dist = process.argv[2];
const destination = process.argv[3];
assert(dist && destination, "usage: capture-summary-visibility.ts <verified-dist> <output.json>");
verifySource(dist);
const upstream = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-lib.ts")).href);
const inputs = [
  { id: "plain", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n" },
  { id: "fenced-only", content: "# Questions\n\n```md\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n```\n" },
  { id: "commented-only", content: "# Questions\n\n<!--\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n-->\n" },
  { id: "commented-heading-after-assumption", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n<!--\n## Commented Example\n-->\n" },
  { id: "pre-only", content: "# Questions\n\n<pre>\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n</pre>\n" },
  { id: "pre-heading-after-assumption", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n<pre>\n## Literal example\n</pre>\n" },
  { id: "answer-in-html-attribute", content: "# Questions\n\n## Consolidated Summary Confirmation\n<div data-example=\"\n[Answer]: Looks correct\n\">literal</div>\n" },
  { id: "answer-after-unclosed-tag", content: "# Questions\n\n## Consolidated Summary Confirmation\nSummary: adopt <foo\n[Answer]: Looks correct\n" },
  { id: "setext-heading-after-assumption", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\nInjected heading\n---\n" },
  { id: "comment-before-horizontal-rule", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n<!-- comment-only -->\n---\n" },
  { id: "html-heading-after-assumption", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n<h2>Injected heading</h2>\n" },
  { id: "html-heading-in-quoted-attribute", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n<div data-example=\"<h2>literal</h2>\">container</div>\n" },
  { id: "list-continuation-unclosed-fence", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n- item\n  ~~~text\n\n## Q3. Which fallback should be used?\n\n[Answer]: A. Manual review\n" },
  { id: "list-continuation-unclosed-comment", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n- item\n  <!--\n\n## Q3. Which fallback should be used?\n\n[Answer]: A. Manual review\n" },
  { id: "quoted-heading-after-assumption", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n> ## Injected heading\n" },
  { id: "quoted-assumption-is-not-a-root-section", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n> ## Assumption Confirmation\n> [Answer]: A. Accept assumptions\n" },
  { id: "multiline-code-comment", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n`literal comment example\n<!-- marker inside code\nstill literal code`\n## Q3. Fabricated question\n\n[Answer]: A. Fabricated\n" },
  { id: "answer-in-multiline-code", content: "# Questions\n\n## Consolidated Summary Confirmation\n`literal example\n[Answer]: Looks correct\nend`\n" },
  { id: "heading-stops-multiline-code", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n`literal\n## Q3. Fabricated question `\n\n[Answer]: A. Fabricated\n" },
  { id: "closing-hash-heading", content: "# Questions\n\n## Consolidated Summary Confirmation ###\n[Answer]: Looks correct\n" },
  { id: "hash-without-whitespace-is-title-text", content: "# Questions\n\n## Consolidated Summary Confirmation###\n[Answer]: Looks correct\n" },
  { id: "bom-before-summary", content: "\uFEFF## Consolidated Summary Confirmation\n[Answer]: Looks correct\n" },
  { id: "literal-nul-does-not-make-a-heading", content: "##\u0000 Consolidated Summary Confirmation\n[Answer]: Looks correct\n" },
  { id: "setext-after-unclosed-attribute", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n<div data-example=\"\nInjected heading\n---\n" },
  { id: "html-heading-after-unclosed-attribute", content: "# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n<div data-example=\"\n<h2>Injected heading</h2>\n" },
  ...[
    ["list-item-tilde", "- ~~~text"], ["blockquote-tilde", "> ~~~text"],
    ["list-item-backtick", " * ```"], ["list-item-comment", "- <!--"], ["blockquote-comment", "> <!--"],
    ["lazy-list-fence", "- item\ncontinued paragraph\n  ~~~text"],
    ["lazy-quote-fence", "> item\n  ~~~text"], ["quote-following-fence", "> item\n~~~text"],
    ["lazy-quote-comment", "> item\n  <!--"],
  ].map(([id, opener]) => ({ id, content: `# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n${opener}\n\n## Q3. Which fallback should be used?\n\n[Answer]: A. Manual review\n` })),
  ...[
    ["h1", "# Q3. Which fallback should be used?"], ["h3", "### Q3. Which fallback should be used?"],
    ["setext-h1", "Q3. Which fallback should be used?\n================================="],
    ["wrapped-html", "<div><h2>Q3. Which fallback should be used?</h2></div>"],
    ["multiline-html", "<h2\nclass=question>Q3. Which fallback should be used?</h2>"],
    ["indented-html", "   <h2>Q3. Which fallback should be used?</h2>"],
    ["list-heading", "- ## Q3. Which fallback should be used?"],
    ["nested-list-heading", "- Parent item\n  - ## Q3. Which fallback should be used?"],
    ["deep-quote-heading", "> > > > > > > > > ## Q3. Which fallback should be used?"],
    ["quoted-feedback", "> ## Requested Changes Feedback"],
    ["inline-html", "Visible text <h2>Q3. Which fallback should be used?</h2>"],
    ["double-escaped-html", "\\\\<h2>Q3. Which fallback should be used?</h2>"],
    ["literal-html-code", "`<h2>Literal example</h2>`"],
    ["literal-html-link", "[example](<h2>)"],
    ["literal-escaped-html", "\\<h2>Literal example</h2>"],
  ].map(([id, heading]) => ({ id, content: `# Questions\n\n## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\n## Assumption Confirmation\n[Answer]: A. Accept assumptions\n\n${heading}\n` })),
];
inputs.push(
  { id: "bom-before-summary-answer", content: "## Consolidated Summary Confirmation\n[Answer]: \uFEFFLooks correct\n" },
  { id: "nel-before-summary-answer", content: "## Consolidated Summary Confirmation\n[Answer]: \u0085Looks correct\n" },
  { id: "trailing-bom-after-summary", content: "## Consolidated Summary Confirmation\n[Answer]: Looks correct\n\uFEFF" },
);
const observations = inputs.map(({ id, content }) => {
  let hash: string | null = null;
  let hash_error: string | null = null;
  try { hash = upstream.summaryConfirmationContentHash(content); }
  catch (error) { hash_error = error instanceof Error ? error.message : String(error); }
  return { id, content, content_base64: Buffer.from(content).toString("base64"), expected_answer: "Looks correct", answer: upstream.summaryConfirmationAnswer(content), hash, hash_error };
});
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_command: [process.execPath, import.meta.path, dist, destination], tool_versions: { bun: Bun.version }, normalization: "none", observations }, null, 2) + "\n");
