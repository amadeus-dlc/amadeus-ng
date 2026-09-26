//! `JournalReader` ポート — 投影 (U4) が使う差分読取と、移行中の公開 (C3 / C6)。

use crate::read_tables::ReadTables;

use super::global_seq_nr::GlobalSeqNr;
use super::journal_batch::JournalBatch;
use super::journal_read_error::JournalReadError;
use super::projection_name::ProjectionName;
use super::{PublicationBatch, ReadModelUpdateError};

/// 投影 (U4) が使う差分読取とチェックポイント (C3 / C6)。
///
/// 集約の永続化そのもの (`IntentExecutionRepository`) とは**別の口**である — 本家
/// event-store-adapter-rs のイベントストアは集約単位の読み書きだけを担い、全集約横断の
/// 順序読取と投影チェックポイントは利用側の関心だからである (ADR-010 決定 4)。
///
/// 本 trait は RMU クレート (`core-read-model-updater`) が所有する
/// (ADR-009 2026-08-28 / 2026-08-29 追記 — 呼ぶのは RMU だけなので中立クレートへ切り出さない)。
/// RMU はライブラリ型を入口に出さないので、**本家の `EventEnvelope` をここから出さない** —
/// 行の材料は我々が所有する [`JournalEntry`](super::JournalEntry) に写して返す。
///
/// 真実源はジャーナルである。差分の順序とチェックポイントの単調性に加え、ファイルの
/// 書込前後を保存した公開計画によって、確定前に停止しても同じ出力を二重追記しない
/// (BR1.4 / NFR3.4)。
///
/// # 移行中である (Issue #153)
///
/// 最後に残すのは差分読取 (`events_after` / `events_through`) だけである
/// (`coding-rules/read-model-updater-structure.md` 原則 2)。構造化面の書込
/// (`advance_checkpoint`) と共有面の点検 (`prepare_read_model`) は PR4 で構造化面の更新器
/// ([`super::StructuredReadModelUpdater`]) と表の DAO へ移した。残っている `pending_publication` /
/// `publish` / `checkpoint` は公開 (Markdown 面) の口で、PR5 で公開の更新器へ移す —
/// `checkpoint` は `publish` が進める番号の読み手なので、`publish` と一緒に動かす
/// (実装はすでにチェックポイントの表の DAO とアンカー照合を通る)。
///
/// 取得・公開は `async fn` (AFIT)。`dyn` は使わず、`Send` / `Sync` 境界も要求しない。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              自動 trait 境界を書けないという注意喚起は本 trait では設計どおりである。"
)]
pub trait JournalReader {
    /// 未完了のファイル公開計画を取得する。
    ///
    /// # Errors
    /// 保存済み計画の読取・復号に失敗した場合。
    async fn pending_publication(
        &self,
        projection: &ProjectionName,
    ) -> Result<Option<PublicationBatch>, JournalReadError>;

    /// 指定した断面までの全履歴を読む。復旧時に新しいイベントを混ぜない。
    ///
    /// # Errors
    /// ジャーナルの読取・復号に失敗した場合。
    async fn events_through(&self, to: GlobalSeqNr) -> Result<JournalBatch, JournalReadError>;

    /// 計画を先に保存し、排他下でファイルを公開してから位置と構造化面を確定する。
    ///
    /// # Errors
    /// 計画・ファイル・確定操作の失敗。未完計画は再開用に保持する。
    async fn publish(
        &mut self,
        projection: &ProjectionName,
        batch: &PublicationBatch,
        tables: &ReadTables,
    ) -> Result<(), ReadModelUpdateError>;
    /// `after` **より大きい** global 通番の行を昇順で走査して返す (全集約横断)。
    ///
    /// 返すのは [`JournalBatch`] — 実行のイベント行 ([`JournalEntry`]) と intent の誕生記録、
    /// そして走査済み最終位置の 3 つ組である。ジャーナルには実行と intent の 2 ストリームが
    /// 同居しており (issue #50)、チェックポイントは種別によらず走査済み最終位置まで進める
    /// (issue #56)。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、復号不能 (`Corrupt`) を返す。
    ///
    /// [`JournalEntry`]: super::journal_entry::JournalEntry
    async fn events_after(&self, after: GlobalSeqNr) -> Result<JournalBatch, JournalReadError>;

    /// 投影のチェックポイント (処理したシーケンス番号) を読む。未登録の投影は
    /// [`GlobalSeqNr::ZERO`]。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、保存済みチェックポイントがジャーナルの現況と食い違う
    /// (`Corrupt` — `CheckpointAnchorMismatch`) を返す。
    async fn checkpoint(
        &self,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, JournalReadError>;
}
