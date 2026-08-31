//! ラダーのテストが共有するフィクスチャ (テスト専用)。
//!
//! 定義ビューと実行状態ビューを 1 か所で組み、`scope_resolution` / `next_use_case` /
//! `continue_use_case` の各テストが同じ形を見るようにする。
//!
//! DAO ポートのフェイクもここに置く。use-case 層はアダプタ層に依存できない (層 = クレートで
//! 物理強制 — `coding-rules/use-case-rules.md` §1) ので、アダプタの `InMemory*Dao` は
//! 使えない。フェイクは**読み終えた値を握って返すだけ**で、I/O も状態遷移も持たない。

use std::collections::BTreeMap;

use crate::orchestration::{
    CheckboxState, DefinitionIdView, DefinitionRevisionView, DefinitionView, ExecutionKindView,
    ExecutionStateDao, ExecutionStateReadError, ExecutionStateView, ExecutionStatus, MemoryRules,
    MemoryRulesDao, MemoryRulesReadError, PhaseView, PlanActionView, ScopeGridView,
    ScopeMetadataView, ScopeSlugView, StageGraphView, StageModeView, StageNumberView,
    StageProgressView, StageSlugView, StageViewBuilder, WorkflowDefinitionDao,
    WorkflowDefinitionReadError,
};

/// 定義リードモデルの読取結果を握るフェイク DAO。
pub(super) struct FakeDefinitionDao(Result<DefinitionView, WorkflowDefinitionReadError>);

impl FakeDefinitionDao {
    /// 読める定義。
    pub(super) fn holding(view: DefinitionView) -> FakeDefinitionDao {
        FakeDefinitionDao(Ok(view))
    }

    /// 読取に失敗する定義。
    pub(super) const fn failing(error: WorkflowDefinitionReadError) -> FakeDefinitionDao {
        FakeDefinitionDao(Err(error))
    }
}

impl WorkflowDefinitionDao for FakeDefinitionDao {
    fn find(&self) -> Result<DefinitionView, WorkflowDefinitionReadError> {
        self.0.clone()
    }
}

/// 実行状態リードモデルの読取結果を握るフェイク DAO。
pub(super) struct FakeStateDao(Result<Option<ExecutionStateView>, ExecutionStateReadError>);

impl FakeStateDao {
    /// 稼働中のリードモデルがある。
    pub(super) const fn holding(view: ExecutionStateView) -> FakeStateDao {
        FakeStateDao(Ok(Some(view)))
    }

    /// リードモデルが無い (誕生分岐へ — 正常な観測)。
    pub(super) const fn absent() -> FakeStateDao {
        FakeStateDao(Ok(None))
    }

    /// リードモデルが在るのに読めない。
    pub(super) const fn failing(error: ExecutionStateReadError) -> FakeStateDao {
        FakeStateDao(Err(error))
    }
}

impl ExecutionStateDao for FakeStateDao {
    fn find(&self) -> Result<Option<ExecutionStateView>, ExecutionStateReadError> {
        self.0.clone()
    }
}

/// memory 層ルール束の読取結果を握るフェイク DAO。
pub(super) struct FakeRulesDao(Result<MemoryRules, MemoryRulesReadError>);

impl FakeRulesDao {
    /// 読めたルール束 (空も正常 — bare run-stage)。
    pub(super) const fn holding(rules: MemoryRules) -> FakeRulesDao {
        FakeRulesDao(Ok(rules))
    }

    /// ルール未整備 (空束)。
    pub(super) fn empty() -> FakeRulesDao {
        FakeRulesDao(Ok(MemoryRules::default()))
    }

    /// 必須ルールファイルが在るのに読めない。
    pub(super) fn unreadable(path: &str, cause: &str) -> FakeRulesDao {
        FakeRulesDao(Err(MemoryRulesReadError::new(
            path.to_string(),
            cause.to_string(),
        )))
    }
}

impl MemoryRulesDao for FakeRulesDao {
    fn find(&self) -> Result<MemoryRules, MemoryRulesReadError> {
        self.0.clone()
    }
}

/// `stage-<index>` の slug。
pub(super) fn slug(index: usize) -> StageSlugView {
    StageSlugView::parse(&format!("stage-{index}")).expect("固定の slug")
}

/// 索引 0 = initialization、以降 = inception のフェーズ割り当て。
pub(super) const fn phase_of(index: usize) -> PhaseView {
    if index == 0 {
        PhaseView::Initialization
    } else {
        PhaseView::Inception
    }
}

/// 実行状態ビューの合成計画に一致する定義ビュー。
/// scope は `classic` (推論キーワードなし) と `bugfix` (キーワード `fix`)。
pub(super) fn definition(stage_count: usize) -> DefinitionView {
    let nodes = (0..stage_count)
        .map(|index| {
            StageViewBuilder::new(
                slug(index),
                StageNumberView::parse(&format!("{index}.1")).expect("固定のステージ番号"),
                format!("Stage {index}"),
                phase_of(index),
                ExecutionKindView::Always,
                StageModeView::Inline,
            )
            .with_lead_agent("orchestrator".to_string())
            .with_scopes(vec!["classic".to_string(), "bugfix".to_string()])
            .build()
        })
        .collect::<Vec<_>>();
    let column = || {
        (0..stage_count)
            .map(|index| (slug(index), PlanActionView::Execute))
            .collect::<BTreeMap<_, _>>()
    };
    let grid = ScopeGridView::new(
        [
            ("classic".to_string(), column()),
            ("bugfix".to_string(), column()),
        ]
        .into_iter()
        .collect(),
    );
    let scopes: BTreeMap<String, ScopeMetadataView> = [
        (
            "classic".to_string(),
            ScopeMetadataView::new("classic").expect("固定の scope 名"),
        ),
        (
            "bugfix".to_string(),
            ScopeMetadataView::new("bugfix")
                .expect("固定の scope 名")
                .with_keywords(vec!["fix".to_string()]),
        ),
    ]
    .into_iter()
    .collect();
    DefinitionView::new(
        DefinitionIdView::parse("claude").expect("固定の定義 id"),
        DefinitionRevisionView::parse(&format!("sha256:{}", "0".repeat(64))).expect("固定の内容版"),
        StageGraphView::new(nodes).expect("固定のグラフ"),
        grid,
        scopes,
    )
}

/// genesis 直後に相当する実行状態ビュー — 索引 0 が in-progress、以降は pending・全 EXECUTE。
pub(super) fn genesis_state(stage_count: usize) -> ExecutionStateView {
    let markers = (0..stage_count)
        .map(|index| {
            if index == 0 {
                CheckboxState::InProgress
            } else {
                CheckboxState::Pending
            }
        })
        .collect::<Vec<_>>();
    state(
        stage_count,
        0,
        &markers,
        &vec![PlanActionView::Execute; stage_count],
    )
}

/// カーソル位置・マーカー列・実効プラン列を指定して実行状態ビューを組む (Running / 非 park)。
pub(super) fn state(
    stage_count: usize,
    cursor: usize,
    markers: &[CheckboxState],
    plans: &[PlanActionView],
) -> ExecutionStateView {
    parked_state(stage_count, cursor, None, markers, plans)
}

/// park マーカー付き (あるいは無し) の実行状態ビュー。
pub(super) fn parked_state(
    stage_count: usize,
    cursor: usize,
    parked_at: Option<usize>,
    markers: &[CheckboxState],
    plans: &[PlanActionView],
) -> ExecutionStateView {
    let rows = (0..stage_count)
        .map(|index| {
            StageProgressView::new(
                slug(index),
                phase_of(index),
                markers
                    .get(index)
                    .copied()
                    .unwrap_or(CheckboxState::Pending),
                plans.get(index).copied().unwrap_or(PlanActionView::Execute),
            )
        })
        .collect();
    let cursor_slug = format!("stage-{cursor}");
    let parked_slug = parked_at.map(|index| format!("stage-{index}"));
    ExecutionStateView::new(
        ScopeSlugView::parse("classic").expect("固定の scope"),
        ExecutionStatus::Running,
        &cursor_slug,
        parked_slug.as_deref(),
        "2026-08-29T16:36:24Z",
        rows,
    )
    .expect("固定のリードモデル")
}

/// `Status: Completed` の実行状態ビュー。
pub(super) fn completed_state(stage_count: usize) -> ExecutionStateView {
    let rows = (0..stage_count)
        .map(|index| {
            StageProgressView::new(
                slug(index),
                phase_of(index),
                CheckboxState::Completed,
                PlanActionView::Execute,
            )
        })
        .collect();
    ExecutionStateView::new(
        ScopeSlugView::parse("classic").expect("固定の scope"),
        ExecutionStatus::Completed,
        &format!("stage-{}", stage_count.saturating_sub(1)),
        None,
        "2026-08-29T16:36:24Z",
        rows,
    )
    .expect("固定のリードモデル")
}
