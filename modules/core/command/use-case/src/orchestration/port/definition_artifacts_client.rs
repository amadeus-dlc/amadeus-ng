//! `DefinitionArtifactsClient` ポート — ジャーナルの最初の 1 行を播種するための**暫定の足場**。
//!
//! # これは暫定の足場である (オーナー裁定 2026-09-01、#79 §5-g / b33)
//!
//! コマンド側が定義を読む正規の口は**集約 + リポジトリ**
//! (`WorkflowDefinitionRepository::find_by_id` = snapshot + journal replay) だけであり、
//! 第 3 の読取口は存在しない。本ポートは読取口ではなく **genesis の内容の出所**である —
//! compile コンテキストが未実装の現在、定義内容の唯一の出所が dist のバイト
//! (`stage-graph.json` / `scope-grid.json` / `scopes/aidlc-<name>.md`) なので、それを読んで
//! `define` (genesis) を一度だけ播種する。これが無いとジャーナルが空のままで
//! `find_by_id` は NotFound しか返せない。
//!
//! compile コンテキストが実装されれば (slice 2)、播種はそのフロー (集約 → イベント → RMU)
//! に置換され、**本ポートは実装ごと消える** (#80)。かつての「外部システムクライアント」
//! 分類は棄却済み (#79 §5-g — 3 入力は AI-DLC v2 系内の成果物であり、都合よく外部システム
//! 扱いしない)。消えるまでの間、改名・分類新設による恒久化はしない
//! (2026-09-01 裁定 — 「暫定の足場だとわかるように書く」)。
//!
//! # なぜ Repository ではないのか
//!
//! 集約の最新状態をファイルから組むのは `coding-rules/cqrs-boundaries.md` 規則 4 への正面
//! 違反である (b30 裁定)。本ポートが読むのは「集約の永続化像」ではなく「播種の材料」であり、
//! 確立・改訂してイベントストアへ書くのは `DefineWorkflowUseCase` の仕事。規則 7 (コマンド側が
//! 自分のリードモデルを読む禁止) との関係は規則 5「書くための読取」の暫定形であって、
//! 恒久の対象外条項ではない。
//!
//! # 何を返すのか
//!
//! 3 入力を読んで**定義を確立するのに要る材料一式**を返す。ドメイン型 (`StageGraph` /
//! `ScopeGrid` / `ScopeMetadata` / `DefinitionRevision` / `WorkflowDefinitionId`) への写像は
//! 実装が済ませる — 腐敗防止層はこの境界にある (`coding-rules/upstream-contracts.md`
//! 「食い違いは境界で変換する」)。
//!
//! 実装は `core-command-interface-adapter::orchestration::DefinitionArtifactsClientImpl`
//! (パス解決・JSON コーデック・frontmatter パーサ・内容版の算出をすべて所有する)。

use super::definition_artifacts::DefinitionArtifacts;
use super::definition_artifacts_error::DefinitionArtifactsError;

/// ジャーナル播種の材料取込口 (暫定の足場 — compile 実装で消える)。
pub trait DefinitionArtifactsClient {
    /// 配布物を読んで、定義を確立するのに要る材料を返す。
    ///
    /// 引数を取らない — **どの定義を取り込むかは配布物自身が名乗る** (`harness.json` の
    /// `name` が定義 id の供給元。ADR-008)。呼出側が id を指定して照合するのではなく、
    /// 読めた identity をそのまま系譜 ID として使う。
    ///
    /// `coding-rules/gateway-taxonomy.md` §2b の動詞禁止 (`load` / `get` / `fetch` を使わない)
    /// は **Repository の射程**であり、Repository ではない本ポートには及ばない
    /// (§1c の `EventStore` / `JournalReader` と同じ理屈 — オーナー裁定 2026-08-31)。
    ///
    /// # Errors
    ///
    /// OS 由来の読取失敗 (`Io`)、読めたが内容が壊れている (`Corrupt`) を返す。
    fn load(&self) -> Result<DefinitionArtifacts, DefinitionArtifactsError>;
}
