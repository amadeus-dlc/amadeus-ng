//! `EventStore<AID, A, E>` ポート — event-store-adapter-rs 同形のローカル定義 (ADR-006 / C3)。

use super::event_store_error::EventStoreError;

/// event-store-adapter-rs と同形のイベントストア (ADR-006)。
///
/// Repository 実装が内部で使う**下位ポート**であり、ユースケースは直接消費しない
/// (消費するのは [`WorkflowExecutionRepository`])。ジェネリック 3 引数は
/// 集約識別子 `AID` / 集約 `A` / イベント `E` で、本 Unit の実装は
/// `EventStore<IntentId, WorkflowExecution, WorkflowExecutionEvent>` を満たす。
///
/// メソッドは `async fn` (AFIT — Rust 2024)。`dyn` は使わず (use-case-rules §2)、
/// `Send` / `Sync` 境界も要求しない (tokio current_thread 前提 — C3 / Q3 = A)。
/// 数値パラメータは C3 の `usize` を実装済みドメイン型に合わせて **u64** に具体化した
/// (C3 の改訂提案として所有者 U5 / U6 へ申し送り — BR1.1)。
///
/// [`WorkflowExecutionRepository`]: super::workflow_execution_repository::WorkflowExecutionRepository
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              自動 trait 境界を書けないという注意喚起は本 trait では設計どおりである。"
)]
pub trait EventStore<AID, A, E> {
    /// スナップショットを更新せずにイベントを 1 件追記する。
    ///
    /// `version` は書込側が前提とする楽観 version (= 永続化済みの最後の `seq_nr`)。
    /// 本 Unit の Repository は毎回スナップショットも更新する (ADR-001) ため使わないが、
    /// ポートの契約として実装・テストする。
    ///
    /// # Errors
    ///
    /// 楽観 version の不一致・`seq_nr` の重複 (`Conflict`)、ストア I/O (`Io`)、
    /// 符号化の失敗 (`Corrupt`) を返す。
    async fn persist_event(&mut self, event: &E, version: u64) -> Result<(), EventStoreError>;

    /// イベント 1 件と適用後の集約を**同一トランザクション**で永続化する (BR1.3 / BR2.3)。
    ///
    /// 期待 version は `aggregate` が持つ楽観 version (= `find_by_id` が載せた値)。
    ///
    /// # Errors
    ///
    /// 楽観 version の不一致・`seq_nr` の重複 (`Conflict`)、ストア I/O (`Io`)、
    /// 符号化の失敗 (`Corrupt`) を返す。
    async fn persist_event_and_snapshot(
        &mut self,
        event: &E,
        aggregate: &A,
    ) -> Result<(), EventStoreError>;

    /// 最新スナップショットを読む。行が無ければ `None`。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、復号不能・不変条件違反 (`Corrupt`)、版不一致 (`Schema`) を返す。
    async fn get_latest_snapshot_by_id(&self, aid: &AID) -> Result<Option<A>, EventStoreError>;

    /// 指定 `seq_nr` **より後**のイベントを `seq_nr` 昇順で読む。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、復号不能 (`Corrupt`)、版不一致 (`Schema`) を返す。
    async fn get_events_by_id_since_seq_nr(
        &self,
        aid: &AID,
        seq_nr: u64,
    ) -> Result<Vec<E>, EventStoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{CorruptCause, EventStoreError};
    use std::collections::BTreeMap;

    /// trait の形 (AFIT・`dyn` なし・`Send` 境界なし) が素の値で実装できることを固定する
    /// ためだけの最小実装。実挙動の契約はアダプタ層の契約テストが持つ。
    #[derive(Debug, Default)]
    struct FakeStore {
        journal: BTreeMap<(u64, u64), String>,
        snapshot: BTreeMap<u64, String>,
    }

    impl EventStore<u64, String, (u64, u64, String)> for FakeStore {
        async fn persist_event(
            &mut self,
            event: &(u64, u64, String),
            version: u64,
        ) -> Result<(), EventStoreError> {
            let (aid, seq_nr, body) = event;
            if self.snapshot.contains_key(aid) && version == 0 {
                return Err(EventStoreError::Conflict {
                    expected: version,
                    actual: 0,
                });
            }
            self.journal.insert((*aid, *seq_nr), body.clone());
            Ok(())
        }

        async fn persist_event_and_snapshot(
            &mut self,
            event: &(u64, u64, String),
            aggregate: &String,
        ) -> Result<(), EventStoreError> {
            let (aid, seq_nr, body) = event;
            self.journal.insert((*aid, *seq_nr), body.clone());
            self.snapshot.insert(*aid, aggregate.clone());
            Ok(())
        }

        async fn get_latest_snapshot_by_id(
            &self,
            aid: &u64,
        ) -> Result<Option<String>, EventStoreError> {
            Ok(self.snapshot.get(aid).cloned())
        }

        async fn get_events_by_id_since_seq_nr(
            &self,
            aid: &u64,
            seq_nr: u64,
        ) -> Result<Vec<(u64, u64, String)>, EventStoreError> {
            Ok(self
                .journal
                .iter()
                .filter(|((id, nr), _)| id == aid && *nr > seq_nr)
                .map(|((id, nr), body)| (*id, *nr, body.clone()))
                .collect())
        }
    }

    /// ジェネリック関数からポート越しに使えること (静的束縛 — `dyn` を要さない)。
    async fn append<S: EventStore<u64, String, (u64, u64, String)>>(
        store: &mut S,
        event: (u64, u64, String),
        aggregate: String,
    ) -> Result<(), EventStoreError> {
        store.persist_event_and_snapshot(&event, &aggregate).await
    }

    #[tokio::test]
    async fn the_snapshot_write_makes_the_aggregate_readable_again() {
        let mut store = FakeStore::default();
        append(&mut store, (1, 1, "e1".to_string()), "a1".to_string())
            .await
            .unwrap();
        assert_eq!(
            store.get_latest_snapshot_by_id(&1).await.unwrap(),
            Some("a1".to_string())
        );
    }

    #[tokio::test]
    async fn an_unknown_aggregate_has_no_snapshot() {
        let store = FakeStore::default();
        assert_eq!(store.get_latest_snapshot_by_id(&9).await.unwrap(), None);
    }

    #[tokio::test]
    async fn the_event_read_is_scoped_to_one_aggregate_and_starts_after_the_given_sequence() {
        let mut store = FakeStore::default();
        store
            .persist_event_and_snapshot(&(1, 1, "a".to_string()), &"s".to_string())
            .await
            .unwrap();
        store
            .persist_event_and_snapshot(&(1, 2, "b".to_string()), &"s".to_string())
            .await
            .unwrap();
        store
            .persist_event_and_snapshot(&(2, 1, "z".to_string()), &"s".to_string())
            .await
            .unwrap();
        let events = store.get_events_by_id_since_seq_nr(&1, 1).await.unwrap();
        assert_eq!(events, [(1, 2, "b".to_string())]);
    }

    #[tokio::test]
    async fn the_append_only_write_reports_a_conflict_as_the_shared_error_type() {
        let mut store = FakeStore::default();
        store
            .persist_event_and_snapshot(&(1, 1, "a".to_string()), &"s".to_string())
            .await
            .unwrap();
        let err = store
            .persist_event(&(1, 2, "b".to_string()), 0)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            EventStoreError::Conflict {
                expected: 0,
                actual: 0
            }
        );
    }

    #[tokio::test]
    async fn the_append_only_write_lands_in_the_journal_without_creating_a_snapshot() {
        // `persist_event` はスナップショットを更新しない (ポートの契約)。読み直したときに
        // 「ジャーナル行はあるがスナップショットが無い」状態が観測できることを固定する。
        let mut store = FakeStore::default();
        store
            .persist_event(&(1, 1, "a".to_string()), 0)
            .await
            .unwrap();
        assert_eq!(store.get_latest_snapshot_by_id(&1).await.unwrap(), None);
        assert_eq!(
            store.get_events_by_id_since_seq_nr(&1, 0).await.unwrap(),
            [(1, 1, "a".to_string())]
        );
    }

    #[tokio::test]
    async fn the_port_returns_the_shared_corrupt_material() {
        // `CorruptCause` は EventStore / Repository の両面が共有する材料である。
        let err = EventStoreError::Corrupt {
            aggregate_id: "1".to_string(),
            seq_nr: Some(2),
            cause: CorruptCause::MissingSnapshot,
        };
        assert!(matches!(
            err,
            EventStoreError::Corrupt {
                cause: CorruptCause::MissingSnapshot,
                ..
            }
        ));
    }
}
