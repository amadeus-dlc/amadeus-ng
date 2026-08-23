# security-requirements — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Requirements（Construction 3.2）成果物（Unit: U9、kind: spec、Bolt: B4）。出典: `../functional-design/rules.md`（BR1.1〜BR5.2 — 改訂内容と
> 合格条件）、`../../../inception/requirements-analysis/requirements.md`（NFR1 upstream 互換（D6 範囲）、NFR2 品質ゲート維持、制約 C4 日本語正本）、
> `../../../inception/contract-design/contract-summary.md`（U9 は契約面を持たない — C1〜C7 に影響なし。C4 / C5 の改訂内容を仕様へ写すのは BR3.1 / BR3.3）、
> `aidlc/spaces/default/codekb/docs/technology-stack.md`（文書ツールチェーン）、確認事項 `nfr-requirements-questions.md`（前提 P1〜P3、Looks correct）。
>
> spec Unit のため「セキュリティ要求」= 正本文書の**改訂の安全性**（逐語契約を壊さない、出典の追跡可能性、自己整合）の要求であり、NFR2（品質ゲート）
> の文書版もここに置く。各要求は Inception の NFR ID を継承し枝番を付ける（NFR1.x / NFR2.x）。

## 1. 範囲と信頼境界

- 対象は**文書だけ**: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md`（4 ファイル、うち 1 新規）、`docs/specs/{01,10,11,12}-*.md` と
  `docs/specs/deviations.md`、`inception/domain-design/components.md`。コードは触らない（`modules/` / `tools/` / `scripts/` / `.github/` の diff ゼロ）。
- 信頼境界: (a) 正本の権威 = オーナー裁定（coding-rules は 1 ルール 1 ファイル・裁定日つき）、(b) 仕様の権威 = upstream の観測可能契約（D6）と ADR、
  (c) PR レビュー（アーキテクチャレビュアー + レビューボット）。改訂は (a)(b) に遡れる出典を持つものだけを通す。
- 秘密情報・個人情報を扱わない。外部ネットワーク不要（upstream ピンの参照は既存のローカル写し `tests/golden/upstream-3c3146cf/` と `docs/specs/research/`）。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | **逐語契約の不変** — 仕様改訂は「構造の規範と所有の記述」に限り、upstream 互換の逐語契約（監査イベント 86 語・CLI 語彙・`AIDLC_*`・LLM 分岐条件の文言・ファイル形式）と `docs/specs/research/*.md`（抽出文書）には触れない | `git diff -- docs/specs/research` が空。10 号 §1 の「逐語の完全列挙は抽出文書と upstream を正とする」の一文を維持。改訂行に逐語文言の変更が含まれないことをレビューで確認 | NFR1, BR5.2 |
| NFR1.2 | **逸脱の登録** — ES 化に伴う観測可能な逸脱（SQLite ファイルの追加・ロック dir 非生成・互換ファイルはリードモデル）は `docs/specs/deviations.md` の表に登録し、本文の改訂だけで済ませない | deviations.md に 1 行（# / 分類 / upstream / amadeus-ng / 理由 / 記録）が追加され、理由欄が ADR-003 / 007 を指す | NFR1, BR3.4 |
| NFR1.3 | **出典の追跡可能性** — 各改訂箇所に出典（ADR 番号 / 契約 ID / Bolt / オーナー裁定日）を括弧書きで残し、推測で仕様を変えない | 改訂行の出典注記をレビューで確認（出典の無い改訂は差し戻し） | NFR1, BR5.2, BR3.x の source |
| NFR2.1 | **コード変更ゼロ** — Bolt B4 は文書のみ。CI 4 ジョブ（check / quint / coverage / audit）と `CI Success` は変更なしで緑のまま | `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空。PR の CI 緑 | NFR2, BR5.1 (d) |
| NFR2.2 | **自己整合の機械検査** — 削除済み API 名・退役機構・旧称が規範として残らない: `effective_plan_action` / `next_in_scope_stage` / `AuditLedgerRepository` / `AuditLedgerService` / `StateFileStore` / `report_forward` / `gate_start` を `coding-rules/*.md` と `docs/specs/*.md`（`research/` を除く）で grep し、履歴注記（『旧』と明記された比較表・禁止名テーブル）以外で 0 件 | grep の結果を PR 本文（または code-summary）に貼る。`StageGraphReader` は gateway-taxonomy §2 の禁止名テーブル（意図的な記録）を除外 | NFR2, BR5.1 (c), FD 回復レビュー所見 3 |
| NFR2.3 | **索引の無矛盾** — `coding-rules/README.md` の一覧表の行数 = ルールファイル数、各行の一言・機械強制・裁定日が各ファイルと一致 | README と `ls coding-rules/*.md` の突合（レビュー） | NFR2, BR4.2 |
| NFR2.4 | **表・見出しの整形** — 改訂した Markdown 表は見出しと同じ列数（regex 内の `|` はエスケープ）、同一見出しの重複を作らない（レビューボットの markdownlint 指摘 MD056 / MD024 を予防） | CodeRabbit の該当指摘 0 件（出たら PR 内で直す — PR コメントは無視しない） | NFR2, PR #25 / #27 の教訓 |
| NFR2.5 | **日本語正本** — 人間可読の改訂は日本語、固定トークン（型名・ファイル名・API 名・ID）は英語のまま | レビューで確認 | 制約 C4 |

## 3. 脅威の検討（STRIDE、文書改訂の規模）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing / Elevation of Privilege | 該当なし（文書だけ、権限変更なし） | — |
| Tampering | 改訂のついでに逐語契約（D6）を書き換えてしまう / 出典の無い「勝手な設計変更」を仕様に混ぜる | NFR1.1（research/ 不変）、NFR1.3（出典注記）、レビュー |
| Repudiation | 誰の裁定で変えたかが追えない | NFR1.3 + PR の記録（Bolt B4）、coding-rules の裁定日 |
| Information Disclosure | 該当なし（秘密情報を含む文書ではない） | — |
| Denial of Service | 該当なし（CI への影響なし — NFR2.1） | — |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| coding-rules / docs/specs / components.md | Public（公開リポジトリ） | 秘密情報なし |

## 5. 適用外

- NFR3（監査完全性）・NFR4（サプライチェーン）・NFR5（性能）: 文書だけの Unit で固有の要求を持たない（依存・コードの変更なし）。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T05:18:10Z
**Iteration:** 1（advisory, unit: u9-canon-docs）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | security-requirements.md NFR2.1 / tech-stack-decisions.md §1「差分の受入」 vs `../functional-design/rules.md` BR5.1(d) vs `nfr-requirements-questions.md` P2 | 「コード変更ゼロ」の diff スコープが本 Unit の成果物間で 3 段階に食い違う。(a) rules.md の BR5.1(d)（本表が出典として名指す規則そのもの）は `git diff --stat -- modules tools` のみ、(b) 質問票 P2（Looks correct 済みの前提）は `modules tools scripts .github`、(c) 本表 NFR2.1 と tech-stack-decisions.md §1 は `modules tools scripts .github Cargo.toml Cargo.lock`（かつ `origin/main..HEAD` 基点を追加）。NFR2.1 は出典欄で「BR5.1 (d)」を明示的に引用しているが、実際には BR5.1(d) より広いスコープへ無断で拡張しており、拡張の理由も rules.md 側への反映もない。BR5.1 は `category: validation`（PR 受入チェックそのもの）と明記された規則であり、この不一致は「PR がどの合格基準で判定されるか」という実行時に効く受入ゲートの定義に直接影響する。文書だけの Bolt が誤って `Cargo.toml` を触った場合、rules.md BR5.1(d) の文言だけを見た開発者・レビュアーは合格と判断しうるが、NFR2.1 の基準では不合格になる ― 逆に本 NFR2.1 の方が安全側だが、正本間の食い違いとして残る。 | (a) rules.md BR5.1(d) を次回 functional-design 改訂機会（pending-revision.md 適用時）に `modules tools scripts .github Cargo.toml Cargo.lock` へ広げて同期するか、(b) NFR2.1 の出典欄に「BR5.1(d) を Cargo.* 追加で強化（理由: 依存操作の見落とし防止）」と一行の根拠注記を足し、rules.md との差分を意図的なものとして明示する。いずれかで正本間の不一致を解消・追跡可能にする。 |
| 2 | Minor | security-requirements.md NFR2.2 vs `../functional-design/rules.md` BR5.1(c) vs `../functional-design/pending-revision.md`（回復レビュー所見 3） | NFR2.2 の sentinel 一覧は `StageGraphReader` を除外し「gateway-taxonomy §2 の禁止名テーブル（意図的な記録）」を理由に挙げているが、これは rules.md BR5.1(c) の**現行の逐語**（`StageGraphReader` を含む 8 sentinel を「履歴注記を除き 0 件」と定義）とは一致せず、`pending-revision.md`（回復レビュー iteration 2 の Major 所見 3、まだ rules.md 本文へは未適用）の裁定に従っている。出典欄は「FD 回復レビュー所見 3」とだけ書いており、`pending-revision.md` というファイル名を明示していないため、rules.md BR5.1 だけを読む開発者には根拠が追えない。実質的な判断（pending-revision の方を正とする）自体は合理的で、`nfr-requirements-questions.md` の確認前提とも矛盾しない。 | NFR2.2 の出典欄を「BR5.1(c)、`../functional-design/pending-revision.md` 所見 3（rules.md 本文は未適用）」のように明示し、rules.md 本体を読む次の実装者が迷わないようにする。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts --stage nfr-requirements` | `{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[],"findings_count":0}` | traceability.json は Inception の NFR1〜NFR5 を過不足なく列挙し、OK/N/A の対象 ID も整合。機械検証 green。 |
| `aidlc-sensor-required-sections.ts` (security-requirements.md) | `{"pass":true,"h2_count":5,...}` | H2 見出し 5 本、閾値（≥2）を満たす。 |
| `aidlc-sensor-required-sections.ts` (tech-stack-decisions.md) | `{"pass":true,"h2_count":3,...}` | H2 見出し 3 本、閾値を満たす。 |
| `aidlc-sensor-upstream-coverage.ts` (security-requirements.md) | `{"pass":true,"reason":"no upstream",...}` | この呼び出し形では consumes 解決ができず判定をスキップしている（`--stage` のみでは per-unit の consumes を解決しない模様）。プロセス上の制約であり本成果物の欠陥ではない。 |

### Summary

U9（spec Unit、Bolt B4）の NFR 要求は、上流の functional-design（rules.md の BR1.x〜BR5.x）・requirements.md の NFR1/NFR2/制約C4・contract-summary.md（U9 は契約面を持たないという主張は C1〜C7 の実地確認と一致）と概ね正しく遡れ、NFR3/NFR4/NFR5 の適用外判定も unit-of-work.md 上の他 Unit（U3/U4/U10）への委譲として妥当。traceability.json と required-sections はいずれも機械検証 green。唯一の実質的な懸念は「コード変更ゼロ」の diff スコープが rules.md・質問票・本成果物の 3 か所で無断に食い違っている点（Major #1）で、PR の実際の合否判定に影響しうるため人間の裁定を推奨する。StageGraphReader の扱い（Minor #2）は判断自体は妥当だが出典の明示が不足している。Critical 0 件・Major 1 件・Minor 1 件のため advisory 目安（Critical 0 かつ Major ≤2）を満たし、READY と判定する。
