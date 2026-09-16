//! 群 A — workflow authority の読取サブコマンドの配線（`lookup` / `scope-table` / `stage-table`）。
//!
//! すべて読取専用なので、コンパイル済み定義（`stage-graph.json` / `scope-grid.json` /
//! `scopes/aidlc-*.md`）というリードモデルをクエリ側の DAO で読み、クエリユースケースが答えを
//! 組み、ここ（合成ルート = プレゼンタ）が plain / JSON / Markdown 表へ描く
//! (`coding-rules/cqrs-boundaries.md` 規則 6/7)。判断・導出はユースケース側にあり、ここは
//! View の列を選んで綴るだけである。
//!
//! # 出口の 2 層
//!
//! 読取に成功したら **stdout・exit 0**（[`Completion::emitted`] — 末尾改行は `main.rs` の
//! `writeln!` が付すので payload には含めない）。未知 slug・引数不足・未配線サブ動詞・読取失敗は
//! **stderr・exit 1**（[`Completion::refused`]）。これは upstream の `console.log` / `error()` の
//! 2 層に対応する（[`crate::presenter`] のモジュール doc）。

use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_interface_adapter::{
    ScopeGridDaoImpl, ScopeMetadataDaoImpl, StageGraphDaoImpl, StateFileDaoImpl,
};
use core_query_use_case::orchestration::{
    FindNextInScopeStageUseCase, ListScopeCatalogUseCase, ListStageGraphUseCase,
    ReadModelReadError, ResolveStageUseCase, ScopeCatalogRowView, StageGraphEntryView,
};

use crate::layout::Layout;
use crate::wording;

use super::Completion;

/// `aidlc-state lookup <sub> [args...]` を構文的にルーティングする。
///
/// どの DAO をどの鍵で引くかの分岐はコントローラ（ここ）の構文的ルーティングであり、状態の値で
/// 決まる分岐は持たない (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。
pub(super) fn lookup(layout: &Layout, sub: Option<&str>, args: &[String]) -> Completion {
    match sub {
        Some("phase-of") => phase_of(layout, args),
        Some("agent-for") => agent_for(layout, args),
        Some("validate-stage") => validate_stage(layout, args),
        Some("next-stage") => next_stage(layout, args),
        Some(other) => Completion::refused(wording::lookup_subcommand_not_wired(other)),
        None => Completion::refused(wording::LOOKUP_USAGE.to_string()),
    }
}

/// `lookup phase-of <slug>` — 解決したステージの phase を plain 出力する。
fn phase_of(layout: &Layout, args: &[String]) -> Completion {
    let Some(slug) = args.first() else {
        return Completion::refused(wording::lookup_subcommand_usage("phase-of <slug>"));
    };
    match resolve(layout, slug) {
        Ok(Some(stage)) => Completion::emitted(stage.phase().to_string()),
        Ok(None) => Completion::refused(wording::state_unknown_stage(slug)),
        Err(error) => Completion::refused(read_failure(&error)),
    }
}

/// `lookup agent-for <slug>` — 解決したステージの lead_agent を plain 出力する。
fn agent_for(layout: &Layout, args: &[String]) -> Completion {
    let Some(slug) = args.first() else {
        return Completion::refused(wording::lookup_subcommand_usage("agent-for <slug>"));
    };
    match resolve(layout, slug) {
        Ok(Some(stage)) => Completion::emitted(stage.lead_agent().to_string()),
        Ok(None) => Completion::refused(wording::state_unknown_stage(slug)),
        Err(error) => Completion::refused(read_failure(&error)),
    }
}

/// `lookup validate-stage <slug-or-number>` — JSON を plain 出力する（不在も exit 0）。
fn validate_stage(layout: &Layout, args: &[String]) -> Completion {
    let Some(input) = args.first() else {
        return Completion::refused(wording::lookup_subcommand_usage(
            "validate-stage <slug-or-number>",
        ));
    };
    match resolve(layout, input) {
        Ok(Some(stage)) => Completion::emitted(render_valid_stage(&stage)),
        Ok(None) => Completion::emitted(render_invalid_stage(input)),
        Err(error) => Completion::refused(read_failure(&error)),
    }
}

/// `lookup next-stage <slug> <scope>` — 次の in-scope ステージ slug か `none` を plain 出力する。
fn next_stage(layout: &Layout, args: &[String]) -> Completion {
    let (Some(after), Some(scope)) = (args.first(), args.get(1)) else {
        return Completion::refused(wording::lookup_subcommand_usage(
            "next-stage <slug> <scope>",
        ));
    };
    let use_case = FindNextInScopeStageUseCase::new(
        StageGraphDaoImpl::new(&layout.definition_data_dir()),
        ScopeGridDaoImpl::new(&layout.definition_data_dir()),
        StateFileDaoImpl::new(&state_file_probe(layout)),
    );
    match use_case.execute(after, scope) {
        Ok(Some(slug)) => Completion::emitted(slug),
        Ok(None) => Completion::emitted("none".to_string()),
        Err(error) => Completion::refused(read_failure(&error)),
    }
}

/// `aidlc-utility scope-table` — scope グリッドの Markdown 表を plain 出力する。
pub(super) fn scope_table(layout: &Layout) -> Completion {
    let use_case = ListScopeCatalogUseCase::new(
        ScopeMetadataDaoImpl::new(&layout.scopes_dir()),
        ScopeGridDaoImpl::new(&layout.definition_data_dir()),
    );
    match use_case.execute() {
        Ok(rows) => Completion::emitted(render_scope_table(&rows)),
        Err(error) => Completion::refused(read_failure(&error)),
    }
}

/// `aidlc-utility stage-table` — stage グラフの Markdown 表を plain 出力する。
pub(super) fn stage_table(layout: &Layout) -> Completion {
    let use_case =
        ListStageGraphUseCase::new(StageGraphDaoImpl::new(&layout.definition_data_dir()));
    match use_case.execute() {
        Ok(stages) => Completion::emitted(render_stage_table(&stages)),
        Err(error) => Completion::refused(read_failure(&error)),
    }
}

/// slug/番号で 1 ステージを引く共通経路。
fn resolve(
    layout: &Layout,
    slug_or_number: &str,
) -> Result<Option<StageGraphEntryView>, ReadModelReadError> {
    ResolveStageUseCase::new(StageGraphDaoImpl::new(&layout.definition_data_dir()))
        .execute(slug_or_number)
}

/// 状態ファイルの所在（record がまだ無ければ存在しないパスを渡す — `find` は不在を `None` で返す）。
fn state_file_probe(layout: &Layout) -> std::path::PathBuf {
    layout
        .state_file()
        .unwrap_or_else(|| layout.project_dir().join(".aidlc-absent-state"))
}

/// リードモデルの読取失敗の診断（材料だけ — 包み方は共通経路）。
fn read_failure(error: &ReadModelReadError) -> String {
    wording::orchestrate_failure(&format!("cannot read the compiled definition: {error}"))
}

/// `validate-stage` の valid JSON（列順 `valid,slug,number,name,phase,lead_agent` を逐語で守る）。
///
/// 契約 JSON の直列化は canon-json の 1 経路に固定されている (BR1.7 / ADR 0001 決定 5)。
/// `ContractCompact` は空白なし・挿入順で、upstream の `JSON.stringify` とバイト一致する。
fn render_valid_stage(stage: &StageGraphEntryView) -> String {
    let mut object = ObjectMembers::new();
    object.insert("valid", JsonValue::Bool(true));
    object.insert("slug", JsonValue::String(stage.slug().to_string()));
    object.insert("number", JsonValue::String(stage.number().to_string()));
    object.insert("name", JsonValue::String(stage.name().to_string()));
    object.insert("phase", JsonValue::String(stage.phase().to_string()));
    object.insert(
        "lead_agent",
        JsonValue::String(stage.lead_agent().to_string()),
    );
    serialize(
        &JsonValue::Object(object),
        SerializationProfile::ContractCompact,
    )
}

/// `validate-stage` の invalid JSON（列順 `valid,input`）。
fn render_invalid_stage(input: &str) -> String {
    let mut object = ObjectMembers::new();
    object.insert("valid", JsonValue::Bool(false));
    object.insert("input", JsonValue::String(input.to_string()));
    serialize(
        &JsonValue::Object(object),
        SerializationProfile::ContractCompact,
    )
}

const SCOPE_TABLE_BEGIN: &str =
    "<!-- BEGIN: compiled scope grid via `bun aidlc-utility.ts scope-table` - do NOT hand-edit -->";
const SCOPE_TABLE_END: &str = "<!-- END: compiled scope grid -->";
const STAGE_TABLE_BEGIN: &str = "<!-- BEGIN: compiled stage graph via `bun aidlc-utility.ts stage-table` - do NOT hand-edit -->";
const STAGE_TABLE_END: &str = "<!-- END: compiled stage graph -->";

/// `scope-table` の正準バイト形 `BEGIN\n\n<table>\n\nEND`（末尾改行は付けない）。
fn render_scope_table(rows: &[ScopeCatalogRowView]) -> String {
    let mut lines = vec![
        "| Scope          | Depth         | TestStrategy | EXECUTE / Total |".to_string(),
        "|----------------|---------------|--------------|-----------------|".to_string(),
    ];
    for row in rows {
        let test_strategy = row.test_strategy().unwrap_or("(default)");
        let execute_total = format!("{} / {}", row.execute(), row.total());
        lines.push(format!(
            "| {:<14} | {:<13} | {:<12} | {:<15} |",
            row.scope(),
            row.depth(),
            test_strategy,
            execute_total,
        ));
    }
    format!(
        "{SCOPE_TABLE_BEGIN}\n\n{}\n\n{SCOPE_TABLE_END}",
        lines.join("\n")
    )
}

/// `stage-table` の正準バイト形 `BEGIN\n\n<table>\n\nEND`（末尾改行は付けない）。
fn render_stage_table(stages: &[StageGraphEntryView]) -> String {
    let mut lines = vec![
        "| Slug | # | Stage | Phase | Execution | Lead Agent | Support Agents | Mode |".to_string(),
        "|------|---|-------|-------|-----------|------------|----------------|------|".to_string(),
    ];
    for stage in stages {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            stage.slug(),
            stage.number(),
            stage.name(),
            display_phase(stage.phase()),
            stage.execution(),
            display_lead_agent(stage.lead_agent()),
            display_support_agents(stage.support_agents()),
            stage.mode(),
        ));
    }
    format!(
        "{STAGE_TABLE_BEGIN}\n\n{}\n\n{STAGE_TABLE_END}",
        lines.join("\n")
    )
}

/// フェーズ名の先頭 1 文字だけ大文字にする（upstream `displayPhase`）。
fn display_phase(phase: &str) -> String {
    let mut chars = phase.chars();
    chars.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_uppercase(), chars.as_str())
    })
}

/// `orchestrator` は `(orchestrator)` と描く（upstream `displayLeadAgent`）。
fn display_lead_agent(agent: &str) -> String {
    if agent == "orchestrator" {
        "(orchestrator)".to_string()
    } else {
        agent.to_string()
    }
}

/// 支援エージェントは `, ` 連結、無ければ `—`（upstream `displaySupportAgents`）。
fn display_support_agents(agents: &[String]) -> String {
    if agents.is_empty() {
        "\u{2014}".to_string()
    } else {
        agents.join(", ")
    }
}
