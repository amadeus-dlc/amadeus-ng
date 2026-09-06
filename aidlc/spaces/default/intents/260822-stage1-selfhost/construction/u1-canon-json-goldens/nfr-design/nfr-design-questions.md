# nfr-design-questions — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> NFR Design（Construction 3.3）の質問票（Unit: U1、kind: library）。出典: `../nfr-requirements/security-requirements.md`
> （NFR1.x / NFR2.x / NFR4.x）、`../nfr-requirements/tech-stack-decisions.md`、`../functional-design/functional-spec.md`
> （W1〜W5）、`../../../inception/contract-design/contract-summary.md`（C1 / C7）。性能の数値目標はないがNFR5.1の劣化測定・記録を適用する。scalability /
> reliability / observability の常駐サービス向け要求は対象外で、本ステージの成果物は `security-design.md` /
> `logical-components.md` / `traceability.json` の 3 つ。
>
> **質問なし。** 設計パターンの選択余地（耐障害・スケール・キャッシュ・観測）は純粋ライブラリには無く、セキュリティ
> 設計（境界での入力検証・深さ上限・秘密情報なし）と論理コンポーネント分割（canon-json クレート内のモジュール境界と
> ゴールデン比較器の置き場）は NFR 要求・技術選定・ADR 0001 から一意に決まる。次の前提を確認して成果物へ進む。

## 以前の前提（2026-08-22の記録）

以下は当時の記録として保存する。現行実装との不整合は末尾の2026-09-06確認要約で訂正し、過去の回答を今回の確認には流用しない。

- P1. セキュリティ設計: 入力検証は `parse` の境界 1 か所に集約（不正 JSON → `ParseError`、再帰深さ上限 128 →
  `ParseError::TooDeep` として明示、BR1.3 / BR1.4 の決定的処理）。契約 JSON の実測最大ネスト深さを code-generation
  の計画で棚卸しし、128 を十分下回ることを確認する（NFR 要求レビュー Minor 1 の引き取り）。秘密情報・PII なし。
  依存は sha2 / serde / serde_json のみで `cargo audit` 対象。
- P2. 論理コンポーネント: `canon-json` クレート内を `value`（JsonValue）/ `profile`（3 プロファイル）/ `writer`
  （体裁・数値・エスケープ）/ `canonical`（再帰ソート）/ `digest`（2 族）/ `parse` の 6 モジュールに分け、公開 API は
  ファサード（`lib.rs` の `pub use`）経由のみ（module-visibility）。ゴールデン比較器（normalize + diff + コーパス読取）
  は `tests/goldens/` 配下のテスト支援クレート（`dev-dependency`）に置き、プロダクトクレートに混ぜない。
- P3. 障害ドメイン: ライブラリは状態を持たず、失敗は呼出側へ `Result` で返す（沈黙の失敗なし、`unwrap` / `expect`
  禁止）。非有限数 → `null` は失敗ではなく規則（BR1.3）。

## 以前に確認済みのまとめ

- U1 に固有の NFR 設計質問はなし。耐障害・スケール・キャッシュ・観測のパターンは純粋ライブラリに不要
- セキュリティ設計（P1）: 入力検証は parse の境界 1 か所、再帰深さ上限 128 は ParseError::TooDeep として明示、契約 JSON の実測深さを code-generation で棚卸し、秘密情報・PII なし、依存 3 つは cargo audit 対象
- 論理コンポーネント（P2）: canon-json を value / profile / writer / canonical / digest / parse の 6 モジュールに分け、公開はファサード経由のみ。ゴールデン比較器はテスト支援側に置く
- 障害ドメイン（P3）: 状態なし、失敗は Result で返す（unwrap / expect 禁止）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Consolidated Summary Confirmation

2026-09-06の再確認。更新済みの品質・安全性要件を、security-design.md・logical-components.md・traceability.jsonの3成果物へ具体化する。

- 入力経路を分ける。`parse(&str)`は深さ検査とJSON構文検査、`parse_bytes(&[u8])`はUTF-8検査後にparseへ委譲、`to_value`は型付き値の変換失敗を返す。直接構築したJsonValueまでparseの深さ検査済みとは扱わない。
- 深さは127段まで受理・128段以上拒否とし、孤立サロゲートはSyntax、不正UTF-8はparse_bytesのEncoding、変換不能な型はToValueErrorで区別する。入力88ファイルの実測を利用し、将来の入力に合わせて上限を自動で引き上げる設計にはしない。
- 論理コンポーネントを現行のcore-infrastructure内canon_jsonへ揃える。value/profile/writer/canonical/digest/digest_family/parseとmod.rsの公開面を記載し、canonicalの整数形式キー優先が全プロファイルに適用されることを明記する。ゴールデンはtests/golden/upstream-3c3146cf、比較器はmodules/core/infrastructure/tests/supportに置く。
- 通常の読取・変換エラーの伝播と、同一プロセス内のメモリ枯渇等を区別する。Resultだけで障害を隔離できるとは主張しない。ハッシュ族のタグだけで用途の取り違えを防げるとはせず、W2の用途表とテストで確認する。
- 型付き値の変換を含む直列化境界の機械強制、採取データと来歴の点検、固定シードの性質検証、カバレッジ・依存検査を設計へ対応付ける。NFR5.1の性能劣化時の測定も対応表に加える。新たなクラウド資源やAWS Bedrockは導入しない。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
