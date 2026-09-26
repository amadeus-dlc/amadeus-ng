//! `SteeringPlanRow` — `read_steering_plan` の 1 行 (フェーズ 1 つぶんの配信計画)。

/// `read_steering_plan` の 1 行。主キーは 1 列 `id` (自然キー `phase` から導いた代理キー)。
/// `read_run_stage.steering_plan_id` と `read_steering_part.steering_plan_id` がこの値を指す。
///
/// **束は phase の関数である** — ステージの `rules_in_context` は束の選択に使わない
/// (設計 §0 の調査事実)。したがって行はフェーズごとに 1 本で、実行にも scope にも依らない。
///
/// `as_of` 列を持たないのは、この面が**参照入力由来**でジャーナルの走査位置と無関係だから
/// である。いつ時点かを名乗るのは `source_digest` の役目であり、それが変わらない限り行は
/// 書き替わらない。`source_digest` はスナップショット全体の性質なので行型には持たせず、
/// DAO ([`super::SteeringPlanDao::replace`]) が全行へ同じ値を書く (`as_of` と同じ流儀)。
///
/// 行は値を運ぶだけである。規則束から行を組む投影は
/// [`crate::read_tables::SteeringTables::pack`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringPlanRow {
    id: String,
    phase: String,
    bundle_digest: String,
    part_count: usize,
    delivered_paths: String,
}

impl SteeringPlanRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        phase: String,
        bundle_digest: String,
        part_count: usize,
        delivered_paths: String,
    ) -> Self {
        Self {
            id,
            phase,
            bundle_digest,
            part_count,
            delivered_paths,
        }
    }

    /// 主キー — 自然キー `phase` から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// フェーズの綴り (`PhaseId::as_str`)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// ルール束のダイジェスト (`sha256:` 前置 — `load-steering` の `bundle` にそのまま出る値)。
    #[must_use]
    pub fn bundle_digest(&self) -> &str {
        &self.bundle_digest
    }

    /// パート総数 (0 = 空計画 = bare run-stage)。
    #[must_use]
    pub const fn part_count(&self) -> usize {
        self.part_count
    }

    /// 配信済みルールのパス台帳の 1 行 JSON 配列 (読み順・重複除去)。
    #[must_use]
    pub fn delivered_paths(&self) -> &str {
        &self.delivered_paths
    }
}
