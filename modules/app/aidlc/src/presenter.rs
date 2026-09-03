//! Presenter — directive を stdout の**1 行 JSON** へ写す（10 §4 Presenters / I1）。
//!
//! # 拒否は 2 層ある
//!
//! upstream の `aidlc-orchestrate.ts` は 2 つをはっきり分けており、こちらもそれに従う。
//!
//! | 層 | 何 | 出口 |
//! | --- | --- | --- |
//! | **ビジネス拒否** | 未知スコープ・排他フラグ・トークン不正など、ワークフローとして正当な「できません」 | **stdout に `error` directive**、exit 0 |
//! | **自己防衛拒否** | 28KiB 超・未知動詞など、配線の不具合や暴走を外へ出さないための拒否 | **stderr + exit 1** |
//!
//! ビジネス拒否は [`Directive::Error`] そのものなので、この層はただ描くだけである。自己
//! 防衛拒否だけがここの判断で、[`OversizeDirective`] がそれを表す。
//!
//! # malformed は表現不能である
//!
//! upstream には「malformed directive の emit 拒否」があるが、あちらは素の
//! オブジェクトを手で組むので必要な検査である。こちらは型付きの [`Directive`] からしか
//! 描かないので、**未知 kind も欠けた必須フィールドも構築の時点で存在しえない**。
//! したがって対応する分岐は置いていない（置いても到達不能な死んだ分岐になる）。
//! 観測される契約 — 「1 回の呼び出しで directive をちょうど 1 つ、half-emitted なし」— は
//! そのまま満たされる。
//!
//! # キーの順序について
//!
//! upstream は `JSON.stringify(validateDirective(d).data)` で、`data` は**検証した
//! オブジェクトそのもの**（作り直さない）。したがってキー順は upstream の各構築点の
//! 挿入順であって、契約として固定されたものではない。ここでは
//! `aidlc-directive.ts` の interface 宣言順に合わせてある。
//!
//! # ゴールデンとの突き合わせ（b44 で配線した）
//!
//! CLI 面のゴールデンは `tests/golden/upstream-3c3146cf/cli/` に 28 ケース採取済みで、
//! `modules/app/aidlc/tests/cli_golden_test.rs` が突き合わせる。**バイト一致で固定できるのは
//! 逐語文言だけの directive**（`continue/invalid-token`）で、`load-steering` と `run-stage` は
//! **キー集合**を固定する（中身は採取時のワークスペースの memory 層と配置に依存するため）。
//! どのケースが駆動できないか（逸脱台帳 #1 のコマンド綴り・vendored されていない scope
//! identity・state なし群）はそのテストのモジュール doc に列挙してある。
//!
//! 既知の欠落 2 つ — upstream の `run-stage` が載せる `conductor_persona` と `narration` を
//! こちらは載せない（b44 以前から。同テストが差を明示的に固定している）。

use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_interface_adapter::mint_continue_token;
use core_query_use_case::orchestration::{
    AskDirective, Directive, GateField, LoadSteeringDirective, RunStageDirective,
};

use crate::oversize_directive::OversizeDirective;

/// 文字列値。
fn text(value: impl Into<String>) -> JsonValue {
    JsonValue::String(value.into())
}

/// 文字列の配列。
fn texts(values: &[String]) -> JsonValue {
    JsonValue::Array(values.iter().map(text).collect())
}

/// upstream の `DIRECTIVE_MAX_BYTES`（`aidlc-orchestrate.ts:1151` — `28 * 1024`）。
pub const DIRECTIVE_MAX_BYTES: usize = 28 * 1024;

/// directive を 1 行 JSON へ写す。
///
/// 継続トークンの封緘鍵を握るのは、`load-steering` の `continue_token` が**この層で**
/// ワイヤ形式になるからである（クエリモデルの `ContinueToken` は型付きの値で、封緘は
/// 輸送形への変換 — `coding-rules/upstream-contracts.md`「境界で変換」）。鍵をどこから
/// 得るか（鋳造するか読むだけか）は合成ルートの方針であり、ここには無い。
#[derive(Debug, Clone)]
pub struct Presenter {
    key: Vec<u8>,
}

impl Presenter {
    /// 封緘鍵を据える。
    #[must_use]
    pub const fn new(key: Vec<u8>) -> Presenter {
        Presenter { key }
    }

    /// directive を 1 行 JSON へ描く（末尾の改行は付けない — 書く側が付ける）。
    ///
    /// # Errors
    ///
    /// 28KiB を超えたら [`OversizeDirective`]。呼出側は**何も stdout へ書かず**、
    /// stderr へ逐語を出して exit 1 する。
    pub fn render(&self, directive: &Directive) -> Result<String, OversizeDirective> {
        // 契約 JSON の直列化は canon-json の 1 経路に固定されている (BR1.7 / ADR 0001
        // 決定 5)。stdout の 1 行 JSON はまさに `ContractCompact` の用途である。
        let rendered = serialize(
            &JsonValue::Object(self.object_for(directive)),
            SerializationProfile::ContractCompact,
        );
        let bytes = rendered.len();
        if bytes > DIRECTIVE_MAX_BYTES {
            return Err(OversizeDirective::new(bytes));
        }
        Ok(rendered)
    }

    fn object_for(&self, directive: &Directive) -> ObjectMembers {
        let mut object = ObjectMembers::new();
        object.insert("kind", text(directive.kind().as_str()));
        match directive {
            Directive::LoadSteering(load) => self.fill_load_steering(&mut object, load),
            Directive::RunStage(run) => fill_run_stage(&mut object, run),
            Directive::Ask(ask) => fill_ask(&mut object, ask),
            Directive::Print { message } => {
                object.insert("message", text(message.clone()));
            }
            Directive::Error { message } => {
                object.insert("message", text(message.clone()));
            }
            Directive::Done { reason } => {
                // upstream の `reason` は必須の string。理由が無い終端は空文字で描く
                // （フィールドごと落とすと validateDirective の必須検査に落ちる形になる）。
                object.insert("reason", text(reason.clone().unwrap_or_default()));
            }
            Directive::Parked { stage, message } => {
                object.insert("reason", text(message.clone()));
                object.insert("stage", text(stage.as_str()));
            }
        }
        object
    }

    fn fill_load_steering(&self, object: &mut ObjectMembers, load: &LoadSteeringDirective) {
        object.insert("stage", text(load.stage().as_str()));
        object.insert("bundle", text(load.bundle().as_str()));
        object.insert("part", number(load.part().as_u32()));
        object.insert("parts", number(load.parts().as_u32()));
        let rules: Vec<JsonValue> = load
            .rules_content()
            .iter()
            .map(|rule| {
                let mut entry = ObjectMembers::new();
                entry.insert("path", text(rule.path()));
                entry.insert("text", text(rule.text()));
                JsonValue::Object(entry)
            })
            .collect();
        object.insert("rules_content", JsonValue::Array(rules));
        object.insert(
            "continue_token",
            text(mint_continue_token(&self.key, load.continue_token())),
        );
    }
}

fn fill_run_stage(object: &mut ObjectMembers, run: &RunStageDirective) {
    if let Some(narration) = run.narration() {
        object.insert("narration", text(narration));
    }
    object.insert("stage", text(run.stage().as_str()));
    object.insert("phase", text(run.phase().as_str()));
    object.insert("lead_agent", text(run.lead_agent()));
    object.insert("support_agents", texts(run.support_agents()));
    object.insert("mode", text(run.mode().as_str()));
    // `single` は真のときだけ載せる（upstream も optional — 既定は不在）。
    if run.is_single() {
        object.insert("single", JsonValue::Bool(true));
    }
    object.insert("inline_context_paths", texts(run.inline_context_paths()));
    object.insert("gate", gate_value(run.gate()));
    object.insert("memory_path", text(run.memory_path()));
    object.insert("consumes", texts(run.consumes()));
    object.insert("produces", texts(run.produces()));
    object.insert("rules_in_context", texts(run.rules_in_context()));
    object.insert("sensors_applicable", texts(run.sensors_applicable()));
    object.insert("stage_file", text(run.stage_file()));
    if let Some(reviewer) = run.reviewer() {
        object.insert("reviewer", text(reviewer));
    }
    if let Some(max) = run.reviewer_max_iterations() {
        object.insert("reviewer_max_iterations", number(max));
    }
    if let Some(class) = run.review_class() {
        object.insert("review_class", text(class.as_str()));
    }
    if !run.protocol_modules().is_empty() {
        object.insert("protocol_modules", texts(run.protocol_modules()));
    }
    if let Some(next) = run.next_stage() {
        object.insert("next_stage", text(next));
    }
    if let Some(unit) = run.unit() {
        object.insert("unit", text(unit.name().as_str()));
    }
}

fn fill_ask(object: &mut ObjectMembers, ask: &AskDirective) {
    object.insert("question", text(ask.question()));
    // new-work ルーティングの 4 フィールドは**揃って**現れる（upstream の判別共用体は
    // `ask_type` の有無で 2 形に分かれ、片方だけ載ることはない）。
    if let (Some(scope), Some(description)) = (ask.proposed_scope(), ask.new_work_description()) {
        object.insert("ask_type", text("new-work-routing"));
        object.insert("response_route", text("next"));
        object.insert("new_work_description", text(description));
        object.insert("proposed_scope", text(scope));
    }
}

/// `gate` は決定的な場合は boolean、walking-skeleton の未解決だけが番兵文字列
/// `"unresolved"` である（upstream `GateValue`）。
fn gate_value(gate: GateField) -> JsonValue {
    match gate {
        GateField::Gated => JsonValue::Bool(true),
        GateField::Ungated => JsonValue::Bool(false),
        GateField::Unresolved => text("unresolved"),
    }
}

/// 非負整数。
fn number(value: u32) -> JsonValue {
    JsonValue::Number(core_infrastructure::canon_json::Number::PosInt(u64::from(
        value,
    )))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::*;
    use core_query_use_case::orchestration::{
        AskKind, PhaseView, ReviewClassView, RunStageDirectiveBuilder, StageModeView,
        StageSlugView, UnitKind, UnitName, UnitRef,
    };

    const KEY: &[u8] = &[7u8; 32];

    fn presenter() -> Presenter {
        Presenter::new(KEY.to_vec())
    }

    fn rendered(directive: &Directive) -> JsonValue {
        let line = presenter().render(directive).expect("上限内なら描ける");
        assert!(!line.contains('\n'), "1 行である: {line}");
        core_infrastructure::canon_json::parse(&line).expect("描いたものは読み直せる")
    }

    /// オブジェクトのメンバを引く（無ければ `None`）。
    fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
        match value {
            JsonValue::Object(members) => members.get(key),
            _ => None,
        }
    }

    /// 文字列メンバを引く。
    fn string_of(value: &JsonValue, key: &str) -> String {
        match field(value, key) {
            Some(JsonValue::String(text)) => text.clone(),
            other => panic!("{key} は文字列であるべき: {other:?}"),
        }
    }

    /// 全部の任意欄を載せた run-stage。**キーの綴りが upstream 契約**なので逐語で確かめる。
    #[test]
    fn a_run_stage_renders_every_contract_key() {
        let directive = Directive::RunStage(
            RunStageDirectiveBuilder::new(
                StageSlugView::parse("domain-design").expect("slug"),
                PhaseView::Inception,
                "aidlc-architect-agent",
                StageModeView::Inline,
                GateField::Gated,
                ".claude/aidlc-common/stages/inception/domain-design.md",
                "<record>/inception/domain-design/memory.md",
            )
            .with_support_agents(vec!["aidlc-developer-agent".to_string()])
            .with_inline_context_paths(vec!["docs/a.md".to_string()])
            .with_consumes(vec!["requirements.md".to_string()])
            .with_produces(vec!["domain-model.md".to_string()])
            .with_sensors(vec!["aidlc-traceability".to_string()])
            .with_rules_in_context(vec!["memory/org.md".to_string()])
            .with_next_stage("Contract Design")
            .with_reviewer(
                "aidlc-architecture-reviewer-agent",
                ReviewClassView::Adversarial,
                3,
            )
            .with_protocol_modules(vec!["gate".to_string()])
            .with_narration("Designing the domain model.")
            .with_unit(UnitRef::new(
                UnitName::parse("u6-next-continue-use-case").expect("unit 名"),
                UnitKind::Library,
            ))
            .with_single()
            .build(),
        );

        let value = rendered(&directive);

        assert_eq!(string_of(&value, "kind"), "run-stage");
        assert_eq!(string_of(&value, "stage"), "domain-design");
        assert_eq!(string_of(&value, "phase"), "inception");
        assert_eq!(string_of(&value, "lead_agent"), "aidlc-architect-agent");
        assert_eq!(string_of(&value, "mode"), "inline");
        assert_eq!(field(&value, "gate"), Some(&JsonValue::Bool(true)));
        assert_eq!(field(&value, "single"), Some(&JsonValue::Bool(true)));
        assert_eq!(
            string_of(&value, "memory_path"),
            "<record>/inception/domain-design/memory.md"
        );
        assert_eq!(
            string_of(&value, "stage_file"),
            ".claude/aidlc-common/stages/inception/domain-design.md"
        );
        assert_eq!(string_of(&value, "next_stage"), "Contract Design");
        assert_eq!(
            string_of(&value, "reviewer"),
            "aidlc-architecture-reviewer-agent"
        );
        assert_eq!(string_of(&value, "review_class"), "adversarial");
        assert_eq!(
            field(&value, "reviewer_max_iterations"),
            Some(&JsonValue::Number(
                core_infrastructure::canon_json::Number::PosInt(3)
            ))
        );
        assert_eq!(
            string_of(&value, "narration"),
            "Designing the domain model."
        );
        assert_eq!(string_of(&value, "unit"), "u6-next-continue-use-case");
        for (key, member) in [
            ("support_agents", "aidlc-developer-agent"),
            ("inline_context_paths", "docs/a.md"),
            ("consumes", "requirements.md"),
            ("produces", "domain-model.md"),
            ("sensors_applicable", "aidlc-traceability"),
            ("rules_in_context", "memory/org.md"),
            ("protocol_modules", "gate"),
        ] {
            assert_eq!(
                field(&value, key),
                Some(&JsonValue::Array(vec![JsonValue::String(
                    member.to_string()
                )])),
                "{key}"
            );
        }
    }

    /// 任意欄は**与えられていなければ出ない**（upstream も optional は不在で表す）。
    #[test]
    fn a_bare_run_stage_omits_the_optional_keys() {
        let value = rendered(&Directive::RunStage(bare_run_stage(GateField::Ungated)));

        assert_eq!(field(&value, "gate"), Some(&JsonValue::Bool(false)));
        for key in [
            "single",
            "narration",
            "reviewer",
            "review_class",
            "reviewer_max_iterations",
            "protocol_modules",
            "next_stage",
            "unit",
        ] {
            assert_eq!(field(&value, key), None, "{key} は出ないはず");
        }
    }

    /// walking-skeleton の未解決ゲートだけが番兵文字列で出る（他は boolean）。
    #[test]
    fn an_unresolved_gate_is_the_sentinel_string() {
        let value = rendered(&Directive::RunStage(bare_run_stage(GateField::Unresolved)));
        assert_eq!(string_of(&value, "gate"), "unresolved");
    }

    /// 上限超過のエラーはバイト数を名指しする（診断の材料）。
    #[test]
    fn the_oversize_error_names_the_byte_count() {
        let huge = Directive::Error {
            message: "x".repeat(DIRECTIVE_MAX_BYTES),
        };
        let reported = presenter()
            .render(&huge)
            .expect_err("上限を超える")
            .to_string();
        assert!(reported.starts_with("directive of "), "{reported}");
        assert!(reported.ends_with("bytes exceeds the cap"), "{reported}");
    }

    /// 任意欄を 1 つも持たない run-stage。
    fn bare_run_stage(gate: GateField) -> RunStageDirective {
        RunStageDirectiveBuilder::new(
            StageSlugView::parse("state-init").expect("slug"),
            PhaseView::Initialization,
            "orchestrator",
            StageModeView::Inline,
            gate,
            "stage.md",
            "memory.md",
        )
        .build()
    }

    #[test]
    fn an_error_directive_carries_its_message_verbatim() {
        let value = rendered(&Directive::Error {
            message: "Unknown scope: \"nope\".".to_string(),
        });
        assert_eq!(string_of(&value, "kind"), "error");
        assert_eq!(string_of(&value, "message"), "Unknown scope: \"nope\".");
    }

    #[test]
    fn a_print_directive_carries_its_message_verbatim() {
        let value = rendered(&Directive::Print {
            message: "Run `aidlc-utility intent-create --scope bugfix`.".to_string(),
        });
        assert_eq!(string_of(&value, "kind"), "print");
        assert_eq!(
            string_of(&value, "message"),
            "Run `aidlc-utility intent-create --scope bugfix`."
        );
    }

    /// upstream の `reason` は必須の string — 理由が無い終端でもフィールドは落とさない。
    #[test]
    fn a_done_directive_always_carries_a_reason_field() {
        let value = rendered(&Directive::Done { reason: None });
        assert_eq!(string_of(&value, "kind"), "done");
        assert_eq!(string_of(&value, "reason"), "");

        let value = rendered(&Directive::Done {
            reason: Some("workflow complete".to_string()),
        });
        assert_eq!(string_of(&value, "reason"), "workflow complete");
    }

    /// parked の逐語メッセージは upstream の `reason` フィールドに載る（`message` ではない）。
    #[test]
    fn a_parked_directive_maps_its_message_onto_the_reason_field() {
        let value = rendered(&Directive::Parked {
            stage: StageSlugView::parse("domain-design").expect("slug は文法内"),
            message: "Workflow parked at \"domain-design\". Resume with /aidlc --resume."
                .to_string(),
        });
        assert_eq!(string_of(&value, "kind"), "parked");
        assert_eq!(string_of(&value, "stage"), "domain-design");
        assert_eq!(
            string_of(&value, "reason"),
            "Workflow parked at \"domain-design\". Resume with /aidlc --resume."
        );
        assert!(
            field(&value, "message").is_none(),
            "message は upstream に無い"
        );
    }

    #[test]
    fn a_plain_ask_omits_the_new_work_routing_fields() {
        let value = rendered(&Directive::Ask(AskDirective::new(
            AskKind::ResumeMenu,
            "Which option?".to_string(),
        )));
        assert_eq!(string_of(&value, "kind"), "ask");
        assert_eq!(string_of(&value, "question"), "Which option?");
        for absent in [
            "ask_type",
            "response_route",
            "new_work_description",
            "proposed_scope",
        ] {
            assert!(field(&value, absent).is_none(), "{absent} は載らない");
        }
    }

    /// new-work ルーティングの 4 フィールドは揃って現れる（片方だけは upstream の
    /// 判別共用体に存在しない形）。
    #[test]
    fn a_new_work_routing_ask_carries_all_four_routing_fields() {
        let value = rendered(&Directive::Ask(
            AskDirective::new(
                AskKind::NewWorkRouting,
                "Start a second intent?".to_string(),
            )
            .with_new_work("bugfix", "fix the crash"),
        ));
        assert_eq!(string_of(&value, "ask_type"), "new-work-routing");
        assert_eq!(string_of(&value, "response_route"), "next");
        assert_eq!(string_of(&value, "new_work_description"), "fix the crash");
        assert_eq!(string_of(&value, "proposed_scope"), "bugfix");
    }

    #[test]
    fn an_oversize_directive_is_refused_rather_than_half_emitted() {
        let huge = "x".repeat(DIRECTIVE_MAX_BYTES + 1);
        let error = presenter()
            .render(&Directive::Print { message: huge })
            .expect_err("上限超過は拒否される");
        assert!(error.bytes() > DIRECTIVE_MAX_BYTES);
    }

    /// 上限ちょうどは通る（境界の向きを固定する）。
    #[test]
    fn a_directive_at_exactly_the_cap_is_emitted() {
        // `{"kind":"print","message":"..."}` の外枠を差し引いた長さで詰める。
        let envelope = presenter()
            .render(&Directive::Print {
                message: String::new(),
            })
            .expect("空メッセージは描ける")
            .len();
        let message = "x".repeat(DIRECTIVE_MAX_BYTES - envelope);
        let line = presenter()
            .render(&Directive::Print { message })
            .expect("ちょうど上限なら描ける");
        assert_eq!(line.len(), DIRECTIVE_MAX_BYTES);
    }
}
