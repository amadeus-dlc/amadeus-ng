//! 実行状態リードモデル (`aidlc-state.md`) のビュー型 — クエリ側が読むための自前モデル。
//!
//! 読む対象は RMU が投影した状態ファイルで、I/O と復号はクエリ側のインターフェイスアダプタ
//! (`core-query-interface-adapter`) が行う。本モジュールの型は**ファイル形式を知らない** —
//! Markdown の行文法も直列化の記述も持たず、検証済みの値だけを保持する。
//!
//! **命名**: リードモデル本体を表す集合型には `~View` 接尾辞を付け、クエリ側の語彙であることを
//! 明示する ([`ExecutionStateView`] / [`StageProgressView`])。その中で使う閉集合の語彙
//! ([`CheckboxState`] / [`ExecutionStatus`]) と位置の値 ([`StageIndex`]) は接尾辞を付けない —
//! データではなく語彙・索引であり、`workflow_view` の閉集合 (`PhaseView` 等) を再利用する
//! 側と綴りが揃う。拒否 (エラー) 型もビューではないので接尾辞を付けない。
//!
//! **DTO は DAO と同じ `port/` に同居する** — DTO/DAO ポートは一つのパッケージである
//! (オーナー裁定 2026-08-31)。本モジュールが返す型を読む契約 (`ExecutionStateDao`) は
//! 隣に住む。
//!
//! 型ファイルの mod も本モジュール自身も private。公開 API は以下の `pub use` を親
//! (`port` → `orchestration`) が中継したものが唯一の宣言であり、消費側のパスは
//! `core_query_use_case::orchestration::<型>` で安定する
//! (`coding-rules/module-visibility.md`)。

mod checkbox_state;
mod execution_state_view;
mod execution_status;
mod stage_index;
mod stage_progress_view;

// 閉集合の語彙と位置
pub use checkbox_state::CheckboxState;
pub use execution_status::ExecutionStatus;
pub use stage_index::StageIndex;

// リードモデル本体
pub use execution_state_view::ExecutionStateView;
pub use stage_progress_view::StageProgressView;

// 拒否 (ビューではないので `View` 接尾辞を付けない)
pub use execution_state_view::ExecutionStateError;
