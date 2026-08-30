//! `IntentExecutionEvent` — 12 変種のドメインイベント (C5、entities.md)。
//!
//! 変種はコマンドと 1:1 (BR1.1 / BR2.4)。ステージ参照はすべて `StageSlug` で、投影側 (U4) が
//! 索引表を要さない自己記述形になっている。イベントは構築後 immutable で、材料はアクセサで
//! 公開する。
//!
//! # 輸送のメタデータは載せない (ADR-010 / B7)
//!
//! 本家 event-store-adapter-rs v3.0.0 は `Event` trait を廃し、識別子・順序番号・発生時刻・
//! 型判別子を [`EventEnvelope`] が運ぶようになった。したがってドメインイベントは
//! **純粋なドメイン内容だけ**を持つ (本家の語で payload)。かつて自前で持っていた封筒
//! (`id` / `schema_version` / `occurred_at`) と、その識別子型は削除し、封筒を
//! 組むのはアダプタ層 (Repository) の責務にした — 「Payload」は輸送の語であってドメインの語では
//! ないので、この enum 自身がドメインイベントの正体である (ubiquitous-language.md)。
//!
//! **直列化の記述は持たない** (改訂 9 / `coding-rules/domain-persistence-neutrality.md`)。
//! 行のバイトを決めるのは書く側 (command interface-adapter) と読む側 (RMU) の DTO であり、
//! この enum が持つのはドメインの語彙だけである。
//!
//! [`EventEnvelope`]: https://docs.rs/event-store-adapter-rs/3.0.0/event_store_adapter_rs/event_envelope/struct.EventEnvelope.html

use super::autonomy_mode::AutonomyMode;
use super::intent::Intent;
use super::stage_entry::StageEntry;
use super::workspace_scan::WorkspaceScan;
use crate::workflow_definition::{DefinitionRevision, StageSlug, WorkflowDefinitionId};

/// 12 変種のドメインイベント (C5 の 11 + `StageCompleted`)。
///
/// `#[non_exhaustive]` は**付けない** — 変種の追加は C5 の改訂を伴う設計事項であり、消費側の
/// 網羅 match が落ちること自体が検出手段である (NFR1.3)。`Unparked` は C5 が `payload: {}` と
/// するので専用の材料型を持たない単位変種にした。
#[expect(
    clippy::large_enum_variant,
    reason = "イベント痩身 (issue #56) で他変種は事実のみになったが、Started はまだ Intent の \
              複製を運ぶ — 計画の正本供給を Intent 自身のジャーナルへ移す issue #50 が前提の \
              残件であり、Box で包んでも意味は変わらない"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentExecutionEvent {
    /// 実行の開始 (解決済み計画を自己完結で持つ — BR2.2。Intent 複製の撤去は #50 待ち)。
    Started(Started),
    /// 非ゲート (initialization フェーズ) ステージの完了。
    StageCompleted(StageCompleted),
    /// 承認ゲートの開放。
    GateOpened(GateOpened),
    /// 承認ゲートの通過。
    GateApproved(GateApproved),
    /// 承認ゲートでの差し戻し。
    GateRejected(GateRejected),
    /// 差し戻し後のゲート再入。
    StageRevised(StageRevised),
    /// ステージの読み飛ばし。
    StageSkipped(StageSkipped),
    /// カーソルの移動 (forward / backward / redo)。
    Jumped(Jumped),
    /// park マーカーの設置。
    Parked(Parked),
    /// park マーカーの除去 (位置は `parked_at` から復元されるので材料なし)。
    Unparked,
    /// 実効プランの再形成 (オーバレイの反転)。
    Recomposed(Recomposed),
    /// 自律モードの設定。
    AutonomyModeSet(AutonomyModeSet),
}

/// `Started` のペイロード — リプレイが `WorkflowDefinition` を要さない自己完結データ (BR2.2)。
///
/// 開始時点の [`Intent`] を**丸ごと運ぶ**。`Intent` 自身も集約だが (改訂 8)、これは集約への
/// 埋め込みではなく**歴史の記録**であり、規則違反ではない
/// (coding-rules/aggregate-references.md「イベントに材料の複製が載るのは違反ではない」)。
/// `IntentExecution` が**保持する**のは `intent_id` だけで、そこは ID 参照のままである。投影核の入力はイベントだけ (cqrs-boundaries 規則 3) なので、
/// 状態ファイルを描くのに要る scope・依頼文・解決済み計画・走査結果はここに載っている必要が
/// ある。集約が適用時に**保持する**のは `intent_id` と実行時状態だけである。
///
/// 各アクセサは intent への素通しである。`depth` / `test_strategy` は集約状態にならず、
/// U4 が `Scope Configuration` を描くためだけの投影材料である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    intent: Intent,
}

impl Started {
    /// 開始時点の intent を束ねる。
    #[must_use]
    pub const fn new(intent: Intent) -> Started {
        Started { intent }
    }

    /// 開始時点の intent そのもの。
    #[must_use]
    pub const fn intent(&self) -> &Intent {
        &self.intent
    }

    /// 参照した定義の系譜 ID (BR2.6)。
    #[must_use]
    pub const fn definition_id(&self) -> &WorkflowDefinitionId {
        self.intent.definition_id()
    }

    /// 参照した定義の内容版 (来歴 — 差が出ても Err にはしない)。
    #[must_use]
    pub const fn definition_revision(&self) -> &DefinitionRevision {
        self.intent.definition_revision()
    }

    /// 選択されたスコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        self.intent.scope()
    }

    /// 人間の要求 (逐語保持)。
    #[must_use]
    pub fn request(&self) -> &str {
        self.intent.request()
    }

    /// 呼出側が解決した depth (`None` = 指定なし)。集約は素通しするだけ。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.intent.depth()
    }

    /// 呼出側が解決した test strategy (`None` = 指定なし)。集約は素通しするだけ。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.intent.test_strategy()
    }

    /// 文書順の全ステージ (解決済み計画)。
    #[must_use]
    pub fn stages(&self) -> &[StageEntry] {
        self.intent.stages()
    }

    /// workspace-detection が出した走査結果 (投影が初期化 3 ステージの行を描く材料)。
    ///
    /// イベントが運ぶのは、走査をやり直すと**当時と違う結果**になり再構成が一致しないため
    /// である (NFR3 — オーナー裁定 2026-08-29)。
    #[must_use]
    pub const fn scan(&self) -> &WorkspaceScan {
        self.intent.scan()
    }
}

/// `StageCompleted` のペイロード — 起きた事実 (どのステージが完了したか) だけを運ぶ。
///
/// 次カーソルは載せない — 導出された状態であり、適用側 (集約) とリードモデル側 (RMU) が
/// それぞれ自分の状態から導く (オーナー裁定 2026-08-30「イベントに状態は含めるな」)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageCompleted {
    stage: StageSlug,
}

impl StageCompleted {
    /// 完了したステージ。
    #[must_use]
    pub const fn new(stage: StageSlug) -> StageCompleted {
        StageCompleted { stage }
    }

    /// 完了したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}

/// `GateOpened` のペイロード。`artifacts` は呼出側が渡す投影材料 (C5)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateOpened {
    stage: StageSlug,
    artifacts: Vec<String>,
}

impl GateOpened {
    /// ゲートを開いたステージと、レビュー対象の成果物パス列。
    #[must_use]
    pub const fn new(stage: StageSlug, artifacts: Vec<String>) -> GateOpened {
        GateOpened { stage, artifacts }
    }

    /// ゲートを開いたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// レビュー対象の成果物パス列 (集約は検証せず載せるだけ)。
    #[must_use]
    pub fn artifacts(&self) -> &[String] {
        &self.artifacts
    }
}

/// `GateApproved` のペイロード — 事実 (どのゲートが・どの入力で承認されたか) だけを運ぶ。
///
/// 次カーソルとフェーズ境界は載せない — どちらも導出された状態であり、適用側とリードモデル
/// 側が自分の状態から導く (オーナー裁定 2026-08-30)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateApproved {
    stage: StageSlug,
    user_input: Option<String>,
}

impl GateApproved {
    /// 承認されたステージと、承認時の人間入力。
    #[must_use]
    pub const fn new(stage: StageSlug, user_input: Option<String>) -> GateApproved {
        GateApproved { stage, user_input }
    }

    /// 承認されたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 承認時の人間入力 (逐語保持)。
    #[must_use]
    pub fn user_input(&self) -> Option<&str> {
        self.user_input.as_deref()
    }
}

/// `GateRejected` のペイロード — 事実 (どのゲートが・どの理由で差し戻されたか) だけを運ぶ。
///
/// 改訂回数は載せない — 適用後の値 = 状態である。集約は自分のカウンタを +1 し、RMU は
/// リードモデルの `Revision Count` を read-modify-write する (upstream `aidlc-state.ts`
/// 自身が getField + 1 で書いており、この導出が正本互換 — オーナー裁定 2026-08-30)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRejected {
    stage: StageSlug,
    feedback: Option<String>,
}

impl GateRejected {
    /// 差し戻したステージと、差し戻し理由。
    #[must_use]
    pub const fn new(stage: StageSlug, feedback: Option<String>) -> GateRejected {
        GateRejected { stage, feedback }
    }

    /// 差し戻したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 差し戻し理由 (逐語保持)。
    #[must_use]
    pub fn feedback(&self) -> Option<&str> {
        self.feedback.as_deref()
    }
}

/// `StageRevised` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRevised {
    stage: StageSlug,
}

impl StageRevised {
    /// ゲートに再入したステージ。
    #[must_use]
    pub const fn new(stage: StageSlug) -> StageRevised {
        StageRevised { stage }
    }

    /// ゲートに再入したステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}

/// `StageSkipped` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSkipped {
    stage: StageSlug,
    reason: String,
}

impl StageSkipped {
    /// 読み飛ばしたステージと、理由。次カーソルは載せない (導出 — オーナー裁定 2026-08-30)。
    #[must_use]
    pub const fn new(stage: StageSlug, reason: String) -> StageSkipped {
        StageSkipped { stage, reason }
    }

    /// 読み飛ばしたステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 読み飛ばしの理由 (逐語保持)。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// `Jumped` のペイロード — 事実 (どこへ跳んだか) だけを運ぶ。
///
/// 方向・出発点・読み飛ばし列・巻き戻し列は載せない — すべて跳躍規則 (BR1.6) による導出で
/// あり、適用側 (集約) とリードモデル側 (RMU) がそれぞれ自分の状態 (カーソル・checkbox・
/// 実効プラン) から導く (オーナー裁定 2026-08-30「イベントに状態は含めるな」)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jumped {
    target: StageSlug,
}

impl Jumped {
    /// 跳んだ先。
    #[must_use]
    pub const fn new(target: StageSlug) -> Jumped {
        Jumped { target }
    }

    /// 跳んだ先。
    #[must_use]
    pub const fn target(&self) -> &StageSlug {
        &self.target
    }
}

/// `Parked` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parked {
    stage: StageSlug,
}

impl Parked {
    /// park した位置のステージ。
    #[must_use]
    pub const fn new(stage: StageSlug) -> Parked {
        Parked { stage }
    }

    /// park した位置のステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }
}

/// `Recomposed` のペイロード — 事実 (どの反転が起きたか) だけを運ぶ。
///
/// 適用後の in-scope 列は載せない — 適用後の状態であり、適用側とリードモデル側が自分の
/// 実効プランから導く (オーナー裁定 2026-08-30)。1 コマンドの複数反転は 1 イベント (C5)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recomposed {
    skipped: Vec<StageSlug>,
    added: Vec<StageSlug>,
}

impl Recomposed {
    /// EXECUTE → SKIP にした列、SKIP → EXECUTE にした列。
    #[must_use]
    pub const fn new(skipped: Vec<StageSlug>, added: Vec<StageSlug>) -> Recomposed {
        Recomposed { skipped, added }
    }

    /// EXECUTE → SKIP に反転したステージ列。
    #[must_use]
    pub fn skipped(&self) -> &[StageSlug] {
        &self.skipped
    }

    /// SKIP → EXECUTE に反転したステージ列。
    #[must_use]
    pub fn added(&self) -> &[StageSlug] {
        &self.added
    }
}

/// `AutonomyModeSet` のペイロード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyModeSet {
    mode: AutonomyMode,
}

impl AutonomyModeSet {
    /// 設定後のモード。
    #[must_use]
    pub const fn new(mode: AutonomyMode) -> AutonomyModeSet {
        AutonomyModeSet { mode }
    }

    /// 設定後のモード。
    #[must_use]
    pub const fn mode(&self) -> AutonomyMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する (集約のテストモジュールと同じ作法)。
    #![allow(clippy::panic)]

    use super::*;
    use crate::orchestration::{
        AutonomyMode, Created, IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
    };
    use crate::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
        WorkflowDefinitionId,
    };

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    fn display(number: &str) -> StageDisplay {
        StageDisplay::new(StageNumber::parse(number).unwrap(), "Stage", "orchestrator").unwrap()
    }

    fn scan() -> WorkspaceScan {
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .unwrap()
    }

    #[test]
    fn the_started_payload_is_self_contained() {
        let entries = vec![StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            display("0.1"),
        )];
        let started = Started::new(Intent::from(Created::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "build it").with_depth("standard"),
            entries.clone(),
            scan(),
        )));
        assert_eq!(
            started.intent().id().as_str(),
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
        assert_eq!(started.definition_id().as_str(), "claude");
        assert_eq!(started.definition_revision().as_str().len(), 71);
        assert_eq!(started.scope(), "classic");
        assert_eq!(started.request(), "build it");
        assert_eq!(started.depth(), Some("standard"));
        assert_eq!(started.test_strategy(), None);
        assert_eq!(started.stages(), entries.as_slice());
    }

    #[test]
    fn the_stage_lifecycle_payloads_carry_their_slugs_and_material() {
        let completed = StageCompleted::new(slug("state-init"));
        assert_eq!(completed.stage(), &slug("state-init"));

        let opened = GateOpened::new(slug("intent-capture"), vec!["intent.md".to_string()]);
        assert_eq!(opened.stage(), &slug("intent-capture"));
        assert_eq!(opened.artifacts(), ["intent.md".to_string()]);

        let approved = GateApproved::new(slug("intent-capture"), Some("looks good".to_string()));
        assert_eq!(approved.stage(), &slug("intent-capture"));
        assert_eq!(approved.user_input(), Some("looks good"));

        let rejected = GateRejected::new(slug("intent-capture"), None);
        assert_eq!(rejected.stage(), &slug("intent-capture"));
        assert_eq!(rejected.feedback(), None);

        let revised = StageRevised::new(slug("intent-capture"));
        assert_eq!(revised.stage(), &slug("intent-capture"));

        let skipped = StageSkipped::new(slug("market-research"), "out of scope".to_string());
        assert_eq!(skipped.stage(), &slug("market-research"));
        assert_eq!(skipped.reason(), "out of scope");
    }

    #[test]
    fn the_control_payloads_carry_the_jump_park_and_recompose_material() {
        let jumped = Jumped::new(slug("state-init"));
        assert_eq!(jumped.target(), &slug("state-init"));

        let parked = Parked::new(slug("intent-capture"));
        assert_eq!(parked.stage(), &slug("intent-capture"));

        let recomposed = Recomposed::new(vec![slug("market-research")], Vec::new());
        assert_eq!(recomposed.skipped(), [slug("market-research")]);
        assert!(recomposed.added().is_empty());

        let mode = AutonomyModeSet::new(AutonomyMode::Autonomous);
        assert_eq!(mode.mode(), AutonomyMode::Autonomous);
    }

    #[test]
    fn events_compare_by_value() {
        let a = IntentExecutionEvent::Parked(Parked::new(slug("intent-capture")));
        let b = IntentExecutionEvent::Parked(Parked::new(slug("intent-capture")));
        assert_eq!(a, b);
        assert_ne!(a, IntentExecutionEvent::Unparked);
    }

    #[test]
    fn the_twelve_variants_are_matched_exhaustively() {
        // NFR1.3 — 変種の追加は C5 の改訂を伴うので `#[non_exhaustive]` は付けない。
        // 本テストは網羅 match をコンパイル時に固定する (腕が欠けたらビルドが落ちる)。
        fn name(payload: &IntentExecutionEvent) -> &'static str {
            match payload {
                IntentExecutionEvent::Started(_) => "Started",
                IntentExecutionEvent::StageCompleted(_) => "StageCompleted",
                IntentExecutionEvent::GateOpened(_) => "GateOpened",
                IntentExecutionEvent::GateApproved(_) => "GateApproved",
                IntentExecutionEvent::GateRejected(_) => "GateRejected",
                IntentExecutionEvent::StageRevised(_) => "StageRevised",
                IntentExecutionEvent::StageSkipped(_) => "StageSkipped",
                IntentExecutionEvent::Jumped(_) => "Jumped",
                IntentExecutionEvent::Parked(_) => "Parked",
                IntentExecutionEvent::Unparked => "Unparked",
                IntentExecutionEvent::Recomposed(_) => "Recomposed",
                IntentExecutionEvent::AutonomyModeSet(_) => "AutonomyModeSet",
            }
        }
        let entries = vec![StageEntry::new(
            slug("state-init"),
            PhaseId::Initialization,
            PlanAction::Execute,
            false,
            display("0.1"),
        )];
        let started = Started::new(Intent::from(Created::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "build it"),
            entries,
            scan(),
        )));
        let named = [
            (IntentExecutionEvent::Started(started), "Started"),
            (
                IntentExecutionEvent::StageCompleted(StageCompleted::new(slug("state-init"))),
                "StageCompleted",
            ),
            (
                IntentExecutionEvent::GateOpened(GateOpened::new(slug("intent-capture"), vec![])),
                "GateOpened",
            ),
            (
                IntentExecutionEvent::GateApproved(GateApproved::new(slug("intent-capture"), None)),
                "GateApproved",
            ),
            (
                IntentExecutionEvent::GateRejected(GateRejected::new(slug("intent-capture"), None)),
                "GateRejected",
            ),
            (
                IntentExecutionEvent::StageRevised(StageRevised::new(slug("intent-capture"))),
                "StageRevised",
            ),
            (
                IntentExecutionEvent::StageSkipped(StageSkipped::new(
                    slug("market-research"),
                    "out of scope".to_string(),
                )),
                "StageSkipped",
            ),
            (
                IntentExecutionEvent::Jumped(Jumped::new(slug("state-init"))),
                "Jumped",
            ),
            (
                IntentExecutionEvent::Parked(Parked::new(slug("intent-capture"))),
                "Parked",
            ),
            (IntentExecutionEvent::Unparked, "Unparked"),
            (
                IntentExecutionEvent::Recomposed(Recomposed::new(
                    Vec::new(),
                    vec![slug("state-init")],
                )),
                "Recomposed",
            ),
            (
                IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(AutonomyMode::Gated)),
                "AutonomyModeSet",
            ),
        ];
        for (event, expected) in &named {
            assert_eq!(name(event), *expected);
        }
    }
}
