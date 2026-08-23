# nfr-requirements-questions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Requirements（Construction 3.2）の質問票（Unit: U9、kind: spec、Bolt: B4）。出典: `../functional-design/rules.md`（BR1.1〜BR5.2）、
> `../../../inception/requirements-analysis/requirements.md`（NFR1〜NFR5、制約 C4）、`../../../inception/contract-design/contract-summary.md`
> （U9 は契約面を持たない — C1〜C7 に影響なし）、`aidlc/spaces/default/codekb/docs/technology-stack.md`（文書ツールチェーン: Markdown、
> markdownlint は CI 外）。
>
> **質問なし。** U9 は文書だけの Unit で、適用される NFR は NFR1（upstream 互換 — 仕様改訂で逐語契約を変えない）と NFR2（品質ゲート — コード変更
> ゼロで CI を緑のまま維持、文書の自己整合を grep で検査）だけ。NFR3（監査完全性）・NFR4（サプライチェーン）・NFR5（性能）は文書に固有の要求を持たない。
> 次の前提を確認して成果物へ進む。

## 前提（確認事項）

- P1. NFR1: 仕様の改訂は「構造の規範と所有の記述」に限り、upstream 互換の逐語契約（D6 — 監査イベント 86 語、CLI 語彙、`AIDLC_*`、LLM 分岐条件の文言、
  ファイル形式）には触れない。逸脱は `docs/specs/deviations.md` への登録（BR3.4）だけで表す。
- P2. NFR2: コード変更ゼロ（`git diff --stat -- modules tools scripts .github` が空）で CI 3 ジョブ + audit は緑のまま。文書の品質ゲートは (a) レビュー
  （アーキテクチャレビュアー + PR のレビューボット）、(b) 自己整合の grep（BR5.1 — 削除済み API 名・退役機構・旧称が規範として残らない）、(c) README と
  ルールファイルの無矛盾（BR4.2）。markdownlint は CI に無い（CodeRabbit が補助的に指摘 — 表の列数・見出し重複は直す）。
- P3. 技術選定: Markdown（日本語正本、固定トークンは英語 — 制約 C4）、改訂箇所に出典注記（ADR / 契約 / Bolt / オーナー裁定）、grep による受入。
  新規ツール・依存なし。

## Consolidated Summary Confirmation

- U9 に固有の NFR 質問はなし。適用 NFR は NFR1（逐語契約を変えない — 構造の規範だけ改訂、逸脱は deviations へ）と NFR2（コード変更ゼロ・CI 緑維持・
  自己整合 grep・README 無矛盾）。NFR3 / NFR4 / NFR5 は適用外
- 技術選定（P3）: Markdown のみ、出典注記、grep 受入、新規ツールなし

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
