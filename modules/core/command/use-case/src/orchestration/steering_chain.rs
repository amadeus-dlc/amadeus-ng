//! steering 連鎖の共有部品 — `next` と `continue` が同じ組み方でダイジェストと部を作る。
//!
//! 4 ダイジェスト束縛 (bundle / directive / route / state) の**素材文字列**はここが決める。
//! upstream は JSON 直列化のバイトを素材にするが、素材の形はエンジンローカルで、契約は
//! 「ドリフトしたら fail-closed」という**挙動**である (10 §11 — チャンク配送の詳細は
//! 未規範化。骨子 I12 に準拠)。ダイジェストの計算そのもの (sha256) は codec が持つ。

use core_command_domain::orchestration::{
    ContinueToken, ContinueTokenBuilder, Directive, LoadSteeringDirective, RunStageDirective,
};
use core_command_domain::workflow_definition::{StageNode, WorkflowDefinition};

use super::continue_token_codec::ContinueTokenCodec;
use super::steering::SteeringPlan;

/// 届けようとしている run-stage のダイジェスト素材 (キー項目の決定論的連結)。
pub(crate) fn directive_material(run_stage: &RunStageDirective) -> String {
    format!(
        "{}|{:?}|{}|{}|{}|{}|{}",
        run_stage.stage().as_str(),
        run_stage.gate(),
        run_stage.stage_file(),
        run_stage.memory_path(),
        run_stage.next_stage().unwrap_or("-"),
        run_stage.unit().unwrap_or("-"),
        run_stage.is_single(),
    )
}

/// グラフノードと scope メンバーシップのルートハッシュ素材。
pub(crate) fn route_material(
    definition: &WorkflowDefinition,
    scope: &str,
    node: &StageNode,
) -> String {
    let stages: Vec<&str> = definition
        .stages_in_scope(scope)
        .iter()
        .map(|(slug, _, _)| slug.as_str())
        .collect();
    format!("{}::{}", node.slug().as_str(), stages.join(","))
}

/// state 束縛の素材 (intent 識別子 + 集約の通番 + ストア採番の版)。
pub(crate) fn state_material(intent_id: &str, seq_nr: usize, version: usize) -> String {
    format!("{intent_id}::{seq_nr}::{version}")
}

/// 連鎖 1 部の発出 — `chunk_index` (0 始まり) の部と、次を指すトークンを封緘する。
///
/// `part <= parts` は [`LoadSteeringDirective::new`] が強制するが、呼出側は計画の範囲内の
/// 索引しか渡さない。範囲外は防御的に stale と同じ扱いで `error` にする。
#[allow(
    clippy::too_many_arguments,
    reason = "連鎖 1 部の材料は束縛 4 点 + 文脈 3 点そのもの — 束ねる中間型は複製にしかならない"
)]
pub(crate) fn emit_part<C: ContinueTokenCodec>(
    codec: &C,
    plan: &SteeringPlan,
    chunk_index: u32,
    run_stage: &RunStageDirective,
    scope: &str,
    bundle_digest: &str,
    directive_digest: &str,
    route_hash: &str,
    state_binding: Option<&str>,
) -> Directive {
    let Some(chunk) = plan.chunks().get(chunk_index as usize) else {
        return Directive::Error {
            message: "internal: a steering part outside the plan was requested".to_string(),
        };
    };
    let mut builder = ContinueTokenBuilder::new(
        run_stage.stage().as_str(),
        scope,
        chunk_index + 1,
        bundle_digest,
        directive_digest,
        route_hash,
        state_binding.unwrap_or("-"),
        run_stage.gate(),
    );
    if state_binding.is_none() {
        builder = builder.without_state_binding();
    }
    if let Some(next_stage) = run_stage.next_stage() {
        builder = builder.with_next_stage(next_stage);
    }
    if let (Some(unit), true) = (run_stage.unit(), run_stage.unit().is_some()) {
        builder = builder.with_unit(unit, "-");
    }
    if run_stage.is_single() {
        builder = builder.with_single();
    }
    let token = builder.build();
    match LoadSteeringDirective::new(
        run_stage.stage().clone(),
        bundle_digest,
        chunk_index + 1,
        plan.parts(),
        chunk.clone(),
        codec.mint(&token),
    ) {
        Ok(part) => Directive::LoadSteering(part),
        Err(error) => Directive::Error {
            message: format!("internal: {error}"),
        },
    }
}

/// 最終の run-stage — 配信済みルール束のパス台帳を載せる。
pub(crate) fn finalize_run_stage(plan: &SteeringPlan, run_stage: &RunStageDirective) -> Directive {
    let mut paths: Vec<String> = Vec::new();
    for chunk in plan.chunks() {
        for piece in chunk {
            if !paths.iter().any(|path| path == piece.path()) {
                paths.push(piece.path().to_string());
            }
        }
    }
    let rebuilt = clone_with_rules(run_stage, paths);
    Directive::RunStage(rebuilt)
}

/// `rules_in_context` を載せ替えた複製。
fn clone_with_rules(run_stage: &RunStageDirective, paths: Vec<String>) -> RunStageDirective {
    // RunStageDirective は不変なので、ビルダーを経由せずフィールド単位の再構成 API を
    // 持たない。台帳だけを差し替えるため、ビルダーで同じ形を組み直す。
    let mut builder = core_command_domain::orchestration::RunStageDirectiveBuilder::new(
        run_stage.stage().clone(),
        run_stage.phase(),
        run_stage.lead_agent(),
        run_stage.mode(),
        run_stage.gate(),
        run_stage.stage_file(),
        run_stage.memory_path(),
    )
    .with_support_agents(run_stage.support_agents().to_vec())
    .with_inline_context_paths(run_stage.inline_context_paths().to_vec())
    .with_consumes(run_stage.consumes().to_vec())
    .with_produces(run_stage.produces().to_vec())
    .with_sensors(run_stage.sensors_applicable().to_vec())
    .with_protocol_modules(run_stage.protocol_modules().to_vec())
    .with_rules_in_context(paths);
    if let Some(next_stage) = run_stage.next_stage() {
        builder = builder.with_next_stage(next_stage);
    }
    if let (Some(reviewer), Some(class)) = (run_stage.reviewer(), run_stage.review_class()) {
        builder = builder.with_reviewer(
            reviewer,
            class,
            run_stage.reviewer_max_iterations().unwrap_or(1),
        );
    }
    if let Some(narration) = run_stage.narration() {
        builder = builder.with_narration(narration);
    }
    if let Some(unit) = run_stage.unit() {
        builder = builder.with_unit(unit);
    }
    if run_stage.is_single() {
        builder = builder.with_single();
    }
    builder.build()
}

/// トークンのピン (`gate` / `next_stage` / `unit` / `single`) を再構築した run-stage へ
/// 再適用する (再構築原則 `:5996-6037` — キャッシュを信用せず、ピンだけを引き継ぐ)。
pub(crate) fn rebuild_with_pins(
    run_stage: &RunStageDirective,
    token: &ContinueToken,
) -> RunStageDirective {
    let mut builder = core_command_domain::orchestration::RunStageDirectiveBuilder::new(
        run_stage.stage().clone(),
        run_stage.phase(),
        run_stage.lead_agent(),
        run_stage.mode(),
        token.gate(),
        run_stage.stage_file(),
        run_stage.memory_path(),
    )
    .with_support_agents(run_stage.support_agents().to_vec())
    .with_inline_context_paths(run_stage.inline_context_paths().to_vec())
    .with_consumes(run_stage.consumes().to_vec())
    .with_produces(run_stage.produces().to_vec())
    .with_sensors(run_stage.sensors_applicable().to_vec())
    .with_protocol_modules(run_stage.protocol_modules().to_vec())
    .with_rules_in_context(run_stage.rules_in_context().to_vec());
    if let Some(next_stage) = token.next_stage() {
        builder = builder.with_next_stage(next_stage);
    }
    if let (Some(reviewer), Some(class)) = (run_stage.reviewer(), run_stage.review_class()) {
        builder = builder.with_reviewer(
            reviewer,
            class,
            run_stage.reviewer_max_iterations().unwrap_or(1),
        );
    }
    if let Some(narration) = run_stage.narration() {
        builder = builder.with_narration(narration);
    }
    if let Some(unit) = token.unit() {
        builder = builder.with_unit(unit);
    }
    if token.is_single() {
        builder = builder.with_single();
    }
    builder.build()
}
