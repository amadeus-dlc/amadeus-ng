# superseding-decisions — U10 CI ガバナンス（`u10-ci-governance`）

> 承認済み計画（`code-generation-plan.md`、指紋対象）・単体テスト手順・終末レビュー受領後の `code-summary.md` /
> `traceability.json` は凍結されているため、その後に確定した裁定と実態をここに記録する（PR #25 の CodeRabbit レビュー指摘
> 3837291748 / 3837291749 / 3837291758 / 3837291760 / 3837291762 / 3837291763 / 3837291764 / 3837291768 の引き取り）。
> 本ファイルが凍結文書の当該箇所を**上書きする**。時刻はすべて UTC。

| # | 凍結文書の記述 | 現行の正 | 根拠 |
|---|---|---|---|
| 1 | `code-generation-plan.md` §2 / `unit-test-instructions.md` §3: `TOLERANCE=0.01`、受入「2 回計測で差 0.00pp」 | **`TOLERANCE=0.05`（暫定）**。PBT シード固定後も `fs_workspace_lock.rs:237` の並行テスト由来で ±1 行（0.0175pp）の揺れが残るため差 0.00pp は未達。U3 のロック退役（ADR-007）後に 0.01 へ引き締める | Bolt B2 ゲートのオーナー裁定（2026-08-22T23:34Z、`construction/code-generation/memory.md`）、`scripts/coverage.sh` と `verify-ci-governance.sh` の実装 |
| 2 | 計画 §2 / `security-design.md` §4 / `tech-stack-decisions.md` §1: 除外 regex `^modules/app/aidlc/src/main\.rs$` | **`(^\\|/)modules/app/aidlc/src/main\.rs$`**。llvm-cov はカバレッジデータに絶対パスを記録するため `^` 単独アンカーは不活性（実測: `main.rs` が 0/2 行で計測対象に残っていた） | `scripts/coverage.sh` の `IGNORE_FILENAME_REGEX`、code-summary §5 |
| 3 | `code-summary.md` §1 Step 10 / 11「未実施」、`traceability.json` FR9.1 / NFR2.1 / NFR4.5 の `OK`（PR 時点では未適用） | **適用済み**: ruleset「main」へ `required_status_checks`（check / quint / coverage、strict）を 2026-08-22T23:43Z に適用（前後 JSON: `ruleset/before.json` / `after.json`）。PR #25 が merge queue を `merge_group` CI 緑で完走し squash-merge（23:44:17Z）— NFR2.1 の正常系受入を満たす | `ruleset/after.json`、`gh run list --event merge_group` |
| 4 | `security-design.md` §3 / `nfr-design-questions.md` P2: 冪等判定「規則が存在すれば何もしない」 | **required コンテキスト集合 + strict フラグの一致で判定**（nfr-design レビュー Minor 2 の引き取り） | `scripts/governance/ruleset-required-checks.sh` |
| 5 | `tech-stack-decisions.md` §3「PBT シード固定の手段は未決」 | **環境変数 `PROPTEST_RNG_SEED`（proptest 1.11 の `RngSeed::Fixed`）を採用** — テストコード変更なし | `scripts/coverage.sh`、`ci.yml` の env |
| 6 | `functional-design-questions.md` P1 が「coding-rules のエラーハンドリング規則」を U10 の設計対象に含めている | **誤り — FR9.6 は U9（canon-docs）の責務**。U10 の成果物・実装には含めない | 計画 §1「作らないもの」、story map |
| 7 | `unit-test-instructions.md` §2 `bash -n a b c`（1 行で 3 本） | `bash -n` は最初のファイルしか解析しないため、**ファイルごとに個別実行**する（実測: 6 本すべて個別に OK） | PR #25 レビュー 3837291760 |
| 8 | 各記録の「2026-08-23」表記 | ローカル時刻（JST）に引きずられた表記。**UTC では 2026-08-22 17:08Z〜23:50Z** の出来事（日誌のタイムスタンプは修正済み） | 監査台帳の実 UTC 時刻 |

凍結文書の本文自体の修正は、ステージゲート（Construction の per-unit ブロック末尾）の前に 1 回の回復レビューで
まとめて行う（レビュー受領証を無効化しないため）。

## 追記（2026-08-23T00:13Z 時点 — オーナー指示、ecb2307 で実装。当初 00:40Z と誤記していたのを訂正）

| # | 内容 | 根拠 |
|---|---|---|
| 9 | **レビュースレッドのゲート**: `ci.yml` に `review-thread-resolution` ジョブ（j5ik2o/ci の再利用ワークフロー `review-thread-resolution.yml`、SHA 固定 `9cf0e9a8…`、`pull_request` のみ）と `ci-success` 集約ジョブ（check / quint / coverage + `pull_request` では review gate 必須、`merge_group` では skipped 許容）を追加。`.github/workflows/review-thread-resolution.yml` がレビューイベント・15 分ごとに状態を再評価。ruleset の required checks を **check / quint / coverage / CI Success** の 4 つに拡張（`ruleset-required-checks.sh` の `REQUIRED_CONTEXTS`、`verify-ci-governance.sh` の期待値も同期）。未解決のレビューコメント（ボット含む）を残した PR は merge queue に入れない | オーナー指示（amadeus-dlc/amadeus の ci.yml に倣う）。PR #25 のコメントが見過ごされかけた教訓 |
| 10 | 本文修正: 凍結文書（code-summary / traceability / 計画・手順のバナー / security-design / tech-stack / 質問票 3 本）に #1〜#8 の内容を反映（PR #26）。各成果物の回復レビュー（nfr-requirements / nfr-design / code-generation の U10 分）はステージゲート前に実施 | オーナー指示「PR コメントを無視しない」 |
| 11 | `nfr-requirements/security-requirements.md` NFR2.4（`TOLERANCE=0.01` / 差 0.00pp）・NFR2.1（3 コンテキスト）・NFR4.4（ジョブ個別の昇格なし）・§1 / §3（外部再利用ワークフローの信頼境界なし） — **実態と不一致（回復レビュー 2026-08-23T00:36Z の Major 1・2）** — 実装は `TOLERANCE=0.05`（暫定、#1）、required checks は 4 コンテキスト（check / quint / coverage / CI Success、#9）、`review-thread-resolution` ジョブは `checks: write` / `statuses: write` / `issues: read` / `pull-requests: read` を個別付与、呼出先は SHA 固定の `j5ik2o/ci` 再利用ワークフロー。回復レビューの受領は終端のため本文は未修正 — nfr-requirements ステージゲートで所見を提示し、Request Changes の修正経路で本文を直す | 本表、`ruleset/2026-08-23-ci-success/after.json`、`.github/workflows/ci.yml` |
