# U1 品質・安全性要件の旧レビュー保存記録

2026-09-06に旧Reviewを保存した。数値ID 1・2はR-01・R-02へ対応付け、今回の独立レビューで状態を確認する。これは現在の承認ではない。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-22T11:54:57Z
**Iteration:** 1（advisory, unit: u1-canon-json-goldens）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Minor | security-requirements.md §3 STRIDE 表（Denial of Service 行）、NFR4.3 | 再帰深さ上限 128（serde_json 既定）を DoS 対策としてのみ扱っており、NFR1（upstream 互換）への影響評価が抜けている。JS の `JSON.parse` は事実上エンジンのコールスタック依存で 128 段より深いネストも解釈できるため、upstream が生成し得る契約 JSON が 128 段を超えた場合、canon-json はそれを `ParseError` で拒否し upstream とバイト一致互換が崩れる（拒否 vs 成功の非互換）。本 Unit の実際の契約 JSON（stage-graph 等、33 ノード規模）が浅い構造であることは自明ではなく、文書上その根拠が示されていない。 | NFR1.1 か NFR4.3 のいずれかに一言追記する — (a) 対象の契約 JSON 群の実測最大ネスト深さが 128 を十分下回ることを棚卸しして明記する、または (b) 128 段超過時の `ParseError` 拒否を意図的な非互換として `docs/specs/deviations.md` 相当の扱いにするかを明示する。 |
| 2 | Minor | security-requirements.md §3 STRIDE 表（Repudiation 行） | 「該当」列を「該当なし」としつつ、「扱い」列で来歴（commit / captured_at / command、BR2.1）による追跡可能性を緩和策として記載しており、適用可否の判定と記述内容が食い違って見える。 | 「該当なし」を「該当（軽微、ゴールデン採取のみ）」等に改めるか、「該当なし」の理由（誰の行為の否認を防ぐ話ではない、等）を一言添えて矛盾を解消する。 |

### Validation Tool Results

本ステージ定義の `sensors` は required-sections / upstream-coverage / linter / type-check / traceability であり、実行可能な自動検証ツール（lint/type-check 系）は本 Markdown/JSON 成果物には該当しない。手動でのクロスチェックは以下のとおり実施した。

| チェック | 結果 | 所見 |
|---|---|---|
| traceability.json の target（NFR1.1〜1.3 / NFR2.1〜2.3 / NFR4.1〜4.4）が security-requirements.md §2 の表に実在するか | 一致 | 全 target ID が §2 の表に存在し、ID 単位で 1:1 対応している |
| NFR1 / NFR2 / NFR4 の文言・数値（カバレッジ 90% 床・cargo audit・unsafe forbid・upstream 互換 D6 範囲）が requirements.md と一致するか | 一致 | requirements.md §3 NFR1/NFR2/NFR4 の記述と枝番要求の間に矛盾なし |
| NFR3 / NFR5 の N/A 根拠 | 妥当 | U1 は永続化・投影を持たない純粋ライブラリであり NFR3 適用外、NFR5 は requirements.md の性能非目標方針と整合 |
| tech-stack-decisions.md の技術選定が ADR 0001 決定 1〜6 と整合するか | 一致 | 単一クレート・3 プロファイル・キー順規則・整数型固定・直接呼出し禁止・ゴールデン先行検証のいずれも ADR の決定と代替案却下理由が対応している |
| `preserve_order` ワークスペース全体有効化の影響評価 | 記載あり | tech-stack-decisions.md §3「未決」に既存 `serde_json::Value` 利用箇所への影響確認を code-generation の棚卸しとして明記しており、放置されていない |
| CanonJson の「依存ゼロ」表記とランタイム依存追加（sha2/serde/serde_json）の整合性 | 矛盾なし | components.md の `depends_on: []` は内部コンポーネント間依存（層構造）を指しており、外部クレート依存とは別軸。tech-stack-decisions.md の記述もこの解釈と整合している |

### Summary

技術選定・セキュリティ境界・品質ゲートの各要求は Inception の NFR1/NFR2/NFR4 および ADR 0001 の決定と整合しており、traceability.json の target もすべて本体に実在する。再帰深さ上限とレピュディエーション行の記述に軽微な精度上の抜け・矛盾があるが、いずれも Minor でありブロッキングではない。
