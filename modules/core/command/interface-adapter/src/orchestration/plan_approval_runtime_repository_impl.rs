//! ワークスペース全体の承認状態を管理する集約のRepository。
use super::dto::{PlanApprovalEventDto, PlanApprovalRuntimeDto, PlanApprovalRuntimeKeyDto};
use super::store_failure::io_kind_of_source;
use core_command_domain::orchestration::{
    PlanApprovalEvent, PlanApprovalRuntime, PlanApprovalRuntimeId,
};
use core_command_domain::workspace::StorePath;
use core_command_use_case::orchestration::{PlanApprovalRuntimeRepository, RepositoryError};
use event_store_adapter_rs::EventStoreForSqlite;
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreReadError, EventStoreWriteError};
use std::io::ErrorKind;
const EVENT_MANIFEST: &str = "plan-approval-event/1";
/// 共有イベントストアを所有する。既存spaceのストアは変更しない。
#[derive(Debug)]
pub struct PlanApprovalRuntimeRepositoryImpl {
    store: EventStoreForSqlite<
        PlanApprovalRuntimeKeyDto,
        PlanApprovalRuntimeDto,
        PlanApprovalEventDto,
    >,
    path: StorePath,
}
impl PlanApprovalRuntimeRepositoryImpl {
    /// 共有ストアを開く。診断からは呼ばず、更新の入口だけで使用する。
    /// # Errors
    /// 媒体を開けない場合。
    pub fn open(path: &StorePath) -> Result<Self, RepositoryError<PlanApprovalRuntimeId>> {
        let store =
            EventStoreForSqlite::new(path.as_path()).map_err(|error| RepositoryError::Io {
                kind: match &error {
                    EventStoreWriteError::IOError(source) => io_kind_of_source(source.as_ref()),
                    _ => ErrorKind::Other,
                },
                path: Some(path.as_path().to_path_buf()),
            })?;
        Ok(Self {
            store,
            path: path.clone(),
        })
    }
    fn read_error(&self, error: EventStoreReadError) -> RepositoryError<PlanApprovalRuntimeId> {
        match error {
            EventStoreReadError::IOError(source) => RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: Some(self.path.as_path().to_path_buf()),
            },
            EventStoreReadError::DeserializationError(source) => RepositoryError::Corrupt {
                id: PlanApprovalRuntimeId::Workspace,
                seq_nr: None,
                source,
            },
            EventStoreReadError::OtherError(_) => RepositoryError::Io {
                kind: ErrorKind::Other,
                path: Some(self.path.as_path().to_path_buf()),
            },
        }
    }
    fn corrupt(message: &str, sequence: Option<usize>) -> RepositoryError<PlanApprovalRuntimeId> {
        RepositoryError::Corrupt {
            id: PlanApprovalRuntimeId::Workspace,
            seq_nr: sequence,
            source: Box::new(std::io::Error::other(message)),
        }
    }
}
impl PlanApprovalRuntimeRepository for PlanApprovalRuntimeRepositoryImpl {
    async fn find_by_id(
        &self,
        id: &PlanApprovalRuntimeId,
    ) -> Result<PlanApprovalRuntime, RepositoryError<PlanApprovalRuntimeId>> {
        let key = PlanApprovalRuntimeKeyDto::of(id);
        let snapshot = self
            .store
            .get_latest_snapshot_by_id(&key)
            .await
            .map_err(|error| self.read_error(error))?;
        let Some(snapshot) = snapshot else {
            let events = self
                .store
                .get_events_by_id_since_seq_nr(&key, 1)
                .await
                .map_err(|error| self.read_error(error))?;
            return Err(if events.is_empty() {
                RepositoryError::NotFound { id: *id }
            } else {
                Self::corrupt("missing approval snapshot", None)
            });
        };
        let base = snapshot
            .aggregate()
            .to_domain()
            .map_err(|source| RepositoryError::Corrupt {
                id: *id,
                seq_nr: None,
                source: Box::new(source),
            })?;
        if base.id() != id {
            return Err(Self::corrupt("foreign approval snapshot", None));
        }
        let first_delta = base
            .seq_nr()
            .checked_add(1)
            .ok_or_else(|| Self::corrupt("approval sequence exhausted", None))?;
        let delta = self
            .store
            .get_events_by_id_since_seq_nr(&key, first_delta)
            .await
            .map_err(|error| self.read_error(error))?;
        let mut expected = base.seq_nr();
        let mut events = Vec::with_capacity(delta.len());
        for envelope in delta {
            expected = expected
                .checked_add(1)
                .ok_or_else(|| Self::corrupt("approval sequence exhausted", None))?;
            if envelope.seq_nr() != expected || envelope.manifest() != EVENT_MANIFEST {
                return Err(Self::corrupt(
                    "approval sequence or manifest mismatch",
                    Some(envelope.seq_nr()),
                ));
            }
            let event =
                envelope
                    .payload()
                    .to_domain()
                    .map_err(|source| RepositoryError::Corrupt {
                        id: *id,
                        seq_nr: Some(envelope.seq_nr()),
                        source: Box::new(source),
                    })?;
            if event.aggregate_id() != id {
                return Err(Self::corrupt(
                    "foreign approval event",
                    Some(envelope.seq_nr()),
                ));
            }
            events.push((event, envelope.seq_nr(), *envelope.occurred_at()));
        }
        Ok(PlanApprovalRuntime::replay(base, events).with_version(snapshot.version()))
    }
    async fn store(
        &mut self,
        event: &PlanApprovalEvent,
        aggregate: &PlanApprovalRuntime,
    ) -> Result<(), RepositoryError<PlanApprovalRuntimeId>> {
        if event.aggregate_id() != aggregate.id() {
            return Err(Self::corrupt(
                "foreign approval write",
                Some(aggregate.seq_nr()),
            ));
        }
        let key = PlanApprovalRuntimeKeyDto::of(aggregate.id());
        let envelope = EventEnvelope::new(
            key.clone(),
            aggregate.seq_nr(),
            aggregate.last_updated_at(),
            PlanApprovalEventDto::of(event),
        )
        .with_manifest(EVENT_MANIFEST);
        // 共有承認は毎回同じトランザクションで基底も更新する。保存の成功戻り値はunitのみ。
        match self
            .store
            .persist_event_and_snapshot(
                envelope,
                PlanApprovalRuntimeDto::of(aggregate),
                aggregate.version(),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(EventStoreWriteError::OptimisticLockError(_)) => {
                let current = self
                    .store
                    .get_latest_snapshot_by_id(&key)
                    .await
                    .map_err(|error| self.read_error(error))?;
                Err(RepositoryError::Conflict {
                    expected: aggregate.version(),
                    actual: current.map_or(0, |snapshot| snapshot.version()),
                })
            }
            Err(EventStoreWriteError::IOError(source)) => Err(RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: Some(self.path.as_path().to_path_buf()),
            }),
            Err(EventStoreWriteError::OtherError(_)) => Err(RepositoryError::Io {
                kind: ErrorKind::Other,
                path: Some(self.path.as_path().to_path_buf()),
            }),
            Err(error) => Err(RepositoryError::Corrupt {
                id: *aggregate.id(),
                seq_nr: Some(aggregate.seq_nr()),
                source: Box::new(error),
            }),
        }
    }
}

#[cfg(test)]
#[path = "plan_approval_runtime_repository_impl_tests.rs"]
mod tests;
