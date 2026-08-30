//! `JournalReader` ポート — 投影 (U4) が使う差分読取とチェックポイント (C3 / C6)。

use super::global_seq_nr::GlobalSeqNr;
use super::journal_batch::JournalBatch;
use super::journal_read_error::JournalReadError;
use super::projection_name::ProjectionName;

/// 投影 (U4) が使う差分読取とチェックポイント (C3 / C6)。
///
/// 集約の永続化そのもの (`IntentExecutionRepository`) とは**別の口**である — 本家
/// event-store-adapter-rs のイベントストアは集約単位の読み書きだけを担い、全集約横断の
/// 順序読取と投影チェックポイントは利用側の関心だからである (ADR-010 決定 4)。
///
/// 本 trait は RMU クレート (`core-read-model-updater`) が所有する
/// (ADR-009 2026-08-28 / 2026-08-29 追記 — 呼ぶのは RMU だけなので中立クレートへ切り出さない)。
/// RMU はライブラリ型を入口に出さないので、**本家の `EventEnvelope` をここから出さない** —
/// 行の材料は我々が所有する [`JournalEntry`] に写して返す。
///
/// 真実源はジャーナルであり、投影は「チェックポイント以降を読んで描き、チェックポイントを
/// 進める」だけで冪等に追いつける — その 2 性質 (順序・単調性) を本ポートが保証する
/// (BR1.4 / NFR3.4)。
///
/// メソッドは `async fn` (AFIT)。`dyn` は使わず、`Send` / `Sync` 境界も要求しない。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              自動 trait 境界を書けないという注意喚起は本 trait では設計どおりである。"
)]
pub trait JournalReader {
    /// `after` **より大きい** global 通番の行を昇順で走査して返す (全集約横断)。
    ///
    /// 返すのは [`JournalBatch`] — 実行のイベント行 ([`JournalEntry`]) と intent の誕生記録、
    /// そして走査済み最終位置の 3 つ組である。ジャーナルには実行と intent の 2 ストリームが
    /// 同居しており (issue #50)、チェックポイントは種別によらず走査済み最終位置まで進める
    /// (issue #56)。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、復号不能 (`Corrupt`) を返す。
    ///
    /// [`JournalEntry`]: super::journal_entry::JournalEntry
    async fn events_after(&self, after: GlobalSeqNr) -> Result<JournalBatch, JournalReadError>;

    /// 投影のチェックポイントを読む。未登録の投影は [`GlobalSeqNr::ZERO`]。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、保存済みチェックポイントがジャーナルの現況と食い違う
    /// (`Corrupt` — `CheckpointAnchorMismatch`) を返す。
    async fn checkpoint(
        &self,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, JournalReadError>;

    /// チェックポイントを `to` へ進める。同値は no-op、現在値未満は拒否 (単調 — BR1.4)。
    ///
    /// 巻き戻し (投影の再生成) は行削除で行う。本ポートには後退の口を置かない。
    ///
    /// # Errors
    ///
    /// 現在値未満への要求 (`CheckpointRegression`)、ジャーナルに無い位置への要求や保存済み
    /// アンカーの食い違い (`Corrupt`)、ストア I/O (`Io`) を返す。
    async fn advance_checkpoint(
        &mut self,
        projection: &ProjectionName,
        to: GlobalSeqNr,
    ) -> Result<(), JournalReadError>;
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::orchestration::{GlobalSeqNr, JournalEntry, JournalReadError, ProjectionName};
    use chrono::{DateTime, Utc};
    use core_command_domain::orchestration::{IntentExecutionEvent, IntentExecutionId};
    use std::collections::BTreeMap;

    fn intent() -> IntentExecutionId {
        IntentExecutionId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
    }

    fn entry(seq_nr: usize) -> JournalEntry {
        JournalEntry::new(
            GlobalSeqNr::new(seq_nr as u64),
            intent(),
            seq_nr,
            DateTime::parse_from_rfc3339("2026-08-23T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            IntentExecutionEvent::Unparked,
        )
    }

    /// trait の形を固定するための最小実装 (単調性の規則も持つ — 契約テストは adapter 側)。
    #[derive(Debug, Default)]
    struct FakeReader {
        journal: Vec<JournalEntry>,
        checkpoints: BTreeMap<ProjectionName, GlobalSeqNr>,
    }

    impl JournalReader for FakeReader {
        async fn events_after(&self, after: GlobalSeqNr) -> Result<JournalBatch, JournalReadError> {
            let rows: Vec<JournalEntry> = self
                .journal
                .iter()
                .filter(|entry| entry.global_seq() > after)
                .cloned()
                .collect();
            let scanned_to = rows.last().map(JournalEntry::global_seq);
            Ok(JournalBatch::new(rows, Vec::new(), scanned_to))
        }

        async fn checkpoint(
            &self,
            projection: &ProjectionName,
        ) -> Result<GlobalSeqNr, JournalReadError> {
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
        ) -> Result<(), JournalReadError> {
            let current = self
                .checkpoints
                .get(projection)
                .copied()
                .unwrap_or(GlobalSeqNr::ZERO);
            if to < current {
                return Err(JournalReadError::CheckpointRegression {
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
            journal: vec![entry(1), entry(2)],
            checkpoints: BTreeMap::new(),
        }
    }

    fn projection() -> ProjectionName {
        ProjectionName::parse("state-file").unwrap()
    }

    #[tokio::test]
    async fn reading_from_zero_returns_the_whole_journal_in_ascending_order() {
        let reader = reader();
        let batch = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
        assert_eq!(
            batch
                .executions()
                .iter()
                .map(|entry| entry.global_seq().to_u64())
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(batch.scanned_to(), Some(GlobalSeqNr::new(2)));
    }

    #[tokio::test]
    async fn reading_after_a_position_returns_only_the_difference() {
        let reader = reader();
        let batch = reader.events_after(GlobalSeqNr::new(1)).await.unwrap();
        let rows = batch.executions();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].global_seq(), GlobalSeqNr::new(2));
        assert_eq!(
            rows[0].execution_id(),
            &intent(),
            "集約識別子が境界を越える"
        );
        assert_eq!(rows[0].seq_nr(), 2);
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
            JournalReadError::CheckpointRegression {
                projection: projection(),
                current: GlobalSeqNr::new(2),
                requested: GlobalSeqNr::new(1),
            }
        );
    }
}
