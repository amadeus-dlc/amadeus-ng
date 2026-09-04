//! 行 → directive の**描画** — リードモデルの行を公開言語の形に写す。
//!
//! # ここに判断は無い
//!
//! 何を描くかは行が言う (`read_next_answer.decision_kind` — 集約が決め RMU が焼いた綴り)。
//! ここがするのは (a) 綴りに対応する directive を選ぶこと、(b) 行の相対パスに配置の基準を
//! 前置すること、(c) 1 行 JSON の列を配列へ開くこと、の 3 つだけである
//! (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記 —
//! 「逐語文言・directive / token の綴りはプレゼンタが行の `kind` に従って描く」)。
//!
//! # パスの絶対化はここでしか起きない
//!
//! 行は**基準ごとの相対パス**を持つ (行が絶対パスを持つと、ワークスペースを移しただけで
//! 全行が書き替わる)。基準は 3 つ — ステージ本体の置き場・record・ハーネス根 — で、
//! いずれも [`Layout`] が知っている。

use core_infrastructure::canon_json::{JsonValue, parse};
use core_query_use_case::orchestration::{
    Bindings, BundleDigest, ContinueToken, ContinueTokenBuilder, Directive, DirectiveDigest,
    GateField, LoadSteeringDirective, PartCount, PartIndex, PhaseView, ReviewClassView,
    RouteDigest, RuleContent, RunStageDirective, RunStageDirectiveBuilder, RunStageView,
    ScopeSlugView, StageModeView, StageName, StageSlugView, StateBinding, SteeringPartView,
    SteeringPlanView,
};

use crate::layout::Layout;

/// 行の値が公開言語の閉集合に無い (投影と描画の食い違い)。
///
/// 行は RMU が閉集合の綴りで書くので、ここへ来るのは投影が壊れているときだけである。
/// 材料 (どの列の、どの値か) だけを運び、文言は呼出側が組む。
fn unreadable_row(column: &str, value: &str) -> String {
    format!("Read model row is not readable: {column} = \"{value}\".")
}

/// 1 行 JSON の文字列配列を開く (`support_agents` / `consumes_rel` / … の列)。
fn strings(column: &str, encoded: &str) -> Result<Vec<String>, String> {
    let JsonValue::Array(items) = parse(encoded).map_err(|_| unreadable_row(column, encoded))?
    else {
        return Err(unreadable_row(column, encoded));
    };
    items
        .into_iter()
        .map(|item| match item {
            JsonValue::String(text) => Ok(text),
            _ => Err(unreadable_row(column, encoded)),
        })
        .collect()
}

/// `read_steering_part.rules_content` の `[{path, text}]` を開く。
pub(crate) fn rule_contents(encoded: &str) -> Result<Vec<RuleContent>, String> {
    let JsonValue::Array(items) =
        parse(encoded).map_err(|_| unreadable_row("rules_content", encoded))?
    else {
        return Err(unreadable_row("rules_content", encoded));
    };
    let mut contents = Vec::with_capacity(items.len());
    for item in items {
        let JsonValue::Object(members) = item else {
            return Err(unreadable_row("rules_content", encoded));
        };
        let (Some(JsonValue::String(path)), Some(JsonValue::String(text))) =
            (members.get("path"), members.get("text"))
        else {
            return Err(unreadable_row("rules_content", encoded));
        };
        contents.push(RuleContent::new(path.clone(), text.clone()));
    }
    Ok(contents)
}

/// 相対パスの列に基準を前置する。
fn under(base: &str, column: &str, encoded: &str) -> Result<Vec<String>, String> {
    Ok(strings(column, encoded)?
        .into_iter()
        .map(|relative| format!("{base}/{relative}"))
        .collect())
}

/// `read_run_stage` 1 行 + 要求のピンから `run-stage` を組む。
///
/// `gate` は**呼出側が決める** — 答えの行が `gated` を運ぶ分岐 (ハッピーパス) はその値、
/// 行を持たない分岐 (`--single` / state なし jump) は行の `gate_default` である。どちらも
/// 行の値であって、ここで計算するものではない。
///
/// # Errors
///
/// record が解決できない、または行の JSON 列が開けない (材料だけを運ぶ診断文言)。
pub(crate) fn run_stage(
    row: &RunStageView,
    layout: &Layout,
    gate: GateField,
    single: bool,
) -> Result<RunStageDirective, String> {
    let Some(record) = layout.record_dir() else {
        return Err("No workspace record was resolved for run-stage assembly.".to_string());
    };
    let record = record.to_string_lossy().into_owned();
    let harness = layout.harness_dir().to_string_lossy().into_owned();
    let stages = layout.stage_library_dir().to_string_lossy().into_owned();
    let slug = StageSlugView::parse(row.stage_slug())
        .map_err(|_| unreadable_row("stage_slug", row.stage_slug()))?;
    let phase = PhaseView::parse(row.phase()).map_err(|_| unreadable_row("phase", row.phase()))?;
    let mode = StageModeView::parse(row.mode()).map_err(|_| unreadable_row("mode", row.mode()))?;
    let mut builder = RunStageDirectiveBuilder::new(
        slug,
        phase,
        row.lead_agent(),
        mode,
        gate,
        format!("{stages}/{}", row.stage_file_rel()),
        format!("{record}/{}", row.memory_path_rel()),
    )
    .with_support_agents(strings("support_agents", row.support_agents())?)
    .with_inline_context_paths(under(
        &harness,
        "inline_context_paths_rel",
        row.inline_context_paths_rel(),
    )?)
    .with_consumes(under(&record, "consumes_rel", row.consumes_rel())?)
    .with_produces(under(&record, "produces_rel", row.produces_rel())?)
    .with_sensors(strings("sensors_applicable", row.sensors_applicable())?)
    .with_protocol_modules(strings("protocol_modules", row.protocol_modules())?);
    if let Some(name) = row.next_stage_name() {
        builder = builder.with_next_stage(name);
    }
    if let (Some(reviewer), Some(class)) = (row.reviewer(), row.review_class()) {
        let class =
            ReviewClassView::parse(class).map_err(|_| unreadable_row("review_class", class))?;
        builder =
            builder.with_reviewer(reviewer, class, row.reviewer_max_iterations().unwrap_or(1));
    }
    if single {
        builder = builder.with_single();
    }
    Ok(builder.build())
}

/// 束縛 (token に封じる 4 値) を行から組む。
///
/// 4 つとも行の列である — bundle は配信計画の行、directive と route は run-stage の行、
/// state は実行の行 (state なしの分岐では `None`)。
#[must_use]
pub(crate) fn bindings(
    run_stage: &RunStageView,
    plan: &SteeringPlanView,
    state: Option<&str>,
) -> Bindings {
    Bindings::new(
        BundleDigest::new(plan.bundle_digest()),
        DirectiveDigest::new(run_stage.directive_digest()),
        RouteDigest::new(run_stage.route_digest()),
        state.map(StateBinding::new),
    )
}

/// 連鎖 1 部 (`load-steering`) を描く。
///
/// # Errors
///
/// 行の `rules_content` が開けない。
pub(crate) fn load_steering(
    directive: &RunStageDirective,
    scope: &ScopeSlugView,
    plan: &SteeringPlanView,
    part: &SteeringPartView,
    bindings: &Bindings,
) -> Result<Directive, String> {
    let index = PartIndex::from_raw(part.part_index())
        .ok_or_else(|| unreadable_row("part_index", &part.part_index().to_string()))?;
    Ok(Directive::LoadSteering(LoadSteeringDirective::new(
        directive.stage().clone(),
        BundleDigest::new(plan.bundle_digest()),
        index,
        PartCount::new(plan.part_count()),
        rule_contents(part.rules_content())?,
        continue_token(directive, scope, index, bindings),
    )))
}

/// 次の `continue` に渡すトークンの中身 (封緘は [`crate::presenter`])。
fn continue_token(
    directive: &RunStageDirective,
    scope: &ScopeSlugView,
    part: PartIndex,
    bindings: &Bindings,
) -> ContinueToken {
    let mut builder = ContinueTokenBuilder::new(
        directive.stage().clone(),
        scope.clone(),
        part,
        bindings.clone(),
        directive.gate(),
    );
    if let Some(next_stage) = directive.next_stage()
        && let Ok(name) = StageName::parse(next_stage)
    {
        builder = builder.with_next_stage(name);
    }
    if let Some(unit) = directive.unit() {
        builder = builder.with_unit(unit.clone());
    }
    if directive.is_single() {
        builder = builder.with_single();
    }
    builder.build()
}

/// 配信済みルールのパス台帳 (`read_steering_plan.delivered_paths` の 1 行 JSON を開く)。
///
/// # Errors
///
/// 列が開けない。
pub(crate) fn delivered_paths(plan: &SteeringPlanView) -> Result<Vec<String>, String> {
    strings("delivered_paths", plan.delivered_paths())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

    use core_query_use_case::orchestration::{StageSlugView, UnitKind, UnitName, UnitRef};

    use super::*;

    /// 列の値が公開言語の閉集合に無いときの材料 (列名と値だけ)。
    fn refusal(column: &str, value: &str) -> String {
        format!("Read model row is not readable: {column} = \"{value}\".")
    }

    /// 配列でない 1 行 JSON は、列名と値を材料に拒む。
    #[test]
    fn a_json_column_that_is_not_an_array_is_refused_with_its_value() {
        assert_eq!(
            strings("support_agents", "\"orchestrator\""),
            Err(refusal("support_agents", "\"orchestrator\""))
        );
    }

    /// 配列でも要素が文字列でなければ拒む (列の型は文字列配列である)。
    #[test]
    fn a_json_array_of_non_strings_is_refused() {
        assert_eq!(strings("sensors", "[1]"), Err(refusal("sensors", "[1]")));
    }

    /// 文字列配列は基準を前置して絶対化される。
    #[test]
    fn a_relative_column_is_prefixed_with_the_base_it_belongs_to() {
        assert_eq!(
            under("/w/record", "consumes_rel", r#"["a.md","b/c.md"]"#),
            Ok(vec![
                "/w/record/a.md".to_string(),
                "/w/record/b/c.md".to_string()
            ])
        );
    }

    /// 部の本文も配列でなければ拒む。
    #[test]
    fn a_rules_content_column_that_is_not_an_array_is_refused() {
        assert_eq!(rule_contents("{}"), Err(refusal("rules_content", "{}")));
    }

    /// 要素がオブジェクトでない部の本文は拒む。
    #[test]
    fn a_rules_content_element_that_is_not_an_object_is_refused() {
        assert_eq!(
            rule_contents("[\"org.md\"]"),
            Err(refusal("rules_content", "[\"org.md\"]"))
        );
    }

    /// `path` と `text` が揃っていない部の本文は拒む。
    #[test]
    fn a_rules_content_object_without_both_members_is_refused() {
        let encoded = r#"[{"path":"org.md"}]"#;
        assert_eq!(
            rule_contents(encoded),
            Err(refusal("rules_content", encoded))
        );
    }

    /// 揃っていれば `path` と `text` の対をそのまま運ぶ。
    #[test]
    fn a_well_formed_rules_content_column_carries_each_path_and_text() {
        let contents =
            rule_contents(r#"[{"path":"memory/org.md","text":"規則。"}]"#).expect("開ける");

        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].path(), "memory/org.md");
        assert_eq!(contents[0].text(), "規則。");
    }

    /// `read_run_stage` の 1 行 (差し替えたい列だけを名指しで置き換える形)。
    struct Row {
        stage_slug: String,
        phase: String,
        mode: String,
        support_agents: String,
        inline_context_paths_rel: String,
        consumes_rel: String,
        produces_rel: String,
        sensors_applicable: String,
        protocol_modules: String,
        review_class: Option<String>,
    }

    impl Row {
        /// 全列が正しい 1 行。
        fn sound() -> Row {
            Row {
                stage_slug: "domain-design".to_string(),
                phase: "inception".to_string(),
                mode: "inline".to_string(),
                support_agents: "[]".to_string(),
                inline_context_paths_rel: "[]".to_string(),
                consumes_rel: "[]".to_string(),
                produces_rel: "[]".to_string(),
                sensors_applicable: "[]".to_string(),
                protocol_modules: "[]".to_string(),
                review_class: None,
            }
        }

        fn build(self) -> RunStageView {
            RunStageView::new(
                "row-1".to_string(),
                "claude".to_string(),
                "classic".to_string(),
                self.stage_slug,
                self.phase,
                "plan-1".to_string(),
                "orchestrator".to_string(),
                self.support_agents,
                self.mode,
                true,
                self.inline_context_paths_rel,
                "domain-design.md".to_string(),
                "inception/domain-design/memory.md".to_string(),
                self.consumes_rel,
                self.produces_rel,
                self.sensors_applicable,
                self.review_class.as_ref().map(|_| "reviewer".to_string()),
                Some(2),
                self.review_class,
                self.protocol_modules,
                Some("Contract Design".to_string()),
                "route".to_string(),
                "directive".to_string(),
            )
        }
    }

    /// 全列が正しい 1 行に、指定の 1 列だけ壊れた値を入れる。
    fn row(inline_context_paths_rel: &str) -> RunStageView {
        let mut sound = Row::sound();
        sound.inline_context_paths_rel = inline_context_paths_rel.to_string();
        sound.build()
    }

    /// record を指すカーソルを据えた配置。
    fn layout_with_record(root: &tempfile::TempDir) -> Layout {
        let layout = Layout::resolve(root.path());
        layout.point_at("260904-demo-abcd1234").expect("カーソル");
        Layout::resolve(root.path())
    }

    /// 相対パスの列が開けなければ run-stage を組まず、その列を材料に拒む。
    #[test]
    fn an_unreadable_relative_path_column_stops_the_run_stage_assembly() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let layout = layout_with_record(&root);

        let refused = run_stage(&row("not-json"), &layout, GateField::Gated, false)
            .expect_err("開けない列がある");

        assert_eq!(refused, refusal("inline_context_paths_rel", "not-json"));
    }

    /// 記録が解決できないターンでは run-stage を組めない。
    #[test]
    fn a_layout_without_a_record_cannot_assemble_a_run_stage() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let layout = Layout::resolve(root.path());

        assert_eq!(
            run_stage(&row("[]"), &layout, GateField::Gated, false),
            Err("No workspace record was resolved for run-stage assembly.".to_string())
        );
    }

    /// 閉集合の外にある列は、その列と値を材料に拒む (投影と描画の食い違い)。
    #[test]
    fn every_closed_vocabulary_column_is_refused_with_its_own_name() {
        /// 壊す 1 列を指す (列名・壊れた値・その列だけを差し替える手)。
        type BrokenColumn = (&'static str, &'static str, fn(&mut Row));

        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let layout = layout_with_record(&root);
        let cases: [BrokenColumn; 9] = [
            (
                "stage_slug",
                "Not A Slug",
                (|r: &mut Row| r.stage_slug = "Not A Slug".to_string()),
            ),
            (
                "phase",
                "elsewhere",
                (|r: &mut Row| r.phase = "elsewhere".to_string()),
            ),
            (
                "mode",
                "telepathy",
                (|r: &mut Row| r.mode = "telepathy".to_string()),
            ),
            (
                "support_agents",
                "{}",
                (|r: &mut Row| r.support_agents = "{}".to_string()),
            ),
            (
                "consumes_rel",
                "{}",
                (|r: &mut Row| r.consumes_rel = "{}".to_string()),
            ),
            (
                "produces_rel",
                "{}",
                (|r: &mut Row| r.produces_rel = "{}".to_string()),
            ),
            (
                "sensors_applicable",
                "{}",
                (|r: &mut Row| r.sensors_applicable = "{}".to_string()),
            ),
            (
                "protocol_modules",
                "{}",
                (|r: &mut Row| r.protocol_modules = "{}".to_string()),
            ),
            (
                "review_class",
                "harsh",
                (|r: &mut Row| r.review_class = Some("harsh".to_string())),
            ),
        ];

        for (column, value, break_it) in cases {
            let mut sound = Row::sound();
            break_it(&mut sound);
            assert_eq!(
                run_stage(&sound.build(), &layout, GateField::Gated, false),
                Err(refusal(column, value)),
                "{column} の壊れた値は列名ごと運ぶ"
            );
        }
    }

    /// 全列が正しければ、相対パスに基準を前置した run-stage が組み上がる。
    #[test]
    fn a_sound_row_assembles_a_run_stage_with_every_optional_field() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let layout = layout_with_record(&root);
        let mut sound = Row::sound();
        sound.review_class = Some("adversarial".to_string());
        sound.consumes_rel = r#"["inception/brief.md"]"#.to_string();

        let directive =
            run_stage(&sound.build(), &layout, GateField::Gated, true).expect("全列が正しい");

        assert_eq!(directive.stage().as_str(), "domain-design");
        assert_eq!(directive.next_stage(), Some("Contract Design"));
        assert!(directive.is_single());
        let consumes = directive.consumes();
        assert!(
            consumes[0].ends_with("/inception/brief.md"),
            "record を前置する: {consumes:?}"
        );
    }

    /// 部番号が 1 始まりでなければ、その列と値を材料に拒む。
    #[test]
    fn a_part_index_outside_the_one_based_range_is_refused() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let layout = layout_with_record(&root);
        let directive = run_stage(&row("[]"), &layout, GateField::Gated, false).expect("組める");
        let plan = SteeringPlanView::new(
            "plan-1".to_string(),
            "inception".to_string(),
            "bundle".to_string(),
            1,
            "[]".to_string(),
        );
        let part = SteeringPartView::new(
            "plan-1".to_string(),
            "inception".to_string(),
            0,
            "[]".to_string(),
        );
        let bindings = Bindings::new(
            BundleDigest::new("b"),
            DirectiveDigest::new("d"),
            RouteDigest::new("r"),
            None,
        );

        assert_eq!(
            load_steering(
                &directive,
                &ScopeSlugView::parse("classic").expect("scope"),
                &plan,
                &part,
                &bindings,
            ),
            Err(refusal("part_index", "0"))
        );
    }

    /// unit を伴う run-stage の継続トークンは、その unit を連鎖の先へ運ぶ。
    #[test]
    fn a_continue_token_carries_the_unit_the_directive_was_scoped_to() {
        let unit = UnitRef::new(
            UnitName::parse("u4-read-model-updater").expect("unit 名"),
            UnitKind::Service,
        );
        let directive = RunStageDirectiveBuilder::new(
            StageSlugView::parse("domain-design").expect("slug"),
            PhaseView::Inception,
            "orchestrator",
            StageModeView::Inline,
            GateField::Gated,
            "domain-design.md",
            "memory.md",
        )
        .with_unit(unit.clone())
        .build();
        let bindings = Bindings::new(
            BundleDigest::new("b"),
            DirectiveDigest::new("d"),
            RouteDigest::new("r"),
            None,
        );

        let token = continue_token(
            &directive,
            &ScopeSlugView::parse("classic").expect("scope"),
            PartIndex::from_raw(1).expect("1 始まり"),
            &bindings,
        );

        assert_eq!(token.unit(), Some(&unit));
    }
}
