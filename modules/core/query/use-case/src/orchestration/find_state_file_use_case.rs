//! `FindStateFileUseCase` — record の状態ファイル 1 面を引く。

use crate::orchestration::{ReadModelReadError, StateFileDao};

/// upstream 互換の状態ファイル (`aidlc-state.md`) の生テキストを引く。
///
/// 本体は `execute() = dao.find()` だけである — 判断・導出・選択・文言組立のどれも持たない
/// (`coding-rules/cqrs-boundaries.md` 規則 6)。引いたテキストを分類するのは
/// `StateVersionClassification`、その結果を文言にするのはプレゼンタである。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindStateFileUseCase<D: StateFileDao> {
    state_file_dao: D,
}

impl<D: StateFileDao> FindStateFileUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(state_file_dao: D) -> FindStateFileUseCase<D> {
        FindStateFileUseCase { state_file_dao }
    }

    /// 状態ファイルの生テキストを引く (不在は `Ok(None)`)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(&self) -> Result<Option<String>, ReadModelReadError> {
        self.state_file_dao.find()
    }
}
