//! セッション監査の通知IDごとの投影結果。
use super::ReadTablesError;
use crate::orchestration::SessionJournalEntry;
use core_command_domain::workspace::{SessionAudit, SessionAuditId};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq)]
/// 更新コマンドの戻り値を使わずに読む、保存通知の結果。
pub struct SessionAuditRow {
    id: String,
    aggregate_id: String,
    target: String,
    kind: String,
}
impl SessionAuditRow {
    /// 一行の全項目を構築する。
    #[must_use]
    pub const fn new(id: String, aggregate_id: String, target: String, kind: String) -> Self {
        Self {
            id,
            aggregate_id,
            target,
            kind,
        }
    }
    /// 保存履歴を集約で直接再構成し、通知ごとの行に写す。
    /// # Errors
    /// 先頭の保存事実が欠けている場合。
    /// # Panics
    /// 通番または通知IDが矛盾する壊れた履歴。
    pub fn project(entries: &[SessionJournalEntry]) -> Result<Vec<Self>, ReadTablesError> {
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
            rows.push(Self::new(
                event.observation_id().to_string(),
                id.to_string(),
                event.target().relative_directory(),
                event.record().kind().as_str().into(),
            ));
        }
        Ok(rows)
    }
    /// 通知ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 所有集約ID。
    #[must_use]
    pub fn aggregate_id(&self) -> &str {
        &self.aggregate_id
    }
    /// 監査帰属先。
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }
    /// 保存された監査種別。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
}
