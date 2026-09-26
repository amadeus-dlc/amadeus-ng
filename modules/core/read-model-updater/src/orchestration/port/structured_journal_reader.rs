//! 構造化面の更新器が使うジャーナルの読み手 (移行中の暫定の面ごとの読み手)。

use rusqlite::Connection;

use super::JournalAnchor;
use crate::orchestration::{GlobalSeqNr, JournalBatch, JournalReadError};

/// 構造化面の更新器 ([`crate::orchestration::StructuredReadModelUpdater`]) が使うジャーナルの
/// 読み手。ジャーナルという要素の代理であり、「ある位置より後／までの事実」と「ある位置の行の
/// 識別子」を読むだけである。リードモデル側の型 (行・表・チェックポイント) に依存しない。
///
/// 読取は更新器が渡す接続の上で行う — 更新器が `BEGIN IMMEDIATE` で開いたトランザクションを
/// 渡せば、書込ロックを取った同じ断面で読める (読んだ位置と書き戻す位置が食い違わない)。
/// 読み手自身は接続も状態も持たない。
///
/// **暫定である** (オーナー裁定 2026-09-26 — ジャーナルを読む口は最後に `JournalReader` 1 本へ
/// まとめる)。旧 `JournalReader` はまだ公開 (`publish`) を抱えているので、それを流用すると
/// 書く口ごと依存してしまう。自己診断の `WorkspaceDoctorJournalReader` と同じ理由で、移行の間だけ
/// 面ごとに立てる。
pub trait StructuredJournalReader {
    /// `after` **より後**の行を全集約横断で読む (ジャーナル上の位置の昇順)。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、復号不能 (`Corrupt`) の場合。
    fn events_after(
        &self,
        connection: &Connection,
        after: GlobalSeqNr,
    ) -> Result<JournalBatch, JournalReadError>;

    /// 先頭から `through` **まで**の行を全集約横断で読む (その後に書かれた行を混ぜない)。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、復号不能 (`Corrupt`) の場合。
    fn events_through(
        &self,
        connection: &Connection,
        through: GlobalSeqNr,
    ) -> Result<JournalBatch, JournalReadError>;

    /// 位置 `position` にある行の識別子。その位置に行が無ければ `None`。
    ///
    /// # Errors
    ///
    /// ストア I/O・列の型が違う (`Io`)、位置が列に収まらない・通番が負 (`Corrupt`) の場合。
    fn anchor_at(
        &self,
        connection: &Connection,
        position: GlobalSeqNr,
    ) -> Result<Option<JournalAnchor>, JournalReadError>;
}
