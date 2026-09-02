//! `ReadTablesError` — 構造化投影核が読み替えずに止める理由。

use std::fmt;

/// 構造化投影 ([`ReadTables::project`]) の失敗。
///
/// どちらも**歴史が途中から切り落とされた兆候**である。読み替えて部分的な行を書くと、
/// 読取コマンドが「在るはずの答えが無い」ではなく「間違った答えが在る」を見ることになる
/// ので、行を 1 つも書かずに止める。
///
/// 材料だけを運ぶ (`coding-rules/error-handling.md` — `Display` は開発者向けの診断であり、
/// 利用者向けの文言は出す側が組む)。
///
/// [`ReadTables::project`]: super::ReadTables::project
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadTablesError {
    /// ストリームの先頭が誕生記録 (`Started` / `Defined`) ではない。
    ///
    /// 集約は誕生記録を種にしてしか起こせないので、先頭が欠けた歴史からは 1 行も導けない。
    MissingGenesis {
        /// 先頭が欠けていたストリームの集約識別子。
        aggregate_id: String,
    },
    /// 実行が指す intent の誕生記録が履歴に無い。
    ///
    /// ゲート前提 (`gated`) と jump の検証は intent を要するクエリなので、intent が無ければ
    /// 実行の行を埋められない。
    IntentUnavailable {
        /// 材料が足りなかった実行の識別子。
        execution_id: String,
        /// その実行が指していた intent の識別子。
        intent_id: String,
    },
}

impl fmt::Display for ReadTablesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadTablesError::MissingGenesis { aggregate_id } => {
                write!(f, "missing genesis for {aggregate_id}")
            }
            ReadTablesError::IntentUnavailable {
                execution_id,
                intent_id,
            } => write!(f, "intent {intent_id} unavailable for {execution_id}"),
        }
    }
}

impl std::error::Error for ReadTablesError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_failure_renders_its_material() {
        assert_eq!(
            ReadTablesError::MissingGenesis {
                aggregate_id: "claude".to_string()
            }
            .to_string(),
            "missing genesis for claude"
        );
        assert_eq!(
            ReadTablesError::IntentUnavailable {
                execution_id: "exec".to_string(),
                intent_id: "intent".to_string()
            }
            .to_string(),
            "intent intent unavailable for exec"
        );
    }

    #[test]
    fn the_failure_owns_its_material_so_the_chain_ends_here() {
        let error = ReadTablesError::MissingGenesis {
            aggregate_id: "claude".to_string(),
        };
        assert!(std::error::Error::source(&error).is_none());
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert_eq!(boxed.to_string(), "missing genesis for claude");
    }
}
