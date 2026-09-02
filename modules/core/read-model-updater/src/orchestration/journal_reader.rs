//! `JournalReader` ポート — 投影 (U4) が使う差分読取とチェックポイント (C3 / C6)。

use crate::read_tables::{ReadTables, SteeringTables};

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

    /// 構造化リードモデルの行を差し替え、チェックポイントを `to` へ進める。
    /// 同値は no-op、現在値未満は拒否 (単調 — BR1.4)。
    ///
    /// 巻き戻し (投影の再生成) は行削除で行う。本ポートには後退の口を置かない。
    ///
    /// # 行の差し替えと前進は 1 つの原子的な操作である
    ///
    /// `tables` は全履歴からの再計算の結果であり、書込は**全行の差し替え**である。それと
    /// チェックポイントの前進が別々にコミットされると、行だけ新しくてチェックポイントが
    /// 古い (次のキャッチアップで同じ差分をもう一度描く) か、逆に行が古いままチェック
    /// ポイントだけ進む (読取コマンドが永久に古い答えを見る) かのどちらかになる。
    /// したがって実装は両方を 1 トランザクションに閉じる (裁定 §3)。前進を拒否するときは
    /// 行も変えない。
    ///
    /// # Errors
    ///
    /// 現在値未満への要求 (`CheckpointRegression`)、ジャーナルに無い位置への要求や保存済み
    /// アンカーの食い違い (`Corrupt`)、ストア I/O (`Io`) を返す。
    async fn advance_checkpoint(
        &mut self,
        projection: &ProjectionName,
        to: GlobalSeqNr,
        tables: &ReadTables,
    ) -> Result<(), JournalReadError>;

    /// 保存済み steering 面が**どの参照入力から作られたか**。未投影なら `None`。
    ///
    /// 取得ループはこの値を、いま読んだ memory 層のダイジェストと比べる。同じなら
    /// steering の行に触らない。値は不透明で、等値比較だけが契約である。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`) を返す。
    async fn steering_source_digest(&self) -> Result<Option<String>, JournalReadError>;

    /// steering 面の行を差し替える (`read_steering_plan` / `read_steering_part`)。
    ///
    /// # チェックポイントとは束ねない
    ///
    /// この面は**参照入力由来**であり、ジャーナルの走査位置とは無関係に変わる。行が
    /// どの参照入力のものかは `tables` が運ぶ `source_digest` が言うので、チェックポイント
    /// と同じ Tx に閉じる理由が無い (裁定 §3 の対象はジャーナル由来の面である)。
    /// したがって実装はこれ**だけ**を 1 つの原子的な差し替えとして書く。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`) を返す。
    async fn replace_steering(&mut self, tables: &SteeringTables) -> Result<(), JournalReadError>;
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::orchestration::{GlobalSeqNr, JournalEntry, JournalReadError, ProjectionName};
    use chrono::{DateTime, Utc};
    use core_command_domain::orchestration::{
        IntentExecutionEvent, IntentExecutionEventId, IntentExecutionId, Unparked,
    };
    use std::collections::BTreeMap;

    /// b40 のテスト用固定イベント識別子 (同じ材料から組んだイベントを同値に保つため)。
    fn event_id() -> IntentExecutionEventId {
        IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").expect("UUIDv7")
    }

    /// b40 のテスト用集約識別子 (行の `aid` と payload の `aggregate_id` を揃える)。
    fn execution_id() -> IntentExecutionId {
        IntentExecutionId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
    }

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
            IntentExecutionEvent::Unparked(Unparked::new(event_id(), execution_id())),
        )
    }

    /// trait の形を固定するための最小実装 (単調性の規則も持つ — 契約テストは adapter 側)。
    #[derive(Debug, Default)]
    struct FakeReader {
        journal: Vec<JournalEntry>,
        checkpoints: BTreeMap<ProjectionName, GlobalSeqNr>,
        /// 最後に受け取った構造化リードモデル (前進と同じ呼出で届く)。
        tables: Option<ReadTables>,
        /// 最後に受け取った steering 面 (前進とは**別の**呼出で届く)。
        steering: Option<SteeringTables>,
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
            Ok(JournalBatch::new(rows, Vec::new(), Vec::new(), scanned_to))
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
            tables: &ReadTables,
        ) -> Result<(), JournalReadError> {
            let current = self
                .checkpoints
                .get(projection)
                .copied()
                .unwrap_or(GlobalSeqNr::ZERO);
            if to < current {
                // 拒否したら行も変えない (原子性の約束をフェイクも守る)。
                return Err(JournalReadError::CheckpointRegression {
                    projection: projection.clone(),
                    current,
                    requested: to,
                });
            }
            self.checkpoints.insert(projection.clone(), to);
            self.tables = Some(tables.clone());
            Ok(())
        }

        async fn steering_source_digest(&self) -> Result<Option<String>, JournalReadError> {
            Ok(self
                .steering
                .as_ref()
                .map(|tables| tables.source_digest().to_string()))
        }

        async fn replace_steering(
            &mut self,
            tables: &SteeringTables,
        ) -> Result<(), JournalReadError> {
            self.steering = Some(tables.clone());
            Ok(())
        }
    }

    fn journal_reader() -> FakeReader {
        FakeReader {
            journal: vec![entry(1), entry(2)],
            checkpoints: BTreeMap::new(),
            tables: None,
            steering: None,
        }
    }

    fn projection() -> ProjectionName {
        ProjectionName::parse("state-file").unwrap()
    }

    #[tokio::test]
    async fn reading_from_zero_returns_the_whole_journal_in_ascending_order() {
        let journal_reader = journal_reader();
        let batch = journal_reader
            .events_after(GlobalSeqNr::ZERO)
            .await
            .unwrap();
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
        let journal_reader = journal_reader();
        let batch = journal_reader
            .events_after(GlobalSeqNr::new(1))
            .await
            .unwrap();
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
        let journal_reader = journal_reader();
        assert_eq!(
            journal_reader.checkpoint(&projection()).await.unwrap(),
            GlobalSeqNr::ZERO
        );
    }

    /// 前進と同じ呼出で渡す構造化リードモデル (空の履歴からの投影で足りる)。
    fn tables() -> ReadTables {
        ReadTables::project(&JournalBatch::empty()).expect("空も投影できる")
    }

    #[tokio::test]
    async fn the_checkpoint_advances_and_is_readable_again() {
        let mut journal_reader = journal_reader();
        journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(2), &tables())
            .await
            .unwrap();
        assert_eq!(
            journal_reader.checkpoint(&projection()).await.unwrap(),
            GlobalSeqNr::new(2)
        );
    }

    /// 参照入力から作った steering 面 (空の memory 層で足りる)。
    fn steering_tables() -> SteeringTables {
        SteeringTables::pack(&crate::read_tables::MemoryRules::default()).expect("空も計画できる")
    }

    #[tokio::test]
    async fn an_unprojected_steering_face_has_no_source_yet() {
        let journal_reader = journal_reader();
        assert_eq!(
            journal_reader.steering_source_digest().await.unwrap(),
            None,
            "まだ 1 度も投影していない"
        );
    }

    #[tokio::test]
    async fn the_steering_face_is_replaced_on_its_own_and_names_its_source() {
        // 前進とは別の呼出である — ジャーナルの走査位置と参照入力は無関係に動く。
        let mut journal_reader = journal_reader();
        let tables = steering_tables();
        journal_reader.replace_steering(&tables).await.unwrap();
        assert_eq!(
            journal_reader.steering_source_digest().await.unwrap(),
            Some(tables.source_digest().to_string())
        );
        assert_eq!(
            journal_reader.checkpoint(&projection()).await.unwrap(),
            GlobalSeqNr::ZERO,
            "steering の差し替えはチェックポイントを動かさない"
        );
    }

    #[tokio::test]
    async fn the_rows_arrive_with_the_advance_not_in_a_separate_call() {
        // ポートの形そのものの確認 — 行と前進が同じ呼出で届くことが原子性の前提である。
        let mut journal_reader = journal_reader();
        assert!(journal_reader.tables.is_none());
        journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(2), &tables())
            .await
            .unwrap();
        assert_eq!(journal_reader.tables, Some(tables()));
    }

    #[tokio::test]
    async fn moving_the_checkpoint_backwards_is_a_regression_and_leaves_the_rows_alone() {
        let mut journal_reader = journal_reader();
        journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(2), &tables())
            .await
            .unwrap();
        let err = journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1), &tables())
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
