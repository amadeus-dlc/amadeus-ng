#!/usr/bin/env bun
/**
 * 固定 2.7.1 の reviewer-scope 判定 (`evaluateReviewerScope`) を採取する。
 *
 * `evaluateReviewerScope` は副作用の無い純粋関数なので、フック全体を起動せずに
 * 関数単位で入出力を採れる。差し向け記録と経路の基点は 1 組に固定し、工具・入力だけを
 * 動かす。使い方:
 *
 *   bun scripts/goldens/capture-reviewer-scope.ts <verified-dist/claude> <output.json>
 */
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination, "固定 dist と保存先が必要");
verifySource(dist);
const hook = await import(
  pathToFileURL(join(resolve(dist), ".claude/hooks/aidlc-reviewer-scope.ts")).href
);

/** 差し向け記録 1 件 (§12a step 1)。許可経路は 3 形を入れてある。 */
const DISPATCH = Object.freeze({
  reviewer: "aidlc-architecture-reviewer-agent",
  stage: "functional-design",
  unit: "u1-alpha",
  exempt: [
    // construction/ を通らない項目 (照合に効かない)
    "/r/inception/requirements-analysis/requirements.md",
    // 兄弟 Unit の 1 ファイルを絶対経路で許す
    "/r/construction/u2-beta/functional-design/entities.md",
    // 同じことを construction/ 起点の相対で許す
    "construction/u3-gamma/contract.md",
  ],
});
/** 記録の置き場と、ハーネスが渡す作業ディレクトリ。どちらも実在しない合成値。 */
const CONTEXT = Object.freeze({ recordRoot: "/r", cwd: "/w" });

const cases: Array<[string, Record<string, unknown>]> = JSON.parse(
  await Bun.file(new URL("./reviewer-scope-corpus.json", import.meta.url)).text(),
);

const observations = cases.map(([tool, input], index) => {
  const verdict = hook.evaluateReviewerScope(tool, input, DISPATCH, CONTEXT);
  return {
    id: `case-${String(index + 1).padStart(3, "0")}`,
    tool,
    tool_input: input,
    block: verdict.block === true,
    target: verdict.target ?? null,
  };
});

mkdirSync(dirname(destination), { recursive: true });
writeFileSync(
  destination,
  `${JSON.stringify(
    {
      source: UPSTREAM,
      capture_command: process.argv,
      subject: ".claude/hooks/aidlc-reviewer-scope.ts :: evaluateReviewerScope",
      dispatch: DISPATCH,
      context: CONTEXT,
      normalization: [],
      // 拒否のとき標準エラーへ出る逐語。工具や候補では変わらず、綴りと Unit だけで決まる。
      block_reasons: [
        "/r/construction/u2-beta/design.md",
        ".",
        "construction/*/design.md",
      ].map((target) => ({ target, text: hook.blockReason(target, DISPATCH) })),
      observations,
    },
    null,
    2,
  )}\n`,
);
console.log(`captured ${observations.length} observations -> ${destination}`);
