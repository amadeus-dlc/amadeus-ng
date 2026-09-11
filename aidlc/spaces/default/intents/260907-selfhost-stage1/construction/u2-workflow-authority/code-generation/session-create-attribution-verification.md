# 作成した作業のセッション帰属

2026-09-09。`remaining-step7-inventory.md`の棚卸し後に親から依頼された最小slice。Testing Contract `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55` を維持し、計画・承認記録を変更していない。

## 問題と修正

以前のnative intent-createは、作成した作業のbindingだけを更新し、セッションのUUID stampを更新していなかった。cold SessionStart→intent-create→SessionEndを一時ワークスペースで実行すると、各コマンドは終了0でもSESSION_ENDEDが0件だった。SessionEndが未stampのUUID作業への付替えを拒む既存保護は正しく、作成側に必要な帰属の保存が欠けていた。

作成成功後に、CLIを起動したセッションへbindingとstampを保存する。既存stampが別UUIDなら、そのUUIDから新UUIDへのhandoffを先に保存する。初期化結果のQueryに成功した後へ結線し、失敗した作成で帰属を書き換えない。

本家固定元は `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `dist/claude/.claude/hooks/aidlc-rebuild-stage-graph.ts:56–105` と `tools/aidlc-lib.ts:3699–3763`。本家の成功作成境界と同じbinding→handoff→stamp順、3フィールドのhandoff JSONと末尾LF、best-effortな機械ローカル保存を保持する。状態・監査の正本へ直接書き込む経路は追加しない。

`.current-session`を作成主体の根拠に使わない。既存の `SessionNavigation::current_id` が解決したセッションだけが対象であり、セッション不在/不正時に最後に開始した別会話へ付け替えない。

## TDDと検証

| 対象 | Red | Green |
| --- | --- | --- |
| cold作成後の終了帰属 | `session-create-stamp-red.log`: 終了監査は期待1件に対して0件 | `session-create-stamp-green.log`: SESSION_ENDEDが作成した作業に1件、stampは実際のintent UUID |
| 別作業への切替印 | `session-create-handoff-red.log`: handoffファイルが存在しない | `session-create-handoff-green.log`: 元/先UUIDと発行時刻が一致、既存のhandoff読取りも成功 |

単位コマンドは `cargo test -p aidlc --test session_hooks_contract <各テスト名>`。一時的に並行編集のReported引数やprojectionのbaselineでビルドが止まった実行は、製品のRedに数えていない。

`cargo test -p aidlc --test session_hooks_contract --test session_compaction_contract` は20件成功・終了0。`test-evidence/session-create-regression.log`に全文を保存した。追加でhandoffのJSON全文と末尾LFを照合し、`session-create-handoff-bytes.log`に1件成功・終了0を保存した。

拒否したscope指定は既存binding/stampを変えずhandoffを作らない。別の会話を後からSessionStartして`.current-session`を変えても、作成セッションを取り違えない。session不在と不正sessionで、既存会話のbinding/stampを奪わない。これらを一時fixtureと公開CLIで確認した。

`cargo lint`と担当ファイルの`rustfmt --check`は成功。Clippyは並行担当の`runtime/jump.rs:15`の添字アクセス3件で停止したため、全体成功とは扱わず親へ共有した。

## 変更範囲と残り

- `modules/app/aidlc/src/session_navigation.rs`: handoffの機械ローカルな保存。
- `modules/app/aidlc/src/runtime.rs`: intent-create成功後の既存結線だけ。
- `modules/app/aidlc/tests/session_hooks_contract.rs`: 作成と終了・拒否・別会話の公開境界。

SessionEndや他のフック本体はこのsliceで変更していない。`rebuild-stage-graph`のPostToolUse封筒そのものとruntime compileは未接続である。この修正は、既存CLIが起動セッションを解決できた場合の作成帰属を完成させたものであり、PID観測ができない環境でのPostToolUse経由の補完まで検証したとは扱わない。配布接続・実地Claudeスモーク・ホスト切替は実行していない。
