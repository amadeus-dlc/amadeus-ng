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

## 再走時の前提（2026-09-07 — functional-design 差し戻し後の U9 再走、Modify）

新規の質問は無い。適用 NFR の集合（NFR1 / NFR2 が適用、NFR3 / NFR4 / NFR5 は適用外）は変わらない。再走で変わるのは要求の**中身**であり、その根拠は
すべて実測（`../functional-design/gap-measurement-20260907.md` §1〜§5、現行コード `main` `02cacea2`、`.github/workflows/ci.yml`、`scripts/coverage.sh`）と
再走版 `../functional-design/rules.md`（BR1.1〜BR5.3、status 付き）にある。次の前提を確認して成果物へ進む。

- P4. 範囲の差し替え: 改訂対象は coding-rules 6 ファイル 12 行（BR1.6 / BR4.2）、仕様 4 号（01 / 10 / 11 / 12 — BR3.2 / BR3.3。deviations は触らない）、
  共有契約 3 本（components.md 全面 / contract-summary.md 節単位 / unit-of-work.md 注記のみ — BR3.7）。コードは触らない。
- P5. 合格基準の同期: NFR2.1 の CI は `ci.yml` 実測の 7 ジョブ（aidlc-distribution / check / quint / coverage / audit / review-thread-resolution / ci-success）、
  diff 範囲は `origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock`。NFR2.2 の sentinel は BR5.1 (c) の 10 語（`WorkflowExecution` /
  `RehydratedWorkflowExecution` / `message-catalog` を追加、`StageGraphReader` は外す）と履歴マーカー除外の判定（同一行に `~~` または 旧 / 失効 / 是正済み /
  改名 / 履歴）。NFR1.2（逸脱登録）は実測 1 行で達成済み。pending-revision 2 項目（出典欄）は BR5.1 が同期済みのため本版で閉じる。
- P6. 新設: NFR1.4「実装状態の正確な表示」（未実装 — unpark / jump / recompose、フック 4 本、doctor、workspace 集約 3・供給面 4、`intents.json` 直列化、
  Bolt / SwarmBatch — は仕様に『予定（未実装）』と明記し、実装済みのように書かない。記録間の矛盾はコードの現状で裁く）、NFR1.5「共有契約の履歴保全」
  （inception 成果物の既存 `## Review` 節・打消し線つき履歴は削除せず、失効は打消し線 + 日付で追記）、NFR2.6「検証規律」（BR5.3 — 仕様・規則・共有契約に
  書く主張は実装コード・テスト・仕様の三つで検証してから採用し、根拠に path:line または型 / 関数名を添える）。
- P7. 技術選定は Markdown / 日本語正本 / 出典注記 / grep 受入のまま。実測の手段として `grep` / `awk` による型・関数・変種の列挙（gap-measurement §1 の方法）を
  追記する。新規ツール・依存なし。

## Consolidated Summary Confirmation

- U9 に固有の NFR 質問はなし。適用 NFR は NFR1（逐語契約を変えない — 構造の規範だけ改訂、逸脱は deviations へ）と NFR2（コード変更ゼロ・CI 緑維持・
  自己整合 grep・README 無矛盾）。NFR3 / NFR4 / NFR5 は適用外
- 技術選定（P3）: Markdown のみ、出典注記、grep 受入、新規ツールなし

**再走時の確認範囲（2026-09-07、Q4 = A の再走、Modify）**

- 2026-08-23 の要約（上）は当時の確認として保持し、やり直さない。今回は P4〜P7 の再走範囲だけを確認する。
- 成果物 3 点を Modify: `security-requirements.md`（§1 範囲を P4 へ、NFR1.1〜NFR2.5 を P5 で同期、NFR1.4 / NFR1.5 / NFR2.6 を新設、§3 STRIDE の Tampering に
  「記録の転記による誤った規範の混入」を追加、旧 `## Review` は履歴節へ）、`tech-stack-decisions.md`（§1 に実測手段、§2 依存の差分を再走の対象一覧へ、§3 未決なし）、
  `traceability.json`（NFR1 の target に NFR1.4 / NFR1.5、NFR2 に NFR2.6 を追加。NFR3 / NFR4 / NFR5 の N/A 理由は維持）。
- 適用外の判定（NFR3 監査完全性 / NFR4 サプライチェーン / NFR5 性能）は変えない — 文書のみ・依存変更なし・性能は非目標（requirements.md）。
- 次ステージは U9 の NFR Design。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
