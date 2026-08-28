//! ドメイン層 — 集約・Domain Primitive・純関数ドメインサービス。I/O なし・計装なし (01 §7)。
//! 依存は Published Language の型と純粋部品 (文言カタログ) のみ。
//!
//! - [`orchestration`] — 「次に何が起こるか」。集約ルート `WorkflowExecution` は
//!   **イベントソーシング形の FSM** で、12 のコマンドがそれぞれ 1 つのドメインイベントを起こし、
//!   `apply_event` が通常実行とリプレイの唯一の状態遷移経路になる (ADR-001 / ADR-002)。
//!   ゲート判定はステージのフェーズで決まり (`gated = phase != initialization`)、時計・乱数・環境を
//!   読まない純粋な同期コードである。
//! - [`workflow_definition`] — 「何を実行しうるか」。コンパイル済みグラフ・スコープグリッド・
//!   スコープ identity の読取モデル。`WorkflowExecution` は定義を `WorkflowDefinitionId` で
//!   間接参照し、オブジェクトは保持しない (ADR-008)。
//! - [`workspace`] — 記録ツリーの語彙 (checkbox・状態ファイル・ロックプロトコル)。
//!
//! 正準 JSON 化はゲートウェイ層の責務である (BR5.2) — 観測互換のワイヤ形式はアダプタ層の
//! ワイヤ構造体が持ち、ドメイン型はそれを知らない。ただし `orchestration` の集約・
//! ドメインイベント・集約識別子は本家 event-store-adapter-rs の trait を**直接実装する**
//! ため、その境界要求として serde と chrono に依存する (ADR-010 Conformist)。
//! この serde は**表現の写し**であって、コマンドを迂回して状態を作る口ではない。

#![forbid(unsafe_code)]

pub mod orchestration;
pub mod workflow_definition;
pub mod workspace;
