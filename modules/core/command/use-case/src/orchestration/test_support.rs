//! ユースケース層のテスト用の土台 — ポートのテストダブルと合成計画のフィクスチャ。
//!
//! # なぜアダプタ層の実装を借りないのか
//!
//! 借りられないからである。`core-command-use-case` の `Cargo.toml` に
//! `core-command-interface-adapter` を書いた瞬間に DIP のクレート分離強制が壊れ、依存も
//! 循環する (`coding-rules/use-case-rules.md` §1)。dev-dependency でも同じなので、
//! ユースケースのテストが使うポート実装は**本クレート内の `#[cfg(test)]` に置く**。
//!
//! ここに置くのは 1 つだけである — ポートのテストも `CommitVerdictUseCase` のテストも同じ
//! [`InMemoryWorkflowExecutionRepository`] を通す (`coding-rules/no-backward-compatibility.md`
//! — 同じ役割の口を 2 つ並立させない)。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentId, StageDisplay, StageEntry, StartRequest, WorkflowExecution, WorkflowExecutionEvent,
    WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
    WorkflowDefinitionId,
};

use super::rehydrated_workflow_execution::RehydratedWorkflowExecution;
use super::repository_error::RepositoryError;
use super::workflow_execution_repository::WorkflowExecutionRepository;

/// フィクスチャの集約識別子 (UUIDv7)。
pub(crate) const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

/// イベントの発生時刻 — 集約は値を素通しするので固定値でよい (NFR3.1)。
pub(crate) fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
        .expect("固定の ISO 8601 UTC")
        .with_timezone(&Utc)
}

/// フィクスチャの集約識別子。
pub(crate) fn intent() -> IntentId {
    IntentId::parse(INTENT).expect("フィクスチャの IntentId は UUIDv7")
}

/// ストアに居ない集約の識別子 (`NotFound` を見るテスト用)。
pub(crate) fn absent_intent() -> IntentId {
    IntentId::parse("018f3b2c-4d5e-7f60-8abc-def012345678")
        .expect("フィクスチャの IntentId は UUIDv7")
}

/// 合成計画の slug (文書順の位置がそのまま名前になる)。
pub(crate) fn slug(index: usize) -> StageSlug {
    StageSlug::parse(&format!("stage-{index}")).expect("フィクスチャの slug は文法内")
}

fn display(index: usize, phase: PhaseId) -> StageDisplay {
    StageDisplay::new(
        StageNumber::parse(&format!("{}.{}", phase.index(), index + 1))
            .expect("フィクスチャのステージ番号は文法内"),
        "Stage",
        "orchestrator",
    )
    .expect("単一行")
}

/// 合成計画の走査結果 (投影は見ないので固定値でよい)。
pub(crate) fn scan() -> WorkspaceScan {
    WorkspaceScan::new(
        BrownfieldGreenfield::Greenfield,
        "Unknown",
        "Unknown",
        "Unknown",
    )
    .expect("単一行")
}

/// フェーズと実効プラン・CONDITIONAL を名指しした合成計画で開始する。
pub(crate) fn start_from_plan(
    plan: &[(PhaseId, PlanAction, bool)],
) -> (WorkflowExecution, WorkflowExecutionEvent) {
    let stages = plan
        .iter()
        .enumerate()
        .map(|(index, (phase, action, conditional))| {
            StageEntry::new(
                slug(index),
                *phase,
                *action,
                *conditional,
                display(index, *phase),
            )
        })
        .collect();
    WorkflowExecution::start_from_plan_unchecked(
        intent(),
        WorkflowDefinitionId::parse("claude").expect("フィクスチャの定義 id"),
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64)))
            .expect("フィクスチャの定義 revision"),
        &StartRequest::new("classic", "report use case"),
        stages,
        scan(),
        at(),
    )
    .expect("合成計画は start の前提を満たす")
}

/// 索引 0 = initialization (非ゲート)、索引 1 以降 = inception (ゲート付き) の合成計画。
pub(crate) fn genesis(stage_count: usize) -> (WorkflowExecution, WorkflowExecutionEvent) {
    let plan: Vec<(PhaseId, PlanAction, bool)> = (0..stage_count)
        .map(|index| {
            let phase = if index == 0 {
                PhaseId::Initialization
            } else {
                PhaseId::Inception
            };
            (phase, PlanAction::Execute, false)
        })
        .collect();
    start_from_plan(&plan)
}

/// [`WorkflowExecutionRepository`] のインメモリ実装。
///
/// 楽観 version は本家の実測どおり「新規作成は 0、1 件書くごとに 1 つ進む」で採番する。
/// レシーバは CQS どおり再構成が `&self`、永続化が `&mut self` である — 内部可変性で
/// `&self` に見せかけない (`coding-rules/interior-mutability.md`)。
///
/// # 応答をスクリプトできる
///
/// `Conflict` は「読んでから書くまでの間に別の書き手が入った」ときにしか起きないので、
/// 単一スレッドのテストからは自然には起こせない。そこで**割り込む書込の回数**を台本として
/// 持たせる ([`InMemoryWorkflowExecutionRepository::holding_behind_concurrent_writes`])。
/// 台本が残っている間、`store` はストアの版だけを 1 つ進めて `Conflict` を返す。
/// 割り込んだ相手が書いた**内容**までは模さない — 版の進行だけが再試行の観測に要る材料である。
#[derive(Debug)]
pub(crate) struct InMemoryWorkflowExecutionRepository {
    stored: Option<WorkflowExecution>,
    version: usize,
    interrupting_writes: usize,
    store_attempts: usize,
    committed: Vec<WorkflowExecutionEvent>,
}

impl InMemoryWorkflowExecutionRepository {
    /// 基本コンストラクタ — 構築経路はここ 1 本に集約する
    /// (`coding-rules/factory-naming.md`)。
    fn new(
        stored: Option<WorkflowExecution>,
        version: usize,
        interrupting_writes: usize,
    ) -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository {
            stored,
            version,
            interrupting_writes,
            store_attempts: 0,
            committed: Vec::new(),
        }
    }

    /// 何も入っていないストア — `find_by_id` は `NotFound` を返す。
    pub(crate) fn empty() -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository::new(None, 0, 0)
    }

    /// 集約 1 つを版 `version` で保持するストア。
    pub(crate) fn holding(
        aggregate: WorkflowExecution,
        version: usize,
    ) -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository::new(Some(aggregate), version, 0)
    }

    /// 最初の `writes` 回の `store` に、別の書き手の書込が割り込むストア。
    ///
    /// 割り込みが起きた回は版だけが 1 つ進み、提示された版は古くなるので `Conflict` になる。
    /// 台本を使い切ると通常の楽観判定へ戻るので、**再構成からやり直した呼出だけが**
    /// 新しい版を提示でき、書込に成功する。
    pub(crate) fn holding_behind_concurrent_writes(
        aggregate: WorkflowExecution,
        version: usize,
        writes: usize,
    ) -> InMemoryWorkflowExecutionRepository {
        InMemoryWorkflowExecutionRepository::new(Some(aggregate), version, writes)
    }

    /// このストアが受理したイベント列 (コミットの有無を見るテスト用)。
    pub(crate) fn committed(&self) -> &[WorkflowExecutionEvent] {
        &self.committed
    }

    /// `store` が呼ばれた回数 (再試行が 1 回だけであることを見るテスト用)。
    pub(crate) const fn store_attempts(&self) -> usize {
        self.store_attempts
    }

    /// ストアが採番している現在の版。
    pub(crate) const fn version(&self) -> usize {
        self.version
    }
}

impl WorkflowExecutionRepository for InMemoryWorkflowExecutionRepository {
    async fn find_by_id(
        &self,
        id: &IntentId,
    ) -> Result<RehydratedWorkflowExecution, RepositoryError> {
        self.stored
            .clone()
            .map(|aggregate| RehydratedWorkflowExecution::new(aggregate, self.version))
            .ok_or_else(|| RepositoryError::NotFound {
                intent_id: id.clone(),
            })
    }

    async fn store(
        &mut self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
        expected_version: usize,
    ) -> Result<(), RepositoryError> {
        self.store_attempts += 1;
        if self.interrupting_writes > 0 {
            // 別の書き手が先に書いた — ストアの版だけが進み、提示された版が古くなる。
            self.interrupting_writes -= 1;
            self.version += 1;
            return Err(RepositoryError::Conflict {
                expected: expected_version,
                actual: self.version,
            });
        }
        if expected_version != self.version {
            return Err(RepositoryError::Conflict {
                expected: expected_version,
                actual: self.version,
            });
        }
        self.version = expected_version + 1;
        self.stored = Some(aggregate.clone());
        self.committed.push(event.clone());
        Ok(())
    }
}
