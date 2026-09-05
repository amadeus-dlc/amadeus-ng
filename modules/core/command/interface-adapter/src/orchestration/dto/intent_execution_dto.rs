//! 集約の永続化 DTO — スナップショット行 `payload` 列のバイト形。
//!
//! 型名は集約の具体名 (`IntentExecutionDto`) — スナップショットとは**ある時点の集約
//! そのもの**であり、Snapshot という別概念の型は作らない (オーナー裁定 2026-08-30)。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecution, IntentExecutionId, IntentId, ReviewAttempt, ReviewClosure, StageIndex,
    StageKey,
};
use core_command_domain::workflow_definition::StageSlug;
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{
    autonomy_of, autonomy_spelling, checkbox_of, checkbox_spelling, phase_of, phase_spelling,
    plan_action_of, plan_action_spelling, review_verdict_of, review_verdict_spelling,
    skeleton_stance_of, skeleton_stance_spelling, status_of, status_spelling,
};

/// スナップショット行の形。**フィールド名と並びが契約**である。
///
/// 楽観 version は載らない — 版数の正本は本家 v3 の `SnapshotEnvelope::version()` (行の列) で
/// あり、`payload` 列は純粋なドメイン内容だけを持つ (ADR-010)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentExecutionDto {
    id: String,
    intent_id: String,
    /// イベント適用の添字帳 (slug + phase) — 集約の自己完結 replay の材料 (issue #44)。
    stages: Vec<StageKeyDto>,
    overlay: Vec<String>,
    checkbox: Vec<String>,
    cursor: usize,
    status: String,
    parked_at: Option<usize>,
    autonomy: String,
    /// conductor が分類した walking-skeleton stance (`null` = 未記録)。
    ///
    /// 欄を持たない行は `None` で読む — 後方互換のための緩和ではなく、
    /// **「欄が無い = まだ分類していない」という正規の意味**である (b47 設計 §2)。
    #[serde(default)]
    skeleton_stance: Option<String>,
    /// ステージごとの現在のレビュー試行（ステージ順。b48 / B10）。
    ///
    /// 欄を持たない行は**全ステージ空**で読む — 後方互換のための緩和ではなく、
    /// 「欄が無い = まだ 1 度も依頼していない」という正規の意味である。長さの整合は
    /// 集約の完全コンストラクタが検査するので、欄不在のときだけ計画長へ広げる。
    #[serde(default)]
    review_attempts: Vec<ReviewAttemptDto>,
    /// ステージごとの現在の試行で昇格が成功したか（ステージ順。b49 / B10）。
    ///
    /// 欄を持たない行は**全ステージ false** で読む — 後方互換のための緩和ではなく、
    /// 「欄が無い = まだ昇格していない」という正規の意味である。長さの整合は集約の完全
    /// コンストラクタが検査するので、欄不在のときだけ計画長へ広げる。
    #[serde(default)]
    practices_affirmed: Vec<bool>,
    approved: Vec<bool>,
    revision_count: Vec<u32>,
    /// 直近のゲート解決の発生時刻 (`null` = まだ 1 度も解決していない。b50 / I11)。
    ///
    /// 欄を持たない行は `None` で読む — 後方互換のための緩和ではなく、
    /// 「欄が無い = まだ解決していない」という正規の意味である。
    #[serde(default)]
    last_gate_resolution_at: Option<DateTime<Utc>>,
    seq_nr: usize,
    last_updated_at: DateTime<Utc>,
}

/// 添字帳 1 行のワイヤ形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StageKeyDto {
    slug: String,
    phase: String,
}

/// レビュー試行 1 ステージ分のワイヤ形。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ReviewAttemptDto {
    requests: u32,
    pending: Vec<u32>,
    closed: Vec<ReviewClosureDto>,
}

/// 閉じた依頼 1 件のワイヤ形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReviewClosureDto {
    iteration: u32,
    verdict: String,
}

impl ReviewAttemptDto {
    /// ドメインの読取面から行の形を組む（書き）。
    fn of(attempt: &ReviewAttempt) -> ReviewAttemptDto {
        ReviewAttemptDto {
            requests: attempt.request_count(),
            pending: attempt.pending().iter().copied().collect(),
            closed: attempt
                .closed()
                .iter()
                .map(|closure| ReviewClosureDto {
                    iteration: closure.iteration(),
                    verdict: review_verdict_spelling(closure.verdict()).to_string(),
                })
                .collect(),
        }
    }

    /// ドメインの値オブジェクトへ戻す（読み）。
    fn to_domain(&self) -> Result<ReviewAttempt, DtoDecodeError> {
        let closed = self
            .closed
            .iter()
            .map(|closure| {
                Ok(ReviewClosure::new(
                    closure.iteration,
                    review_verdict_of(&closure.verdict, "review_attempts.closed.verdict")?,
                ))
            })
            .collect::<Result<Vec<_>, DtoDecodeError>>()?;
        Ok(ReviewAttempt::restored(
            self.requests,
            self.pending.iter().copied().collect(),
            closed,
        ))
    }
}

impl IntentExecutionDto {
    /// 集約の読取面からスナップショット行の形を組む (書き)。
    ///
    /// memento 型は経由しない (オーナー裁定 2026-08-30 — 集約と構造同一の写し型は複製で
    /// しかない)。issue #44 で `stages` (添字帳) が行に加わった — 旧形式 (`stages` なし) の
    /// 行は読めないが、互換処理は持たない: プレリリースのローカルストアであり、旧ストアは
    /// 作り直す (`coding-rules/no-backward-compatibility.md` — 使われない口を並立させない)。
    #[must_use]
    pub fn of(execution: &IntentExecution) -> IntentExecutionDto {
        let stages = 0..execution.stage_count();
        IntentExecutionDto {
            id: execution.id().as_str().to_string(),
            intent_id: execution.intent_id().as_str().to_string(),
            stages: execution
                .stage_keys()
                .iter()
                .map(|key| StageKeyDto {
                    slug: key.slug().as_str().to_string(),
                    phase: phase_spelling(key.phase()).to_string(),
                })
                .collect(),
            overlay: stages
                .clone()
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.effective_plan(stage))
                .map(|action| plan_action_spelling(action).to_string())
                .collect(),
            checkbox: stages
                .clone()
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.checkbox(stage))
                .map(|state| checkbox_spelling(state).to_string())
                .collect(),
            cursor: execution.cursor().to_usize(),
            status: status_spelling(execution.status()).to_string(),
            parked_at: execution.parked_at().map(StageIndex::to_usize),
            autonomy: autonomy_spelling(execution.autonomy()).to_string(),
            skeleton_stance: execution
                .skeleton_stance()
                .map(|stance| skeleton_stance_spelling(stance).to_string()),
            review_attempts: stages
                .clone()
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.review_attempt(stage))
                .map(ReviewAttemptDto::of)
                .collect(),
            practices_affirmed: stages
                .clone()
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.practices_affirmed(stage))
                .collect(),
            approved: stages
                .clone()
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.approved(stage))
                .collect(),
            revision_count: stages
                .filter_map(|value| execution.stage_index(value))
                .filter_map(|stage| execution.revision_count(stage))
                .collect(),
            last_gate_resolution_at: execution.last_gate_resolution_at(),
            seq_nr: execution.seq_nr(),
            last_updated_at: *execution.last_updated_at(),
        }
    }

    /// 行から集約へ戻す (読み — 集約の完全コンストラクタ [`IntentExecution::new`] を必ず通る)。
    ///
    /// # Errors
    ///
    /// 綴りの復号失敗・識別子の文法違反・集約不変条件の違反 (いずれも呼出側が `Corrupt` へ
    /// 写す — BR1.5)。
    pub fn to_domain(&self) -> Result<IntentExecution, DtoDecodeError> {
        let id = IntentExecutionId::parse(&self.id)
            .map_err(|_| DtoDecodeError::malformed("id", &self.id))?;
        let intent_id = IntentId::parse(&self.intent_id)
            .map_err(|_| DtoDecodeError::malformed("intent_id", &self.intent_id))?;
        let stage_keys = self
            .stages
            .iter()
            .map(|key| {
                Ok(StageKey::new(
                    StageSlug::parse(&key.slug)
                        .map_err(|_| DtoDecodeError::malformed("stages.slug", &key.slug))?,
                    phase_of(&key.phase, "stages.phase")?,
                ))
            })
            .collect::<Result<Vec<_>, DtoDecodeError>>()?;
        let overlay = self
            .overlay
            .iter()
            .map(|raw| plan_action_of(raw, "overlay"))
            .collect::<Result<Vec<_>, DtoDecodeError>>()?;
        let checkbox = self
            .checkbox
            .iter()
            .map(|raw| checkbox_of(raw))
            .collect::<Result<Vec<_>, DtoDecodeError>>()?;
        // 欄不在の行は「まだ 1 度も依頼していない」— 計画長ぶんの空試行で読む。
        let review_attempts = if self.review_attempts.is_empty() {
            vec![ReviewAttempt::default(); stage_keys.len()]
        } else {
            self.review_attempts
                .iter()
                .map(ReviewAttemptDto::to_domain)
                .collect::<Result<Vec<_>, DtoDecodeError>>()?
        };
        // 欄不在の行は「まだ昇格していない」— 計画長ぶんの false で読む。
        let practices_affirmed = if self.practices_affirmed.is_empty() {
            vec![false; stage_keys.len()]
        } else {
            self.practices_affirmed.clone()
        };
        IntentExecution::new(
            id,
            intent_id,
            stage_keys,
            overlay,
            checkbox,
            self.cursor,
            status_of(&self.status)?,
            self.parked_at,
            autonomy_of(&self.autonomy)?,
            self.skeleton_stance
                .as_deref()
                .map(|raw| skeleton_stance_of(raw, "skeleton_stance"))
                .transpose()?,
            review_attempts,
            practices_affirmed,
            self.approved.clone(),
            self.revision_count.clone(),
            self.last_gate_resolution_at,
            self.seq_nr,
            self.last_updated_at,
        )
        .map_err(|error| DtoDecodeError::malformed("intent_execution", error.reason()))
    }
}
