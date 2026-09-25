# 戻しの手順 — Issue #134 の修正

## 前提と上流の成果物

bugfix scope のため、上流の `ci-config`・`quality-gates`・`infrastructure-specification`・`cicd-pipeline` は存在しない（scope の設計どおり）。戻しの経路は `cd-config.md` の既存の経路（PR → CI → merge queue → main）をそのまま使う。

## いつ戻すか

- main に入った後で、`replace_pipeline` を IMMEDIATE にしたことが原因とみられる不具合が出たとき。
  - 例: 投影が書込ロックを 5 秒待ってからタイムアウトし、以前より遅くなる。
  - 例: 別の書き込みとの間でロック待ちが連鎖する。
- 判断するのはオーナー。

## 戻し方

データの移行やスキーマの変更は無い。戻しは、コードを 1 コミット戻すだけで済む。

1. main の該当コミット（squash でできた 1 コミット）を特定する。
   ```bash
   git log --oneline --grep "#134" origin/main
   ```
2. 作業ブランチを切り、revert する。
   ```bash
   git switch -c revert/134-replace-pipeline-immediate origin/main
   git revert <コミット>
   ```
3. 行きと同じ経路（PR → CI → merge queue）で main に戻す。戻しを merge queue へ入れてよいかはオーナーが決める（今回の事前承認は修正の PR #154 に限ったもの）。
4. 戻した後の確認:
   - main の先端で `cargo test --workspace` が緑であること。再現テスト `replace_pipeline_waits_for_a_write_lock_held_by_another_connection` もコミットごと消えるので、そのテストの結果は見ない。
   - CI の `check` と `coverage` が緑であること。

## 戻したときの影響

- #134 の症状（並行した link 報告で、記録に成功した側が `WouldBlock` で失敗を返す）が再び起きうる状態に戻る。
- 記録済みの受領はジャーナルに残る。次の catch_up で公開される既存の振る舞いがあるので、データは失われない（`a_publication_failure_preserves_the_receipt_for_recovery` が固定している）。

## 連絡と見直し

- 戻したら、Issue #134 を開き直し、戻した理由と観測をコメントに書く。
- 戻しの原因を調べてから、もう一度直す。そのときは再試行の仕組み（要件の Q1 で B・C として見送った案）を検討に戻す。
