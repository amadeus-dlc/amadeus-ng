# code-summary — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Code Generation（Construction 3.5）の実装要約（Unit: U1、Bolt: B1、ブランチ `bolt/b1-u1-canon-json-goldens`）。
> 出典: `code-generation-plan.md`（Step 0〜19、承認指紋 `sha256:f56763a1…c36ff`）、`unit-test-instructions.md`、
> 開発エージェントの 2 回の委任（Step 1〜16 / Step 17〜19）の報告、コンダクタによる差分レビューと品質ゲートの
> 再実行（2026-08-22）。
>
> 計画ファイルのチェックボックスは**承認時のバイト列のまま**にしてある（承認指紋がファイル全体のバイト列に掛かる
> ため）。Step ごとの完了状況は本ファイル §1 が正本。

## 1. Step ごとの完了状況

| Step | 内容 | 状況 |
|---|---|---|
| 0 | Bolt 開始（`aidlc-bolt.ts start --name B1 --batch 1`）・ブランチ作成・aidlc 記録コミット `50e4ed7` | 完了（コンダクタ） |
| 1 | workspace 依存（serde / serde_json preserve_order + float_roundtrip / sha2 / proptest）、`clippy.toml` disallowed-methods、canon-json 骨格 | 完了 `7e38abe` |
| 2 | テストランナー確認（`cargo test -p canon-json` 0 tests / exit 0）、統合テストの配置 | 完了 |
| 3〜4 | hash-canonical 受入表の再採取スクリプトと採取（32 ケース、欠落 0）、README 節追記 | 完了 `64322d0` |
| 5〜7 | Data model 層（value / profile / digest 型）Red → Green → Refactor | 完了 `7999f81` |
| 8〜10 | Business logic 層（writer / canonical / digest / parse）Red → Green → Refactor、受入表全行一致 | 完了 `7999f81` |
| 11 | PBT（決定性・往復・冪等性・非有限 → null・再パース可能） | 完了 `48c0f7d` |
| 12〜14 | API 層（`to_value`、ファサード `pub use` 列挙、クレート rustdoc） | 完了 `bce109b` |
| 15 | 棚卸し I1〜I6（§4） | 完了 `d823f2f`（contract-observed ケース追加） |
| 16 | 品質ゲート（fmt / clippy / lint / test）— コンダクタ再実行でも全緑 | 完了 |
| 17 | CLI 主要遷移（11 動詞 22 ケース、欠落 2）とフック 4 本（14 ケース、欠落 1）の実行出力ゴールデン採取 + README 節追記 | 完了 `f5a4353` |
| 18 | 比較器（`tests/support/mod.rs`: 正規化・コーパス読取・行 diff）と `golden_corpus_read.rs` 9 本 | 完了 `d1688a9` |
| 19 | 品質ゲート（fmt / clippy / lint / test）— コンダクタ再実行でも全緑 | 完了 |

## 2. 作成・変更ファイル（委任 1）

- 新規: `modules/shared/canon-json/src/{value,profile,writer,canonical,digest,parse}.rs`、
  `modules/shared/canon-json/tests/golden_hash_canonical.rs`、`modules/shared/canon-json/proptest-regressions/{parse,digest}.txt`、
  `scripts/goldens/recapture-hash-canonical.sh`、`scripts/goldens/capture-hash-canonical.ts`、
  `tests/golden/upstream-3c3146cf/hash-canonical/{cases.json,provenance.json}`、`tests/golden/upstream-3c3146cf/normalization.json`
- 変更: `Cargo.toml`、`Cargo.lock`、`clippy.toml`、`modules/shared/canon-json/{Cargo.toml,src/lib.rs}`、
  `modules/core/domain/Cargo.toml`、`modules/core/interface-adapter/Cargo.toml`（serde / serde_json を workspace 依存へ）、
  `tests/golden/upstream-3c3146cf/README.md`（末尾に節を追記。既存部分 3,802 バイトと `stage-graph.json` /
  `scope-grid.json` の sha256 は採取前と一致 — バイト不変を確認済み）
- 委任 2 の追加分は §2b に記す。

### 2b. 作成・変更ファイル（委任 2）

- 新規: `scripts/goldens/recapture-cli.sh`（141 行）、`scripts/goldens/capture-cli.ts`（1,058 行）、
  `tests/golden/upstream-3c3146cf/cli/{provenance.json,cases-missing.json}` + 11 動詞 22 ケース
  （`<verb>/<case>/{argv,stdin,stdout.json,state.diff,audit.md,exit,stderr,case.json}`）、
  `tests/golden/upstream-3c3146cf/hooks/{provenance.json,cases-missing.json}` + 4 フック 14 ケース
  （`<hook>/<case>/{stdin.json,exit,stderr,stdout,audit.md,case.json}`）、
  `modules/shared/canon-json/tests/support/mod.rs`、`modules/shared/canon-json/tests/golden_corpus_read.rs`
- 変更: `tests/golden/upstream-3c3146cf/normalization.json`（規則の追加）、`tests/golden/upstream-3c3146cf/README.md`
  （cli / hooks 節の追記: 採取手順・来歴・非対話化 env 5 種・ケースレイアウト・BR2.4 の範囲表・C2 フック写像表）

### 2d. CLI / フックゴールデンの採取（委任 2）

- 取得: upstream `dist/claude`（262 ファイル、ツリーマニフェスト sha256 `ea223c42…`）を SHA 指定 shallow fetch
  （フォールバック: codeload tarball）。使い捨てワークスペースで bun 1.3.13 により実行。captured_at 2026-08-22T13:43Z。
- 非対話化 env: `AIDLC_SKIP_HUMAN_PRESENCE_GUARD` / `AIDLC_DISABLE_ENSEMBLE_EVIDENCE` /
  `AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD` / `AIDLC_SKIP_ARTIFACT_GUARD` / `AIDLC_DISABLE_USAGE_TRACKING`（README に表）。
- CLI 22 ケース: next 4 / intent-create 1 / continue 2 / report 5 / practices-promote 1 / skip 1 / jump 3 / recompose 2 /
  park 1 / unpark 1 / set-autonomy 1。欠落 2（理由付き）: `set-autonomy/gated` — ピン 3c3146cf では状態ファイル
  テンプレートに `Construction Autonomy Mode` 行が無く正常系に到達できない（upstream 既知バグ M12、逸脱台帳 #2 と
  整合）、`continue/multi-part` — 28 KiB 超の規則束が要る（U6 で合成入力を用意）。
- フック 14 ケース: stop-forwarding-loop 3 / record-human-turn 2 / state-transition-guard 4 / write-audit-log 5。
  欠落 1: `stop-forwarding-loop/transcript-carve-out`（本物のトランスクリプト JSONL が要る — U7 で合成）。
  C2 名 → upstream 実装ファイルの写像（`aidlc-continue-workflow.ts` / `aidlc-record-human-turn.ts` /
  `aidlc-state-transition-guard.ts` / `aidlc-write-audit-log.ts`）は README と provenance に記録。
- 正規化: `<TS>` / `<CLONE>` / `<ROOT>` / `<SESSION>`（規則は `normalization.json`）。コンダクタの走査で絶対パス・
  ホスト名・セッション ID の残存なし。
- 比較器テスト（`golden_corpus_read.rs` 9 本）: 全ケース読取可能、来歴あり、BR2.4 の範囲網羅、正規化規則の読込、
  環境固有値の置換、正規化の不動点性、欠落ケースの理由必須、行 diff の指示位置。

## 2c. TDD の証跡（委任 1）

| 層 | Red（失敗コマンドと要約行） | Green |
|---|---|---|
| Data model（Step 5 → 6） | `cargo test -p canon-json` → `test result: FAILED. 10 passed; 18 failed;`（ObjectMembers の挿入順・置換・get・len / JsonValue の構造的同値 / ToValueError の Display / SerializationProfile の属性 / Digest の表記 / ParseError の Display） | `test result: ok. 28 passed; 0 failed;` |
| Business logic（Step 8 → 9） | lib: `test result: FAILED. 36 passed; 31 failed;`（writer 13 / canonical 4 / digest 3 / parse 11）。受入表: `cargo test -p canon-json --test golden_hash_canonical` → `test result: FAILED. 2 passed; 5 failed;`（各観測で 30 / 30 行が不一致） | lib `test result: ok. 67 passed;` / 受入表 `test result: ok. 7 passed;` |
| API（Step 12 → 13） | `test result: FAILED. 80 passed; 7 failed;`（struct の宣言順 / ネスト / None → null / 数値の変種対応 / 動的マップの挿入順 / 非文字列キーの拒否 / to_value → serialize の端から端） | `test result: ok. 87 passed; 0 failed;` |

委任 2（Step 18、比較器）: スタブ（空を返す実装）+ `golden_corpus_read.rs` を先に置き `cargo test -p canon-json --test
golden_corpus_read` → `test result: FAILED. 1 passed; 8 failed` を記録 → 実装して `test result: ok. 9 passed; 0 failed` →
Refactor で clippy `missing_const_for_fn` 3 件を `const fn` へ（コミット `d1688a9` 本文より。委任 2 の最終報告は
コンダクタへ届かず、証跡はコミット本文とコンダクタの再実行で確認した）。

Red は「コンパイルは通るが値が誤っているスタブ」を置いて失敗行を記録する方式（Rust では新規型のテストを先に書くと
コンパイルエラーになり `test result: FAILED` 行が出ないため）。公開面ガード 2 本（`facade_tests::*`）は退行防止の
ガードで初回から緑（Red を経ていない — 報告に明記）。PBT 11 本は実バグ（serde_json の浮動小数読取の最終桁 1 ULP ずれ）
を検出し、`float_roundtrip` フィーチャで解消。再現シードは `proptest-regressions/` にコミット済み。

## 3. 主要な実装判断

- **数値**: `Number { PosInt(u64), NegInt(i64), Float(f64) }`。|v| > 2^53 の整数は JS と同じく f64 経路で書く
  （`9007199254740993` → `9007199254740992`）。f64 は `format!("{:e}")` の最短桁 + 指数から ECMA-262
  `Number::toString` の 4 規則で組み立てる（`1e+21`、`1e-7`、`-0` → `0`、非有限 → `null`）。
- **キー順**: `canonical::member_order` — integer-like（0〜2^32-2 の正準十進）は数値昇順で先頭、残りは
  contract-* では挿入順、hash-canonical では UTF-16 コード単位順（upstream `Object.keys().sort()` +
  `Object.fromEntries` の実測と一致）。
- **深さ上限**: `parse` 前に文字列リテラル外の `{`/`[` を数え、**128 段目に達した時点で** `TooDeep { limit: 128 }`
  （serde_json 既定の再帰上限と同じ判定点 — 127 段まで受理）。
- **preserve_order + float_roundtrip**: `serde_json` を `[workspace.dependencies]` に一元化。`float_roundtrip` は
  PBT で検出された最終桁 1 ULP のずれ（既定の最善努力精度）を JS の正しい丸めに合わせるために追加（NFR1）。
- **機械強制**: `clippy.toml` の `disallowed-methods` で `serde_json::to_string*` / `to_vec*` / `to_writer*` /
  `to_value` を拒否。canon-json 内の唯一の呼出点（`value::to_value`）のみ理由付き allow。
- **ファサード**: `lib.rs` の `pub use` 列挙だけが公開面。テスト `the_facade_publishes_exactly_the_declared_surface`
  と `every_module_declaration_is_private` が `lib.rs` 自体を読んで固定。

## 4. 棚卸し（計画 §4、委任 1 の実測）

| 項目 | 実測 |
|---|---|
| I1 最大ネスト深さ | 契約 JSON 全体で **5 段**（stage-graph 5 / scope-grid 4 / harness 2 / model-rates 4 / ars-priors 5）、ゴールデン入力は最大 8 段。受入上限（127 段）に対し十分な余裕。上限の引き上げ不要 |
| I2 integer-like キー | **実在**: `.claude/tools/data/ars-priors.json` の `$.evThresholds` に `"1"`〜`"5"`。BR1.2（数値昇順で先頭寄せ）で写像、元から昇順なので出力上は同一。受入表 `contract-observed` クラスに固定 |
| I3 キーの文字集合 | 非 ASCII キー 0 件（8 ファイル）。UTF-16 順 ≠ コードポイント順のケースは受入表で明示的に固定（前提が崩れても検出可能） |
| I4 浮動小数フィールド | **実在**: `model-rates.json` 40 箇所・`ars-priors.json` 11 箇所、相異なる値 22 種（0.1〜50.0）。受入表 `contract-observed` に全数固定 |
| I5 `serde_json::to_*` / `to_value` 直接呼出 | 0 件（既存は `from_str` のみ）。lint 導入による既存コード修正なし |
| I6 `preserve_order` の影響 | 既存 234 テスト全緑（`float_roundtrip` 追加後も再確認） |
| I7 `components.md` の CanonJson 外部依存 | コンダクタが更新済み（`[serde, serde_json(preserve_order, float_roundtrip), sha2]`） |

注: `.claude/tools/data/scopes/` は本インストールに存在しない（スコープ定義は `.claude/scopes/*.md`、契約 JSON ではない）。
当初想定「契約 JSON に integer-like キー・浮動小数は現れない」（ADR 0001 決定 3・4 の棚卸し前提）は実測で否定された。

## 5. テストとカバレッジ

- `cargo test -p canon-json`: ユニット + PBT 87、受入表統合テスト 7、コーパス読取・比較器 9、doc test 1（合計 104、コンダクタ実測）。
- 受入表: 32 ケース（クラス: nesting 2 / integer-like-keys 4 / non-finite 3 / negative-zero 2 / exponent 3 /
  large-integers 3 / non-ascii 3 / escape 2 / empty 4 / struct-field-order 1 / float-integral 1 / scalar 1 /
  duplicate-keys 1 / contract-observed 2）、全行一致。
- PBT 11 本（writer 4 / parse 4 / digest 3）。ワークスペース合計 329 テスト（着手前 234 → +95）。
- カバレッジ: `cargo llvm-cov -p canon-json --summary-only` = line 98.59% / region 98.46% / function 97.59%
  （コンダクタ再実測も同値）。`bash scripts/coverage.sh` = workspace 97.06%（`[PASS]`、床 90%）。

## 6. 計画からの逸脱

- `to_value` は `Result<JsonValue, ToValueError>`（functional-spec §2 は infallible 表記 — `unwrap` 禁止の帰結、計画 §2 に記載済み）。
- `parse_bytes` を追加（`ParseError::Encoding` の到達可能化、計画 §2 に記載済み）。
- `Indent` / `KeyOrder` 列挙を公開面に含めた（`SerializationProfile` のアクセサ戻り値型）。
- 受入表のフィールド名は C7 の平板表記ではなく `expected: { canonical_output, canonical_digest, compact_output,
  compact_digest_prefixed, compact_digest_hex, pretty_output }` の入れ子 — C7 を実体に合わせて改訂済み（監査メモあり）。
- `serde_json` に `float_roundtrip` フィーチャを追加（計画外、理由は §3。`tech-stack-decisions.md` §2 の依存差分表への反映は
  ステージゲートで提案）。
- `ParseError` / `ToValueError` に `std::error::Error` を手実装（消費側で `?` を使えるようにするため。thiserror / anyhow は不使用。
  既存コードベースに前例がなく house style の裁定が必要 — ゲートで確認）。
- 深さ上限の実効値は **127 段受理 / 128 段目で TooDeep**（serde_json の上限値 128 と同じ判定点。計画・security-design §2 の
  「128 超」という文面より 1 段厳しい — 文面の更新は U9 / ゲートで確認）。
- `sha2` は 0.10 系（実績優先）。受入表は 30 → 32 ケース（contract-observed 追加）。
- 委任 2: `regex` を canon-json の **dev-dependency** に追加（比較器の正規化用。ランタイム依存 3 つは不変 — NFR4.1）。
  `normalization.json` に規則 4 本を追加（記録ディレクトリの日付スタンプ 2 本、継続トークン、監査シャード名の実行時
  literal 置換）し適用順序を明文化。再採取を 2 回まわして `captured_at` 以外の全バイト一致を確認（BR2.5 の再現性）。
- 計画ファイルのチェックボックス更新は承認指紋を壊すため差し戻し（`d6856d5`）。

## 7. 後続への引き渡し

- U9（canon-docs）: ADR 0001「未確定事項」(a)〜(e) の確定値は README 節に記録済み。
- U3 / U6 / U7: `WorkflowDefinitionRepositoryImpl` の serde_json 直接利用の置換、continue_token、CLI 出力は
  それぞれの Unit で canon-json 経由に。
- U10: `cargo audit` / `rust-toolchain.toml` / `unsafe_code` workspace lint 昇格、PBT シード固定。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-22T14:04:23Z
**Iteration:** 1（advisory, unit: u1-canon-json-goldens）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | git 履歴 `main-sync..HEAD`（`64322d0` → `7999f81`）vs `nfr-requirements/security-requirements.md` NFR2.1、`traceability.json` NFR2.1 | NFR2.1 の合格基準は「受入表テストが実装前に存在し**失敗することを PR 履歴で確認できる**」。実際には受入表データ `hash-canonical/cases.json` は `64322d0` で先行しているが、受入表**テスト** `modules/shared/canon-json/tests/golden_hash_canonical.rs` は writer / canonical / digest / parse と**同一コミット `7999f81`** に入っており、テストが在って実装が無いコミットは履歴上 1 つも存在しない（Data model / Business logic / API の各 Red も同様に 1 コミットへ畳まれている）。Red の証跡は `code-summary.md` §2c の散文のみで、これは `unit-test-instructions.md` §2 が定める方式（要約行を code-summary に写す）とは整合するが、NFR2.1 の文面とは食い違う。`traceability.json` は NFR2.1 を `OK` → `golden_hash_canonical.rs` として計上しており、成果物が証明できる範囲を上回っている。 | ゲートでどちらかに裁定する — (a) 散文証跡を正式な証跡と認め NFR2.1 の合格基準を「code-summary §2c に失敗出力を記録」へ改める（併せて traceability の NFR2.1 に注記）、または (b) 後続 Bolt では Red を独立コミットにする運用を team.md / project.md へ確定させる。実装そのものの修正は不要。 |
| 2 | Minor | `modules/shared/canon-json/src/lib.rs:145`（`pub use parse::{MAX_DEPTH, …}`）vs `code-generation-plan.md` §2、`code-summary.md` §6 | 公開面は承認済み計画 §2 の 14 項目に対し実装は 17 項目。`Indent` / `KeyOrder` は §6 の逸脱台帳に記載があるが、**`MAX_DEPTH` は計画 §2 にも §6 にも無い**（`facade_tests::DECLARED_SURFACE` とクレート rustdoc には載っており、実装内では自己整合している）。同様に `to_value<T: Serialize + ?Sized>` は計画の `to_value<T: Serialize>` を緩めている（無害な拡張だが未記載）。公開 API の拡大は消費側（U3 / U6 / U7）の契約面なので、台帳に無い拡大は次の Unit が気づけない。 | `code-summary.md` §6 の逸脱台帳に `MAX_DEPTH` の公開と `?Sized` の 2 行を追加し、ゲートで公開面 17 項目として承認を取る。 |
| 3 | Minor | `modules/shared/canon-json/tests/golden_corpus_read.rs`（`missing_cases_are_recorded_with_a_reason` の `assert!(!entries.is_empty(), …)`） | 欠落ケース一覧が**空でないこと**をテストが要求している。欠落 3 件（`set-autonomy/gated` / `continue/multi-part` / `stop-forwarding-loop/transcript-carve-out`）はいずれも `follow_up` で U6 / U7 が追加採取すると明記されており、その追加採取が完了するとこのテストはコーパスが改善したのに落ちる。ゴールデンを直さず実装を直すという BR2.3 / BR2.5 の運用と噛み合わない罠になる。 | 「空でない」検査を外し、各エントリの必須フィールド検査だけ残す。件数を固定したいなら `entries.len() == provenance["missing_case_count"]` の整合検査に置き換える。 |
| 4 | Minor | `tests/golden/upstream-3c3146cf/normalization.json`（最後の `<CLONE>` regex `[A-Za-z0-9][A-Za-z0-9_.-]*-[0-9a-f]{8}`） | 「末尾が `-` + 16 進 8 桁」というだけの広いパターンで、upstream の記録ディレクトリ名の規約 `<slug>-<id8>`（id8 は 16 進 8 桁）や `upstream-3c3146cf` のような文字列もマッチする。現コーパスには `<CLONE>` の出現が 0 件で固定点テストも通っているため**現時点の隠蔽は無い**が、U6 / U7 が同じ規則で実装出力を比較するときは記録ディレクトリ名の差分が黙って `<CLONE>` に潰れうる。NFR1.3「正規化規則の適用前後で差分を隠さない」に対する将来リスク。 | シャード名の文脈に錨を打つ（`audit/` パス配下またはシャードファイル名に限定する）か、先行する `runtime-clone` の literal 置換だけに任せて広い regex を落とす。判断は U6 / U7 の採取前で足りる。 |
| 5 | Minor | `modules/shared/canon-json/tests/golden_corpus_read.rs`（`the_br2_4_range_is_covered`）vs `functional-design/rules.md` BR2.4 | BR2.4 はフック 4 本について「許可 / 拒否 / 無視 を 2〜3 件ずつ」と書くが、テストは**フックあたりの件数 ≥ 2** しか見ていない。実測は `record-human-turn` 2 件（いずれも exit=0、拒否なし）、`write-audit-log` 5 件（拒否なし）、`stop-forwarding-loop` 3 件（拒否は exit ではなく stdout の `{"decision":"block"}` で表現）、`state-transition-guard` 4 件（許可 1 / 拒否 2 / 無視 1）。構造上「拒否」経路を持たないフックがあるためこれ自体は妥当だが、どのフックのどの区分が N/A なのかがテストにも README にも書かれておらず、「拒否ケースが無い」のか「採り漏らした」のかを後続 Unit が区別できない。 | README の C2 フック写像表に区分（許可 / 拒否 / 無視）の該当・N/A 欄を 1 列足すか、テスト側の定数に区分の期待値を持たせる。 |
| 6 | Minor | `modules/shared/canon-json/src/parse.rs`（`MAX_DEPTH` の rustdoc）、`code-summary.md` §3 | 「`serde_json` は 128 段目で自前の再帰エラーを返す」「serde_json 既定の再帰上限と**同じ判定点**」という説明が 1 段ずれている。`serde_json` の `RECURSION_LIMIT = 128` は 128 段までを受理し 129 段目で失敗するのに対し、`check_depth` は 128 段目で `TooDeep` を返す（127 段まで受理）。**挙動は正しく安全**（serde_json の再帰エラーが表に出る前に決定的に弾くという NFR4.3 の意図を満たす）で、理由付けの散文だけが不正確。`security-design.md` §2 / 計画の「128 超」という文面との差は §6 で既に逸脱として挙げ U9 / ゲートへ送っている。 | rustdoc と §3 の一文を「serde_json の上限 128 の 1 段手前で弾く」に直す（U9 の文面更新と同じ場所でよい）。 |

### Validation Tool Results

| 実行したもの | 結果 | 解釈 |
|---|---|---|
| `cargo fmt --all --check` | PASS（exit 0、差分なし） | 整形は規約どおり |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS（警告 0） | workspace lints 47 ルール（`missing_docs` / `unwrap_used` / `expect_used` / `unreachable_pub` 等）と `clippy.toml` の `disallowed-methods` 7 件を満たす |
| `cargo lint`（`tools/lint` カスタムリンター） | PASS（出力なし） | coding-rules 機械強制 3 ルール（`no-public-fields` 等）に違反なし |
| `cargo test -p canon-json` | PASS 104 件（lib 87 / `golden_corpus_read` 9 / `golden_hash_canonical` 7 / doc 1） | `code-summary.md` §5 の申告値と完全一致 |
| `cargo llvm-cov -p canon-json --summary-only` | line **98.59%** / region **98.46%** / function **97.59%** | §5 の申告値と完全一致。最低は `parse.rs` の line 95.19%、`writer.rs` は 100% |
| PBT 本数の実測（`cargo test -p canon-json --lib -- --list`） | 11 本（writer 4 / parse 4 / digest 3） | §5・§2c の申告と一致。`proptest-regressions/{parse,digest}.txt` に再現シード 4 件がコミット済み（`float_roundtrip` で解消した 1 ULP ずれの実在を裏づける） |
| 来歴の**独立**再検証（`curl` で upstream を再取得し sha256 照合） | 一致 — ファイル `99528925…196cb9`、`sed -n '104,123p'` 抽出スニペット `c8894a43…04418f` | `hash-canonical/provenance.json` と `recapture-hash-canonical.sh` の固定値が実物と一致。取得した 104-123 行は `canonicalize` / `sha256` / `hashObject` そのもので、`hashObject` = `sha256:` + hex(sha256(JSON.stringify(canonicalize(v)))) が BR1.6 正準族の定義と一致する。**ゴールデンは捏造ではなく実採取である** |
| README のバイト不変（BR2.5 / C7） | 一致 — `main-sync` 版 3,802 バイトの sha256 `f85ac329…ab5e39` と現行先頭 3,802 バイトの sha256 が同一 | 既存部分は 1 バイトも動いていない。`stage-graph.json` / `scope-grid.json` は `git diff` 空 |
| コーパスの環境固有値残存（NFR4.4） | 検出 0 | `/Users/`・`/home/`・`/private/var/folders`・ホスト名・ユーザ名・`/tmp/` の残存なし。生 ISO タイムスタンプは `provenance.json` / `case.json` の `captured_at` のみ（比較対象外のメタデータ）。フィクスチャは upstream 既定の memory を使っており、本プロジェクト固有の日本語 memory 内容の混入も 0 件 |
| `traceability.json` の 26 target 実在確認 | 26/26 実在 | 全 target がワークスペース上の実ファイルへ解決する（Unit ID ではなくファイルパス指定で単一 target 規約も満たす） |
| BR2.3 受入表クラス網羅 | 32 ケース / 14 クラス | 計画 §3 BR2.3 の要求クラス（nesting / integer-like-keys / non-finite / negative-zero / exponent / large-integers / non-ascii / escape / empty / struct-field-order / float-integral）を全数充足。追加で scalar / duplicate-keys / contract-observed |
| BR2.4 CLI 範囲 | 11 動詞 22 ケース、`report` は awaiting-approval / approved / rejected / revised を全数保持 | BR2.4 の最小集合を満たす。欠落 2 + 1 はいずれも `reason` / `evidence` / `follow_up` 付きで記録され、捏造なし（所見 5 は区分の可読性の話） |
| コーディング規則の突合 | 適合 | `#![forbid(unsafe_code)]` あり。`pub struct` の全フィールドが private（`Digest` / `ObjectMembers`、enum 変種フィールドは field-visibility.md により対象外）。6 モジュールすべて private + `lib.rs` の `pub use` 列挙のみ（`facade_tests` 2 本が `lib.rs` 自体を読んで固定）。`ParseError` / `ToValueError` は手実装 enum + `fmt::Display`（thiserror / anyhow 不使用）。プロダクトコードに `unwrap` / `expect` なし |
| C7 / `components.md` の追随 | 実施済み | `contract-summary.md` C7 の layout をコーパスの実体（`cases.json` の入れ子 `expected`、cli / hooks の 8 ファイル構成）へ改訂、`components.md` の CanonJson `external_dependencies` を実依存へ更新（計画 §4 I7） |

### Summary

品質ゲート・カバレッジ・テスト本数の申告値はすべて再実行で一致し、最も重要な「ゴールデンが本物か」は upstream を独立に再取得して sha256 を突き合わせることで確認できた（ファイル・抽出スニペットとも完全一致、`hashObject` の定義も BR1.6 と一致）。README の既存 3,802 バイトと配布実バイト 2 ファイルは不変で、コーパスに環境固有値の残存もない。公開面・モジュール private 化・手実装エラー enum・`forbid(unsafe_code)` などコーディング規則の突合も適合しており、開発者が追加の設計判断なしに次の Unit へ進める状態にある。唯一 Major としたのは NFR2.1 の合格基準「Red を PR 履歴で確認できる」が commit 粒度の畳み込みで満たされていない点で、実装欠陥ではなく証跡方式の裁定事項（散文証跡を認めるか、以後 Red を独立コミットにするか）である。残る Minor 5 件は、逸脱台帳への `MAX_DEPTH` 追記、欠落ケース非空アサートの緩和、`<CLONE>` 正規化パターンの絞り込み、フック区分の可読化、深さ上限の説明文の 1 段ずれ訂正で、いずれもゲート後の小修正で足りる。
