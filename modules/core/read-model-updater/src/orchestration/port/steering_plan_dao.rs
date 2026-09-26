//! `read_steering_plan` 表の DAO — この表 1 つの I/O だけを持つ。

use rusqlite::{Connection, Transaction};

use super::SteeringPlanRow;
use crate::orchestration::JournalReadError;

/// `read_steering_plan` 表の DAO。
///
/// 単一テーブルの I/O だけを持つ (オーナー裁定 2026-09-26)。対になる `read_steering_part` には
/// 触れない — 2 表を同じトランザクションで差し替えるのは更新器
/// ([`crate::orchestration::SteeringReadModelUpdater`]) の仕事である。保存済みの
/// `source_digest` といま読んだ参照入力を比べて書くかどうかを決めるのも更新器であり、
/// この DAO は値を読み書きするだけである。
///
/// 書込は更新器が開いたトランザクションを `&mut` で受け取る。DAO 自身は状態を持たない。
pub trait SteeringPlanDao {
    /// 表と索引が無ければ作る (冪等)。
    ///
    /// # Errors
    ///
    /// 表を作れない場合 (`Io`)。
    fn create_table(&self, transaction: &mut Transaction<'_>) -> Result<(), JournalReadError>;

    /// 表が在るか (`sqlite_master` の読取だけで、書込ロックを取らない)。
    ///
    /// 更新器は開く段でこれを見て、表が揃っていれば書込トランザクションを開かない。
    /// 欠けているときだけ `BEGIN IMMEDIATE` の中で [`Self::create_table`] を呼ぶ。
    ///
    /// # Errors
    ///
    /// 読めない場合 (`Io`)。
    fn table_exists(&self, connection: &Connection) -> Result<bool, JournalReadError>;

    /// 保存済みの行が名乗る参照入力の照合子。行が無ければ `None`。
    ///
    /// 全行に同じ値が書かれているので、`phase` が最小の行の値を返す。
    ///
    /// # Errors
    ///
    /// 読めない場合 (`Io`)。
    fn find_source_digest(
        &self,
        connection: &Connection,
    ) -> Result<Option<String>, JournalReadError>;

    /// 表の行をすべて `rows` に差し替える。`source_digest` は全行へ同じ値を書く。
    ///
    /// # Errors
    ///
    /// 書けない (`Io`)、値が列に収まらない (`Corrupt`) 場合。
    fn replace(
        &self,
        transaction: &mut Transaction<'_>,
        rows: &[SteeringPlanRow],
        source_digest: &str,
    ) -> Result<(), JournalReadError>;
}
