//! `IntentExecutionEvent` — 13 変種のドメインイベント (C5、entities.md)。
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

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — 利便再エクスポートではない。
// coding-rules/module-visibility.md)。
use super::intent_execution_event_id::IntentExecutionEventId;
use super::intent_execution_id::IntentExecutionId;

mod autonomy_mode_set;
mod gate_approved;
mod gate_opened;
mod gate_rejected;
mod jumped;
mod parked;
mod recomposed;
mod single_stage_run_committed;
mod skeleton_stance_recorded;
mod stage_revised;
mod stage_skipped;
mod started;
mod unparked;

pub use autonomy_mode_set::AutonomyModeSet;
pub use gate_approved::GateApproved;
pub use gate_opened::GateOpened;
pub use gate_rejected::GateRejected;
pub use jumped::Jumped;
pub use parked::Parked;
pub use recomposed::Recomposed;
pub use single_stage_run_committed::SingleStageRunCommitted;
pub use skeleton_stance_recorded::SkeletonStanceRecorded;
pub use stage_revised::StageRevised;
pub use stage_skipped::StageSkipped;
pub use started::Started;
pub use unparked::Unparked;

/// 13 変種のドメインイベント (C5)。
///
/// `#[non_exhaustive]` は**付けない** — 変種の追加は C5 の改訂を伴う設計事項であり、消費側の
/// 網羅 match が落ちること自体が検出手段である (NFR1.3)。
///
/// # イベントはエンティティ — 全変種が `id` と `aggregate_id` を持つ
///
/// ドメインイベントはエンティティの一種なので、変種ごとに自前の識別子
/// [`IntentExecutionEventId`] を持ち、どの集約の事実かは別フィールド `aggregate_id` が運ぶ
/// (オーナー裁定 2026-09-02、`coding-rules/domain-object-kinds.md` /
/// `coding-rules/aggregate-commands.md`)。採番は集約のコマンド内 (`generate`) であり、通番
/// `seq_nr` と発生時刻 `occurred_at` は従来どおり封筒が運ぶ (ADR-010 / B7)。
///
/// `Unparked` は C5 が `payload: {}` とする材料なしの事実だが、それでも識別子は持つので
/// 単位変種ではなく [`Unparked`] 構造体を張る。
///
/// [`IntentExecutionEventId`]: super::intent_execution_event_id::IntentExecutionEventId
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentExecutionEvent {
    /// 実行の開始。
    Started(Started),
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
    Unparked(Unparked),
    /// 実効プランの再形成 (オーバレイの反転)。
    Recomposed(Recomposed),
    /// 自律モードの設定。
    AutonomyModeSet(AutonomyModeSet),
    /// 隔離実行 (`--single`) の疑似ワークフロー ID 付き対の記録 (**適用はフレーム空**)。
    SingleStageRunCommitted(SingleStageRunCommitted),
    /// conductor が分類した walking-skeleton stance の記録。
    SkeletonStanceRecorded(SkeletonStanceRecorded),
}

impl IntentExecutionEvent {
    /// このイベント自身の識別子 (全変種が持つ — イベントはエンティティ)。
    #[must_use]
    pub const fn id(&self) -> &IntentExecutionEventId {
        match self {
            IntentExecutionEvent::Started(payload) => payload.id(),
            IntentExecutionEvent::GateOpened(payload) => payload.id(),
            IntentExecutionEvent::GateApproved(payload) => payload.id(),
            IntentExecutionEvent::GateRejected(payload) => payload.id(),
            IntentExecutionEvent::StageRevised(payload) => payload.id(),
            IntentExecutionEvent::StageSkipped(payload) => payload.id(),
            IntentExecutionEvent::Jumped(payload) => payload.id(),
            IntentExecutionEvent::Parked(payload) => payload.id(),
            IntentExecutionEvent::Unparked(payload) => payload.id(),
            IntentExecutionEvent::Recomposed(payload) => payload.id(),
            IntentExecutionEvent::AutonomyModeSet(payload) => payload.id(),
            IntentExecutionEvent::SingleStageRunCommitted(payload) => payload.id(),
            IntentExecutionEvent::SkeletonStanceRecorded(payload) => payload.id(),
        }
    }

    /// **どの集約の事実か** — 全変種が運ぶ実行の識別子。
    ///
    /// 復号境界 (Repository の再生・RMU の `decode_entry`) はこれと行の `aid` を照合する。
    #[must_use]
    pub const fn aggregate_id(&self) -> &IntentExecutionId {
        match self {
            IntentExecutionEvent::Started(payload) => payload.aggregate_id(),
            IntentExecutionEvent::GateOpened(payload) => payload.aggregate_id(),
            IntentExecutionEvent::GateApproved(payload) => payload.aggregate_id(),
            IntentExecutionEvent::GateRejected(payload) => payload.aggregate_id(),
            IntentExecutionEvent::StageRevised(payload) => payload.aggregate_id(),
            IntentExecutionEvent::StageSkipped(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Jumped(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Parked(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Unparked(payload) => payload.aggregate_id(),
            IntentExecutionEvent::Recomposed(payload) => payload.aggregate_id(),
            IntentExecutionEvent::AutonomyModeSet(payload) => payload.aggregate_id(),
            IntentExecutionEvent::SingleStageRunCommitted(payload) => payload.aggregate_id(),
            IntentExecutionEvent::SkeletonStanceRecorded(payload) => payload.aggregate_id(),
        }
    }
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する (集約のテストモジュールと同じ作法)。
    #![allow(clippy::panic)]

    use std::collections::HashSet;

    use super::*;
    use crate::orchestration::{AutonomyMode, SkeletonStance, StageDisplay, StageEntry};
    use crate::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};

    use super::super::intent_id::IntentId;

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    /// 決め打ちのイベント識別子 (綴りを固定したいテスト用)。
    fn evid() -> IntentExecutionEventId {
        IntentExecutionEventId::parse("018f3b2c-4d5e-7f60-8abc-def012345678").unwrap()
    }

    fn agg() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap()
    }

    /// genesis の材料を束ねたペイロード (計画は 1 ステージの最小形)。
    fn started() -> Started {
        Started::new(
            evid(),
            agg(),
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            vec![StageEntry::new(
                slug("state-init"),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                StageDisplay::new(
                    StageNumber::parse("0.1").unwrap(),
                    "State Init",
                    "orchestrator",
                )
                .unwrap(),
            )],
        )
    }

    /// 13 変種を 1 つずつ (同じ id / aggregate_id で組む)。
    fn every_variant() -> Vec<IntentExecutionEvent> {
        vec![
            IntentExecutionEvent::Started(started()),
            IntentExecutionEvent::GateOpened(GateOpened::new(
                evid(),
                agg(),
                slug("intent-capture"),
                vec!["intent.md".to_string()],
            )),
            IntentExecutionEvent::GateApproved(GateApproved::new(
                evid(),
                agg(),
                slug("intent-capture"),
                Some("looks good".to_string()),
            )),
            IntentExecutionEvent::GateRejected(GateRejected::new(
                evid(),
                agg(),
                slug("intent-capture"),
                None,
            )),
            IntentExecutionEvent::StageRevised(StageRevised::new(
                evid(),
                agg(),
                slug("intent-capture"),
            )),
            IntentExecutionEvent::StageSkipped(StageSkipped::new(
                evid(),
                agg(),
                slug("market-research"),
                "out of scope".to_string(),
            )),
            IntentExecutionEvent::Jumped(Jumped::new(evid(), agg(), slug("state-init"))),
            IntentExecutionEvent::Parked(Parked::new(evid(), agg(), slug("intent-capture"))),
            IntentExecutionEvent::Unparked(Unparked::new(evid(), agg())),
            IntentExecutionEvent::Recomposed(Recomposed::new(
                evid(),
                agg(),
                vec![slug("market-research")],
                Vec::new(),
            )),
            IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
                evid(),
                agg(),
                AutonomyMode::Autonomous,
            )),
            IntentExecutionEvent::SingleStageRunCommitted(SingleStageRunCommitted::new(
                evid(),
                agg(),
                slug("intent-capture"),
            )),
            IntentExecutionEvent::SkeletonStanceRecorded(SkeletonStanceRecorded::new(
                evid(),
                agg(),
                SkeletonStance::On,
            )),
        ]
    }

    #[test]
    fn the_started_payload_carries_the_genesis_material() {
        // genesis の材料 (実行 id・intent id・解決済み計画) を運ぶ — 誕生状態の導出に
        // `&Intent` を要さないので、実行のストリームは自ストリームだけで再生できる。
        let started = started();
        assert_eq!(
            started.aggregate_id().as_str(),
            "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"
        );
        assert_eq!(
            started.intent_id().as_str(),
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
        assert_eq!(started.stages().len(), 1);
        assert_eq!(
            started.stages().first().map(StageEntry::slug),
            Some(&slug("state-init"))
        );
    }

    #[test]
    fn the_stage_lifecycle_payloads_carry_their_slugs_and_material() {
        let opened = GateOpened::new(
            evid(),
            agg(),
            slug("intent-capture"),
            vec!["intent.md".to_string()],
        );
        assert_eq!(opened.stage(), &slug("intent-capture"));
        assert_eq!(opened.artifacts(), ["intent.md".to_string()]);

        let approved = GateApproved::new(
            evid(),
            agg(),
            slug("intent-capture"),
            Some("looks good".to_string()),
        );
        assert_eq!(approved.stage(), &slug("intent-capture"));
        assert_eq!(approved.user_input(), Some("looks good"));

        let rejected = GateRejected::new(evid(), agg(), slug("intent-capture"), None);
        assert_eq!(rejected.stage(), &slug("intent-capture"));
        assert_eq!(rejected.feedback(), None);

        let revised = StageRevised::new(evid(), agg(), slug("intent-capture"));
        assert_eq!(revised.stage(), &slug("intent-capture"));

        let skipped = StageSkipped::new(
            evid(),
            agg(),
            slug("market-research"),
            "out of scope".to_string(),
        );
        assert_eq!(skipped.stage(), &slug("market-research"));
        assert_eq!(skipped.reason(), "out of scope");
    }

    #[test]
    fn the_control_payloads_carry_the_jump_park_and_recompose_material() {
        let jumped = Jumped::new(evid(), agg(), slug("state-init"));
        assert_eq!(jumped.target(), &slug("state-init"));

        let parked = Parked::new(evid(), agg(), slug("intent-capture"));
        assert_eq!(parked.stage(), &slug("intent-capture"));

        // `Unparked` は材料なしだが単位変種ではない — 識別子は持つ。
        let unparked = Unparked::new(evid(), agg());
        assert_eq!(unparked.id(), &evid());
        assert_eq!(unparked.aggregate_id(), &agg());

        let recomposed = Recomposed::new(evid(), agg(), vec![slug("market-research")], Vec::new());
        assert_eq!(recomposed.skipped(), [slug("market-research")]);
        assert!(recomposed.added().is_empty());

        let mode = AutonomyModeSet::new(evid(), agg(), AutonomyMode::Autonomous);
        assert_eq!(mode.mode(), AutonomyMode::Autonomous);
    }

    #[test]
    fn every_variant_answers_its_own_id_and_its_aggregate_id() {
        // イベントはエンティティ — 変種によらず自前の id と「どの集約の事実か」を答える。
        for event in every_variant() {
            assert_eq!(event.id(), &evid());
            assert_eq!(event.aggregate_id(), &agg());
        }
    }

    #[test]
    fn events_compare_by_value() {
        let a = IntentExecutionEvent::Parked(Parked::new(evid(), agg(), slug("intent-capture")));
        let b = IntentExecutionEvent::Parked(Parked::new(evid(), agg(), slug("intent-capture")));
        assert_eq!(a, b);
        assert_ne!(
            a,
            IntentExecutionEvent::Unparked(Unparked::new(evid(), agg()))
        );
    }

    #[test]
    fn two_events_of_the_same_shape_are_distinguished_by_their_generated_ids() {
        // 同じ材料でも別の事実である — 識別子が違えば別のエンティティになる。
        let first = Parked::new(
            IntentExecutionEventId::generate(),
            agg(),
            slug("intent-capture"),
        );
        let second = Parked::new(
            IntentExecutionEventId::generate(),
            agg(),
            slug("intent-capture"),
        );
        assert_ne!(first.id(), second.id());
        assert_ne!(first, second);
    }

    #[test]
    fn the_thirteen_variants_are_matched_exhaustively() {
        // NFR1.3 — 変種の追加は C5 の改訂を伴うので `#[non_exhaustive]` は付けない。
        // 本テストは網羅 match をコンパイル時に固定する (腕が欠けたらビルドが落ちる)。
        const fn name(payload: &IntentExecutionEvent) -> &'static str {
            match payload {
                IntentExecutionEvent::Started(_) => "Started",
                IntentExecutionEvent::GateOpened(_) => "GateOpened",
                IntentExecutionEvent::GateApproved(_) => "GateApproved",
                IntentExecutionEvent::GateRejected(_) => "GateRejected",
                IntentExecutionEvent::StageRevised(_) => "StageRevised",
                IntentExecutionEvent::StageSkipped(_) => "StageSkipped",
                IntentExecutionEvent::Jumped(_) => "Jumped",
                IntentExecutionEvent::Parked(_) => "Parked",
                IntentExecutionEvent::Unparked(_) => "Unparked",
                IntentExecutionEvent::Recomposed(_) => "Recomposed",
                IntentExecutionEvent::AutonomyModeSet(_) => "AutonomyModeSet",
                IntentExecutionEvent::SingleStageRunCommitted(_) => "SingleStageRunCommitted",
                IntentExecutionEvent::SkeletonStanceRecorded(_) => "SkeletonStanceRecorded",
            }
        }
        let expected = [
            "Started",
            "GateOpened",
            "GateApproved",
            "GateRejected",
            "StageRevised",
            "StageSkipped",
            "Jumped",
            "Parked",
            "Unparked",
            "Recomposed",
            "AutonomyModeSet",
            "SingleStageRunCommitted",
            "SkeletonStanceRecorded",
        ];
        let named: Vec<&'static str> = every_variant().iter().map(name).collect();
        assert_eq!(named, expected);
        let distinct: HashSet<&'static str> = named.iter().copied().collect();
        assert_eq!(distinct.len(), 13);
    }
}
