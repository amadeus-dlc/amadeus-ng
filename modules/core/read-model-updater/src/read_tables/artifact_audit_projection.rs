//! `read_artifact_audit` の行を組む投影 — 成果物監査の保存履歴を [`ArtifactAuditRow`] へ写す。

use std::collections::BTreeMap;

use core_command_domain::workspace::{ArtifactAudit, ArtifactAuditId, ArtifactAuditRecord};

use super::ReadTablesError;
use crate::orchestration::{ArtifactAuditRow, ArtifactJournalEntry};

/// 保存履歴から集約を再構成し、各対象の最新観測を一行に写す。
///
/// # Errors
///
/// 先頭の有効な保存事実が欠けている場合。
///
/// # Panics
///
/// 保存履歴の対象または通番が集約の不変条件に反する場合。
pub(super) fn rows(
    entries: &[ArtifactJournalEntry],
) -> Result<Vec<ArtifactAuditRow>, ReadTablesError> {
    let mut aggregates: BTreeMap<ArtifactAuditId, ArtifactAudit> = BTreeMap::new();
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
            let observation = event.observation();
            let record = ArtifactAuditRecord::new(
                observation.file().into(),
                observation.tool().into(),
                observation.context().into(),
                observation.created(),
                *entry.occurred_at(),
            );
            let aggregate =
                ArtifactAudit::new(id.clone(), observation.target().clone(), 1, 0, record)
                    .map_err(|_| ReadTablesError::MissingGenesis {
                        aggregate_id: id.to_string(),
                    })?;
            aggregates.insert(id.clone(), aggregate);
        }
    }
    Ok(aggregates
        .into_values()
        .map(|aggregate| {
            ArtifactAuditRow::new(
                aggregate.id().to_string(),
                aggregate.target().relative_directory(),
                aggregate.last_file().into(),
                aggregate.last_tool().into(),
                aggregate.last_context().into(),
                aggregate.last_created(),
                aggregate.last_at(),
            )
        })
        .collect())
}
