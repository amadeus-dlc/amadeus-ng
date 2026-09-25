# 単体テストの手順 — Issue #134 `replace_pipeline` の書込ロック待ち

## テストフレームワークと設定

- Rust 標準のテストと `#[tokio::test]` を使う（`core-read-model-updater` の既存テストと同じ）。新しい依存や設定は足さない。
- 置き場は `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` の `#[cfg(test)] pub(super) mod tests`。既存の `a_write_lock_held_by_another_connection_is_reported_as_would_block` と同じモジュールに置き、`opened_store` / `raw` / `execution_id` などの既存ヘルパを再利用する。
- テストでの `unwrap` / `expect` は `clippy.toml` で許可されている。添字参照はモジュール冒頭の `#![allow(clippy::indexing_slicing)]` の範囲で使ってよい。

## この修正のテストだけを走らせるコマンド

走らせる前に、ランナーが動くことを既存テストで確かめる（計画の Step 2）。

```bash
cargo test -p core-read-model-updater --lib orchestration::journal_reader_impl::tests::a_write_lock_held_by_another_connection_is_reported_as_would_block -- --exact
```

再現テスト（計画の Step 4〜6。改訂 1 でも名前は変えない）:

```bash
cargo test -p core-read-model-updater --lib orchestration::journal_reader_impl::tests::replace_pipeline_waits_for_a_write_lock_held_by_another_connection -- --exact
```

関係する既存の契約テスト（計画の Step 7。3 本を名前で絞る）:

```bash
cargo test -p aidlc --test pipeline_link_contract -- --exact a_publication_failure_preserves_the_receipt_for_recovery a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery concurrent_duplicate_completions_persist_only_one_receipt
```

計画の Step 7 では、クレート単位のコマンド `cargo test -p core-read-model-updater` も使う。ワークスペース全体のテストとカバレッジは、Build and Test（作業用のチェックアウト）と PR の CI で確かめる。この手順書の単位テストのコマンドには含めない。

実際のテスト名が上の例と違う名前になった場合は、コマンドのフィルタを実際の名前に合わせる必要がある。この手順書は承認の対象なので、書き換えるときは Plan Approval をやり直す。

## 期待するカバレッジ

- ワークスペースの行カバレッジ 90% の床（`scripts/coverage.sh`、CI の `coverage` ジョブ）を割らないこと。閾値や除外の設定は変えない。
- 変更行（トランザクション開始の 1 行）は、新しい再現テストと既存の投影テストの両方で通る。

## 期待する結果

| テスト | 修正前（DEFERRED） | 修正後（IMMEDIATE） |
| --- | --- | --- |
| 新しい再現テスト | 即座（ほぼ 0 秒）に `JournalReadError::Io { kind: WouldBlock, .. }` で失敗 | ホルダの解放を待って `Ok(())`（所要 100ms 以上）、`read_pipeline_progress` が期待どおり |
| `a_write_lock_held_by_another_connection_is_reported_as_would_block` | 緑 | 緑（`advance_checkpoint` の経路で、変更の影響を受けない） |
| 契約テスト 3 本 | 緑（`concurrent_…` はタイミング次第） | 緑 |

## モック・スタブの方針

- モックは使わない。実物の SQLite を一時ファイルで開く。
- ロックを握る相手は、`raw(&path)` で開いた別の `rusqlite::Connection`。別スレッドで `BEGIN IMMEDIATE` を実行し、`std::sync::mpsc` のチャネルで「握った」ことを主スレッドへ知らせる（合図 1）。
- ホルダは合図 1 の後、主スレッドからの「これから `replace_pipeline` を呼ぶ」合図（合図 2）を待つ。合図 2 を受けてから HOLD（200ms）の間だけ握り、COMMIT する（改訂 1）。
- 待つ側は `JournalReaderImpl::open_with_busy_timeout` で開く。busy timeout は 2000ms で、HOLD はこれより十分短い。
- `replace_pipeline` の所要時間を `std::time::Instant` で測る。HOLD の半分（100ms）以上であることを表明し、ロック待ちを実際に観測したことを確かめる（改訂 1）。修正前の失敗は、タイミングに依存せず即時に起きる。

## テストデータの扱い

- `tempfile::tempdir()` の一時ディレクトリにストアを作る。ディレクトリはテストの終わりに自動で消える。
- 本家の表は `opened_store(&dir)` で実物の DDL から作る。`read_pipeline_progress` 表は既存の読取スキーマ準備の経路で作る。
- ホルダのスレッドは、テストの終わりまでに必ず `join` する。失敗の経路でもスレッドを取り残さないように、合図 2 の送信側を落とせばホルダが解放して終わる作りにする。
