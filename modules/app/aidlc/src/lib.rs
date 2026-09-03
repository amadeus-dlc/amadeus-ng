//! 合成ルート（U7）の**テスト可能な本体**。
//!
//! `main.rs` は配線だけの薄さに保ち（カバレッジ除外はあの 1 ファイルのみ —
//! `scripts/coverage.sh`）、パース・写像・描画・パス解決といった**判断を持つ部分は
//! すべてここへ置く**。除外領域に実ロジックを落とさないための分割である
//! （`coding-rules/cqrs-boundaries.md` 禁止パターン「駆動ループを合成ルートに置く」と
//! 同じ趣旨）。
//!
//! # ここが両側を知る唯一の場所である
//!
//! コマンド側（`core-command-*`）・クエリ側（`core-query-*`）・中間の RMU を同時に
//! `Cargo.toml` に書いてよいのは本クレートだけである（`coding-rules/cqrs-boundaries.md`
//! §対象外「合成ルートは両側を知る。それが合成ルートの仕事である」）。
//!
//! # モジュールの分担
//!
//! - [`cli`] — argv を型付きの要求へ写す（マルチコール解決とフラグのパース）
//! - [`layout`] — カーソルを読んでワークスペースの配置を決める
//! - [`execution_cursor`] — record が指す実行（`<record>/.aidlc-execution`）の読み書き。
//!   置き場は [`layout`] が決め、ファイル名はこちらが持つ（[`clone_identity`] /
//!   [`steering`] と同じ流儀）
//! - `turn` — `next` / `continue` の**構文的ルーティング**（どの引当をどの鍵で呼ぶか）
//! - `directive_drawing` — リードモデルの行を directive へ描く（相対パスの絶対化・
//!   1 行 JSON 列の展開・continue_token の中身）
//! - [`presenter`] — directive を stdout の 1 行 JSON へ描き、28KiB を守る
//! - [`steering`] — 継続トークンの封緘鍵の**置き場と鋳造方針**（機構は
//!   `core_infrastructure::secret_file`）
//! - [`wording`] — 逐語文言。ドメインとポートは材料しか運ばないので、文言を組むのは
//!   出す側であるここである（`coding-rules/error-handling.md`）

#![forbid(unsafe_code)]

pub mod cli;
pub mod clone_identity;
mod directive_drawing;
pub mod execution_cursor;
pub mod layout;
mod oversize_directive;
pub mod presenter;
pub mod record_name;
pub mod runtime;
pub mod scaffold;
pub mod steering;
mod turn;
pub mod wording;
