//! `StructuredSurfaceContent` — 構造化面 20 表の内容 (共有面の記録と照らし合わせる材料)。

use std::io::ErrorKind;

use rusqlite::types::Value;

use super::{JournalReadError, TableContent};

/// 内容の同一性に含める表と、その並び (ダイジェストの材料の一部)。
///
/// **並びも名前も変えてはならない** — 保存済みの記録 (`amadeus_read_model_head.content_digest`)
/// はこの並びで計算されている。変えると既存のストアの記録がすべて食い違い、公開が止まる。
const TABLES: &[&str] = &[
    "read_session_audit",
    "read_artifact_audit",
    "read_answer_result",
    "read_report_result",
    "read_jump_result",
    "read_definition",
    "read_definition_stage",
    "read_definition_scope",
    "read_definition_scope_keyword",
    "read_definition_scope_stage",
    "read_definition_scope_phase_entry",
    "read_intent",
    "read_intent_stage",
    "read_execution",
    "read_execution_stage",
    "read_next_answer",
    "read_next_jump",
    "read_next_jump_phase",
    "read_run_stage",
    "read_scope_change",
];

/// 構造化面 20 表の内容 (表ごとの全行の生の値を、上の並びで束ねたもの)。
///
/// 表をまたぐ同一性 (2 つの断面が同じか・記録のダイジェストと合うか) はこの値が持つ —
/// 表の DAO は 1 表の値を読むだけで、表をまたぐ照合をしない
/// (`coding-rules/read-model-updater-structure.md` 原則 3)。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StructuredSurfaceContent {
    tables: Vec<TableContent>,
}

impl StructuredSurfaceContent {
    /// 20 表の内容を、`read_session_audit` から `read_scope_change` までの決まった並びで束ねる。
    pub(crate) fn new(tables: [TableContent; 20]) -> Self {
        Self {
            tables: tables.into(),
        }
    }

    /// 値の格納型・値・表の並び・行の並びを含む内容のダイジェスト。大きな整数もバイト列として
    /// 扱うので、丸めで別の内容が同じダイジェストになることは無い。
    ///
    /// # Errors
    ///
    /// 材料を正準 JSON にできない場合 (`Io`)。
    pub(crate) fn digest(&self) -> Result<String, JournalReadError> {
        let material = self
            .tables
            .iter()
            .map(|table| {
                table
                    .rows()
                    .iter()
                    .map(|row| row.iter().map(tagged).collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let json =
            core_infrastructure::canon_json::to_value(&(TABLES, material)).map_err(|_| {
                JournalReadError::Io {
                    kind: ErrorKind::Other,
                    path: None,
                }
            })?;
        Ok(core_infrastructure::canon_json::hash_compact(&json).rendered())
    }
}

/// 値 1 つを (格納型の印, バイト列) へ写す。
fn tagged(value: &Value) -> (u8, Vec<u8>) {
    match value {
        Value::Null => (0, Vec::new()),
        Value::Integer(value) => (1, value.to_be_bytes().to_vec()),
        Value::Real(value) => (2, value.to_bits().to_be_bytes().to_vec()),
        Value::Text(value) => (3, value.clone().into_bytes()),
        Value::Blob(value) => (4, value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content(first: Vec<Vec<Value>>) -> StructuredSurfaceContent {
        let mut tables: [TableContent; 20] = std::array::from_fn(|_| TableContent::new(Vec::new()));
        tables[0] = TableContent::new(first);
        StructuredSurfaceContent::new(tables)
    }

    #[test]
    fn equal_contents_have_equal_digests_and_a_changed_value_changes_it() {
        let a = content(vec![vec![Value::Text("x".into()), Value::Integer(1)]]);
        let b = content(vec![vec![Value::Text("x".into()), Value::Integer(1)]]);
        let c = content(vec![vec![Value::Text("x".into()), Value::Integer(2)]]);
        assert_eq!(a, b);
        assert_eq!(a.digest().unwrap(), b.digest().unwrap());
        assert_ne!(a, c);
        assert_ne!(a.digest().unwrap(), c.digest().unwrap());
    }

    #[test]
    fn the_storage_type_is_part_of_the_identity() {
        // 同じ見た目でも格納型が違えば別の内容である (整数の 1 と文字列の "1")。
        let integer = content(vec![vec![Value::Integer(1)]]);
        let text = content(vec![vec![Value::Text("1".into())]]);
        assert_ne!(integer.digest().unwrap(), text.digest().unwrap());
    }

    #[test]
    fn the_digest_of_an_empty_surface_is_pinned() {
        // 保存済みの記録はこの計算で作られている (Issue #153 の PR4 で表ごとの DAO へ分ける前の
        // `read_tables::content_digest` と同じ値であることを、分ける時点で実測して釘留めした)。
        // 材料の形 (表の並び・値の写し方) を変えると既存ストアの記録と食い違う。
        let empty =
            StructuredSurfaceContent::new(std::array::from_fn(|_| TableContent::new(Vec::new())));
        assert_eq!(
            empty.digest().unwrap(),
            "73c311160f476db10ecb33477d7509128b0c338c5299dfe77d1dabbdfe20c3ec"
        );
    }
}
