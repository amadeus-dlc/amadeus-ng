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

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use core_command_domain::workflow_definition::{
    DefinitionRevision, ScopeGrid, ScopeMetadata, StageGraph, WorkflowDefinitionId,
};

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

/// 取り込んだ配布物 — 定義を確立・改訂するための材料一式。
///
/// # これは集約の写し (memento 双子) ではない
///
/// フィールドの並びは集約 `WorkflowDefinition` の内容と一致するが、性格が違う
/// (`coding-rules/aggregate-commands.md` が禁じた `IntentSnapshot` 型との違い):
///
/// - **永続化の復号中間表現ではない。** 集約の保存像を読み戻すための型ではなく、
///   外部配布物を読んだ結果である。
/// - **これを引数に取るファクトリを作らない。** `Intent::from_material` のような
///   「genesis と同一署名の双子」は生やさず、ユースケースが分解して `define` / `redefine`
///   の引数へ渡す。
/// - ドメインではなくポート層に住み、ドメイン型の合成でしかない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionArtifacts {
    id: WorkflowDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
}

impl DefinitionArtifacts {
    /// 読み取った 3 入力のモデルと、それが名乗る系譜 ID・内容版を束ねる。
    #[must_use]
    pub const fn new(
        id: WorkflowDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
    ) -> DefinitionArtifacts {
        DefinitionArtifacts {
            id,
            revision,
            graph,
            grid,
            scopes,
        }
    }

    /// 配布物が名乗る系譜 ID (`harness.json` の `name` — ADR-008)。
    #[must_use]
    pub const fn id(&self) -> &WorkflowDefinitionId {
        &self.id
    }

    /// 読めた 3 入力の内容ダイジェスト。
    ///
    /// 「ディスクにあったバイトの版」ではなく「**読めた 3 入力の内容**の版」である —
    /// グリッドが欠けて転置導出へ倒れた場合も、導出結果を同じ形へ直列化して算出するので、
    /// 同じ内容の grid ファイルが置かれたときと同じ値になる。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevision {
        &self.revision
    }

    /// `stage-graph.json` 由来のステージグラフ (文書順を保持したまま)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// `scope-grid.json` 由来の静的 EXECUTE / SKIP グリッド (欠損時は転置導出)。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// スコープ `.md` 由来のメタデータ (スコープ名の辞書順)。有効スコープの権威。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }

    /// 材料を分解して手放す (`define` / `redefine` の引数へ渡すため)。
    ///
    /// 内容 3 点だけを返し、系譜 ID と内容版は返さない — 改訂は識別子を変えず、
    /// 内容版は呼出側が [`DefinitionArtifacts::revision`] で先に読むからである。
    #[must_use]
    pub fn into_content(self) -> (StageGraph, ScopeGrid, BTreeMap<String, ScopeMetadata>) {
        (self.graph, self.grid, self.scopes)
    }
}

/// 配布物を取り込めない形 (材料のみ — 逐語文言は出す側が組む)。
///
/// `Corrupt` の**分類は契約に載せない** (裁定 6 — エラーは契約の一部であり、内部実装が
/// バレる情報を含めない)。どのファイルがどう壊れていたかはアダプタ私有の型を
/// `Error::source` の連鎖で運ぶ。
///
/// **3 入力で失敗態度が非対称なことは実装の挙動として維持される** (12 §4。この非対称そのものが
/// 観測可能な契約で、「より厳格にする」方向の改変も逸脱になる):
///
/// - `harness.json` が読めない / 不正 JSON / `name` 欠落 = **fatal**。定義 id の供給元であり、
///   失われると集約に識別子を与えられない (ADR-008)。
/// - `stage-graph.json` が読めない / 不正 JSON = **fatal**。
/// - `scope-grid.json` が読めない / 不正 = **fatal にしない**。グラフの `scopes[]` からの
///   転置導出へフォールバックする。したがって `load` はグリッド欠損では失敗しない。
/// - identity ファイルとグリッド列の不一致は**双方向とも正当**であり、どちらもエラーにしない。
///
/// `source` が比較不能なため `PartialEq` は実装しない — テストは `matches!` と `source` の
/// 文字列確認で判定する。
#[derive(Debug)]
pub enum DefinitionArtifactsError {
    /// OS 由来の読取失敗 (欠損・権限・種別違い)。**内容の破損ではない**。
    Io {
        /// OS / ドライバ由来の分類。
        kind: ErrorKind,
        /// 読もうとしたパス。
        path: PathBuf,
    },
    /// 読めたが内容が壊れている。
    Corrupt {
        /// アダプタ私有の原因 (契約は型を約束しない — 診断表示だけを運ぶ)。
        source: Box<dyn Error + Send + Sync>,
    },
}

impl fmt::Display for DefinitionArtifactsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefinitionArtifactsError::Io { kind, path } => {
                write!(f, "io: {kind:?} at {}", path.display())
            }
            DefinitionArtifactsError::Corrupt { .. } => f.write_str("corrupt definition artifacts"),
        }
    }
}

impl Error for DefinitionArtifactsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DefinitionArtifactsError::Corrupt { source } => Some(source.as_ref()),
            DefinitionArtifactsError::Io { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core_command_domain::workflow_definition::{
        ExecutionKind, PhaseId, StageMode, StageNodeBuilder, StageNumber, StageSlug,
    };

    #[test]
    fn the_material_reports_back_every_part_it_was_given() {
        // 取込が読んだ 5 つの材料は、そのまま `define` / `redefine` の引数になる。
        // `into_content` は内容 3 点だけを手放し、系譜 ID と内容版は先に読む形である。
        let graph = StageGraph::new(vec![
            StageNodeBuilder::new(
                StageSlug::parse("state-init").expect("slug"),
                StageNumber::parse("0.1").expect("番号"),
                "State Init".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .scopes(vec!["classic".to_string()])
            .build(),
        ])
        .expect("グラフ");
        let grid = ScopeGrid::from_graph(&graph);
        let scopes: BTreeMap<String, ScopeMetadata> = [(
            "classic".to_string(),
            ScopeMetadata::new("classic").expect("スコープ"),
        )]
        .into_iter()
        .collect();
        let id = WorkflowDefinitionId::parse("claude").expect("定義 id");
        let revision =
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision");

        let artifacts = DefinitionArtifacts::new(
            id.clone(),
            revision.clone(),
            graph.clone(),
            grid.clone(),
            scopes.clone(),
        );
        assert_eq!(artifacts.id(), &id);
        assert_eq!(artifacts.revision(), &revision);
        assert_eq!(artifacts.graph(), &graph);
        assert_eq!(artifacts.grid(), &grid);
        assert_eq!(artifacts.scopes(), &scopes);

        let (out_graph, out_grid, out_scopes) = artifacts.into_content();
        assert_eq!(out_graph, graph);
        assert_eq!(out_grid, grid);
        assert_eq!(out_scopes, scopes);
    }

    #[derive(Debug)]
    struct FakeCause;

    impl fmt::Display for FakeCause {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("stage graph at /d/stage-graph.json is not valid JSON")
        }
    }

    impl Error for FakeCause {}

    #[test]
    fn every_variant_renders_its_material() {
        assert_eq!(
            DefinitionArtifactsError::Io {
                kind: ErrorKind::NotFound,
                path: PathBuf::from("/d/stage-graph.json"),
            }
            .to_string(),
            "io: NotFound at /d/stage-graph.json"
        );
        assert_eq!(
            DefinitionArtifactsError::Corrupt {
                source: Box::new(FakeCause),
            }
            .to_string(),
            "corrupt definition artifacts"
        );
    }

    #[test]
    fn the_corrupt_cause_travels_the_source_chain() {
        // 分類は契約に載らない (裁定 6) — 原因は `Error::source` の連鎖で診断表示だけを運ぶ。
        let error = DefinitionArtifactsError::Corrupt {
            source: Box::new(FakeCause),
        };
        assert_eq!(
            Error::source(&error)
                .expect("Corrupt は原因を連鎖する")
                .to_string(),
            "stage graph at /d/stage-graph.json is not valid JSON"
        );
        assert!(
            Error::source(&DefinitionArtifactsError::Io {
                kind: ErrorKind::NotFound,
                path: PathBuf::from("/d"),
            })
            .is_none()
        );
    }
}
