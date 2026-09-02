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

// 変種ペイロードは 1 ファイル 1 公開型で本ファイル同名のサブツリーに置き、ここで連鎖
// 再輸出する (所有サブツリーのファサード — 利便再エクスポートではない。
// coding-rules/module-visibility.md)。
mod autonomy_mode_set;
mod gate_approved;
mod gate_opened;
mod gate_rejected;
mod jumped;
mod parked;
mod recomposed;
mod stage_completed;
mod stage_revised;
mod stage_skipped;
mod started;

pub use autonomy_mode_set::AutonomyModeSet;
pub use gate_approved::GateApproved;
pub use gate_opened::GateOpened;
pub use gate_rejected::GateRejected;
pub use jumped::Jumped;
pub use parked::Parked;
pub use recomposed::Recomposed;
pub use stage_completed::StageCompleted;
pub use stage_revised::StageRevised;
pub use stage_skipped::StageSkipped;
pub use started::Started;

/// 12 変種のドメインイベント (C5 の 11 + `StageCompleted`)。
///
/// `#[non_exhaustive]` は**付けない** — 変種の追加は C5 の改訂を伴う設計事項であり、消費側の
/// 網羅 match が落ちること自体が検出手段である (NFR1.3)。`Unparked` は C5 が `payload: {}` と
/// するので専用の材料型を持たない単位変種にした。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentExecutionEvent {
    /// 実行の開始。
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

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する (集約のテストモジュールと同じ作法)。
    #![allow(clippy::panic)]

    use super::*;
    use crate::orchestration::{AutonomyMode, IntentExecutionId, StageDisplay, StageEntry};
    use crate::workflow_definition::{PhaseId, PlanAction, StageNumber, StageSlug};

    use super::super::intent_id::IntentId;

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    /// genesis の材料 3 つを束ねたペイロード (計画は 1 ステージの最小形)。
    fn started() -> Started {
        Started::new(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
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

    #[test]
    fn the_started_payload_carries_the_genesis_material() {
        // genesis の材料 (実行 id・intent id・解決済み計画) を運ぶ — 誕生状態の導出に
        // `&Intent` を要さないので、実行のストリームは自ストリームだけで再生できる。
        let started = started();
        assert_eq!(
            started.id().as_str(),
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
        let named = [
            (IntentExecutionEvent::Started(started()), "Started"),
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
