//! `WorkflowExecutionEvent` — 封筒 + 12 変種のペイロード (C5、entities.md)。
//!
//! 変種はコマンドと 1:1 (BR1.1 / BR2.4)。ステージ参照はすべて `StageSlug` で、投影側 (U4) が
//! 索引表を要さない自己記述形になっている。イベントは構築後 immutable で、材料はアクセサで
//! 公開する。
//!
//! 封筒は本家 event-store-adapter-rs の [`Event`] を**直接実装する** (ADR-010 Conformist —
//! 腐敗防止層は置かない)。`id` / `aggregate_id` / `seq_nr` / `occurred_at` / `is_created` は
//! 本家が所有する契約であり、綴りも型もそのまま受け入れる。serde 境界も本家の要求で、
//! `Serialize` / `Deserialize` は表現の写しにすぎない (材料の変更経路にはならない)。

use chrono::{DateTime, Utc};
use event_store_adapter_rs::types::Event;
use serde::{Deserialize, Serialize};

use super::autonomy_mode::AutonomyMode;
use super::intent_id::IntentId;
use super::jump_direction::JumpDirection;
use super::phase_boundary::PhaseBoundary;
use super::stage_entry::StageEntry;
use super::start_request::StartRequest;
use super::workflow_execution_event_id::WorkflowExecutionEventId;
use crate::workflow_definition::{DefinitionRevision, StageSlug, WorkflowDefinitionId};

/// イベント封筒 (C5 envelope)。
///
/// 集約識別子と順序番号は [`WorkflowExecutionEventId`] が 1 か所で持つ — 封筒に別立ての
/// 写しを置くと 2 つの正本ができるためである。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowExecutionEvent {
    id: WorkflowExecutionEventId,
    schema_version: u32,
    occurred_at: DateTime<Utc>,
    payload: WorkflowExecutionEventPayload,
}

impl WorkflowExecutionEvent {
    /// C5 が全イベントに予約するスキーマ版 (追加フィールドは消費側が無視する additive-safe)。
    pub const SCHEMA_VERSION: u32 = 1;

    /// 封筒を組む。`occurred_at` は **呼出側が時計から渡す** (集約は時計を持たない — NFR3.1)。
    /// `schema_version` は常に [`Self::SCHEMA_VERSION`]、`id` は集約識別子と `seq_nr` から
    /// 決定的に採番する。
    #[must_use]
    pub const fn new(
        intent_id: IntentId,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        payload: WorkflowExecutionEventPayload,
    ) -> WorkflowExecutionEvent {
        WorkflowExecutionEvent {
            id: WorkflowExecutionEventId::new(intent_id, seq_nr),
            schema_version: WorkflowExecutionEvent::SCHEMA_VERSION,
            occurred_at,
            payload,
        }
    }

    /// C5 の予約フィールド。
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// 変種ごとのペイロード。
    #[must_use]
    pub const fn payload(&self) -> &WorkflowExecutionEventPayload {
        &self.payload
    }
}

/// 本家 event-store-adapter-rs のドメインイベント契約 (ADR-010 Conformist)。
impl Event for WorkflowExecutionEvent {
    type AggregateID = IntentId;
    type ID = WorkflowExecutionEventId;

    fn id(&self) -> &WorkflowExecutionEventId {
        &self.id
    }

    fn aggregate_id(&self) -> &IntentId {
        self.id.intent_id()
    }

    fn seq_nr(&self) -> usize {
        self.id.seq_nr()
    }

    fn occurred_at(&self) -> &DateTime<Utc> {
        &self.occurred_at
    }

    /// genesis は `Started` だけである (BR2.2 — 既存の集約には適用できない)。
    fn is_created(&self) -> bool {
        matches!(self.payload, WorkflowExecutionEventPayload::Started(_))
    }
}

/// 12 変種のペイロード (C5 の 11 + `StageCompleted`)。
///
/// `#[non_exhaustive]` は**付けない** — 変種の追加は C5 の改訂を伴う設計事項であり、消費側の
/// 網羅 match が落ちること自体が検出手段である (NFR1.3)。`Unparked` は C5 が `payload: {}` と
/// するので専用のペイロード型を持たない単位変種にした。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowExecutionEventPayload {
    /// 実行の開始 (解決済み計画を自己完結で持つ — BR2.2)。
    Started(Started),
    /// 非ゲート (initialization フェーズ) ステージの完了。
    StageCompleted(StageCompleted),
    /// 承認ゲートの開放。
    GateOpened(GateOpened),
    /// 承認ゲートの通過。
    GateApproved(GateApproved),
    /// 承認ゲートでの差し戻し。
    GateRejected(GateRejected),
    /// 差し戻し後のゲート再入。
    StageRevised(StageRevised),
    /// ステージの読み飛ばし。
    StageSkipped(StageSkipped),
    /// カーソルの移動 (forward / backward / redo)。
    Jumped(Jumped),
    /// park マーカーの設置。
    Parked(Parked),
    /// park マーカーの除去 (位置は `parked_at` から復元されるので材料なし)。
    Unparked,
    /// 実効プランの再形成 (オーバレイの反転)。
    Recomposed(Recomposed),
    /// 自律モードの設定。
    AutonomyModeSet(AutonomyModeSet),
}

/// `Started` のペイロード — リプレイが `WorkflowDefinition` を要さない自己完結データ (BR2.2)。
///
/// `scope` / `request` / `depth` / `test_strategy` は呼出側が解決して渡した [`StartRequest`] を
/// C5 の平坦なレコードとして展開したもの。`depth` / `test_strategy` は集約状態にはならず
/// (17 属性は不変)、U4 が `Scope Configuration` を描くためだけにイベントへ載る。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Started {
    definition_id: WorkflowDefinitionId,
    definition_revision: DefinitionRevision,
    scope: String,
    request: String,
    depth: Option<String>,
    test_strategy: Option<String>,
    stages: Vec<StageEntry>,
}

impl Started {
    /// 参照した定義の ID / 内容版と、呼出側の要求・解決済み計画を束ねる。
    #[must_use]
    pub fn new(
        definition_id: WorkflowDefinitionId,
        definition_revision: DefinitionRevision,
        request: &StartRequest,
        stages: Vec<StageEntry>,
    ) -> Started {
        Started {
            definition_id,
            definition_revision,
            scope: request.scope().to_string(),
            request: request.request().to_string(),
            depth: request.depth().map(str::to_string),
            test_strategy: request.test_strategy().map(str::to_string),
            stages,
        }
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
        &self.scope
    }

    /// 人間の要求 (逐語保持)。
    #[must_use]
    pub fn request(&self) -> &str {
        &self.request
    }

    /// 呼出側が解決した depth (`None` = 指定なし)。集約は素通しするだけ。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    /// 呼出側が解決した test strategy (`None` = 指定なし)。集約は素通しするだけ。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }

    /// 文書順の全ステージ (解決済み計画)。
    #[must_use]
    pub fn stages(&self) -> &[StageEntry] {
        &self.stages
    }
}

/// `StageCompleted` のペイロード。`next_stage` が `None` ならワークフロー完了。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageCompleted {
    stage: StageSlug,
    next_stage: Option<StageSlug>,
}

impl StageCompleted {
    /// 完了したステージと次に開くステージ。
    #[must_use]
    pub const fn new(stage: StageSlug, next_stage: Option<StageSlug>) -> StageCompleted {
        StageCompleted { stage, next_stage }
    }

    /// 完了したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 次に開くステージ (`None` = ワークフロー完了)。
    #[must_use]
    pub const fn next_stage(&self) -> Option<&StageSlug> {
        self.next_stage.as_ref()
    }
}

/// `GateOpened` のペイロード。`artifacts` は呼出側が渡す投影材料 (C5)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateOpened {
    stage: StageSlug,
    artifacts: Vec<String>,
}

impl GateOpened {
    /// ゲートを開いたステージと、レビュー対象の成果物パス列。
    #[must_use]
    pub const fn new(stage: StageSlug, artifacts: Vec<String>) -> GateOpened {
        GateOpened { stage, artifacts }
    }

    /// ゲートを開いたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// レビュー対象の成果物パス列 (集約は検証せず載せるだけ)。
    #[must_use]
    pub fn artifacts(&self) -> &[String] {
        &self.artifacts
    }
}

/// `GateApproved` のペイロード。`phase_boundary` は呼出側が導出して渡す投影材料 (C5)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateApproved {
    stage: StageSlug,
    user_input: Option<String>,
    next_stage: Option<StageSlug>,
    phase_boundary: Option<PhaseBoundary>,
}

impl GateApproved {
    /// 承認されたステージと、承認時の人間入力・次ステージ・フェーズ境界。
    #[must_use]
    pub const fn new(
        stage: StageSlug,
        user_input: Option<String>,
        next_stage: Option<StageSlug>,
        phase_boundary: Option<PhaseBoundary>,
    ) -> GateApproved {
        GateApproved {
            stage,
            user_input,
            next_stage,
            phase_boundary,
        }
    }

    /// 承認されたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 承認時の人間入力 (逐語保持)。
    #[must_use]
    pub fn user_input(&self) -> Option<&str> {
        self.user_input.as_deref()
    }

    /// 次に開くステージ (`None` = ワークフロー完了)。
    #[must_use]
    pub const fn next_stage(&self) -> Option<&StageSlug> {
        self.next_stage.as_ref()
    }

    /// 跨いだフェーズ境界 (呼出側供給の投影材料)。
    #[must_use]
    pub const fn phase_boundary(&self) -> Option<PhaseBoundary> {
        self.phase_boundary
    }
}

/// `GateRejected` のペイロード。`revision_count` は集約が +1 した後の値 (BR1.4)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRejected {
    stage: StageSlug,
    feedback: Option<String>,
    revision_count: u32,
}

impl GateRejected {
    /// 差し戻したステージと、差し戻し理由・改訂回数。
    #[must_use]
    pub const fn new(
        stage: StageSlug,
        feedback: Option<String>,
        revision_count: u32,
    ) -> GateRejected {
        GateRejected {
            stage,
            feedback,
            revision_count,
        }
    }

    /// 差し戻したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 差し戻し理由 (逐語保持)。
    #[must_use]
    pub fn feedback(&self) -> Option<&str> {
        self.feedback.as_deref()
    }

    /// 差し戻し後の改訂回数 (状態ファイル `Revision Count` の材料)。
    #[must_use]
    pub const fn revision_count(&self) -> u32 {
        self.revision_count
    }
}

/// `StageRevised` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageRevised {
    stage: StageSlug,
}

impl StageRevised {
    /// ゲートに再入したステージ。
    #[must_use]
    pub const fn new(stage: StageSlug) -> StageRevised {
        StageRevised { stage }
    }

    /// ゲートに再入したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}

/// `StageSkipped` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSkipped {
    stage: StageSlug,
    reason: String,
    next_stage: Option<StageSlug>,
}

impl StageSkipped {
    /// 読み飛ばしたステージと、理由・次ステージ。
    #[must_use]
    pub const fn new(
        stage: StageSlug,
        reason: String,
        next_stage: Option<StageSlug>,
    ) -> StageSkipped {
        StageSkipped {
            stage,
            reason,
            next_stage,
        }
    }

    /// 読み飛ばしたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 読み飛ばしの理由 (逐語保持)。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// 次に開くステージ (`None` = ワークフロー完了)。
    #[must_use]
    pub const fn next_stage(&self) -> Option<&StageSlug> {
        self.next_stage.as_ref()
    }
}

/// `Jumped` のペイロード。承認の消去は `direction` と `target` から適用側が導出する (BR1.6)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Jumped {
    direction: JumpDirection,
    source: StageSlug,
    target: StageSlug,
    stages_reset: Vec<StageSlug>,
    stages_skipped: Vec<StageSlug>,
}

impl Jumped {
    /// 方向・出発点・到達点と、pending へ戻す列 / skipped にする列。
    #[must_use]
    pub const fn new(
        direction: JumpDirection,
        source: StageSlug,
        target: StageSlug,
        stages_reset: Vec<StageSlug>,
        stages_skipped: Vec<StageSlug>,
    ) -> Jumped {
        Jumped {
            direction,
            source,
            target,
            stages_reset,
            stages_skipped,
        }
    }

    /// 跳んだ方向 (カーソルとターゲットの大小から導出済み)。
    #[must_use]
    pub const fn direction(&self) -> JumpDirection {
        self.direction
    }

    /// 跳ぶ前のカーソル位置。
    #[must_use]
    pub const fn source(&self) -> &StageSlug {
        &self.source
    }

    /// 跳んだ先。
    #[must_use]
    pub const fn target(&self) -> &StageSlug {
        &self.target
    }

    /// pending へ戻したステージ列 (backward)。
    #[must_use]
    pub fn stages_reset(&self) -> &[StageSlug] {
        &self.stages_reset
    }

    /// skipped にしたステージ列 (forward)。
    #[must_use]
    pub fn stages_skipped(&self) -> &[StageSlug] {
        &self.stages_skipped
    }
}

/// `Parked` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parked {
    stage: StageSlug,
}

impl Parked {
    /// park した位置のステージ。
    #[must_use]
    pub const fn new(stage: StageSlug) -> Parked {
        Parked { stage }
    }

    /// park した位置のステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}

/// `Recomposed` のペイロード。1 コマンドの複数反転をまとめて 1 イベントにする (C5)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recomposed {
    skipped: Vec<StageSlug>,
    added: Vec<StageSlug>,
    stages_in_scope: Vec<StageSlug>,
}

impl Recomposed {
    /// EXECUTE → SKIP にした列、SKIP → EXECUTE にした列、適用後の in-scope 列。
    #[must_use]
    pub const fn new(
        skipped: Vec<StageSlug>,
        added: Vec<StageSlug>,
        stages_in_scope: Vec<StageSlug>,
    ) -> Recomposed {
        Recomposed {
            skipped,
            added,
            stages_in_scope,
        }
    }

    /// EXECUTE → SKIP に反転したステージ列。
    #[must_use]
    pub fn skipped(&self) -> &[StageSlug] {
        &self.skipped
    }

    /// SKIP → EXECUTE に反転したステージ列。
    #[must_use]
    pub fn added(&self) -> &[StageSlug] {
        &self.added
    }

    /// 適用後に実効プランが EXECUTE のステージ列 (状態ファイル `Total Stages` の材料)。
    #[must_use]
    pub fn stages_in_scope(&self) -> &[StageSlug] {
        &self.stages_in_scope
    }
}

/// `AutonomyModeSet` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyModeSet {
    mode: AutonomyMode,
}

impl AutonomyModeSet {
    /// 設定後のモード。
    #[must_use]
    pub const fn new(mode: AutonomyMode) -> AutonomyModeSet {
        AutonomyModeSet { mode }
    }

    /// 設定後のモード。
    #[must_use]
    pub const fn mode(&self) -> AutonomyMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        AutonomyMode, IntentId, JumpDirection, PhaseBoundary, StageEntry, StartRequest,
    };
    use crate::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    fn intent() -> IntentId {
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
    }

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn envelope(payload: WorkflowExecutionEventPayload) -> WorkflowExecutionEvent {
        WorkflowExecutionEvent::new(intent(), 3, at("2026-08-23T00:00:00Z"), payload)
    }

    #[test]
    fn the_envelope_carries_the_identity_sequence_schema_and_time() {
        let event = envelope(WorkflowExecutionEventPayload::Unparked);
        assert_eq!(event.aggregate_id(), &intent());
        assert_eq!(event.seq_nr(), 3);
        assert_eq!(event.schema_version(), 1);
        assert_eq!(WorkflowExecutionEvent::SCHEMA_VERSION, 1);
        assert_eq!(event.occurred_at(), &at("2026-08-23T00:00:00Z"));
        assert_eq!(event.payload(), &WorkflowExecutionEventPayload::Unparked);
    }

    #[test]
    fn the_started_payload_is_self_contained() {
        let entries = vec![StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
        )];
        let started = Started::new(
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            &StartRequest::new("classic", "build it").with_depth("standard"),
            entries.clone(),
        );
        assert_eq!(started.definition_id().as_str(), "claude");
        assert_eq!(started.definition_revision().as_str().len(), 71);
        assert_eq!(started.scope(), "classic");
        assert_eq!(started.request(), "build it");
        assert_eq!(started.depth(), Some("standard"));
        assert_eq!(started.test_strategy(), None);
        assert_eq!(started.stages(), entries.as_slice());
    }

    #[test]
    fn the_stage_lifecycle_payloads_carry_their_slugs_and_material() {
        let completed = StageCompleted::new(slug("state-init"), Some(slug("intent-capture")));
        assert_eq!(completed.stage(), &slug("state-init"));
        assert_eq!(completed.next_stage(), Some(&slug("intent-capture")));

        let opened = GateOpened::new(slug("intent-capture"), vec!["intent.md".to_string()]);
        assert_eq!(opened.stage(), &slug("intent-capture"));
        assert_eq!(opened.artifacts(), ["intent.md".to_string()]);

        let approved = GateApproved::new(
            slug("intent-capture"),
            Some("looks good".to_string()),
            None,
            Some(PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception)),
        );
        assert_eq!(approved.user_input(), Some("looks good"));
        assert_eq!(approved.next_stage(), None);
        assert_eq!(
            approved.phase_boundary(),
            Some(PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception))
        );

        let rejected = GateRejected::new(slug("intent-capture"), None, 2);
        assert_eq!(rejected.feedback(), None);
        assert_eq!(rejected.revision_count(), 2);

        let revised = StageRevised::new(slug("intent-capture"));
        assert_eq!(revised.stage(), &slug("intent-capture"));

        let skipped = StageSkipped::new(slug("market-research"), "out of scope".to_string(), None);
        assert_eq!(skipped.reason(), "out of scope");
        assert_eq!(skipped.next_stage(), None);
    }

    #[test]
    fn the_control_payloads_carry_the_jump_park_and_recompose_material() {
        let jumped = Jumped::new(
            JumpDirection::Backward,
            slug("intent-capture"),
            slug("state-init"),
            vec![slug("intent-capture")],
            Vec::new(),
        );
        assert_eq!(jumped.direction(), JumpDirection::Backward);
        assert_eq!(jumped.source(), &slug("intent-capture"));
        assert_eq!(jumped.target(), &slug("state-init"));
        assert_eq!(jumped.stages_reset(), [slug("intent-capture")]);
        assert!(jumped.stages_skipped().is_empty());

        let parked = Parked::new(slug("intent-capture"));
        assert_eq!(parked.stage(), &slug("intent-capture"));

        let recomposed = Recomposed::new(
            vec![slug("market-research")],
            Vec::new(),
            vec![slug("state-init")],
        );
        assert_eq!(recomposed.skipped(), [slug("market-research")]);
        assert!(recomposed.added().is_empty());
        assert_eq!(recomposed.stages_in_scope(), [slug("state-init")]);

        let mode = AutonomyModeSet::new(AutonomyMode::Autonomous);
        assert_eq!(mode.mode(), AutonomyMode::Autonomous);
    }

    #[test]
    fn the_event_identifier_is_the_aggregate_and_the_sequence_number() {
        // 採番は決定的 — 集約は時計も乱数も持たない (NFR3.1 / ADR-002)。
        let event = envelope(WorkflowExecutionEventPayload::Unparked);
        assert_eq!(event.id(), &WorkflowExecutionEventId::new(intent(), 3));
        assert_eq!(
            event.id().to_string(),
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6#3"
        );
    }

    #[test]
    fn only_the_genesis_event_reports_itself_as_a_creation() {
        let started = envelope(WorkflowExecutionEventPayload::Started(Started::new(
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            &StartRequest::new("mvp", "build"),
            Vec::new(),
        )));
        assert!(started.is_created());
        assert!(!envelope(WorkflowExecutionEventPayload::Unparked).is_created());
    }

    #[test]
    fn the_event_round_trips_through_serde() {
        // 本家 `Event` は `Serialize` / `Deserialize` を境界に持つ (ADR-010 Conformist)。
        let event = envelope(WorkflowExecutionEventPayload::Parked(Parked::new(slug(
            "intent-capture",
        ))));
        // 本家 trait の serde 境界の往復確認であり、契約 JSON (BR1.7) の直列化経路では
        // ないため、canon-json を経ない素の serde_json を使う。
        #[allow(
            clippy::disallowed_methods,
            reason = "契約 JSON ではなく serde 境界そのものの往復確認 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(
            serde_json::from_str::<WorkflowExecutionEvent>(&json).unwrap(),
            event
        );
    }

    #[test]
    fn events_compare_by_value() {
        let a = envelope(WorkflowExecutionEventPayload::Parked(Parked::new(slug(
            "intent-capture",
        ))));
        let b = envelope(WorkflowExecutionEventPayload::Parked(Parked::new(slug(
            "intent-capture",
        ))));
        assert_eq!(a, b);
        assert_ne!(a, envelope(WorkflowExecutionEventPayload::Unparked));
    }

    #[test]
    fn the_twelve_variants_are_matched_exhaustively() {
        // NFR1.3 — 変種の追加は C5 の改訂を伴うので `#[non_exhaustive]` は付けない。
        // 本テストは網羅 match をコンパイル時に固定する (腕が欠けたらビルドが落ちる)。
        fn name(payload: &WorkflowExecutionEventPayload) -> &'static str {
            match payload {
                WorkflowExecutionEventPayload::Started(_) => "Started",
                WorkflowExecutionEventPayload::StageCompleted(_) => "StageCompleted",
                WorkflowExecutionEventPayload::GateOpened(_) => "GateOpened",
                WorkflowExecutionEventPayload::GateApproved(_) => "GateApproved",
                WorkflowExecutionEventPayload::GateRejected(_) => "GateRejected",
                WorkflowExecutionEventPayload::StageRevised(_) => "StageRevised",
                WorkflowExecutionEventPayload::StageSkipped(_) => "StageSkipped",
                WorkflowExecutionEventPayload::Jumped(_) => "Jumped",
                WorkflowExecutionEventPayload::Parked(_) => "Parked",
                WorkflowExecutionEventPayload::Unparked => "Unparked",
                WorkflowExecutionEventPayload::Recomposed(_) => "Recomposed",
                WorkflowExecutionEventPayload::AutonomyModeSet(_) => "AutonomyModeSet",
            }
        }
        assert_eq!(name(&WorkflowExecutionEventPayload::Unparked), "Unparked");
        assert_eq!(
            name(&WorkflowExecutionEventPayload::AutonomyModeSet(
                AutonomyModeSet::new(AutonomyMode::Gated)
            )),
            "AutonomyModeSet"
        );
    }
}
