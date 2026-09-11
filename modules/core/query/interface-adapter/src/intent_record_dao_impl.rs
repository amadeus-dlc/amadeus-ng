//! RMUが保持するintents.jsonのID対応を読む。
use core_query_use_case::orchestration::{IntentRecordDao, IntentRecordView, ReadModelReadError};
use std::{fs, io, path::PathBuf};
/// 人が持つ既存登録も含む投影済み登録簿の読取り。
#[derive(Debug)]
pub struct IntentRecordDaoImpl {
    path: PathBuf,
}
impl IntentRecordDaoImpl {
    /// 対象登録簿を指定する。
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }
    fn error(&self, kind: io::ErrorKind) -> ReadModelReadError {
        ReadModelReadError::new(kind, Some(self.path.clone()))
    }
}
impl IntentRecordDao for IntentRecordDaoImpl {
    fn find(&self, intent_id: &str) -> Result<Option<IntentRecordView>, ReadModelReadError> {
        let bytes = fs::read(&self.path).map_err(|error| self.error(error.kind()))?;
        let entries: Vec<serde_json::Value> =
            serde_json::from_slice(&bytes).map_err(|_| self.error(io::ErrorKind::InvalidData))?;
        let mut found = None;
        for entry in entries {
            if entry.get("uuid").and_then(serde_json::Value::as_str) != Some(intent_id) {
                continue;
            }
            if found.is_some() {
                return Err(self.error(io::ErrorKind::InvalidData));
            }
            let directory = entry
                .get("dirName")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| self.error(io::ErrorKind::InvalidData))?;
            found = Some(IntentRecordView::new(
                intent_id.to_string(),
                directory.to_string(),
            ));
        }
        Ok(found)
    }
}
