//! `Intent` — 静的な intent (`id` + 依頼 + 解決済み計画 + 走査結果)。
//!
//! 「1 つの intent を元に実行 (`IntentExecution`) は何回でも起きる」(1 intent : n 実行 —
//! オーナー裁定 2026-08-29) という意味論において、**回数によらず変わらない側**がこの型で
//! ある。集約ではない — 遷移も判断も持たない **Always Valid の不変構造体**であり、作った
//! あとに状態が変わる経路は存在しない。
//!
//! 実行時の状態 (カーソル・checkbox・park・承認履歴…) は集約 [`IntentExecution`] が持ち、
//! 集約はこの型を**埋め込まず `IntentId` で参照する** (coding-rules/aggregate-references.md)。
//! 判断に計画が要るコマンド・クエリは、この型を `&` 参照で引数に受け取る。
//!
//! [`IntentExecution`]: super::intent_execution::IntentExecution

use std::fmt;

use serde::{Deserialize, Serialize};

use super::intent_id::IntentId;
use super::stage_display::StageDisplay;
use super::stage_entry::StageEntry;
use super::start_request::StartRequest;
use super::workspace_scan::WorkspaceScan;
use crate::workflow_definition::{
    DefinitionRevision, ExecutionKind, PhaseId, PlanAction, UnknownScope, WorkflowDefinition,
    WorkflowDefinitionId,
};

/// 静的な intent — 実行が何回起きても変わらない側 (Always Valid)。
///
/// `stages` は**この intent 向けに解決済みの計画**である。定義 (`WorkflowDefinition` =
/// 全 intent 共通のプロセス定義) を `definition_id` / `definition_revision` でピンし、
/// そこから解決した EXECUTE / SKIP 列を文書順に持つ。定義そのものは持たない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    id: IntentId,
    definition_id: WorkflowDefinitionId,
    definition_revision: DefinitionRevision,
    start_request: StartRequest,
    stages: Vec<StageEntry>,
    scan: WorkspaceScan,
}

/// `Intent` を組めない形 (材料のみ — 利用者向け文言はアダプタ層)。
///
/// initialization フェーズの扱いは BR2.2 — 状態ファイルを起こす工程そのものなので、
/// SKIP にも CONDITIONAL にもできない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentError {
    /// 定義が知らないスコープ名 (材料は定義側の `UnknownScope`)。`resolve` だけが返す。
    UnknownScope(UnknownScope),
    /// 解決済み計画が 0 件 — コンパイル済みグラフが空の場合のみ (防御的)。
    Empty,
    /// initialization フェーズのステージが SKIP に畳まれた、または先頭ステージが SKIP。
    InitializationMustExecute,
    /// initialization フェーズのステージが CONDITIONAL。
    InitializationMustBeUnconditional,
    /// ステージの表示属性 (表題・担当エージェント) が単一行でない。
    ///
    /// 表示属性は状態ファイルの bullet 行に書かれる値なので、改行が混ざると 2 行目以降が
    /// フィールドとして読めなくなる。定義側の値をそのまま信じず、計画を解決する時点で止める。
    StageDisplayNotSingleLine {
        /// 問題のあったステージ。
        stage: String,
        /// 走査順に最初に見つかった不正コードポイント。
        found: char,
    },
}

impl Intent {
    /// 識別子・定義のピン・依頼・解決済み計画・走査結果から intent を組む (基本コンストラクタ)。
    ///
    /// # Errors
    ///
    /// ステージ 0 件、initialization ステージの SKIP / CONDITIONAL、先頭ステージの SKIP を
    /// 拒否する (先頭はカーソルの初期位置なので、実効 EXECUTE でなければ実行が始められない)。
    /// スコープ名が定義にあるかは**ここでは検査しない** — 計画を解決する側の責務である。
    pub fn new(
        id: IntentId,
        definition_id: WorkflowDefinitionId,
        definition_revision: DefinitionRevision,
        start_request: StartRequest,
        stages: Vec<StageEntry>,
        scan: WorkspaceScan,
    ) -> Result<Intent, IntentError> {
        match stages.first() {
            None => return Err(IntentError::Empty),
            Some(first) if first.plan_action() != PlanAction::Execute => {
                return Err(IntentError::InitializationMustExecute);
            }
            Some(_) => {}
        }
        for entry in &stages {
            if entry.phase() != PhaseId::Initialization {
                continue;
            }
            if entry.plan_action() != PlanAction::Execute {
                return Err(IntentError::InitializationMustExecute);
            }
            if entry.is_conditional() {
                return Err(IntentError::InitializationMustBeUnconditional);
            }
        }
        Ok(Intent {
            id,
            definition_id,
            definition_revision,
            start_request,
            stages,
            scan,
        })
    }

    /// 定義と呼出側の要求から解決済み計画を組み立てて intent を作る (補助コンストラクタ)。
    ///
    /// `definition.id()` / `definition.revision()` は**無条件に控える** — 比較対象となる既存状態が
    /// 無い静的コンストラクタなので検査はしない (BR2.6)。以後の定義照合は実行側の
    /// `next_decision` が行う。組み立てた計画は基本コンストラクタ [`Intent::new`] に渡すので、
    /// 不変条件の検査点は 1 か所のままである。
    ///
    /// # Errors
    ///
    /// 未知スコープ (`UnknownScope`)、表示属性が単一行でない (`StageDisplayNotSingleLine`)、
    /// および [`Intent::new`] が拒む形をそのまま返す。
    pub fn resolve(
        id: IntentId,
        definition: &WorkflowDefinition,
        start_request: StartRequest,
        scan: WorkspaceScan,
    ) -> Result<Intent, IntentError> {
        let scope = start_request.scope();
        if !definition.is_valid_scope(scope) {
            let valid = definition
                .valid_scopes()
                .into_iter()
                .map(str::to_string)
                .collect();
            return Err(IntentError::UnknownScope(UnknownScope::new(scope, valid)));
        }
        let nodes = definition.graph().nodes();
        let mut stages = Vec::new();
        for (index, (slug, phase, action)) in
            definition.stages_in_scope(scope).into_iter().enumerate()
        {
            // `stages_in_scope` は execution も表示属性も返さないので、同じ文書順のノード列から
            // 索引一致で拾う (BR2.2)。グリッド列が無いステージは `None → SKIP` に畳む。
            let node = nodes.get(index);
            let conditional =
                node.is_some_and(|node| node.execution() == ExecutionKind::Conditional);
            // 表示属性は**計画を解決するこの時点で**焼き込む (オーナー裁定 2026-08-29)。
            // 投影は後から定義を引かないので、再構成しても当時と同じバイトになる (NFR3)。
            let display = match node {
                Some(node) => {
                    StageDisplay::new(node.number().clone(), node.name(), node.lead_agent())
                        .map_err(|unsafe_char| IntentError::StageDisplayNotSingleLine {
                            stage: slug.as_str().to_string(),
                            found: unsafe_char.to_char(),
                        })?
                }
                // 索引一致が崩れるのはグラフが壊れている場合だけ (防御的)。
                None => return Err(IntentError::Empty),
            };
            stages.push(StageEntry::new(
                slug.clone(),
                phase,
                action.unwrap_or(PlanAction::Skip),
                conditional,
                display,
            ));
        }
        Intent::new(
            id,
            definition.id().clone(),
            definition.revision().clone(),
            start_request,
            stages,
            scan,
        )
    }

    /// この intent の識別子 (以後不変。`intents.json` の uuid にあたる)。
    #[must_use]
    pub const fn id(&self) -> &IntentId {
        &self.id
    }

    /// 参照した定義の系譜 ID (BR2.6)。
    #[must_use]
    pub const fn definition_id(&self) -> &WorkflowDefinitionId {
        &self.definition_id
    }

    /// 参照した定義の内容版 (来歴 — 差が出ても Err にはしない)。
    #[must_use]
    pub const fn definition_revision(&self) -> &DefinitionRevision {
        &self.definition_revision
    }

    /// 選択されたスコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        self.start_request.scope()
    }

    /// 人間の要求 (逐語保持)。
    #[must_use]
    pub fn request(&self) -> &str {
        self.start_request.request()
    }

    /// 呼出側が解決した depth (`None` = 指定なし)。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.start_request.depth()
    }

    /// 呼出側が解決した test strategy (`None` = 指定なし)。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.start_request.test_strategy()
    }

    /// 文書順の解決済み計画。
    #[must_use]
    pub fn stages(&self) -> &[StageEntry] {
        &self.stages
    }

    /// 解決済み計画のステージ数 (1 以上 — 空は構築できない)。
    #[must_use]
    pub const fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// ワークスペース走査の結果 (状態ファイルの `Project Information` に写る)。
    #[must_use]
    pub const fn scan(&self) -> &WorkspaceScan {
        &self.scan
    }
}

impl fmt::Display for IntentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentError::UnknownScope(scope) => write!(
                f,
                "unknown scope: {} (valid: {})",
                scope.scope(),
                scope.valid_scopes().join(", ")
            ),
            IntentError::Empty => f.write_str("empty stage list"),
            IntentError::InitializationMustExecute => {
                f.write_str("initialization stage is not EXECUTE")
            }
            IntentError::InitializationMustBeUnconditional => {
                f.write_str("initialization stage is CONDITIONAL")
            }
            IntentError::StageDisplayNotSingleLine { stage, found } => write!(
                f,
                "stage display is not single line: stage {stage}, found U+{:04X}",
                *found as u32
            ),
        }
    }
}

impl std::error::Error for IntentError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{IntentId, StageDisplay, StageEntry, WorkspaceScan};
    use crate::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, ExecutionKind, PhaseId, PlanAction, ScopeGrid,
        ScopeMetadata, StageGraph, StageMode, StageNodeBuilder, StageNumber, StageSlug,
        WorkflowDefinition, WorkflowDefinitionId,
    };
    use std::collections::BTreeMap;

    const SAMPLE: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn id() -> IntentId {
        IntentId::parse(SAMPLE).unwrap()
    }

    fn def_id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").unwrap()
    }

    fn revision() -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap()
    }

    fn scan() -> WorkspaceScan {
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .unwrap()
    }

    fn request() -> StartRequest {
        StartRequest::new("classic", "build the thing")
            .with_depth("standard")
            .with_test_strategy("balanced")
    }

    fn entry(name: &str, number: &str, phase: PhaseId, action: PlanAction) -> StageEntry {
        StageEntry::new(
            StageSlug::parse(name).unwrap(),
            phase,
            action,
            false,
            StageDisplay::new(StageNumber::parse(number).unwrap(), "Stage", "orchestrator")
                .unwrap(),
        )
    }

    fn stages() -> Vec<StageEntry> {
        vec![
            entry(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                PlanAction::Execute,
            ),
            entry(
                "intent-capture",
                "1.1",
                PhaseId::Ideation,
                PlanAction::Execute,
            ),
            entry(
                "market-research",
                "1.2",
                PhaseId::Ideation,
                PlanAction::Skip,
            ),
        ]
    }

    fn intent() -> Intent {
        Intent::new(id(), def_id(), revision(), request(), stages(), scan()).unwrap()
    }

    #[test]
    fn the_parts_are_reported_back_verbatim() {
        let intent = intent();
        assert_eq!(intent.id(), &id());
        assert_eq!(intent.definition_id(), &def_id());
        assert_eq!(intent.definition_revision(), &revision());
        assert_eq!(intent.scope(), "classic");
        assert_eq!(intent.request(), "build the thing");
        assert_eq!(intent.depth(), Some("standard"));
        assert_eq!(intent.test_strategy(), Some("balanced"));
        assert_eq!(intent.stages(), stages().as_slice());
        assert_eq!(intent.scan(), &scan());
    }

    #[test]
    fn the_stage_count_is_the_length_of_the_resolved_plan() {
        assert_eq!(intent().stage_count(), 3);
    }

    #[test]
    fn an_empty_plan_is_refused() {
        assert_eq!(
            Intent::new(id(), def_id(), revision(), request(), Vec::new(), scan()),
            Err(IntentError::Empty)
        );
    }

    #[test]
    fn a_first_stage_that_is_not_execute_is_refused() {
        // 先頭はカーソルの初期位置なので、実効 EXECUTE でなければ cursor_in_scope を破る。
        let stages = vec![
            entry(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                PlanAction::Skip,
            ),
            entry(
                "intent-capture",
                "1.1",
                PhaseId::Ideation,
                PlanAction::Execute,
            ),
        ];
        assert_eq!(
            Intent::new(id(), def_id(), revision(), request(), stages, scan()),
            Err(IntentError::InitializationMustExecute)
        );
    }

    #[test]
    fn an_initialization_stage_folded_to_skip_is_refused() {
        let stages = vec![
            entry(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                PlanAction::Execute,
            ),
            entry(
                "state-detect",
                "0.2",
                PhaseId::Initialization,
                PlanAction::Skip,
            ),
        ];
        assert_eq!(
            Intent::new(id(), def_id(), revision(), request(), stages, scan()),
            Err(IntentError::InitializationMustExecute)
        );
    }

    #[test]
    fn a_conditional_initialization_stage_is_refused() {
        let conditional = StageEntry::new(
            StageSlug::parse("state-detect").unwrap(),
            PhaseId::Initialization,
            PlanAction::Execute,
            true,
            StageDisplay::new(StageNumber::parse("0.2").unwrap(), "Stage", "orchestrator").unwrap(),
        );
        let stages = vec![
            entry(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                PlanAction::Execute,
            ),
            conditional,
        ];
        assert_eq!(
            Intent::new(id(), def_id(), revision(), request(), stages, scan()),
            Err(IntentError::InitializationMustBeUnconditional)
        );
    }

    #[test]
    fn a_conditional_stage_outside_initialization_is_accepted() {
        let conditional = StageEntry::new(
            StageSlug::parse("market-research").unwrap(),
            PhaseId::Ideation,
            PlanAction::Execute,
            true,
            StageDisplay::new(StageNumber::parse("1.2").unwrap(), "Stage", "orchestrator").unwrap(),
        );
        let stages = vec![
            entry(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                PlanAction::Execute,
            ),
            conditional,
        ];
        assert!(Intent::new(id(), def_id(), revision(), request(), stages, scan()).is_ok());
    }

    #[test]
    fn a_stage_whose_display_is_not_single_line_is_refused_when_the_plan_is_resolved() {
        // 表示属性は状態ファイルの bullet 行に書かれる値なので、改行が混ざる定義は計画を
        // 解決するこの時点で止める (定義側の値をそのまま信じない)。
        let node = StageNodeBuilder::new(
            StageSlug::parse("state-init").unwrap(),
            StageNumber::parse("0.1").unwrap(),
            "Broken\nName".to_string(),
            PhaseId::Initialization,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .scopes(vec!["classic".to_string()])
        .build();
        let grid = ScopeGrid::new(
            [(
                "classic".to_string(),
                [(StageSlug::parse("state-init").unwrap(), PlanAction::Execute)]
                    .into_iter()
                    .collect(),
            )]
            .into_iter()
            .collect(),
        );
        let scopes: BTreeMap<String, ScopeMetadata> = [(
            "classic".to_string(),
            ScopeMetadata::new("classic").unwrap(),
        )]
        .into_iter()
        .collect();
        let definition = WorkflowDefinition::new(
            def_id(),
            revision(),
            StageGraph::new(vec![node]).unwrap(),
            grid,
            scopes,
        );
        assert_eq!(
            Intent::resolve(id(), &definition, request(), scan()),
            Err(IntentError::StageDisplayNotSingleLine {
                stage: "state-init".to_string(),
                found: '\n',
            })
        );
    }

    #[test]
    fn intents_built_from_the_same_parts_compare_equal() {
        assert_eq!(intent(), intent());
        let other = Intent::new(
            IntentId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").unwrap(),
            def_id(),
            revision(),
            request(),
            stages(),
            scan(),
        )
        .unwrap();
        assert_ne!(intent(), other);
    }

    #[test]
    fn the_intent_round_trips_through_serde() {
        let intent = intent();
        #[allow(
            clippy::disallowed_methods,
            reason = "契約 JSON ではなく serde 境界そのものの往復確認 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&intent).unwrap();
        assert_eq!(serde_json::from_str::<Intent>(&json).unwrap(), intent);
    }

    #[test]
    fn the_rejection_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(IntentError::Empty);
        assert_eq!(err.to_string(), "empty stage list");
    }

    #[test]
    fn rejections_compare_by_value() {
        assert_eq!(IntentError::Empty, IntentError::Empty);
        assert_ne!(IntentError::Empty, IntentError::InitializationMustExecute);
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(IntentError::Empty.to_string(), "empty stage list");
        assert_eq!(
            IntentError::InitializationMustExecute.to_string(),
            "initialization stage is not EXECUTE"
        );
        assert_eq!(
            IntentError::InitializationMustBeUnconditional.to_string(),
            "initialization stage is CONDITIONAL"
        );
        assert_eq!(
            IntentError::UnknownScope(UnknownScope::new(
                "nope",
                vec!["classic".to_string(), "mvp".to_string()]
            ))
            .to_string(),
            "unknown scope: nope (valid: classic, mvp)"
        );
        assert_eq!(
            IntentError::StageDisplayNotSingleLine {
                stage: "state-init".to_string(),
                found: '\n',
            }
            .to_string(),
            "stage display is not single line: stage state-init, found U+000A"
        );
    }
}
