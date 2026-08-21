//! インターフェイスアダプタ層 — Controllers / Presenters / Gateways。I/O 責務はここ (01 §7)。infra-io に依存できる唯一の層。

#![forbid(unsafe_code)]

pub mod orchestration;
pub mod workspace;
