//! `WorkflowDefinition` — Published Language (`stage-graph.json` / `scope-grid.json` /
//! `scopes/*.md`) のモデルを内包する集約。「何を実行しうるか」の静的定義を
//! 1 つの集約にまとめ、orchestration が依存する 6 述語を純関数として提供する
//! (01 §3.1 / 10 §3)。
//!
//! 識別子 `WorkflowDefinitionId` と内容版 `DefinitionRevision` を持つ (ADR-008)。id は
//! 内容が変わっても不変の系譜 ID、revision は 3 入力の内容ダイジェストであり、どちらも
//! **取込境界**が付与する (ドメインは計算しない — 正準 JSON とダイジェストはアダプタの
//! 責務である)。
//!
//! # 状態の正本はジャーナルである (2026-08-31 オーナー裁定)
//!
//! かつてこの集約は Repository 実装が dist の 3 入力をディスクから読んで組み立てていた。
//! その実装は 2026-08-31 に破棄された (「NG 中の NG」) — 集約の最新状態をファイルから
//! 組み立てるのは `coding-rules/cqrs-boundaries.md` 規則 4 への正面違反だからである。
//!
//! いまの構築経路は 3 本だけである (coding-rules/aggregate-commands.md「再構成の形」):
//! genesis [`WorkflowDefinition::define`]、リプレイ [`WorkflowDefinition::replay`]、
//! および誕生記録の変換 [`From<(Defined, DateTime<Utc>)>`]。**保存値からの検証付き再構成 (旧
//! `from_artifacts`) は撤去した** — 第 3 の構築口であり、genesis と同一引数列の双子
//! でもあった (`Intent::from_material` と同じ誤りの形)。dist の 3 入力を読むのは
//! 書込ユースケースの取込境界であって、この集約の再構成経路ではない。
//!
//! # 観測可能契約 (レポート §6.1 — 逸脱台帳行き)
//!
//! - **未知スコープの非対称**: `subgraph_for_scope` だけが `Err(UnknownScope)`。
//!   `first_in_scope_stage_of_phase` / `stages_in_scope` は同じ未知スコープに対して
//!   `None` / 空を返す。
//! - **`.md` あり × グリッド列なし** = zero-EXECUTE な**正当**スコープ (エラーにしない)。
//! - **グリッド列あり × `.md` なし** = ランタイムから不可視 (有効スコープの権威は `.md`)。
//! - **グリッドに slug が無い** = `None`。`SKIP` に畳まない (3 値契約)。
//! - **文書順の保持**: `stages_in_scope` は文書順で全ステージを返し、`subgraph_for_scope`
//!   だけが数値順にソートする。2 経路の使い分けを潰さない。
//!
//! `enabled: false` のノードは**除外しない**。意味論が未確定 (レポート §7) のため、
//! モデルは `StageNode::is_enabled()` を露出するだけで判断は呼出側に委ねる。
//!
//! # 集約への畳み込み (FR8.4)
//!
//! かつてここにあった `effective_plan_action` / `next_in_scope_stage` は
//! **`IntentExecution` 側へ移設**した。recompose オーバレイと checkbox は実行の状態で
//! あって定義の状態ではなく、定義側に置くと「呼出側が状態を持ち回って定義に問い直す」
//! Ask 形になるためである (tell-dont-ask.md)。定義側に残るのは静的グリッドの照会
//! (`grid().action(scope, slug)`) と文書順の全ステージ列 (`stages_in_scope`) だけで、
//! 実効プランの合成は集約が行う。

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};

use super::definition_revision::DefinitionRevision;
use super::phase::PhaseId;
use super::plan_action::PlanAction;
use super::scope_grid::ScopeGrid;
use super::scope_metadata::ScopeMetadata;
use super::stage_graph::StageGraph;
use super::stage_node::StageNode;
use super::stage_route::StageRoute;
use super::stage_slug::StageSlug;
use super::workflow_definition_event::{Defined, Redefined, WorkflowDefinitionEvent};
use super::workflow_definition_id::WorkflowDefinitionId;

/// `validScopes()` に無いスコープ名。
///
/// upstream の逐語文言 `Unknown scope: "<scope>". Valid scopes: <csv>` を組み立てるのに
/// 必要な材料をそのまま保持する (文言化は文言カタログ側の責務)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownScope {
    scope: String,
    /// 有効スコープ名 (辞書順)。
    valid_scopes: Vec<String>,
}

impl UnknownScope {
    /// 拒否されたスコープ名と、拒否時点の有効スコープ一覧 (辞書順) を束ねる。
    /// どちらも生値のまま保持する。
    #[must_use]
    pub fn new(scope: impl Into<String>, valid_scopes: Vec<String>) -> UnknownScope {
        UnknownScope {
            scope: scope.into(),
            valid_scopes,
        }
    }

    /// 拒否されたスコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 有効スコープ名 (辞書順)。
    #[must_use]
    pub fn valid_scopes(&self) -> &[String] {
        &self.valid_scopes
    }
}

/// ワークフロー定義の集約。
///
/// 等価は**内容と識別子の両方**で決まる (derive)。「同じ系譜の同じ内容」を 1 つの等価関係で
/// 表すのが自然だからである。id だけの同一性比較が要るのは `IntentExecution` 側の定義照合で、
/// そちらは `id()` 同士を突き合わせる
/// (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-equality.md)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDefinition {
    id: WorkflowDefinitionId,
    revision: DefinitionRevision,
    graph: StageGraph,
    grid: ScopeGrid,
    scopes: BTreeMap<String, ScopeMetadata>,
    /// 適用済みイベント数と一致する順序番号 (`Defined` = 1)。次のイベントの通番は
    /// `seq_nr + 1` であり、封筒を組む Repository はこの値を使う。
    seq_nr: usize,
    /// ストアが採番した楽観 version — **次の書込に提示する不透明トークン**である。
    ///
    /// 解釈も比較も算術もしない (BR5.3)。再構成した Repository が
    /// [`WorkflowDefinition::with_version`] で刻み、書込む Repository が
    /// [`WorkflowDefinition::version`] で読む。
    version: usize,
    /// 最後に適用したイベントの発生時刻 (集約は時計を持たない — NFR3.1)。
    ///
    /// 封筒の `occurred_at` はここから来る。`IntentExecution` と同型であり、`store` の
    /// 引数で時刻を運ばない (オーナー裁定 2026-08-31 — 手本と対にする)。
    last_updated_at: DateTime<Utc>,
}

/// 改訂を受け付けられない形 (材料のみ — 利用者向け文言はアダプタ層)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedefineError {
    /// 提示された内容版が現在と同じ — 書くべき事実が無い。
    ///
    /// 無言の no-op にはしない (coding-rules/aggregate-commands.md「拒否はガード付き Err」)。
    /// 取込を冪等に見せるかどうかは呼出側 (ユースケース) の判断であり、集約は「変化が無い」
    /// という事実を返すだけである。
    Unchanged {
        /// 現在と一致した内容版。
        revision: DefinitionRevision,
    },
    /// 通番が上限に達した (飽和加算で成功を装わない)。
    SequenceExhausted,
}

impl WorkflowDefinition {
    /// まだ 1 度も永続化していない集約が提示する版 (新規作成の楽観 version)。
    ///
    /// 本家 v3 の規約「新規作成は `seq_nr == 1` かつ `version == 0`」の 0 に名前を与えた
    /// ものである — 呼出側にも Repository 実装にも裸の `0` を書かせない。
    pub const UNPERSISTED_VERSION: usize = 0;

    /// 定義を**確立する** — 集約と誕生イベントの対を返す (genesis ファクトリ)。
    ///
    /// 集約のファクトリは (集約インスタンス, 誕生イベント) の**両方**を返すことが必須である
    /// (coding-rules/aggregate-commands.md)。Repository の永続化は
    /// `store(&event, &aggregate)` の形でジャーナル追記分とスナップショット分を同一
    /// トランザクションで受け取るので、どちらが欠けても永続化が組めない。
    ///
    /// `id` / `revision` は**取込境界が付与する** (ADR-008)。ドメインは revision を計算しない。
    /// `occurred_at` も同様に外から来る — 集約は時計を持たない (NFR3.1)。
    ///
    /// グリッド列と `.md` の**不一致は検証しない** — 双方向の不一致がどちらも正当な
    /// 観測可能契約だからである (zero-EXECUTE スコープ / ランタイム不可視スコープ)。
    #[must_use]
    pub fn define(
        id: WorkflowDefinitionId,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
        occurred_at: DateTime<Utc>,
    ) -> (WorkflowDefinition, WorkflowDefinitionEvent) {
        let defined = Defined::new(id, revision, graph, grid, scopes);
        (
            WorkflowDefinition::from((defined.clone(), occurred_at)),
            WorkflowDefinitionEvent::Defined(defined),
        )
    }

    /// 定義を別の内容版へ**改訂する** (1 コマンド 1 イベント)。
    ///
    /// 取込が読んだ 3 入力が現在の内容版と違うときに呼ぶ。同じ内容版なら書くべき事実が
    /// 無いので `Unchanged` で拒否する — 判断は集約が持ち、呼出側に内容版の比較を
    /// 再実装させない (tell-dont-ask.md)。取込を冪等に見せたい呼出側はこの拒否を
    /// 「変化なし」として畳めばよい。
    ///
    /// # Errors
    ///
    /// 内容版が現在と同じ (`Unchanged`)、通番の枯渇 (`SequenceExhausted`)。
    pub fn redefine(
        &mut self,
        revision: DefinitionRevision,
        graph: StageGraph,
        grid: ScopeGrid,
        scopes: BTreeMap<String, ScopeMetadata>,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowDefinitionEvent, RedefineError> {
        if self.revision == revision {
            return Err(RedefineError::Unchanged { revision });
        }
        let Some(seq_nr) = self.seq_nr.checked_add(1) else {
            return Err(RedefineError::SequenceExhausted);
        };
        let event =
            WorkflowDefinitionEvent::Redefined(Redefined::new(revision, graph, grid, scopes));
        self.apply_event(seq_nr, occurred_at, &event);
        Ok(event)
    }

    /// スナップショット種に差分イベントを畳み込んで復元する (Event Sourcing の再生経路)。
    ///
    /// 本家 v3 の `UserAccount::replay(events, snapshot)` と同型。スナップショット種は
    /// 誕生記録の変換 (`From<(Defined, DateTime<Utc>)>`) で得る。**再構成は失敗を返さない** — 歴史を読む
    /// だけであり、壊れた歴史は回復せずクラッシュする (オーナー裁定 2026-08-30)。
    ///
    /// # Panics
    ///
    /// 通番の飛び (行の欠け・重複)。
    #[must_use]
    pub fn replay(
        snapshot: WorkflowDefinition,
        events: impl IntoIterator<Item = (usize, DateTime<Utc>, WorkflowDefinitionEvent)>,
    ) -> WorkflowDefinition {
        let mut definition = snapshot;
        for (seq_nr, occurred_at, event) in events {
            definition.apply_event(seq_nr, occurred_at, &event);
        }
        definition
    }

    /// イベントを 1 つ適用する (通常実行とリプレイの唯一の状態遷移経路 — BR1.1)。
    ///
    /// 通番も発生時刻も**適用の引数**であって、イベントに載る材料ではない (輸送のメタデータは
    /// 封筒が運ぶ — ADR-010 / B7)。適用が通れば `self.seq_nr` / `self.last_updated_at` が
    /// そのイベントの通番と発生時刻になる。
    #[allow(
        clippy::expect_used,
        reason = "壊れた歴史は回復不能 — 再構成は失敗を返さずクラッシュする (オーナー裁定 2026-08-30)"
    )]
    fn apply_event(
        &mut self,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        event: &WorkflowDefinitionEvent,
    ) {
        let expected = self
            .seq_nr
            .checked_add(1)
            .expect("apply_event: sequence exhausted");
        assert_eq!(
            seq_nr, expected,
            "apply_event: sequence gap (expected {expected}, actual {seq_nr})"
        );
        // 変種の網羅 match — 腕の欠落はビルドで落ちる。
        match event {
            // genesis イベントは差分適用では何も変えない — スナップショット種が誕生を
            // 含む (本家サンプル同型: apply は変異イベントだけを見る)。
            WorkflowDefinitionEvent::Defined(_) => {}
            WorkflowDefinitionEvent::Redefined(redefined) => {
                self.revision = redefined.revision().clone();
                self.graph = redefined.graph().clone();
                self.grid = redefined.grid().clone();
                self.scopes = redefined.scopes().clone();
            }
        }
        self.seq_nr = seq_nr;
        self.last_updated_at = occurred_at;
    }

    /// ストアが採番した版を刻む (**Repository 実装専用**)。
    ///
    /// 再構成の最後にストアが読んだ版を載せるための一手である — ユースケースは呼ばない。
    /// 版はドメインが導出できない値 (正本はスナップショット行の列) なので、外から刻む口が
    /// 要る。
    #[must_use]
    pub const fn with_version(mut self, version: usize) -> WorkflowDefinition {
        self.version = version;
        self
    }

    /// 再構成した通番を刻む (**Repository 実装専用**)。
    ///
    /// 基底の通番はスナップショット**封筒の列**から来る — 定義のスナップショット行は
    /// 通番を内容として持たない (`Intent` と同じ形)。差分再生はここで刻んだ値の次から
    /// 始まる。
    #[must_use]
    pub const fn with_seq_nr(mut self, seq_nr: usize) -> WorkflowDefinition {
        self.seq_nr = seq_nr;
        self
    }

    /// 適用済みイベント数と一致する順序番号 (`Defined` = 1)。
    ///
    /// 次のイベントの通番は `seq_nr + 1` であり、改訂を通ったあとの値は**そのイベントの
    /// 通番そのもの**である。封筒を組む Repository はこの値を使う。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// 次の書込に提示する楽観 version (ストアが採番した不透明トークン)。
    ///
    /// 解釈も比較も算術もしない — 読んだ値をそのままストアへ返すだけである (BR5.3)。
    /// `seq_nr` から導いてはならない。
    #[must_use]
    pub const fn version(&self) -> usize {
        self.version
    }

    /// 最後に適用したイベントの発生時刻 (集約は時計を持たない — NFR3.1)。
    ///
    /// `define` / `redefine` を通ったあとの値は**そのイベントの発生時刻**であり、封筒の
    /// `occurred_at` になる。封筒を組む Repository はここから読む
    /// (`IntentExecution::last_updated_at` と同型 — オーナー裁定 2026-08-31)。
    #[must_use]
    pub const fn last_updated_at(&self) -> &DateTime<Utc> {
        &self.last_updated_at
    }

    /// この定義の系譜 ID。内容が変わっても不変 (ADR-008)。
    #[must_use]
    pub const fn id(&self) -> &WorkflowDefinitionId {
        &self.id
    }

    /// この定義の内容版。3 入力が 1 バイトでも変われば変わる (ADR-008)。
    #[must_use]
    pub const fn revision(&self) -> &DefinitionRevision {
        &self.revision
    }

    /// `stage-graph.json` 由来のステージグラフ (文書順を保持したまま)。
    #[must_use]
    pub const fn graph(&self) -> &StageGraph {
        &self.graph
    }

    /// `scope-grid.json` 由来の静的 EXECUTE / SKIP グリッド。recompose サフィックスは含まない。
    #[must_use]
    pub const fn grid(&self) -> &ScopeGrid {
        &self.grid
    }

    /// スコープ `.md` 由来のメタデータ (スコープ名の辞書順)。有効スコープの権威。
    #[must_use]
    pub const fn scopes(&self) -> &BTreeMap<String, ScopeMetadata> {
        &self.scopes
    }

    /// スコープ `.md` のメタデータ。`.md` が無ければ `None` (= 無効スコープ)。
    #[must_use]
    pub fn scope_metadata(&self, scope: &str) -> Option<&ScopeMetadata> {
        self.scopes.get(scope)
    }

    /// `validScopes()` — 権威はスコープ `.md` の存在 (グリッドではない)。辞書順。
    #[must_use]
    pub fn valid_scopes(&self) -> Vec<&str> {
        self.scopes.keys().map(String::as_str).collect()
    }

    /// `valid_scopes()` に含まれるか。権威は `.md` の存在であってグリッド列の有無ではない。
    #[must_use]
    pub fn is_valid_scope(&self, scope: &str) -> bool {
        self.scopes.contains_key(scope)
    }

    /// `subgraphForScope` — 静的グリッドの EXECUTE セルを抽出し、**数値順**で返す。
    ///
    /// ランタイムでは topo ソートしない (compile のエッジ局所不変条件により数値順が
    /// 有効な topo 順であることが保証されている — レポート §4.6)。
    ///
    /// # Errors
    ///
    /// スコープ `.md` が存在しなければ `UnknownScope` (有効スコープ一覧を添える)。
    /// **未知スコープで `Err` を返すのはこの述語だけ**である (非対称契約)。
    pub fn subgraph_for_scope(&self, scope: &str) -> Result<Vec<&StageNode>, UnknownScope> {
        if !self.is_valid_scope(scope) {
            return Err(UnknownScope::new(
                scope,
                self.valid_scopes()
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            ));
        }
        // 列が無い有効スコープは zero-EXECUTE (エラーではない)。
        Ok(self
            .graph
            .numeric_order()
            .into_iter()
            .filter(|node| self.grid.action(scope, node.slug()) == Some(PlanAction::Execute))
            .collect())
    }

    /// `firstInScopeStageOfPhase` — `subgraph_for_scope` の**数値順**の並びで最初に
    /// 当該フェーズに属するノード。walking skeleton のアンカーの導出元 (ハードコードではない)。
    ///
    /// 未知スコープは `None`。
    #[must_use]
    pub fn first_in_scope_stage_of_phase(&self, phase: PhaseId, scope: &str) -> Option<&StageNode> {
        self.subgraph_for_scope(scope)
            .ok()?
            .into_iter()
            .find(|node| node.phase() == phase)
    }

    /// `stagesInScope` — **全ステージ**について `(slug, phase, action)` を**文書順**で返す。
    ///
    /// `action` は静的グリッドの 3 値 (recompose サフィックスは合成しない)。
    /// 未知スコープは空 (`subgraph_for_scope` との非対称)。
    #[must_use]
    pub fn stages_in_scope(&self, scope: &str) -> Vec<(&StageSlug, PhaseId, Option<PlanAction>)> {
        if !self.is_valid_scope(scope) {
            return Vec::new();
        }
        self.graph
            .nodes()
            .iter()
            .map(|node| {
                (
                    node.slug(),
                    node.phase(),
                    self.grid.action(scope, node.slug()),
                )
            })
            .collect()
    }

    /// ステージの route 同一性 — 対象ステージと、その scope の in-scope ステージ列。
    /// steering 連鎖の route 束縛が指す対象を VO として返す (素材文字列は組まない)。
    #[must_use]
    pub fn stage_route(&self, scope: &str, node: &StageNode) -> StageRoute {
        let stages = self
            .stages_in_scope(scope)
            .iter()
            .map(|(slug, _, _)| (*slug).clone())
            .collect();
        StageRoute::new(node.slug().clone(), stages)
    }
}

impl From<(Defined, DateTime<Utc>)> for WorkflowDefinition {
    /// 誕生記録とその発生時刻から集約を導出する (リプレイのスナップショット種)。
    ///
    /// **構造体リテラルはここだけ** — genesis ([`WorkflowDefinition::define`]) もこの変換を
    /// 通る (coding-rules/factory-naming.md「すべての構築経路が基本コンストラクタを通る」)。
    /// 誕生時点の通番は 1、版は未永続 (ストアが採番する) である。
    ///
    /// 時刻を対で受けるのは、**発生時刻がイベントの材料ではなく封筒のメタデータ**だから
    /// である (`Defined` は時刻を持たない — ADR-010 / B7)。誕生記録だけでは集約の
    /// `last_updated_at` を埋められないので、変換の入力は対になる。
    fn from((defined, occurred_at): (Defined, DateTime<Utc>)) -> WorkflowDefinition {
        WorkflowDefinition {
            id: defined.id().clone(),
            revision: defined.revision().clone(),
            graph: defined.graph().clone(),
            grid: defined.grid().clone(),
            scopes: defined.scopes().clone(),
            seq_nr: 1,
            version: WorkflowDefinition::UNPERSISTED_VERSION,
            last_updated_at: occurred_at,
        }
    }
}

impl fmt::Display for RedefineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RedefineError::Unchanged { revision } => {
                write!(f, "definition unchanged at revision {}", revision.as_str())
            }
            RedefineError::SequenceExhausted => f.write_str("sequence exhausted"),
        }
    }
}

impl std::error::Error for RedefineError {}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。panic! は想定外バリアントの即時失敗という検証用途で使う。
    #![allow(clippy::indexing_slicing, clippy::panic)]

    use super::*;
    use crate::workflow_definition::{ExecutionKind, StageMode, StageNodeBuilder, StageNumber};
    use proptest::prelude::*;

    /// grid 列がある 2 スコープ + `.md` だけがある 1 スコープ + `.md` が無い 1 スコープ。
    const REGISTERED: [&str; 3] = ["alpha", "beta", "delta"];
    const POOL: [&str; 3] = ["alpha", "beta", "gamma"];

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    fn node(name: &str, number: &str, phase: PhaseId, scopes: &[&str]) -> StageNode {
        StageNodeBuilder::new(
            slug(name),
            StageNumber::parse(number).unwrap(),
            name.to_string(),
            phase,
            ExecutionKind::Always,
            StageMode::Inline,
        )
        .scopes(scopes.iter().map(|s| (*s).to_string()).collect())
        .build()
    }

    fn registry(names: &[&str]) -> BTreeMap<String, ScopeMetadata> {
        names
            .iter()
            .map(|n| ((*n).to_string(), ScopeMetadata::new(n).unwrap()))
            .collect()
    }

    fn id(value: &str) -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse(value).unwrap()
    }

    fn revision(fill: char) -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64))).unwrap()
    }

    /// イベントの発生時刻 (集約は時計を持たないので固定値を渡す)。
    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-31T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    /// 文書順 = 数値順の小さな出荷グラフ相当の 3 入力。
    fn artifacts() -> (StageGraph, ScopeGrid) {
        let graph = StageGraph::new(vec![
            node("bootstrap", "0.1", PhaseId::Initialization, &[]),
            node(
                "intent-capture",
                "1.1",
                PhaseId::Ideation,
                &["alpha", "beta"],
            ),
            node("requirements", "1.2", PhaseId::Ideation, &["alpha"]),
            node("threat-model", "2.1", PhaseId::Inception, &["alpha"]),
            node(
                "code-generation",
                "3.1",
                PhaseId::Construction,
                &["alpha", "beta"],
            ),
            node("ops-runbook", "4.1", PhaseId::Operation, &["gamma"]),
        ])
        .unwrap();
        let grid = ScopeGrid::from_graph(&graph);
        (graph, grid)
    }

    /// 文書順 = 数値順の小さな出荷グラフ相当。
    fn sample() -> WorkflowDefinition {
        let (graph, grid) = artifacts();
        WorkflowDefinition::define(
            id("claude"),
            revision('0'),
            graph,
            grid,
            registry(&REGISTERED),
            at(),
        )
        .0
    }

    // ---- ファクトリ (coding-rules/aggregate-commands.md) ----

    #[test]
    fn the_genesis_factory_returns_the_definition_and_its_birth_event() {
        // 集約のファクトリは (集約, 誕生イベント) の対を返すことが必須である —
        // Repository の永続化が `store(&event, &aggregate, ..)` の形でその両方を要求する。
        let (graph, grid) = artifacts();
        let (definition, event) = WorkflowDefinition::define(
            id("claude"),
            revision('0'),
            graph.clone(),
            grid.clone(),
            registry(&REGISTERED),
            at(),
        );
        let WorkflowDefinitionEvent::Defined(defined) = &event else {
            panic!("genesis は Defined を返す: {event:?}");
        };
        assert_eq!(defined.id(), definition.id());
        assert_eq!(defined.revision(), definition.revision());
        // 誕生イベントは**内容そのもの**を運ぶ — これがリプレイのスナップショット種になる。
        assert_eq!(defined.graph(), &graph);
        assert_eq!(defined.grid(), &grid);
        assert_eq!(defined.scopes(), &registry(&REGISTERED));
        assert_eq!(definition.seq_nr(), 1, "誕生の通番は 1");
        assert_eq!(
            definition.version(),
            WorkflowDefinition::UNPERSISTED_VERSION
        );
    }

    #[test]
    fn the_birth_record_converts_back_into_the_same_definition() {
        // 再構成はファクトリではない — 歴史を読み戻す経路なので**イベントを作らない**。
        // 型がそれを保証する: 変換の戻り値は集約だけである。
        let (graph, grid) = artifacts();
        let (born, event) = WorkflowDefinition::define(
            id("claude"),
            revision('0'),
            graph,
            grid,
            registry(&REGISTERED),
            at(),
        );
        let WorkflowDefinitionEvent::Defined(defined) = event else {
            panic!("genesis は Defined を返す");
        };
        assert_eq!(WorkflowDefinition::from((defined, at())), born);
    }

    #[test]
    fn the_aggregate_carries_the_time_of_the_event_it_last_applied() {
        // 封筒の `occurred_at` はここから来る (`IntentExecution` と同型) — `store` の引数で
        // 時刻を運ばないので、集約が運ばなければ封筒が組めない。
        let definition = sample();
        assert_eq!(definition.last_updated_at(), &at(), "誕生の時刻が刻まれる");

        let later = at() + chrono::TimeDelta::try_seconds(90).expect("固定のオフセット");
        let mut redefined = definition;
        let (graph, grid) = artifacts();
        redefined
            .redefine(revision('1'), graph, grid, registry(&["alpha"]), later)
            .expect("内容版が違えば改訂できる");
        assert_eq!(
            redefined.last_updated_at(),
            &later,
            "改訂はその事実の発生時刻へ進める"
        );
    }

    #[test]
    fn replaying_takes_the_time_from_each_envelope() {
        // リプレイと通常実行は同一経路 (BR1.1) — 時刻も差分イベントごとに封筒から来る。
        let later = at() + chrono::TimeDelta::try_seconds(90).expect("固定のオフセット");
        let (graph, grid) = artifacts();
        let event = WorkflowDefinitionEvent::Redefined(Redefined::new(
            revision('1'),
            graph,
            grid,
            registry(&["alpha"]),
        ));

        let replayed = WorkflowDefinition::replay(sample(), vec![(2, later, event)]);

        assert_eq!(replayed.last_updated_at(), &later);
        assert_eq!(replayed.seq_nr(), 2);
    }

    // ---- 改訂 (1 コマンド 1 イベント) ----

    #[test]
    fn redefining_with_a_new_revision_replaces_the_content_and_advances_the_sequence() {
        let mut definition = sample();
        let (graph, grid) = artifacts();
        let event = definition
            .redefine(revision('1'), graph, grid, registry(&["alpha"]), at())
            .expect("内容版が違えば改訂できる");

        let WorkflowDefinitionEvent::Redefined(redefined) = &event else {
            panic!("改訂は Redefined を返す: {event:?}");
        };
        assert_eq!(redefined.revision(), &revision('1'));
        assert_eq!(definition.revision(), &revision('1'));
        assert_eq!(definition.valid_scopes(), ["alpha"], "内容が入れ替わる");
        assert_eq!(definition.id(), &id("claude"), "系譜 ID は不変");
        assert_eq!(definition.seq_nr(), 2, "改訂は次の通番になる");
    }

    #[test]
    fn redefining_with_the_same_revision_is_refused_instead_of_silently_doing_nothing() {
        // 拒否はガード付き Err (coding-rules/aggregate-commands.md)。冪等に見せるかどうかは
        // 呼出側の判断であり、集約は「変化が無い」という事実を返すだけである。
        let mut definition = sample();
        let before = definition.clone();
        let (graph, grid) = artifacts();
        let error = definition
            .redefine(revision('0'), graph, grid, registry(&REGISTERED), at())
            .expect_err("同じ内容版は拒否される");

        assert_eq!(
            error,
            RedefineError::Unchanged {
                revision: revision('0')
            }
        );
        assert_eq!(definition, before, "拒否された改訂は何も動かさない");
    }

    #[test]
    fn the_refusal_carries_material_not_wording() {
        assert_eq!(
            RedefineError::Unchanged {
                revision: revision('0')
            }
            .to_string(),
            format!("definition unchanged at revision sha256:{}", "0".repeat(64))
        );
        assert_eq!(
            RedefineError::SequenceExhausted.to_string(),
            "sequence exhausted"
        );
        let error: Box<dyn std::error::Error> = Box::new(RedefineError::SequenceExhausted);
        assert_eq!(error.to_string(), "sequence exhausted");
    }

    #[test]
    fn an_exhausted_sequence_is_refused_rather_than_saturating() {
        let mut definition = sample().with_seq_nr(usize::MAX);
        let (graph, grid) = artifacts();
        assert_eq!(
            definition.redefine(revision('1'), graph, grid, registry(&["alpha"]), at()),
            Err(RedefineError::SequenceExhausted)
        );
    }

    // ---- リプレイ (通常実行と同一経路 — BR1.1) ----

    #[test]
    fn replaying_no_events_returns_the_snapshot_state() {
        let snapshot = sample();
        assert_eq!(
            WorkflowDefinition::replay(snapshot.clone(), Vec::new()),
            snapshot
        );
    }

    #[test]
    fn replaying_the_genesis_event_is_a_no_op() {
        // スナップショット種が誕生を含むので、genesis イベントの差分適用は何も変えない
        // (本家サンプル同型)。通番だけが基底の次へ進む。
        let (graph, grid) = artifacts();
        let (definition, event) = WorkflowDefinition::define(
            id("claude"),
            revision('0'),
            graph,
            grid,
            registry(&REGISTERED),
            at(),
        );
        let replayed = WorkflowDefinition::replay(definition.clone(), vec![(2, at(), event)]);
        assert_eq!(replayed.revision(), definition.revision());
        assert_eq!(replayed.valid_scopes(), definition.valid_scopes());
    }

    #[test]
    fn replaying_a_redefinition_reaches_the_same_state_as_the_command_path() {
        // 通常実行とリプレイは同じ `apply_event` を通る (BR1.1) — 片方だけ直す余地が無い。
        let (graph, grid) = artifacts();
        let mut commanded = sample();
        let event = commanded
            .redefine(revision('1'), graph, grid, registry(&["alpha"]), at())
            .expect("改訂できる");

        let replayed = WorkflowDefinition::replay(sample(), vec![(2, at(), event)]);
        assert_eq!(replayed, commanded);
    }

    #[test]
    #[should_panic(expected = "sequence gap")]
    fn a_sequence_gap_crashes_the_replay() {
        // 壊れた歴史は回復せずクラッシュが正 (オーナー裁定 2026-08-30)。
        let (graph, grid) = artifacts();
        let event = WorkflowDefinitionEvent::Redefined(crate::workflow_definition::Redefined::new(
            revision('1'),
            graph,
            grid,
            registry(&["alpha"]),
        ));
        let _ = WorkflowDefinition::replay(sample(), vec![(5, at(), event)]);
    }

    #[test]
    fn the_store_stamps_the_version_it_read() {
        // 版はドメインが導出できない値 (正本はスナップショット行の列) なので外から刻む。
        let definition = sample().with_version(7);
        assert_eq!(definition.version(), 7);
        assert_eq!(definition.seq_nr(), 1, "版と通番は別物である");
    }

    // ---- エンティティの識別子と内容版 (ADR-008) ----

    #[test]
    fn the_definition_carries_the_identity_and_the_revision_the_repository_assigned() {
        let wd = sample();
        assert_eq!(wd.id(), &id("claude"));
        assert_eq!(wd.revision(), &revision('0'));
        assert_eq!(wd.id().as_str(), "claude");
        assert!(wd.revision().as_str().starts_with("sha256:"));
    }

    #[test]
    fn two_definitions_with_the_same_content_but_different_lineage_are_not_equal() {
        let one = sample();
        let graph = one.graph().clone();
        let grid = one.grid().clone();
        let other = WorkflowDefinition::define(
            id("kiro"),
            revision('0'),
            graph,
            grid,
            registry(&REGISTERED),
            at(),
        )
        .0;
        assert_ne!(one, other);
        assert_ne!(one.id(), other.id());
        // 内容版は同じ — 系譜だけが違う。
        assert_eq!(one.revision(), other.revision());
    }

    #[test]
    fn the_revision_changes_without_the_identity_changing() {
        let one = sample();
        let other = WorkflowDefinition::define(
            one.id().clone(),
            revision('1'),
            one.graph().clone(),
            one.grid().clone(),
            registry(&REGISTERED),
            at(),
        )
        .0;
        // ピン更新 = 内容版だけが進む。系譜 ID は不変 (ADR-008)。
        assert_eq!(one.id(), other.id());
        assert_ne!(one.revision(), other.revision());
        assert_ne!(one, other);
    }

    #[test]
    fn the_six_predicates_survive_the_entity_change() {
        // FR8.4 で 2 述語を集約へ移したあとに定義側へ残る照会一式。
        let wd = sample();
        assert!(wd.is_valid_scope("alpha"));
        assert_eq!(wd.valid_scopes(), ["alpha", "beta", "delta"]);
        assert!(wd.scope_metadata("alpha").is_some());
        assert!(wd.subgraph_for_scope("alpha").is_ok());
        assert_eq!(wd.stages_in_scope("alpha").len(), 6);
        assert!(
            wd.first_in_scope_stage_of_phase(PhaseId::Ideation, "alpha")
                .is_some()
        );
    }

    #[test]
    fn stages_in_scope_reports_the_phase_of_every_stage_alongside_the_action() {
        // Started の StageEntry 列 (集約側) はこの 3 つ組から作られるため、PhaseId が
        // 文書順で正しく載っていることが FR8.4 移設後の前提になる。
        let wd = sample();
        let rows = wd.stages_in_scope("alpha");
        let phases: Vec<PhaseId> = rows.iter().map(|(_, phase, _)| *phase).collect();
        assert_eq!(
            phases,
            [
                PhaseId::Initialization,
                PhaseId::Ideation,
                PhaseId::Ideation,
                PhaseId::Inception,
                PhaseId::Construction,
                PhaseId::Operation,
            ]
        );
        // 索引 0 だけが initialization — 集約の gated(s) 判定の材料。
        assert_eq!(rows[0].1, PhaseId::Initialization);
        assert!(
            rows[1..]
                .iter()
                .all(|(_, p, _)| *p != PhaseId::Initialization)
        );
    }

    // ---- ユビキタス言語の例示 ----

    #[test]
    fn valid_scopes_are_authored_by_the_md_files_not_by_the_grid() {
        let wd = sample();
        // gamma はグリッド列を持つが `.md` が無い → ランタイムから不可視
        assert!(wd.grid().contains_scope("gamma"));
        assert_eq!(wd.valid_scopes(), vec!["alpha", "beta", "delta"]);
        assert!(!wd.is_valid_scope("gamma"));
    }

    #[test]
    fn a_scope_with_no_grid_column_is_a_legitimate_zero_execute_scope() {
        let wd = sample();
        assert!(!wd.grid().contains_scope("delta"));
        assert_eq!(wd.subgraph_for_scope("delta"), Ok(Vec::new()));
        assert_eq!(
            wd.first_in_scope_stage_of_phase(PhaseId::Ideation, "delta"),
            None
        );
        // stages_in_scope は全ステージを返すが action は 3 値の None
        let listed = wd.stages_in_scope("delta");
        assert_eq!(listed.len(), 6);
        assert!(listed.iter().all(|(_, _, action)| action.is_none()));
    }

    #[test]
    fn unknown_scopes_are_asymmetric_error_here_none_everywhere_else() {
        let wd = sample();
        let err = wd.subgraph_for_scope("gamma").unwrap_err();
        assert_eq!(err.scope(), "gamma");
        assert_eq!(err.valid_scopes(), ["alpha", "beta", "delta"]);
        assert_eq!(
            wd.first_in_scope_stage_of_phase(PhaseId::Operation, "gamma"),
            None
        );
        assert!(wd.stages_in_scope("gamma").is_empty());
    }

    #[test]
    fn subgraph_extracts_execute_cells_in_numeric_order_including_initialization() {
        let wd = sample();
        let beta: Vec<&str> = wd
            .subgraph_for_scope("beta")
            .unwrap()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        // initialization は宣言せずとも全列 EXECUTE (転置の特例)
        assert_eq!(beta, vec!["bootstrap", "intent-capture", "code-generation"]);
    }

    #[test]
    fn the_static_grid_query_is_three_valued() {
        // FR8.4 で `effective_plan_action` を集約へ移設したあと、定義側に残るのは
        // 静的グリッドの照会だけ。3 値契約 (EXECUTE / SKIP / 未コンパイル) はここが持つ。
        let wd = sample();
        assert_eq!(
            wd.grid().action("alpha", &slug("threat-model")),
            Some(PlanAction::Execute)
        );
        assert_eq!(
            wd.grid().action("beta", &slug("threat-model")),
            Some(PlanAction::Skip)
        );
        // グリッド列にない slug は None (SKIP に畳まない)
        assert_eq!(wd.grid().action("alpha", &slug("no-such-stage")), None);
        // 列そのものが無い有効スコープも None
        assert_eq!(wd.grid().action("delta", &slug("bootstrap")), None);
    }

    #[test]
    fn skeleton_anchor_is_derived_from_the_scope_subgraph() {
        let wd = sample();
        let anchor = wd
            .first_in_scope_stage_of_phase(PhaseId::Construction, "beta")
            .unwrap();
        assert_eq!(anchor.slug().as_str(), "code-generation");
        // beta には Inception の EXECUTE が無いのでアンカーも無い
        assert_eq!(
            wd.first_in_scope_stage_of_phase(PhaseId::Inception, "beta"),
            None
        );
    }

    #[test]
    fn document_order_and_numeric_order_are_two_distinct_paths() {
        // 文書順が数値順と一致しない手編集グラフでも両経路の使い分けが残る
        let graph = StageGraph::new(vec![
            node("late", "1.10", PhaseId::Ideation, &["alpha"]),
            node("boot", "0.1", PhaseId::Initialization, &[]),
            node("early", "1.9", PhaseId::Ideation, &["alpha"]),
        ])
        .unwrap();
        let grid = ScopeGrid::from_graph(&graph);
        let wd = WorkflowDefinition::define(
            id("claude"),
            revision('0'),
            graph,
            grid,
            registry(&["alpha"]),
            at(),
        )
        .0;

        let numeric: Vec<&str> = wd
            .subgraph_for_scope("alpha")
            .unwrap()
            .iter()
            .map(|n| n.slug().as_str())
            .collect();
        assert_eq!(numeric, vec!["boot", "early", "late"]);

        // stages_in_scope は文書順
        let listed: Vec<&str> = wd
            .stages_in_scope("alpha")
            .iter()
            .map(|(s, _, _)| s.as_str())
            .collect();
        assert_eq!(listed, vec!["late", "boot", "early"]);
    }

    // ---- PBT: ランダム合成グラフ + グリッド ----

    type NodeSpec = (u32, u32, Vec<usize>);

    fn arb_specs() -> impl Strategy<Value = Vec<NodeSpec>> {
        proptest::collection::vec(
            (
                0u32..5,
                0u32..40,
                proptest::collection::vec(0usize..POOL.len(), 0..3),
            ),
            1..10,
        )
    }

    fn build(specs: &[NodeSpec]) -> WorkflowDefinition {
        let nodes: Vec<StageNode> = specs
            .iter()
            .enumerate()
            .map(|(i, (phase_index, seq, scope_indices))| {
                let phase = PhaseId::from_index(*phase_index).unwrap_or(PhaseId::Ideation);
                let scopes: Vec<&str> = scope_indices.iter().map(|&j| POOL[j]).collect();
                node(
                    &format!("s{i}"),
                    &format!("{phase_index}.{seq}"),
                    phase,
                    &scopes,
                )
            })
            .collect();
        let graph = StageGraph::new(nodes).unwrap();
        let grid = ScopeGrid::from_graph(&graph);
        WorkflowDefinition::define(
            id("claude"),
            revision('0'),
            graph,
            grid,
            registry(&REGISTERED),
            at(),
        )
        .0
    }

    proptest! {
        /// `subgraph_for_scope` の結果はすべてグリッド EXECUTE かつ数値順、
        /// かつ EXECUTE セルを 1 つも取りこぼさない。
        #[test]
        fn subgraph_is_exactly_the_execute_cells_in_numeric_order(specs in arb_specs()) {
            let wd = build(&specs);
            for scope in wd.valid_scopes() {
                let sub = wd.subgraph_for_scope(scope).unwrap();
                for n in &sub {
                    prop_assert_eq!(
                        wd.grid().action(scope, n.slug()),
                        Some(PlanAction::Execute)
                    );
                }
                for w in sub.windows(2) {
                    prop_assert!(
                        w[0].number().numeric_cmp(w[1].number()) != std::cmp::Ordering::Greater
                    );
                }
                let expected = wd
                    .graph()
                    .nodes()
                    .iter()
                    .filter(|n| wd.grid().action(scope, n.slug()) == Some(PlanAction::Execute))
                    .count();
                prop_assert_eq!(sub.len(), expected);
            }
        }

        /// 未知スコープの非対称契約: `subgraph_for_scope` だけが `Err`。
        #[test]
        fn unknown_scope_is_error_only_for_subgraph(
            specs in arb_specs(),
            name in "[a-z]{1,10}",
        ) {
            let wd = build(&specs);
            prop_assume!(!wd.is_valid_scope(&name));
            let err = wd.subgraph_for_scope(&name).unwrap_err();
            prop_assert_eq!(err.scope(), name.as_str());
            prop_assert_eq!(err.valid_scopes(), ["alpha", "beta", "delta"]);
            for phase in PhaseId::ALL {
                prop_assert!(wd.first_in_scope_stage_of_phase(phase, &name).is_none());
            }
            prop_assert!(wd.stages_in_scope(&name).is_empty());
        }

        /// `stages_in_scope` は全ステージを文書順で返し、`action` は静的グリッドの 3 値。
        #[test]
        fn stages_in_scope_lists_every_stage_in_document_order(
            specs in arb_specs(),
            scope_index in 0usize..REGISTERED.len(),
        ) {
            let wd = build(&specs);
            let scope = REGISTERED[scope_index];
            let listed = wd.stages_in_scope(scope);
            prop_assert_eq!(listed.len(), wd.graph().len());
            for (i, (s, phase, action)) in listed.iter().enumerate() {
                let n = wd.graph().at(i).unwrap();
                prop_assert_eq!(*s, n.slug());
                prop_assert_eq!(*phase, n.phase());
                prop_assert_eq!(*action, wd.grid().action(scope, n.slug()));
            }
        }

        /// `first_in_scope_stage_of_phase` は subgraph の数値順で最初の該当フェーズ。
        #[test]
        fn first_in_scope_stage_of_phase_agrees_with_the_subgraph(
            specs in arb_specs(),
            scope_index in 0usize..REGISTERED.len(),
        ) {
            let wd = build(&specs);
            let scope = REGISTERED[scope_index];
            let sub = wd.subgraph_for_scope(scope).unwrap();
            for phase in PhaseId::ALL {
                let expected = sub.iter().find(|n| n.phase() == phase).map(|n| n.slug());
                let actual = wd
                    .first_in_scope_stage_of_phase(phase, scope)
                    .map(StageNode::slug);
                prop_assert_eq!(actual, expected);
            }
        }
    }
}
