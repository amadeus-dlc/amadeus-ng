//! steering ダイジェストの導出 — 束縛 4 種 ([`Bindings`] の材料) を**ドメインの純計算**で組む。
//!
//! ダイジェストは CPU とメモリだけを使う計算であり、I/O を持たない — したがってポートでは
//! なくドメインの責務である (オーナー裁定 2026-08-30「CPU とメモリだけを使った計算機のこと
//! をいっているならドメインモデルの責務だろ」。旧 `ContinueTokenCodec` ポートの廃止 —
//! issue #45)。各ダイジェストの素材の正本は**それぞれ 1 つの型**が持つので、導出は自由関数
//! ではなく**所有する型の関連メソッド**である (`coding-rules/domain-services.md` — 対象を
//! 決めれば `::` / `.` で全タスクが見える)。素材の組み方 (canon_json 手組み) は 4 種で同じ
//! 規律なので、`impl` ブロックをこのモジュールに束ねて材料ヘルパを共有する。
//!
//! # 素材は名前付き構造の canon_json 手組みである
//!
//! `Debug` 表現への依存は derive 変更で黙ってダイジェストが変わる時限爆弾であり、区切り文字
//! 連結は区切り文字注入を許す (オーナー裁定 2026-08-30)。かといって serde derive をドメインに
//! 持ち込むこともしない (`coding-rules/domain-persistence-neutrality.md`) — 素材は言語拡張
//! [`canon_json`] の [`JsonValue`] を**手で組み**、CompactRaw 族 ([`hash_compact`]) で
//! ハッシュする (bundle hash・directiveHash・route hash はこの族 — `canon_json::digest` の
//! 族 doc)。トークンはセッションローカルなので、旧実装 (アダプタの serde_json 直列化) と
//! ダイジェスト値が変わっても互換負債は無い。
//!
//! [`Bindings`]: super::steering_binding::Bindings
//! [`canon_json`]: core_infrastructure::canon_json

use core_infrastructure::canon_json::{JsonValue, Number, hash_compact};

use super::intent_execution::IntentExecution;
use super::steering_binding::{BundleDigest, DirectiveDigest, RouteDigest, StateBinding};
use super::steering_plan::SteeringPlan;
use crate::workflow_definition::{StageRoute, StageSlug};

use super::directive::{GateField, RunStageDirective};

impl SteeringPlan {
    /// ルール束のダイジェスト (`b`) — piece の読み順 (path + text の列)。
    #[must_use]
    pub fn bundle_digest(&self) -> BundleDigest {
        let pieces = self
            .chunks()
            .iter()
            .flatten()
            .map(|piece| {
                object([
                    ("path", JsonValue::String(piece.path().to_string())),
                    ("text", JsonValue::String(piece.text().to_string())),
                ])
            })
            .collect();
        BundleDigest::new(hash_compact(&JsonValue::Array(pieces)).rendered())
    }
}

impl RunStageDirective {
    /// 届けようとしている run-stage のダイジェスト (`d`) — キー項目の名前付き素材。
    #[must_use]
    pub fn directive_digest(&self) -> DirectiveDigest {
        let unit = self.unit().map_or(JsonValue::Null, |unit| {
            object([
                ("name", JsonValue::String(unit.name().as_str().to_string())),
                ("kind", JsonValue::String(unit.kind().as_str().to_string())),
            ])
        });
        let material = object([
            (
                "stage",
                JsonValue::String(self.stage().as_str().to_string()),
            ),
            ("gate", gate_material(self.gate())),
            (
                "stage_file",
                JsonValue::String(self.stage_file().to_string()),
            ),
            (
                "memory_path",
                JsonValue::String(self.memory_path().to_string()),
            ),
            (
                "next_stage",
                self.next_stage()
                    .map_or(JsonValue::Null, |name| JsonValue::String(name.to_string())),
            ),
            ("unit", unit),
            ("single", JsonValue::Bool(self.is_single())),
        ]);
        DirectiveDigest::new(hash_compact(&material).rendered())
    }
}

impl StageRoute {
    /// グラフノードと scope メンバーシップの route ダイジェスト (`r`)。
    ///
    /// 型は `workflow_definition` の語彙だが、ダイジェストは steering 連鎖の束縛語彙なので
    /// `impl` はこのモジュールに置く (素材規律を 4 種で共有するため)。
    #[must_use]
    pub fn route_digest(&self) -> RouteDigest {
        let material = object([
            (
                "stage",
                JsonValue::String(self.stage().as_str().to_string()),
            ),
            (
                "stages",
                JsonValue::Array(
                    self.stages_in_scope()
                        .iter()
                        .map(StageSlug::as_str)
                        .map(|slug| JsonValue::String(slug.to_string()))
                        .collect(),
                ),
            ),
        ]);
        RouteDigest::new(hash_compact(&material).rendered())
    }
}

impl IntentExecution {
    /// state 束縛のダイジェスト (`h`)。
    ///
    /// 束縛の対象は「どの intent の・何番目まで進んだ歴史の・どの採番版か」であり、その
    /// 3 つはすべてこの集約が持っている (オーナー裁定 2026-08-30 — 三つ組 VO は廃止済み)。
    #[must_use]
    pub fn state_binding(&self) -> StateBinding {
        let material = object([
            (
                "intent_id",
                JsonValue::String(self.intent_id().as_str().to_string()),
            ),
            ("seq_nr", integer(self.seq_nr())),
            ("version", integer(self.version())),
        ]);
        StateBinding::new(hash_compact(&material).rendered())
    }
}

/// `gate` の素材 — boolean か `"unresolved"` (upstream ワイヤ形式と同じ 3 値)。
fn gate_material(gate: GateField) -> JsonValue {
    match gate {
        GateField::Gated => JsonValue::Bool(true),
        GateField::Ungated => JsonValue::Bool(false),
        GateField::Unresolved => JsonValue::String("unresolved".to_string()),
    }
}

/// 挿入順を保持する素材オブジェクト (順序が素材バイトの一部である)。
fn object<const N: usize>(members: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        members
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

/// 非負整数の素材。
const fn integer(value: usize) -> JsonValue {
    JsonValue::Number(Number::PosInt(value as u64))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::orchestration::{
        Bindings, ContinueTokenBuilder, Created, GateField, Intent, IntentExecutionId, IntentId,
        PartIndex, RuleContent, RunStageDirectiveBuilder, StageDisplay, StageEntry, StartRequest,
        SteeringPlan, UnitKind, UnitName, UnitRef, WorkspaceScan,
    };
    use crate::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, ScopeSlug, StageMode,
        StageNumber, StageSlug, WorkflowDefinitionId,
    };
    use chrono::{DateTime, Utc};

    /// state 束縛の素材になる集約 — 版だけ差し替えられる形で組む。
    fn execution(version: usize) -> IntentExecution {
        let intent = Intent::from(Created::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "state binding"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                StageDisplay::new(StageNumber::parse("0.1").unwrap(), "Stage", "orchestrator")
                    .unwrap(),
            )],
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        ));
        let (execution, _event) = IntentExecution::start(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            &intent,
            DateTime::<Utc>::UNIX_EPOCH,
        );
        execution.with_version(version)
    }

    #[test]
    fn digests_are_deterministic_and_subject_specific() {
        let plan = SteeringPlan::new(vec![vec![RuleContent::new(
            "org.md".to_string(),
            "# Org\n".to_string(),
        )]]);
        assert_eq!(plan.bundle_digest(), plan.bundle_digest());
        let other = SteeringPlan::new(vec![vec![RuleContent::new(
            "org.md".to_string(),
            "# Org2\n".to_string(),
        )]]);
        assert_ne!(plan.bundle_digest(), other.bundle_digest());

        let route = StageRoute::new(
            StageSlug::parse("functional-design").unwrap(),
            vec![StageSlug::parse("intent-capture").unwrap()],
        );
        assert_eq!(route.route_digest(), route.route_digest());
        let moved_route = StageRoute::new(
            StageSlug::parse("functional-design").unwrap(),
            vec![
                StageSlug::parse("intent-capture").unwrap(),
                StageSlug::parse("scope-definition").unwrap(),
            ],
        );
        assert_ne!(route.route_digest(), moved_route.route_digest());

        let held = execution(4);
        assert_eq!(held.state_binding(), held.state_binding());
        // 版が動けば束縛も動く — 通番が同じでも別の state である。
        let moved = execution(5);
        assert_ne!(held.state_binding(), moved.state_binding());
    }

    #[test]
    fn the_directive_digest_reads_the_named_material_not_a_debug_dump() {
        let run_stage = RunStageDirectiveBuilder::new(
            StageSlug::parse("functional-design").unwrap(),
            PhaseId::Inception,
            "aidlc-architect-agent",
            StageMode::Inline,
            GateField::Gated,
            "stage.md",
            "memory.md",
        )
        .build();
        let same = run_stage.directive_digest();
        assert_eq!(run_stage.directive_digest(), same);
        let pinned_single = ContinueTokenBuilder::new(
            StageSlug::parse("functional-design").unwrap(),
            ScopeSlug::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            GateField::Gated,
        )
        .with_single()
        .build();
        let single = run_stage.with_pins(&pinned_single).directive_digest();
        assert_ne!(single, same, "single ピンはダイジェストに効く");

        // unit を運ぶ run-stage は別素材 (unit の名前 + 種別が素材に載る)。
        let pinned_unit = ContinueTokenBuilder::new(
            StageSlug::parse("functional-design").unwrap(),
            ScopeSlug::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            GateField::Gated,
        )
        .with_unit(UnitRef::new(
            UnitName::parse("u6-next-continue-use-case").unwrap(),
            UnitKind::Library,
        ))
        .build();
        let with_unit = run_stage.with_pins(&pinned_unit).directive_digest();
        assert_ne!(with_unit, same, "unit ピンはダイジェストに効く");
        assert_ne!(with_unit, single);

        // gate の 3 値はどれも別素材である (unresolved は文字列素材)。
        let unresolved = RunStageDirectiveBuilder::new(
            StageSlug::parse("functional-design").unwrap(),
            PhaseId::Inception,
            "aidlc-architect-agent",
            StageMode::Inline,
            GateField::Unresolved,
            "stage.md",
            "memory.md",
        )
        .build()
        .directive_digest();
        assert_ne!(unresolved, same);
    }
}
