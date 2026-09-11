# 番号回答の承認連携: 是正案

## 確認済みの事実

- 人間の「1」を `Approve Plan` として回答ファイルへ書き、正規のanswerコマンドへ渡しても受領が拒否された。
- `.codex/tools/aidlc-testing-posture.ts:1420–1422` は、完全一致が必須の形式以外では「1」を `Approve Plan`、「2」を `Request Changes` へ変換する。番号対応は既にある。
- `.codex/hooks/aidlc-record-human-turn.ts:62–79` は文字列をJSONとして再解析し、数値に対して空文字列を返す。`"1" → 1 → ""` により、承認照合へ値が届かない。
- 別セッションや古い応答等を拒否する `recordPlanApprovalReceipt` の照合は変更不要。

## 修正する内容

添付の `numeric-human-response.patch` を正式な同期パッチ `scripts/aidlc-sync/patches/numeric-human-response.patch` として登録し、通常の `bun scripts/aidlc-sync.ts --apply` で3配布先へ反映する案である。数値として解析された回答文字列を保持するだけで、承認の照合条件や受領発行を緩和しない。

`scripts/aidlc-plan-progress.test.ts` の `approvedProject` に既定値 `Approve Plan` のprompt引数を追加し、人間入力フックへ渡す。「1」を渡したケースで、正規のanswer記録と、その後の生成許可まで検証する。対象は3配布先。先にこの回帰を実行してRedを確認する。

Green後は「2」が修正要求になること、未知番号・別セッション・古い質問・完全一致必須形式で誤承認しないことも同じ境界で確認する。数字を単に承認済みに書き換える処理は追加しない。

## 是正前の制限

回帰テストの編集を試みたが、PreToolUseの計画承認保護が拒否した。したがってソース修正・テスト追加・Red/Green検証・同期適用はまだ行っていない。添付パッチはレビュー用の案で、適用済みではない。

この作業ツリー自身の承認受領を手書き・模擬フック入力で生成しない。人間の応答が正規のフックから受領される状態で、TDDによる是正を続行する。

## 是正結果（2026-09-08）

人間の実際の回答 `Approve Plan` を受け、正規のanswerコマンドで `PLAN_APPROVAL_RECORDED`、beginコマンドで `status: generation` を確認してから修正した。

- **Red**: `bun test ./scripts/aidlc-plan-progress.test.ts --test-name-pattern '番号1'` は3配布先すべてで失敗した。実セッションと同じ `Plan Approval requires the actual offered choice from this prompt and session` を再現した。
- **Green**: `scripts/aidlc-sync/patches/numeric-human-response.patch` を登録し、`bun scripts/aidlc-sync.ts --apply` で3つの配布hookを更新した。同じ3件はすべて成功した。
- **回帰**: `bun test ./scripts/aidlc-plan-progress.test.ts` は36件成功、失敗0件、438 assertions。番号2は修正要求として処理され、未知番号・別表記の数値・別セッション・再提示前の回答は生成を許可しない。JSON回答と引用文字列の番号も保持する。
- **同期**: `bun scripts/aidlc-sync.ts --check` は差分0、同期済み。vendorは変更していない。
- **関連検査一式**: `bun test ./scripts/aidlc-sync.test.ts ./scripts/aidlc-harness.test.ts ./scripts/aidlc-unit-lifecycle.test.ts ./scripts/aidlc-plan-progress.test.ts ./scripts/aidlc-traceability.test.ts ./.codex/hooks/aidlc-codex-adapter.test.ts` は74件成功、失敗0件、558 assertions、終了0。ログは `/tmp/amadeus-stage1-bun-regression.log`。
- **変更前のRust基準**: `cargo test --workspace` は2,354件成功、失敗0件、終了0。ログは `/tmp/amadeus-stage1-cargo-baseline.log`。

配布元の番号→意味への変換と、セッション・対象・質問の対応検査は変更していない。修正したのは、人間回答の抽出時に数値文字列を失っていた処理である。ここで示す番号回答の成功は使い捨てプロジェクトの回帰検証であり、本リポジトリの承認イベントを模擬生成した結果ではない。

## 実セッションでの番号承認

第1回レビュー後、追記された計画に対してユーザーが実際に「1」と回答した。親が正規のanswerを実行して `PLAN_APPROVAL_RECORDED`、続くbeginで `status: generation` を確認し、R-01〜R-03の修正を再開した。番号は配布元の意味変換によって `Approve Plan` と照合されており、承認イベントや保護された受領を手書き・模擬入力で生成していない。
