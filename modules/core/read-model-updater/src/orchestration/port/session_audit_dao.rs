//! `read_session_audit` 表の DAO — この表 1 つの I/O だけを持つ。

use rusqlite::{Connection, Transaction};

use super::{SessionAuditRow, TableContent};
use crate::orchestration::{GlobalSeqNr, JournalReadError};

/// `read_session_audit` 表 (セッション監査の通知ごとの結果) の DAO。
///
/// 単一テーブルの I/O だけを持つ (オーナー裁定 2026-09-26)。構造化面の 20 表を同じ
/// トランザクションで差し替え、処理したシーケンス番号と一緒に確定すること、全表の内容が
/// 共有面の記録と一致するかを確かめることは更新器
/// ([`crate::orchestration::StructuredReadModelUpdater`]) の仕事であり、この DAO は値を
/// 読み書きするだけである。表の DDL (索引を含む) はこの DAO が持つ。
///
/// 書込は更新器が開いたトランザクションを `&mut` で受け取る。DAO 自身は状態を持たない。
pub trait SessionAuditDao {
    /// 表と索引が無ければ作る (冪等)。
    ///
    /// # Errors
    ///
    /// 表を作れない場合 (`Io`)。
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 表と、その表の索引がすべて在るか (`sqlite_master` の読取だけで、書込ロックを取らない)。
    ///
    /// 更新器は開く段でこれを見て、揃っていれば書込トランザクションを開かない。欠けていれば
    /// [`Self::create_table`] を呼ぶ — 索引だけが欠けた表も作り直しの対象である (索引の DDL が
    /// 崩れた表の形を見つける)。
    ///
    /// # Errors
    ///
    /// 読めない場合 (`Io`)。
    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError>;

    /// 表を落とす (索引も一緒に落ちる)。読み面の版が動いたときの作り直しだけが呼ぶ。
    ///
    /// # Errors
    ///
    /// 落とせない場合 (`Io`)。
    fn drop_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 全行を消す (全差し替えの前半)。
    ///
    /// # Errors
    ///
    /// 消せない場合 (`Io`)。
    fn delete_all(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// `rows` を保存する (主キー `id` が同じ行は上書きする)。`as_of` は全行へ同じ値を書く。
    ///
    /// 上書きで保存するのは、監査だけを描く断面が既存の行を消さずに書き足すからである
    /// (どの断面で消すかを決めるのは更新器)。
    ///
    /// # Errors
    ///
    /// 書けない (`Io`)、値が列に収まらない (`Corrupt`) 場合。
    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        rows: &[SessionAuditRow],
        as_of: GlobalSeqNr,
    ) -> Result<(), JournalReadError>;

    /// 格納された全行の生の値 (主キー `id` の昇順、列は表の定義順)。
    ///
    /// 構造化面の内容の同一性 (共有面の記録との照合) の材料である。照合そのものは更新器が
    /// 20 表ぶんを束ねて行う。
    ///
    /// # Errors
    ///
    /// 読めない場合 (`Io`)。
    fn find_content(&self, connection: &Connection) -> Result<TableContent, JournalReadError>;
}
