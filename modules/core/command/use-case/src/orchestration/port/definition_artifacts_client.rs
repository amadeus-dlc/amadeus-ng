//! `DefinitionArtifactsClient` ポート — ハーネス配布物 (Published Language 3 入力) の**取込境界**。
//!
//! # なぜ Repository ではないのか
//!
//! 相手はストアではなく**別システムが配った成果物**である。`stage-graph.json` /
//! `scope-grid.json` / `scopes/aidlc-<name>.md` は upstream の compile コンテキストが出力し、
//! ハーネスと一緒に配られたバイトであって、我々が書いた集約の永続化像ではない。したがって
//! 責務は Gateway 2 分類のうち**外部システムクライアント**である
//! (`coding-rules/gateway-taxonomy.md` §1 — 「別プロセス・別システムとの RPC」の同類。
//! 媒体がファイルシステムであることは実装の内部詳細)。
//!
//! この区別は `coding-rules/cqrs-boundaries.md` 規則 7 との関係で重要である。規則 7 が禁じるのは
//! **コマンド側が自分のリードモデル (RMU の投影物) を読む**ことであり、ここで読むのは
//! RMU が描いたものではなく**外部から来た配布物**である。compile コンテキストが本システムに
//! 実装された暁には、この取込は当該コンテキストのフロー (集約 → イベント → RMU) に置き換わり、
//! 本ポートは消える。
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

/// ハーネス配布物の取込口。
pub trait DefinitionArtifactsClient {
    /// 配布物を読んで、定義を確立するのに要る材料を返す。
    ///
    /// 引数を取らない — **どの定義を取り込むかは配布物自身が名乗る** (`harness.json` の
    /// `name` が定義 id の供給元。ADR-008)。呼出側が id を指定して照合するのではなく、
    /// 読めた identity をそのまま系譜 ID として使う。
    ///
    /// 動詞は相手方 (upstream compile 成果物) の Published Language の語である。
    /// `coding-rules/gateway-taxonomy.md` §2b の動詞禁止 (`load` / `get` / `fetch` を使わない)
    /// は **Repository の射程**であり、外部システムクライアントは相手方の語彙に従う
    /// (§1c の `EventStore` / `JournalReader` と同じ理屈 — オーナー裁定 2026-08-31)。
    ///
    /// # Errors
    ///
    /// OS 由来の読取失敗 (`Io`)、読めたが内容が壊れている (`Corrupt`) を返す。
    fn load(&self) -> Result<DefinitionArtifacts, DefinitionArtifactsError>;
}
