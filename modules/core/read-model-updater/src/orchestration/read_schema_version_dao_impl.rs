//! `ReadSchemaVersionDao` の SQLite 実装 — `PRAGMA user_version` 1 つだけを読み書きする。

use rusqlite::{Connection, Transaction};

use super::store_failure::SqliteResultExt;
use super::{JournalReadError, ReadSchemaVersionDao};

/// 保存されている版。
const SELECT: &str = "PRAGMA user_version";

/// 読み面スキーマの版の DAO の実装。状態を持たない。
#[derive(Debug, Clone, Copy, Default)]
pub struct ReadSchemaVersionDaoImpl;

impl ReadSchemaVersionDao for ReadSchemaVersionDaoImpl {
    fn find(&self, connection: &Connection) -> Result<i64, JournalReadError> {
        connection
            .query_row(SELECT, [], |row| row.get(0))
            .at_connection(connection)
    }

    fn save(
        &self,
        transaction: &mut Transaction<'_>,
        version: i64,
    ) -> Result<(), JournalReadError> {
        // `PRAGMA` は束縛変数を取れないので、値は `i64` として整形する (外から来る文字列は
        // 通らないので SQL の注入面は無い)。
        transaction
            .execute_batch(&format!("PRAGMA user_version = {version}"))
            .at_connection(transaction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unset_version_is_zero_and_a_saved_one_reads_back() {
        let mut connection = Connection::open_in_memory().unwrap();
        assert_eq!(ReadSchemaVersionDaoImpl.find(&connection).unwrap(), 0);
        let mut transaction = connection.transaction().unwrap();
        ReadSchemaVersionDaoImpl.save(&mut transaction, 7).unwrap();
        transaction.commit().unwrap();
        assert_eq!(ReadSchemaVersionDaoImpl.find(&connection).unwrap(), 7);
    }

    #[test]
    fn a_version_saved_in_a_rolled_back_transaction_does_not_stick() {
        let mut connection = Connection::open_in_memory().unwrap();
        let mut transaction = connection.transaction().unwrap();
        ReadSchemaVersionDaoImpl.save(&mut transaction, 7).unwrap();
        drop(transaction);
        assert_eq!(ReadSchemaVersionDaoImpl.find(&connection).unwrap(), 0);
    }
}
