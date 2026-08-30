//! `IntentExecution` 集約 — 1 つの intent の実行状態 (10 §2.1) をイベントソーシング形の
//! FSM として持つ集約ルート (ADR-001 / ADR-002)。
//!
//! **状態としてのデータ**(カーソル・CheckboxState・`Status` と直交する park マーカー・recompose
//! オーバレイ・AutonomyMode・ゲート承認履歴・差し戻し回数)、**状態遷移**(12 の decide コマンド)、
//! **判断**(`next_decision` / `jump_resolve` / `stale_report`) を 1 つの型に閉じ込める。
//!
//! - **1 コマンド 1 イベント** (BR1.1): 各 decide はガードを全て通してからイベントを 1 つ構築し、
//!   `apply_event` で自身に適用して返す。ガード不成立の `Err` では `self` に触れない。
//! - **通常実行とリプレイは同一経路** (BR2.3): 状態を動かすのは `apply_event` だけであり、decide は
//!   「どのイベントを起こすか」を決めるだけである。
//! - **ゲート判定はフェーズ**で決まる (BR1.3): `gated(s) = stages[s].phase != initialization`。
//!   索引 0 の特別扱いはしない (実グラフの initialization は 3 ステージある)。Quint slice-1 の
//!   `gated(s) = s != 0` は initialization 1 ステージの合成計画に対する抽象で、ITF 準拠テストは
//!   その合成計画で駆動する (BR2.5)。
//! - **時計を持たない** (NFR3.1): `occurred_at` は呼出側 (ユースケース) が Clock から渡す。
//! - **楽観 version は持たない** (ADR-010 / B7): 本家 event-store-adapter-rs v3.0.0 で
//!   `Aggregate` trait が廃れ、楽観ロックの版数は `SnapshotEnvelope::version()` (ストアの列) が
//!   正本になった。集約が持つ順序番号は `seq_nr` **だけ**であり、ストアの採番トークンとは混ざらない。
//!   **直列化の記述は持たない** (改訂 9 / `coding-rules/domain-persistence-neutrality.md`) —
//!   行のバイトを決めるのはアダプタ層の DTO で、復号は状態の写し (memento) を経由し
//!   [`IntentExecution::new`] の検査点を必ず通る。したがって「不変条件を満たす集約
//!   しか存在しない」という保証は永続化経路でも破れない (security-design §2 の検査点 3)。
//! - **panic しない** (NFR4.3): ステージ位置は `StageIndex` で型保証し、範囲外は `Option::None` /
//!   `Err` で表す。`# Panics` を持つ公開 API は無い。
//!
//! 意味論の形式的正本は `formal/orchestration/engine_loop.qnt` (slice 1 v2)。ITF 準拠テスト
//! (`tests/engine_loop_conformance.rs`) がモデルトレースを再生して射影を突き合わせる。

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};

use super::apply_error::ApplyError;
use super::autonomy_mode::AutonomyMode;
use super::command_error::CommandError;
use super::intent::Intent;
use super::intent_execution_error::IntentExecutionError;
use super::intent_execution_event::{
    AutonomyModeSet, GateApproved, GateOpened, GateRejected, IntentExecutionEvent, Jumped, Parked,
    Recomposed, StageCompleted, StageRevised, StageSkipped, Started,
};
use super::intent_execution_id::IntentExecutionId;
use super::intent_id::IntentId;
use super::jump_direction::JumpDirection;
use super::next_decision::{NextDecision, NextRequest};
use super::phase_boundary::PhaseBoundary;
use super::stage_entry::StageEntry;
use super::stage_index::StageIndex;
use super::stage_key::StageKey;
use super::status::Status;
use crate::workflow_definition::{PlanAction, StageSlug, WorkflowDefinition};
use crate::workspace::CheckboxState;

/// 前進 (`complete_stage` / `approve_gate`) と差し戻し (`reject_gate`) が受理する checkbox 集合。
///
/// これは**本集約が所有する遷移の前提集合** (I7 ゲート前提) であって、`CheckboxState` の一般分類
/// (in-flight / finished / active) ではない (tell-dont-ask.md「集約所有の前提集合」)。
// amadeus-lint: allow(checkbox-vocabulary) — I7: 集約が所有するゲート遷移の前提集合
const GATE_ADVANCE_PRECONDITION: [CheckboxState; 2] =
    [CheckboxState::InProgress, CheckboxState::AwaitingApproval];

/// `skip_stage` が受理する checkbox 集合 (I13 skipped 受理前提)。
///
/// 同じ集合が「実効 SKIP のカーソルから自力で復旧できるか」の判定にもなる — 復旧手段が
/// `skip_stage` そのものだからである (BR3.1 (5) の `RecoverSkipInconsistency`)。
// amadeus-lint: allow(checkbox-vocabulary) — I13: 集約が所有する skip 受理の前提集合
const SKIP_PRECONDITION: [CheckboxState; 2] = [CheckboxState::InProgress, CheckboxState::Revising];

/// エンジンループの状態機械 (集約ルート)。
///
/// serde はアダプタ層の永続化 DTO (`WireIntentExecution`) が持ち、復号は完全コンストラクタ
/// [`IntentExecution::new`] を必ず通る — 検査点が 1 か所に保たれる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentExecution {
    id: IntentExecutionId,
    /// 実行の対象 intent — **ID で参照し、`Intent` を埋め込まない**
    /// (coding-rules/aggregate-references.md)。1 intent : n 実行なので、埋め込めば同じ静的
    /// 材料の複製を n 個抱えることになる。計画が要る判断は `&Intent` を引数で受け取り、
    /// 受け取り時にこの ID と照合する。
    intent_id: IntentId,
    /// イベント適用の添字帳 (slug + phase)。イベントはステージを slug で名指すので、
    /// 適用の自己完結 (本家同型の `replay(snapshot, events)`) にこの最小複製が要る
    /// (オーナー裁定 2026-08-30)。表示属性・計画・条件フラグは複製しない — それらが要る
    /// 判断は従来どおり `&Intent` を引数で受け取る。
    stage_keys: Vec<StageKey>,
    /// 実効プラン (recompose オーバレイ)。静的な計画 (`Intent::stages`) は保持しない。
    overlay: Vec<PlanAction>,
    checkbox: Vec<CheckboxState>,
    cursor: StageIndex,
    status: Status,
    parked_at: Option<StageIndex>,
    autonomy: AutonomyMode,
    approved: Vec<bool>,
    revision_count: Vec<u32>,
    seq_nr: usize,
    /// ストアが採番した楽観 version — **次の書込に提示する不透明トークン**である。
    ///
    /// 採番するのはストアであり正本はスナップショット行の列 (本家 v3 の
    /// `SnapshotEnvelope::version()`) だが、読んだ版を書込まで引き回すのは集約の仕事である
    /// (オーナー裁定 2026-08-30 — 読んだ時点の版で書くから、その間の書込を競合として弾ける)。
    /// 再構成した Repository が [`IntentExecution::with_version`] で刻み、書込む Repository が
    /// [`IntentExecution::version`] で読む。まだ 1 度も永続化していない集約は
    /// [`IntentExecution::UNPERSISTED_VERSION`] を持つ。
    ///
    /// `seq_nr` とは別物である — あちらはドメインが採番する順序番号で、値がたまたま一致する
    /// ことがあっても意味は違う。**型は基本データ型 `usize` のままにする** — 不透明トークンを
    /// newtype で包む案は却下済みであり、`seq_nr` と隣り合っていても包まない
    /// (オーナー裁定 2026-08-30。取り違えは型ではなく規律とレビューで防ぐ)。
    version: usize,
    /// 最後に適用したイベントの発生時刻。集約は時計を持たないので、この値は常に適用した
    /// イベントから来る (NFR3.1)。Repository はこれをイベント封筒の `occurred_at` に使う。
    last_updated_at: DateTime<Utc>,
}

impl IntentExecution {
    /// まだ 1 度も永続化していない集約が提示する版 (新規作成の楽観 version)。
    ///
    /// 本家 v3 の規約「新規作成は `seq_nr == 1` かつ `version == 0`」の 0 に名前を与えた
    /// ものである — 呼出側にも Repository 実装にも裸の `0` を書かせない。
    pub const UNPERSISTED_VERSION: usize = 0;

    // ---- W1: 生成 (BR2.2 / BR2.6) ----

    /// intent を 1 回実行し始める (genesis)。
    ///
    /// 受け取る [`Intent`] は Always Valid（空でない計画・initialization は EXECUTE かつ
    /// 非 CONDITIONAL）なので、ここに失敗経路は無い。計画の解決とスコープ検査は
    /// [`Intent::create`] が済ませている。
    ///
    /// 実行の識別子は**呼出側がミントする** — upstream がツール層で uuid をミントするのと
    /// 同じ位置づけであり、集約は時計も乱数も持たない。集約が控えるのは `intent_id` と
    /// 実行時状態だけで、静的な材料は `Started` が歴史として運ぶ
    /// (coding-rules/aggregate-references.md「イベントに材料の複製が載るのは違反ではない」)。
    ///
    /// 戻り値が**対**なのは規則である (coding-rules/aggregate-commands.md) — Repository の
    /// 永続化は `store(&event, &aggregate, ..)` の形でジャーナル追記分 (誕生イベント) と
    /// スナップショット分 (適用後の集約) を同一トランザクションで受け取るので、どちらが
    /// 欠けても永続化が組めない。再構成経路 (`from_snapshot` / `apply_event`) は逆に
    /// **イベントを作らない** — 作ればリプレイのたびに歴史が増える。
    #[must_use]
    pub fn start(
        id: IntentExecutionId,
        intent: Intent,
        occurred_at: DateTime<Utc>,
    ) -> (IntentExecution, IntentExecutionEvent) {
        let execution = IntentExecution::genesis_state(id, &intent, occurred_at);
        (
            execution,
            IntentExecutionEvent::Started(Started::new(intent)),
        )
    }

    /// テスト専用: 通番だけ差し替えた複製 (通番枯渇の境界を memento 無しで作るため)。
    #[cfg(test)]
    const fn with_seq_nr(mut self, seq_nr: usize) -> IntentExecution {
        self.seq_nr = seq_nr;
        self
    }

    /// genesis 時点の状態を組む (構造体リテラルはここだけ — [`IntentExecution::start`] と
    /// [`IntentExecution::replay`] の両経路が必ずここを通る)。
    fn genesis_state(
        id: IntentExecutionId,
        intent: &Intent,
        occurred_at: DateTime<Utc>,
    ) -> IntentExecution {
        let count = intent.stage_count();
        let stage_keys: Vec<StageKey> = intent
            .stages()
            .iter()
            .map(|entry| StageKey::new(entry.slug().clone(), entry.phase()))
            .collect();
        let overlay: Vec<PlanAction> = intent
            .stages()
            .iter()
            .map(StageEntry::plan_action)
            .collect();
        let mut checkbox = vec![CheckboxState::Pending; count];
        if let Some(first) = checkbox.first_mut() {
            *first = CheckboxState::InProgress;
        }
        IntentExecution {
            id,
            intent_id: intent.id().clone(),
            stage_keys,
            overlay,
            checkbox,
            cursor: StageIndex::new(0),
            status: Status::Running,
            parked_at: None,
            autonomy: AutonomyMode::Gated,
            approved: vec![false; count],
            revision_count: vec![0; count],
            seq_nr: 1,
            version: IntentExecution::UNPERSISTED_VERSION,
            last_updated_at: occurred_at,
        }
    }

    /// ストアが採番した版を刻む (**Repository 実装専用**)。
    ///
    /// 再構成の最後にストアが読んだ版を載せるための一手である — ユースケースは呼ばない。
    /// 版はドメインが導出できない値 (正本はスナップショット行の列) なので、外から刻む口が
    /// 要る。`store` を通ったあとの集約が持つ版は**書込前に提示した版**のままであり、次の
    /// 書込は再構成からやり直す (`coding-rules` の楽観ロック方針)。
    #[must_use]
    pub const fn with_version(mut self, version: usize) -> IntentExecution {
        self.version = version;
        self
    }

    /// 完全コンストラクタ — 全フィールドを検査して組む (Always Valid)。
    ///
    /// ストア境界の復元 DTO はこの口を必ず通る — 検査を迂回する構築口は存在しない
    /// (`coding-rules/domain-persistence-neutrality.md`)。スナップショットとは**ある時点の
    /// 集約そのもの**であり、専用の型も専用のコンストラクタ名も無い (オーナー裁定
    /// 2026-08-30「集約そのものがすでに snapshot」)。楽観 version はここでは載らない
    /// (正本は行の列 — [`IntentExecution::with_version`] で刻む)。
    ///
    /// # Errors
    ///
    /// 空の計画・ステージ数と食い違う実行時ベクトル・範囲外のカーソル / parked 位置・
    /// 通番 0・集約不変条件の違反。壊れた行はアダプタが `Corrupt` へ写す (BR1.5)。
    #[expect(
        clippy::too_many_arguments,
        reason = "完全コンストラクタ — 集約と構造同一の memento 写し型を作らない \
                  (オーナー裁定 2026-08-30) ため、フィールドがそのまま引数になる"
    )]
    pub fn new(
        id: IntentExecutionId,
        intent_id: IntentId,
        stage_keys: Vec<StageKey>,
        overlay: Vec<PlanAction>,
        checkbox: Vec<CheckboxState>,
        cursor: usize,
        status: Status,
        parked_at: Option<usize>,
        autonomy: AutonomyMode,
        approved: Vec<bool>,
        revision_count: Vec<u32>,
        seq_nr: usize,
        last_updated_at: DateTime<Utc>,
    ) -> Result<IntentExecution, IntentExecutionError> {
        let count = stage_keys.len();
        if count == 0 {
            return Err(IntentExecutionError::new("empty plan"));
        }
        for (name, actual) in [
            ("overlay", overlay.len()),
            ("checkbox", checkbox.len()),
            ("approved", approved.len()),
            ("revision_count", revision_count.len()),
        ] {
            if actual != count {
                return Err(IntentExecutionError::new(format!(
                    "{name} length {actual} does not match {count} stages"
                )));
            }
        }
        if cursor >= count {
            return Err(IntentExecutionError::new(format!(
                "cursor {cursor} out of bounds for {count} stages"
            )));
        }
        if let Some(parked) = parked_at
            && parked >= count
        {
            return Err(IntentExecutionError::new(format!(
                "parked_at {parked} out of bounds for {count} stages"
            )));
        }
        if seq_nr == 0 {
            return Err(IntentExecutionError::new("seq_nr must be at least 1"));
        }
        let execution = IntentExecution {
            id,
            intent_id,
            stage_keys,
            overlay,
            checkbox,
            cursor: StageIndex::new(cursor),
            status,
            parked_at: parked_at.map(StageIndex::new),
            autonomy,
            approved,
            revision_count,
            seq_nr,
            version: IntentExecution::UNPERSISTED_VERSION,
            last_updated_at,
        };
        execution
            .check_invariants()
            .map_err(|violation| IntentExecutionError::new(format!("invariant: {violation}")))?;
        Ok(execution)
    }

    /// スナップショットと以降の差分イベントから集約を復元する (Event Sourcing の再生経路)。
    ///
    /// 本家 v3 の example (`UserAccount::replay(events, snapshot)`) と同型 — スナップショット
    /// (= ある時点の集約そのもの。特別な型は無い) を基底に、その通番より後のイベントを
    /// [`IntentExecution::apply_event`] で畳み込む。通常実行とリプレイの同一経路 (BR1.1)。
    /// 集約は添字帳 ([`StageKey`]) を自分で持つので、再生に外部材料は要らない
    /// (オーナー裁定 2026-08-30「replay や apply_event が集約側に必要」)。
    ///
    /// # Panics
    ///
    /// 壊れた歴史 (通番の飛び・未知ステージ・不変条件違反)。再構成は失敗を返さない —
    /// 壊れた歴史は回復せずクラッシュする (オーナー裁定 2026-08-30)。
    #[must_use]
    pub fn replay(
        snapshot: IntentExecution,
        events: impl IntoIterator<Item = (usize, DateTime<Utc>, IntentExecutionEvent)>,
    ) -> IntentExecution {
        let mut execution = snapshot;
        for (seq_nr, occurred_at, event) in events {
            execution.apply_event(seq_nr, occurred_at, &event);
        }
        execution
    }

    // ---- 観測 (read model) ----

    /// この実行の識別子 (以後不変)。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionId {
        &self.id
    }

    /// 実行の対象 intent の識別子 (以後不変。`intents.json` の uuid にあたる)。
    ///
    /// 集約が持つのはこの ID だけである — [`Intent`] そのものは埋め込まない
    /// (coding-rules/aggregate-references.md)。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 渡された intent がこの実行のものかを確かめる (取り違え防御)。
    ///
    /// ID 参照だからこそ書ける照合である。長さも見るのは、同じ intent の別リビジョンで
    /// 実行時ベクトルと計画の長さが食い違う写しを弾くためである。
    fn matches(&self, intent: &Intent) -> bool {
        intent.id() == &self.intent_id && intent.stage_count() == self.overlay.len()
    }

    /// 適用済みイベント数と一致する順序番号 (`Started` = 1 — BR2.1)。
    ///
    /// 次のイベントの通番は `seq_nr + 1` であり、`commit` を通ったあとの値は**そのイベントの
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

    /// イベント適用の添字帳 (slug + phase — 計画のステージ順)。スナップショット DTO が読む。
    #[must_use]
    pub fn stage_keys(&self) -> &[StageKey] {
        &self.stage_keys
    }

    /// 最後に適用したイベントの発生時刻 (集約は時計を持たない — NFR3.1)。
    ///
    /// `commit` を通ったあとの値は**そのイベントの発生時刻**であり、封筒の `occurred_at` になる。
    #[must_use]
    pub const fn last_updated_at(&self) -> &DateTime<Utc> {
        &self.last_updated_at
    }

    /// 実行が追いかけているステージ総数 (スコープ外のステージも含む)。
    ///
    /// 実行時ベクトルの長さがそのまま総数である — 静的な計画は [`Intent`] 側にあり、
    /// 集約はその長さぶんの実行時状態を持つ。
    #[must_use]
    pub const fn stage_count(&self) -> usize {
        self.overlay.len()
    }

    /// 生の位置から `StageIndex` を作る唯一の公開経路。範囲外は `None` (BR5.1)。
    #[must_use]
    pub fn stage_index(&self, value: usize) -> Option<StageIndex> {
        (value < self.stage_count()).then(|| StageIndex::new(value))
    }

    /// `Current Stage` の位置。
    #[must_use]
    pub const fn cursor(&self) -> StageIndex {
        self.cursor
    }

    /// `Status` 行の現在値 (park マーカーとは直交)。
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// park マーカーが記録している位置 (`None` は未 park)。
    #[must_use]
    pub const fn parked_at(&self) -> Option<StageIndex> {
        self.parked_at
    }

    /// 現在の `Construction Autonomy Mode`。
    #[must_use]
    pub const fn autonomy(&self) -> AutonomyMode {
        self.autonomy
    }

    /// 名指しステージの checkbox マーカー。範囲外は `None`。
    #[must_use]
    pub fn checkbox(&self, stage: StageIndex) -> Option<CheckboxState> {
        self.checkbox.get(stage.to_usize()).copied()
    }

    /// 名指しステージのゲート承認履歴。範囲外は `None`。
    #[must_use]
    pub fn approved(&self, stage: StageIndex) -> Option<bool> {
        self.approved.get(stage.to_usize()).copied()
    }

    /// 名指しステージの差し戻し回数。範囲外は `None`。
    #[must_use]
    pub fn revision_count(&self, stage: StageIndex) -> Option<u32> {
        self.revision_count.get(stage.to_usize()).copied()
    }

    /// 実効プラン — オーバレイ (recompose) が静的グリッドに勝つ (BR4.2)。範囲外は `None`。
    #[must_use]
    pub fn effective_plan(&self, stage: StageIndex) -> Option<PlanAction> {
        self.overlay.get(stage.to_usize()).copied()
    }

    /// ゲート付きか — `phase != initialization` (BR1.3)。範囲外・intent 不一致は `None`。
    #[must_use]
    pub fn gated(&self, intent: &Intent, stage: StageIndex) -> Option<bool> {
        if !self.matches(intent) {
            return None;
        }
        self.key_at(stage).map(StageKey::is_gated)
    }

    /// parked 分岐の発火は導出述語 (マーカー有 ∧ 位置一致 — BR1.7)。
    #[must_use]
    pub fn parked_active(&self) -> bool {
        self.parked_at == Some(self.cursor)
    }

    /// コマンド受理述語 (BR1.0)。偽なら `unpark` 以外の decide は `NotRunning`。
    #[must_use]
    pub fn accepts_commands(&self) -> bool {
        self.status.is_running() && !self.parked_active()
    }

    // ---- 内部の索引ヘルパ (すべて `StageIndex` 経由 — 生の添字を使わない) ----

    fn key_at(&self, stage: StageIndex) -> Option<&StageKey> {
        self.stage_keys.get(stage.to_usize())
    }

    fn is_gated(&self, stage: StageIndex) -> bool {
        self.key_at(stage).is_some_and(StageKey::is_gated)
    }

    fn in_scope(&self, stage: StageIndex) -> bool {
        self.effective_plan(stage) == Some(PlanAction::Execute)
    }

    fn next_in_scope(&self, after: StageIndex) -> Option<StageIndex> {
        ((after.to_usize() + 1)..self.stage_count())
            .map(StageIndex::new)
            .find(|&stage| self.in_scope(stage))
    }

    fn next_in_scope_slug(&self, after: StageIndex) -> Option<StageSlug> {
        self.next_in_scope(after)
            .and_then(|stage| self.key_at(stage))
            .map(|key| key.slug().clone())
    }

    /// 名指しステージの完了で跨ぐフェーズ境界 (跨がなければ `None`)。
    ///
    /// 走査は [`IntentExecution::next_in_scope_slug`] と同じ「次の実効 EXECUTE ステージ」で
    /// あり、`plan` と `overlay` を知っている集約だけが正しく答えられる。フェーズが違えば
    /// 境界、同じか次が無い (= 最終) なら `None` — upstream の `crossesPhaseBoundary`
    /// (`aidlc-state.ts` の `completedStage.phase !== nextStage.phase`) と同一の規則である。
    fn crossed_phase_boundary(&self, stage: StageIndex) -> Option<PhaseBoundary> {
        let from = self.key_at(stage)?.phase();
        let to = self.key_at(self.next_in_scope(stage)?)?.phase();
        (from != to).then(|| PhaseBoundary::new(from, to))
    }

    fn slug_of(&self, stage: StageIndex) -> Result<StageSlug, CommandError> {
        self.key_at(stage)
            .map(|key| key.slug().clone())
            .ok_or(CommandError::InvalidTarget(stage))
    }

    /// 計画上の位置へ解決する。
    ///
    /// 取り違えガードはここには置かない — 呼出経路 (`apply_event` → `mutate` → `advance`) の
    /// 入口で既に照合済みだからである。二重に置くと到達しない枝が残る。
    fn resolve(&self, slug: &StageSlug) -> Result<StageIndex, ApplyError> {
        self.stage_keys
            .iter()
            .position(|key| key.slug() == slug)
            .map(StageIndex::new)
            .ok_or_else(|| ApplyError::UnknownStage(slug.clone()))
    }

    /// ステージに状態の印を付ける (状態ファイルのチェックボックスがこの印の表現)。
    fn mark_stage(&mut self, stage: StageIndex, value: CheckboxState) {
        if let Some(slot) = self.checkbox.get_mut(stage.to_usize()) {
            *slot = value;
        }
    }

    /// ステージの承認を記録する (`GateApproved` の適用)。
    fn record_approval(&mut self, stage: StageIndex) {
        if let Some(slot) = self.approved.get_mut(stage.to_usize()) {
            *slot = true;
        }
    }

    /// ステージの承認履歴を無効化する (BR1.6 — jump が承認を巻き戻す)。
    fn invalidate_approval(&mut self, stage: StageIndex) {
        if let Some(slot) = self.approved.get_mut(stage.to_usize()) {
            *slot = false;
        }
    }

    // ---- ガード ----

    /// BR1.0 — コマンドを受理できる状態か検査し、カーソルを返す。
    /// 渡された intent がこの実行のものであることを確かめてから、受理述語を見る。
    ///
    /// 取り違えガードを**入口 1 か所**に置く — 後段の索引ヘルパは不一致なら `None` を返すが、
    /// それは `InvalidTarget` に見えてしまい原因を取り違える。
    fn guard_running_for(&self, intent: &Intent) -> Result<StageIndex, CommandError> {
        if !self.matches(intent) {
            return Err(CommandError::IntentMismatch);
        }
        self.guard_running()
    }

    fn guard_running(&self) -> Result<StageIndex, CommandError> {
        if self.accepts_commands() {
            Ok(self.cursor)
        } else {
            Err(CommandError::NotRunning)
        }
    }

    fn require_checkbox(
        &self,
        stage: StageIndex,
        allowed: &[CheckboxState],
    ) -> Result<CheckboxState, CommandError> {
        let actual = self
            .checkbox(stage)
            .ok_or(CommandError::InvalidTarget(stage))?;
        if allowed.contains(&actual) {
            Ok(actual)
        } else {
            Err(CommandError::CheckboxPrecondition { stage, actual })
        }
    }

    fn require_gated(&self, stage: StageIndex, gated: bool) -> Result<(), CommandError> {
        if self.is_gated(stage) == gated {
            Ok(())
        } else {
            Err(CommandError::InvalidTarget(stage))
        }
    }

    /// ガードを通過したイベントを自身に適用して返す (BR1.1)。
    ///
    /// 通番と発生時刻は**適用の引数**であって、イベントに載る材料ではない (ADR-010 / B7 —
    /// 輸送のメタデータは封筒が運ぶ)。適用が通れば `self.seq_nr` がそのイベントの通番、
    /// `self.last_updated_at` がその発生時刻になるので、封筒を組む Repository はそこから読む。
    ///
    /// `apply_event` は通番違反・未知 slug・不変条件違反でクラッシュするが、ここへ来る
    /// イベントは通番を自分で採番し slug を自分の `stages` から取り、遷移も不変条件を保つ
    /// ので、そのいずれにも該当しない (該当したらプログラミング誤りであり、クラッシュが正 —
    /// オーナー裁定 2026-08-30)。通番枯渇だけは入口で明示に拒否する (`SequenceExhausted` —
    /// 飽和加算で seq_nr が停滞したまま成功を装わない)。
    fn commit(
        &mut self,
        event: IntentExecutionEvent,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let Some(seq_nr) = self.seq_nr.checked_add(1) else {
            return Err(CommandError::SequenceExhausted);
        };
        self.apply_event(seq_nr, occurred_at, &event);
        Ok(event)
    }

    // ---- W2: decide (11 コマンド、1 コマンド 1 イベント) ----
    //
    // 以下のコマンドはすべて `Result<IntentExecutionEvent, CommandError>` を返す —
    // 「集約の `&mut self` コマンドは必ず単一のドメインイベントを戻り値で返す」という規則
    // (coding-rules/aggregate-commands.md) の形である。イベントは書込パイプラインの産物
    // (受領証) であって読取チャネルではないので、CQS の「Command は戻り値なし」は集約には
    // 適用しない (同規則が command-query-separation.md を精密化する)。拒否は無言の no-op に
    // せず、ガード付きの `Err` で返す。
    //
    // 計画が要るコマンドは `&Intent` を引数で受け取り、入口で取り違えを照合する
    // (coding-rules/aggregate-references.md)。

    /// 非ゲート (initialization フェーズ) ステージの完了 — `StageCompleted`。
    ///
    /// # Errors
    ///
    /// 非受理 (`NotRunning`)、ゲート付きステージでの呼出 (`InvalidTarget`)、checkbox 前提違反
    /// (`CheckboxPrecondition`) を拒否する。
    pub fn complete_stage(
        &mut self,
        intent: &Intent,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let stage = self.guard_running_for(intent)?;
        self.require_gated(stage, false)?;
        self.require_checkbox(stage, &GATE_ADVANCE_PRECONDITION)?;
        let material = StageCompleted::new(self.slug_of(stage)?, self.next_in_scope_slug(stage));
        self.commit(IntentExecutionEvent::StageCompleted(material), occurred_at)
    }

    /// 承認ゲートの開放 — `GateOpened`。`artifacts` は呼出側が渡す投影材料 (C5)。
    ///
    /// # Errors
    ///
    /// 非受理、非ゲートステージ (`InvalidTarget`)、in-progress 以外 (`CheckboxPrecondition`) を
    /// 拒否する (「only an in-progress stage can open a gate」)。
    pub fn open_gate(
        &mut self,
        intent: &Intent,
        artifacts: Vec<String>,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let stage = self.guard_running_for(intent)?;
        self.require_gated(stage, true)?;
        self.require_checkbox(stage, &[CheckboxState::InProgress])?;
        let material = GateOpened::new(self.slug_of(stage)?, artifacts);
        self.commit(IntentExecutionEvent::GateOpened(material), occurred_at)
    }

    /// 承認ゲートの通過 — `GateApproved`。フェーズ境界は**集約が自分の計画から導出する**。
    ///
    /// `open_gate` を省いた in-progress からの承認も受理する (BR1.3)。
    ///
    /// 導出は [`IntentExecution::crossed_phase_boundary`] が持つ。呼出側 (ユースケース) は
    /// フロー制御だけを担い、判断は集約に閉じる (オーナー統一ルール「集約は FSM」)。
    ///
    /// # Errors
    ///
    /// 非受理、非ゲートステージ (`InvalidTarget`)、checkbox 前提違反を拒否する。
    pub fn approve_gate(
        &mut self,
        intent: &Intent,
        user_input: Option<String>,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let stage = self.guard_running_for(intent)?;
        self.require_gated(stage, true)?;
        self.require_checkbox(stage, &GATE_ADVANCE_PRECONDITION)?;
        let material = GateApproved::new(
            self.slug_of(stage)?,
            user_input,
            self.next_in_scope_slug(stage),
            self.crossed_phase_boundary(stage),
        );
        self.commit(IntentExecutionEvent::GateApproved(material), occurred_at)
    }

    /// 承認ゲートでの差し戻し — `GateRejected`。改訂回数を +1 してイベントに載せる (BR1.4)。
    ///
    /// # Errors
    ///
    /// 非受理、非ゲートステージ (`InvalidTarget`)、checkbox 前提違反を拒否する。
    pub fn reject_gate(
        &mut self,
        intent: &Intent,
        feedback: Option<String>,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let stage = self.guard_running_for(intent)?;
        self.require_gated(stage, true)?;
        self.require_checkbox(stage, &GATE_ADVANCE_PRECONDITION)?;
        let next = self
            .revision_count(stage)
            .ok_or(CommandError::InvalidTarget(stage))?
            .saturating_add(1);
        let material = GateRejected::new(self.slug_of(stage)?, feedback, next);
        self.commit(IntentExecutionEvent::GateRejected(material), occurred_at)
    }

    /// 差し戻し後のゲート再入 — `StageRevised`。
    ///
    /// # Errors
    ///
    /// 非受理、revising 以外 (`CheckboxPrecondition`) を拒否する
    /// (「only a revising stage can re-enter its gate」)。
    pub fn revise_stage(
        &mut self,
        intent: &Intent,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let stage = self.guard_running_for(intent)?;
        self.require_checkbox(stage, &[CheckboxState::Revising])?;
        let material = StageRevised::new(self.slug_of(stage)?);
        self.commit(IntentExecutionEvent::StageRevised(material), occurred_at)
    }

    /// ステージの読み飛ばし — `StageSkipped` (CONDITIONAL または実効 SKIP のみ — BR1.5)。
    ///
    /// # Errors
    ///
    /// 非受理、checkbox 前提違反、CONDITIONAL でも実効 SKIP でもない場合 (`NotSkippable`) を
    /// 拒否する。
    pub fn skip_stage(
        &mut self,
        intent: &Intent,
        reason: String,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let stage = self.guard_running_for(intent)?;
        self.require_checkbox(stage, &SKIP_PRECONDITION)?;
        // conditional は複製しない静的材料 — コマンド文脈なので intent から直接引く
        // (guard_running_for が取り違えを照合済み)。
        let conditional = intent
            .stages()
            .get(stage.to_usize())
            .is_some_and(StageEntry::is_conditional);
        if !(conditional || self.effective_plan(stage) == Some(PlanAction::Skip)) {
            return Err(CommandError::NotSkippable(stage));
        }
        let material =
            StageSkipped::new(self.slug_of(stage)?, reason, self.next_in_scope_slug(stage));
        self.commit(IntentExecutionEvent::StageSkipped(material), occurred_at)
    }

    /// カーソルの移動 — `Jumped` (BR1.6)。差分集合をイベントに載せ、承認の消去は適用側が
    /// `direction` と `target` から決定的に導出する。
    ///
    /// # Errors
    ///
    /// [`IntentExecution::jump_resolve`] と同じ (`NotRunning` / `InvalidTarget`)。
    pub fn jump(
        &mut self,
        intent: &Intent,
        target: StageIndex,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let direction = self.jump_resolve(intent, target)?;
        let source = self.cursor;
        let mut stages_reset = Vec::new();
        let mut stages_skipped = Vec::new();
        match direction {
            JumpDirection::Forward => {
                for value in source.to_usize()..target.to_usize() {
                    let stage = StageIndex::new(value);
                    let Some(marker) = self.checkbox(stage) else {
                        continue;
                    };
                    let skip_current = value == source.to_usize() && marker.is_active();
                    let skip_between = value > source.to_usize() && marker.is_in_flight();
                    if skip_current || skip_between {
                        stages_skipped.push(self.slug_of(stage)?);
                    }
                }
            }
            JumpDirection::Backward => {
                for value in (target.to_usize() + 1)..self.stage_count() {
                    let stage = StageIndex::new(value);
                    if self.in_scope(stage) && self.checkbox(stage) != Some(CheckboxState::Pending)
                    {
                        stages_reset.push(self.slug_of(stage)?);
                    }
                }
            }
            JumpDirection::Redo => {}
        }
        let material = Jumped::new(
            direction,
            self.slug_of(source)?,
            self.slug_of(target)?,
            stages_reset,
            stages_skipped,
        );
        self.commit(IntentExecutionEvent::Jumped(material), occurred_at)
    }

    /// park マーカーの設置 — `Parked` (autonomous 下は拒否 — BR1.7)。
    ///
    /// # Errors
    ///
    /// 非受理 (`NotRunning`)、autonomous 中 (`RefusedUnderAutonomy`)。
    pub fn park(
        &mut self,
        intent: &Intent,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let stage = self.guard_running_for(intent)?;
        if self.autonomy.is_autonomous() {
            return Err(CommandError::RefusedUnderAutonomy);
        }
        let material = Parked::new(self.slug_of(stage)?);
        self.commit(IntentExecutionEvent::Parked(material), occurred_at)
    }

    /// park マーカーの除去 — `Unparked`。位置は `parked_at` から復元される (BR1.7)。
    ///
    /// # Errors
    ///
    /// park が活性でなければ `NotRunning`。
    pub fn unpark(
        &mut self,
        intent: &Intent,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        if !self.matches(intent) {
            return Err(CommandError::IntentMismatch);
        }
        if !self.parked_active() {
            return Err(CommandError::NotRunning);
        }
        self.commit(IntentExecutionEvent::Unparked, occurred_at)
    }

    /// 実効プランの再形成 — `Recomposed` (BR1.8)。反転対象は 1 件以上で、いずれかが不正なら
    /// 全体を `Err` にする (部分適用しない)。
    ///
    /// # Errors
    ///
    /// 非受理、autonomous 中 (`RefusedUnderAutonomy`)、対象が空・カーソル以前・範囲外
    /// (`InvalidTarget`)、pending 以外 (`CheckboxPrecondition`)。
    pub fn recompose(
        &mut self,
        intent: &Intent,
        flips: &[StageIndex],
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        let cursor = self.guard_running_for(intent)?;
        if self.autonomy.is_autonomous() {
            return Err(CommandError::RefusedUnderAutonomy);
        }
        let targets: BTreeSet<usize> = flips.iter().map(|stage| stage.to_usize()).collect();
        if targets.is_empty() {
            return Err(CommandError::InvalidTarget(cursor));
        }
        for &value in &targets {
            let stage = StageIndex::new(value);
            if value <= cursor.to_usize() || value >= self.stage_count() {
                return Err(CommandError::InvalidTarget(stage));
            }
            self.require_checkbox(stage, &[CheckboxState::Pending])?;
        }

        let mut projected = self.overlay.clone();
        let mut skipped = Vec::new();
        let mut added = Vec::new();
        for &value in &targets {
            let stage = StageIndex::new(value);
            let slug = self.slug_of(stage)?;
            match self.effective_plan(stage) {
                Some(PlanAction::Execute) => skipped.push(slug),
                Some(PlanAction::Skip) => added.push(slug),
                None => return Err(CommandError::InvalidTarget(stage)),
            }
            if let Some(slot) = projected.get_mut(value) {
                *slot = slot.flipped();
            }
        }
        let stages_in_scope = projected
            .iter()
            .enumerate()
            .filter(|(_, action)| **action == PlanAction::Execute)
            .filter_map(|(index, _)| {
                self.key_at(StageIndex::new(index))
                    .map(|key| key.slug().clone())
            })
            .collect();
        let material = Recomposed::new(skipped, added, stages_in_scope);
        self.commit(IntentExecutionEvent::Recomposed(material), occurred_at)
    }

    /// 自律モードを切り替える — `AutonomyModeSet` (BR1.8)。
    ///
    /// 方向は 2 つあり、仕様はそれぞれを**昇格**(gated → autonomous) と**降格**(その逆) と呼ぶ。
    /// 本メソッドは両方向を受ける。**昇格だけが human presence を要する** (I11) が、その
    /// ガードはユースケース層にある (監査台帳の射影が要る) — ここは状態変更のみ。
    ///
    /// 発する監査イベント文字列 `AUTONOMY_MODE_SET` と CLI 動詞 `set-autonomy` は upstream の
    /// Published Language なので逐語で維持するが、**本メソッド名は外に出ない**のでドメインの語
    /// を使う (coding-rules/ubiquitous-language.md)。
    ///
    /// # Errors
    ///
    /// 非受理なら `NotRunning`。
    pub fn switch_autonomy(
        &mut self,
        intent: &Intent,
        mode: AutonomyMode,
        occurred_at: DateTime<Utc>,
    ) -> Result<IntentExecutionEvent, CommandError> {
        self.guard_running_for(intent)?;
        self.commit(
            IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(mode)),
            occurred_at,
        )
    }

    // ---- W3: apply (リプレイと通常実行の同一経路 — BR2.1 / BR2.3) ----

    /// イベントを 1 つ適用する。通常実行 (decide 経由) とリプレイの唯一の状態遷移経路。
    ///
    /// `seq_nr` と `occurred_at` は封筒が運ぶ材料なので**引数で受け取る** (ADR-010 / B7)。
    /// 採番と連続性の検査は依然としてドメインの責務である — 本家 v3 は「採番・連続性は
    /// 利用側 (ドメイン) の責務」と明文化しており、ライブラリは飛び番を検出しない。
    ///
    /// **失敗を返さない** — 再構成は歴史を読むだけであり、通番の飛び・未知 slug・不変条件
    /// 違反は「壊れた歴史」か「プログラミング誤り」なので回復せずクラッシュする
    /// (オーナー裁定 2026-08-30、本家 v3 サンプル同型)。検査は一時コピーに対して行い、
    /// 成功した場合だけ差し替える。
    ///
    /// # Panics
    ///
    /// 別 intent の計画を渡された、通番が現在値 + 1 でない・枯渇した、イベントのステージ
    /// slug が `stages` に無い、適用後に集約不変条件が破れる場合。
    #[allow(
        clippy::panic,
        reason = "壊れた歴史は回復不能 — 再構成は失敗を返さずクラッシュする (オーナー裁定 2026-08-30)"
    )]
    pub fn apply_event(
        &mut self,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        event: &IntentExecutionEvent,
    ) {
        let expected = self
            .seq_nr
            .checked_add(1)
            .unwrap_or_else(|| panic!("apply_event: sequence exhausted at {}", self.seq_nr));
        assert_eq!(
            seq_nr, expected,
            "apply_event: sequence gap (expected {expected}, actual {seq_nr})"
        );
        let mut next = self.clone();
        next.mutate(event)
            .unwrap_or_else(|error| panic!("apply_event: corrupted history — {error}"));
        next.seq_nr = seq_nr;
        next.last_updated_at = occurred_at;
        next.check_invariants()
            .unwrap_or_else(|violation| panic!("apply_event: invariant violated — {violation}"));
        *self = next;
    }

    /// 12 変種の網羅 match (NFR1.3)。`#[non_exhaustive]` を付けないので腕の欠落はビルドで落ちる。
    fn mutate(&mut self, event: &IntentExecutionEvent) -> Result<(), ApplyError> {
        match event {
            IntentExecutionEvent::Started(_) => {
                // `Started` は genesis 専用 — 既存の集約には適用できない (BR2.2)。
                return Err(ApplyError::InvariantViolation(
                    "Started applies only at genesis".to_string(),
                ));
            }
            IntentExecutionEvent::StageCompleted(completed) => {
                let stage = self.resolve(completed.stage())?;
                self.mark_stage(stage, CheckboxState::Completed);
                self.advance(completed.next_stage())?;
            }
            IntentExecutionEvent::GateOpened(opened) => {
                let stage = self.resolve(opened.stage())?;
                self.mark_stage(stage, CheckboxState::AwaitingApproval);
            }
            IntentExecutionEvent::GateApproved(approved) => {
                let stage = self.resolve(approved.stage())?;
                self.record_approval(stage);
                self.mark_stage(stage, CheckboxState::Completed);
                self.advance(approved.next_stage())?;
            }
            IntentExecutionEvent::GateRejected(rejected) => {
                let stage = self.resolve(rejected.stage())?;
                self.mark_stage(stage, CheckboxState::Revising);
                if let Some(slot) = self.revision_count.get_mut(stage.to_usize()) {
                    *slot = rejected.revision_count();
                }
            }
            IntentExecutionEvent::StageRevised(revised) => {
                let stage = self.resolve(revised.stage())?;
                self.mark_stage(stage, CheckboxState::AwaitingApproval);
            }
            IntentExecutionEvent::StageSkipped(skipped) => {
                let stage = self.resolve(skipped.stage())?;
                self.mark_stage(stage, CheckboxState::Skipped);
                self.advance(skipped.next_stage())?;
            }
            IntentExecutionEvent::Jumped(jumped) => {
                self.apply_jump(jumped)?;
            }
            IntentExecutionEvent::Parked(parked) => {
                let stage = self.resolve(parked.stage())?;
                self.parked_at = Some(stage);
            }
            IntentExecutionEvent::Unparked => {
                self.parked_at = None;
            }
            IntentExecutionEvent::Recomposed(recomposed) => {
                for slug in recomposed.skipped() {
                    let stage = self.resolve(slug)?;
                    if let Some(slot) = self.overlay.get_mut(stage.to_usize()) {
                        *slot = PlanAction::Skip;
                    }
                }
                for slug in recomposed.added() {
                    let stage = self.resolve(slug)?;
                    if let Some(slot) = self.overlay.get_mut(stage.to_usize()) {
                        *slot = PlanAction::Execute;
                    }
                }
            }
            IntentExecutionEvent::AutonomyModeSet(set) => {
                self.autonomy = set.mode();
            }
        }
        Ok(())
    }

    fn apply_jump(&mut self, jumped: &Jumped) -> Result<(), ApplyError> {
        let source = self.resolve(jumped.source())?;
        let target = self.resolve(jumped.target())?;
        for slug in jumped.stages_reset() {
            let stage = self.resolve(slug)?;
            self.mark_stage(stage, CheckboxState::Pending);
        }
        for slug in jumped.stages_skipped() {
            let stage = self.resolve(slug)?;
            self.mark_stage(stage, CheckboxState::Skipped);
        }
        match jumped.direction() {
            // backward は target 以降の承認履歴を、redo は出発点の承認履歴を無効化する (BR1.6)。
            JumpDirection::Backward => {
                for value in target.to_usize()..self.stage_count() {
                    self.invalidate_approval(StageIndex::new(value));
                }
            }
            JumpDirection::Redo => self.invalidate_approval(source),
            JumpDirection::Forward => {}
        }
        self.mark_stage(target, CheckboxState::InProgress);
        self.cursor = target;
        Ok(())
    }

    /// 完了・スキップの後段 — 次の in-scope ステージへ進むか、無ければ完了する (BR1.5)。
    fn advance(&mut self, next_stage: Option<&StageSlug>) -> Result<(), ApplyError> {
        match next_stage {
            Some(slug) => {
                let stage = self.resolve(slug)?;
                self.mark_stage(stage, CheckboxState::InProgress);
                self.cursor = stage;
            }
            None => self.status = Status::Completed,
        }
        Ok(())
    }

    /// 集約不変条件 (Quint の cursor_in_scope / at_most_one_active / no_gate_bypass /
    /// parked_position)。材料は不変条件名で、文言はアダプタ層の責務。
    ///
    /// memento 復元経路の撤去 (オーナー裁定 2026-08-30 — 再構成はジャーナル全再生) に伴い、
    /// 復号ガードだった構造検査 (長さ整合・通番 0・範囲外カーソル) は削除した — genesis が
    /// 長さを固定し、遷移は `resolve` で索引を束縛するため、構成不能である。
    fn check_invariants(&self) -> Result<(), String> {
        if let Some(parked) = self.parked_at
            && parked != self.cursor
        {
            return Err("parked_position".to_string());
        }
        if self.accepts_commands() && !self.in_scope(self.cursor) {
            return Err("cursor_in_scope".to_string());
        }
        let active = (0..self.stage_count())
            .filter_map(|value| self.checkbox(StageIndex::new(value)))
            .filter(|marker| marker.is_active())
            .count();
        if active > 1 {
            return Err(format!("at_most_one_active: {active}"));
        }
        for value in 0..self.stage_count() {
            let stage = StageIndex::new(value);
            if self.is_gated(stage)
                && self.checkbox(stage) == Some(CheckboxState::Completed)
                && self.approved(stage) != Some(true)
            {
                return Err(format!("no_gate_bypass at stage {value}"));
            }
        }
        Ok(())
    }

    // ---- W4 / W5: クエリ (書込なし) ----

    /// 現状態から次に何をすべきかを 1 つ決める (BR3.1 の優先順)。書込なし。
    ///
    /// 第 2 引数の定義は id の照合にだけ使う — 計画は `Started` で自己完結しているため、現時点の
    /// 分岐は定義の内容を参照しない (将来の分岐のための契約上の引数)。
    ///
    /// # Errors
    ///
    /// 引数の定義 id が `definition_id` と異なれば `DefinitionMismatch` (revision の差は Ok — BR2.6)。
    pub fn next_decision(
        &self,
        intent: &Intent,
        definition: &WorkflowDefinition,
        request: &NextRequest,
    ) -> Result<NextDecision, CommandError> {
        if !self.matches(intent) {
            return Err(CommandError::IntentMismatch);
        }
        if definition.id() != intent.definition_id() {
            return Err(CommandError::DefinitionMismatch {
                expected: intent.definition_id().clone(),
                actual: definition.id().clone(),
            });
        }
        if self.parked_active() && !request.is_reentry() {
            return Ok(if request.is_resume() {
                NextDecision::UnparkThenResume
            } else {
                NextDecision::Parked { stage: self.cursor }
            });
        }
        if request.is_resume() {
            return Ok(NextDecision::ResumeMenu);
        }
        if request.is_free_text() {
            return Ok(NextDecision::NewWorkRouting);
        }
        if !self.status.is_running() {
            return Ok(NextDecision::Done);
        }
        let cursor = self.cursor;
        if let Some(marker) = self.checkbox(cursor)
            && marker.is_in_flight()
        {
            if self.effective_plan(cursor) == Some(PlanAction::Skip) {
                // 実効 SKIP のステージに run-stage は出さない。自力で `skip_stage` を呼べる
                // 前提集合 (SKIP_PRECONDITION) にいるときだけ復旧可能と報告する。
                return Ok(if SKIP_PRECONDITION.contains(&marker) {
                    NextDecision::RecoverSkipInconsistency {
                        stage: cursor,
                        checkbox: marker,
                    }
                } else {
                    NextDecision::InconsistentSkip {
                        stage: cursor,
                        checkbox: marker,
                    }
                });
            }
            return Ok(NextDecision::RunStage {
                stage: cursor,
                gate: self.is_gated(cursor),
            });
        }
        Ok(match self.next_in_scope(cursor) {
            Some(stage) => NextDecision::RunStage {
                stage,
                gate: self.is_gated(stage),
            },
            None => NextDecision::Done,
        })
    }

    /// jump の検証と方向の導出 (書込なし — `aidlc-jump resolve` に対応、BR3.3)。
    ///
    /// # Errors
    ///
    /// 非受理 (`NotRunning`)、範囲外・initialization・スコープ外ターゲット、initialization カーソルの
    /// redo (`InvalidTarget`) を拒否する。
    pub fn jump_resolve(
        &self,
        intent: &Intent,
        target: StageIndex,
    ) -> Result<JumpDirection, CommandError> {
        if !self.matches(intent) {
            return Err(CommandError::IntentMismatch);
        }
        if !self.accepts_commands() {
            return Err(CommandError::NotRunning);
        }
        if target.to_usize() >= self.stage_count() {
            return Err(CommandError::InvalidTarget(target));
        }
        let direction = JumpDirection::of(self.cursor.to_usize(), target.to_usize());
        match direction {
            // INIT_JUMP_ERROR: initialization フェーズのステージへは跳べない。scope 外も不可。
            JumpDirection::Forward | JumpDirection::Backward => {
                if !self.is_gated(target) || !self.in_scope(target) {
                    return Err(CommandError::InvalidTarget(target));
                }
            }
            JumpDirection::Redo => {
                if !self.is_gated(self.cursor) {
                    return Err(CommandError::InvalidTarget(target));
                }
            }
        }
        Ok(direction)
    }

    /// カーソル通過済み completed への再報告は**何もコミットせず**冪等 done (BR1.9)。
    ///
    /// # Errors
    ///
    /// 非受理 (`NotRunning`)、カーソル通過済み completed でない対象 (`NotStale`)。
    pub fn stale_report(&self, stage: StageIndex) -> Result<NextDecision, CommandError> {
        if !self.accepts_commands() {
            return Err(CommandError::NotRunning);
        }
        if stage.to_usize() >= self.cursor.to_usize()
            || self.checkbox(stage) != Some(CheckboxState::Completed)
        {
            return Err(CommandError::NotStale(stage));
        }
        Ok(NextDecision::Done)
    }
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なため同様に許容する。
    #![allow(clippy::indexing_slicing, clippy::panic)]

    use super::*;
    use crate::orchestration::{
        AutonomyMode, CommandError, Created, EngineSignal, Intent, IntentError, IntentEvent,
        IntentExecutionEvent, IntentExecutionId, IntentId, JumpDirection, NextDecision,
        NextRequest, PhaseBoundary, StageCompleted, StageDisplay, StageEntry, StageIndex,
        StartRequest, Started, Status, WorkspaceScan,
    };
    use crate::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, ExecutionKind, PhaseId, PlanAction, ScopeGrid,
        ScopeMetadata, StageGraph, StageMode, StageNode, StageNodeBuilder, StageNumber, StageSlug,
        WorkflowDefinition, WorkflowDefinitionId,
    };
    use crate::workspace::CheckboxState;
    use std::collections::BTreeMap;

    use CheckboxState::{AwaitingApproval, Completed, InProgress, Pending, Revising, Skipped};
    use PlanAction::{Execute, Skip};

    /// ITF 再生も含め、集約は `occurred_at` を素通しするだけなので固定値でよい。
    const AT_TEXT: &str = "2026-08-23T00:00:00Z";

    fn occurred() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(AT_TEXT)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn slug(i: usize) -> StageSlug {
        StageSlug::parse(&format!("stage-{i}")).unwrap()
    }

    fn intent_id() -> IntentId {
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
    }

    fn execution_id() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap()
    }

    /// 実行と、その計画を持つ intent を束ねたテストフィクスチャ。
    ///
    /// 本番のコマンド・クエリは `&Intent` を引数で受け取る
    /// (`coding-rules/aggregate-references.md`) が、テストでは実行と intent が常に対で動くので、
    /// 束ねて転送する。intent を要さない面は `Deref` でそのまま集約へ抜ける。取り違えガード
    /// そのものを見るテストは、この転送を通さず生の API を直に呼ぶ。
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Run {
        intent: Intent,
        execution: IntentExecution,
    }

    impl std::ops::Deref for Run {
        type Target = IntentExecution;

        fn deref(&self) -> &IntentExecution {
            &self.execution
        }
    }

    impl Run {
        fn start(intent: Intent) -> Run {
            let (execution, _) = IntentExecution::start(execution_id(), intent.clone(), occurred());
            Run { intent, execution }
        }

        fn genesis(intent: Intent) -> (Run, IntentExecutionEvent) {
            let (execution, event) =
                IntentExecution::start(execution_id(), intent.clone(), occurred());
            (Run { intent, execution }, event)
        }

        fn stages(&self) -> &[StageEntry] {
            self.intent.stages()
        }

        fn definition_id(&self) -> &WorkflowDefinitionId {
            self.intent.definition_id()
        }

        fn definition_revision(&self) -> &DefinitionRevision {
            self.intent.definition_revision()
        }

        fn gated(&self, stage: StageIndex) -> Option<bool> {
            self.execution.gated(&self.intent, stage)
        }

        fn complete_stage(
            &mut self,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution.complete_stage(&self.intent, occurred_at)
        }

        fn open_gate(
            &mut self,
            artifacts: Vec<String>,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution
                .open_gate(&self.intent, artifacts, occurred_at)
        }

        fn approve_gate(
            &mut self,
            user_input: Option<String>,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution
                .approve_gate(&self.intent, user_input, occurred_at)
        }

        fn reject_gate(
            &mut self,
            feedback: Option<String>,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution
                .reject_gate(&self.intent, feedback, occurred_at)
        }

        fn revise_stage(
            &mut self,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution.revise_stage(&self.intent, occurred_at)
        }

        fn skip_stage(
            &mut self,
            reason: String,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution.skip_stage(&self.intent, reason, occurred_at)
        }

        fn jump(
            &mut self,
            target: StageIndex,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution.jump(&self.intent, target, occurred_at)
        }

        fn park(
            &mut self,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution.park(&self.intent, occurred_at)
        }

        fn unpark(
            &mut self,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution.unpark(&self.intent, occurred_at)
        }

        fn recompose(
            &mut self,
            flips: &[StageIndex],
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution.recompose(&self.intent, flips, occurred_at)
        }

        fn switch_autonomy(
            &mut self,
            mode: AutonomyMode,
            occurred_at: DateTime<Utc>,
        ) -> Result<IntentExecutionEvent, CommandError> {
            self.execution
                .switch_autonomy(&self.intent, mode, occurred_at)
        }

        fn apply_event(
            &mut self,
            seq_nr: usize,
            occurred_at: DateTime<Utc>,
            event: &IntentExecutionEvent,
        ) {
            self.execution.apply_event(seq_nr, occurred_at, event);
        }

        fn next_decision(
            &self,
            definition: &WorkflowDefinition,
            request: NextRequest,
        ) -> Result<NextDecision, CommandError> {
            self.execution
                .next_decision(&self.intent, definition, &request)
        }

        fn jump_resolve(&self, target: StageIndex) -> Result<JumpDirection, CommandError> {
            self.execution.jump_resolve(&self.intent, target)
        }
    }

    fn def_id(value: &str) -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse(value).unwrap()
    }

    fn revision(fill: char) -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64))).unwrap()
    }

    /// 索引 < `init` を initialization、残りを inception にした合成計画。
    fn entries(init: usize, actions: &[PlanAction], conditional: &[bool]) -> Vec<StageEntry> {
        actions
            .iter()
            .zip(conditional.iter())
            .enumerate()
            .map(|(i, (action, cond))| {
                let phase = if i < init {
                    PhaseId::Initialization
                } else {
                    PhaseId::Inception
                };
                StageEntry::new(
                    slug(i),
                    phase,
                    *action,
                    *cond,
                    display(&format!("{}.{}", phase.index(), i + 1)),
                )
            })
            .collect()
    }

    fn start_request() -> StartRequest {
        StartRequest::new("classic", "build it")
    }

    /// テストの表示属性 (投影は見ないので番号・表題・担当は固定でよい)。
    fn display(number: &str) -> StageDisplay {
        StageDisplay::new(StageNumber::parse(number).unwrap(), "Stage", "orchestrator").unwrap()
    }

    /// テストの走査結果。
    fn scan() -> WorkspaceScan {
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .unwrap()
    }

    /// 合成計画から intent を組む (検査は `From<Created>` の 1 か所)。
    fn plan(init: usize, actions: &[PlanAction], conditional: &[bool]) -> Intent {
        Intent::from(Created::new(
            intent_id(),
            def_id("claude"),
            revision('0'),
            start_request(),
            entries(init, actions, conditional),
            scan(),
        ))
    }

    fn start_with(init: usize, actions: &[PlanAction], conditional: &[bool]) -> Run {
        Run::start(plan(init, actions, conditional))
    }

    fn all_exec(n: usize) -> Run {
        start_with(1, &vec![Execute; n], &vec![false; n])
    }

    /// フェーズと実効プランを名指しした合成計画で開始する (フェーズ境界の導出を見るテスト用)。
    fn start_from_phased_plan(phased: &[(PhaseId, PlanAction)]) -> Run {
        let stages = phased
            .iter()
            .enumerate()
            .map(|(i, (phase, action))| {
                StageEntry::new(
                    slug(i),
                    *phase,
                    *action,
                    false,
                    display(&format!("{}.{}", phase.index(), i + 1)),
                )
            })
            .collect();
        Run::start(Intent::from(Created::new(
            intent_id(),
            def_id("claude"),
            revision('0'),
            start_request(),
            stages,
            scan(),
        )))
    }

    /// 全ステージ EXECUTE の、フェーズだけ名指しした合成計画。
    fn phased(phases: &[PhaseId]) -> Run {
        let plan: Vec<(PhaseId, PlanAction)> =
            phases.iter().map(|phase| (*phase, Execute)).collect();
        start_from_phased_plan(&plan)
    }

    /// 定義から計画を解決して実行を開始する (旧 7 引数の genesis に相当)。
    fn start_from_definition(
        definition: &WorkflowDefinition,
        request: StartRequest,
    ) -> (Run, IntentExecutionEvent) {
        // genesis は (集約, 誕生イベント) の対を返す。実行を起こすのに要るのは対の左である
        // (改訂 8 / coding-rules/aggregate-commands.md)。
        let (intent, _created) = Intent::create(intent_id(), definition, request, scan()).unwrap();
        Run::genesis(intent)
    }

    /// カーソルのゲートを承認し、生まれた `GateApproved` が載せた境界を取り出す。
    fn approval_boundary(w: &mut Run) -> Option<PhaseBoundary> {
        let event = w.approve_gate(None, occurred()).unwrap();
        let IntentExecutionEvent::GateApproved(approved) = &event else {
            panic!("expected GateApproved");
        };
        approved.phase_boundary()
    }

    fn at(w: &IntentExecution, i: usize) -> StageIndex {
        w.stage_index(i).unwrap()
    }

    /// `next_decision` の第 2 引数用の最小定義 (id の照合にしか使われない — BR3.1)。
    fn bare_definition(id: &str) -> WorkflowDefinition {
        WorkflowDefinition::from_artifacts(
            def_id(id),
            revision('0'),
            StageGraph::new(Vec::new()).unwrap(),
            ScopeGrid::new(BTreeMap::new()),
            BTreeMap::new(),
        )
    }

    fn node(name: &str, number: &str, phase: PhaseId, execution: ExecutionKind) -> StageNode {
        StageNodeBuilder::new(
            StageSlug::parse(name).unwrap(),
            StageNumber::parse(number).unwrap(),
            name.to_string(),
            phase,
            execution,
            StageMode::Inline,
        )
        .scopes(vec!["classic".to_string()])
        .build()
    }

    /// 文書順 = 数値順の小さな出荷グラフ相当 (initialization 1 + ideation 2)。
    fn shipped_definition(grid: ScopeGrid) -> WorkflowDefinition {
        let graph = StageGraph::new(vec![
            node(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                ExecutionKind::Always,
            ),
            node(
                "intent-capture",
                "1.1",
                PhaseId::Ideation,
                ExecutionKind::Always,
            ),
            node(
                "market-research",
                "1.2",
                PhaseId::Ideation,
                ExecutionKind::Conditional,
            ),
        ])
        .unwrap();
        let scopes: BTreeMap<String, ScopeMetadata> = [(
            "classic".to_string(),
            ScopeMetadata::new("classic").unwrap(),
        )]
        .into_iter()
        .collect();
        WorkflowDefinition::from_artifacts(def_id("claude"), revision('a'), graph, grid, scopes)
    }

    fn full_grid() -> ScopeGrid {
        let column: BTreeMap<StageSlug, PlanAction> = [
            (StageSlug::parse("state-init").unwrap(), Execute),
            (StageSlug::parse("intent-capture").unwrap(), Execute),
            (StageSlug::parse("market-research").unwrap(), Execute),
        ]
        .into_iter()
        .collect();
        ScopeGrid::new([("classic".to_string(), column)].into_iter().collect())
    }

    /// intent の解決済み計画をそのまま実効プランへ写す (birth 時の overlay)。
    // ---- W1: start (BR2.2 / BR2.6) ----

    #[test]
    fn an_execution_starts_from_the_left_of_the_intent_create_pair() {
        // 改訂 8: `Intent` も集約なので genesis は対を返す。実行を起こすのに渡すのは
        // **対の左** (集約インスタンス) であり、誕生イベントは呼出側が `store` へ回す。
        let (intent, created) = Intent::create(
            intent_id(),
            &shipped_definition(full_grid()),
            start_request(),
            scan(),
        )
        .unwrap();
        // 誕生イベントは材料 (値) を運ぶ — 変換で対の左と同じ集約に戻る。
        let IntentEvent::Created(payload) = created;
        assert_eq!(Intent::from(payload), intent);

        let (execution, started) =
            IntentExecution::start(execution_id(), intent.clone(), occurred());
        assert_eq!(execution.intent_id(), intent.id());
        let IntentExecutionEvent::Started(started) = &started else {
            panic!("start must emit Started");
        };
        // `Started` は intent を丸ごと運ぶ現行形のままである (BR2.2 自己完結)。
        assert_eq!(started.intent(), &intent);
    }

    #[test]
    fn start_records_the_definition_identity_and_the_resolved_plan() {
        let definition = shipped_definition(full_grid());
        let (w, event) = start_from_definition(&definition, start_request());

        // 通番・発生時刻・識別子は封筒 (アダプタ層) の材料であり、イベント自身は持たない。
        // genesis 直後の集約がその 3 点を保持している (B7)。
        assert_eq!(w.seq_nr(), 1);
        assert_eq!(w.last_updated_at(), &occurred());
        assert_eq!(w.id(), &execution_id());
        assert_eq!(w.intent_id(), &intent_id());
        let IntentExecutionEvent::Started(started) = &event else {
            panic!("start must emit Started");
        };
        assert_eq!(started.definition_id(), definition.id());
        assert_eq!(started.definition_revision(), definition.revision());
        assert_eq!(started.scope(), "classic");
        assert_eq!(started.request(), "build it");
        assert_eq!(started.stages().len(), 3);
        assert_eq!(started.stages()[0].phase(), PhaseId::Initialization);
        assert!(!started.stages()[1].is_conditional());
        assert!(started.stages()[2].is_conditional());

        assert_eq!(w.stage_count(), 3);
        assert_eq!(w.cursor(), at(&w, 0));
        assert_eq!(w.checkbox(at(&w, 0)), Some(InProgress));
        assert_eq!(w.checkbox(at(&w, 1)), Some(Pending));
        assert_eq!(w.status(), Status::Running);
        assert_eq!(w.autonomy(), AutonomyMode::Gated);
        assert_eq!(w.parked_at(), None);
        assert_eq!(w.definition_id(), definition.id());
        assert_eq!(w.definition_revision(), definition.revision());
        assert_eq!(w.revision_count(at(&w, 0)), Some(0));
    }

    #[test]
    fn start_carries_the_depth_and_test_strategy_the_caller_resolved() {
        // C5 の Started payload は depth / test_strategy を持つ (U4 が Scope Configuration を
        // 描く材料)。集約はこの 2 値に意味論を持たず、素通しでイベントに載せるだけである。
        let definition = shipped_definition(full_grid());
        let request = StartRequest::new("classic", "build it")
            .with_depth("standard")
            .with_test_strategy("comprehensive");
        let (_, event) = start_from_definition(&definition, request);
        let IntentExecutionEvent::Started(started) = &event else {
            panic!("start must emit Started");
        };
        assert_eq!(started.scope(), "classic");
        assert_eq!(started.request(), "build it");
        assert_eq!(started.depth(), Some("standard"));
        assert_eq!(started.test_strategy(), Some("comprehensive"));

        // 省略時は None のまま載る (フラグ未指定 = 既定の解決は呼出側の責務)。
        let bare = StartRequest::new("classic", "build it");
        let (_, plain) = start_from_definition(&definition, bare);
        let IntentExecutionEvent::Started(started) = &plain else {
            panic!("start must emit Started");
        };
        assert_eq!(started.depth(), None);
        assert_eq!(started.test_strategy(), None);
    }

    #[test]
    fn an_unknown_scope_is_refused_with_the_definition_material() {
        let definition = shipped_definition(full_grid());
        let unknown = StartRequest::new("nope", "build it");
        let err = Intent::create(intent_id(), &definition, unknown, scan()).unwrap_err();
        let IntentError::UnknownScope(scope) = err else {
            panic!("expected UnknownScope");
        };
        assert_eq!(scope.scope(), "nope");
        assert_eq!(scope.valid_scopes(), ["classic".to_string()]);
    }

    #[test]
    fn a_missing_grid_cell_folds_to_skip_outside_initialization() {
        // グリッドに列が無いステージは `None → SKIP` に畳む (BR2.2)。
        let column: BTreeMap<StageSlug, PlanAction> =
            [(StageSlug::parse("state-init").unwrap(), Execute)]
                .into_iter()
                .collect();
        let grid = ScopeGrid::new([("classic".to_string(), column)].into_iter().collect());
        let (w, _) = start_from_definition(&shipped_definition(grid), start_request());
        assert_eq!(w.effective_plan(at(&w, 1)), Some(Skip));
        assert_eq!(w.effective_plan(at(&w, 2)), Some(Skip));
    }

    // 計画そのものの不変条件（空・initialization の SKIP / CONDITIONAL・先頭 SKIP）は
    // `From<Created>` が持つようになったので、その拒否のテストは `intent.rs` にある。

    // ---- 取り違えガード (aggregate-references.md — ID 参照だから照合が書ける) ----

    /// 同じ形の計画を、**別の intent 識別子**で組む。
    fn foreign_plan(n: usize) -> Intent {
        Intent::from(Created::new(
            IntentId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").unwrap(),
            def_id("claude"),
            revision('0'),
            start_request(),
            entries(1, &vec![Execute; n], &vec![false; n]),
            scan(),
        ))
    }

    #[test]
    fn a_command_refuses_an_intent_that_belongs_to_another_intent() {
        let mut w = all_exec(3);
        assert_eq!(
            w.execution
                .complete_stage(&foreign_plan(3), occurred())
                .unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(w.seq_nr(), 1, "拒否では状態が動かない");
    }

    #[test]
    fn a_command_refuses_an_intent_whose_plan_length_disagrees() {
        // 同じ intent でも、実行時ベクトルと計画の長さが食い違う写しは受け取らない。
        let mut w = all_exec(3);
        let shorter = plan(1, &[Execute, Execute], &[false, false]);
        assert_eq!(shorter.id(), w.intent_id(), "識別子は一致している前提");
        assert_eq!(
            w.execution
                .complete_stage(&shorter, occurred())
                .unwrap_err(),
            CommandError::IntentMismatch
        );
    }

    #[test]
    fn every_intent_taking_command_refuses_a_foreign_intent() {
        let mut w = all_exec(3);
        let foreign = foreign_plan(3);
        let at0 = occurred();
        assert_eq!(
            w.execution
                .open_gate(&foreign, Vec::new(), at0)
                .unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(
            w.execution.approve_gate(&foreign, None, at0).unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(
            w.execution.reject_gate(&foreign, None, at0).unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(
            w.execution.revise_stage(&foreign, at0).unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(
            w.execution
                .skip_stage(&foreign, "x".to_string(), at0)
                .unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(
            w.execution.park(&foreign, at0).unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(
            w.execution.unpark(&foreign, at0).unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(
            w.execution.recompose(&foreign, &[], at0).unwrap_err(),
            CommandError::IntentMismatch
        );
        assert_eq!(
            w.execution
                .switch_autonomy(&foreign, AutonomyMode::Autonomous, at0)
                .unwrap_err(),
            CommandError::IntentMismatch
        );
        let target = at(&w, 1);
        assert_eq!(
            w.execution.jump(&foreign, target, at0).unwrap_err(),
            CommandError::IntentMismatch
        );
    }

    #[test]
    fn the_queries_that_need_the_plan_refuse_a_foreign_intent() {
        let w = all_exec(3);
        let foreign = foreign_plan(3);
        let definition = bare_definition("claude");
        assert_eq!(
            w.execution
                .next_decision(&foreign, &definition, &NextRequest::default()),
            Err(CommandError::IntentMismatch)
        );
        assert_eq!(
            w.execution.jump_resolve(&foreign, at(&w, 1)),
            Err(CommandError::IntentMismatch)
        );
        assert_eq!(
            w.execution.gated(&foreign, at(&w, 1)),
            None,
            "ゲート付きかは他人の計画からは答えない"
        );
    }

    // ---- W2: 12 コマンド (BR1.0〜BR1.9) ----

    #[test]
    fn a_gated_stage_cannot_complete_without_passing_through_approval() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        assert_eq!(w.cursor(), at(&w, 1));
        w.open_gate(Vec::new(), occurred()).unwrap();
        w.approve_gate(None, occurred()).unwrap();
        assert_eq!(w.approved(at(&w, 1)), Some(true));
        assert_eq!(w.checkbox(at(&w, 1)), Some(Completed));
    }

    #[test]
    fn complete_stage_is_refused_on_a_gated_stage() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let target = at(&w, 1);
        assert_eq!(
            w.complete_stage(occurred()),
            Err(CommandError::InvalidTarget(target))
        );
    }

    #[test]
    fn approve_gate_and_the_gate_openers_are_refused_on_a_non_gated_stage() {
        let mut w = all_exec(3);
        let target = at(&w, 0);
        assert_eq!(
            w.approve_gate(None, occurred()),
            Err(CommandError::InvalidTarget(target))
        );
        assert_eq!(
            w.open_gate(Vec::new(), occurred()),
            Err(CommandError::InvalidTarget(target))
        );
        assert_eq!(
            w.reject_gate(None, occurred()),
            Err(CommandError::InvalidTarget(target))
        );
    }

    #[test]
    fn approve_gate_accepts_the_open_gate_shortcut() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        // open_gate を省いた in-progress からの承認も受理する (BR1.3)。
        let event = w.approve_gate(Some("ok".to_string()), occurred()).unwrap();
        let IntentExecutionEvent::GateApproved(approved) = &event else {
            panic!("expected GateApproved");
        };
        assert_eq!(approved.user_input(), Some("ok"));
        assert_eq!(approved.stage(), &slug(1));
        assert_eq!(approved.next_stage(), Some(&slug(2)));
        assert_eq!(w.checkbox(at(&w, 1)), Some(Completed));
    }

    #[test]
    fn gate_lifecycle_preconditions_are_strict() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        assert!(matches!(
            w.revise_stage(occurred()),
            Err(CommandError::CheckboxPrecondition { .. })
        ));
        w.open_gate(Vec::new(), occurred()).unwrap();
        assert!(matches!(
            w.open_gate(Vec::new(), occurred()),
            Err(CommandError::CheckboxPrecondition { .. })
        ));
        w.reject_gate(None, occurred()).unwrap();
        assert_eq!(w.checkbox(at(&w, 1)), Some(Revising));
        w.revise_stage(occurred()).unwrap();
        assert_eq!(w.checkbox(at(&w, 1)), Some(AwaitingApproval));
    }

    #[test]
    fn reject_gate_increments_the_revision_count_and_carries_it() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let first = w.reject_gate(Some("redo".to_string()), occurred()).unwrap();
        let IntentExecutionEvent::GateRejected(rejected) = &first else {
            panic!("expected GateRejected");
        };
        assert_eq!(rejected.revision_count(), 1);
        assert_eq!(rejected.feedback(), Some("redo"));
        assert_eq!(w.revision_count(at(&w, 1)), Some(1));
        w.revise_stage(occurred()).unwrap();
        w.reject_gate(None, occurred()).unwrap();
        assert_eq!(w.revision_count(at(&w, 1)), Some(2));
    }

    #[test]
    fn skipped_is_refused_unless_conditional_or_plan_skip() {
        let mut w = start_with(1, &[Execute, Execute, Execute], &[false, false, true]);
        w.complete_stage(occurred()).unwrap();
        let cursor = at(&w, 1);
        assert_eq!(
            w.skip_stage("no".to_string(), occurred()),
            Err(CommandError::NotSkippable(cursor))
        );
        w.approve_gate(None, occurred()).unwrap();
        let event = w.skip_stage("conditional".to_string(), occurred()).unwrap();
        let IntentExecutionEvent::StageSkipped(skipped) = &event else {
            panic!("expected StageSkipped");
        };
        assert_eq!(skipped.reason(), "conditional");
        assert_eq!(skipped.next_stage(), None);
        assert_eq!(w.status(), Status::Completed);
        assert_eq!(w.checkbox(at(&w, 2)), Some(Skipped));
    }

    #[test]
    fn forward_jump_skips_intervening_in_flight_stages() {
        let mut w = all_exec(5);
        w.complete_stage(occurred()).unwrap();
        let target = at(&w, 3);
        let event = w.jump(target, occurred()).unwrap();
        let IntentExecutionEvent::Jumped(jumped) = &event else {
            panic!("expected Jumped");
        };
        assert_eq!(jumped.direction(), JumpDirection::Forward);
        assert_eq!(jumped.source(), &slug(1));
        assert_eq!(jumped.target(), &slug(3));
        assert_eq!(jumped.stages_skipped(), [slug(1), slug(2)]);
        assert!(jumped.stages_reset().is_empty());
        assert_eq!(w.checkbox(at(&w, 1)), Some(Skipped));
        assert_eq!(w.checkbox(at(&w, 2)), Some(Skipped));
        assert_eq!(w.checkbox(at(&w, 3)), Some(InProgress));
        assert_eq!(w.cursor(), target);
    }

    #[test]
    fn backward_jump_resets_downstream_and_invalidates_approvals() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        w.open_gate(Vec::new(), occurred()).unwrap();
        w.approve_gate(None, occurred()).unwrap();
        let target = at(&w, 1);
        let event = w.jump(target, occurred()).unwrap();
        let IntentExecutionEvent::Jumped(jumped) = &event else {
            panic!("expected Jumped");
        };
        assert_eq!(jumped.direction(), JumpDirection::Backward);
        assert_eq!(jumped.stages_reset(), [slug(2)]);
        assert_eq!(w.checkbox(at(&w, 1)), Some(InProgress));
        assert_eq!(w.checkbox(at(&w, 2)), Some(Pending));
        assert_eq!(w.approved(at(&w, 1)), Some(false));
    }

    #[test]
    fn jump_to_an_initialization_stage_is_refused() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let target = at(&w, 0);
        assert_eq!(
            w.jump(target, occurred()),
            Err(CommandError::InvalidTarget(target))
        );
        assert_eq!(
            w.jump_resolve(target),
            Err(CommandError::InvalidTarget(target))
        );
    }

    #[test]
    fn redo_reopens_the_cursor_and_drops_its_approval() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        w.open_gate(Vec::new(), occurred()).unwrap();
        w.reject_gate(None, occurred()).unwrap();
        let cursor = w.cursor();
        assert_eq!(w.jump_resolve(cursor), Ok(JumpDirection::Redo));
        w.jump(cursor, occurred()).unwrap();
        assert_eq!(w.checkbox(cursor), Some(InProgress));
        assert_eq!(w.approved(cursor), Some(false));
    }

    #[test]
    fn a_redo_on_an_initialization_cursor_is_refused() {
        let w = all_exec(3);
        let cursor = w.cursor();
        assert_eq!(
            w.jump_resolve(cursor),
            Err(CommandError::InvalidTarget(cursor))
        );
    }

    #[test]
    fn park_preserves_position_and_autonomous_park_is_refused() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let event = w.park(occurred()).unwrap();
        let IntentExecutionEvent::Parked(parked) = &event else {
            panic!("expected Parked");
        };
        assert_eq!(parked.stage(), &slug(1));
        assert!(w.parked_active());
        assert!(!w.accepts_commands());
        w.unpark(occurred()).unwrap();
        assert_eq!(w.cursor(), at(&w, 1));
        assert_eq!(w.parked_at(), None);
        w.switch_autonomy(AutonomyMode::Autonomous, occurred())
            .unwrap();
        assert_eq!(w.park(occurred()), Err(CommandError::RefusedUnderAutonomy));
    }

    #[test]
    fn every_command_but_unpark_is_refused_while_parked() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        w.park(occurred()).unwrap();
        let target = at(&w, 2);
        assert_eq!(w.complete_stage(occurred()), Err(CommandError::NotRunning));
        assert_eq!(
            w.open_gate(Vec::new(), occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(
            w.approve_gate(None, occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(
            w.reject_gate(None, occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(w.revise_stage(occurred()), Err(CommandError::NotRunning));
        assert_eq!(
            w.skip_stage("x".to_string(), occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(w.jump(target, occurred()), Err(CommandError::NotRunning));
        assert_eq!(w.park(occurred()), Err(CommandError::NotRunning));
        assert_eq!(
            w.recompose(&[target], occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(
            w.switch_autonomy(AutonomyMode::Autonomous, occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(w.stale_report(at(&w, 0)), Err(CommandError::NotRunning));
        w.unpark(occurred()).unwrap();
        assert!(w.accepts_commands());
    }

    #[test]
    fn unpark_is_refused_when_the_marker_is_not_active() {
        let mut w = all_exec(3);
        assert_eq!(w.unpark(occurred()), Err(CommandError::NotRunning));
    }

    #[test]
    fn recompose_flips_only_pending_stages_ahead_of_the_cursor() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        let cursor = w.cursor();
        assert_eq!(
            w.recompose(&[cursor], occurred()),
            Err(CommandError::InvalidTarget(cursor))
        );
        let event = w.recompose(&[at(&w, 2), at(&w, 3)], occurred()).unwrap();
        let IntentExecutionEvent::Recomposed(recomposed) = &event else {
            panic!("expected Recomposed");
        };
        assert_eq!(recomposed.skipped(), [slug(2), slug(3)]);
        assert!(recomposed.added().is_empty());
        assert_eq!(recomposed.stages_in_scope(), [slug(0), slug(1)]);
        assert_eq!(w.effective_plan(at(&w, 2)), Some(Skip));
        assert_eq!(w.effective_plan(at(&w, 3)), Some(Skip));
        // plan (静的グリッド) は不変 — オーバレイだけが動く。
        assert_eq!(w.stages()[2].plan_action(), Execute);
        w.switch_autonomy(AutonomyMode::Autonomous, occurred())
            .unwrap();
        assert_eq!(
            w.recompose(&[at(&w, 2)], occurred()),
            Err(CommandError::RefusedUnderAutonomy)
        );
    }

    #[test]
    fn recompose_rejects_the_whole_set_when_one_target_is_invalid() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        let cursor = w.cursor();
        assert_eq!(
            w.recompose(&[at(&w, 2), cursor], occurred()),
            Err(CommandError::InvalidTarget(cursor))
        );
        // 部分適用しない (BR1.8)。
        assert_eq!(w.effective_plan(at(&w, 2)), Some(Execute));
        assert_eq!(
            w.recompose(&[], occurred()),
            Err(CommandError::InvalidTarget(cursor))
        );
    }

    #[test]
    fn set_autonomy_replaces_the_mode() {
        let mut w = all_exec(3);
        let event = w
            .switch_autonomy(AutonomyMode::Autonomous, occurred())
            .unwrap();
        let IntentExecutionEvent::AutonomyModeSet(set) = &event else {
            panic!("expected AutonomyModeSet");
        };
        assert_eq!(set.mode(), AutonomyMode::Autonomous);
        assert_eq!(w.autonomy(), AutonomyMode::Autonomous);
    }

    #[test]
    fn a_refused_command_leaves_the_state_and_the_sequence_untouched() {
        let mut w = all_exec(3);
        let before = w.clone();
        assert!(w.revise_stage(occurred()).is_err());
        assert_eq!(w, before);
        assert_eq!(w.seq_nr(), before.seq_nr());
    }

    #[test]
    fn a_completed_workflow_refuses_every_command() {
        let mut w = all_exec(2);
        w.complete_stage(occurred()).unwrap();
        w.approve_gate(None, occurred()).unwrap();
        assert_eq!(w.status(), Status::Completed);
        assert!(!w.accepts_commands());
        assert_eq!(w.complete_stage(occurred()), Err(CommandError::NotRunning));
    }

    // ---- BR1.9: stale_report ----

    #[test]
    fn stale_rereport_yields_done_and_commits_nothing() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let before = w.clone();
        assert_eq!(w.stale_report(at(&w, 0)), Ok(NextDecision::Done));
        assert_eq!(w, before);
        let cursor = at(&w, 1);
        assert_eq!(w.stale_report(cursor), Err(CommandError::NotStale(cursor)));
    }

    // ---- W3: apply_event (BR2.1) ----

    #[test]
    #[should_panic(expected = "apply_event: sequence gap (expected 2, actual 9)")]
    fn apply_event_crashes_on_a_sequence_gap() {
        // 通番の飛びは壊れた歴史 — 再構成は失敗を返さずクラッシュする (オーナー裁定 2026-08-30)。
        let mut w = all_exec(3);
        let event =
            IntentExecutionEvent::StageCompleted(StageCompleted::new(slug(0), Some(slug(1))));
        w.apply_event(9, occurred(), &event);
    }

    #[test]
    #[should_panic(expected = "apply_event: sequence exhausted")]
    fn apply_event_crashes_at_sequence_exhaustion() {
        // 通番を末端に据える (実運用では到達しない規模の境界)。壊れた歴史はクラッシュが正
        // (オーナー裁定 2026-08-30)。
        let base = all_exec(3);
        let mut w = Run {
            intent: base.intent,
            execution: base.execution.with_seq_nr(usize::MAX),
        };
        let event =
            IntentExecutionEvent::StageCompleted(StageCompleted::new(slug(0), Some(slug(1))));
        w.apply_event(1, occurred(), &event);
    }

    #[test]
    fn a_command_at_sequence_exhaustion_is_refused() {
        let base = all_exec(3);
        let mut w = Run {
            intent: base.intent,
            execution: base.execution.with_seq_nr(usize::MAX),
        };
        assert_eq!(
            w.complete_stage(occurred()),
            Err(CommandError::SequenceExhausted)
        );
        assert_eq!(w.seq_nr(), usize::MAX, "状態は変わらない");
    }

    #[test]
    #[should_panic(expected = "apply_event: corrupted history")]
    fn apply_event_crashes_on_an_unknown_stage() {
        let mut w = all_exec(3);
        let unknown = StageSlug::parse("no-such-stage").unwrap();
        let event = IntentExecutionEvent::StageCompleted(StageCompleted::new(unknown, None));
        w.apply_event(2, occurred(), &event);
    }

    #[test]
    #[should_panic(expected = "apply_event: invariant violated")]
    fn apply_event_crashes_on_an_event_that_breaks_an_invariant() {
        let mut w = all_exec(3);
        // ゲート付きステージを承認なしで completed にすると no_gate_bypass が破れる。
        let event =
            IntentExecutionEvent::StageCompleted(StageCompleted::new(slug(1), Some(slug(2))));
        w.apply_event(2, occurred(), &event);
    }

    #[test]
    #[should_panic(expected = "Started applies only at genesis")]
    fn apply_event_crashes_on_a_started_outside_genesis() {
        let mut w = all_exec(3);
        let event = IntentExecutionEvent::Started(Started::new(Intent::from(Created::new(
            intent_id(),
            def_id("claude"),
            revision('0'),
            StartRequest::new("classic", "again"),
            entries(1, &[Execute], &[false]),
            scan(),
        ))));
        w.apply_event(2, occurred(), &event);
    }

    #[test]
    fn a_command_equals_the_old_state_plus_its_event() {
        let mut w = all_exec(4);
        let before = w.clone();
        let event = w.complete_stage(occurred()).unwrap();
        let mut replayed = before;
        replayed.apply_event(w.seq_nr(), *w.last_updated_at(), &event);
        assert_eq!(replayed, w);
    }

    #[test]
    fn a_gate_approval_derives_the_boundary_it_crosses_from_its_own_plan() {
        // ideation の最終ステージを承認すると、次の実効 EXECUTE は inception なので境界が立つ。
        let mut w = phased(&[
            PhaseId::Initialization,
            PhaseId::Ideation,
            PhaseId::Inception,
        ]);
        w.complete_stage(occurred()).unwrap();
        assert_eq!(
            approval_boundary(&mut w),
            Some(PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception))
        );
    }

    #[test]
    fn a_gate_approval_inside_one_phase_derives_no_boundary() {
        // 次の実効 EXECUTE が同じフェーズなら境界は立たない。
        let mut w = phased(&[
            PhaseId::Initialization,
            PhaseId::Ideation,
            PhaseId::Ideation,
        ]);
        w.complete_stage(occurred()).unwrap();
        assert_eq!(approval_boundary(&mut w), None);
    }

    #[test]
    fn approving_the_last_stage_in_scope_derives_no_boundary() {
        // 次が無い (= 最終) なら境界は立たない。
        let mut w = phased(&[PhaseId::Initialization, PhaseId::Ideation]);
        w.complete_stage(occurred()).unwrap();
        assert_eq!(approval_boundary(&mut w), None);
    }

    #[test]
    fn the_boundary_skips_over_stages_that_are_not_in_scope() {
        // 導出は `next_in_scope_slug` と同じ走査なので、実効 SKIP のステージは跨いで数える。
        let mut w = start_from_phased_plan(&[
            (PhaseId::Initialization, Execute),
            (PhaseId::Ideation, Execute),
            (PhaseId::Ideation, Skip),
            (PhaseId::Inception, Execute),
        ]);
        w.complete_stage(occurred()).unwrap();
        assert_eq!(
            approval_boundary(&mut w),
            Some(PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception))
        );
    }

    #[test]
    #[should_panic(expected = "apply_event: invariant violated — parked_position")]
    fn applying_a_park_away_from_the_cursor_crashes() {
        // park の位置はカーソルと同じでなければならない (parked_position)。カーソル 0 のまま
        // ステージ 1 を park するイベントは壊れた歴史である。
        let mut w = all_exec(3);
        let event = IntentExecutionEvent::Parked(Parked::new(slug(1)));
        w.apply_event(2, occurred(), &event);
    }

    #[test]
    #[should_panic(expected = "apply_event: invariant violated — cursor_in_scope")]
    fn applying_a_recompose_that_skips_the_cursor_crashes() {
        // park していない実行のカーソルは実効 EXECUTE の上に居なければならない
        // (cursor_in_scope)。カーソル上のステージを SKIP へ畳む差分は壊れた歴史である。
        let mut w = all_exec(3);
        let event = IntentExecutionEvent::Recomposed(Recomposed::new(
            vec![slug(0)],
            Vec::new(),
            vec![slug(1), slug(2)],
        ));
        w.apply_event(2, occurred(), &event);
    }

    #[test]
    #[should_panic(expected = "apply_event: invariant violated — at_most_one_active")]
    fn applying_a_jump_that_leaves_two_active_stages_crashes() {
        // 出発点を reset も skip もしない jump は出発点の InProgress を残したまま到達先も
        // InProgress にする — active が 2 つになる歴史は壊れている (at_most_one_active)。
        let mut w = all_exec(3);
        let event = IntentExecutionEvent::Jumped(Jumped::new(
            JumpDirection::Forward,
            slug(0),
            slug(1),
            Vec::new(),
            Vec::new(),
        ));
        w.apply_event(2, occurred(), &event);
    }

    // ---- W3: replay (スナップショット + 差分再生 — BR2.3 / BR1.1、本家同型) ----

    #[test]
    fn a_delta_replay_from_a_snapshot_reproduces_the_command_built_state() {
        // コマンドで進めた状態と「途中のスナップショット + 以降のイベント差分再生」が一致
        // する — 本家 example (`UserAccount::replay(events, snapshot)`) と同型で、集約は
        // 添字帳を自分で持つため外部材料なしで再生できる (オーナー裁定 2026-08-30)。
        let mut w = all_exec(4);
        let snapshot = w.execution.clone(); // genesis 時点の写し (= ある時点の集約)
        let mut delta = Vec::new();
        let event = w.complete_stage(occurred()).unwrap();
        delta.push((w.seq_nr(), *w.last_updated_at(), event));
        let event = w.open_gate(Vec::new(), occurred()).unwrap();
        delta.push((w.seq_nr(), *w.last_updated_at(), event));
        let event = w.reject_gate(None, occurred()).unwrap();
        delta.push((w.seq_nr(), *w.last_updated_at(), event));

        let replayed = IntentExecution::replay(snapshot, delta);
        assert_eq!(replayed, w.execution);
    }

    #[test]
    fn a_replay_with_no_delta_returns_the_snapshot_as_is() {
        // スナップショットが最新なら差分は空 — そのまま返る。
        let w = all_exec(3);
        let snapshot = w.execution.clone();
        assert_eq!(IntentExecution::replay(snapshot, Vec::new()), w.execution);
    }

    #[test]
    fn the_full_constructor_round_trips_the_aggregate_state() {
        // 集約 → 列の材料 → 完全コンストラクタが同じ状態へ戻る (検査付きの唯一の口 — BR1.5)。
        let mut w = all_exec(3);
        let _ = w.complete_stage(occurred()).unwrap();
        let source = &w.execution;
        let rebuilt = IntentExecution::new(
            source.id().clone(),
            source.intent_id().clone(),
            source.stage_keys().to_vec(),
            (0..source.stage_count())
                .filter_map(|value| source.stage_index(value))
                .filter_map(|stage| source.effective_plan(stage))
                .collect(),
            (0..source.stage_count())
                .filter_map(|value| source.stage_index(value))
                .filter_map(|stage| source.checkbox(stage))
                .collect(),
            source.cursor().to_usize(),
            source.status(),
            source.parked_at().map(StageIndex::to_usize),
            source.autonomy(),
            (0..source.stage_count())
                .filter_map(|value| source.stage_index(value))
                .filter_map(|stage| source.approved(stage))
                .collect(),
            (0..source.stage_count())
                .filter_map(|value| source.stage_index(value))
                .filter_map(|stage| source.revision_count(stage))
                .collect(),
            source.seq_nr(),
            *source.last_updated_at(),
        )
        .unwrap();
        assert_eq!(&rebuilt, source);
    }

    #[test]
    fn the_full_constructor_rejects_broken_rows() {
        let w = all_exec(3);
        let source = &w.execution;
        let keys = source.stage_keys().to_vec();
        let overlay: Vec<PlanAction> = vec![PlanAction::Execute; 3];
        let checkbox = vec![
            CheckboxState::InProgress,
            CheckboxState::Pending,
            CheckboxState::Pending,
        ];
        // 空の計画。
        assert!(
            IntentExecution::new(
                source.id().clone(),
                source.intent_id().clone(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0,
                source.status(),
                None,
                source.autonomy(),
                Vec::new(),
                Vec::new(),
                1,
                *source.last_updated_at(),
            )
            .is_err()
        );
        // ステージ数と食い違う実行時ベクトル。
        assert!(
            IntentExecution::new(
                source.id().clone(),
                source.intent_id().clone(),
                keys.clone(),
                vec![PlanAction::Execute; 2],
                checkbox.clone(),
                0,
                source.status(),
                None,
                source.autonomy(),
                vec![false; 3],
                vec![0; 3],
                1,
                *source.last_updated_at(),
            )
            .is_err()
        );
        // 範囲外の parked 位置。
        assert!(
            IntentExecution::new(
                source.id().clone(),
                source.intent_id().clone(),
                keys.clone(),
                overlay.clone(),
                checkbox.clone(),
                0,
                source.status(),
                Some(9),
                source.autonomy(),
                vec![false; 3],
                vec![0; 3],
                1,
                *source.last_updated_at(),
            )
            .is_err()
        );
        // 範囲外カーソルと通番 0。
        assert!(
            IntentExecution::new(
                source.id().clone(),
                source.intent_id().clone(),
                keys.clone(),
                overlay.clone(),
                checkbox.clone(),
                9,
                source.status(),
                None,
                source.autonomy(),
                vec![false; 3],
                vec![0; 3],
                1,
                *source.last_updated_at(),
            )
            .is_err()
        );
        assert!(
            IntentExecution::new(
                source.id().clone(),
                source.intent_id().clone(),
                keys,
                overlay,
                checkbox,
                0,
                source.status(),
                None,
                source.autonomy(),
                vec![false; 3],
                vec![0; 3],
                0,
                *source.last_updated_at(),
            )
            .is_err()
        );
    }

    #[test]
    fn the_version_the_store_assigned_is_stamped_after_reconstruction() {
        // 版はイベント列から導出できない — ストアが読んだ値を再構成の最後に刻む。
        let mut w = all_exec(3);
        let snapshot = w.execution.clone();
        let fresh = IntentExecution::replay(snapshot, Vec::new()).with_version(7);
        assert_eq!(fresh.version(), 7);
        // genesis は未永続 — `start` を通った集約はまだ版を持たない。
        assert_eq!(w.execution.version(), IntentExecution::UNPERSISTED_VERSION);
        // 版はコマンドで動かない (採番するのはストアである)。
        let _ = w.complete_stage(occurred()).unwrap();
        assert_eq!(w.execution.version(), IntentExecution::UNPERSISTED_VERSION);
    }

    // ---- W4: next_decision (BR3.1 / BR2.6) ----

    #[test]
    fn next_decision_refuses_a_different_definition() {
        let w = all_exec(3);
        let other = bare_definition("kiro");
        assert_eq!(
            w.next_decision(&other, NextRequest::default()),
            Err(CommandError::DefinitionMismatch {
                expected: def_id("claude"),
                actual: def_id("kiro"),
            })
        );
    }

    #[test]
    fn a_newer_revision_of_the_same_definition_is_accepted() {
        let w = all_exec(3);
        let drifted = WorkflowDefinition::from_artifacts(
            def_id("claude"),
            revision('f'),
            StageGraph::new(Vec::new()).unwrap(),
            ScopeGrid::new(BTreeMap::new()),
            BTreeMap::new(),
        );
        assert!(w.next_decision(&drifted, NextRequest::default()).is_ok());
    }

    #[test]
    fn next_decision_walks_the_branches_in_priority_order() {
        let definition = bare_definition("claude");
        let mut w = all_exec(3);

        // (6) cursor が in-flight
        assert_eq!(
            w.next_decision(&definition, NextRequest::default()),
            Ok(NextDecision::RunStage {
                stage: at(&w, 0),
                gate: false
            })
        );
        // (1) park 中
        w.park(occurred()).unwrap();
        assert_eq!(
            w.next_decision(&definition, NextRequest::default()),
            Ok(NextDecision::Parked { stage: at(&w, 0) })
        );
        assert_eq!(
            w.next_decision(&definition, NextRequest::new(true, false, false)),
            Ok(NextDecision::UnparkThenResume)
        );
        // 再入フラグは park ガードを外す
        assert_eq!(
            w.next_decision(&definition, NextRequest::new(false, true, false)),
            Ok(NextDecision::RunStage {
                stage: at(&w, 0),
                gate: false
            })
        );
        w.unpark(occurred()).unwrap();
        // (2) resume
        assert_eq!(
            w.next_decision(&definition, NextRequest::new(true, false, false)),
            Ok(NextDecision::ResumeMenu)
        );
        // (3) 自由記述
        assert_eq!(
            w.next_decision(&definition, NextRequest::new(false, false, true)),
            Ok(NextDecision::NewWorkRouting)
        );
        // (7) 次の in-scope / gate = true
        w.complete_stage(occurred()).unwrap();
        assert_eq!(
            w.next_decision(&definition, NextRequest::default()),
            Ok(NextDecision::RunStage {
                stage: at(&w, 1),
                gate: true
            })
        );
        // (4) completed
        w.approve_gate(None, occurred()).unwrap();
        w.approve_gate(None, occurred()).unwrap();
        assert_eq!(w.status(), Status::Completed);
        assert_eq!(
            w.next_decision(&definition, NextRequest::default()),
            Ok(NextDecision::Done)
        );
    }

    #[test]
    fn next_decision_reports_the_two_skip_inconsistencies() {
        // 実効 SKIP のカーソルは `cursor_in_scope` が禁じるので、集約のコマンド経由では作れない。
        // 到達しうるのは「park 中 (受理述語が偽なので cursor_in_scope を検査しない) に
        // recompose のイベントが畳まれた歴史」の全再生である (BR3.1 (5) の防御腕)。
        // Pending は再生でも作れない (advance / jump が到達先を必ず InProgress にする) ため、
        // 到達可能な 3 マーカーで両分岐 (回復可能 / 不能) を固定する。
        let definition = bare_definition("claude");
        let base = all_exec(3);
        let reentry = NextRequest::new(false, true, false);
        for (marker, mid_events, expected_recoverable) in [
            (
                InProgress,
                vec![IntentExecutionEvent::StageCompleted(StageCompleted::new(
                    slug(0),
                    Some(slug(1)),
                ))],
                true,
            ),
            (
                Revising,
                vec![
                    IntentExecutionEvent::StageCompleted(StageCompleted::new(
                        slug(0),
                        Some(slug(1)),
                    )),
                    IntentExecutionEvent::GateOpened(GateOpened::new(slug(1), Vec::new())),
                    IntentExecutionEvent::GateRejected(GateRejected::new(slug(1), None, 1)),
                ],
                true,
            ),
            (
                AwaitingApproval,
                vec![
                    IntentExecutionEvent::StageCompleted(StageCompleted::new(
                        slug(0),
                        Some(slug(1)),
                    )),
                    IntentExecutionEvent::GateOpened(GateOpened::new(slug(1), Vec::new())),
                ],
                false,
            ),
        ] {
            // genesis のスナップショット (seq 1) を基底に、以降のイベントを差分再生する。
            let mut delta = Vec::new();
            for event in mid_events {
                delta.push((delta.len() + 2, occurred(), event));
            }
            delta.push((
                delta.len() + 2,
                occurred(),
                IntentExecutionEvent::Parked(Parked::new(slug(1))),
            ));
            delta.push((
                delta.len() + 2,
                occurred(),
                IntentExecutionEvent::Recomposed(Recomposed::new(
                    vec![slug(1)],
                    Vec::new(),
                    vec![slug(0), slug(2)],
                )),
            ));
            let execution = IntentExecution::replay(base.execution.clone(), delta);
            let w = Run {
                intent: base.intent.clone(),
                execution,
            };
            assert_eq!(w.checkbox(at(&w, 1)), Some(marker));
            let stage = at(&w, 1);
            let expected = if expected_recoverable {
                NextDecision::RecoverSkipInconsistency {
                    stage,
                    checkbox: marker,
                }
            } else {
                NextDecision::InconsistentSkip {
                    stage,
                    checkbox: marker,
                }
            };
            assert_eq!(w.next_decision(&definition, reentry), Ok(expected));
            // どちらの不整合も Quint の DError に写る (BR3.1)。
            assert_eq!(
                EngineSignal::from(&w.next_decision(&definition, reentry).unwrap()),
                EngineSignal::EngineError
            );
        }
    }

    #[test]
    fn jump_resolve_is_a_read_only_query() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        let before = w.clone();
        assert_eq!(w.jump_resolve(at(&w, 3)), Ok(JumpDirection::Forward));
        assert_eq!(w, before);
        let out_of_scope = at(&w, 2);
        w.recompose(&[out_of_scope], occurred()).unwrap();
        assert_eq!(
            w.jump_resolve(out_of_scope),
            Err(CommandError::InvalidTarget(out_of_scope))
        );
    }

    // ---- 実グラフの索引 (NFR1.2) ----

    #[test]
    fn every_initialization_stage_is_non_gated_and_the_rest_are_gated() {
        let mut w = start_with(3, &[Execute; 6], &[false; 6]);
        for i in 0..3 {
            assert_eq!(w.gated(at(&w, i)), Some(false), "stage {i}");
        }
        for i in 3..6 {
            assert_eq!(w.gated(at(&w, i)), Some(true), "stage {i}");
        }
        // 索引 0〜2 は complete_stage で進み、open_gate は拒否される。
        for i in 0..3 {
            let cursor = at(&w, i);
            assert_eq!(w.cursor(), cursor);
            assert_eq!(
                w.open_gate(Vec::new(), occurred()),
                Err(CommandError::InvalidTarget(cursor))
            );
            w.complete_stage(occurred()).unwrap();
            assert_eq!(w.approved(cursor), Some(false));
        }
        // 索引 3 以降はゲート — complete_stage は拒否される。
        let cursor = at(&w, 3);
        assert_eq!(w.cursor(), cursor);
        assert_eq!(
            w.complete_stage(occurred()),
            Err(CommandError::InvalidTarget(cursor))
        );
        let init_target = at(&w, 1);
        assert_eq!(
            w.jump(init_target, occurred()),
            Err(CommandError::InvalidTarget(init_target))
        );
    }

    #[test]
    fn stage_index_is_only_constructed_within_range() {
        let w = all_exec(3);
        assert_eq!(w.stage_index(2).map(StageIndex::to_usize), Some(2));
        assert_eq!(w.stage_index(3), None);
        assert_eq!(w.stage_index(usize::MAX), None);
    }

    #[test]
    fn queries_about_a_foreign_stage_index_answer_none_instead_of_panicking() {
        let wide = all_exec(5);
        let narrow = all_exec(2);
        let foreign = at(&wide, 4);
        assert_eq!(narrow.checkbox(foreign), None);
        assert_eq!(narrow.approved(foreign), None);
        assert_eq!(narrow.effective_plan(foreign), None);
        assert_eq!(narrow.gated(foreign), None);
        assert_eq!(narrow.revision_count(foreign), None);
    }

    #[test]
    fn the_signal_projection_of_a_decision_matches_the_model_vocabulary() {
        let definition = bare_definition("claude");
        let w = all_exec(3);
        let decision = w
            .next_decision(&definition, NextRequest::default())
            .unwrap();
        assert_eq!(
            EngineSignal::from(&decision),
            EngineSignal::RunStage(at(&w, 0))
        );
    }

    // ---- PBT (NFR2.2): 6 性質 + 定義側から移設した 2 性質 ----
    //
    // 生成器は合成定義 (stage_count 2〜8、initialization 1〜3 ステージ) とコマンド列 (≤ 60)。
    // シードは `PROPTEST_RNG_SEED` で固定する (scripts/coverage.sh / CI と同値)。

    use proptest::prelude::*;

    #[derive(Debug, Clone)]
    enum Cmd {
        Complete,
        OpenGate,
        Approve,
        Reject,
        Revise,
        SkipStage,
        Jump(usize),
        Park,
        Unpark,
        Recompose(usize),
        SetAutonomy(bool),
        Next,
        Stale(usize),
    }

    fn cmd_strategy(n: usize) -> impl Strategy<Value = Cmd> {
        prop_oneof![
            Just(Cmd::Complete),
            Just(Cmd::OpenGate),
            Just(Cmd::Approve),
            Just(Cmd::Reject),
            Just(Cmd::Revise),
            Just(Cmd::SkipStage),
            (0..n).prop_map(Cmd::Jump),
            Just(Cmd::Park),
            Just(Cmd::Unpark),
            (0..n).prop_map(Cmd::Recompose),
            any::<bool>().prop_map(Cmd::SetAutonomy),
            Just(Cmd::Next),
            (0..n).prop_map(Cmd::Stale),
        ]
    }

    /// 合成計画 — 索引 0..init が initialization (常に EXECUTE・非 CONDITIONAL)、残りは inception。
    fn synthetic_stages() -> impl Strategy<Value = Vec<StageEntry>> {
        (2usize..=8)
            .prop_flat_map(|count| {
                let init_max = if count < 3 { count } else { 3 };
                (Just(count), 1usize..=init_max)
            })
            .prop_flat_map(|(count, init)| {
                (
                    Just(count),
                    Just(init),
                    proptest::collection::vec(any::<bool>(), count),
                    proptest::collection::vec(any::<bool>(), count),
                )
            })
            .prop_map(|(count, init, exec_bits, cond_bits)| {
                (0..count)
                    .map(|index| {
                        let initialization = index < init;
                        let phase = if initialization {
                            PhaseId::Initialization
                        } else {
                            PhaseId::Inception
                        };
                        let execute =
                            initialization || exec_bits.get(index).copied().unwrap_or(true);
                        let conditional =
                            !initialization && cond_bits.get(index).copied().unwrap_or(false);
                        StageEntry::new(
                            slug(index),
                            phase,
                            if execute { Execute } else { Skip },
                            conditional,
                            display(&format!("{}.{}", phase.index(), index + 1)),
                        )
                    })
                    .collect()
            })
    }

    fn start_synthetic(stages: Vec<StageEntry>) -> Run {
        Run::start(Intent::from(Created::new(
            intent_id(),
            def_id("claude"),
            revision('0'),
            start_request(),
            stages,
            scan(),
        )))
    }

    /// 1 コマンドを駆動する。`Err` は「発火しないアクション」なので状態は一切動かない (BR1.1 (e))。
    fn drive(
        w: &mut Run,
        definition: &WorkflowDefinition,
        cmd: &Cmd,
    ) -> Option<IntentExecutionEvent> {
        let before = w.clone();
        let outcome = match cmd {
            Cmd::Complete => w.complete_stage(occurred()),
            Cmd::OpenGate => w.open_gate(Vec::new(), occurred()),
            Cmd::Approve => w.approve_gate(None, occurred()),
            Cmd::Reject => w.reject_gate(None, occurred()),
            Cmd::Revise => w.revise_stage(occurred()),
            Cmd::SkipStage => w.skip_stage("pbt".to_string(), occurred()),
            Cmd::Jump(target) => match w.stage_index(*target) {
                Some(stage) => w.jump(stage, occurred()),
                None => Err(CommandError::NotRunning),
            },
            Cmd::Park => w.park(occurred()),
            Cmd::Unpark => w.unpark(occurred()),
            Cmd::Recompose(target) => match w.stage_index(*target) {
                Some(stage) => w.recompose(&[stage], occurred()),
                None => Err(CommandError::NotRunning),
            },
            Cmd::SetAutonomy(autonomous) => w.switch_autonomy(
                if *autonomous {
                    AutonomyMode::Autonomous
                } else {
                    AutonomyMode::Gated
                },
                occurred(),
            ),
            Cmd::Next => {
                let _ = w.next_decision(definition, NextRequest::default());
                assert_eq!(*w, before, "next_decision は書き込まない");
                return None;
            }
            Cmd::Stale(target) => {
                if let Some(stage) = w.stage_index(*target) {
                    let _ = w.stale_report(stage);
                }
                assert_eq!(*w, before, "stale_report は書き込まない");
                return None;
            }
        };
        match outcome {
            Ok(event) => Some(event),
            Err(_) => {
                assert_eq!(*w, before, "Err は状態を変えない (BR1.1)");
                None
            }
        }
    }

    fn assert_quint_invariants(w: &Run) {
        let count = w.stage_count();
        // cursor_in_scope: コマンドを受理できる間、カーソルは実効 EXECUTE 上にある。
        if w.accepts_commands() {
            assert_eq!(
                w.effective_plan(w.cursor()),
                Some(Execute),
                "cursor_in_scope"
            );
        }
        let mut active = 0_usize;
        for value in 0..count {
            let stage = w.stage_index(value).unwrap();
            let marker = w.checkbox(stage).unwrap();
            if marker.is_active() {
                active += 1;
            }
            // no_gate_bypass: ゲート付きステージの completed は必ず承認履歴を伴う。
            if w.gated(stage) == Some(true) && marker == Completed {
                assert_eq!(w.approved(stage), Some(true), "no_gate_bypass at {value}");
            }
        }
        assert!(active <= 1, "at_most_one_active: {active}");
        // parked_position: park マーカーが活性ならカーソル位置と一致する。
        if w.parked_active() {
            assert_eq!(w.parked_at(), Some(w.cursor()), "parked_position");
        }
    }

    proptest! {
        /// (a) decide 後の状態 == 旧状態 + apply_event、(d) Quint 不変条件、(e) Err 無副作用、
        /// (f) from_state(state()) == self を全ステップで固定する。
        #[test]
        fn every_command_equals_the_old_state_plus_its_event(
            stages in synthetic_stages(),
            cmds in proptest::collection::vec(cmd_strategy(8), 1..60),
        ) {
            let definition = bare_definition("claude");
            let mut w = start_synthetic(stages);
            assert_quint_invariants(&w);
            for cmd in &cmds {
                let before = w.clone();
                if let Some(event) = drive(&mut w, &definition, cmd) {
                    let mut replayed = before;
                    replayed.apply_event(w.seq_nr(), *w.last_updated_at(), &event);
                    prop_assert_eq!(&replayed.execution, &w.execution);
                }
                assert_quint_invariants(&w);
            }
        }

        /// (b) リプレイの決定性 — genesis スナップショット + 以降のイベント差分再生 == 通常
        /// 実行 (BR2.3、本家同型)。(c) seq_nr は 1 イベントにつき 1 だけ増える (BR2.1 —
        /// 順序違反のクラッシュはユニットテスト `apply_event_crashes_on_a_sequence_gap` が
        /// 固定する)。
        #[test]
        fn replaying_the_event_stream_reproduces_the_executed_aggregate(
            stages in synthetic_stages(),
            cmds in proptest::collection::vec(cmd_strategy(8), 1..60),
        ) {
            let definition = bare_definition("claude");
            let mut w = start_synthetic(stages);
            let snapshot = w.execution.clone();
            // 封筒の材料 (通番・発生時刻) は commit を通った集約から採る (B7 — Repository も同じ)。
            let mut delta: Vec<(usize, DateTime<Utc>, IntentExecutionEvent)> = Vec::new();
            let mut expected_seq = w.seq_nr();
            for cmd in &cmds {
                if let Some(event) = drive(&mut w, &definition, cmd) {
                    expected_seq += 1;
                    prop_assert_eq!(w.seq_nr(), expected_seq);
                    delta.push((w.seq_nr(), *w.last_updated_at(), event));
                }
            }

            let replayed = IntentExecution::replay(snapshot, delta);
            prop_assert_eq!(&replayed, &w.execution);
        }

        /// 定義側から移設した性質 (1): 実効プランはグリッドに recompose のサフィックスを重ねた値で
        /// あり、静的な `plan` は決して動かない (BR4.2)。
        #[test]
        fn the_recompose_suffix_beats_the_grid_and_the_static_plan_never_moves(
            stages in synthetic_stages(),
            cmds in proptest::collection::vec(cmd_strategy(8), 1..60),
        ) {
            let definition = bare_definition("claude");
            let grid: Vec<PlanAction> = stages.iter().map(StageEntry::plan_action).collect();
            let mut w = start_synthetic(stages);
            let mut expected = grid.clone();
            for cmd in &cmds {
                if let Some(event) = drive(&mut w, &definition, cmd)
                    && let IntentExecutionEvent::Recomposed(recomposed) = &event
                {
                    for slug in recomposed.skipped() {
                        let index = w.stages().iter().position(|e| e.slug() == slug).unwrap();
                        expected[index] = Skip;
                    }
                    for slug in recomposed.added() {
                        let index = w.stages().iter().position(|e| e.slug() == slug).unwrap();
                        expected[index] = Execute;
                    }
                }
                for value in 0..w.stage_count() {
                    let stage = w.stage_index(value).unwrap();
                    prop_assert_eq!(w.effective_plan(stage), Some(expected[value]));
                    prop_assert_eq!(w.stages()[value].plan_action(), grid[value]);
                }
            }
        }

        /// 定義側から移設した性質 (2): `next_decision` が名指しする先読みステージは、カーソルより
        /// 後ろで**最初**の in-scope ステージである (読み飛ばしの最小性)。無ければ `Done`。
        #[test]
        fn the_lookahead_target_is_the_first_in_scope_stage_in_document_order(
            stages in synthetic_stages(),
            cmds in proptest::collection::vec(cmd_strategy(8), 1..60),
        ) {
            let definition = bare_definition("claude");
            let mut w = start_synthetic(stages);
            for cmd in &cmds {
                drive(&mut w, &definition, cmd);
                let Ok(decision) = w.next_decision(&definition, NextRequest::default()) else {
                    continue;
                };
                let cursor = w.cursor().to_usize();
                let cursor_in_flight = w
                    .checkbox(w.cursor())
                    .is_some_and(CheckboxState::is_in_flight);
                match decision {
                    NextDecision::RunStage { stage, gate } if stage.to_usize() != cursor => {
                        prop_assert!(stage.to_usize() > cursor);
                        for value in (cursor + 1)..stage.to_usize() {
                            let earlier = w.stage_index(value).unwrap();
                            prop_assert_ne!(w.effective_plan(earlier), Some(Execute));
                        }
                        prop_assert_eq!(w.effective_plan(stage), Some(Execute));
                        prop_assert_eq!(gate, w.gated(stage).unwrap());
                    }
                    NextDecision::Done if w.accepts_commands() && !cursor_in_flight => {
                        for value in (cursor + 1)..w.stage_count() {
                            let later = w.stage_index(value).unwrap();
                            prop_assert_ne!(w.effective_plan(later), Some(Execute));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
