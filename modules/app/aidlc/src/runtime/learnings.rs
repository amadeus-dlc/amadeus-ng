//! §13 の学びの儀式 — 候補の提示 (`surface`) と正本への書込み (`persist`)。
//!
//! 固定本家 2.7.1 `a277af21` の `tools/aidlc-learnings.ts`（surface は `:293-374`、
//! persist は `:684-948`）と `aidlc-common/protocols/stage-protocol.md:1055-1110` に対応する。
//!
//! # 2 つの動詞の非対称
//!
//! `surface` は**読むだけ**である — 状態ファイル・runtime-graph の stage 行・ステージ日誌を
//! 読み、候補と据え置きの質問を JSON で出す。`Open questions` は候補へ**昇格しない**（別配列）。
//!
//! `persist` は**書く**。書込みはコマンド → イベント → SQLite → RMU を通り、実践行と監査行
//! `RULE_LEARNED` は投影が描く。重複抑止の材料（監査シャードとメモリ層）は人も編集する面
//! なので、書込み直前にディスクを実測して集約へ渡す。
//!
//! # 素性は surface の時点で固定する
//!
//! 選択ファイルが運ぶ `space` / `intent` を**そのまま**使い、実行時のカーソルを読み直さない。
//! surface と persist のあいだに利用者が別の作業へ移っても、書込みは並べた時点の作業へ落ちる。
//!
//! # この build で範囲外のもの
//!
//! センサーの生成と stage frontmatter への束ね（本家 `:874` 以降）、記録に結び付かない
//! 平置きレイアウト（`intent: null`）は配線していない。どちらも黙って飛ばさず自己防衛拒否する。

use super::{Completion, Layout, Utc};
use crate::cli::LearningsArgs;
use crate::wording;
use core_command_domain::orchestration::{
    Learning, LearningCandidateId, LearningObservation, LearningObservations, LearningProvenance,
    LearningScope, LearningSource, MemoryEntries, PracticeHeading,
};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{IntentDirName, SpaceName};
use core_command_interface_adapter::orchestration::IntentExecutionRepositoryImpl;
use core_command_use_case::orchestration::CaptureLearningsUseCase;
use core_infrastructure::canon_json::{SerializationProfile, serialize, to_value};
use std::path::{Path, PathBuf};

/// 監査台帳のブロック区切り。
const BLOCK_SEPARATOR: &str = "\n---\n";
/// 学びの監査行のイベント名。
const RULE_LEARNED_LINE: &str = "**Event**: RULE_LEARNED";
/// 候補が既定で落ちる層の綴り（本家の `default_scope`）。
const DEFAULT_SCOPE: &str = "project";

// ---------------------------------------------------------------------------
// surface — 候補を並べる（読むだけ）
// ---------------------------------------------------------------------------

/// surface が出す 1 件の候補。キーの並びが契約である。
#[derive(Debug, serde::Serialize)]
struct SurfaceCandidate {
    id: String,
    source_heading: &'static str,
    ts: String,
    summary: String,
    context: String,
    default_scope: &'static str,
}

/// 据え置きの質問（候補へ昇格しない）。
#[derive(Debug, serde::Serialize)]
struct SurfaceParkedQuestion {
    ts: String,
    summary: String,
}

/// surface の出力全体。
#[derive(Debug, serde::Serialize)]
struct SurfaceOutput {
    schema_version: u8,
    stage_slug: String,
    phase: String,
    space: String,
    intent: String,
    memory_entries_total: usize,
    candidates: Vec<SurfaceCandidate>,
    parked_open_questions: Vec<SurfaceParkedQuestion>,
}

pub(super) async fn surface(layout: &Layout, args: &LearningsArgs) -> Completion {
    let Some(slug) = args.slug() else {
        return Completion::refused(wording::LEARNINGS_SURFACE_USAGE.to_string());
    };
    let Some(record) = layout.record_dir().map(Path::to_path_buf) else {
        return Completion::refused(no_record_refusal(layout));
    };
    // 状態ファイルは投影の産物である — 読む前に追いつかせる。
    if let Err(message) = super::update_read_models_before_reading(layout).await {
        return Completion::refused(message);
    }
    let Some(state_path) = layout.state_file() else {
        return Completion::refused(no_record_refusal(layout));
    };
    let state = match std::fs::read_to_string(&state_path) {
        Ok(state) => state,
        Err(error) => {
            return Completion::refused(wording::learnings_unreadable_state(&error.to_string()));
        }
    };
    let Some(current) = state_field(&state, "Current Stage") else {
        return Completion::refused(wording::LEARNINGS_NO_CURRENT_STAGE.to_string());
    };
    if current != slug {
        return Completion::refused(wording::learnings_slug_is_not_current(slug, &current));
    }
    let memory_path = match runtime_graph_memory_path(&record, slug) {
        Ok(path) => path,
        Err(message) => return Completion::refused(message),
    };
    let raw = std::fs::read_to_string(layout.project_dir().join(&memory_path)).unwrap_or_default();
    let entries = MemoryEntries::parse(&raw);
    let (candidates, parked) = entries.fold_left(
        (Vec::new(), Vec::new()),
        |(mut candidates, mut parked): (Vec<SurfaceCandidate>, Vec<SurfaceParkedQuestion>),
         entry| {
            if entry.is_parked() {
                parked.push(SurfaceParkedQuestion {
                    ts: entry.timestamp().to_string(),
                    summary: entry.summary().to_string(),
                });
            } else {
                candidates.push(SurfaceCandidate {
                    id: format!("c{}", candidates.len().saturating_add(1)),
                    source_heading: entry.heading().as_str(),
                    ts: entry.timestamp().to_string(),
                    summary: entry.summary().to_string(),
                    context: entry.context().to_string(),
                    default_scope: DEFAULT_SCOPE,
                });
            }
            (candidates, parked)
        },
    );
    let output = SurfaceOutput {
        schema_version: 1,
        stage_slug: slug.to_string(),
        phase: phase_of(&memory_path),
        space: layout.space().to_string(),
        intent: record
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        memory_entries_total: entries.len(),
        candidates,
        parked_open_questions: parked,
    };
    match to_value(&output) {
        Ok(value) => Completion::emitted(serialize(&value, SerializationProfile::ContractCompact)),
        Err(error) => Completion::refused(wording::orchestrate_failure(&error.to_string())),
    }
}

/// `memory_path` は `<前置>/<phase>/<stage>/memory.md` — phase は後ろから 3 番目である。
fn phase_of(memory_path: &str) -> String {
    let segments: Vec<&str> = memory_path.split('/').collect();
    segments
        .len()
        .checked_sub(3)
        .and_then(|index| segments.get(index))
        .copied()
        .unwrap_or_default()
        .to_string()
}

/// 記録が解決できないときの拒否 — 「複数あってカーソルが無い」だけは本家の文言を出す。
fn no_record_refusal(layout: &Layout) -> String {
    if record_dirs(layout).is_empty() {
        wording::LEARNINGS_WITHOUT_INTENT.to_string()
    } else {
        wording::learnings_ambiguous_intent(layout.space())
    }
}

/// space 配下の記録ディレクトリ名（存在するものだけ）。
fn record_dirs(layout: &Layout) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(layout.intents_dir()) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .collect()
}

/// `<record>/runtime-graph.json` からその stage の `memory_path` を読む。
fn runtime_graph_memory_path(record: &Path, slug: &str) -> Result<String, String> {
    let path = record.join("runtime-graph.json");
    if !path.exists() {
        return Err(wording::learnings_runtime_graph_missing(
            &path.to_string_lossy(),
        ));
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| wording::learnings_runtime_graph_malformed(&error.to_string()))?;
    let graph: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| wording::learnings_runtime_graph_malformed(&error.to_string()))?;
    let stages = graph
        .get("stages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| wording::LEARNINGS_RUNTIME_GRAPH_NO_STAGES.to_string())?;
    let row = stages
        .iter()
        .find(|row| row.get("stage_slug").and_then(serde_json::Value::as_str) == Some(slug))
        .ok_or_else(|| wording::learnings_stage_not_in_graph(slug))?;
    row.get("memory_path")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| wording::learnings_stage_without_memory_path(slug))
}

/// 状態ファイルの `- **<name>**: <value>` を読む。
fn state_field(state: &str, name: &str) -> Option<String> {
    let prefix = format!("- **{name}**:");
    state
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(|value| value.trim().to_string())
}

// ---------------------------------------------------------------------------
// persist — 確定した学びを正本へ書く
// ---------------------------------------------------------------------------

/// persist の出力。キーの並びが契約である。
#[derive(Debug, serde::Serialize)]
struct PersistOutput {
    stage_slug: String,
    rule_learned: usize,
    sensor_proposed: usize,
    notes: Vec<String>,
}

/// 選択ファイルの中身（surface の時点で固定された素性を含む）。
struct Selections {
    stage: StageSlug,
    space: SpaceName,
    intent: IntentDirName,
    learnings: Vec<Learning>,
}

pub(super) async fn persist(layout: &Layout, args: &LearningsArgs) -> Completion {
    let Some(selections_json) = args.selections_json() else {
        return Completion::refused(wording::LEARNINGS_PERSIST_USAGE.to_string());
    };
    let selections = match read_selections(Path::new(selections_json)) {
        Ok(selections) => selections,
        Err(message) => return Completion::refused(message),
    };
    if let Some(slug) = args.slug()
        && slug != selections.stage.as_str()
    {
        return Completion::refused(wording::learnings_persist_slug_mismatch(
            selections.stage.as_str(),
            slug,
        ));
    }
    // 固定した素性で配置を組み直す — 実行時のカーソルは読まない。
    let pinned = Layout::for_record(layout.project_dir(), &selections.space, &selections.intent);
    if let Err(message) = verify_pinned_record(&pinned, &selections) {
        return Completion::refused(message);
    }
    let lock_file = pinned.aidlc_root().join(".aidlc-learnings.lock");
    let _lock = match acquire_lock(&lock_file) {
        Ok(lock) => lock,
        Err(message) => return Completion::refused(wording::learnings_persist_failed(&message)),
    };
    // 実測の前に投影を追いつかせる — 未投影の書込みを「無い」と読むと二重に書く。
    if let Err(message) = super::update_read_models_before_reading(&pinned).await {
        return Completion::refused(message);
    }
    let observations = match observe(&pinned, &selections) {
        Ok(observations) => observations,
        Err(message) => return Completion::refused(message),
    };
    let rule_learned = observations.captured().audit_rows();
    if let Err(message) = store(&pinned, &selections, &observations).await {
        return Completion::refused(message);
    }
    let output = PersistOutput {
        stage_slug: selections.stage.as_str().to_string(),
        rule_learned,
        sensor_proposed: 0,
        notes: Vec::new(),
    };
    super::after_projection(&pinned, || match to_value(&output) {
        Ok(value) => Completion::emitted(serialize(&value, SerializationProfile::ContractCompact)),
        Err(error) => Completion::refused(wording::orchestrate_failure(&error.to_string())),
    })
    .await
}

/// 監査台帳の追記を直列化する（本家の `withAuditLock` に対応する区間）。
fn acquire_lock(path: &Path) -> Result<core_infrastructure::ExclusiveFileLock, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|error| error.to_string())?;
    core_infrastructure::ExclusiveFileLock::acquire(file, std::time::Duration::from_secs(5))
        .map_err(|error| error.to_string())
}

/// 固定した space と記録が**いまも在る**ことを確かめる（fail-closed）。
fn verify_pinned_record(pinned: &Layout, selections: &Selections) -> Result<(), String> {
    let space_dir = pinned
        .aidlc_root()
        .join("spaces")
        .join(selections.space.as_str());
    if !space_dir.is_dir() {
        return Err(wording::learnings_missing_space(selections.space.as_str()));
    }
    let missing_record = pinned.record_dir().is_none_or(|dir| !dir.is_dir())
        || pinned.state_file().is_none_or(|file| !file.is_file());
    if missing_record {
        return Err(wording::learnings_missing_intent(
            selections.intent.as_str(),
            selections.space.as_str(),
        ));
    }
    for scope in [LearningScope::Project, LearningScope::Team] {
        let path = pinned.memory_dir().join(scope.method_file());
        if !path.is_file() {
            return Err(wording::learnings_method_files_missing(
                &path.to_string_lossy(),
            ));
        }
    }
    Ok(())
}

/// 監査シャードとメモリ層を実測して、学びごとの両側の在否を組む。
fn observe(pinned: &Layout, selections: &Selections) -> Result<LearningObservations, String> {
    let audit = read_audit(pinned);
    let provenance = LearningProvenance::new(selections.space.clone(), selections.intent.clone());
    let mut faces: Vec<(LearningScope, String)> = Vec::new();
    for scope in [LearningScope::Project, LearningScope::Team] {
        let path = pinned.memory_dir().join(scope.method_file());
        let content = std::fs::read_to_string(&path)
            .map_err(|error| wording::learnings_persist_failed(&error.to_string()))?;
        faces.push((scope, content));
    }
    let observations = selections
        .learnings
        .iter()
        .map(|learning| {
            let marker = learning.marker(&provenance, &selections.stage);
            let present = faces
                .iter()
                .find(|(scope, _)| *scope == learning.scope())
                .is_some_and(|(_, content)| content.contains(&marker));
            LearningObservation::new(
                learning.clone(),
                audit_carries_rule(&audit, selections.stage.as_str(), learning),
                present,
            )
        })
        .collect();
    Ok(LearningObservations::new(observations))
}

/// 監査シャードをファイル名順に連結する。
fn read_audit(pinned: &Layout) -> String {
    let Some(dir) = pinned.audit_dir() else {
        return String::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return String::new();
    };
    let mut shards: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    shards.sort();
    shards.iter().fold(String::new(), |mut buffer, shard| {
        if let Ok(text) = std::fs::read_to_string(shard) {
            buffer.push_str(&text.replace("\r\n", "\n"));
        }
        buffer
    })
}

/// 同じ (Stage, Content-Hash) の `RULE_LEARNED` が既に在るか。
fn audit_carries_rule(audit: &str, stage: &str, learning: &Learning) -> bool {
    let stage_line = format!("**Stage**: {stage}");
    let hash_line = format!("**Content-Hash**: {}", learning.content_hash().as_str());
    audit.split(BLOCK_SEPARATOR).any(|block| {
        let mut has_event = false;
        let mut has_stage = false;
        let mut has_hash = false;
        for line in block.lines() {
            has_event |= line == RULE_LEARNED_LINE;
            has_stage |= line.trim_end() == stage_line;
            has_hash |= line.trim_end() == hash_line;
        }
        has_event && has_stage && has_hash
    })
}

/// コマンド → イベント → SQLite。
async fn store(
    pinned: &Layout,
    selections: &Selections,
    observations: &LearningObservations,
) -> Result<(), String> {
    let store = super::store_path(pinned)?;
    let execution_id = match super::active_execution(pinned) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        Ok(None) => {
            return Err(wording::learnings_missing_intent(
                selections.intent.as_str(),
                selections.space.as_str(),
            ));
        }
        Err(error) => {
            return Err(wording::unreadable_execution_cursor(&error.to_string()));
        }
    };
    let repository = IntentExecutionRepositoryImpl::open(&store)
        .map_err(|error| wording::learnings_persist_failed(&error.to_string()))?;
    CaptureLearningsUseCase::new(repository)
        .execute(
            &execution_id,
            &selections.stage,
            LearningProvenance::new(selections.space.clone(), selections.intent.clone()),
            observations,
            Utc::now(),
        )
        .await
        .map_err(|error| wording::learnings_persist_failed(&super::chained(&error)))
}

/// 選択ファイルを読む。素性は**推測しない** — 欄が無ければ拒否する。
fn read_selections(path: &Path) -> Result<Selections, String> {
    if !path.exists() {
        return Err(wording::learnings_selections_not_found(
            &path.to_string_lossy(),
        ));
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|error| wording::learnings_selections_malformed(&error.to_string()))?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| wording::learnings_selections_malformed(&error.to_string()))?;
    let Some(object) = parsed.as_object() else {
        return Err(wording::LEARNINGS_SELECTIONS_SHAPE.to_string());
    };
    let stage = object
        .get("stage_slug")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| wording::LEARNINGS_SELECTIONS_SHAPE.to_string())?;
    let stage =
        StageSlug::parse(stage).map_err(|_| wording::LEARNINGS_SELECTIONS_SHAPE.to_string())?;
    let space = object
        .get("space")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| wording::LEARNINGS_SELECTIONS_NO_SPACE.to_string())?;
    let space =
        SpaceName::parse(space).map_err(|_| wording::LEARNINGS_SELECTIONS_BAD_SPACE.to_string())?;
    let intent = match object.get("intent") {
        // 記録に結び付かない選択はこの build に無い（平置きレイアウトを持たない）。
        Some(serde_json::Value::Null) => {
            return Err(wording::LEARNINGS_UNSCOPED_NOT_WIRED.to_string());
        }
        Some(serde_json::Value::String(intent)) => intent.clone(),
        // 欄が無い場合も本家と同じ「文字列か null」の拒否へ倒す。
        None | Some(_) => return Err(wording::LEARNINGS_SELECTIONS_BAD_INTENT_TYPE.to_string()),
    };
    let intent = IntentDirName::parse(&intent)
        .map_err(|_| wording::LEARNINGS_SELECTIONS_BAD_INTENT.to_string())?;
    let rows = object
        .get("selections")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| wording::LEARNINGS_SELECTIONS_SHAPE.to_string())?;
    let mut learnings = Vec::with_capacity(rows.len());
    for row in rows {
        learnings.push(read_learning(row)?);
    }
    Ok(Selections {
        stage,
        space,
        intent,
        learnings,
    })
}

/// 選択 1 件を読む。センサーの選択はこの build に無い（自己防衛拒否）。
fn read_learning(row: &serde_json::Value) -> Result<Learning, String> {
    let Some(object) = row.as_object() else {
        return Err(wording::LEARNINGS_SELECTION_NOT_OBJECT.to_string());
    };
    let candidate = object
        .get("candidate_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| wording::LEARNINGS_SELECTION_NO_CANDIDATE_ID.to_string())?;
    if object.get("type").and_then(serde_json::Value::as_str) == Some("sensor") {
        return Err(wording::LEARNINGS_SENSOR_NOT_WIRED.to_string());
    }
    let (Some(heading), Some(text)) = (
        object.get("heading").and_then(serde_json::Value::as_str),
        object.get("text").and_then(serde_json::Value::as_str),
    ) else {
        return Err(wording::LEARNINGS_SELECTION_NO_HEADING_OR_TEXT.to_string());
    };
    let candidate_id = LearningCandidateId::parse(candidate)
        .map_err(|_| wording::learnings_selection_bad_candidate_id(candidate))?;
    Ok(Learning::new(
        candidate_id,
        LearningScope::of_spelling(
            object
                .get("scope")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
        ),
        PracticeHeading::from_routed(heading),
        text,
        LearningSource::of_spelling(
            object
                .get("source")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::{phase_of, state_field};

    #[test]
    fn the_phase_is_the_third_segment_from_the_end_of_the_memory_path() {
        assert_eq!(
            phase_of(
                "aidlc/spaces/default/intents/260908-x/inception/requirements-analysis/memory.md"
            ),
            "inception"
        );
        assert_eq!(
            phase_of("construction/code-generation/memory.md"),
            "construction"
        );
        assert_eq!(phase_of("memory.md"), "");
    }

    #[test]
    fn the_state_field_reads_the_value_after_the_label() {
        let state = "## Current Status\n- **Current Stage**: requirements-analysis\n- **Status**: Running\n";
        assert_eq!(
            state_field(state, "Current Stage").as_deref(),
            Some("requirements-analysis")
        );
        assert_eq!(state_field(state, "Next Stage"), None);
    }
}
