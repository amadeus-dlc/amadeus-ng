//! `JournalReader` ポート — 投影 (U4) が使う差分読取とチェックポイント (C3 / C6)。

use core_domain::orchestration::WorkflowExecutionEvent;

use super::event_store_error::EventStoreError;
use super::global_seq_nr::GlobalSeqNr;
use super::projection_name::ProjectionName;

/// 投影 (U4) が使う差分読取とチェックポイント (C3 / C6)。
///
/// ストア実装が [`EventStore`] と同時に実装する。真実源はジャーナルであり、投影は
/// 「チェックポイント以降を読んで描き、チェックポイントを進める」だけで冪等に追いつける
/// — その 2 性質 (順序・単調性) を本ポートが保証する (BR1.4 / NFR3.4)。
///
/// メソッドは `async fn` (AFIT)。`dyn` は使わず、`Send` / `Sync` 境界も要求しない。
///
/// [`EventStore`]: super::event_store::EventStore
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              自動 trait 境界を書けないという注意喚起は本 trait では設計どおりである。"
)]
pub trait JournalReader {
    /// `after` **より大きい** global 通番の行を昇順で返す (全集約横断)。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、復号不能・未知 `type` (`Corrupt`)、版不一致 (`Schema`) を返す。
    async fn events_after(
        &self,
        after: GlobalSeqNr,
    ) -> Result<Vec<(GlobalSeqNr, WorkflowExecutionEvent)>, EventStoreError>;

    /// 投影のチェックポイントを読む。未登録の投影は [`GlobalSeqNr::ZERO`]。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、版不一致 (`Schema`) を返す。
    async fn checkpoint(&self, projection: &ProjectionName)
    -> Result<GlobalSeqNr, EventStoreError>;

    /// チェックポイントを `to` へ進める。同値は no-op、現在値未満は拒否 (単調 — BR1.4)。
    ///
    /// 巻き戻し (投影の再生成) は行削除で行う。本ポートには後退の口を置かない。
    ///
    /// # Errors
    ///
    /// 現在値未満への要求 (`CheckpointRegression`)、ストア I/O (`Io`)、版不一致 (`Schema`)
    /// を返す。
    async fn advance_checkpoint(
        &mut self,
        projection: &ProjectionName,
        to: GlobalSeqNr,
    ) -> Result<(), EventStoreError>;
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::orchestration::{EventStoreError, GlobalSeqNr, ProjectionName};
    use core_domain::orchestration::{
        IntentId, WorkflowExecutionEvent, WorkflowExecutionEventPayload,
    };
    use std::collections::BTreeMap;

    fn intent() -> IntentId {
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
    }

    fn event(seq_nr: u64) -> WorkflowExecutionEvent {
        WorkflowExecutionEvent::new(
            intent(),
            seq_nr,
            "2026-08-23T00:00:00Z",
            WorkflowExecutionEventPayload::Unparked,
        )
    }

    /// trait の形を固定するための最小実装 (単調性の規則も持つ — 契約テストは adapter 側)。
    #[derive(Debug, Default)]
    struct FakeReader {
        journal: Vec<(GlobalSeqNr, WorkflowExecutionEvent)>,
        checkpoints: BTreeMap<ProjectionName, GlobalSeqNr>,
    }

    impl JournalReader for FakeReader {
        async fn events_after(
            &self,
            after: GlobalSeqNr,
        ) -> Result<Vec<(GlobalSeqNr, WorkflowExecutionEvent)>, EventStoreError> {
            Ok(self
                .journal
                .iter()
                .filter(|(global, _)| *global > after)
                .cloned()
                .collect())
        }

        async fn checkpoint(
            &self,
            projection: &ProjectionName,
        ) -> Result<GlobalSeqNr, EventStoreError> {
            Ok(self
                .checkpoints
                .get(projection)
                .copied()
                .unwrap_or(GlobalSeqNr::ZERO))
        }

        async fn advance_checkpoint(
            &mut self,
            projection: &ProjectionName,
            to: GlobalSeqNr,
        ) -> Result<(), EventStoreError> {
            let current = self
                .checkpoints
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
            self.checkpoints.insert(projection.clone(), to);
            Ok(())
        }
    }

    fn reader() -> FakeReader {
        FakeReader {
            journal: vec![
                (GlobalSeqNr::new(1), event(1)),
                (GlobalSeqNr::new(2), event(2)),
            ],
            checkpoints: BTreeMap::new(),
        }
    }

    fn projection() -> ProjectionName {
        ProjectionName::parse("state-file").unwrap()
    }

    #[tokio::test]
    async fn reading_from_zero_returns_the_whole_journal_in_ascending_order() {
        let reader = reader();
        let rows = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
        assert_eq!(
            rows.iter().map(|(g, _)| g.value()).collect::<Vec<_>>(),
            [1, 2]
        );
    }

    #[tokio::test]
    async fn reading_after_a_position_returns_only_the_difference() {
        let reader = reader();
        let rows = reader.events_after(GlobalSeqNr::new(1)).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, GlobalSeqNr::new(2));
    }

    #[tokio::test]
    async fn an_unregistered_projection_reads_as_zero() {
        let reader = reader();
        assert_eq!(
            reader.checkpoint(&projection()).await.unwrap(),
            GlobalSeqNr::ZERO
        );
    }

    #[tokio::test]
    async fn the_checkpoint_advances_and_is_readable_again() {
        let mut reader = reader();
        reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(2))
            .await
            .unwrap();
        assert_eq!(
            reader.checkpoint(&projection()).await.unwrap(),
            GlobalSeqNr::new(2)
        );
    }

    #[tokio::test]
    async fn moving_the_checkpoint_backwards_is_a_regression() {
        let mut reader = reader();
        reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(2))
            .await
            .unwrap();
        let err = reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1))
            .await
            .unwrap_err();
        assert_eq!(
            err,
            EventStoreError::CheckpointRegression {
                projection: projection(),
                current: GlobalSeqNr::new(2),
                requested: GlobalSeqNr::new(1),
            }
        );
    }
}
