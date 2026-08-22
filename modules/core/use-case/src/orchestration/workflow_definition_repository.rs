//! `WorkflowDefinitionRepository` ポート — 集約 `WorkflowDefinition` (12-workflow-definition
//! §2.1) の **load 専用 Repository**。集約は Published Language のコンパイル成果物であり
//! 本システムからは書き換えないため `save` を持たない (書き側 = compile はスライス 2 の
//! 別コンテキスト)。10-orchestration §3 のポート表に対応し、規範は 12-workflow-definition
//! §4 / §5 が所有する。
//!
//! Published Language の 3 入力
//! (`stage-graph.json` / `scope-grid.json` / `<harnessRoot>/scopes/aidlc-<name>.md`) を
//! **1 つの Repository で** 集約 `WorkflowDefinition` に束ねて供給する (compile が graph と
//! grid を lockstep で出すため、片方だけ新しい状態は upstream でも想定外)。
//!
//! 名前は「集約名＋Repository」規則に従う (docs/memory/gateway-taxonomy.md)。格納形式
//! (`stage-graph.json` というファイル名) は Repository **実装**の内部詳細であり、ポート名に
//! 現れてはならない。
//!
//! **失敗態度は 3 入力で意図的に非対称** (12 §4。この非対称そのものが観測可能な契約で、
//! 「より厳格にする」方向の改変も逸脱になる):
//!
//! - `stage-graph.json` が読めない / 不正 JSON = **fatal** (`Err`)。`AIDLC_STAGE_GRAPH` の
//!   オーバライドが効いているときだけ逐語文言の hint 節が切り替わる (#1・#2)。
//! - `scope-grid.json` が読めない / 不正 = **fatal にしない**。グラフの `scopes[]` からの
//!   転置導出へフォールバックする (#3)。したがって `load` はグリッド欠損では失敗しない。
//! - identity ファイルとグリッド列の不一致は**双方向とも正当** (#5 zero-EXECUTE な正当
//!   スコープ / #6 ランタイム不可視) であり、どちらもエラーにしない。
//! - いずれの失敗でも **stdout に何も書かない** (#10 — half-emitted directive を出さない)。
//!
//! 実装は `core-interface-adapter`
//! (`orchestration::WorkflowDefinitionRepositoryImpl` が実 I/O、
//! `orchestration::InMemoryWorkflowDefinitionRepository` がテストダブル)。パス解決と env
//! オーバライドの意味論、および逐語文言の組み立ては実装側に閉じる (12 §6)。ポートは
//! **材料だけ**を運ぶ。

use core_domain::workflow_definition::WorkflowDefinition;

/// 3 入力の読取失敗。逐語文言そのものは持たず、**文言を組み立てる材料**を運ぶ
/// (レンダリングはアダプタ層 — 12 §6)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphReadError {
    /// `stage-graph.json` が読めない (12 §4 #1)。
    ///
    /// `env_override` はパスが `AIDLC_STAGE_GRAPH` 由来かを表す。真のとき逐語文言の hint 節が
    /// 「unset して既定に戻せ」形へ切り替わる — **この分岐自体が観測可能な契約**。
    NotReadable {
        /// 読もうとした `stage-graph.json` の解決済みパス。
        path: String,
        /// 読取が失敗した理由 (OS 由来)。
        cause: String,
        /// `path` が `AIDLC_STAGE_GRAPH` 由来か。
        env_override: bool,
    },
    /// `stage-graph.json` が不正 JSON (12 §4 #2)。`NotReadable` とは別文言。
    InvalidJson {
        /// パースに失敗した `stage-graph.json` の解決済みパス。
        path: String,
        /// パースが失敗した理由 (JSON パーサ由来)。
        cause: String,
    },
    /// scope identity ファイルの読取・frontmatter 検証の失敗 (`name` 欠落・`skeleton` の
    /// 不正値など — 12 §3.3)。
    ScopeFile {
        /// 失敗の詳細。逐語文言そのものではなく、その材料。
        message: String,
    },
    /// JSON としては読めたがドメイン型へ写せない (未知 `phase`、文法外 `slug` など)。
    ///
    /// upstream はロード時に検証しないが、serde による構造的パースは「ロード時無検証」からの
    /// 逸脱ではなく補強として扱う (12 §10) — dist の正規データに対しては観測差が生じない。
    Malformed {
        /// 写像に失敗した箇所の詳細。逐語文言そのものではなく、その材料。
        message: String,
    },
}

/// 集約 `WorkflowDefinition` の Repository (load 専用)。
pub trait WorkflowDefinitionRepository {
    /// 3 入力を読み、集約 `WorkflowDefinition` を組み立てて返す。
    ///
    /// # Errors
    ///
    /// グラフの読取失敗 (`NotReadable`)、不正 JSON (`InvalidJson`)、scope identity の検証失敗
    /// (`ScopeFile`)、ドメイン型への写像失敗 (`Malformed`)。
    /// **グリッドの欠損・不正はエラーにしない** — 転置導出へフォールバックする (12 §4 #3)。
    fn load(&self) -> Result<WorkflowDefinition, GraphReadError>;
}
