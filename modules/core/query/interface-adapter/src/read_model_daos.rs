//! `ReadModelDaos` — 1 要求ぶんの読取専用接続と、その上に建つ 12 の DAO 実装。

use std::path::Path;
use std::rc::Rc;

use core_query_use_case::orchestration::ReadModelReadError;

use super::definition_dao_impl::DefinitionDaoImpl;
use super::execution_dao_impl::ExecutionDaoImpl;
use super::jump_dao_impl::JumpDaoImpl;
use super::jump_phase_dao_impl::JumpPhaseDaoImpl;
use super::next_answer_dao_impl::NextAnswerDaoImpl;
use super::phase_entry_dao_impl::PhaseEntryDaoImpl;
use super::read_model_store::ReadModelStore;
use super::run_stage_dao_impl::RunStageDaoImpl;
use super::scope_change_dao_impl::ScopeChangeDaoImpl;
use super::scope_dao_impl::ScopeDaoImpl;
use super::scope_keyword_dao_impl::ScopeKeywordDaoImpl;
use super::steering_part_dao_impl::SteeringPartDaoImpl;
use super::steering_plan_dao_impl::SteeringPlanDaoImpl;

/// **1 要求 = 1 接続**。12 の DAO 実装はこの 1 つを分け合う。
///
/// # なぜ束ねるのか
///
/// `next` は 1 要求で最大 5 表を、`continue` は 3 表をたどる (ユースケースが FK を追う —
/// `coding-rules/cqrs-boundaries.md` 規則 6)。実装ごとに接続を開くと、その多段の引当が
/// 別々のスナップショットを見る余地が残る (RMU の差し替えは 1 トランザクションなので、
/// 途中で差し替わりうる)。開く口をここ 1 か所にすれば、たどる間じゅう同じ接続である。
///
/// # 共有は不変共有である
///
/// 束ねた `ReadModelStore` は読取専用で開いた接続であり、DAO はそれを読むだけである。
/// したがって共有に要るのは参照の複製だけで、内部可変性は要らない — `Rc` で足りる
/// (`coding-rules/interior-mutability.md`「共有が不要な型を `*Shared` でラップしない」)。
///
/// # 開く口であって DAO ではない
///
/// この型は `find` を 1 本も持たない。持つのは「どの実装をどの接続の上に建てるか」だけで
/// あり、ポート ([`core_query_use_case::orchestration::NextAnswerDao`] 他) を実装するのは
/// 建てられた側である。
#[derive(Debug, Clone)]
pub struct ReadModelDaos {
    store: Rc<ReadModelStore>,
}

impl ReadModelDaos {
    /// 構造化リードモデルのストアを読取専用で 1 度だけ開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<ReadModelDaos, ReadModelReadError> {
        Ok(ReadModelDaos {
            store: Rc::new(ReadModelStore::open(path)?),
        })
    }

    /// `read_definition` を引く実装。
    #[must_use]
    pub fn definition(&self) -> DefinitionDaoImpl {
        DefinitionDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_execution` を引く実装。
    #[must_use]
    pub fn execution(&self) -> ExecutionDaoImpl {
        ExecutionDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_next_jump` を引く実装。
    #[must_use]
    pub fn jump(&self) -> JumpDaoImpl {
        JumpDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_next_jump_phase` を引く実装。
    #[must_use]
    pub fn jump_phase(&self) -> JumpPhaseDaoImpl {
        JumpPhaseDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_next_answer` を引く実装。
    #[must_use]
    pub fn next_answer(&self) -> NextAnswerDaoImpl {
        NextAnswerDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_definition_scope_phase_entry` を引く実装。
    #[must_use]
    pub fn phase_entry(&self) -> PhaseEntryDaoImpl {
        PhaseEntryDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_run_stage` を引く実装。
    #[must_use]
    pub fn run_stage(&self) -> RunStageDaoImpl {
        RunStageDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_scope_change` を引く実装。
    #[must_use]
    pub fn scope_change(&self) -> ScopeChangeDaoImpl {
        ScopeChangeDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_definition_scope` を引く実装。
    #[must_use]
    pub fn scope(&self) -> ScopeDaoImpl {
        ScopeDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_definition_scope_keyword` を引く実装。
    #[must_use]
    pub fn scope_keyword(&self) -> ScopeKeywordDaoImpl {
        ScopeKeywordDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_steering_part` を引く実装。
    #[must_use]
    pub fn steering_part(&self) -> SteeringPartDaoImpl {
        SteeringPartDaoImpl::new(Rc::clone(&self.store))
    }

    /// `read_steering_plan` を引く実装。
    #[must_use]
    pub fn steering_plan(&self) -> SteeringPlanDaoImpl {
        SteeringPlanDaoImpl::new(Rc::clone(&self.store))
    }
}
