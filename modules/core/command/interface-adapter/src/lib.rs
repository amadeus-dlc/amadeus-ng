//! **コマンド側**のインターフェイスアダプタ層 — Controllers / Presenters / Gateways。
//! I/O 責務はここ (01 §7)。
//!
//! 読取・投影の実装（`JournalReaderImpl`・投影ライタ）は中間クレート
//! `core-read-model-updater` (RMU) が丸ごと所有する。コマンド側のクレートがそれを同居させて
//! はならず、`Cargo.toml` に RMU が現れたら違反である
//! (2026-08-29 オーナー裁定 — `coding-rules/cqrs-boundaries.md`)。
//!
//! 境界づけられたコンテキスト (`orchestration`) 直下に Gateway (= Repository 実装と外部
//! システムクライアント) を置く。時計は Gateway ではなく**横断機構**なので、コンテキストに
//! 属さないクレート root の機構モジュール (`clock`) に置く
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
//!
//! 機構モジュールの mod は private。公開 API は下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_command_interface_adapter::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

#![forbid(unsafe_code)]

mod clock;

pub mod orchestration;

// 横断機構の注入シーム (Gateway ではない)
pub use clock::{Clock, FakeClock, SystemClock};
