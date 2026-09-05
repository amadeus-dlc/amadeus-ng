//! SQLiteに計画を先行保存し、同じDBの書込排他下で公開・確定する。

use super::store_failure::SqliteResultExt;
use super::{
    CatchUpError, GlobalSeqNr, JournalReadError, JournalReaderImpl, ProjectionName,
    PublicationBatch, PublicationFile,
};
use crate::read_tables::ReadTables;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS amadeus_publication (
 projection TEXT PRIMARY KEY, start_position INTEGER NOT NULL, target_position INTEGER NOT NULL,
 committed INTEGER NOT NULL CHECK(committed IN (0,1)), file_count INTEGER NOT NULL CHECK(file_count>=0),
 request_id TEXT NOT NULL, generation INTEGER NOT NULL CHECK(generation>0), rebuild_mode INTEGER NOT NULL CHECK(rebuild_mode IN (0,1)),
 format_version INTEGER NOT NULL, plan_digest TEXT NOT NULL, target_binding TEXT, transform_revision TEXT NOT NULL,
 served_position INTEGER, served_generation INTEGER);
 CREATE TABLE IF NOT EXISTS amadeus_publication_file (
 projection TEXT NOT NULL, ordinal INTEGER NOT NULL, path TEXT NOT NULL,
 before_content BLOB, after_content BLOB NOT NULL, append_mode INTEGER NOT NULL CHECK(append_mode IN (0,1)),
 memory_mode INTEGER NOT NULL CHECK(memory_mode IN (0,1)), PRIMARY KEY(projection,ordinal));
 CREATE TABLE IF NOT EXISTS amadeus_publication_history (
 projection TEXT NOT NULL, generation INTEGER NOT NULL, request_id TEXT NOT NULL,
 start_position INTEGER NOT NULL, target_position INTEGER NOT NULL, state TEXT NOT NULL CHECK(state IN ('committed','superseded')),
 served_position INTEGER, served_generation INTEGER,
 PRIMARY KEY(projection,generation), UNIQUE(projection,request_id));
 CREATE TABLE IF NOT EXISTS amadeus_publication_history_file (
 projection TEXT NOT NULL, generation INTEGER NOT NULL, ordinal INTEGER NOT NULL, path TEXT NOT NULL,
 before_content BLOB, after_content BLOB NOT NULL, append_mode INTEGER NOT NULL, memory_mode INTEGER NOT NULL,
 PRIMARY KEY(projection,generation,ordinal));
 CREATE TABLE IF NOT EXISTS amadeus_publication_snapshot (
 projection TEXT NOT NULL, target_binding TEXT NOT NULL,
 start_position INTEGER NOT NULL, target_position INTEGER NOT NULL,
 file_count INTEGER NOT NULL, request_id TEXT NOT NULL, generation INTEGER NOT NULL,
 rebuild_mode INTEGER NOT NULL, format_version INTEGER NOT NULL, plan_digest TEXT NOT NULL,
 transform_revision TEXT NOT NULL, PRIMARY KEY(projection,target_binding));
 CREATE TABLE IF NOT EXISTS amadeus_publication_snapshot_file (
 projection TEXT NOT NULL, target_binding TEXT NOT NULL, ordinal INTEGER NOT NULL,
 path TEXT NOT NULL, before_content BLOB, after_content BLOB NOT NULL,
 append_mode INTEGER NOT NULL, memory_mode INTEGER NOT NULL,
 PRIMARY KEY(projection,target_binding,ordinal));";

pub(super) fn initialize(connection: &Connection, path: &Path) -> Result<(), JournalReadError> {
    connection.execute_batch(SCHEMA).at_store(path)
}

pub(super) fn pending(
    connection: &Connection,
    path: &Path,
    projection: &ProjectionName,
) -> Result<Option<PublicationBatch>, JournalReadError> {
    read(connection, path, projection, true, None)
}

pub(super) fn latest(
    connection: &Connection,
    path: &Path,
    projection: &ProjectionName,
) -> Result<Option<PublicationBatch>, JournalReadError> {
    read(connection, path, projection, false, None)
}

pub(super) fn snapshot(
    connection: &Connection,
    path: &Path,
    projection: &ProjectionName,
    binding: &str,
) -> Result<Option<PublicationBatch>, JournalReadError> {
    read(connection, path, projection, false, Some(binding))
}

fn invalid(path: &Path) -> JournalReadError {
    JournalReadError::Io {
        kind: std::io::ErrorKind::InvalidData,
        path: Some(path.to_path_buf()),
    }
}

fn conflict(path: &Path) -> CatchUpError {
    CatchUpError::PublicationConflict {
        path: path.to_path_buf(),
    }
}

type Header = (
    i64,
    i64,
    i64,
    String,
    i64,
    bool,
    i64,
    String,
    Option<String>,
    String,
);

fn fingerprint(batch: &PublicationBatch, path: &Path) -> Result<String, JournalReadError> {
    let files = batch
        .files()
        .iter()
        .map(|file| {
            Ok((
                file.path().to_str().ok_or_else(|| invalid(path))?,
                file.before(),
                file.after(),
                file.is_append(),
                file.is_memory(),
            ))
        })
        .collect::<Result<Vec<_>, JournalReadError>>()?;
    // 大きな通番もJSの数値丸めを受けないよう、識別材料では十進文字列にする。
    let material = core_infrastructure::canon_json::to_value(&(
        batch.request_id(),
        batch.generation().to_string(),
        batch.from().to_u64().to_string(),
        batch.to().to_u64().to_string(),
        batch.is_rebuild(),
        batch.target_binding(),
        batch.transform_revision(),
        files,
    ))
    .map_err(|_| invalid(path))?;
    Ok(core_infrastructure::canon_json::hash_compact(&material).rendered())
}

fn read(
    connection: &Connection,
    path: &Path,
    projection: &ProjectionName,
    pending_only: bool,
    snapshot_binding: Option<&str>,
) -> Result<Option<PublicationBatch>, JournalReadError> {
    let invalid_number = |_: std::num::TryFromIntError| invalid(path);
    let header_sql = if snapshot_binding.is_some() {
        "SELECT start_position,target_position,file_count,request_id,generation,rebuild_mode,format_version,plan_digest,target_binding,transform_revision FROM amadeus_publication_snapshot WHERE projection=?1 AND target_binding=?3 AND ?2=0"
    } else {
        "SELECT start_position,target_position,file_count,request_id,generation,rebuild_mode,format_version,plan_digest,target_binding,transform_revision FROM amadeus_publication WHERE projection=?1 AND (?2=0 OR committed=0) AND ?3 IS NULL"
    };
    let header: Option<Header> = connection
        .query_row(
            header_sql,
            params![projection.as_str(), pending_only, snapshot_binding],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                ))
            },
        )
        .optional()
        .at_store(path)?;
    let Some((start, end, count, id, generation, rebuild, version, digest, binding, revision)) =
        header
    else {
        return Ok(None);
    };
    if version != 1 {
        return Err(invalid(path));
    }
    let start = u64::try_from(start).map_err(invalid_number)?;
    let end = u64::try_from(end).map_err(invalid_number)?;
    let generation = u64::try_from(generation).map_err(invalid_number)?;
    if end < start || generation == 0 || uuid::Uuid::parse_str(&id).is_err() {
        return Err(invalid(path));
    }
    let file_sql = if snapshot_binding.is_some() {
        "SELECT ordinal,path,before_content,after_content,append_mode,memory_mode FROM amadeus_publication_snapshot_file WHERE projection=?1 AND target_binding=?2 ORDER BY ordinal"
    } else {
        "SELECT ordinal,path,before_content,after_content,append_mode,memory_mode FROM amadeus_publication_file WHERE projection=?1 AND ?2 IS NULL ORDER BY ordinal"
    };
    let mut statement = connection.prepare(file_sql).at_store(path)?;
    let rows = statement
        .query_map(params![projection.as_str(), snapshot_binding], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                PublicationFile::restored(
                    PathBuf::from(r.get::<_, String>(1)?),
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ),
            ))
        })
        .at_store(path)?;
    let mut files = Vec::new();
    for row in rows {
        let (ordinal, file) = row.at_store(path)?;
        if usize::try_from(ordinal).ok() != Some(files.len()) || !file.path().is_absolute() {
            return Err(invalid(path));
        }
        files.push(file);
    }
    if usize::try_from(count).ok() != Some(files.len()) {
        return Err(invalid(path));
    }
    let batch = PublicationBatch::restored(
        GlobalSeqNr::new(start),
        GlobalSeqNr::new(end),
        files,
        id,
        generation,
        rebuild,
    )
    .bound(binding, revision);
    if fingerprint(&batch, path)? != digest || (pending_only && !batch.uses_current_transform()) {
        return Err(invalid(path));
    }
    Ok(Some(batch))
}

fn archive(
    transaction: &rusqlite::Transaction<'_>,
    path: &Path,
    projection: &ProjectionName,
    superseded: bool,
) -> Result<(), JournalReadError> {
    transaction.execute("INSERT INTO amadeus_publication_history SELECT projection,generation,request_id,start_position,target_position,?2,served_position,served_generation FROM amadeus_publication WHERE projection=?1",
        params![projection.as_str(),if superseded { "superseded" } else { "committed" }]).at_store(path)?;
    // 解決が必要だった未完計画の前後は保全する。通常の完了計画は要求IDと範囲だけ残す。
    if superseded {
        transaction.execute("INSERT INTO amadeus_publication_history_file SELECT f.projection,p.generation,f.ordinal,f.path,f.before_content,f.after_content,f.append_mode,f.memory_mode FROM amadeus_publication_file f JOIN amadeus_publication p ON f.projection=p.projection WHERE f.projection=?1",
            [projection.as_str()]).at_store(path)?;
    }
    Ok(())
}

fn prepare(
    connection: &mut Connection,
    path: &Path,
    projection: &ProjectionName,
    candidate: &PublicationBatch,
) -> Result<Option<PublicationBatch>, CatchUpError> {
    let invalid_number = |_: std::num::TryFromIntError| invalid(path);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .at_store(path)?;
    let archived: Option<String> = transaction
        .query_row(
            "SELECT state FROM amadeus_publication_history WHERE projection=?1 AND request_id=?2",
            params![projection.as_str(), candidate.request_id()],
            |r| r.get(0),
        )
        .optional()
        .at_store(path)?;
    if let Some(state) = archived {
        return if state == "committed" {
            Ok(None)
        } else {
            Err(conflict(path))
        };
    }
    let previous = latest(&transaction, path, projection)?;
    let unfinished = pending(&transaction, path, projection)?;
    if let Some(saved) = &previous
        && saved.request_id() == candidate.request_id()
    {
        if saved.target_binding() != candidate.target_binding()
            || saved.transform_revision() != candidate.transform_revision()
        {
            return Err(conflict(path));
        }
        return Ok(unfinished); // 完了要求の再送は無操作。
    }
    let replacing = candidate.predecessor().is_some();
    if let Some(saved) = &unfinished {
        if replacing {
            if candidate.predecessor() != Some(saved.request_id())
                || candidate.from() != saved.from()
                || candidate.to() != saved.to()
            {
                return Err(conflict(path));
            }
        } else {
            if candidate.generation() > 0
                || saved.target_binding() != candidate.target_binding()
                || saved.from() != candidate.from()
                || saved.to() != candidate.to()
                || saved
                    .files()
                    .iter()
                    .map(PublicationFile::path)
                    .ne(candidate.files().iter().map(PublicationFile::path))
            {
                return Err(conflict(path));
            }
            return Ok(Some(saved.clone())); // 同じ範囲の並行要求は先に保存された計画へ合流。
        }
    } else if replacing || candidate.generation() > 0 {
        return Err(conflict(path));
    }
    let current = JournalReaderImpl::read_checkpoint(&transaction, projection, path)?;
    if !candidate.is_rebuild() && current >= candidate.to() && candidate.to() > GlobalSeqNr::ZERO {
        return Ok(None);
    }
    if current != candidate.from() || candidate.to() < candidate.from() {
        return Err(conflict(path));
    }
    let generation = previous
        .as_ref()
        .map_or(0, PublicationBatch::generation)
        .checked_add(1)
        .ok_or_else(|| invalid(path))?;
    let mut files = candidate.files().to_vec();
    let prior_target = candidate
        .target_binding()
        .map(|binding| snapshot(&transaction, path, projection, binding))
        .transpose()?
        .flatten();
    if unfinished.is_none()
        && let Some(previous) = &prior_target
        && previous.target_binding().is_some()
        && previous.target_binding() == candidate.target_binding()
        && previous.uses_current_transform()
    {
        for file in previous.files() {
            if !files.iter().any(|current| current.path() == file.path())
                && let Some((unchanged, _)) = file.restore_missing()?
            {
                files.push(unchanged);
            }
        }
    }
    let accepted = candidate.clone().with_files(files).accepted(
        candidate.request_id().to_string(),
        generation,
        candidate.is_rebuild(),
    );
    let digest = fingerprint(&accepted, path)?;
    if previous.is_some() {
        archive(&transaction, path, projection, unfinished.is_some())?;
    }
    transaction
        .execute(
            "DELETE FROM amadeus_publication_file WHERE projection=?1",
            [projection.as_str()],
        )
        .at_store(path)?;
    transaction.execute("INSERT INTO amadeus_publication VALUES (?1,?2,?3,0,?4,?5,?6,?7,1,?8,?9,?10,NULL,NULL) ON CONFLICT(projection) DO UPDATE SET start_position=excluded.start_position,target_position=excluded.target_position,committed=0,file_count=excluded.file_count,request_id=excluded.request_id,generation=excluded.generation,rebuild_mode=excluded.rebuild_mode,format_version=excluded.format_version,plan_digest=excluded.plan_digest,target_binding=excluded.target_binding,transform_revision=excluded.transform_revision,served_position=NULL,served_generation=NULL",
        params![projection.as_str(),i64::try_from(candidate.from().to_u64()).map_err(invalid_number)?,i64::try_from(candidate.to().to_u64()).map_err(invalid_number)?,i64::try_from(accepted.files().len()).map_err(invalid_number)?,candidate.request_id(),i64::try_from(generation).map_err(invalid_number)?,candidate.is_rebuild(),digest,candidate.target_binding(),candidate.transform_revision()]).at_store(path)?;
    for (ordinal, file) in accepted.files().iter().enumerate() {
        if !file.path().is_absolute() {
            return Err(invalid(path).into());
        }
        transaction
            .execute(
                "INSERT INTO amadeus_publication_file VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    projection.as_str(),
                    i64::try_from(ordinal).map_err(invalid_number)?,
                    file.path().to_str().ok_or_else(|| invalid(path))?,
                    file.before(),
                    file.after(),
                    file.is_append(),
                    file.is_memory()
                ],
            )
            .at_store(path)?;
    }
    transaction.commit().at_store(path)?;
    Ok(Some(accepted))
}

pub(super) fn publish(
    connection: &mut Connection,
    path: &Path,
    projection: &ProjectionName,
    candidate: &PublicationBatch,
    tables: &ReadTables,
) -> Result<(), CatchUpError> {
    let Some(batch) = prepare(connection, path, projection, candidate)? else {
        return Ok(());
    };
    publish_prepared(connection, path, projection, &batch, tables)
}

/// 耐久化済みの計画を再照合し、次のトランザクションで公開・確定する。
/// 準備とこの再照合の間に別の書き手が完了・置換していても、古い計画では書かない。
fn publish_prepared(
    connection: &mut Connection,
    path: &Path,
    projection: &ProjectionName,
    batch: &PublicationBatch,
    tables: &ReadTables,
) -> Result<(), CatchUpError> {
    // 計画は耐久化済み。比較開始から確定までDBの書込排他を保持する。
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .at_store(path)?;
    let Some(saved) = pending(&transaction, path, projection)? else {
        let current = latest(&transaction, path, projection)?;
        return if current
            .as_ref()
            .is_some_and(|p| p.request_id() == batch.request_id())
        {
            Ok(())
        } else {
            Err(conflict(path))
        };
    };
    if &saved != batch {
        return Err(conflict(path));
    }
    let current = JournalReaderImpl::read_checkpoint(&transaction, projection, path)?;
    if current != saved.from() && current != saved.to() {
        return Err(conflict(path));
    }
    super::shared_projection::verify(&transaction, path)?;
    saved.apply()?;
    JournalReaderImpl::advance_on(&transaction, path, projection, saved.to(), tables)?;
    let head = super::shared_projection::read(&transaction, path)?.ok_or_else(|| conflict(path))?;
    transaction
        .execute(
            "UPDATE amadeus_publication SET committed=1,served_position=?3,served_generation=?4 WHERE projection=?1 AND request_id=?2",
            params![projection.as_str(), saved.request_id(),head.position(),head.generation()],
        )
        .at_store(path)?;
    if let Some(binding) = saved.target_binding() {
        // 出力先ごとに最新の完了断面を残す。別intentの公開で復元材料を失わない。
        transaction.execute("DELETE FROM amadeus_publication_snapshot_file WHERE projection=?1 AND target_binding=?2",
            params![projection.as_str(),binding]).at_store(path)?;
        transaction.execute("INSERT INTO amadeus_publication_snapshot SELECT projection,target_binding,start_position,target_position,file_count,request_id,generation,rebuild_mode,format_version,plan_digest,transform_revision FROM amadeus_publication WHERE projection=?1
            ON CONFLICT(projection,target_binding) DO UPDATE SET start_position=excluded.start_position,target_position=excluded.target_position,file_count=excluded.file_count,request_id=excluded.request_id,generation=excluded.generation,rebuild_mode=excluded.rebuild_mode,format_version=excluded.format_version,plan_digest=excluded.plan_digest,transform_revision=excluded.transform_revision",
            [projection.as_str()]).at_store(path)?;
        transaction.execute("INSERT INTO amadeus_publication_snapshot_file SELECT projection,?2,ordinal,path,before_content,after_content,append_mode,memory_mode FROM amadeus_publication_file WHERE projection=?1",
            params![projection.as_str(),binding]).at_store(path)?;
    }
    transaction.commit().at_store(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::journal_reader_impl::tests::{birth_event, opened_store};
    use crate::orchestration::{JournalBatch, JournalReader};
    use core_command_domain::workspace::StorePath;

    /// 既存の本家ストア用フィクスチャを共有し、両相は実SQLiteの別接続で動かす。
    fn fixture() -> (tempfile::TempDir, StorePath, PathBuf, PublicationBatch) {
        let dir = tempfile::tempdir().unwrap();
        let (_store, path) = opened_store(&dir);
        drop(JournalReaderImpl::open(&path).unwrap());
        let state = dir.path().join("state.md");
        std::fs::write(&state, "before\n").unwrap();
        let batch = PublicationBatch::rebuild(
            GlobalSeqNr::ZERO,
            GlobalSeqNr::ZERO,
            vec![PublicationFile::replacement(&state, "before\n", "after\n")],
        );
        (dir, path, state, batch)
    }

    fn projection() -> ProjectionName {
        ProjectionName::parse("publication-race").unwrap()
    }

    fn tables() -> ReadTables {
        ReadTables::project(&JournalBatch::empty()).unwrap()
    }

    #[test]
    fn another_writer_can_finish_the_same_prepared_request() {
        let (_dir, path, state, candidate) = fixture();
        let mut first = Connection::open(path.as_path()).unwrap();
        let mut second = Connection::open(path.as_path()).unwrap();
        let saved = prepare(&mut first, path.as_path(), &projection(), &candidate)
            .unwrap()
            .unwrap();
        publish(
            &mut second,
            path.as_path(),
            &projection(),
            &candidate,
            &tables(),
        )
        .unwrap();
        std::fs::write(&state, "after\nuser addition\n").unwrap();

        publish_prepared(&mut first, path.as_path(), &projection(), &saved, &tables()).unwrap();

        assert_eq!(
            std::fs::read_to_string(&state).unwrap(),
            "after\nuser addition\n"
        );
        assert!(
            pending(&first, path.as_path(), &projection())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_replacement_that_finished_before_the_prepared_writer_is_not_overwritten() {
        let (_dir, path, state, candidate) = fixture();
        let mut first = Connection::open(path.as_path()).unwrap();
        let mut second = Connection::open(path.as_path()).unwrap();
        let saved = prepare(&mut first, path.as_path(), &projection(), &candidate)
            .unwrap()
            .unwrap();
        let replacement = PublicationBatch::rebuild(
            GlobalSeqNr::ZERO,
            GlobalSeqNr::ZERO,
            vec![PublicationFile::replacement(
                &state,
                "before\n",
                "replacement\n",
            )],
        )
        .replacing(saved.request_id());
        publish(
            &mut second,
            path.as_path(),
            &projection(),
            &replacement,
            &tables(),
        )
        .unwrap();

        let error = publish_prepared(&mut first, path.as_path(), &projection(), &saved, &tables())
            .unwrap_err();

        assert_eq!(
            error,
            CatchUpError::PublicationConflict {
                path: path.as_path().to_path_buf()
            }
        );
        assert_eq!(std::fs::read_to_string(&state).unwrap(), "replacement\n");
        assert!(
            pending(&first, path.as_path(), &projection())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            latest(&first, path.as_path(), &projection())
                .unwrap()
                .unwrap()
                .request_id(),
            replacement.request_id()
        );
    }

    #[test]
    fn a_newer_pending_generation_fences_the_previous_prepared_writer() {
        let (_dir, path, state, candidate) = fixture();
        let mut first = Connection::open(path.as_path()).unwrap();
        let mut second = Connection::open(path.as_path()).unwrap();
        let saved = prepare(&mut first, path.as_path(), &projection(), &candidate)
            .unwrap()
            .unwrap();
        let replacement = PublicationBatch::rebuild(
            GlobalSeqNr::ZERO,
            GlobalSeqNr::ZERO,
            vec![PublicationFile::replacement(
                &state,
                "before\n",
                "replacement\n",
            )],
        )
        .replacing(saved.request_id());
        let newer = prepare(&mut second, path.as_path(), &projection(), &replacement)
            .unwrap()
            .unwrap();
        assert!(newer.generation() > saved.generation());

        let error = publish_prepared(&mut first, path.as_path(), &projection(), &saved, &tables())
            .unwrap_err();

        assert_eq!(
            error,
            CatchUpError::PublicationConflict {
                path: path.as_path().to_path_buf()
            }
        );
        assert_eq!(std::fs::read_to_string(&state).unwrap(), "before\n");
        assert_eq!(
            pending(&first, path.as_path(), &projection()).unwrap(),
            Some(newer.clone())
        );
        publish_prepared(
            &mut second,
            path.as_path(),
            &projection(),
            &newer,
            &tables(),
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&state).unwrap(), "replacement\n");
    }

    #[test]
    fn a_resolution_based_on_an_old_predecessor_cannot_replace_the_new_generation() {
        let (_dir, path, state, candidate) = fixture();
        let mut first = Connection::open(path.as_path()).unwrap();
        let mut second = Connection::open(path.as_path()).unwrap();
        let original = prepare(&mut first, path.as_path(), &projection(), &candidate)
            .unwrap()
            .unwrap();
        // AはP1に基づく解決内容を作るが、まだ保存していない。
        let first_resolution = PublicationBatch::rebuild(
            GlobalSeqNr::ZERO,
            GlobalSeqNr::ZERO,
            vec![PublicationFile::replacement(
                &state,
                "before\n",
                "first resolution\n",
            )],
        )
        .replacing(original.request_id());
        // 先にBが同じP1を置換してP2を準備する。
        let second_resolution = PublicationBatch::rebuild(
            GlobalSeqNr::ZERO,
            GlobalSeqNr::ZERO,
            vec![PublicationFile::replacement(
                &state,
                "before\n",
                "second resolution\n",
            )],
        )
        .replacing(original.request_id());
        let newer = prepare(
            &mut second,
            path.as_path(),
            &projection(),
            &second_resolution,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            prepare(&mut first, path.as_path(), &projection(), &first_resolution),
            Err(CatchUpError::PublicationConflict {
                path: path.as_path().to_path_buf()
            })
        );

        assert_eq!(
            pending(&first, path.as_path(), &projection()).unwrap(),
            Some(newer.clone())
        );
        assert_eq!(std::fs::read_to_string(&state).unwrap(), "before\n");
        publish_prepared(
            &mut second,
            path.as_path(),
            &projection(),
            &newer,
            &tables(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&state).unwrap(),
            "second resolution\n"
        );
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "本家スキーマへ復号可能なジャーナル行を追加するフィクスチャ (BR1.7の射程外)"
    )]
    #[tokio::test]
    async fn a_checkpoint_advanced_between_phases_fences_the_prepared_writer() {
        let (_dir, path, state, candidate) = fixture();
        let mut first = Connection::open(path.as_path()).unwrap();
        let saved = prepare(&mut first, path.as_path(), &projection(), &candidate)
            .unwrap()
            .unwrap();
        let event = birth_event();
        let payload = serde_json::to_vec(&crate::orchestration::IntentEventDto::of(
            &event,
            chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
        ))
        .unwrap();
        // Aが空の履歴の再生成を準備した後にIntentが誕生し、Bがその断面を確定する。
        first.execute(
            "INSERT INTO journal(pkey,skey,aid,seq_nr,payload,occurred_at,manifest) VALUES (?1,'1',?1,1,?2,0,'intent-event/1')",
            params![event.aggregate_id().as_str(), payload],
        ).unwrap();
        let mut second = JournalReaderImpl::open(&path).unwrap();
        let history = second.events_after(GlobalSeqNr::ZERO).await.unwrap();
        let last = history.scanned_to().unwrap();
        let advanced = ReadTables::project(&history).unwrap();
        second
            .advance_checkpoint(&projection(), last, &advanced)
            .await
            .unwrap();

        let error = publish_prepared(&mut first, path.as_path(), &projection(), &saved, &tables())
            .unwrap_err();

        assert_eq!(
            error,
            CatchUpError::PublicationConflict {
                path: path.as_path().to_path_buf()
            }
        );
        assert_eq!(std::fs::read_to_string(&state).unwrap(), "before\n");
        assert_eq!(
            pending(&first, path.as_path(), &projection()).unwrap(),
            Some(saved)
        );
        assert_eq!(second.checkpoint(&projection()).await.unwrap(), last);
    }

    #[test]
    fn a_valid_plan_from_an_old_transform_is_not_resumed() {
        let connection = Connection::open_in_memory().unwrap();
        let path = Path::new("publication-test.db");
        initialize(&connection, path).unwrap();
        let projection = ProjectionName::parse("state-file").unwrap();
        let candidate = PublicationBatch::rebuild(GlobalSeqNr::ZERO, GlobalSeqNr::ZERO, vec![]);
        let old = candidate
            .clone()
            .accepted(candidate.request_id().to_string(), 1, true)
            .bound(None, "publication-0/read-0".to_string());
        // 破損検出とは独立に、旧バイナリが正しく保存した計画の互換性を検証する。
        let digest = fingerprint(&old, path).unwrap();
        connection
            .execute(
                "INSERT INTO amadeus_publication VALUES (?1,0,0,0,0,?2,1,1,1,?3,NULL,?4,NULL,NULL)",
                params![
                    projection.as_str(),
                    old.request_id(),
                    digest,
                    old.transform_revision()
                ],
            )
            .unwrap();
        assert_eq!(latest(&connection, path, &projection).unwrap(), Some(old));
        assert!(matches!(
            pending(&connection, path, &projection),
            Err(JournalReadError::Io {
                kind: std::io::ErrorKind::InvalidData,
                ..
            })
        ));
    }
}
