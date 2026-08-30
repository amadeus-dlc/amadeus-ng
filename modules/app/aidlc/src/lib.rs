//! 合成ルート (U7) のライブラリ面 — 両側を知る配線の読取部分。
//!
//! ワークフロー定義 (リードモデル `stage-graph.json` ほか) の読取は**クエリサイド**
//! (`core-query-interface-adapter`) が持つ (オーナー裁定 2026-08-30 — リードモデルを読む
//! 責務はクエリサイドの実装であり、コマンド側には置けない)。合成ルートはそれを呼んで、
//! memory 層のルールファイル (本 lib の loader) と合わせて `TurnMaterials` の**値**として
//! コマンドのユースケースへ渡す — ユースケース層から見るとポートは Gateway のみ、現在の
//! Gateway は Repository のみである。バイナリ (main) の配線は U7 フェーズ A の課題で、
//! 本 lib は memory 層の読取部分を先に実体化する。

mod steering_loader;

pub use steering_loader::load_memory_rules;
