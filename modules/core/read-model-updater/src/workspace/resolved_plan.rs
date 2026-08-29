//! `ResolvedPlan` — 投影が参照する解決済み計画（`Started` が運んだステージ列）。
//!
//! # なぜ投影の「引数」なのか
//!
//! 表示属性（ステージ番号・表題・担当エージェント）は `Started` だけが運ぶ（オーナー裁定
//! 2026-08-29）。ところが差分投影は checkpoint 以降しか読まないので、`GateApproved` を描く
//! バッチに `Started` が入っているとは限らない。
//!
//! そこで計画は**投影核の引数**にした — リードモデルと同じ「渡されるデータ」である。取ってくる
//! のは取得ループの仕事であり、投影核は相変わらず `JournalReader`・接続・checkpoint を知らない
//! （`coding-rules/cqrs-boundaries.md` の二層構造は保たれる）。
//!
//! イベントを太らせる案（遷移イベントごとに表示属性を持たせる）を採らなかったのは、同じ事実が
//! ジャーナルに何度も転写され、`Started` の計画と食い違いうるからである。正本は 1 つでよい。

use core_command_domain::orchestration::{
    IntentExecutionEvent, StageDisplay, Started, WorkspaceScan,
};
use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageSlug};

/// 投影が参照する解決済み計画（文書順の全ステージ + 走査結果）。
///
/// 外から組み立てられるのは `Started` からだけである — 順序も表示属性も `Started` が確定した
/// 事実であり、投影が勝手に足したり並べ替えたりしてよいものではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlan {
    stages: Vec<PlannedStage>,
    scan: WorkspaceScan,
    scope: String,
    request: String,
}

/// 計画上の 1 ステージ（`StageEntry` から投影が要る分だけ写したもの）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedStage {
    slug: StageSlug,
    phase: PhaseId,
    plan_action: PlanAction,
    display: StageDisplay,
}

impl PlannedStage {
    /// ステージ slug。
    #[must_use]
    pub const fn slug(&self) -> &StageSlug {
        &self.slug
    }

    /// このステージのフェーズ。
    #[must_use]
    pub const fn phase(&self) -> PhaseId {
        self.phase
    }

    /// 静的グリッド由来の計画。
    #[must_use]
    pub const fn plan_action(&self) -> PlanAction {
        self.plan_action
    }

    /// 表示属性（番号・表題・担当）。
    #[must_use]
    pub const fn display(&self) -> &StageDisplay {
        &self.display
    }

    /// スコープ内か（`EXECUTE` のもの）。
    #[must_use]
    pub fn is_in_scope(&self) -> bool {
        self.plan_action == PlanAction::Execute
    }
}

impl ResolvedPlan {
    /// `Started` から計画を写す（唯一の構成関数）。
    #[must_use]
    pub fn of(started: &Started) -> ResolvedPlan {
        ResolvedPlan {
            stages: started
                .stages()
                .iter()
                .map(|entry| PlannedStage {
                    slug: entry.slug().clone(),
                    phase: entry.phase(),
                    plan_action: entry.plan_action(),
                    display: entry.display().clone(),
                })
                .collect(),
            scan: started.scan().clone(),
            scope: started.scope().to_string(),
            request: started.request().to_string(),
        }
    }

    /// ジャーナル行の列から `Started` を探して計画を組む（先頭の 1 件だけを見る）。
    ///
    /// 見つからなければ `None` — 呼出側（取得ループ）が「計画がまだ無い」として扱う。
    #[must_use]
    pub fn find_in(events: &[IntentExecutionEvent]) -> Option<ResolvedPlan> {
        events.iter().find_map(|event| match event {
            IntentExecutionEvent::Started(started) => Some(ResolvedPlan::of(started)),
            _ => None,
        })
    }

    /// 文書順の全ステージ。
    #[must_use]
    pub fn stages(&self) -> &[PlannedStage] {
        &self.stages
    }

    /// 走査結果（初期化 3 ステージの行の材料）。
    #[must_use]
    pub const fn scan(&self) -> &WorkspaceScan {
        &self.scan
    }

    /// 選択されたスコープ名（`**Scope**:` 行の材料）。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 人間の要求（`**Request**:` 行の材料）。
    #[must_use]
    pub fn request(&self) -> &str {
        &self.request
    }

    /// slug から計画上のステージを引く。
    #[must_use]
    pub fn find(&self, slug: &StageSlug) -> Option<&PlannedStage> {
        self.stages.iter().find(|stage| stage.slug() == slug)
    }

    /// slug から表示属性を引く。
    #[must_use]
    pub fn display_of(&self, slug: &StageSlug) -> Option<&StageDisplay> {
        self.find(slug).map(PlannedStage::display)
    }

    /// 指定ステージの**次にくるスコープ内ステージ**（`- **Next Stage**:` の材料）。
    #[must_use]
    pub fn next_in_scope_after(&self, slug: &StageSlug) -> Option<&PlannedStage> {
        let at = self.stages.iter().position(|stage| stage.slug() == slug)?;
        self.stages
            .iter()
            .skip(at.saturating_add(1))
            .find(|stage| stage.is_in_scope())
    }

    /// スコープ内ステージの件数（`- **Total Stages**:` の材料）。
    #[must_use]
    pub fn in_scope_count(&self) -> usize {
        self.stages
            .iter()
            .filter(|stage| stage.is_in_scope())
            .count()
    }

    /// スコープ内フェーズ（文書順・重複なし）。
    #[must_use]
    pub fn phases_in_scope(&self) -> Vec<PhaseId> {
        let mut seen = Vec::new();
        for stage in self.stages.iter().filter(|stage| stage.is_in_scope()) {
            if !seen.contains(&stage.phase()) {
                seen.push(stage.phase());
            }
        }
        seen
    }

    /// スコープ**外**フェーズ（文書順・重複なし — `PHASE_SKIPPED` の材料）。
    ///
    /// 1 ステージもスコープ内に無いフェーズだけを数える。同じフェーズに EXECUTE が 1 つでも
    /// あればそのフェーズは走るからである。
    #[must_use]
    pub fn phases_out_of_scope(&self) -> Vec<PhaseId> {
        let in_scope = self.phases_in_scope();
        let mut seen = Vec::new();
        for stage in &self.stages {
            let phase = stage.phase();
            if !in_scope.contains(&phase) && !seen.contains(&phase) {
                seen.push(phase);
            }
        }
        seen
    }

    /// あるフェーズのスコープ内ステージ件数（`**Stage count**:` の材料）。
    #[must_use]
    pub fn in_scope_count_of(&self, phase: PhaseId) -> usize {
        self.stages
            .iter()
            .filter(|stage| stage.is_in_scope() && stage.phase() == phase)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::{Intent, IntentId, StageEntry, StartRequest};
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, StageNumber, WorkflowDefinitionId,
    };

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("テストの slug は文法内")
    }

    fn entry(name: &str, number: &str, phase: PhaseId, action: PlanAction) -> StageEntry {
        StageEntry::new(
            slug(name),
            phase,
            action,
            false,
            StageDisplay::new(
                StageNumber::parse(number).expect("番号"),
                "Title",
                "orchestrator",
            )
            .expect("単一行"),
        )
    }

    fn started() -> Started {
        Started::new(
            Intent::from_material(
                IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7"),
                WorkflowDefinitionId::parse("claude").expect("定義 id"),
                DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
                StartRequest::new("classic", "/aidlc Build a small ordering service"),
                vec![
                    entry(
                        "state-init",
                        "0.3",
                        PhaseId::Initialization,
                        PlanAction::Execute,
                    ),
                    entry("intent-capture", "1.1", PhaseId::Ideation, PlanAction::Skip),
                    entry(
                        "practices-discovery",
                        "2.2",
                        PhaseId::Inception,
                        PlanAction::Execute,
                    ),
                    entry(
                        "requirements-analysis",
                        "2.3",
                        PhaseId::Inception,
                        PlanAction::Execute,
                    ),
                    entry("user-stories", "2.4", PhaseId::Inception, PlanAction::Skip),
                    entry(
                        "domain-design",
                        "2.6",
                        PhaseId::Inception,
                        PlanAction::Execute,
                    ),
                ],
                WorkspaceScan::new(
                    BrownfieldGreenfield::Greenfield,
                    "Unknown",
                    "Unknown",
                    "Unknown",
                )
                .expect("単一行"),
            )
            .expect("合成計画は Intent の不変条件を満たす"),
        )
    }

    fn plan() -> ResolvedPlan {
        ResolvedPlan::of(&started())
    }

    #[test]
    fn the_plan_copies_the_stage_list_the_started_event_resolved() {
        let plan = plan();
        assert_eq!(plan.stages().len(), 6);
        assert_eq!(plan.scope(), "classic");
        assert_eq!(plan.request(), "/aidlc Build a small ordering service");
        assert_eq!(plan.scan().project_type(), "Greenfield");
    }

    #[test]
    fn a_stage_is_found_by_its_slug_with_its_display() {
        let plan = plan();
        let found = plan.find(&slug("domain-design")).expect("計画上にある");
        assert_eq!(found.display().number().as_str(), "2.6");
        assert_eq!(found.phase(), PhaseId::Inception);
        assert_eq!(found.plan_action(), PlanAction::Execute);
        assert!(found.is_in_scope());
        let skipped = plan.find(&slug("user-stories")).expect("計画上にある");
        assert_eq!(skipped.plan_action(), PlanAction::Skip);
        assert!(!skipped.is_in_scope());
        assert_eq!(
            plan.display_of(&slug("domain-design"))
                .map(|d| d.number().as_str()),
            Some("2.6")
        );
        assert_eq!(plan.find(&slug("no-such-stage")), None);
    }

    #[test]
    fn the_next_stage_skips_over_the_ones_out_of_scope() {
        // `user-stories` は SKIP なので `requirements-analysis` の次は `domain-design`。
        let plan = plan();
        assert_eq!(
            plan.next_in_scope_after(&slug("requirements-analysis"))
                .map(|stage| stage.slug().as_str()),
            Some("domain-design")
        );
        assert_eq!(plan.next_in_scope_after(&slug("domain-design")), None);
        assert_eq!(plan.next_in_scope_after(&slug("no-such-stage")), None);
    }

    #[test]
    fn the_counts_only_see_the_stages_in_scope() {
        let plan = plan();
        assert_eq!(plan.in_scope_count(), 4);
        assert_eq!(plan.in_scope_count_of(PhaseId::Initialization), 1);
        assert_eq!(plan.in_scope_count_of(PhaseId::Inception), 3);
        assert_eq!(plan.in_scope_count_of(PhaseId::Ideation), 0);
    }

    #[test]
    fn a_phase_is_out_of_scope_only_when_none_of_its_stages_execute() {
        // ideation は 1 つも EXECUTE が無いのでスコープ外。inception は SKIP が混じるが
        // EXECUTE があるのでスコープ内である。
        let plan = plan();
        assert_eq!(
            plan.phases_in_scope(),
            [PhaseId::Initialization, PhaseId::Inception]
        );
        assert_eq!(plan.phases_out_of_scope(), [PhaseId::Ideation]);
    }

    #[test]
    fn the_plan_is_found_in_a_journal_batch_that_carries_the_genesis() {
        let events = vec![
            IntentExecutionEvent::Unparked,
            IntentExecutionEvent::Started(started()),
        ];
        assert_eq!(ResolvedPlan::find_in(&events), Some(plan()));
        assert_eq!(
            ResolvedPlan::find_in(&[IntentExecutionEvent::Unparked]),
            None
        );
        assert_eq!(ResolvedPlan::find_in(&[]), None);
    }
}
