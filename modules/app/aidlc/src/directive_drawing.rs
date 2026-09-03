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
