//! workspace コンテキストの**投影 API** — リードモデル（`aidlc-state.md` と監査シャード）を
//! 描く側（11-workspace §2.3）。
//!
//! ES 化により、状態ファイル・監査ブロックの**描画**はドメイン層から投影の責務へ移った
//! （ADR-003 / ADR-004）。描くのは ReadModelUpdater であって、ドメイン層ではない。
//! ドメインに残るのは値オブジェクトの Always Valid 検証（`StateFieldValue` の単一行検査、
//! `EventType` の閉集合）と、集約に置けない横断の判断（`classify_state_version` /
//! `find_all_events`）である。
//!
//! 実装ファイルの mod は private。公開 API は `pub use` が唯一の宣言であり、消費側のパスは
//! `core_query_read_model_updater::workspace::<名前>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod audit_block;
mod audit_shard;
mod projection;
mod read_model;
mod state_file;
mod state_writers;

// 純粋投影核とその作業面（二層構造の下側 — ストレージを知らない）
pub use projection::{ProjectionError, project};
pub use read_model::ReadModel;

// 監査ブロックの描画（W9 の逐語契約。投影の行もフックの直接行も同じ描き手を通る）
pub use audit_block::{SHARD_HEADER, render_audit_block};

// 投影ライタ（リードモデルをディスクへ落とす 2 面。状態ファイルは置換、シャードは追記）
pub use audit_shard::{AuditShardWriteError, append as append_audit_shard};
pub use state_file::{
    StateFileReadError, StateFileWriteError, read as read_state_file,
    write_atomic as write_state_file,
};

// 状態ファイルの writer 4 種 + 読取（純粋な string→string — 11-workspace §2.3）
pub use state_writers::{
    FieldNotFound, HeadingNotFound, find_field, with_field, with_field_if_present,
    with_field_or_insert, without_field,
};
