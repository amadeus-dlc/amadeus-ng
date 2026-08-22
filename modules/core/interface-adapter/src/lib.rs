//! インターフェイスアダプタ層 — Controllers / Presenters / Gateways。I/O 責務はここ (01 §7)。infra-io に依存できる唯一の層。
//!
//! 境界づけられたコンテキスト (`orchestration` / `workspace`) 直下に Gateway
//! (= Repository 実装と外部システムクライアント) を置く。時計・プロセス生存判定は
//! Gateway ではなく**横断機構**なので、コンテキストに属さないクレート root の機構モジュール
//! (`clock` / `process_probe`) に置く (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md)。
//!
//! 機構モジュールの mod は private。公開 API は下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_interface_adapter::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

#![forbid(unsafe_code)]

mod clock;
mod process_probe;

pub mod orchestration;
pub mod workspace;

// 横断機構の注入シーム (Gateway ではない)
pub use clock::{Clock, FakeClock, SystemClock};
pub use process_probe::{FakeProcessProbe, OsProcessProbe, ProcessProbe};
