//! **コマンド側**のユースケース層 — CLI 動詞＝ユースケース、ポート (trait) 定義。
//! domain と PL のみに依存 (01 §7)。
//!
//! リードモデルを描く側は中間クレート `core-read-model-updater` (RMU) が丸ごと所有する。
//! 本クレートに**その型は 1 つも無く**、`Cargo.toml` に RMU もクエリ側クレートも現れない —
//! 境界はクレート分離で物理強制する (`coding-rules/cqrs-boundaries.md`)。

#![forbid(unsafe_code)]

pub mod orchestration;
