//! `NextAnswerView` — `read_next_answer` 1 行の写し (`next` の答え 1 つ)。

/// `read_next_answer` の 1 行 (自然キー `execution_id` × `request_kind` は UNIQUE 索引)。
///
/// # 判断はここに無い
///
/// `decision_kind` は書込側の集約が返した「次の一手」の答えを RMU が焼いた綴りであり、クエリ側は
/// それを**読むだけ**である (21 分岐のラダーはクエリ側に無い)。この型は述語も導出も持たず、
/// 綴りに従ってどう描くかはプレゼンタが決める (設計 §0-1 / §0-3)。
///
/// # 関連は FK 列で指す
///
/// 実行の現在地も run-stage の材料もこの行には**入っていない**。行が運ぶのは
/// [`NextAnswerView::execution_id`] と [`NextAnswerView::run_stage_id`] という 2 本の FK で、
/// それをたどって表ごとに引くのはユースケースの仕事である (オーナー裁定 2026-09-03 —
/// `coding-rules/cqrs-boundaries.md` 規則 6)。
///
/// `run_stage_id` が `None` なのは **RMU が「材料は無い」と書いた**ということである
/// (`NextAnswerRow::of` — 決定が run-stage のとき、かつ指す先が同じスナップショットに在る
/// ときだけ値を持つ)。`stage_slug` が非 NULL でも `run_stage_id` が NULL なことはある
/// (park の答えは位置を名乗るが run-stage ではない) ので、**slug から材料を引き直しては
/// ならない** — それは行に無い事実を作ることである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextAnswerView {
    decision_kind: String,
    stage_index: Option<u32>,
    stage_slug: Option<String>,
    gated: Option<bool>,
    checkbox: Option<String>,
    execution_id: String,
    run_stage_id: Option<String>,
}

impl NextAnswerView {
    /// 7 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        decision_kind: String,
        stage_index: Option<u32>,
        stage_slug: Option<String>,
        gated: Option<bool>,
        checkbox: Option<String>,
        execution_id: String,
        run_stage_id: Option<String>,
    ) -> NextAnswerView {
        NextAnswerView {
            decision_kind,
            stage_index,
            stage_slug,
            gated,
            checkbox,
            execution_id,
            run_stage_id,
        }
    }

    /// 答えの分類子 (`run-stage` … `inconsistent-skip`)。
    #[must_use]
    pub fn decision_kind(&self) -> &str {
        &self.decision_kind
    }

    /// 答えが名指すステージ位置 (名指さない分岐は `None`)。
    #[must_use]
    pub const fn stage_index(&self) -> Option<u32> {
        self.stage_index
    }

    /// 答えが名指すステージの slug。
    #[must_use]
    pub fn stage_slug(&self) -> Option<&str> {
        self.stage_slug.as_deref()
    }

    /// `run-stage` のときだけ在る — そのステージがゲート付きか。
    #[must_use]
    pub const fn gated(&self) -> Option<bool> {
        self.gated
    }

    /// 不整合 2 形のときだけ在る — 観測 checkbox の綴り。
    #[must_use]
    pub fn checkbox(&self) -> Option<&str> {
        self.checkbox.as_deref()
    }

    /// この答えを出した実行を指す FK。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// run-stage の材料を指す FK (材料が無ければ `None`)。
    #[must_use]
    pub fn run_stage_id(&self) -> Option<&str> {
        self.run_stage_id.as_deref()
    }
}
