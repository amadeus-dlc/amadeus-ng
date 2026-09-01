//! steering ダイジェストの導出 — 束縛 3 種 ([`Bindings`] の材料のうち計画・directive・route)
//! を**純計算**で組む。
//!
//! ダイジェストは CPU とメモリだけを使う計算であり、I/O を持たない — したがってポートでは
//! なく型の責務である (オーナー裁定 2026-08-30「CPU とメモリだけを使った計算機のことを
//! いっているならドメインモデルの責務だろ」)。各ダイジェストの素材の正本は**それぞれ 1 つの
//! 型**が持つので、導出は自由関数ではなく**所有する型の関連メソッド**である
//! (`coding-rules/domain-services.md` — 対象を決めれば `::` / `.` で全タスクが見える)。
//! 素材の組み方 (canon_json 手組み) は 3 種で同じ規律なので、`impl` ブロックをこの
//! モジュールに束ねて材料ヘルパを共有する。
//!
//! 4 つめの state 束縛 (`h`) だけはリードモデルの持ち物なので、
//! [`crate::orchestration::ExecutionStateView::state_binding`] が所有する。
//!
//! # 素材は名前付き構造の canon_json 手組みである
//!
//! `Debug` 表現への依存は derive 変更で黙ってダイジェストが変わる時限爆弾であり、区切り文字
//! 連結は区切り文字注入を許す (オーナー裁定 2026-08-30)。かといって serde derive をここへ
//! 持ち込むこともしない — 素材は言語拡張 [`canon_json`] の [`JsonValue`] を**手で組み**、
//! CompactRaw 族 ([`hash_compact`]) でハッシュする。
//!
//! [`Bindings`]: super::bindings::Bindings
//! [`canon_json`]: core_infrastructure::canon_json

use core_infrastructure::canon_json::{JsonValue, hash_compact};

use super::bundle_digest::BundleDigest;
use super::directive_digest::DirectiveDigest;
use super::gate_field::GateField;
use super::route_digest::RouteDigest;
use super::run_stage_directive::RunStageDirective;
use super::steering_plan::SteeringPlan;
use crate::orchestration::{StageRouteView, StageSlugView};

impl SteeringPlan {
    /// ルール束のダイジェスト (`b`) — **部境界ごと**の piece の読み順 (path + text)。
    ///
    /// 素材は chunk の入れ子配列である — 平坦化すると `[[A], [B]]` と `[[A, B]]` が同じ
    /// ダイジェストになり、内容が同じまま分割だけが変わった計画 (分割アルゴリズムの更新
    /// など) を continue の照合が見逃して、部の欠落・重複配信を許してしまう
    /// (fail-closed I12 の網羅)。
    #[must_use]
    pub fn bundle_digest(&self) -> BundleDigest {
        let chunks = self
            .chunks()
            .iter()
            .map(|chunk| {
                JsonValue::Array(
                    chunk
                        .iter()
                        .map(|piece| {
                            object([
                                ("path", JsonValue::String(piece.path().to_string())),
                                ("text", JsonValue::String(piece.text().to_string())),
                            ])
                        })
                        .collect(),
                )
            })
            .collect();
        BundleDigest::new(hash_compact(&JsonValue::Array(chunks)).rendered())
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

impl StageRouteView {
    /// グラフノードと scope メンバーシップの route ダイジェスト (`r`)。
    ///
    /// 型は `workflow_view` の語彙だが、ダイジェストは steering 連鎖の束縛語彙なので
    /// `impl` はこのモジュールに置く (素材規律を 3 種で共有するため)。
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
                        .map(StageSlugView::as_str)
                        .map(|slug| JsonValue::String(slug.to_string()))
                        .collect(),
                ),
            ),
        ]);
        RouteDigest::new(hash_compact(&material).rendered())
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

#[cfg(test)]
mod tests {
    use super::super::bindings::Bindings;
    use super::super::continue_token::ContinueTokenBuilder;
    use super::super::part_index::PartIndex;
    use super::super::rule_content::RuleContent;
    use super::super::run_stage_directive::RunStageDirectiveBuilder;
    use super::super::unit_kind::UnitKind;
    use super::super::unit_name::UnitName;
    use super::super::unit_ref::UnitRef;
    use super::*;
    use crate::orchestration::{PhaseView, ScopeSlugView, StageModeView};

    fn run_stage(gate: GateField) -> RunStageDirective {
        RunStageDirectiveBuilder::new(
            StageSlugView::parse("functional-design").unwrap(),
            PhaseView::Inception,
            "aidlc-architect-agent",
            StageModeView::Inline,
            gate,
            "stage.md",
            "memory.md",
        )
        .build()
    }

    fn pin(gate: GateField) -> ContinueTokenBuilder {
        ContinueTokenBuilder::new(
            StageSlugView::parse("functional-design").unwrap(),
            ScopeSlugView::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                None,
            ),
            gate,
        )
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

        // 部境界はダイジェストの一部 — 内容が同じでも分割が違えば別の束である
        // (平坦化すると照合が分割の変化を見逃す)。
        let piece = |name: &str| RuleContent::new(name.to_string(), "# X\n".to_string());
        let split = SteeringPlan::new(vec![vec![piece("a.md")], vec![piece("b.md")]]);
        let joined = SteeringPlan::new(vec![vec![piece("a.md"), piece("b.md")]]);
        assert_ne!(split.bundle_digest(), joined.bundle_digest());

        let route = StageRouteView::new(
            StageSlugView::parse("functional-design").unwrap(),
            vec![StageSlugView::parse("intent-capture").unwrap()],
        );
        assert_eq!(route.route_digest(), route.route_digest());
        let moved_route = StageRouteView::new(
            StageSlugView::parse("functional-design").unwrap(),
            vec![
                StageSlugView::parse("intent-capture").unwrap(),
                StageSlugView::parse("scope-definition").unwrap(),
            ],
        );
        assert_ne!(route.route_digest(), moved_route.route_digest());
    }

    #[test]
    fn the_directive_digest_reads_the_named_material_not_a_debug_dump() {
        let directive = run_stage(GateField::Gated);
        let same = directive.directive_digest();
        assert_eq!(directive.directive_digest(), same);

        let single = directive
            .with_pins(&pin(GateField::Gated).with_single().build())
            .directive_digest();
        assert_ne!(single, same, "single ピンはダイジェストに効く");

        // unit を運ぶ run-stage は別素材 (unit の名前 + 種別が素材に載る)。
        let with_unit = directive
            .with_pins(
                &pin(GateField::Gated)
                    .with_unit(UnitRef::new(
                        UnitName::parse("u6-next-continue-use-case").unwrap(),
                        UnitKind::Library,
                    ))
                    .build(),
            )
            .directive_digest();
        assert_ne!(with_unit, same, "unit ピンはダイジェストに効く");
        assert_ne!(with_unit, single);

        // gate の 3 値はどれも別素材である (unresolved は文字列素材)。
        let unresolved = run_stage(GateField::Unresolved).directive_digest();
        let ungated = run_stage(GateField::Ungated).directive_digest();
        assert_ne!(unresolved, same);
        assert_ne!(ungated, same);
        assert_ne!(ungated, unresolved);
    }
}
