//! ドメイン層 — 集約・Domain Primitive・純関数ドメインサービス。I/O なし・計装なし (01 §7)。
//! 依存は Published Language の型と純粋部品 (文言カタログ) のみ。
//!
//! - [`orchestration`] — 「次に何が起こるか」。集約ルート `IntentExecution` は
//!   **イベントソーシング形の FSM** で、12 のコマンドがそれぞれ 1 つのドメインイベントを起こし、
//!   `apply_event` が通常実行とリプレイの唯一の状態遷移経路になる (ADR-001 / ADR-002)。
//!   ゲート判定はステージのフェーズで決まり (`gated = phase != initialization`)、時計・乱数・環境を
//!   読まない純粋な同期コードである。
//! - [`workflow_definition`] — 「何を実行しうるか」。コンパイル済みグラフ・スコープグリッド・
//!   スコープ identity の読取モデル。`IntentExecution` は定義を `WorkflowDefinitionId` で
//!   間接参照し、オブジェクトは保持しない (ADR-008)。
//! - [`workspace`] — 記録ツリーの語彙 (checkbox・状態ファイル・ロックプロトコル)。
//!
//! **このクレートは永続化知識から中立である** (改訂 9 /
//! `coding-rules/domain-persistence-neutrality.md`)。正準 JSON 化も、ジャーナル行・
//! スナップショット行のワイヤ形式も、ストアの trait 実装も、すべてアダプタ層 (書く側) と
//! RMU (読む側) の永続化 DTO が持つ。`Cargo.toml` に serde と event-store-adapter-rs が
//! **無いこと**がその機械強制であり、違反はビルドで落ちる。
//!
//! 依存に残る `chrono` は対象外である — 時刻の**値**は永続化知識ではない。

#![forbid(unsafe_code)]

pub mod orchestration;
pub mod workflow_definition;
pub mod workspace;
