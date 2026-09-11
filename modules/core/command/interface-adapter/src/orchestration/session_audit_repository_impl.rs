//! SessionAudit集約を共有SQLiteへ保存するRepository。
use super::dto::{SessionAuditDto, SessionAuditEventDto, SessionAuditKeyDto};
use super::store_failure::io_kind_of_source;
use core_command_domain::workspace::StorePath;
use core_command_domain::workspace::{SessionAudit, SessionAuditEvent, SessionAuditId};
use core_command_use_case::orchestration::{RepositoryError, SessionAuditRepository};
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreReadError, EventStoreWriteError};
use event_store_adapter_rs::{EventStoreForMemory, EventStoreForSqlite};
use std::io::ErrorKind;
const MANIFEST: &str = "session-audit-event/1";
/// 本家SQLiteストア。
pub type SessionAuditSqliteStore =
    EventStoreForSqlite<SessionAuditKeyDto, SessionAuditDto, SessionAuditEventDto>;
/// 本家memoryストア。SQLiteと同じRepository実装を通る。
pub type SessionAuditMemoryStore =
    EventStoreForMemory<SessionAuditKeyDto, SessionAuditDto, SessionAuditEventDto>;
/// SessionAuditのストアを所有する。
#[derive(Debug)]
pub struct SessionAuditRepositoryImpl<S = SessionAuditSqliteStore> {
    store: S,
    path: Option<StorePath>,
}
impl SessionAuditRepositoryImpl<SessionAuditSqliteStore> {
    /// 共有SQLiteを開く。初期化は呼出側のワークスペース境界が行う。
    /// # Errors
    /// ストアを開けない場合。
    pub fn open(path: &StorePath) -> Result<Self, RepositoryError<SessionAuditId>> {
        let store = SessionAuditSqliteStore::new(path.as_path()).map_err(|e| Self::io(e, path))?;
        Ok(Self::new(store, Some(path.clone())))
    }
}
impl SessionAuditRepositoryImpl<SessionAuditMemoryStore> {
    /// 揮発ストアで同一の保存・再構成・競合処理を使う。
    #[must_use]
    pub fn in_memory() -> Self {
        Self::new(SessionAuditMemoryStore::new(), None)
    }
}
impl<S> SessionAuditRepositoryImpl<S> {
    const fn new(store: S, path: Option<StorePath>) -> Self {
        Self { store, path }
    }
    fn io(error: EventStoreWriteError, path: &StorePath) -> RepositoryError<SessionAuditId> {
        match error {
            EventStoreWriteError::IOError(source) => RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: Some(path.as_path().to_path_buf()),
            },
            _ => RepositoryError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            },
        }
    }
    fn read_error(&self, error: EventStoreReadError) -> RepositoryError<SessionAuditId> {
        match error {
            EventStoreReadError::IOError(source) => RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: self.path.as_ref().map(|path| path.as_path().to_path_buf()),
            },
            EventStoreReadError::DeserializationError(_) => RepositoryError::Io {
                kind: ErrorKind::InvalidData,
                path: self.path.as_ref().map(|path| path.as_path().to_path_buf()),
            },
            EventStoreReadError::OtherError(_) => RepositoryError::Io {
                kind: ErrorKind::Other,
                path: self.path.as_ref().map(|path| path.as_path().to_path_buf()),
            },
        }
    }
}
impl<S> SessionAuditRepository for SessionAuditRepositoryImpl<S>
where
    S: EventStore<AID = SessionAuditKeyDto, A = SessionAuditDto, P = SessionAuditEventDto>,
{
    async fn find_by_id(
        &self,
        id: &SessionAuditId,
    ) -> Result<SessionAudit, RepositoryError<SessionAuditId>> {
        let key = SessionAuditKeyDto::of(id);
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
                seq_nr: Some(base.seq_nr()),
                source: Box::new(std::io::Error::other(
                    "session audit snapshot identity mismatch",
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
                    source: Box::new(std::io::Error::other("session audit manifest mismatch")),
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
        Ok(SessionAudit::replay(base, events).with_version(snapshot.version()))
    }
    async fn store(
        &mut self,
        event: &SessionAuditEvent,
        aggregate: &SessionAudit,
    ) -> Result<(), RepositoryError<SessionAuditId>> {
        if event.aggregate_id() != aggregate.id() {
            return Err(RepositoryError::Corrupt {
                id: aggregate.id().clone(),
                seq_nr: Some(aggregate.seq_nr()),
                source: Box::new(std::io::Error::other("foreign session audit write")),
            });
        }
        let key = SessionAuditKeyDto::of(aggregate.id());
        let envelope = EventEnvelope::new(
            key,
            aggregate.seq_nr(),
            aggregate.occurred_at(),
            SessionAuditEventDto::of(event),
        )
        .with_manifest(MANIFEST);
        match self
            .store
            .persist_event_and_snapshot(
                envelope,
                SessionAuditDto::of(aggregate),
                aggregate.version(),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(EventStoreWriteError::OptimisticLockError(_)) => {
                // 競合判断はストアの結果を使う。再読取は失敗診断の実在版だけを揃える。
                let actual = self
                    .store
                    .get_latest_snapshot_by_id(&SessionAuditKeyDto::of(aggregate.id()))
                    .await
                    .map_err(|error| self.read_error(error))?
                    .map_or(0, |snapshot| snapshot.version());
                Err(RepositoryError::Conflict {
                    expected: aggregate.version(),
                    actual,
                })
            }
            Err(EventStoreWriteError::IOError(source)) => Err(RepositoryError::Io {
                kind: io_kind_of_source(source.as_ref()),
                path: self.path.as_ref().map(|path| path.as_path().to_path_buf()),
            }),
            Err(_) => Err(RepositoryError::Io {
                kind: ErrorKind::Other,
                path: self.path.as_ref().map(|path| path.as_path().to_path_buf()),
            }),
        }
    }
}

#[cfg(test)]
#[path = "session_audit_repository_impl_tests.rs"]
mod tests;
