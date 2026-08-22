#!/usr/bin/env bun
/**
 * hash-canonical 受入表の採取 (FR7.1 / BR2.1 / BR2.3)。
 *
 * upstream ピン `3c3146cf` から抽出した `canonicalize` / `sha256` / `hashObject` を
 * **実行して** 期待値を採る。入力クラスごとに 4 つの観測を採る:
 *
 * - `canonical_output`   = `JSON.stringify(canonicalize(v))`      … hash-canonical プロファイル
 * - `canonical_digest`   = `hashObject(v)`                        … 正準族 (`sha256:` 接頭辞)
 * - `compact_output`     = `JSON.stringify(v)`                    … contract-compact プロファイル
 * - `compact_digest_hex` = `sha256(JSON.stringify(v))` の生 hex   … 非正準族
 * - `pretty_output`      = `JSON.stringify(v, null, 2) + "\n"`    … contract-pretty プロファイル
 *
 * 採取に失敗したケースは `missing_cases` に理由付きで記録し、値を捏造しない (W4)。
 *
 * 呼び出しは `scripts/goldens/recapture-hash-canonical.sh` 経由 (スニペットの取得・
 * sha256 照合はシェル側の責務)。
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

type Snippet = {
  canonicalize: (value: unknown) => unknown;
  sha256: (value: string) => string;
  hashObject: (value: unknown) => string;
};

/** 非有限数のように JSON テキストで表せない入力を組み立てる宣言的な木。 */
type ConstructNode =
  | { t: "null" }
  | { t: "bool"; v: boolean }
  | { t: "f64"; v: "nan" | "inf" | "-inf" }
  | { t: "u64"; v: string }
  | { t: "str"; v: string }
  | { t: "arr"; v: ConstructNode[] }
  | { t: "obj"; v: [string, ConstructNode][] };

type CaseSpec = {
  id: string;
  class: string;
  description: string;
  /** JSON テキストで表せる入力 (`JSON.parse` で評価する)。 */
  input?: string;
  /** JSON で表せない入力の JS 式 (記録用。評価は `construct` で行う)。 */
  input_js?: string;
  /** JSON で表せない入力の構築木 (Rust 側も同じ木から組み立てる)。 */
  construct?: ConstructNode;
};

const CASES: CaseSpec[] = [
  // ---- ネスト --------------------------------------------------------------
  {
    id: "hash-canonical/nesting/deep-mixed",
    class: "nesting",
    description: "オブジェクトと配列の交互ネスト。再帰ソートが全階層に効くことを固定する",
    input: '{"z":{"b":[1,2,{"c":[{"d":{"e":null}}]}],"a":true}}',
  },
  {
    id: "hash-canonical/nesting/array-of-objects",
    class: "nesting",
    description: "配列要素のオブジェクトも再帰ソートの対象になる (配列自体の順は保存)",
    input: '[{"z":1,"a":2},{"m":{"n":[true,false]}},[[{"y":0,"x":1}]]]',
  },

  // ---- integer-like キー ---------------------------------------------------
  {
    id: "hash-canonical/integer-like/mixed",
    class: "integer-like-keys",
    description: "integer-like キーは挿入順に関係なく数値昇順で先頭 (ECMAScript の所有プロパティ順序)",
    input: '{"b":1,"10":2,"2":3,"a":4,"0":5}',
  },
  {
    id: "hash-canonical/integer-like/numeric-vs-string-order",
    class: "integer-like-keys",
    description: "integer-like は数値昇順 (文字列順なら 1 < 10 < 9)。非 integer-like は挿入順",
    input: '{"1":"one","10":"ten","9":"nine","x":0}',
  },
  {
    id: "hash-canonical/integer-like/non-canonical-decimal",
    class: "integer-like-keys",
    description: "正準十進表記でないキー (01 / +1 / 1.0 / -1) は integer-like ではない",
    input: '{"01":"a","1":"b","+1":"c","1.0":"d","-1":"e"}',
  },
  {
    id: "hash-canonical/integer-like/boundary",
    class: "integer-like-keys",
    description: "配列インデックスの上限 2^32-2 は integer-like、2^32-1 は非 integer-like",
    input: '{"4294967295":"beyond","4294967294":"max","0":"zero"}',
  },

  // ---- 非有限数 (JSON テキストで表せない) ----------------------------------
  {
    id: "hash-canonical/non-finite/root-nan",
    class: "non-finite",
    description: "トップレベルの NaN は null になる",
    input_js: "NaN",
    construct: { t: "f64", v: "nan" },
  },
  {
    id: "hash-canonical/non-finite/object-members",
    class: "non-finite",
    description: "オブジェクトのメンバの NaN / ±Infinity は null になる",
    input_js: '({ a: NaN, b: Infinity, c: -Infinity, d: 1 })',
    construct: {
      t: "obj",
      v: [
        ["a", { t: "f64", v: "nan" }],
        ["b", { t: "f64", v: "inf" }],
        ["c", { t: "f64", v: "-inf" }],
        ["d", { t: "u64", v: "1" }],
      ],
    },
  },
  {
    id: "hash-canonical/non-finite/array-elements",
    class: "non-finite",
    description: "配列要素の NaN / ±Infinity は null になる (要素は詰められない)",
    input_js: "[NaN, Infinity, -Infinity, 1]",
    construct: {
      t: "arr",
      v: [
        { t: "f64", v: "nan" },
        { t: "f64", v: "inf" },
        { t: "f64", v: "-inf" },
        { t: "u64", v: "1" },
      ],
    },
  },

  // ---- 負ゼロ --------------------------------------------------------------
  {
    id: "hash-canonical/negative-zero/root",
    class: "negative-zero",
    description: "-0.0 は '0' と書かれる",
    input: "-0.0",
  },
  {
    id: "hash-canonical/negative-zero/in-object",
    class: "negative-zero",
    description: "-0.0 / 0.0 / -0 の 3 表記がすべて '0' になる",
    input: '{"neg_float":-0.0,"pos_float":0.0,"neg_int":-0,"pos_int":0}',
  },

  // ---- 指数表記 ------------------------------------------------------------
  {
    id: "hash-canonical/exponent/thresholds",
    class: "exponent",
    description: "非指数表記の閾値の両側 (1e20 / 1e21、1e-6 / 1e-7)",
    input: "[1e20,1e21,1e-6,1e-7]",
  },
  {
    id: "hash-canonical/exponent/mantissa-and-extremes",
    class: "exponent",
    description: "多桁仮数と f64 の両端 (最大有限値・最小非正規化数)",
    input: "[123e-20,1.5e300,1.7976931348623157e308,5e-324,-2.5e-7]",
  },
  {
    id: "hash-canonical/exponent/fraction-boundary",
    class: "exponent",
    description: "0.000001 (非指数) と 0.0000001 (指数) の境界、および 0.1 / 0.5",
    input: "[0.000001,0.0000001,0.1,0.5,-0.25]",
  },

  // ---- 2^53 超の整数 -------------------------------------------------------
  {
    id: "hash-canonical/large-int/around-2p53",
    class: "large-integers",
    description: "2^53 の前後。2^53+1 は f64 に丸められて 2^53 と同じ表記になる",
    input: "[9007199254740992,9007199254740993,9007199254740994,-9007199254740993]",
  },
  {
    id: "hash-canonical/large-int/u64-range",
    class: "large-integers",
    description: "u64 上限付近。f64 丸めの結果が指数なしの桁埋め表記になる",
    input: "[18446744073709551615,12345678901234567890]",
  },
  {
    id: "hash-canonical/large-int/i64-min",
    class: "large-integers",
    description: "i64 最小値 (= -2^63)。最短往復表記は '-9223372036854776000'",
    input: "[-9223372036854775808,9223372036854775807]",
  },

  // ---- 非 ASCII ------------------------------------------------------------
  {
    id: "hash-canonical/non-ascii/values-and-keys",
    class: "non-ascii",
    description: "非 ASCII はエスケープせず UTF-8 のまま出力する",
    input: '{"キー":"値","emoji":"🎉","mixed":"aあ漢字","accent":"é"}',
  },
  {
    id: "hash-canonical/non-ascii/bmp-key-order",
    class: "non-ascii",
    description: "BMP 内のキー整列。ASCII 大文字 < ASCII 小文字 < 非 ASCII",
    input: '{"b":1,"あ":2,"A":3,"a":4}',
  },
  {
    id: "hash-canonical/non-ascii/utf16-vs-codepoint-key-order",
    class: "non-ascii",
    description:
      "UTF-16 コード単位順とコードポイント順が割れる唯一の形: 非 BMP (サロゲート D83D) は U+FB00 より前",
    input: '{"\\ufb00":"ff-ligature","\\ud83d\\ude00":"grinning"}',
  },

  // ---- エスケープ ----------------------------------------------------------
  {
    id: "hash-canonical/escape/control-and-quotes",
    class: "escape",
    description: "最小エスケープ集合。'/' はエスケープしない、C0 の短縮形と \\u00xx",
    input:
      '{"quote":"a\\"b","backslash":"a\\\\b","slash":"a/b","newline":"a\\nb","tab":"a\\tb","carriage":"a\\rb","backspace":"a\\bb","formfeed":"a\\fb","nul":"\\u0000","bell":"\\u0007","unit-sep":"\\u001f","del":"\\u007f"}',
  },
  {
    id: "hash-canonical/escape/line-separators",
    class: "escape",
    description: "U+2028 / U+2029 が生出力かエスケープかを upstream 実測で固定する",
    input: '["\\u2028","\\u2029"]',
  },

  // ---- 空のコンテナ --------------------------------------------------------
  {
    id: "hash-canonical/empty/containers",
    class: "empty",
    description: "空の配列・オブジェクトは '[]' / '{}' (pretty でも改行を入れない)",
    input: '{"arr":[],"obj":{},"nested":{"a":[],"b":{}},"arr_of_empty":[[],{}]}',
  },
  {
    id: "hash-canonical/empty/root-array",
    class: "empty",
    description: "ルートが空配列",
    input: "[]",
  },
  {
    id: "hash-canonical/empty/root-object",
    class: "empty",
    description: "ルートが空オブジェクト",
    input: "{}",
  },
  {
    id: "hash-canonical/empty/empty-string-key",
    class: "empty",
    description: "空文字列キー。整列では最小",
    input: '{"z":1,"":2}',
  },

  // ---- 型付き struct のフィールド順 ----------------------------------------
  {
    id: "hash-canonical/struct-order/directive-like",
    class: "struct-field-order",
    description:
      "型付き契約型のフィールド宣言順 (contract-*) と再帰ソート (hash-canonical) の対比",
    input:
      '{"kind":"run-stage","stage":"domain-design","agent":"aidlc-architect-agent","consumes":["a","b"],"continue_token":null}',
  },

  // ---- 浮動小数の整数値 ----------------------------------------------------
  {
    id: "hash-canonical/float-integral/one-point-zero",
    class: "float-integral",
    description: "1.0 は '1'。整数値の f64 は小数点を落とす",
    input: "[1.0,2.0,-3.0,1e2,100.0,0.0]",
  },

  // ---- スカラのルート ------------------------------------------------------
  {
    id: "hash-canonical/scalar/roots",
    class: "scalar",
    description: "ルートがスカラの 4 形 (null / bool / string / integer)",
    input: '[null,true,false,"hello",42,-42]',
  },

  // ---- 実在の契約 JSON から採った値 (棚卸し I2 / I4 の裏取り) --------------
  {
    id: "hash-canonical/contract-floats/observed-values",
    class: "contract-observed",
    description:
      "`.claude/tools/data/*.json` に実在する浮動小数 22 種 (重複除去・昇順)。棚卸し I4 で発見した実データ",
    input:
      "[0.1,0.15,0.2,0.25,0.3,0.4,0.5,0.7,1.0,1.25,2.0,3.0,3.75,5.0,6.0,6.25,10.0,12.5,15.0,20.0,25.0,50.0]",
  },
  {
    id: "hash-canonical/contract-integer-like/ev-thresholds",
    class: "contract-observed",
    description:
      "`ars-priors.json` の `evThresholds` と同じ形 (integer-like キー 1〜5)。挿入順を崩して先頭寄せを確認する。棚卸し I2 で発見した実データ",
    input: '{"5":0.5,"1":0,"3":0.3,"4":0.4,"2":0.2}',
  },

  // ---- 重複キー ------------------------------------------------------------
  {
    id: "hash-canonical/duplicate-keys/last-wins",
    class: "duplicate-keys",
    description: "同名キーは後勝ちで値を置換し、位置は最初の出現位置を維持する",
    input: '{"a":1,"b":2,"a":3}',
  },
];

function build(node: ConstructNode): unknown {
  switch (node.t) {
    case "null":
      return null;
    case "bool":
      return node.v;
    case "f64":
      if (node.v === "nan") return Number.NaN;
      if (node.v === "inf") return Number.POSITIVE_INFINITY;
      return Number.NEGATIVE_INFINITY;
    case "u64":
      return Number(node.v);
    case "str":
      return node.v;
    case "arr":
      return node.v.map(build);
    case "obj": {
      const out: Record<string, unknown> = {};
      for (const [key, child] of node.v) out[key] = build(child);
      return out;
    }
  }
}

async function main(): Promise<void> {
  const [snippetPath, outDir, metaPath] = process.argv.slice(2);
  if (!snippetPath || !outDir || !metaPath) {
    console.error(
      "usage: bun capture-hash-canonical.ts <snippet.ts> <out-dir> <meta.json>",
    );
    process.exit(2);
  }

  const meta = JSON.parse(readFileSync(metaPath, "utf-8"));
  const snippet = (await import(snippetPath)) as Snippet;

  const capturedAt = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
  const cases: unknown[] = [];
  const missing: unknown[] = [];

  for (const spec of CASES) {
    try {
      const value =
        spec.construct !== undefined
          ? build(spec.construct)
          : JSON.parse(spec.input as string);

      const canonicalOutput = JSON.stringify(snippet.canonicalize(value));
      const canonicalDigest = snippet.hashObject(value);
      const compactOutput = JSON.stringify(value);
      const compactDigestPrefixed = snippet.sha256(compactOutput);
      const prettyOutput = `${JSON.stringify(value, null, 2)}\n`;

      if (canonicalOutput === undefined || compactOutput === undefined) {
        throw new Error("JSON.stringify returned undefined");
      }
      if (!compactDigestPrefixed.startsWith("sha256:")) {
        throw new Error(`unexpected digest shape: ${compactDigestPrefixed}`);
      }

      cases.push({
        id: spec.id,
        class: spec.class,
        description: spec.description,
        ...(spec.input !== undefined ? { input: spec.input } : {}),
        ...(spec.input_js !== undefined ? { input_js: spec.input_js } : {}),
        ...(spec.construct !== undefined ? { construct: spec.construct } : {}),
        expected: {
          canonical_output: canonicalOutput,
          canonical_digest: canonicalDigest,
          compact_output: compactOutput,
          compact_digest_prefixed: compactDigestPrefixed,
          compact_digest_hex: compactDigestPrefixed.slice("sha256:".length),
          pretty_output: prettyOutput,
        },
        provenance: {
          commit: meta.upstream_commit,
          captured_at: capturedAt,
          command: meta.command,
        },
      });
    } catch (error) {
      missing.push({
        id: spec.id,
        class: spec.class,
        reason: error instanceof Error ? error.message : String(error),
      });
    }
  }

  const corpus = {
    family: "hash-canonical",
    upstream_commit: meta.upstream_commit,
    case_count: cases.length,
    cases,
  };

  const provenance = {
    family: "hash-canonical",
    upstream_repo: meta.upstream_repo,
    upstream_commit: meta.upstream_commit,
    upstream_version: meta.upstream_version,
    source_url: meta.source_url,
    source_path: meta.source_path,
    source_file_sha256: meta.source_file_sha256,
    snippet: {
      lines: meta.snippet_lines,
      sha256: meta.snippet_sha256,
      functions: ["canonicalize", "sha256", "hashObject"],
      spec_reference: "docs/upstream/specs/09-cli-tools.md §8.4",
    },
    captured_at: capturedAt,
    command: meta.command,
    bun_version: meta.bun_version,
    case_count: cases.length,
    missing_cases: missing,
  };

  mkdirSync(outDir, { recursive: true });
  writeFileSync(join(outDir, "cases.json"), `${JSON.stringify(corpus, null, 2)}\n`);
  writeFileSync(
    join(outDir, "provenance.json"),
    `${JSON.stringify(provenance, null, 2)}\n`,
  );

  process.stderr.write(
    `captured ${cases.length} cases, ${missing.length} missing -> ${outDir}\n`,
  );
  if (missing.length > 0) {
    process.stderr.write(`missing: ${JSON.stringify(missing, null, 2)}\n`);
  }
}

await main();
