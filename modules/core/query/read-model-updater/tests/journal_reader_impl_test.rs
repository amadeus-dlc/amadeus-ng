//! `JournalReaderImpl` の契約 (BR1.4) — 全集約横断の順序読取と投影チェックポイント。
//!
//! この面は本家 event-store-adapter-rs に無い (ADR-010 決定 4)。本家の `journal` 表を
//! 同じ DB ファイルへの**別接続**から `rowid` 順に読み、チェックポイントは我々の表に持つ。
//! したがって SQLite バックエンドにしか存在しない。
//!
//! 本家スキーマそのものへのピン留め (ガードテスト) は `journal_reader_impl.rs` の
//! インラインテストが持つ。ここが見るのは読み方の約束である。
//!
//! ジャーナル行を書くのは**本家のイベントストア**であり、コマンド側の Repository ではない
//! (`tests/support/mod.rs` 冒頭の理由を参照)。

// テストコードでは unwrap / expect を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod support;

use core_domain::workspace::{SpaceName, StorePath};
use core_query_read_model_updater::orchestration::{
    CorruptCause, GlobalSeqNr, JournalEntry, JournalReadError, JournalReader, JournalReaderImpl,
    ProjectionName,
};
use rusqlite::Connection;
use tempfile::TempDir;

use support::{JournalWriter, UpstreamStore, at, intent_id, open_store, other_intent_id, seed};

/// 一時ディレクトリ配下の 1 つのストアファイル。
struct Fixture {
    _dir: TempDir,
    path: StorePath,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir を持つ"))
            .expect("intents/ を先に作る");
        Fixture { _dir: dir, path }
    }

    /// ジャーナル行の書き手 — 本家のイベントストアそのもの (コマンド側の Repository では
    /// ない。`support` モジュールの冒頭を参照)。
    fn store(&self) -> UpstreamStore {
        open_store(&self.path)
    }

    fn reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.path).expect("Reader は開ける")
    }

    fn raw(&self) -> Connection {
        Connection::open(self.path.as_path()).expect("生の接続")
    }
}

fn projection() -> ProjectionName {
    ProjectionName::parse("state-file").expect("投影名は kebab")
}

// ---------------------------------------------------------------------------
// 差分読取 (BR1.4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_journal_reads_every_event_in_global_order() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let reader = fixture.reader();
    let rows = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect("差分読取");
    assert_eq!(rows.len(), 5);
    let globals: Vec<u64> = rows
        .iter()
        .map(|entry| entry.global_seq().to_u64())
        .collect();
    let mut sorted = globals.clone();
    sorted.sort_unstable();
    assert_eq!(globals, sorted, "global 通番の昇順");
    let seqs: Vec<usize> = rows.iter().map(JournalEntry::seq_nr).collect();
    assert_eq!(seqs, [1, 2, 3, 4, 5], "欠落なく順に読める");
    assert!(
        rows.iter().all(|entry| entry.intent_id() == &intent_id()),
        "集約識別子が境界を越えて残る"
    );
    assert!(
        rows.iter().all(|entry| entry.occurred_at() == &at()),
        "発生時刻はドメイン供給値のまま往復する"
    );
}

#[tokio::test]
async fn the_journal_reads_only_the_difference() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let reader = fixture.reader();
    let all = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let third = all.get(2).expect("3 件目").global_seq();
    let rest = reader.events_after(third).await.expect("差分");
    assert_eq!(rest.len(), 2);
    assert!(rest.iter().all(|entry| entry.global_seq() > third));
}

#[tokio::test]
async fn a_renumbered_journal_is_refused_by_the_anchor() {
    // rowid の振り直し (仕様が許す再番号付け) が起きると、保存済みチェックポイントの
    // アンカー (aid, seq_nr) が journal の同 rowid と食い違う。差分読取の欠落・重複という
    // 静かな破損ではなく、明示エラーで止まることを検証する。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    drop(store);

    let mut reader = fixture.reader();
    let before = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let third = before.get(2).expect("3 件目").global_seq();
    reader
        .advance_checkpoint(&projection(), third)
        .await
        .expect("チェックポイント前進");
    drop(reader);

    // SQLite の仕様は「INTEGER PRIMARY KEY の無い表の rowid を VACUUM が変えてよい」と
    // している (現行 3.51 は隙間があっても保持する — 下の VACUUM 釘留めテスト参照)。
    // ここでは仕様が許す振り直しそのもの (先頭行の削除 + 後続の繰り上げ) を直接再現する。
    let conn = fixture.raw();
    conn.execute("DELETE FROM journal WHERE rowid = 1", [])
        .expect("先頭行を消して隙間を作る");
    for old_rowid in 2i64..=5 {
        conn.execute(
            "UPDATE journal SET rowid = ?1 WHERE rowid = ?2",
            rusqlite::params![old_rowid - 1, old_rowid],
        )
        .expect("rowid を繰り上げる");
    }
    drop(conn);

    let reader = fixture.reader();
    let error = reader
        .checkpoint(&projection())
        .await
        .expect_err("アンカー照合で止まる");
    assert!(
        matches!(
            error,
            JournalReadError::Corrupt {
                cause: CorruptCause::CheckpointAnchorMismatch,
                ..
            }
        ),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_truncated_journal_behind_the_checkpoint_is_refused() {
    // チェックポイントが指す行そのものが消えている場合もアンカー照合で止まる。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    drop(store);

    let mut reader = fixture.reader();
    let all = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let last = all.last().expect("末尾").global_seq();
    reader
        .advance_checkpoint(&projection(), last)
        .await
        .expect("チェックポイント前進");
    drop(reader);

    fixture
        .raw()
        .execute("DELETE FROM journal WHERE rowid >= 4", [])
        .expect("末尾を切り落とす");

    let reader = fixture.reader();
    let error = reader
        .checkpoint(&projection())
        .await
        .expect_err("指し先が無い");
    assert!(
        matches!(
            error,
            JournalReadError::Corrupt {
                cause: CorruptCause::CheckpointAnchorMismatch,
                ..
            }
        ),
        "実際: {error:?}"
    );
}

#[tokio::test]
async fn a_vacuum_rebuild_does_not_move_the_cursor() {
    // journal は削除ゼロの純追記 (DELETE は本家 v2.0.0 でも snapshot 表にしか無い) なので、
    // rowid は隙間の無い連番 1..N であり、VACUUM の再構築でも値が保たれる。ここはその前提を
    // 実挙動で釘留めする回帰テスト — 破れたらアンカー照合 (上の 2 テスト) が実行時に
    // 明示エラーで止める。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    drop(store);

    let mut reader = fixture.reader();
    let before = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let third = before.get(2).expect("3 件目").global_seq();
    reader
        .advance_checkpoint(&projection(), third)
        .await
        .expect("チェックポイント前進");
    drop(reader); // VACUUM は排他を要するため他接続を全部閉じてから実行する

    fixture
        .raw()
        .execute_batch("VACUUM")
        .expect("VACUUM は通る");

    let reader = fixture.reader();
    let after = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let keys = |rows: &[JournalEntry]| -> Vec<u64> {
        rows.iter()
            .map(|entry| entry.global_seq().to_u64())
            .collect::<Vec<_>>()
    };
    assert_eq!(keys(&after), keys(&before), "rowid は VACUUM 前と同一");
    let saved = reader.checkpoint(&projection()).await.expect("保存済み");
    assert_eq!(saved, third, "チェックポイントも生きている");
    let resumed = reader.events_after(saved).await.expect("差分");
    assert_eq!(
        keys(&resumed),
        keys(&before)[3..].to_vec(),
        "続行が欠落も重複もしない"
    );
}

#[tokio::test]
async fn the_journal_interleaves_two_aggregates_in_commit_order() {
    // 本家のイベントストアは集約単位でしか読めない。横断のカーソルが「コミット順」で
    // あることこそ、この実装を自前で持つ理由である (ADR-010 決定 4)。
    let fixture = Fixture::new();
    let mut store = fixture.store();

    let mut first = JournalWriter::start(&mut store, intent_id()).await;
    JournalWriter::start(&mut store, other_intent_id()).await;
    first
        .advance(&mut store, |aggregate| aggregate.complete_stage(at()))
        .await;

    let reader = fixture.reader();
    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    assert_eq!(
        rows.iter()
            .map(|entry| (
                entry.global_seq().to_u64(),
                entry.intent_id().as_str().to_string()
            ))
            .collect::<Vec<_>>(),
        [
            (1, support::INTENT.to_string()),
            (2, support::OTHER_INTENT.to_string()),
            (3, support::INTENT.to_string()),
        ],
        "書いた順に 1 本の列へ並ぶ"
    );
}

#[tokio::test]
async fn the_reader_observes_writes_made_after_it_was_opened() {
    // Reader は同じファイルへの**生きた接続**である (写しではない)。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let mut writer = JournalWriter::start(&mut store, intent_id()).await;

    let reader = fixture.reader();
    writer
        .advance(&mut store, |aggregate| aggregate.complete_stage(at()))
        .await;

    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    assert_eq!(rows.len(), 2);
}

#[tokio::test]
async fn a_tampered_journal_payload_is_corrupt() {
    // 行を壊せるのは生の SQL だけである (実装に破壊用のフックを開けない — BR2.8)。
    let fixture = Fixture::new();
    let mut store = fixture.store();
    JournalWriter::start(&mut store, intent_id()).await;
    fixture
        .raw()
        .execute("UPDATE journal SET payload = X'7B6E6F74206A736F6E'", [])
        .expect("payload を壊す");

    let reader = fixture.reader();
    let err = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .expect_err("復号できない");
    assert_eq!(
        err,
        JournalReadError::Corrupt {
            aggregate_id: support::INTENT.to_string(),
            seq_nr: None,
            cause: CorruptCause::UndecodablePayload,
        }
    );
}

// ---------------------------------------------------------------------------
// チェックポイント (BR1.4)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_unregistered_projection_reads_as_zero() {
    let fixture = Fixture::new();
    let _store = fixture.store();
    let reader = fixture.reader();
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::ZERO
    );
}

#[tokio::test]
async fn the_checkpoint_advances_and_repeats_are_noops() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let mut reader = fixture.reader();
    let rows = reader.events_after(GlobalSeqNr::ZERO).await.expect("全件");
    let last = rows.last().expect("5 件ある").global_seq();

    reader
        .advance_checkpoint(&projection(), last)
        .await
        .expect("前進");
    assert_eq!(reader.checkpoint(&projection()).await.expect("読取"), last);

    reader
        .advance_checkpoint(&projection(), last)
        .await
        .expect("同値は no-op");
    assert_eq!(reader.checkpoint(&projection()).await.expect("読取"), last);
    assert!(
        reader.events_after(last).await.expect("差分").is_empty(),
        "追いついた投影に差分は無い"
    );
}

#[tokio::test]
async fn the_checkpoint_survives_reopening_the_store() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;
    {
        let mut reader = fixture.reader();
        reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(3))
            .await
            .expect("前進");
    }

    let reader = fixture.reader();
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::new(3),
        "自前の表はファイルに残る"
    );
}

#[tokio::test]
async fn a_checkpoint_regression_is_refused() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let mut reader = fixture.reader();
    reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(3))
        .await
        .expect("前進");
    let err = reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(2))
        .await
        .expect_err("後退は拒否");
    assert_eq!(
        err,
        JournalReadError::CheckpointRegression {
            projection: projection(),
            current: GlobalSeqNr::new(3),
            requested: GlobalSeqNr::new(2),
        }
    );
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::new(3),
        "拒否しても現在値は動かない"
    );
}

#[tokio::test]
async fn two_projections_keep_independent_checkpoints() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    seed(&mut store).await;

    let other = ProjectionName::parse("intents-registry").expect("投影名は kebab");
    let mut reader = fixture.reader();
    reader
        .advance_checkpoint(&projection(), GlobalSeqNr::new(4))
        .await
        .expect("前進");
    assert_eq!(
        reader.checkpoint(&other).await.expect("読取"),
        GlobalSeqNr::ZERO,
        "別の投影は動かない"
    );
    reader
        .advance_checkpoint(&other, GlobalSeqNr::new(2))
        .await
        .expect("前進");
    assert_eq!(
        reader.checkpoint(&projection()).await.expect("読取"),
        GlobalSeqNr::new(4)
    );
    assert_eq!(
        reader.checkpoint(&other).await.expect("読取"),
        GlobalSeqNr::new(2)
    );
}
