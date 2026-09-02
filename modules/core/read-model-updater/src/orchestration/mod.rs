//! orchestration コンテキストの**読取語彙と取得ループ** — 差分読取・投影チェックポイント
//! （C3 / C6）と、その上に立つ RMU の取得ループ。
//!
//! ポート（`JournalReader`）も SQLite 実装（`JournalReaderImpl`）も**このクレートが所有する**。
//! 呼ぶのは RMU だけであり、ジャーナルを読むことが RMU の仕事そのものだからである
//! （2026-08-28 / 2026-08-29 裁定 — ADR-009）。中立クレートへ切り出す必要は無い。
//!
//! # 取得ループは 2 系統のリードモデルを 1 回で描く
//!
//! Markdown 面（系統 (1) — `aidlc-state.md` と監査シャード）は [`crate::workspace`] の投影核が
//! 描き、構造化面（系統 (2) — SQLite の `read_*` 表）は [`crate::read_tables`] の投影核が描く。
//! 後者の行は `advance_checkpoint` の引数として読み手へ渡り、**行の差し替えとチェックポイントの
//! 前進は 1 トランザクション**に閉じる（裁定 §3）。ポートが行を受け取る形になっているのは
//! そのためであり、行を書く別の口を並立させない。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_read_model_updater::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod catch_up_error;
mod corrupt_cause;
mod definition_entry;
mod dto;
mod global_seq_nr;
mod journal_batch;
mod journal_entry;
mod journal_read_error;
mod journal_reader;
mod journal_reader_impl;
mod projection_name;
mod projection_name_error;
mod projection_targets;
mod read_model_updater;
mod store_failure;

// ポート (trait) と実 I/O 実装
pub use journal_reader::JournalReader;
pub use journal_reader_impl::JournalReaderImpl;

// 取得ループ (RMU コンポーネント本体 — 二層構造の上側)
pub use projection_targets::ProjectionTargets;
pub use read_model_updater::ReadModelUpdater;

// Domain Primitive (永続化の通番と投影の名前)
pub use global_seq_nr::GlobalSeqNr;
pub use projection_name::ProjectionName;

// ポートが返す読取レコード (本家の封筒型はポートから出さない — ADR-009 2026-08-28 追記)
pub use definition_entry::DefinitionEntry;
pub use journal_batch::JournalBatch;
pub use journal_entry::JournalEntry;

// エラー
// 読む側の永続化 DTO (側ごと専用化 — coding-rules/cqrs-boundaries.md)。
pub use dto::{
    AutonomyModeSetDto, DtoDecodeError, GateApprovedDto, GateOpenedDto, GateRejectedDto,
    IntentEventDto, IntentExecutionEventDto, JumpedDto, ParkedDto, RecomposedDto,
    StageCompletedDto, StageRevisedDto, StageSkippedDto, StartedDto, WorkflowDefinitionEventDto,
};

pub use catch_up_error::CatchUpError;
pub use corrupt_cause::CorruptCause;
pub use journal_read_error::JournalReadError;
pub use projection_name_error::ProjectionNameError;
