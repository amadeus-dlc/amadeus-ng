# U2 NFR 設計改訂の検証記録（2026-09-07 再走）

2026-09-07（UTC 2026-09-06 深夜）、利用者の要約確認（P5〜P11、Looks correct）後に security-design.md・logical-components.md・
traceability.json を改訂した。旧 READY レビュー節（2026-08-23）と pending-revision.md は `security-design-review-history-2026-08-23.md`
へ逐語退避し、pending-revision.md は `git rm` した（P11）。

## 実行結果

| 検査 | 結果 | 確認範囲 |
|---|---|---|
| `aidlc-sensor-required-sections.ts --stage nfr-design`（security-design.md） | pass、H2 8 本、所見 0 | 必須見出し |
| `aidlc-sensor-required-sections.ts --stage nfr-design`（logical-components.md） | pass、H2 5 本、所見 0 | 必須見出し |
| `aidlc-sensor-traceability.ts --stage nfr-design` | pass、gaps / orphans / missing / invalid すべて空 | NFR1.1〜NFR4.5 + NFR2.5 の 17 ID |
| `aidlc-sensor-upstream-coverage.ts --stage nfr-design`（security-design.md） | pass（reason: no upstream） | — |
| `git diff --check`（nfr-design 配下） | PASS | 空白エラー |
| `orchestration/` のエントリ数（`ls | wc -l`） | 53（51 ファイル + `intent_event/` / `intent_execution_event/` の 2 ディレクトリ） | logical-components §1。質問票 P7 の「55 ファイル」は概数の誤りで、成果物は実測値で記載した（質問票は確認済みバイトのため訂正せず、ここに記録する） |
| `workspace/` → `orchestration/` の参照（`grep 'use crate::orchestration' src/workspace/`） | 0 件 | logical-components §2 の一方向依存 |
| `IntentExecution` の状態列（`intent_execution.rs` struct 実測） | `stage_keys` / `overlay` / `checkbox` / `review_attempts` / `practices_affirmed` / `approved` / `revision_count` の 7 列 | StageSlots への統合対象 |
| 生の `Vec` / `&[..]` 公開（`src/orchestration` の `pub fn` 実測） | `intent.rs:260`、`intent_execution.rs:441`、`review_attempt.rs:66`、`stage_entry.rs:100`、`created.rs:87`、`started.rs:68`、`gate_opened.rs:40`、`recomposed.rs:37,43`、`practices_affirmed.rs:61,67,73` | logical-components §1 の「本再走の変更」列 |
| 兄弟クレートの呼出（`grep` 実測） | interface-adapter DTO 4 ファイル 5 箇所、RMU 4 ファイル 5 箇所、use-case 2 ファイル 5 箇所、ITF テスト 3 箇所 | logical-components §1 末尾の追随表 |
| `GraphReadError` / 定義用 `find_by_id` の存在（`grep` 実測） | `GraphReadError` 0 件（`find_by_id` は実行集約の Repository メソッドのみ） | 旧 pending-revision 項目 1 / 5 の失効根拠（P11） |
| エラー変種数（`command_error.rs` / `report_refusal.rs` 実測） | `CommandError` 17 変種、`ReportRefusal` 13 変種 | security-design §2 |

## 検証の限界

CI 全体の実行、`cargo audit`、Quint ゲート、ITF 準拠テスト、カバレッジ計測の再実行は本改訂では行っていない（nfr-requirements の
2026-09-06 実測を引用）。FCC 化・`next_decision` の Result 化・`slots` 統合は設計であり、現行コードには未反映（functional-spec §9 の
引継ぎ）。`TransitionSteps` と `ReviewAttempt` 内部列の不変条件（機能設計レビュー R-01）、`filter` / `divide` の結果型（R-04）は
凍結中の機能設計本文の未決で、functional-design ゲートの Request Changes で確定する前提を security-design §6 / logical-components §1 に
明記した。
