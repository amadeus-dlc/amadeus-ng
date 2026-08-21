//! ドメイン層 — 集約・Domain Primitive・純関数ドメインサービス。I/O なし・計装なし (01 §7)。
//! 依存は Published Language の型と純粋部品 (文言カタログ) のみ。

#![forbid(unsafe_code)]

pub mod orchestration;
pub mod workflow_definition;
pub mod workspace;
