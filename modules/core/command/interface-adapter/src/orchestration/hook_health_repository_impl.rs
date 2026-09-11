//! HookHealth集約を本家のSQLite・メモリストアへ保存するRepository。
use super::dto::{HookHealthAggregateKeyDto, HookHealthDto, HookHealthEventDto};
use super::store_failure::io_kind_of_source;
use core_command_domain::workspace::{HookHealth, HookHealthEvent, HookHealthId, StorePath};
use core_command_use_case::orchestration::{HookHealthRepository, RepositoryError};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreReadError, EventStoreWriteError};
use event_store_adapter_rs::{EventStoreForMemory, EventStoreForSqlite};
use std::io::ErrorKind;
const MANIFEST: &str = "hook-health-event/1";
/// SQLiteの格納先。型引数はアダプタ所有のDTO。
pub type HookHealthSqliteStore =
    EventStoreForSqlite<HookHealthAggregateKeyDto, HookHealthDto, HookHealthEventDto>;
/// 本家のメモリ格納先。同じRepository本体を使う。
pub type HookHealthMemoryStore =
    EventStoreForMemory<HookHealthAggregateKeyDto, HookHealthDto, HookHealthEventDto>;
/// HookHealthのストアを所有する。
#[derive(Debug)]
pub struct HookHealthRepositoryImpl<S> {
    store: S,
    location: Option<StorePath>,
}
impl<S> HookHealthRepositoryImpl<S> {
    const fn new(store: S, location: Option<StorePath>) -> Self {
        Self { store, location }
    }
}
impl<S: Clone> HookHealthRepositoryImpl<S> {
    /// 同じ格納先を共有するRepositoryを開き直す。
    #[must_use]
    pub fn reopened(&self) -> Self {
        Self::new(self.store.clone(), self.location.clone())
    }
}
impl HookHealthRepositoryImpl<HookHealthMemoryStore> {
    /// 本家のメモリストアで同じ保存・再構成処理を使用する。
    #[must_use]
    pub fn in_memory() -> Self {
        Self::new(HookHealthMemoryStore::new(), None)
    }
}
impl HookHealthRepositoryImpl<HookHealthSqliteStore> {
    /// 共有SQLiteを開く。初期化は呼出側のワークスペース境界が行う。
    /// # Errors
    /// ストアを開けない場合。
    pub fn open(path: &StorePath) -> Result<Self, RepositoryError<HookHealthId>> {
        let store =
            HookHealthSqliteStore::new(path.as_path()).map_err(|e| Self::io(e, Some(path)))?;
        Ok(Self::new(store, Some(path.clone())))
    }
}
impl<S> HookHealthRepositoryImpl<S>
where
    S: EventStore<AID = HookHealthAggregateKeyDto, A = HookHealthDto, P = HookHealthEventDto>,
{
    fn io(error: EventStoreWriteError, path: Option<&StorePath>) -> RepositoryError<HookHealthId> {
        match error {
            EventStoreWriteError::IOError(source) => RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: path.map(|p| p.as_path().to_path_buf()),
            },
            _ => RepositoryError::Io {
                kind: ErrorKind::Other,
                path: path.map(|p| p.as_path().to_path_buf()),
            },
        }
    }
    fn read_error(&self, error: EventStoreReadError) -> RepositoryError<HookHealthId> {
        match error {
            EventStoreReadError::IOError(source) => RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: self.location.as_ref().map(|p| p.as_path().to_path_buf()),
            },
            EventStoreReadError::DeserializationError(_) => RepositoryError::Io {
                kind: ErrorKind::InvalidData,
                path: self.location.as_ref().map(|p| p.as_path().to_path_buf()),
            },
            EventStoreReadError::OtherError(_) => RepositoryError::Io {
                kind: ErrorKind::Other,
                path: self.location.as_ref().map(|p| p.as_path().to_path_buf()),
            },
        }
    }
}
impl<S> HookHealthRepository for HookHealthRepositoryImpl<S>
where
    S: EventStore<AID = HookHealthAggregateKeyDto, A = HookHealthDto, P = HookHealthEventDto>,
{
    async fn find_by_id(
        &self,
        id: &HookHealthId,
    ) -> Result<HookHealth, RepositoryError<HookHealthId>> {
        let key = HookHealthAggregateKeyDto::of(id);
        let snapshot = self
            .store
            .get_latest_snapshot_by_id(&key)
            .await
            .map_err(|e| self.read_error(e))?
            .ok_or_else(|| RepositoryError::NotFound { id: id.clone() })?;
        let base = snapshot
            .aggregate()
            .to_domain()
            .map_err(|source| RepositoryError::Corrupt {
                id: id.clone(),
                seq_nr: None,
                source: Box::new(source),
            })?;
        if base.id() != id {
            return Err(RepositoryError::Corrupt {
                id: id.clone(),
                seq_nr: Some(snapshot.seq_nr()),
                source: Box::new(std::io::Error::other(
                    "hook health snapshot identity mismatch",
                )),
            });
        }
        let first = base
            .seq_nr()
            .checked_add(1)
            .ok_or_else(|| RepositoryError::Corrupt {
                id: id.clone(),
                seq_nr: None,
                source: Box::new(std::io::Error::other("sequence exhausted")),
            })?;
        let delta = self
            .store
            .get_events_by_id_since_seq_nr(&key, first)
            .await
            .map_err(|e| self.read_error(e))?;
        let mut events = Vec::with_capacity(delta.len());
        for envelope in delta {
            if envelope.manifest() != MANIFEST || envelope.aggregate_id() != &key {
                return Err(RepositoryError::Corrupt {
                    id: id.clone(),
                    seq_nr: Some(envelope.seq_nr()),
                    source: Box::new(std::io::Error::other("hook health manifest mismatch")),
                });
            }
            let event =
                envelope
                    .payload()
                    .to_domain()
                    .map_err(|source| RepositoryError::Corrupt {
                        id: id.clone(),
                        seq_nr: Some(envelope.seq_nr()),
                        source: Box::new(source),
                    })?;
            events.push((event, envelope.seq_nr(), *envelope.occurred_at()));
        }
        Ok(HookHealth::replay(base, events).with_version(snapshot.version()))
    }
    async fn store(
        &mut self,
        event: &HookHealthEvent,
        aggregate: &HookHealth,
    ) -> Result<(), RepositoryError<HookHealthId>> {
        if event.aggregate_id() != aggregate.id() {
            return Err(RepositoryError::Corrupt {
                id: aggregate.id().clone(),
                seq_nr: Some(aggregate.seq_nr()),
                source: Box::new(std::io::Error::other("foreign hook health write")),
            });
        }
        let key = HookHealthAggregateKeyDto::of(aggregate.id());
        let envelope = EventEnvelope::new(
            key,
            aggregate.seq_nr(),
            aggregate.observed_at(),
            HookHealthEventDto::of(event),
        )
        .with_manifest(MANIFEST);
        match self
            .store
            .persist_event_and_snapshot(envelope, HookHealthDto::of(aggregate), aggregate.version())
            .await
        {
            Ok(()) => Ok(()),
            Err(EventStoreWriteError::OptimisticLockError(_)) => Err(RepositoryError::Conflict {
                expected: aggregate.version(),
                actual: self
                    .store
                    .get_latest_snapshot_by_id(&HookHealthAggregateKeyDto::of(aggregate.id()))
                    .await
                    .ok()
                    .flatten()
                    .map_or(0, |snapshot| snapshot.version()),
            }),
            Err(error) => Err(Self::io(error, self.location.as_ref())),
        }
    }
}

#[cfg(test)]
#[path = "hook_health_repository_impl_tests.rs"]
mod tests;
