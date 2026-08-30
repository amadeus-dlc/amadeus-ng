//! 合成ルート (U7) のライブラリ面 — **両側を知る**読取 loader。
//!
//! `stage-graph.json` は compile コンテキストのイベント投影 = リードモデルであり、コマンド側
//! から読むのは CQRS 違反 — 読めるのは両側を知る合成ルートだけである (オーナー裁定
//! 2026-08-30 / issue #46)。memory 層のルールファイルも同じ理由でここが読む。読んだ生バイトは
//! アダプタの純 parse (`parse_workflow_definition`) とドメインの値 (`MemoryRules`) へ渡し、
//! ユースケースには `TurnMaterials` の**値**として届く — use-case のポートは Repository
//! 2 本だけである。
//!
//! バイナリ (main) の配線は U7 フェーズ A の課題で、本 lib はその読取部分を先に実体化する。

mod definition_loader;
mod steering_loader;

pub use definition_loader::{DefinitionPaths, load_workflow_definition};
pub use steering_loader::load_memory_rules;
