//! 集約の永続化 DTO — スナップショット行 `payload` 列のバイト形。
//!
//! 型名は集約の具体名 (`IntentExecutionDto`) — スナップショットとは**ある時点の集約
//! そのもの**であり、Snapshot という別概念の型は作らない (オーナー裁定 2026-08-30)。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecution, IntentExecutionId, IntentId, ReviewAttempt, ReviewClosure, ReviewClosures,
    StageIndex, StageKey, StageSlot, StageSlots,
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
            pending: attempt.pending_iterations(),
            closed: attempt.closed().fold_left(Vec::new(), |mut rows, closure| {
                rows.push(ReviewClosureDto {
                    iteration: closure.iteration(),
                    verdict: review_verdict_spelling(closure.verdict()).to_string(),
                });
                rows
            }),
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
            self.pending.clone(),
            ReviewClosures::new(closed),
        ))
    }
}

/// 位置ごとの記録を行の 7 列へ展開する途中の受け皿 (この DTO の内部だけで使う)。
///
/// **ドメインの一級コレクションを行の形へ写す作業台**であって、リードモデルの表現ではない
/// (`coding-rules/cqrs-boundaries.md`)。1 度の走査で 7 列を同時に埋めるので、列の長さは
/// 構造的に揃う。
#[derive(Debug, Default)]
struct SlotColumns {
    stages: Vec<StageKeyDto>,
    overlay: Vec<String>,
    checkbox: Vec<String>,
    review_attempts: Vec<ReviewAttemptDto>,
    practices_affirmed: Vec<bool>,
    approved: Vec<bool>,
    revision_count: Vec<u32>,
}

impl SlotColumns {
    /// 1 位置ぶんの記録を 7 列へ足す。
    fn push(&mut self, slot: &StageSlot) {
        self.stages.push(StageKeyDto {
            slug: slot.key().slug().as_str().to_string(),
            phase: phase_spelling(slot.key().phase()).to_string(),
        });
        self.overlay
            .push(plan_action_spelling(slot.plan_action()).to_string());
        self.checkbox
            .push(checkbox_spelling(slot.checkbox()).to_string());
        self.review_attempts
            .push(ReviewAttemptDto::of(slot.review_attempt()));
        self.practices_affirmed.push(slot.practices_affirmed());
        self.approved.push(slot.approved());
        self.revision_count.push(slot.revision_count());
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
        // 位置ごとの記録を 1 度だけ畳んで 7 列へ展開する — 列の長さは同じ走査から出るので、
        // ここでずれることが構成上あり得ない (b51: 7 並列列 → `StageSlots`)。
        let columns = execution
            .slots()
            .fold_left(SlotColumns::default(), |mut columns, slot| {
                columns.push(slot);
                columns
            });
        IntentExecutionDto {
            id: execution.id().as_str().to_string(),
            intent_id: execution.intent_id().as_str().to_string(),
            stages: columns.stages,
            overlay: columns.overlay,
            checkbox: columns.checkbox,
            cursor: execution.cursor().to_usize(),
            status: status_spelling(execution.status()).to_string(),
            parked_at: execution.parked_at().map(StageIndex::to_usize),
            autonomy: autonomy_spelling(execution.autonomy()).to_string(),
            skeleton_stance: execution
                .skeleton_stance()
                .map(|stance| skeleton_stance_spelling(stance).to_string()),
            review_attempts: columns.review_attempts,
            practices_affirmed: columns.practices_affirmed,
            approved: columns.approved,
            revision_count: columns.revision_count,
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
        let count = self.stages.len();
        // 欄不在の行は「まだ 1 度も依頼していない」/「まだ昇格していない」— 添字帳の長さぶんの
        // 既定値で読む。ここで広げておくと、以降は添字で 7 列を突き合わせるだけになる。
        let review_attempts = if self.review_attempts.is_empty() {
            vec![ReviewAttemptDto::default(); count]
        } else {
            self.review_attempts.clone()
        };
        let practices_affirmed = if self.practices_affirmed.is_empty() {
            vec![false; count]
        } else {
            self.practices_affirmed.clone()
        };
        // 7 列を添字で合わせて位置ごとの記録へ畳む。列の長さが食い違う行は
        // `InvariantViolation` — 集約側では構成不能になった形なので、境界で断つ (層 (1))。
        let mut slots = Vec::with_capacity(count);
        for (index, key) in self.stages.iter().enumerate() {
            let column = |present: bool| {
                if present {
                    Ok(())
                } else {
                    Err(DtoDecodeError::InvariantViolation)
                }
            };
            let overlay = self
                .overlay
                .get(index)
                .ok_or(DtoDecodeError::InvariantViolation)?;
            let checkbox = self
                .checkbox
                .get(index)
                .ok_or(DtoDecodeError::InvariantViolation)?;
            let attempt = review_attempts
                .get(index)
                .ok_or(DtoDecodeError::InvariantViolation)?;
            let affirmed = practices_affirmed
                .get(index)
                .ok_or(DtoDecodeError::InvariantViolation)?;
            let approved = self
                .approved
                .get(index)
                .ok_or(DtoDecodeError::InvariantViolation)?;
            let revision_count = self
                .revision_count
                .get(index)
                .ok_or(DtoDecodeError::InvariantViolation)?;
            column(self.overlay.len() == count && self.checkbox.len() == count)?;
            column(self.approved.len() == count && self.revision_count.len() == count)?;
            column(review_attempts.len() == count && practices_affirmed.len() == count)?;
            slots.push(StageSlot::new(
                StageKey::new(
                    StageSlug::parse(&key.slug)
                        .map_err(|_| DtoDecodeError::malformed("stages.slug", &key.slug))?,
                    phase_of(&key.phase, "stages.phase")?,
                ),
                plan_action_of(overlay, "overlay")?,
                checkbox_of(checkbox)?,
                *approved,
                *revision_count,
                attempt.to_domain()?,
                *affirmed,
            ));
        }
        let slots = StageSlots::new(slots).map_err(|_| DtoDecodeError::InvariantViolation)?;
        IntentExecution::new(
            id,
            intent_id,
            slots,
            self.cursor,
            status_of(&self.status)?,
            self.parked_at,
            autonomy_of(&self.autonomy)?,
            self.skeleton_stance
                .as_deref()
                .map(|raw| skeleton_stance_of(raw, "skeleton_stance"))
                .transpose()?,
            self.last_gate_resolution_at,
            self.seq_nr,
            self.last_updated_at,
        )
        .map_err(|error| DtoDecodeError::malformed("intent_execution", error.reason()))
    }
}
