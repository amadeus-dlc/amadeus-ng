//! `WorkspaceDoctor` 集約を本家の SQLite ストアへ保存する Repository。
//!
//! 格納先は**一時ストア 1 種類だけ**である。診断は正本のストアへ書かない — C7 は初回状態で
//! ファイル・イベントを一切作らないこと (DC1) を定め、DC10 は空間ストアの journal への
//! 書込が失敗しても診断出力が残ることを定めるので、診断の事実を正本へ追記する形は契約と
//! 両立しない。永続する事実は `HEALTH_CHECKED` 1 件だけで、それは U2 の更新コマンドが
//! 別経路で書く (C7 `effects`、オーナー裁定 2026-09-12)。
use super::dto::{WorkspaceDoctorAggregateKeyDto, WorkspaceDoctorDto, WorkspaceDoctorEventDto};
use super::store_failure::io_kind_of_source;
use core_command_domain::workspace::{WorkspaceDoctor, WorkspaceDoctorEvent, WorkspaceDoctorId};
use core_command_use_case::orchestration::{RepositoryError, WorkspaceDoctorRepository};
use event_store_adapter_rs::EventStoreForSqlite;
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{EventStore, EventStoreReadError, EventStoreWriteError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
const MANIFEST: &str = "workspace-doctor-event/1";
/// 同一プロセス内で一時ストアの名前を衝突させないための採番。
static EPHEMERAL_SEQUENCE: AtomicU64 = AtomicU64::new(0);
/// SQLite の格納先。型引数はアダプタ所有の DTO。
///
/// 格納先が 1 種類しか無いので型引数を持たず、この別名も**このファイルの内部**である
/// (先回りで公開しない — `coding-rules/module-visibility.md`)。
type WorkspaceDoctorSqliteStore = EventStoreForSqlite<
    WorkspaceDoctorAggregateKeyDto,
    WorkspaceDoctorDto,
    WorkspaceDoctorEventDto,
>;
/// `WorkspaceDoctor` のストアを所有する。
#[derive(Debug)]
pub struct WorkspaceDoctorRepositoryImpl {
    store: WorkspaceDoctorSqliteStore,
    location: PathBuf,
}
impl WorkspaceDoctorRepositoryImpl {
    const fn new(store: WorkspaceDoctorSqliteStore, location: PathBuf) -> Self {
        Self { store, location }
    }
    /// 実ファイルを作らない一時ストアを開く (C7 DC1 — 診断は何も作らない)。
    ///
    /// 場所は SQLite の共有キャッシュ URI であり、同じ URI を開いた接続だけが同じ
    /// データベースを見る。最後の接続が閉じると消える。
    /// # Errors
    /// ストアを開けない場合。
    pub fn open_ephemeral() -> Result<Self, RepositoryError<WorkspaceDoctorId>> {
        let serial = EPHEMERAL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let location = PathBuf::from(format!(
            "file:aidlc-doctor-{}-{serial}?mode=memory&cache=shared",
            std::process::id()
        ));
        let store = WorkspaceDoctorSqliteStore::new(&location)
            .map_err(|e| Self::io(e, location.as_path()))?;
        Ok(Self::new(store, location))
    }
    /// このリポジトリが開いたストアの場所。
    ///
    /// 合成ルートが同じ場所を RMU とクエリ側 DAO へ渡すために読む — 一時ストアの URI は
    /// この層が採番するので、外から組み立てられる形にしない。
    #[must_use]
    pub fn location(&self) -> &Path {
        &self.location
    }
    fn io(error: EventStoreWriteError, path: &Path) -> RepositoryError<WorkspaceDoctorId> {
        let kind = match error {
            EventStoreWriteError::IOError(source) => io_kind_of_source(source.as_ref()),
            _ => ErrorKind::Other,
        };
        RepositoryError::Io {
            kind,
            path: Some(path.to_path_buf()),
        }
    }
    fn read_error(&self, error: EventStoreReadError) -> RepositoryError<WorkspaceDoctorId> {
        let kind = match error {
            EventStoreReadError::IOError(source) => io_kind_of_source(source.as_ref()),
            EventStoreReadError::DeserializationError(_) => ErrorKind::InvalidData,
            EventStoreReadError::OtherError(_) => ErrorKind::Other,
        };
        RepositoryError::Io {
            kind,
            path: Some(self.location.clone()),
        }
    }
}
impl WorkspaceDoctorRepository for WorkspaceDoctorRepositoryImpl {
    async fn find_by_id(
        &self,
        id: &WorkspaceDoctorId,
    ) -> Result<WorkspaceDoctor, RepositoryError<WorkspaceDoctorId>> {
        let key = WorkspaceDoctorAggregateKeyDto::of(id);
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
                    "workspace doctor snapshot identity mismatch",
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
                    source: Box::new(std::io::Error::other("workspace doctor manifest mismatch")),
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
        Ok(WorkspaceDoctor::replay(base, events).with_version(snapshot.version()))
    }
    async fn store(
        &mut self,
        event: &WorkspaceDoctorEvent,
        aggregate: &WorkspaceDoctor,
    ) -> Result<(), RepositoryError<WorkspaceDoctorId>> {
        if event.aggregate_id() != aggregate.id() {
            return Err(RepositoryError::Corrupt {
                id: aggregate.id().clone(),
                seq_nr: Some(aggregate.seq_nr()),
                source: Box::new(std::io::Error::other("foreign workspace doctor write")),
            });
        }
        let key = WorkspaceDoctorAggregateKeyDto::of(aggregate.id());
        let envelope = EventEnvelope::new(
            key,
            aggregate.seq_nr(),
            aggregate.diagnosed_at(),
            WorkspaceDoctorEventDto::of(event),
        )
        .with_manifest(MANIFEST);
        match self
            .store
            .persist_event_and_snapshot(
                envelope,
                WorkspaceDoctorDto::of(aggregate),
                aggregate.version(),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(EventStoreWriteError::OptimisticLockError(_)) => Err(RepositoryError::Conflict {
                expected: aggregate.version(),
                actual: self
                    .store
                    .get_latest_snapshot_by_id(&WorkspaceDoctorAggregateKeyDto::of(aggregate.id()))
                    .await
                    .ok()
                    .flatten()
                    .map_or(0, |snapshot| snapshot.version()),
            }),
            Err(error) => Err(Self::io(error, self.location.as_path())),
        }
    }
}
