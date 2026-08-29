//! **コマンド側**のユースケース層 — CLI 動詞＝ユースケース、ポート (trait) 定義。
//! domain と PL のみに依存 (01 §7)。
//!
//! クエリ側（リードモデルを描く側）は `core-query-read-model-updater` が丸ごと所有する。
//! 本クレートに**クエリ側の型は 1 つも無く**、`Cargo.toml` にクエリ側クレートも現れない —
//! 境界はクレート分離で物理強制する (`coding-rules/cqrs-boundaries.md`)。

#![forbid(unsafe_code)]

pub mod orchestration;
