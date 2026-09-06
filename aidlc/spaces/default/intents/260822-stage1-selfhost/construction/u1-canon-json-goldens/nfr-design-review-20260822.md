# U1 非機能設計の旧レビュー保存記録

2026-09-06に旧Reviewを保存した。旧ID1・2をR-01・R-02へ対応付け、今回の独立レビューで状態を確認する。現在の承認ではない。

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
