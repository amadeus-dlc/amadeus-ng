//! `JournalReader` ポート — 投影 (U4) が使う差分読取とチェックポイント (C3 / C6)。

use crate::read_tables::{ReadTables, SteeringTables};

use super::global_seq_nr::GlobalSeqNr;
use super::journal_batch::JournalBatch;
use super::journal_read_error::JournalReadError;
use super::projection_name::ProjectionName;
use super::{CatchUpError, PublicationBatch};

/// 投影 (U4) が使う差分読取とチェックポイント (C3 / C6)。
///
/// 集約の永続化そのもの (`IntentExecutionRepository`) とは**別の口**である — 本家
/// event-store-adapter-rs のイベントストアは集約単位の読み書きだけを担い、全集約横断の
/// 順序読取と投影チェックポイントは利用側の関心だからである (ADR-010 決定 4)。
///
/// 本 trait は RMU クレート (`core-read-model-updater`) が所有する
/// (ADR-009 2026-08-28 / 2026-08-29 追記 — 呼ぶのは RMU だけなので中立クレートへ切り出さない)。
/// RMU はライブラリ型を入口に出さないので、**本家の `EventEnvelope` をここから出さない** —
/// 行の材料は我々が所有する [`JournalEntry`](super::JournalEntry) に写して返す。
///
/// 真実源はジャーナルである。差分の順序とチェックポイントの単調性に加え、ファイルの
/// 書込前後を保存した公開計画によって、確定前に停止しても同じ出力を二重追記しない
/// (BR1.4 / NFR3.4)。
///
/// メソッドは `async fn` (AFIT)。`dyn` は使わず、`Send` / `Sync` 境界も要求しない。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              自動 trait 境界を書けないという注意喚起は本 trait では設計どおりである。"
)]
pub trait JournalReader {
    /// 公開処理を始める前に、旧変換規約の共有面を現行履歴から再生成する。
    /// 読取専用のopenでは実行しない。
    ///
    /// # Errors
    /// 再生成時の履歴・投影・書込失敗を元の分類で返す。
    fn prepare_read_model(&mut self) -> Result<(), CatchUpError>;

    /// 未完了のファイル公開計画を取得する。
    ///
    /// # Errors
    /// 保存済み計画の読取・復号に失敗した場合。
    async fn pending_publication(
        &self,
        projection: &ProjectionName,
    ) -> Result<Option<PublicationBatch>, JournalReadError>;

    /// 指定した断面までの全履歴を読む。復旧時に新しいイベントを混ぜない。
    ///
    /// # Errors
    /// ジャーナルの読取・復号に失敗した場合。
    async fn events_through(&self, to: GlobalSeqNr) -> Result<JournalBatch, JournalReadError>;

    /// 計画を先に保存し、排他下でファイルを公開してから位置と構造化面を確定する。
    ///
    /// # Errors
    /// 計画・ファイル・確定操作の失敗。未完計画は再開用に保持する。
    async fn publish(
        &mut self,
        projection: &ProjectionName,
        batch: &PublicationBatch,
        tables: &ReadTables,
    ) -> Result<(), CatchUpError>;
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

    /// 構造化リードモデルを検証・公開し、チェックポイントを `to` へ進める。
    /// 共有面より新しければ差し替え、同値なら内容一致を確認し、古ければ共有面を維持する。
    /// 個別チェックポイントの現在値未満は拒否する (単調 — BR1.4)。
    ///
    /// 巻き戻し (投影の再生成) は行削除で行う。本ポートには後退の口を置かない。
    ///
    /// # 行の差し替えと前進は 1 つの原子的な操作である
    ///
    /// `tables` は指定位置までの全履歴からの再計算の結果であり、更新時は全行を差し替える。それと
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
