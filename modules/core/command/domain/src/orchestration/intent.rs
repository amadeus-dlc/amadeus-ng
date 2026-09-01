//! `Intent` — 静的な intent 集約 (`id` + 依頼 + 解決済み計画 + 走査結果)。
//!
//! 「1 つの intent を元に実行 (`IntentExecution`) は何回でも起きる」(1 intent : n 実行 —
//! オーナー裁定 2026-08-29) という意味論において、**回数によらず変わらない側**がこの型で
//! ある。
//!
//! これは**集約**である (オーナー裁定 2026-08-30 — [`WorkflowDefinition`] と同じ類型で、
//! 静的で変異が現状無いだけ)。基本コンストラクタは genesis の [`Intent::create`] —
//! **全情報 (`&WorkflowDefinition`) を受けて (集約, 誕生イベント) の対**を返す — である
//! (オーナー裁定 2026-08-30)。再構成は**イベント列から**行う — 誕生記録の変換
//! (`From<Created>`) がスナップショット種を与え、[`Intent::replay`] が差分イベントを
//! 畳み込む。再構成はファクトリではなく、失敗も返さない — 壊れた歴史はクラッシュが正
//! である (オーナー裁定 2026-08-30、本家 v3 サンプル同型)。誕生イベントを `store` し
//! イベントから再構成する `IntentRepository` の実装はアダプタ層にある (issue #50)。
//!
//! 実行時の状態 (カーソル・checkbox・park・承認履歴…) は集約 [`IntentExecution`] が持ち、
//! 集約はこの型を**埋め込まず `IntentId` で参照する** (coding-rules/aggregate-references.md)。
//! 判断に計画が要るコマンド・クエリは、この型を `&` 参照で引数に受け取る。
//!
//! [`IntentExecution`]: super::intent_execution::IntentExecution
//! [`WorkflowDefinition`]: crate::workflow_definition::WorkflowDefinition

use chrono::{DateTime, Utc};

use super::intent_error::IntentError;
use super::intent_event::Created;
use super::intent_event::IntentEvent;
use super::intent_id::IntentId;
use super::stage_display::StageDisplay;
use super::stage_entry::StageEntry;
use super::start_request::StartRequest;
use super::workspace_scan::WorkspaceScan;
use crate::workflow_definition::DefinitionRevision;
use crate::workflow_definition::ExecutionKind;
use crate::workflow_definition::PhaseId;
use crate::workflow_definition::PlanAction;
use crate::workflow_definition::UnknownScope;
use crate::workflow_definition::WorkflowDefinition;
use crate::workflow_definition::WorkflowDefinitionId;

/// 静的な intent — 実行が何回起きても変わらない側 (Always Valid)。
///
/// `stages` は**この intent 向けに解決済みの計画**である。定義 (`WorkflowDefinition` =
/// 全 intent 共通のプロセス定義) を `definition_id` / `definition_revision` でピンし、
/// そこから解決した EXECUTE / SKIP 列を文書順に持つ。定義そのものは持たない。
///
/// **永続化の記述は持たない** (`coding-rules/domain-persistence-neutrality.md`)。行のバイトを
/// 決めるのはアダプタ層の DTO で、復号は `Created` を組んで `From<Created>` を通る —
/// 検査を迂回する構築口は存在しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    id: IntentId,
    definition_id: WorkflowDefinitionId,
    definition_revision: DefinitionRevision,
    start_request: StartRequest,
    stages: Vec<StageEntry>,
    scan: WorkspaceScan,
    created_at: DateTime<Utc>,
}

impl Intent {
    /// 新しい intent を作る (基本コンストラクタ = genesis — 対を返す)。
    ///
    /// **基本コンストラクタは全情報を受ける genesis である** (オーナー裁定 2026-08-30)。定義
    /// そのもの (`&WorkflowDefinition`) から計画を解決し、(集約, 誕生イベント) の対を返す。
    /// 対で返すのは、Repository の永続化が `store(&event, &aggregate, ..)` の形でジャーナル
    /// 1 行とスナップショットを同一トランザクションで受け取るからである — どちらが欠けても
    /// 永続化が組めない (coding-rules/aggregate-commands.md)。補助コンストラクタを設ける場合は
    /// 必ずここへ委譲する (coding-rules/factory-naming.md)。再構成 (`From<Created>` /
    /// [`Intent::replay`]) はファクトリではないので、この委譲規律の対象外である。
    ///
    /// 動詞 `create` は upstream の `intent-create` そのものである
    /// (coding-rules/factory-naming.md — ドメイン語がある場合はそちらを優先する)。
    ///
    /// `definition.id()` / `definition.revision()` は**無条件に控える** — 比較対象となる既存状態が
    /// 無い genesis なので検査はしない (BR2.6)。以後の定義照合は読むだけの観測なのでクエリ側
    /// (`next` / `continue` の判断と steering 束縛ダイジェスト) の持ち物である (b26 段階 2)。
    /// 表示属性は**計画を解決するこの時点で**焼き込む (オーナー裁定 2026-08-29) — 投影は後から
    /// 定義を引かないので、再構成しても当時と同じバイトになる (NFR3)。
    ///
    /// # Errors
    ///
    /// 未知スコープ (`UnknownScope`)、表示属性が単一行でない (`StageDisplayNotSingleLine`)、
    /// および計画の不変条件違反 (`Empty` / `InitializationMustExecute` /
    /// `InitializationMustBeUnconditional`) を拒否する。
    pub fn create(
        id: IntentId,
        definition: &WorkflowDefinition,
        start_request: StartRequest,
        scan: WorkspaceScan,
        occurred_at: DateTime<Utc>,
    ) -> Result<(Intent, IntentEvent), IntentError> {
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
        Intent::check_plan(&stages)?;
        let created = Created::new(
            id,
            definition.id().clone(),
            definition.revision().clone(),
            start_request,
            stages,
            scan,
        );
        let intent = Intent::from((created.clone(), occurred_at));
        Ok((intent, IntentEvent::Created(created)))
    }

    /// スナップショット種に差分イベントを畳み込んで復元する (Event Sourcing の再生経路)。
    ///
    /// 本家 v3 の `UserAccount::replay(events, snapshot)` と同型。スナップショット種は
    /// 誕生記録の変換 (`From<Created>`) で得る。**再構成は失敗を返さない** — 歴史を読む
    /// だけであり、壊れた歴史は回復せずクラッシュする (オーナー裁定 2026-08-30)。現状
    /// イベントは `Created` 1 種のみのため差分は常に空だが、後続イベントが増えたときの
    /// 適用経路はここに閉じる (通常実行とリプレイの同一経路 — BR1.1)。
    #[must_use]
    pub fn replay(events: impl IntoIterator<Item = IntentEvent>, snapshot: Intent) -> Intent {
        events.into_iter().fold(snapshot, |mut intent, event| {
            intent.apply_event(&event);
            intent
        })
    }

    /// イベントを 1 つ適用する (リプレイの唯一の状態遷移経路)。
    #[allow(
        clippy::unused_self,
        reason = "変異イベントが増えたときの適用経路をここに閉じる (BR1.1) — 現状 genesis の \
                  1 変種だけなので状態は動かない"
    )]
    const fn apply_event(&mut self, event: &IntentEvent) {
        // 変種の網羅 match — 腕の欠落はビルドで落ちる。genesis イベントは差分適用では
        // 何も変えない (スナップショット種が誕生を含む — 本家サンプル同型)。
        match event {
            IntentEvent::Created(_) => {}
        }
    }

    /// 解決済み計画の不変条件 (genesis と再構成で完全に同一の 1 か所)。
    fn check_plan(stages: &[StageEntry]) -> Result<(), IntentError> {
        match stages.first() {
            None => return Err(IntentError::Empty),
            Some(first) if first.plan_action() != PlanAction::Execute => {
                return Err(IntentError::InitializationMustExecute);
            }
            Some(_) => {}
        }
        for entry in stages {
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
        Ok(())
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

    /// 呼出側が解決したレビュー上限 (`None` = 指定なし)。
    #[must_use]
    pub fn review(&self) -> Option<&str> {
        self.start_request.review()
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

    /// 鋳造の発生時刻 (genesis の `occurred_at` — ジャーナル封筒の時刻の出所。
    /// `IntentExecution::last_updated_at` / `WorkflowDefinition::last_updated_at` と対)。
    #[must_use]
    pub const fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }
}

impl From<(Created, DateTime<Utc>)> for Intent {
    /// 誕生記録とその発生時刻から集約を導出する (リプレイのスナップショット種 —
    /// `WorkflowDefinition` の `From<(Defined, occurred_at)>` と対)。
    ///
    /// 発生時刻は封筒 (ジャーナル行のメタデータ) の持ち物なのでイベントは運ばず、
    /// 読み手が封筒から対にして渡す。構造体リテラルはここだけ — genesis
    /// ([`Intent::create`]) もこの変換を通る。記録された歴史は書込時に検査済みである。
    /// 万一壊れた歴史 (計画不変条件違反) を読んだ場合は回復せずクラッシュする
    /// (オーナー裁定 2026-08-30 — 再構成は失敗を返さない。本家 v3 ではこの位置づけの
    /// 検査自体が無く、serde 復号がそのまま集約になる)。
    #[allow(
        clippy::expect_used,
        reason = "壊れた歴史は回復不能 — 再構成は失敗を返さずクラッシュする (オーナー裁定 2026-08-30)"
    )]
    fn from((created, occurred_at): (Created, DateTime<Utc>)) -> Intent {
        Intent::check_plan(&created.stages).expect("recorded history violates the plan invariants");
        Intent {
            id: created.id,
            definition_id: created.definition_id,
            definition_revision: created.definition_revision,
            start_request: created.start_request,
            stages: created.stages,
            scan: created.scan,
            created_at: occurred_at,
        }
    }
}

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

    use chrono::{DateTime, Utc};

    const SAMPLE: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn id() -> IntentId {
        IntentId::parse(SAMPLE).unwrap()
    }

    fn def_id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").unwrap()
    }

    /// 定義イベントの発生時刻 (定義の genesis に渡す固定値)。
    fn defined_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-31T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
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
        Intent::from((
            Created::new(id(), def_id(), revision(), request(), stages(), scan()),
            defined_at(),
        ))
    }

    /// 1 ステージ (initialization・EXECUTE) だけの最小定義。
    fn single_stage_definition() -> WorkflowDefinition {
        let node = StageNodeBuilder::new(
            StageSlug::parse("state-init").unwrap(),
            StageNumber::parse("0.1").unwrap(),
            "State Init".to_string(),
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
        WorkflowDefinition::define(
            def_id(),
            revision(),
            StageGraph::new(vec![node]).unwrap(),
            grid,
            scopes,
            defined_at(),
        )
        .0
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
    #[should_panic(expected = "recorded history violates the plan invariants")]
    fn an_empty_plan_crashes_reconstruction() {
        // 再構成は失敗を返さない — 壊れた歴史はクラッシュが正 (オーナー裁定 2026-08-30)。
        let _ = Intent::from((
            Created::new(id(), def_id(), revision(), request(), Vec::new(), scan()),
            defined_at(),
        ));
    }

    #[test]
    #[should_panic(expected = "recorded history violates the plan invariants")]
    fn a_first_stage_that_is_not_execute_crashes_reconstruction() {
        // 再構成は失敗を返さない — 壊れた歴史はクラッシュが正 (オーナー裁定 2026-08-30)。
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
        let _ = Intent::from((
            Created::new(id(), def_id(), revision(), request(), stages, scan()),
            defined_at(),
        ));
    }

    #[test]
    #[should_panic(expected = "recorded history violates the plan invariants")]
    fn an_initialization_stage_folded_to_skip_crashes_reconstruction() {
        // 再構成は失敗を返さない — 壊れた歴史はクラッシュが正 (オーナー裁定 2026-08-30)。
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
        let _ = Intent::from((
            Created::new(id(), def_id(), revision(), request(), stages, scan()),
            defined_at(),
        ));
    }

    #[test]
    #[should_panic(expected = "recorded history violates the plan invariants")]
    fn a_conditional_initialization_stage_crashes_reconstruction() {
        // 再構成は失敗を返さない — 壊れた歴史はクラッシュが正 (オーナー裁定 2026-08-30)。
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
        let _ = Intent::from((
            Created::new(id(), def_id(), revision(), request(), stages, scan()),
            defined_at(),
        ));
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
        let intent = Intent::from((
            Created::new(id(), def_id(), revision(), request(), stages, scan()),
            defined_at(),
        ));
        assert_eq!(intent.stage_count(), 2);
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
        let definition = WorkflowDefinition::define(
            def_id(),
            revision(),
            StageGraph::new(vec![node]).unwrap(),
            grid,
            scopes,
            defined_at(),
        )
        .0;
        assert_eq!(
            Intent::create(id(), &definition, request(), scan(), defined_at()),
            Err(IntentError::StageDisplayNotSingleLine {
                stage: "state-init".to_string(),
                found: '\n',
            })
        );
    }

    #[test]
    fn intents_built_from_the_same_parts_compare_equal() {
        assert_eq!(intent(), intent());
        let other = Intent::from((
            Created::new(
                IntentId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").unwrap(),
                def_id(),
                revision(),
                request(),
                stages(),
                scan(),
            ),
            defined_at(),
        ));
        assert_ne!(intent(), other);
    }

    #[test]
    fn creating_an_intent_yields_the_aggregate_and_its_birth_event() {
        // 基本コンストラクタ = genesis は定義から計画を解決し、(インスタンス, 誕生イベント) の
        // 対を返す (coding-rules/aggregate-commands.md)。イベントは誕生の材料 (値) を運び、
        // 変換 `From<Created>` で同じ集約に戻る。
        let (intent, event) = Intent::create(
            id(),
            &single_stage_definition(),
            request(),
            scan(),
            defined_at(),
        )
        .expect("解決できる計画");
        let IntentEvent::Created(created) = event;
        assert_eq!(created.id(), intent.id());
        assert_eq!(created.stages(), intent.stages());
        assert_eq!(Intent::from((created, defined_at())), intent);
        assert_eq!(intent.stage_count(), 1);
    }

    #[test]
    fn reconstructing_an_intent_produces_no_event() {
        // 再構成は歴史を読み戻す経路である — イベントを作ればリプレイのたびに歴史が増える。
        // 戻り値の型に `IntentEvent` が現れないことがその保証である。
        let intent: Intent = Intent::from((
            Created::new(id(), def_id(), revision(), request(), stages(), scan()),
            defined_at(),
        ));
        assert_eq!(intent.stages(), stages().as_slice());
    }

    #[test]
    fn a_birth_record_round_trip_preserves_the_aggregate() {
        // 集約の読取値 → 誕生記録 → 変換の 1 往復が同値に戻ること — 永続化境界を渡っても
        // 情報が欠けない保証である (アダプタ復号と同じ経路)。
        let (intent, _event) = Intent::create(
            id(),
            &single_stage_definition(),
            request(),
            scan(),
            defined_at(),
        )
        .expect("解決できる計画");
        let round_tripped = Intent::from((
            Created::new(
                intent.id().clone(),
                intent.definition_id().clone(),
                intent.definition_revision().clone(),
                request(),
                intent.stages().to_vec(),
                intent.scan().clone(),
            ),
            defined_at(),
        ));
        assert_eq!(round_tripped, intent);
    }

    #[test]
    fn replaying_no_events_returns_the_snapshot_state() {
        // 差分が空ならスナップショット種がそのまま返る — 現状イベントは `Created` 1 種のみ
        // なので、実運用の差分は常に空である。
        let (intent, _event) = Intent::create(
            id(),
            &single_stage_definition(),
            request(),
            scan(),
            defined_at(),
        )
        .expect("解決できる計画");
        let replayed = Intent::replay(Vec::new(), intent.clone());
        assert_eq!(replayed, intent);
    }

    #[test]
    fn replaying_the_genesis_event_is_a_no_op() {
        // genesis イベントは差分適用では何も変えない — スナップショット種が誕生を含む
        // (本家サンプル同型: apply は変異イベントだけを見る)。
        let (intent, event) = Intent::create(
            id(),
            &single_stage_definition(),
            request(),
            scan(),
            defined_at(),
        )
        .expect("解決できる計画");
        let replayed = Intent::replay(vec![event], intent.clone());
        assert_eq!(replayed, intent);
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
