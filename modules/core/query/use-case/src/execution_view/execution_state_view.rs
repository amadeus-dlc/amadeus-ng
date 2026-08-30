//! `ExecutionStateView` — リードモデル `aidlc-state.md` のクエリモデルと、その上の判断。
//!
//! クエリ側は RMU が構築したリードモデルだけに依存し、**集約を再構成しない**
//! (`coding-rules/cqrs-boundaries.md` 規則 6)。したがって「次に何をすべきか」の判断
//! ([`ExecutionStateView::next_decision`]) は集約のメソッドではなく、本ビューの
//! 関連メソッドとして持つ (`coding-rules/domain-services.md` — 対象を決めれば `::` / `.` で
//! 全タスクが見える)。
//!
//! # 旧コマンド側集約との対応 (B26 段階 1 の移植記録)
//!
//! ラダーが `IntentExecution` から読んでいた 11 観測は、すべて本ビューの面として
//! リードモデルから導ける。対応は以下のとおり。
//!
//! | 旧 (集約) | 本ビュー | リードモデル上の出所 |
//! |---|---|---|
//! | `accepts_commands()` | [`Self::accepts_commands`] | `Status` ∧ park マーカー |
//! | `checkbox(i)` | [`Self::checkbox`] | Stage Progress 行のマーカー |
//! | `cursor()` | [`Self::cursor`] | `- **Current Stage**:` の slug 位置 |
//! | `effective_plan(i)` | [`Self::effective_plan`] | Stage Progress 行末の EXECUTE/SKIP |
//! | `in_scope(i)` | [`Self::is_in_scope`] | 同上 (`EXECUTE` か) |
//! | `is_gated(i)` | [`Self::is_gated`] | `### <PHASE> PHASE` 見出し |
//! | `next_in_scope(i)` | [`Self::next_in_scope`] | 行末トークンの前進走査 |
//! | `parked_active()` | [`Self::parked_active`] | `- **Parked At Stage**:` == cursor |
//! | `stage_count()` | [`Self::stage_count`] | Stage Progress の行数 |
//! | `status()` | [`Self::status`] | `- **Status**:` |
//! | `matches(intent)` / 定義 id 照合 | — (下記) | — |
//!
//! **`matches` 相当の照合は本ビューには無い**。旧 `IntentExecution::next_decision` の
//! `IntentMismatch` / `DefinitionMismatch` は「集約に別の `&Intent` / `&WorkflowDefinition`
//! を渡す」取り違えを防ぐガードであり、渡す相手が 2 つある**コマンド側の署名にだけ**存在
//! する危険である。クエリ側は 1 つのビューを読むだけで、リードモデルは定義 id のピンを
//! 記録していない (`aidlc-state.md` にも `intents.json` にもフィールドが無い — 実測) ため、
//! どちらの照合も再現できず意味も持たない。よって [`Self::next_decision`] は `Result` では
//! なく [`NextDecision`] を直接返す。

use core_infrastructure::canon_json::{JsonValue, hash_compact};

use super::checkbox_state::CheckboxState;
use super::execution_status::ExecutionStatus;
use super::stage_index::StageIndex;
use super::stage_progress_view::StageProgressView;
use crate::orchestration::{NextDecision, NextRequest, StateBinding};
use crate::workflow_view::{PlanActionView, ScopeSlugView, StageSlugView};

/// 実効 SKIP の不整合から自力で復旧できる checkbox 前提集合 (`skip_stage` を呼べる状態)。
const SKIP_PRECONDITION: [CheckboxState; 2] = [CheckboxState::InProgress, CheckboxState::Revising];

/// 実行状態リードモデルのビュー (構築後 immutable)。
///
/// 構築するのはクエリ側のアダプタ (`aidlc-state.md` のパーサ) だけで、ビュー自身は
/// I/O も直列化も知らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStateView {
    scope: ScopeSlugView,
    status: ExecutionStatus,
    cursor: StageIndex,
    parked_at: Option<StageIndex>,
    last_updated: String,
    stages: Vec<StageProgressView>,
}

/// [`ExecutionStateView`] の構築が拒否する形。
///
/// リードモデルとして成立しない (= 判断の土台にできない) 観測だけを拒否する。文言は
/// 出す側が組む (`coding-rules/error-handling.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStateError {
    /// Stage Progress の行が 1 本も無い (カーソルの置き場が無い)。
    NoStages,
    /// `Current Stage` が Stage Progress のどの行とも一致しない。
    UnknownCursor {
        /// 一致しなかった slug (逐語)。
        stage: String,
    },
    /// `Parked At Stage` が Stage Progress のどの行とも一致しない。
    UnknownParkedStage {
        /// 一致しなかった slug (逐語)。
        stage: String,
    },
}

impl std::fmt::Display for ExecutionStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStateError::NoStages => f.write_str("no stage progress rows"),
            ExecutionStateError::UnknownCursor { stage } => {
                write!(f, "unknown current stage {stage:?}")
            }
            ExecutionStateError::UnknownParkedStage { stage } => {
                write!(f, "unknown parked stage {stage:?}")
            }
        }
    }
}

impl std::error::Error for ExecutionStateError {}

impl ExecutionStateView {
    /// 読み終えたリードモデルを束ねる (基本コンストラクタ — 構造体リテラルはここだけ)。
    ///
    /// カーソルと park 位置は **slug で**受け取り、ここで Stage Progress 上の位置へ解決する。
    /// 生の索引を外から渡させると範囲外が構成可能になるので、解決はこの型が所有する。
    ///
    /// # Errors
    ///
    /// 行が 1 本も無い (`NoStages`)、カーソル・park 位置が行に無い
    /// (`UnknownCursor` / `UnknownParkedStage`)。
    pub fn new(
        scope: ScopeSlugView,
        status: ExecutionStatus,
        cursor: &str,
        parked_at: Option<&str>,
        last_updated: impl Into<String>,
        stages: Vec<StageProgressView>,
    ) -> Result<ExecutionStateView, ExecutionStateError> {
        if stages.is_empty() {
            return Err(ExecutionStateError::NoStages);
        }
        let position = |slug: &str| stages.iter().position(|row| row.slug().as_str() == slug);
        let cursor_index = position(cursor).map(StageIndex::new).ok_or_else(|| {
            ExecutionStateError::UnknownCursor {
                stage: cursor.to_string(),
            }
        })?;
        let parked_index = match parked_at {
            None => None,
            Some(slug) => Some(position(slug).map(StageIndex::new).ok_or_else(|| {
                ExecutionStateError::UnknownParkedStage {
                    stage: slug.to_string(),
                }
            })?),
        };
        Ok(ExecutionStateView {
            scope,
            status,
            cursor: cursor_index,
            parked_at: parked_index,
            last_updated: last_updated.into(),
            stages,
        })
    }

    // ---- 観測 ----

    /// `- **Scope**:` — 稼働中ワークフローの scope (解決ラダーの最上位観測)。
    #[must_use]
    pub const fn scope(&self) -> &ScopeSlugView {
        &self.scope
    }

    /// `- **Status**:` の現在値 (park マーカーとは直交)。
    #[must_use]
    pub const fn status(&self) -> ExecutionStatus {
        self.status
    }

    /// `- **Current Stage**:` の位置。
    #[must_use]
    pub const fn cursor(&self) -> StageIndex {
        self.cursor
    }

    /// park マーカーが記録している位置 (`None` は未 park)。
    #[must_use]
    pub const fn parked_at(&self) -> Option<StageIndex> {
        self.parked_at
    }

    /// `- **Last Updated**:` の逐語値 (state 束縛の素材)。
    #[must_use]
    pub fn last_updated(&self) -> &str {
        &self.last_updated
    }

    /// Stage Progress の全行 (文書順)。
    #[must_use]
    pub fn stages(&self) -> &[StageProgressView] {
        &self.stages
    }

    /// 追いかけているステージ総数 (スコープ外の行も含む)。
    #[must_use]
    pub const fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// 生の位置から [`StageIndex`] を作る唯一の公開経路。範囲外は `None` (BR5.1)。
    #[must_use]
    pub fn stage_index(&self, value: usize) -> Option<StageIndex> {
        (value < self.stage_count()).then(|| StageIndex::new(value))
    }

    /// 名指し位置の行。範囲外は `None`。
    #[must_use]
    pub fn row(&self, stage: StageIndex) -> Option<&StageProgressView> {
        self.stages.get(stage.to_usize())
    }

    /// 名指し位置の slug。範囲外は `None`。
    #[must_use]
    pub fn slug(&self, stage: StageIndex) -> Option<&StageSlugView> {
        self.row(stage).map(StageProgressView::slug)
    }

    /// 名指し位置の checkbox マーカー。範囲外は `None`。
    #[must_use]
    pub fn checkbox(&self, stage: StageIndex) -> Option<CheckboxState> {
        self.row(stage).map(StageProgressView::checkbox)
    }

    /// 実効プラン (行末 EXECUTE / SKIP)。範囲外は `None`。
    #[must_use]
    pub fn effective_plan(&self, stage: StageIndex) -> Option<PlanActionView> {
        self.row(stage).map(StageProgressView::plan)
    }

    /// スコープ内か — 実効プランが EXECUTE (範囲外は偽)。
    #[must_use]
    pub fn is_in_scope(&self, stage: StageIndex) -> bool {
        self.row(stage).is_some_and(StageProgressView::is_in_scope)
    }

    /// ゲート付きか — `phase != initialization` (範囲外は偽)。
    #[must_use]
    pub fn is_gated(&self, stage: StageIndex) -> bool {
        self.row(stage).is_some_and(StageProgressView::is_gated)
    }

    /// `after` より後ろで最初の in-scope ステージ (BR1.5 の前進走査)。
    #[must_use]
    pub fn next_in_scope(&self, after: StageIndex) -> Option<StageIndex> {
        ((after.to_usize() + 1)..self.stage_count())
            .map(StageIndex::new)
            .find(|&stage| self.is_in_scope(stage))
    }

    /// parked 分岐の発火は導出述語 (マーカー有 ∧ 位置一致 — BR1.7)。
    #[must_use]
    pub fn parked_active(&self) -> bool {
        self.parked_at == Some(self.cursor)
    }

    /// コマンド受理述語 (BR1.0)。
    #[must_use]
    pub fn accepts_commands(&self) -> bool {
        self.status.is_running() && !self.parked_active()
    }

    // ---- 判断 ----

    /// 現状態から次に何をすべきかを 1 つ決める (BR3.1 の優先順)。書込なし。
    ///
    /// 分岐と優先順は旧 `IntentExecution::next_decision` と同一である。集約の取り違えガード
    /// (`IntentMismatch` / `DefinitionMismatch`) だけが**存在しない** — モジュール doc の
    /// 「`matches` 相当の照合」を参照。
    #[must_use]
    pub fn next_decision(&self, request: &NextRequest) -> NextDecision {
        if self.parked_active() && !request.is_reentry() {
            return if request.is_resume() {
                NextDecision::UnparkThenResume
            } else {
                NextDecision::Parked { stage: self.cursor }
            };
        }
        if request.is_resume() {
            return NextDecision::ResumeMenu;
        }
        if request.is_free_text() {
            return NextDecision::NewWorkRouting;
        }
        if !self.status.is_running() {
            return NextDecision::Done;
        }
        let cursor = self.cursor;
        if let Some(marker) = self.checkbox(cursor)
            && marker.is_in_flight()
        {
            if self.effective_plan(cursor) == Some(PlanActionView::Skip) {
                // 実効 SKIP のステージに run-stage は出さない。自力で `skip_stage` を呼べる
                // 前提集合 (SKIP_PRECONDITION) にいるときだけ復旧可能と報告する。
                return if SKIP_PRECONDITION.contains(&marker) {
                    NextDecision::RecoverSkipInconsistency {
                        stage: cursor,
                        checkbox: marker,
                    }
                } else {
                    NextDecision::InconsistentSkip {
                        stage: cursor,
                        checkbox: marker,
                    }
                };
            }
            return NextDecision::RunStage {
                stage: cursor,
                gate: self.is_gated(cursor),
            };
        }
        match self.next_in_scope(cursor) {
            Some(stage) => NextDecision::RunStage {
                stage,
                gate: self.is_gated(stage),
            },
            None => NextDecision::Done,
        }
    }

    /// state 束縛のダイジェスト (`h`) — 「この state はまだ動いていないか」の照合子。
    ///
    /// # 旧実装との対応
    ///
    /// 旧 `IntentExecution::state_binding` の素材は「どの intent の・何番目まで進んだ歴史の・
    /// どの採番版か」(`intent_id` / `seq_nr` / `version`) だった。この 3 つはいずれも
    /// **集約の内部状態**であり、リードモデルには 1 つも現れない (実測: `aidlc-state.md` に
    /// intent uuid・通番・楽観 version のフィールドは無い)。
    ///
    /// そこで束縛の**意味**「state が動いたら値も動く」をリードモデルの語で組み直す —
    /// scope・カーソル・Status・park 位置・最終更新時刻に加え、Stage Progress の全行
    /// (slug + マーカー + 実効プラン) を素材に取る。旧実装より粒度は細かく、旧実装が捉えた
    /// 遷移 (通番の前進 = 何らかの状態変化) はすべてこちらでも値が動く。
    ///
    /// mint (`next`) と verify (`continue`) の双方がこの同一の導出を通るので、トークン契約は
    /// 保たれる。トークンはセッションローカルなので、旧実装と値が変わっても互換負債は無い。
    ///
    /// 素材は名前付き構造の canon-json 手組みである (`Debug` 表現への依存は derive 変更で
    /// 黙って値が変わる時限爆弾、区切り文字連結は区切り文字注入を許す — オーナー裁定
    /// 2026-08-30)。
    #[must_use]
    pub fn state_binding(&self) -> StateBinding {
        let stages = JsonValue::Array(
            self.stages
                .iter()
                .map(|row| {
                    object([
                        ("slug", JsonValue::String(row.slug().as_str().to_string())),
                        (
                            "checkbox",
                            JsonValue::String(row.checkbox().marker().to_string()),
                        ),
                        ("plan", JsonValue::String(row.plan().as_str().to_string())),
                    ])
                })
                .collect(),
        );
        let material = object([
            ("scope", JsonValue::String(self.scope.as_str().to_string())),
            (
                "cursor",
                self.slug(self.cursor).map_or(JsonValue::Null, |slug| {
                    JsonValue::String(slug.as_str().to_string())
                }),
            ),
            (
                "status",
                JsonValue::String(self.status.as_str().to_string()),
            ),
            (
                "parked_at",
                self.parked_at
                    .and_then(|stage| self.slug(stage))
                    .map_or(JsonValue::Null, |slug| {
                        JsonValue::String(slug.as_str().to_string())
                    }),
            ),
            ("last_updated", JsonValue::String(self.last_updated.clone())),
            ("stages", stages),
        ]);
        StateBinding::new(hash_compact(&material).rendered())
    }
}

/// 挿入順を保持する素材オブジェクト (順序が素材バイトの一部である)。
fn object<const N: usize>(members: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        members
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容する (clippy.toml に相当設定が無いため
    // モジュール単位で allow — 集約側のテストモジュールと同じ作法)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::workflow_view::PhaseView;
    use proptest::prelude::*;

    fn row(
        slug: &str,
        phase: PhaseView,
        marker: CheckboxState,
        plan: PlanActionView,
    ) -> StageProgressView {
        StageProgressView::new(StageSlugView::parse(slug).unwrap(), phase, marker, plan)
    }

    /// 索引 0 = initialization (非ゲート)、以降 = inception (ゲート付き) の 4 行。
    fn view(
        status: ExecutionStatus,
        cursor: &str,
        parked_at: Option<&str>,
        markers: [CheckboxState; 4],
        plans: [PlanActionView; 4],
    ) -> ExecutionStateView {
        let mut stages = Vec::new();
        for index in 0..4usize {
            let phase = if index == 0 {
                PhaseView::Initialization
            } else {
                PhaseView::Inception
            };
            stages.push(row(
                &format!("stage-{index}"),
                phase,
                markers[index],
                plans[index],
            ));
        }
        ExecutionStateView::new(
            ScopeSlugView::parse("classic").unwrap(),
            status,
            cursor,
            parked_at,
            "2026-08-29T16:36:24Z",
            stages,
        )
        .unwrap()
    }

    fn running(cursor: &str, markers: [CheckboxState; 4]) -> ExecutionStateView {
        view(
            ExecutionStatus::Running,
            cursor,
            None,
            markers,
            [PlanActionView::Execute; 4],
        )
    }

    const ALL_PENDING: [CheckboxState; 4] = [CheckboxState::Pending; 4];

    #[test]
    fn the_constructor_resolves_the_cursor_and_the_park_marker_by_slug() {
        let held = view(
            ExecutionStatus::Running,
            "stage-2",
            Some("stage-2"),
            ALL_PENDING,
            [PlanActionView::Execute; 4],
        );
        assert_eq!(held.cursor().to_usize(), 2);
        assert_eq!(held.parked_at().map(StageIndex::to_usize), Some(2));
        assert!(held.parked_active());
        assert!(!held.accepts_commands());
        assert_eq!(held.scope().as_str(), "classic");
        assert_eq!(held.status(), ExecutionStatus::Running);
        assert_eq!(held.last_updated(), "2026-08-29T16:36:24Z");
        assert_eq!(held.stage_count(), 4);
    }

    #[test]
    fn a_park_marker_elsewhere_does_not_fire_the_parked_branch() {
        let held = view(
            ExecutionStatus::Running,
            "stage-1",
            Some("stage-2"),
            ALL_PENDING,
            [PlanActionView::Execute; 4],
        );
        assert!(!held.parked_active());
        assert!(held.accepts_commands());
    }

    #[test]
    fn the_constructor_refuses_a_read_model_that_cannot_carry_a_cursor() {
        assert_eq!(
            ExecutionStateView::new(
                ScopeSlugView::parse("classic").unwrap(),
                ExecutionStatus::Running,
                "stage-0",
                None,
                "t",
                Vec::new(),
            ),
            Err(ExecutionStateError::NoStages)
        );
        let stages = vec![row(
            "stage-0",
            PhaseView::Initialization,
            CheckboxState::Pending,
            PlanActionView::Execute,
        )];
        assert_eq!(
            ExecutionStateView::new(
                ScopeSlugView::parse("classic").unwrap(),
                ExecutionStatus::Running,
                "ghost",
                None,
                "t",
                stages.clone(),
            ),
            Err(ExecutionStateError::UnknownCursor {
                stage: "ghost".to_string()
            })
        );
        assert_eq!(
            ExecutionStateView::new(
                ScopeSlugView::parse("classic").unwrap(),
                ExecutionStatus::Running,
                "stage-0",
                Some("ghost"),
                "t",
                stages,
            ),
            Err(ExecutionStateError::UnknownParkedStage {
                stage: "ghost".to_string()
            })
        );
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(
            ExecutionStateError::NoStages.to_string(),
            "no stage progress rows"
        );
        assert_eq!(
            ExecutionStateError::UnknownCursor {
                stage: "ghost".to_string()
            }
            .to_string(),
            "unknown current stage \"ghost\""
        );
        let boxed: Box<dyn std::error::Error> = Box::new(ExecutionStateError::UnknownParkedStage {
            stage: "ghost".to_string(),
        });
        assert_eq!(boxed.to_string(), "unknown parked stage \"ghost\"");
    }

    #[test]
    fn the_row_faces_read_the_stage_progress_line() {
        let held = view(
            ExecutionStatus::Running,
            "stage-1",
            None,
            [
                CheckboxState::Completed,
                CheckboxState::InProgress,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
            [
                PlanActionView::Execute,
                PlanActionView::Execute,
                PlanActionView::Skip,
                PlanActionView::Execute,
            ],
        );
        let at = |i: usize| held.stage_index(i).unwrap();
        assert_eq!(held.checkbox(at(0)), Some(CheckboxState::Completed));
        assert_eq!(held.effective_plan(at(2)), Some(PlanActionView::Skip));
        assert!(held.is_in_scope(at(1)));
        assert!(!held.is_in_scope(at(2)));
        assert!(!held.is_gated(at(0)), "initialization は非ゲート");
        assert!(held.is_gated(at(1)));
        assert_eq!(held.slug(at(3)).map(StageSlugView::as_str), Some("stage-3"));
        assert_eq!(
            held.next_in_scope(at(1)).map(StageIndex::to_usize),
            Some(3),
            "SKIP は読み飛ばす"
        );
        assert_eq!(held.next_in_scope(at(3)), None);
        assert_eq!(held.stage_index(4), None, "範囲外は None");
    }

    #[test]
    fn the_park_branch_wins_unless_a_reentry_flag_is_present() {
        let parked = view(
            ExecutionStatus::Running,
            "stage-1",
            Some("stage-1"),
            [
                CheckboxState::Completed,
                CheckboxState::InProgress,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
            [PlanActionView::Execute; 4],
        );
        assert_eq!(
            parked.next_decision(&NextRequest::default()),
            NextDecision::Parked {
                stage: parked.cursor()
            }
        );
        assert_eq!(
            parked.next_decision(&NextRequest::new(true, false, false)),
            NextDecision::UnparkThenResume
        );
        // reentry フラグは park ガードを外す — 以後は通常の分岐へ落ちる。
        assert_eq!(
            parked.next_decision(&NextRequest::new(false, true, false)),
            NextDecision::RunStage {
                stage: parked.cursor(),
                gate: true
            }
        );
    }

    #[test]
    fn resume_and_free_text_route_before_the_status_check() {
        let held = running("stage-1", ALL_PENDING);
        assert_eq!(
            held.next_decision(&NextRequest::new(true, false, false)),
            NextDecision::ResumeMenu
        );
        assert_eq!(
            held.next_decision(&NextRequest::new(false, false, true)),
            NextDecision::NewWorkRouting
        );
        // resume は自由記述より優先する (BR3.1 の順)。
        assert_eq!(
            held.next_decision(&NextRequest::new(true, false, true)),
            NextDecision::ResumeMenu
        );
    }

    #[test]
    fn a_completed_workflow_stops_the_loop() {
        let done = view(
            ExecutionStatus::Completed,
            "stage-3",
            None,
            [CheckboxState::Completed; 4],
            [PlanActionView::Execute; 4],
        );
        assert_eq!(
            done.next_decision(&NextRequest::default()),
            NextDecision::Done
        );
    }

    #[test]
    fn an_in_flight_cursor_runs_that_very_stage_with_its_gate() {
        let at_init = running(
            "stage-0",
            [
                CheckboxState::InProgress,
                CheckboxState::Pending,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
        );
        assert_eq!(
            at_init.next_decision(&NextRequest::default()),
            NextDecision::RunStage {
                stage: at_init.cursor(),
                gate: false
            }
        );
        let gated = running(
            "stage-2",
            [
                CheckboxState::Completed,
                CheckboxState::Completed,
                CheckboxState::AwaitingApproval,
                CheckboxState::Pending,
            ],
        );
        assert_eq!(
            gated.next_decision(&NextRequest::default()),
            NextDecision::RunStage {
                stage: gated.cursor(),
                gate: true
            }
        );
    }

    #[test]
    fn a_finished_cursor_advances_to_the_next_in_scope_stage() {
        let held = view(
            ExecutionStatus::Running,
            "stage-1",
            None,
            [
                CheckboxState::Completed,
                CheckboxState::Completed,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
            [
                PlanActionView::Execute,
                PlanActionView::Execute,
                PlanActionView::Skip,
                PlanActionView::Execute,
            ],
        );
        assert_eq!(
            held.next_decision(&NextRequest::default()),
            NextDecision::RunStage {
                stage: held.stage_index(3).unwrap(),
                gate: true
            }
        );
        // 後続の in-scope が無ければ完了。
        let last = view(
            ExecutionStatus::Running,
            "stage-3",
            None,
            [CheckboxState::Completed; 4],
            [PlanActionView::Execute; 4],
        );
        assert_eq!(
            last.next_decision(&NextRequest::default()),
            NextDecision::Done
        );
    }

    #[test]
    fn an_effective_skip_under_the_cursor_is_an_inconsistency_not_a_run_stage() {
        for marker in SKIP_PRECONDITION {
            let recoverable = view(
                ExecutionStatus::Running,
                "stage-1",
                None,
                [
                    CheckboxState::Completed,
                    marker,
                    CheckboxState::Pending,
                    CheckboxState::Pending,
                ],
                [
                    PlanActionView::Execute,
                    PlanActionView::Skip,
                    PlanActionView::Execute,
                    PlanActionView::Execute,
                ],
            );
            assert_eq!(
                recoverable.next_decision(&NextRequest::default()),
                NextDecision::RecoverSkipInconsistency {
                    stage: recoverable.cursor(),
                    checkbox: marker
                }
            );
        }
        // 前提集合の外 (pending / awaiting-approval) は復旧経路が無い。
        for marker in [CheckboxState::Pending, CheckboxState::AwaitingApproval] {
            let stuck = view(
                ExecutionStatus::Running,
                "stage-1",
                None,
                [
                    CheckboxState::Completed,
                    marker,
                    CheckboxState::Pending,
                    CheckboxState::Pending,
                ],
                [
                    PlanActionView::Execute,
                    PlanActionView::Skip,
                    PlanActionView::Execute,
                    PlanActionView::Execute,
                ],
            );
            assert_eq!(
                stuck.next_decision(&NextRequest::default()),
                NextDecision::InconsistentSkip {
                    stage: stuck.cursor(),
                    checkbox: marker
                }
            );
        }
    }

    #[test]
    fn the_state_binding_moves_whenever_the_read_model_moves() {
        let held = running(
            "stage-1",
            [
                CheckboxState::Completed,
                CheckboxState::InProgress,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
        );
        assert_eq!(held.state_binding(), held.state_binding(), "決定的");

        // カーソルが動けば束縛も動く。
        let moved_cursor = running(
            "stage-2",
            [
                CheckboxState::Completed,
                CheckboxState::InProgress,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
        );
        assert_ne!(held.state_binding(), moved_cursor.state_binding());

        // マーカーが動けば束縛も動く。
        let moved_marker = running(
            "stage-1",
            [
                CheckboxState::Completed,
                CheckboxState::AwaitingApproval,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
        );
        assert_ne!(held.state_binding(), moved_marker.state_binding());

        // 実効プラン (recompose の結果) が動けば束縛も動く。
        let moved_plan = view(
            ExecutionStatus::Running,
            "stage-1",
            None,
            [
                CheckboxState::Completed,
                CheckboxState::InProgress,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
            [
                PlanActionView::Execute,
                PlanActionView::Execute,
                PlanActionView::Skip,
                PlanActionView::Execute,
            ],
        );
        assert_ne!(held.state_binding(), moved_plan.state_binding());

        // Status・park・最終更新時刻も素材である。
        let completed = view(
            ExecutionStatus::Completed,
            "stage-1",
            None,
            [
                CheckboxState::Completed,
                CheckboxState::InProgress,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
            [PlanActionView::Execute; 4],
        );
        assert_ne!(held.state_binding(), completed.state_binding());
        let parked = view(
            ExecutionStatus::Running,
            "stage-1",
            Some("stage-1"),
            [
                CheckboxState::Completed,
                CheckboxState::InProgress,
                CheckboxState::Pending,
                CheckboxState::Pending,
            ],
            [PlanActionView::Execute; 4],
        );
        assert_ne!(held.state_binding(), parked.state_binding());
        let stamped = ExecutionStateView::new(
            ScopeSlugView::parse("classic").unwrap(),
            ExecutionStatus::Running,
            "stage-1",
            None,
            "2026-08-30T00:00:00Z",
            held.stages().to_vec(),
        )
        .unwrap();
        assert_ne!(held.state_binding(), stamped.state_binding());
    }

    // ---- PBT: 先読みの最小性 (b26 段階 2 でコマンド側から移設) ----
    //
    // 旧・集約側の同名性質は**コマンド列を駆動して**状態を作っていたが、クエリ側に集約も
    // コマンドも無いので、リードモデルの観測そのものを生成する。行の到達可能性は問わない —
    // 状態ファイルは手編集されうるし、ビューはパーサが読めた観測をそのまま受け取るからである
    // (コマンド経由では作れない盤面もここでは正当な入力になる)。
    //
    // 掃く空間は旧生成器 (`synthetic_stages` — 削除済み、HEAD の
    // `command/domain/src/orchestration/intent_execution.rs` に原型) に合わせてある:
    // 行数 1〜8 (旧 2〜8)、**initialization 行数 1〜3** (`min(n, 3)` で頭打ち)、
    // 各行の実効プランは Execute / Skip の任意。
    // 旧生成器との差は 2 つで、いずれもクエリ側で**広い**方向である:
    // (a) 1 行の盤面も含む — リードモデルとして表現可能である
    // (b) initialization 行も Skip になりうる — 旧生成器は静的計画を作る都合で常に Execute
    //     だったが、行末トークンは投影が書くものであり、ビューに Execute を強制する不変条件は
    //     無い (手編集された状態ファイルはこの形を取りうる)

    /// 6 値の checkbox マーカー。
    ///
    /// 分類 (in-flight / finished) の**判断**は [`CheckboxState`] が述語で持つ (tell-don't-ask)。
    /// ここは判断ではなく生成器なので全変種を並べる。
    fn arb_checkbox() -> impl Strategy<Value = CheckboxState> {
        prop_oneof![
            Just(CheckboxState::Pending),
            Just(CheckboxState::InProgress),
            Just(CheckboxState::AwaitingApproval),
            Just(CheckboxState::Revising),
            Just(CheckboxState::Completed),
            Just(CheckboxState::Skipped),
        ]
    }

    /// 行末の実効プラン。
    fn arb_plan() -> impl Strategy<Value = PlanActionView> {
        prop_oneof![Just(PlanActionView::Execute), Just(PlanActionView::Skip)]
    }

    /// 任意のリードモデル観測から組んだビューと、その **initialization 行数**。
    ///
    /// 行数を対で返すのは、ゲート境界の期待値を [`ExecutionStateView::is_gated`] を呼び返さずに
    /// 出すためである — 同じ関数どうしで突き合わせると恒真アサートになる。
    fn arb_view() -> impl Strategy<Value = (usize, ExecutionStateView)> {
        (1usize..=8)
            .prop_flat_map(|n| {
                (
                    Just(n),
                    // initialization は先頭から連続する 1〜3 行 (旧生成器と同じ頭打ち)。
                    1usize..=n.min(3),
                    proptest::collection::vec(arb_checkbox(), n),
                    proptest::collection::vec(arb_plan(), n),
                    0..n,
                    proptest::option::of(0..n),
                    prop_oneof![
                        Just(ExecutionStatus::Running),
                        Just(ExecutionStatus::Completed)
                    ],
                )
            })
            .prop_map(|(n, init, markers, plans, cursor, parked, status)| {
                let stages = (0..n)
                    .map(|index| {
                        let phase = if index < init {
                            PhaseView::Initialization
                        } else {
                            PhaseView::Inception
                        };
                        row(
                            &format!("stage-{index}"),
                            phase,
                            markers[index],
                            plans[index],
                        )
                    })
                    .collect();
                let parked_slug = parked.map(|p| format!("stage-{p}"));
                let view = ExecutionStateView::new(
                    ScopeSlugView::parse("classic").unwrap(),
                    status,
                    &format!("stage-{cursor}"),
                    parked_slug.as_deref(),
                    "pbt",
                    stages,
                )
                .unwrap();
                (init, view)
            })
    }

    proptest! {
        /// 先読みが名指しするステージは、カーソルより後ろで**最初**の in-scope 行である
        /// (読み飛ばしの最小性)。無ければ `Done` に落ちる。
        ///
        /// カーソル自身を指す `RunStage` は先読みではない (cursor が in-flight のときの
        /// 「そのステージを走らせろ」) ので対象外。`Done` 側は、先読みまで到達する前提
        /// (受理可能 ∧ カーソルが in-flight でない) が成り立つときだけ主張する — park 中・
        /// 完了済み・不整合の `Done` は別分岐であり、この性質の射程ではない。
        #[test]
        fn the_lookahead_target_is_the_first_in_scope_stage_in_document_order(
            (init, view) in arb_view(),
        ) {
            let cursor = view.cursor();
            let cursor_in_flight = view
                .checkbox(cursor)
                .is_some_and(CheckboxState::is_in_flight);
            match view.next_decision(&NextRequest::default()) {
                NextDecision::RunStage { stage, gate } if stage != cursor => {
                    prop_assert!(stage.to_usize() > cursor.to_usize());
                    for value in (cursor.to_usize() + 1)..stage.to_usize() {
                        let earlier = view.stage_index(value).unwrap();
                        prop_assert!(!view.is_in_scope(earlier), "先に in-scope があった");
                    }
                    prop_assert!(view.is_in_scope(stage));
                    // ゲート付きかは行の phase から決まる (BR1.3)。生成器は先頭 `init` 行を
                    // initialization に置くので、境界を位置から**独立に**出す —
                    // `view.is_gated()` と突き合わせると同じ関数どうしの恒真になる。
                    prop_assert_eq!(gate, stage.to_usize() >= init);
                }
                NextDecision::Done if view.accepts_commands() && !cursor_in_flight => {
                    for value in (cursor.to_usize() + 1)..view.stage_count() {
                        let later = view.stage_index(value).unwrap();
                        prop_assert!(!view.is_in_scope(later), "先読みできる行が残っていた");
                    }
                }
                _ => {}
            }
        }
    }
}
