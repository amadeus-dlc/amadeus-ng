# security-design — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> NFR Design（Construction 3.3）成果物（Unit: U1、kind: library）。出典: `../nfr-requirements/security-requirements.md`
> （NFR1.1〜1.3 / NFR2.1〜2.3 / NFR4.1〜4.4、STRIDE）、`../nfr-requirements/tech-stack-decisions.md`（sha2 / serde +
> serde_json preserve_order / 自前ライタ / clippy disallowed-methods）、`../functional-design/functional-spec.md`（W1〜W5）、
> `../../../inception/contract-design/contract-summary.md`（C1 / C7）、確認事項 `nfr-design-questions.md`（前提 P1〜P3）。
> performance / scalability / reliability / observability の要求・設計は kind = library のため存在しない。
>
> 設計ステージの制約に従い、コードは ≤15 行の例示のみ。

## 1. 設計方針

U1 はネットワーク・認証・認可・永続化を持たない純粋ライブラリ。セキュリティ設計は 3 点に絞る:
**(a) 入力検証を 1 つの境界に集約する**、**(b) 出力を決定的にする**（規則に従えば同じ入力は同じバイト列 — ゴールデン
で固定）、**(c) サプライチェーンを最小に保つ**。

## 2. 入力検証の設計（NFR4.3）

- 境界は `parse(text) -> Result<JsonValue, ParseError>` の 1 か所。呼出側はこの境界を通った `JsonValue` だけを扱い、
  内部で再検証しない（境界を信頼する）。
- `ParseError` は変種で原因を区別する（文言はアダプタ層で付ける — 材料のみ保持）:
  - `Syntax { offset, detail }` — 不正 JSON
  - `TooDeep { limit: 128 }` — 再帰深さ上限（serde_json 既定 128）超過。スタック枯渇の防止
  - `Encoding` — 不正 UTF-8
- 深さ上限と互換（NFR 要求レビュー Minor 1 の引き取り）: upstream の `JSON.parse` はエンジンのスタック依存で 128 段
  超も読めるため、128 段超の契約 JSON が存在すれば拒否 vs 成功の非互換になる。code-generation の計画で対象の契約
  JSON 群（stage-graph / scope-grid / scopes / directive / ゴールデン入力）の**実測最大深さ**を棚卸しし、十分に浅い
  （想定: 10 段未満）ことを確認する。超える場合のみ上限を引き上げる（`serde_json::Deserializer::disable_recursion_limit`
  は使わない — 上限の数値を設定で持つ）。
- 非有限数（NaN / ±Infinity）は入力として JSON に現れない（JSON 文法外）。出力側で f64 から生じ得る場合は BR1.3 で
  `null` に決定的に落とす。制御文字は BR1.4 の最小エスケープで決定的に処理。

```text
// 境界の形（例示）
fn parse(text: &str) -> Result<JsonValue, ParseError>;   // 唯一の入口
enum ParseError { Syntax { offset: usize, detail: String }, TooDeep { limit: usize }, Encoding }
```

## 3. 出力の決定性（NFR1.1 / NFR1.2 / NFR1.3）

- 直列化は純粋関数（同じ `JsonValue` + 同じプロファイル → 同じバイト列）。乱数・時刻・環境変数に依存しない。
- ダイジェストは直列化バイト列から計算し、族ごとのプロファイル（正準族 = hash-canonical、非正準族 = contract-compact）を
  型で固定する（`Digest { family, hex }` — 族の取り違えを型で防ぐ）。用途 → 族の対応表は functional-spec W2 の補遺として
  code-generation の計画に載せる（functional-design レビュー Minor 2 の引き取り）。
- ゴールデン比較はプレースホルダ 4 種（<TS>/<CLONE>/<ROOT>/<SESSION>）だけを正規化し、規則はコーパスに固定する。

## 4. サプライチェーン（NFR4.1 / NFR4.2）

- 追加依存は `sha2`（ハッシュ）・`serde` / `serde_json`（読取、`preserve_order`）のみ。`cargo audit`（U10 で CI 追加）の
  対象。バージョンは `Cargo.lock` で固定。
- `unsafe_code = "forbid"`（U10 でワークスペース lint へ昇格）。canon-json は unsafe を使わない。
- `serde_json` の直列化関数の直接呼び出しは clippy `disallowed-methods` で canon-json 以外から拒否（BR1.7）。
- bun（ゴールデン採取）は開発時ツール。プロダクトの依存木に入れない（D1）。

## 5. 秘密情報・データ（NFR4.4）

- 秘密情報・PII を扱わない。ゴールデンは使い捨てワークスペースで採取し、正規化でホスト名・パス・ID を除く。
  レビューで目視確認（BR2.1 の来歴と併せて）。
- ログ出力なし（ライブラリはログを書かない。診断は呼出側の `AIDLC_LOG`）。

## 6. 失敗の扱い（前提 P3）

- 失敗はすべて `Result` で呼出側に返す。`unwrap` / `expect` はプロダクトコードで禁止（workspace lint）。
- 沈黙の失敗なし: ゴールデン不一致はテスト失敗、採取失敗は欠落として記録（捏造しない）。

## 7. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR1.1 | 純粋な直列化関数 + 3 プロファイル + ゴールデン先行（§3） |
| NFR1.2 | `Digest { family }` で族を型固定、用途 → 族表（§3） |
| NFR1.3 | 正規化 4 種のみ、規則はコーパス固定（§3） |
| NFR2.1 / NFR2.2 / NFR2.3 | ゴールデン先行 TDD、カバレッジ 90% 床、PBT（往復・決定性・冪等性） — logical-components §3 のテスト配置 |
| NFR4.1 / NFR4.2 | 依存 3 つ・cargo audit・unsafe forbid（§4） |
| NFR4.3 | parse 境界 1 か所・ParseError 変種・深さ上限 128 + 実測棚卸し（§2） |
| NFR4.4 | 使い捨てワークスペース採取・正規化・レビュー（§5） |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-22T12:07:54Z
**Iteration:** 1（advisory, unit: u1-canon-json-goldens）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Minor | security-design.md §4／logical-components.md §1（vs `inception/domain-design/components.md` CanonJson エントリ） | 上流の `components.md` は CanonJson の `external_dependencies: []` を明示しているが、本 Unit の nfr-design（security-design §4、tech-stack-decisions §1〜2）は `sha2` / `serde` / `serde_json` を実際の外部クレート依存として新規追加している。前段の nfr-requirements レビューは `depends_on: []`（内部コンポーネント間依存）と外部クレート依存が別軸であることを確認済みだが、`external_dependencies: []` というフィールド自体の陳腐化には触れていない。設計そのものは矛盾しないが、上流の inception 成果物が古い値のまま残る点は棚卸し漏れのリスクになる。 | code-generation の計画（すでに §2 で棚卸し事項を list 済み）に一行追加し、`components.md` の `external_dependencies` を実体化後に更新する対象として明記する。 |
| 2 | Minor | security-design.md §4 | `serde_json` の直接呼び出し禁止を「直列化関数」に限定して記述しており、`rules.md` BR1.7 が同じ clippy `disallowed-methods` で禁止している「契約経路の `to_value`」への言及が抜けている。設計は矛盾していないが、この成果物単体を読む実装者には禁止範囲が直列化関数だけに見える。 | §4 の当該箇条に「契約経路の `to_value` の直接呼び出しも同じ機構で禁止する（BR1.7）」を一言追記する。 |

### Validation Tool Results

本ステージの `sensors`（required-sections / upstream-coverage / linter / type-check / traceability）はいずれも自動実行可能な CLI ツールとして本レビューには提供されておらず、実行結果は無い。以下は手動でのクロスチェック結果。

| チェック | 結果 | 所見 |
|---|---|---|
| traceability.json の 10 件（NFR1.1〜1.3 / NFR2.1〜2.3 / NFR4.1〜4.4）が security-design.md / logical-components.md の該当節に実在するか | 一致 | 全 target が指す §（security-design §2〜§5、logical-components §1・§4）は実在し、内容も要求と矛盾しない |
| 前段 security-requirements.md レビュー Minor 1（深さ上限 128 の upstream 互換影響）の引き取り状況 | 引き取り済み | security-design §2 に「契約 JSON の実測最大深さを code-generation で棚卸し」を明記しており、未対応のまま放置されていない |
| logical-components のモジュール分割（value/profile/writer/canonical/digest/parse + facade）と `coding-rules/module-visibility.md`（mod は private、公開は facade の `pub use` 列挙、利便再エクスポート禁止）の整合 | 整合 | 6 モジュールすべて private・`lib.rs` の `pub use` 列挙のみを公開面とし、便宜的な再エクスポートを設計していない |
| logical-components §1 のゴールデン配置と `inception/contract-design/contract-summary.md` C7（`tests/goldens/{hash-canonical,cli,hooks}/...`）の整合 | 一致 | layout・owner（U1）・consumers（U6/U7）の記述が C7 と食い違わない |
| ADR 0001（単一クレート・自前ライタ・serde_json 直接呼び出し禁止）との整合（tech-stack-decisions.md 経由） | 整合 | クレート配置・書き出し方式・機械強制の選定理由が ADR の決定と代替案却下理由に対応している |
| コード例示が ≤15 行制約を守っているか | 準拠 | security-design.md 中のコードブロックは 1 件（4 行）のみ |
| infrastructure-design SKIP との整合（logical-components §5） | 整合 | U1 はインフラ資源を持たないため引き渡し事項なしと明記しており、CI（U10）側の関係のみ記載 |

### Summary

セキュリティ設計・論理コンポーネント分割・traceability はいずれも上流の NFR 要求・functional-design・coding-rules・ADR 0001 と矛盾なく、実装者が追加で設計判断を仰ぐ必要はない。上流成果物（`components.md`）のフィールド陳腐化と BR1.7 の記述範囲に軽微な精度上の抜けがあるが、いずれも Minor でブロッキングではない。
