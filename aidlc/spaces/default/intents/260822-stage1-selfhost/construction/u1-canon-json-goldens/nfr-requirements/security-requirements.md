# security-requirements — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> NFR Requirements（Construction 3.2）成果物（Unit: U1、kind: library）。出典: `../functional-design/functional-spec.md`
> （W1〜W5）、`../functional-design/rules.md`（BR1.x / BR2.x）、`../../../inception/requirements-analysis/requirements.md`
> （NFR1 upstream 互換・NFR2 品質ゲート・NFR4 セキュリティ/サプライチェーン）、`../../../inception/contract-design/contract-summary.md`
> （C1 / C7）、`aidlc/spaces/default/codekb/docs/technology-stack.md`（既存依存・unsafe forbid の現状）、
> 確認事項 `nfr-requirements-questions.md`（前提 P1〜P4、Looks correct）。
>
> 各要求は Inception の NFR ID を継承し枝番を付ける（NFR1.x / NFR2.x / NFR4.x）。

## 1. 範囲と信頼境界

- U1 は依存ゼロの純粋ライブラリ（`canon-json`）と、リポジトリ内の静的データ（ゴールデンコーパス）。
  ネットワーク・認証・認可・永続化を持たない。
- 読む入力は (a) ワークスペース内の契約 JSON（stage-graph / scope-grid / scopes / directive 等 — 信頼境界の
  内側だが、外部ファイルとして境界検証を行う）、(b) ゴールデン（リポジトリ管理下）。
- 秘密情報・個人情報（PII）は扱わない。ゴールデンは使い捨てワークスペースで採取し、環境固有値は
  プレースホルダに正規化する（BR2.2）ため、採取者のホスト名・パス・ID を含まない。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | 3 プロファイル（contract-pretty / contract-compact / hash-canonical）の直列化出力が upstream とバイト一致する | hash-canonical 受入表の全行一致（出力文字列 + ダイジェスト）、CLI/フックゴールデンとの正規化後バイト一致（後続 Bolt で検証） | NFR1, FR7.3, BR1.1〜BR1.6 |
| NFR1.2 | ダイジェストの 2 族（`sha256:` 接頭辞の正準族 / 生 hex の非正準族）が用途ごとに固定され、取り違えがない | 用途 → 族の対応表（functional-spec W2 の補遺として code-generation で固定）に対するテスト | NFR1, BR1.6, C1 |
| NFR1.3 | ゴールデン比較は決定的で、正規化規則の適用前後で差分を隠さない | 正規化は <TS>/<CLONE>/<ROOT>/<SESSION> の 4 種のみ。規則はコーパスに固定され、再採取で同一結果 | NFR1, BR2.2 |
| NFR2.1 | ゴールデン先行の TDD — 受入表・ゴールデンを red として先に置き、実装で green にする | 受入表テストが実装前に存在し失敗することを PR 履歴で確認できる | NFR2, team.md Testing Posture |
| NFR2.2 | カバレッジ 90% 床を維持（canon-json は 100% 近傍を目標） | `scripts/coverage.sh` green | NFR2 |
| NFR2.3 | PBT で決定性を検証 — 同一入力 → 同一出力、parse → serialize の往復、hash の冪等性 | proptest ケースが CI で green | NFR2, BR1.1〜BR1.6 |
| NFR4.1 | ランタイム依存の追加は `sha2`・`serde`・`serde_json` のみ。`cargo audit` clean | `Cargo.toml` の差分レビュー、CI の cargo audit（U10 で追加）green | NFR4, 前提 P1 |
| NFR4.2 | `unsafe_code` forbid（ワークスペース lint で強制 — U10） | clippy / rustc で violation ゼロ | NFR4 |
| NFR4.3 | 入力は境界で検証 — 不正 JSON は `ParseError`、再帰深さ上限（serde_json 既定 128）を維持しスタック枯渇を防ぐ、非有限数・制御文字は規則で決定的に処理 | 不正 JSON・深いネスト・非有限数・制御文字のテストが期待どおり振る舞う | NFR4, BR1.3, BR1.4 |
| NFR4.4 | ゴールデンに秘密情報・PII・環境固有値を含めない | 採取は使い捨てワークスペース、正規化規則の適用、レビューでの目視確認 | NFR4, BR2.1, BR2.2 |

## 3. 脅威の検討（STRIDE、ライブラリ規模）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing / Elevation of Privilege | 該当なし（認証・認可を持たない） | — |
| Tampering | ゴールデンの改竄（実装側に合わせて正解を書き換える誘惑） | ゴールデンは git 管理 + PR レビュー。BR2.3 / BR2.5（実装を直す、ゴールデンは直さない。更新はピン更新 intent のみ） |
| Repudiation | 該当なし | 来歴（commit / captured_at / command）で採取の出所を追える（BR2.1） |
| Information Disclosure | 採取環境の絶対パス・ホスト名・ID がゴールデンに残る | BR2.2 の正規化 + NFR4.4 のレビュー |
| Denial of Service | 深いネスト・巨大入力によるスタック枯渇・メモリ | 再帰深さ上限 128（NFR4.3）。入力はワークスペースの契約ファイルで KiB 規模 |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| 契約 JSON（stage-graph 等） | Internal（upstream dist 由来の公開資産） | 検証して読む。書込は canon-json 経由のみ |
| ゴールデンコーパス | Internal（リポジトリ内） | 正規化済み。秘密情報・PII なし |
| ダイジェスト | Internal | 秘密ではない（内容のハッシュ）。鍵管理なし |

## 5. 適用外

- NFR3（監査完全性）: U1 は永続化・投影を持たない — 適用外。
- NFR5（性能）: 数値目標なし（非目標の明示）。

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
