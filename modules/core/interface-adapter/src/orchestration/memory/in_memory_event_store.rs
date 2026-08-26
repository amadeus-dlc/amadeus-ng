//! `InMemoryEventStore` — `EventStore` と `JournalReader` の in-memory 実装 (BR2.7)。
//!
//! 実 I/O を持たないだけで、**行の形も規則も SQLite 実装と同じ**にしてある (C6 の 3 表:
//! journal / snapshot / checkpoint)。ペイロードは実装と同じワイヤ (正準 JSON) で持ち、
//! 楽観 version の判定も同じ手順を踏む。そうしておかないと、共有した契約テスト
//! (`tests/support/contract.rs`) が「in-memory では通るが SQLite では通らない」経路を
//! 見逃すからである (gateway-taxonomy §6 — Repository は in-memory から始める)。
//!
//! テストダブルなので `Impl` 接尾辞は付けない
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。

use std::collections::BTreeMap;
use std::ops::Bound;

use chrono::{DateTime, SecondsFormat, Utc};
use core_domain::orchestration::{IntentId, WorkflowExecution, WorkflowExecutionEvent};
use core_use_case::orchestration::{
    CorruptCause, EventStore, EventStoreError, GlobalSeqNr, JournalReader, ProjectionName,
};
use event_store_adapter_rs::types::{Aggregate, Event};

use super::super::wire::{EventPayloadWire, StateWire};

/// C6 `journal` の 1 行 (封筒は列、ペイロードは正準 JSON)。
#[derive(Debug, Clone)]
struct JournalRow {
    global_seq_nr: GlobalSeqNr,
    schema_version: u32,
    event_type: String,
    payload: String,
    occurred_at: String,
}

/// C6 `snapshot` の 1 行 (集約 1 行)。
#[derive(Debug, Clone)]
struct SnapshotRow {
    version: usize,
    seq_nr: usize,
    schema_version: u32,
    payload: String,
}

/// 3 表と global 通番の採番器。
#[derive(Debug, Clone, Default)]
struct Tables {
    journal: BTreeMap<(IntentId, usize), JournalRow>,
    snapshot: BTreeMap<IntentId, SnapshotRow>,
    checkpoint: BTreeMap<ProjectionName, GlobalSeqNr>,
    last_global_seq_nr: u64,
}

impl Tables {
    /// スナップショット行の現在 version (行が無ければ 0 — genesis の期待値)。
    fn current_version(&self, aggregate_id: &IntentId) -> usize {
        self.snapshot.get(aggregate_id).map_or(0, |row| row.version)
    }

    /// 次の global 通番を採番する (C6 の `AUTOINCREMENT` に相当。1 から始まる)。
    const fn next_global_seq_nr(&mut self) -> GlobalSeqNr {
        self.last_global_seq_nr = self.last_global_seq_nr.saturating_add(1);
        GlobalSeqNr::new(self.last_global_seq_nr)
    }
}

/// `EventStore` + `JournalReader` の in-memory 実装。3 表を**単一所有**する。
///
/// SQLite 実装と同じく内部可変性を持たない — 読取は `&self`、書込は `&mut self` である
/// (`coding-rules/interior-mutability.md`)。`clone` は 3 表を丸ごと写した**独立した別の
/// ストア**であり、同じ可変状態を指す別ハンドルではない。「同じストアを別のインスタンスから
/// 開き直す」(= 別プロセスからの再オープン、SQLite 実装で同じファイルを開き直すこと) は、
/// 書き終えた 3 表を引き継いだ写しで表す。
#[derive(Debug, Clone, Default)]
pub struct InMemoryEventStore {
    tables: Tables,
}

/// 行の材料を添えて `Corrupt` を組む。
fn corrupt_error(
    aggregate_id: &IntentId,
    seq_nr: Option<usize>,
    cause: CorruptCause,
) -> EventStoreError {
    EventStoreError::Corrupt {
        aggregate_id: aggregate_id.as_str().to_string(),
        seq_nr,
        cause,
    }
}

impl InMemoryEventStore {
    /// 空のストアを作る。
    #[must_use]
    pub fn new() -> InMemoryEventStore {
        InMemoryEventStore::default()
    }

    /// ジャーナル行をイベントへ復号する (封筒は列から組む — functional-spec §4.1)。
    fn decode_event(
        aggregate_id: &IntentId,
        seq_nr: usize,
        row: &JournalRow,
    ) -> Result<WorkflowExecutionEvent, EventStoreError> {
        let payload = EventPayloadWire::decode(
            aggregate_id.as_str(),
            seq_nr,
            row.schema_version,
            &row.event_type,
            &row.payload,
        )?;
        let occurred_at = DateTime::parse_from_rfc3339(&row.occurred_at)
            .map(|at| at.with_timezone(&Utc))
            .map_err(|_| {
                corrupt_error(aggregate_id, Some(seq_nr), CorruptCause::UndecodablePayload)
            })?;
        Ok(WorkflowExecutionEvent::new(
            aggregate_id.clone(),
            seq_nr,
            occurred_at,
            payload,
        ))
    }

    /// スナップショット行を集約へ復元し、行の version を載せる (BR1.2 の前半)。
    fn decode_snapshot(
        aggregate_id: &IntentId,
        row: &SnapshotRow,
    ) -> Result<WorkflowExecution, EventStoreError> {
        let state = StateWire::decode(aggregate_id.as_str(), row.schema_version, &row.payload)?;
        let aggregate = WorkflowExecution::from_state(state).map_err(|_| {
            corrupt_error(
                aggregate_id,
                Some(row.seq_nr),
                CorruptCause::InvariantViolation,
            )
        })?;
        let mut aggregate = aggregate;
        aggregate.set_version(row.version);
        Ok(aggregate)
    }

    /// ジャーナル行を 1 件も持たない集約か (`NotFound` と `MissingSnapshot` の区別 — BR1.2)。
    pub(crate) fn journal_is_empty(&self, aggregate_id: &IntentId) -> bool {
        self.tables
            .journal
            .range((Bound::Included((aggregate_id.clone(), 0)), Bound::Unbounded))
            .take_while(|((id, _), _)| id == aggregate_id)
            .next()
            .is_none()
    }
}

impl EventStore<IntentId, WorkflowExecution, WorkflowExecutionEvent> for InMemoryEventStore {
    async fn persist_event(
        &mut self,
        event: &WorkflowExecutionEvent,
        version: usize,
    ) -> Result<(), EventStoreError> {
        let aggregate_id = event.aggregate_id();
        let payload =
            EventPayloadWire::encode(aggregate_id.as_str(), event.seq_nr(), event.payload())?;
        let tables = &mut self.tables;

        // スナップショットは更新しないが、追記の前提としての楽観 version は検査する
        // (event-store-adapter-rs の `persist_event(event, version)` と同じ意味論)。
        let actual = tables.current_version(aggregate_id);
        if actual != version {
            return Err(EventStoreError::Conflict {
                expected: version,
                actual,
            });
        }
        let key = (aggregate_id.clone(), event.seq_nr());
        if tables.journal.contains_key(&key) {
            return Err(EventStoreError::Conflict {
                expected: version,
                actual,
            });
        }
        let global_seq_nr = tables.next_global_seq_nr();
        tables.journal.insert(
            key,
            JournalRow {
                global_seq_nr,
                schema_version: EventPayloadWire::SCHEMA_VERSION,
                event_type: EventPayloadWire::event_type(event.payload()).to_string(),
                payload,
                occurred_at: event
                    .occurred_at()
                    .to_rfc3339_opts(SecondsFormat::Secs, true),
            },
        );
        Ok(())
    }

    async fn persist_event_and_snapshot(
        &mut self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), EventStoreError> {
        let aggregate_id = event.aggregate_id();
        let expected = aggregate.version();
        let new_version = event.seq_nr();
        let event_payload =
            EventPayloadWire::encode(aggregate_id.as_str(), new_version, event.payload())?;
        // 書込後の姿を写すので、payload の version も列と同じ新 version に揃える
        // (SQLite 実装と同形 — 行の中身が実装ごとに違うと契約テストの意味が薄れる)。
        let state_payload = {
            let mut stored = aggregate.clone();
            stored.set_version(new_version);
            StateWire::encode(aggregate_id.as_str(), &stored.state())?
        };

        let tables = &mut self.tables;
        let actual = tables.current_version(aggregate_id);

        // (1) journal の UNIQUE(aggregate_id, seq_nr)。
        let key = (aggregate_id.clone(), new_version);
        if tables.journal.contains_key(&key) {
            return Err(EventStoreError::Conflict { expected, actual });
        }
        // (2) snapshot は genesis だけ INSERT、それ以外は `WHERE version = expected` の UPDATE。
        if expected == 0 {
            if tables.snapshot.contains_key(aggregate_id) {
                return Err(EventStoreError::Conflict { expected, actual });
            }
        } else if actual != expected {
            return Err(EventStoreError::Conflict { expected, actual });
        }

        // 検査をすべて通してから書く (Tx と同じ観測 — 失敗は状態を変えない)。
        let global_seq_nr = tables.next_global_seq_nr();
        tables.journal.insert(
            key,
            JournalRow {
                global_seq_nr,
                schema_version: EventPayloadWire::SCHEMA_VERSION,
                event_type: EventPayloadWire::event_type(event.payload()).to_string(),
                payload: event_payload,
                occurred_at: event
                    .occurred_at()
                    .to_rfc3339_opts(SecondsFormat::Secs, true),
            },
        );
        tables.snapshot.insert(
            aggregate_id.clone(),
            SnapshotRow {
                version: new_version,
                seq_nr: new_version,
                schema_version: StateWire::SCHEMA_VERSION,
                payload: state_payload,
            },
        );
        Ok(())
    }

    async fn get_latest_snapshot_by_id(
        &self,
        aid: &IntentId,
    ) -> Result<Option<WorkflowExecution>, EventStoreError> {
        match self.tables.snapshot.get(aid) {
            None => Ok(None),
            Some(row) => InMemoryEventStore::decode_snapshot(aid, row).map(Some),
        }
    }

    async fn get_events_by_id_since_seq_nr(
        &self,
        aid: &IntentId,
        seq_nr: usize,
    ) -> Result<Vec<WorkflowExecutionEvent>, EventStoreError> {
        self.tables
            .journal
            .range((Bound::Excluded((aid.clone(), seq_nr)), Bound::Unbounded))
            .take_while(|((id, _), _)| id == aid)
            .map(|((_, nr), row)| InMemoryEventStore::decode_event(aid, *nr, row))
            .collect()
    }
}

impl JournalReader for InMemoryEventStore {
    async fn events_after(
        &self,
        after: GlobalSeqNr,
    ) -> Result<Vec<(GlobalSeqNr, WorkflowExecutionEvent)>, EventStoreError> {
        let mut rows: Vec<(&IntentId, usize, &JournalRow)> = self
            .tables
            .journal
            .iter()
            .filter(|(_, row)| row.global_seq_nr > after)
            .map(|((id, nr), row)| (id, *nr, row))
            .collect();
        rows.sort_by_key(|(_, _, row)| row.global_seq_nr);
        rows.iter()
            .map(|(id, nr, row)| {
                InMemoryEventStore::decode_event(id, *nr, row)
                    .map(|event| (row.global_seq_nr, event))
            })
            .collect()
    }

    async fn checkpoint(
        &self,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, EventStoreError> {
        Ok(self
            .tables
            .checkpoint
            .get(projection)
            .copied()
            .unwrap_or(GlobalSeqNr::ZERO))
    }

    async fn advance_checkpoint(
        &mut self,
        projection: &ProjectionName,
        to: GlobalSeqNr,
    ) -> Result<(), EventStoreError> {
        let tables = &mut self.tables;
        let current = tables
            .checkpoint
            .get(projection)
            .copied()
            .unwrap_or(GlobalSeqNr::ZERO);
        if to < current {
            return Err(EventStoreError::CheckpointRegression {
                projection: projection.clone(),
                current,
                requested: to,
            });
        }
        tables.checkpoint.insert(projection.clone(), to);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::orchestration::{StageEntry, StartRequest, WorkflowExecutionEventPayload};
    use core_domain::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };

    const AT_TEXT: &str = "2026-08-23T00:00:00Z";

    /// 固定の発生時刻 (時計はテストが完全に決める)。
    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(AT_TEXT)
            .unwrap()
            .with_timezone(&Utc)
    }
    const INTENT: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn intent() -> IntentId {
        IntentId::parse(INTENT).unwrap()
    }

    fn genesis() -> (WorkflowExecution, WorkflowExecutionEvent) {
        WorkflowExecution::start_from_plan_unchecked(
            intent(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            &StartRequest::new("classic", "in-memory"),
            vec![
                StageEntry::new(
                    StageSlug::parse("state-init").unwrap(),
                    PhaseId::Initialization,
                    PlanAction::Execute,
                    false,
                ),
                StageEntry::new(
                    StageSlug::parse("intent-capture").unwrap(),
                    PhaseId::Ideation,
                    PlanAction::Execute,
                    false,
                ),
            ],
            at(),
        )
        .unwrap()
    }

    async fn seeded() -> InMemoryEventStore {
        let mut store = InMemoryEventStore::new();
        let (aggregate, event) = genesis();
        store
            .persist_event_and_snapshot(&event, &aggregate)
            .await
            .unwrap();
        store
    }

    #[tokio::test]
    async fn a_clone_carries_the_rows_but_not_the_mutable_state() {
        // `clone` は 3 表を引き継いだ**独立した**ストアである。同じ可変状態を指す別
        // ハンドルを配ると `&mut self` の排他性が嘘になる (interior-mutability.md)。
        let mut store = seeded().await;
        let copy = store.clone();
        assert!(
            copy.get_latest_snapshot_by_id(&intent())
                .await
                .unwrap()
                .is_some(),
            "写した時点の行は引き継ぐ"
        );

        let mut aggregate = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap()
            .unwrap();
        let next = aggregate.complete_stage(at()).unwrap();
        store.persist_event(&next, 1).await.unwrap();

        assert_eq!(
            store
                .get_events_by_id_since_seq_nr(&intent(), 0)
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            copy.get_events_by_id_since_seq_nr(&intent(), 0)
                .await
                .unwrap()
                .len(),
            1,
            "写した後の追記は写しに及ばない"
        );
    }

    #[tokio::test]
    async fn the_snapshot_row_and_its_payload_both_carry_the_new_version() {
        // 集約は遷移で version を動かさないので、書込側が `set_version(新 version)` を
        // 載せてから写す。列と payload が食い違わないので、`find_by_id` の
        // `set_version(列の version)` は載せ替えではなく確認になる (BR1.2)。
        let store = seeded().await;
        let found = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.version(), 1);
        assert_eq!(found.seq_nr(), 1);
        assert!(
            store
                .tables
                .snapshot
                .get(&intent())
                .unwrap()
                .payload
                .contains("\"version\":1"),
            "payload の version も新 version"
        );
    }

    #[tokio::test]
    async fn the_append_only_write_checks_the_optimistic_version_without_touching_the_snapshot() {
        let mut store = seeded().await;
        let mut aggregate = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap()
            .unwrap();
        let next = aggregate.complete_stage(at()).unwrap();

        let err = store.persist_event(&next, 0).await.unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Conflict {
                expected: 0,
                actual: 1
            }
        );

        store.persist_event(&next, 1).await.unwrap();
        let events = store
            .get_events_by_id_since_seq_nr(&intent(), 0)
            .await
            .unwrap();
        assert_eq!(events.len(), 2, "ジャーナルには追記されている");
        let snapshot = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.version(), 1, "スナップショットは動かない");
    }

    #[tokio::test]
    async fn a_journal_row_without_a_snapshot_row_is_detectable() {
        // `persist_event` はスナップショットを書かない — Repository 側の
        // `Corrupt(MissingSnapshot)` はこの状態から起きる (BR1.2)。
        let mut store = InMemoryEventStore::new();
        let (_, event) = genesis();
        store.persist_event(&event, 0).await.unwrap();
        assert!(
            store
                .get_latest_snapshot_by_id(&intent())
                .await
                .unwrap()
                .is_none()
        );
        assert!(!store.journal_is_empty(&intent()));
    }

    #[tokio::test]
    async fn a_snapshot_row_whose_payload_cannot_be_decoded_is_corrupt() {
        // 行を直接壊せるのは実装の内部だけ (公開の破壊フックは置かない)。
        let mut store = seeded().await;
        if let Some(row) = store.tables.snapshot.get_mut(&intent()) {
            row.payload = "{not json".to_string();
        }
        let err = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            EventStoreError::Corrupt {
                cause: CorruptCause::UndecodablePayload,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn a_snapshot_row_written_with_another_schema_version_is_corrupt() {
        let mut store = seeded().await;
        if let Some(row) = store.tables.snapshot.get_mut(&intent()) {
            row.schema_version = 2;
        }
        let err = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            EventStoreError::Corrupt {
                cause: CorruptCause::SchemaVersion,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn a_journal_row_with_an_unknown_event_type_is_corrupt() {
        let mut store = seeded().await;
        if let Some(row) = store.tables.journal.get_mut(&(intent(), 1)) {
            row.event_type = "Exploded".to_string();
        }
        let err = store
            .get_events_by_id_since_seq_nr(&intent(), 0)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            EventStoreError::Corrupt {
                cause: CorruptCause::UnknownEventType,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn a_journal_row_keeps_its_envelope_columns_through_the_round_trip() {
        let store = seeded().await;
        let events = store
            .get_events_by_id_since_seq_nr(&intent(), 0)
            .await
            .unwrap();
        let event = events.first().unwrap();
        assert_eq!(event.aggregate_id(), &intent());
        assert_eq!(event.seq_nr(), 1);
        assert_eq!(event.occurred_at(), &at());
        assert!(matches!(
            event.payload(),
            WorkflowExecutionEventPayload::Started(_)
        ));
    }

    #[tokio::test]
    async fn the_global_sequence_starts_at_one_and_never_repeats() {
        let mut store = seeded().await;
        let mut aggregate = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap()
            .unwrap();
        let next = aggregate.complete_stage(at()).unwrap();
        store
            .persist_event_and_snapshot(&next, &aggregate)
            .await
            .unwrap();
        let rows = store.events_after(GlobalSeqNr::ZERO).await.unwrap();
        assert_eq!(
            rows.iter().map(|(g, _)| g.to_u64()).collect::<Vec<_>>(),
            [1, 2]
        );
    }

    #[tokio::test]
    async fn a_snapshot_row_that_breaks_an_aggregate_invariant_is_corrupt() {
        // 復号 (検査点 1 / 2) は通るが集約不変条件 (検査点 3) が破れている行。
        // `seq_nr = 0` は「イベントを 1 件も適用していない集約の写し」で、
        // 書込経路からは決して生まれない (BR1.2)。
        let mut store = seeded().await;
        {
            let row = store
                .tables
                .snapshot
                .get_mut(&intent())
                .expect("genesis の写し");
            assert!(row.payload.contains(r#""seq_nr":1"#), "{}", row.payload);
            row.payload = row.payload.replace(r#""seq_nr":1"#, r#""seq_nr":0"#);
        }
        let err = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Corrupt {
                aggregate_id: INTENT.to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::InvariantViolation,
            },
            "材料は行の seq_nr であって payload の値ではない"
        );
    }

    #[tokio::test]
    async fn appending_the_same_sequence_twice_conflicts_even_when_the_version_still_matches() {
        // `persist_event` はスナップショットを動かさないので、2 回目も楽観 version は
        // 一致する。競合を検出するのは journal の UNIQUE(aggregate_id, seq_nr) だけである。
        let mut store = seeded().await;
        let mut aggregate = store
            .get_latest_snapshot_by_id(&intent())
            .await
            .unwrap()
            .unwrap();
        let next = aggregate.complete_stage(at()).unwrap();
        store.persist_event(&next, 1).await.unwrap();

        let err = store.persist_event(&next, 1).await.unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Conflict {
                expected: 1,
                actual: 1
            }
        );
        let events = store
            .get_events_by_id_since_seq_nr(&intent(), 0)
            .await
            .unwrap();
        assert_eq!(events.len(), 2, "拒否された追記は行を増やさない");
    }

    #[tokio::test]
    async fn a_genesis_write_against_an_existing_snapshot_conflicts() {
        // 期待 version 0 (= genesis) なのにスナップショット行が既にある。SQLite 実装で
        // `INSERT INTO snapshot` が主キー違反になるのと同じ観測にする。
        let mut store = seeded().await;
        let (aggregate, _) = genesis();
        let mut advanced = aggregate.clone();
        let second = advanced.complete_stage(at()).unwrap();
        assert_eq!(second.seq_nr(), 2, "journal の UNIQUE には掛からない通番");

        let err = store
            .persist_event_and_snapshot(&second, &aggregate)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Conflict {
                expected: 0,
                actual: 1
            }
        );
    }

    #[tokio::test]
    async fn a_write_from_a_stale_version_conflicts_with_the_stored_one() {
        // 期待 version が 0 以外で、スナップショット列の version と食い違う場合。
        // SQLite 実装の `UPDATE ... WHERE version = expected` が 0 行になる場合に当たる。
        let mut store = seeded().await;
        let (mut stale, _) = genesis();
        stale.set_version(5);
        let bogus =
            WorkflowExecutionEvent::new(intent(), 6, at(), WorkflowExecutionEventPayload::Unparked);

        let err = store
            .persist_event_and_snapshot(&bogus, &stale)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Conflict {
                expected: 5,
                actual: 1
            }
        );
    }

    #[tokio::test]
    async fn a_version_beyond_the_json_exact_limit_is_refused_before_anything_is_written() {
        // 書込後の姿の `version` は列と同じ新 version なので、JSON が正確に保てない大きさは
        // スナップショットの符号化で弾かれる (黙って丸めた行を書かない — NFR3.1)。
        const JSON_EXACT_INTEGER_LIMIT: usize = 9_007_199_254_740_992;
        let mut store = seeded().await;
        let (aggregate, _) = genesis();
        let bogus = WorkflowExecutionEvent::new(
            intent(),
            JSON_EXACT_INTEGER_LIMIT + 1,
            at(),
            WorkflowExecutionEventPayload::Unparked,
        );

        let err = store
            .persist_event_and_snapshot(&bogus, &aggregate)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Corrupt {
                aggregate_id: INTENT.to_string(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            }
        );
        let events = store
            .get_events_by_id_since_seq_nr(&intent(), 0)
            .await
            .unwrap();
        assert_eq!(events.len(), 1, "genesis の 1 件だけが残る");
    }
}
