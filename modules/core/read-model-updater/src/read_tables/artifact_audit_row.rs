//! 成果物監査行。
#![allow(missing_docs)]
use super::ReadTablesError;
use crate::orchestration::ArtifactJournalEntry;
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{ArtifactAudit, ArtifactAuditId, ArtifactAuditRecord};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactAuditRow {
    id: String,
    target: String,
    file: String,
    tool: String,
    context: String,
    created: bool,
    occurred_at: DateTime<Utc>,
}
impl ArtifactAuditRow {
    /// 保存履歴から集約を再構成し、各対象の最新観測を一行に写す。
    /// # Errors
    /// 先頭の有効な保存事実が欠けている場合。
    /// # Panics
    /// 保存履歴の対象または通番が集約の不変条件に反する場合。
    pub fn project(entries: &[ArtifactJournalEntry]) -> Result<Vec<Self>, ReadTablesError> {
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
                Self::new(
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

    #[must_use]
    pub const fn new(
        id: String,
        target: String,
        file: String,
        tool: String,
        context: String,
        created: bool,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            target,
            file,
            tool,
            context,
            created,
            occurred_at,
        }
    }
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }
    #[must_use]
    pub fn tool(&self) -> &str {
        &self.tool
    }
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }
    #[must_use]
    pub const fn created(&self) -> bool {
        self.created
    }
    #[must_use]
    pub const fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}
