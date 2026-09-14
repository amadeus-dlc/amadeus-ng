//! 開けたイベントストアの形。

/// 表の名前と読み面スキーマの版。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSchema {
    tables: Vec<String>,
    schema_version: i64,
}

impl StoreSchema {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(tables: Vec<String>, schema_version: i64) -> Self {
        Self {
            tables,
            schema_version,
        }
    }

    /// 存在する表の名前 (名前順)。
    #[must_use]
    pub fn tables(&self) -> &[String] {
        &self.tables
    }

    /// `PRAGMA user_version`。
    #[must_use]
    pub const fn schema_version(&self) -> i64 {
        self.schema_version
    }

    /// 名指した表があるか。
    #[must_use]
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.iter().any(|table| table == name)
    }
}
