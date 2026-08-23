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
//! serde には依存しない — JSON 化・正準化はゲートウェイ層の責務である (BR5.2)。

#![forbid(unsafe_code)]

pub mod orchestration;
pub mod workflow_definition;
pub mod workspace;
