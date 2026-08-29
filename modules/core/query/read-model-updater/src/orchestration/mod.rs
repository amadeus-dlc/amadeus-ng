//! orchestration コンテキストの**読取語彙と取得ループ** — 差分読取・投影チェックポイント
//! （C3 / C6）と、その上に立つ RMU の取得ループ。
//!
//! ポート（`JournalReader`）も SQLite 実装（`JournalReaderImpl`）も**このクレートが所有する**。
//! 呼ぶのは RMU だけであり、ジャーナルを読むことが RMU の仕事そのものだからである
//! （2026-08-28 / 2026-08-29 裁定 — ADR-009）。中立クレートへ切り出す必要は無い。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_query_read_model_updater::orchestration::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod corrupt_cause;
mod global_seq_nr;
mod journal_entry;
mod journal_read_error;
mod journal_reader;
mod journal_reader_impl;
mod projection_name;
mod store_failure;
mod updater;

// ポート (trait) と実 I/O 実装
pub use journal_reader::JournalReader;
pub use journal_reader_impl::JournalReaderImpl;

// 取得ループ (RMU コンポーネント本体 — 二層構造の上側)
pub use updater::{ProjectionTargets, ReadModelUpdater};

// Domain Primitive (永続化の通番と投影の名前)
pub use global_seq_nr::GlobalSeqNr;
pub use projection_name::ProjectionName;

// ポートが返す読取レコード (本家の封筒型はポートから出さない — ADR-009 2026-08-28 追記)
pub use journal_entry::JournalEntry;

// エラー
pub use corrupt_cause::CorruptCause;
pub use journal_read_error::JournalReadError;
pub use projection_name::ProjectionNameError;
pub use updater::CatchUpError;
