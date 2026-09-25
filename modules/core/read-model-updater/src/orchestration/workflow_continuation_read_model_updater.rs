//! 停止制御の履歴から、要求IDの結果と本家互換の反復markerを投影する。
use super::{JournalReadError, ReadModelUpdater, store_failure::SqliteResultExt};
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    ContinuationAttemptId, ContinuationRequest, ContinuationSignature, WorkflowContinuation,
    WorkflowContinuationEvent, WorkflowContinuationEventId, WorkflowContinuationId,
};
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct GuardDto {
    signature: Option<String>,
    count: u64,
    initialized: bool,
}
impl GuardDto {
    fn to_domain(
        &self,
    ) -> Result<core_command_domain::orchestration::ContinuationGuard, JournalReadError> {
        let invalid = |_| JournalReadError::Io {
            kind: std::io::ErrorKind::InvalidData,
            path: None,
        };
        core_command_domain::orchestration::ContinuationGuard::new(
            self.signature
                .as_deref()
                .map(ContinuationSignature::parse)
                .transpose()
                .map_err(invalid)?,
            self.count,
            self.initialized,
        )
        .map_err(invalid)
    }
}
#[derive(Deserialize)]
struct RequestDto {
    id: String,
    signature: Option<String>,
    reentrant: bool,
    limit: u64,
    wait: Option<String>,
    probe_only: bool,
    observed_guard: Option<GuardDto>,
}
#[derive(Deserialize)]
struct EventDto {
    id: String,
    aggregate_id: String,
    request: RequestDto,
    count: u64,
    blocked: bool,
    publication: Option<bool>,
}
impl EventDto {
    fn to_domain(&self) -> Result<WorkflowContinuationEvent, JournalReadError> {
        let invalid = |_| JournalReadError::Io {
            kind: std::io::ErrorKind::InvalidData,
            path: None,
        };
        let id = WorkflowContinuationEventId::parse(&self.id).map_err(invalid)?;
        let aggregate = WorkflowContinuationId::parse(&self.aggregate_id).map_err(invalid)?;
        let request = ContinuationRequest::new(
            ContinuationAttemptId::parse(&self.request.id).map_err(invalid)?,
            self.request
                .signature
                .as_deref()
                .map(ContinuationSignature::parse)
                .transpose()
                .map_err(invalid)?,
            self.request.reentrant,
            self.request.limit,
        )
        .map_err(invalid)?
        .with_wait(
            self.request
                .wait
                .as_deref()
                .map(core_command_domain::orchestration::ContinuationWait::parse)
                .transpose()
                .map_err(invalid)?,
        )
        .map_err(invalid)?;
        let request = if self.request.probe_only {
            request.with_wait_probe().map_err(invalid)?
        } else {
            request
        };
        let request = match &self.request.observed_guard {
            Some(guard) => request.with_observed_guard(guard.to_domain()?),
            None => request,
        };
        let event =
            WorkflowContinuationEvent::new(id, aggregate, request, self.count, self.blocked)
                .map_err(invalid)?;
        Ok(match self.publication {
            Some(published) => event.with_publication(published),
            None => event,
        })
    }
}
/// 共有DBで停止制御自身のstreamだけを扱う投影器。
///
/// 投影する停止制御のstreamと、反復markerを公開する記録ディレクトリは構築時に束ねる
/// （[`ReadModelUpdater`] の契約）。同じ更新器を何度呼んでも同じstreamを描き直す。
#[derive(Debug)]
pub struct WorkflowContinuationReadModelUpdater {
    connection: Connection,
    path: PathBuf,
    id: WorkflowContinuationId,
    record: PathBuf,
}
impl WorkflowContinuationReadModelUpdater {
    /// 既存DBへ接続し、投影するstreamと公開先の記録ディレクトリを束ねる。DBや業務streamは作らない。
    /// # Errors
    /// 接続またはread表初期化の失敗。
    pub fn open(
        path: &Path,
        id: WorkflowContinuationId,
        record: PathBuf,
    ) -> Result<Self, JournalReadError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .at_store(path)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .at_store(path)?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS read_continuation_result(id TEXT PRIMARY KEY,aggregate_id TEXT NOT NULL,signature TEXT NOT NULL,count INTEGER NOT NULL,blocked INTEGER NOT NULL,seq_nr INTEGER NOT NULL,block_cap TEXT NOT NULL,wait_reason TEXT,counter_published INTEGER,settled INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS continuation_projection_checkpoint(id TEXT PRIMARY KEY,seq_nr INTEGER NOT NULL);").at_store(path)?;
        Ok(Self {
            connection,
            path: path.to_path_buf(),
            id,
            record,
        })
    }
}

impl ReadModelUpdater for WorkflowContinuationReadModelUpdater {
    type Error = JournalReadError;

    /// 現在の全履歴から計算し、ファイル公開後に同じDB排他内で結果と位置を確定する。
    ///
    /// 内部は同期 I/O だけである。非同期なのは共通契約の境界だけ。
    /// # Errors
    /// 読取・復号・ファイル公開・DB確定の失敗。
    /// # Panics
    /// 保存済み履歴の通番または進捗回数が矛盾する場合。
    async fn update_read_models(&mut self) -> Result<(), JournalReadError> {
        self.project()
    }
}

impl WorkflowContinuationReadModelUpdater {
    /// 束ねたstreamの全履歴を投影する（[`ReadModelUpdater::update_read_models`] の本体）。
    fn project(&mut self) -> Result<(), JournalReadError> {
        let id = &self.id;
        let record = self.record.as_path();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(&self.path)?;
        let mut statement = tx.prepare("SELECT seq_nr,occurred_at,payload FROM journal WHERE aid=?1 AND manifest='workflow-continuation-event/1' ORDER BY seq_nr").at_store(&self.path)?;
        let rows = statement
            .query_map([id.as_str()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .at_store(&self.path)?;
        let mut history = Vec::new();
        for row in rows {
            let (sequence, at, bytes) = row.at_store(&self.path)?;
            let sequence = usize::try_from(sequence).map_err(|_| invalid_data(&self.path))?;
            let dto: EventDto =
                serde_json::from_slice(&bytes).map_err(|_| JournalReadError::Io {
                    kind: std::io::ErrorKind::InvalidData,
                    path: Some(self.path.clone()),
                })?;
            let event = dto.to_domain()?;
            if event.aggregate_id() != id {
                return Err(JournalReadError::Io {
                    kind: std::io::ErrorKind::InvalidData,
                    path: Some(self.path.clone()),
                });
            }
            history.push((event, sequence, DateTime::<Utc>::from_timestamp_nanos(at)));
        }
        drop(statement);
        let Some((first, first_sequence, first_at)) = history.first() else {
            return Ok(());
        };
        if *first_sequence != 1 || first.publication().is_some() {
            return Err(invalid_data(&self.path));
        }
        let seed = WorkflowContinuation::new(
            id.clone(),
            first.request().clone(),
            {
                let before = first.request().observed_guard().cloned().unwrap_or(
                    core_command_domain::orchestration::ContinuationGuard::new(None, 0, false)
                        .map_err(|_| invalid_data(&self.path))?,
                );
                let selected = before
                    .after(first.request())
                    .map_err(|_| invalid_data(&self.path))?;
                core_command_domain::orchestration::ContinuationCounter::new(
                    before,
                    selected,
                    (first.request().wait().is_some() || first.request().is_wait_probe())
                        .then_some(true),
                )
            },
            *first_sequence,
            0,
            *first_at,
        )
        .map_err(|_| JournalReadError::Io {
            kind: std::io::ErrorKind::InvalidData,
            path: Some(self.path.clone()),
        })?;
        if seed.count() != first.count() {
            return Err(invalid_data(&self.path));
        }
        let aggregate = WorkflowContinuation::replay(seed, history.iter().skip(1).cloned());
        for (event, sequence, _) in &history {
            if event.publication().is_some() {
                continue;
            }
            let confirmation = history.iter().find_map(|(candidate, _, _)| {
                (candidate.request().id() == event.request().id())
                    .then(|| candidate.publication())
                    .flatten()
            });
            let no_write = event.request().wait().is_some() || event.request().is_wait_probe();
            let existing: Option<(Option<bool>, bool)> = tx
                .query_row(
                    "SELECT counter_published,settled FROM read_continuation_result WHERE id=?1",
                    [event.request().id().as_str()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .at_store(&self.path)?;
            let (published, settled) = if let Some(succeeded) = confirmation {
                (Some(succeeded), true)
            } else if no_write {
                (None, true)
            } else if let Some(previous) = existing {
                previous
            } else {
                (Some(publish_counter(record, event)), false)
            };
            tx.execute("INSERT INTO read_continuation_result VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(id) DO UPDATE SET counter_published=excluded.counter_published,settled=excluded.settled", params![event.request().id().as_str(),id.as_str(),event.request().signature().map_or("", ContinuationSignature::as_str),i64::try_from(event.count()).map_err(|_| invalid_data(&self.path))?,event.blocked(),i64::try_from(*sequence).map_err(|_| invalid_data(&self.path))?,event.request().limit().to_string(),event.request().wait().map(|wait| wait.as_str()),published,settled]).at_store(&self.path)?;
        }
        tx.execute("INSERT INTO continuation_projection_checkpoint VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET seq_nr=excluded.seq_nr",params![id.as_str(),i64::try_from(aggregate.seq_nr()).map_err(|_| invalid_data(&self.path))?]).at_store(&self.path)?;
        tx.commit().at_store(&self.path)
    }
}

fn invalid_data(path: &Path) -> JournalReadError {
    JournalReadError::Io {
        kind: std::io::ErrorKind::InvalidData,
        path: Some(path.to_path_buf()),
    }
}

fn publish_counter(record: &Path, event: &WorkflowContinuationEvent) -> bool {
    let mut fields = ObjectMembers::new();
    fields.insert(
        "signature",
        JsonValue::String(
            event
                .request()
                .signature()
                .map_or("", ContinuationSignature::as_str)
                .to_string(),
        ),
    );
    fields.insert("count", JsonValue::Number(Number::PosInt(event.count())));
    let bytes = core_infrastructure::canon_json::serialize(
        &JsonValue::Object(fields),
        core_infrastructure::canon_json::SerializationProfile::ContractCompact,
    );
    let directory = record.join(".aidlc-stop-hook");
    std::fs::create_dir_all(&directory)
        .and_then(|()| {
            core_infrastructure::atomic::write_file_atomic(
                &directory.join("block-count.json"),
                bytes.as_bytes(),
            )
        })
        .is_ok()
}
