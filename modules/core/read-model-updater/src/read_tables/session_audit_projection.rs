//! `read_session_audit` の行を組む投影 — セッション監査の保存履歴を通知IDごとの
//! [`SessionAuditRow`] へ写す。

use std::collections::{BTreeMap, BTreeSet};

use core_command_domain::workspace::{SessionAudit, SessionAuditId};

use super::ReadTablesError;
use crate::orchestration::{SessionAuditRow, SessionJournalEntry};

/// 保存履歴を集約で直接再構成し、通知ごとの行に写す。
///
/// # Errors
///
/// 先頭の保存事実が欠けている場合。
///
/// # Panics
///
/// 通番または通知IDが矛盾する壊れた履歴。
pub(super) fn rows(
    entries: &[SessionJournalEntry],
) -> Result<Vec<SessionAuditRow>, ReadTablesError> {
    let mut aggregates: BTreeMap<SessionAuditId, SessionAudit> = BTreeMap::new();
    let mut observations = BTreeSet::new();
    let mut rows = Vec::new();
    for entry in entries {
        let event = entry.event();
        let id = event.aggregate_id();
        if let Some(aggregate) = aggregates.get_mut(id) {
            aggregate.apply_event(event, entry.seq_nr(), *entry.occurred_at());
        } else {
            if entry.seq_nr() != 1 {
                return Err(ReadTablesError::MissingGenesis {
                    aggregate_id: id.to_string(),
                });
            }
            let aggregate = SessionAudit::new(
                id.clone(),
                event.target().clone(),
                event.record().clone(),
                event.observation_id().clone(),
                entry.seq_nr(),
                0,
                *entry.occurred_at(),
            )
            .map_err(|_| ReadTablesError::MissingGenesis {
                aggregate_id: id.to_string(),
            })?;
            aggregates.insert(id.clone(), aggregate);
        }
        assert!(
            observations.insert(event.observation_id().clone()),
            "duplicate SessionAudit observation"
        );
        rows.push(SessionAuditRow::new(
            event.observation_id().to_string(),
            id.to_string(),
            event.target().relative_directory(),
            event.record().kind().as_str().into(),
        ));
    }
    Ok(rows)
}
