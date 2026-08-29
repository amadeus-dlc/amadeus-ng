//! 集約の写しの永続化 DTO — スナップショット行 `payload` 列のバイト形。

use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecution, IntentExecutionId, IntentExecutionSnapshot, IntentExecutionSnapshotBuilder,
    IntentId,
};
use serde::{Deserialize, Serialize};

use super::wire_error::WireDecodeError;
use super::wire_vocabulary::{
    autonomy_of, autonomy_spelling, checkbox_of, checkbox_spelling, plan_action_of,
    plan_action_spelling, status_of, status_spelling,
};

/// スナップショット行の形。**フィールド名と並びが契約**である。
///
/// 楽観 version は載らない — 版数の正本は本家 v3 の `SnapshotEnvelope::version()` (行の列) で
/// あり、`payload` 列は純粋なドメイン内容だけを持つ (ADR-010)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireSnapshot {
    id: String,
    intent_id: String,
    overlay: Vec<String>,
    checkbox: Vec<String>,
    cursor: usize,
    status: String,
    parked_at: Option<usize>,
    autonomy: String,
    approved: Vec<bool>,
    revision_count: Vec<u32>,
    seq_nr: usize,
    last_updated_at: DateTime<Utc>,
}

impl WireSnapshot {
    /// 集約の写しを読んで DTO を組む (書き)。
    #[must_use]
    pub fn of(execution: &IntentExecution) -> WireSnapshot {
        let snapshot: IntentExecutionSnapshot = execution.snapshot();
        WireSnapshot {
            id: snapshot.id().as_str().to_string(),
            intent_id: snapshot.intent_id().as_str().to_string(),
            overlay: snapshot
                .overlay()
                .iter()
                .map(|action| plan_action_spelling(*action).to_string())
                .collect(),
            checkbox: snapshot
                .checkbox()
                .iter()
                .map(|state| checkbox_spelling(*state).to_string())
                .collect(),
            cursor: snapshot.cursor(),
            status: status_spelling(snapshot.status()).to_string(),
            parked_at: snapshot.parked_at(),
            autonomy: autonomy_spelling(snapshot.autonomy()).to_string(),
            approved: snapshot.approved().to_vec(),
            revision_count: snapshot.revision_count().to_vec(),
            seq_nr: snapshot.seq_nr(),
            last_updated_at: *snapshot.last_updated_at(),
        }
    }

    /// 検査点 (`IntentExecution::from_snapshot`) を通して集約へ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed`、実行時不変条件を破る写しは
    /// `InvariantViolation` を返す。
    pub fn to_domain(&self) -> Result<IntentExecution, WireDecodeError> {
        let overlay = self
            .overlay
            .iter()
            .map(|raw| plan_action_of(raw, "overlay"))
            .collect::<Result<Vec<_>, WireDecodeError>>()?;
        let checkbox = self
            .checkbox
            .iter()
            .map(|raw| checkbox_of(raw))
            .collect::<Result<Vec<_>, WireDecodeError>>()?;
        let snapshot = IntentExecutionSnapshotBuilder::new(
            IntentExecutionId::parse(&self.id)
                .map_err(|_| WireDecodeError::malformed("id", self.id.clone()))?,
            IntentId::parse(&self.intent_id)
                .map_err(|_| WireDecodeError::malformed("intent_id", self.intent_id.clone()))?,
            overlay,
        )
        .checkbox(checkbox)
        .cursor(self.cursor)
        .status(status_of(&self.status)?)
        .parked_at(self.parked_at)
        .autonomy(autonomy_of(&self.autonomy)?)
        .approved(self.approved.clone())
        .revision_count(self.revision_count.clone())
        .seq_nr(self.seq_nr)
        .last_updated_at(self.last_updated_at)
        .build();
        IntentExecution::from_snapshot(snapshot).map_err(|_| WireDecodeError::InvariantViolation)
    }
}
