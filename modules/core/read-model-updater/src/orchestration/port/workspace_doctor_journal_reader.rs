//! 自己診断の事実をジャーナルから読むポート — ジャーナルという要素の代理。

use rusqlite::Connection;

use super::WorkspaceDoctorJournalEntry;
use crate::orchestration::{GlobalSeqNr, JournalReadError};

/// ジャーナルから自己診断の事実 (`workspace-doctor-event/1`) だけを読む。
///
/// ジャーナルの要素 — **ある位置より後／までの事実を読む** — だけを持ち、リードモデル側の
/// 型 (行・表・チェックポイント) に依存しない (オーナー裁定 2026-09-26)。読む位置を決める
/// のも、読んだ事実をどう投影するかを決めるのも更新器である。
///
/// 読取は更新器が開いたトランザクションの上で行う (`&Transaction` は `&Connection` に
/// 参照外しされる)。書込ロックを取ったトランザクションの中で読むので、読んだ位置と
/// 書き戻す位置が食い違わない。
pub trait WorkspaceDoctorJournalReader {
    /// `after` **より大きい**位置の事実を、位置の昇順で返す。
    ///
    /// # Errors
    ///
    /// ジャーナルを読めない (`Io`)、行を事実へ復号できない (`Corrupt`) 場合。
    fn events_after(
        &self,
        connection: &Connection,
        after: GlobalSeqNr,
    ) -> Result<Vec<WorkspaceDoctorJournalEntry>, JournalReadError>;

    /// `to` **以下**の位置の事実を、位置の昇順で返す。集約を最初から起こすときに使う。
    ///
    /// # Errors
    ///
    /// ジャーナルを読めない (`Io`)、行を事実へ復号できない (`Corrupt`) 場合。
    fn events_through(
        &self,
        connection: &Connection,
        to: GlobalSeqNr,
    ) -> Result<Vec<WorkspaceDoctorJournalEntry>, JournalReadError>;
}
