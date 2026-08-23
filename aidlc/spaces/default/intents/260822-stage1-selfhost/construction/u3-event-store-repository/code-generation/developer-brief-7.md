# developer-brief-7 — 委任 7: カバレッジの回復（相対ゲート）（U3 / Bolt B5）

Conversation language: 日本語（コメント・報告はすべて日本語）。

## 背景

受入 BR5.2 (b) の相対ゲート（`scripts/coverage.sh --base origin/main`、TOLERANCE 0.01 — 本 Bolt で引き締め）が **head 96.81% < base 97.39%** で赤。退役で消えた
テスト（37 本）の分だけ分母比が動き、新規コード（adapter の ストア / ワイヤ / InMemory）のエラー経路が未カバーのまま残っているため。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（Bolt B5）の委任 7。リポジトリルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、ブランチ
`bolt/b5-u3-event-store-repository`（委任 1〜6 はコミット済み。今はあなたしか走っていません）。

所有ファイル: **テストコードのみ** — `modules/core/interface-adapter/tests/**`（新規ファイル可）、`modules/core/interface-adapter/src/orchestration/**` と
`modules/core/use-case/src/orchestration/**` の既存ファイル内の `#[cfg(test)] mod tests`（追記のみ — プロダクトコードは変更しない。ただし到達不能と判明した分岐を
テスト可能にするための**挙動不変の最小リファクタ**（例: 私有関数の切り出し）は可、報告に明記）。報告 `developer-report-7.md`（新規）。
触らないもの: 計画・検査手順・質問票、`docs/**`、`formal/**`、`scripts/**`、`Cargo.toml`、`tools/lint`。`git add` / `git commit` はしない。`.claude/` のツールは実行しない。

## 先に読むもの

1. `.../u3-event-store-repository/code-generation/coverage-gaps-b5.md`（未カバー行の一覧 — これが作業のマップ）
2. `.../u3-event-store-repository/code-generation/code-generation-plan.md` §5.2〜§5.3、`unit-test-instructions.md`
3. 対象ファイルの該当行（`event_store_impl.rs` / `memory/workflow_execution_repository.rs` / `memory/in_memory_event_store.rs` / `wire/mod.rs` / `wire/state_wire.rs` /
   `workflow_execution_repository_impl.rs` / `workflow_definition_repository_impl.rs`）と既存テスト（`tests/support/**`、`tests/*_test.rs`、インライン tests）。

## 作業

1. `coverage-gaps-b5.md` の行を上から順に、**意味のあるテスト**（エラー経路: Io 写像・Corrupt 各原因・Schema・CheckpointRegression・Busy、Display の材料、
   InMemory Repository の `find_by_id` 経路（snapshot なし / replay 1 件以上 / from_state 失敗）、ワイヤの拒否分岐、`workflow_definition_repository_impl.rs` の
   `NotFound` / `HarnessIdentity` / 読取失敗の各変種…）でカバーする。到達不能コード（構造的に起きない分岐）は無理に通さず、報告に「到達不能（理由）」として列挙。
2. 目標: ワークスペース全体で **+70 行以上**の新規カバー（`cargo llvm-cov --workspace --lcov` で前後比較、`PROPTEST_RNG_SEED=20260823`）。
   達成確認は `bash scripts/coverage.sh --base origin/main` が `[PASS] relative gate` になること（約 5 分かかる）。
3. 検査: `cargo test --workspace` 全緑、`cargo clippy --workspace --all-targets -- -D warnings`（`indexing_slicing` / `panic` は deny — テストは file/mod 単位の `#![allow]` で）、
   `cargo fmt --all --check`。
4. 報告 `developer-report-7.md`: 「追加テスト一覧（ファイル / 件数 / 対象行）」「到達不能として残した行」「coverage.sh の出力（before / after、相対ゲート）」「検査結果」
   「設計質問」「未了」。最終応答は要約（日本語、10 行以内）。
