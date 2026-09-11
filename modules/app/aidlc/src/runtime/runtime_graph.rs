//! runtime-graph 再構築フック — 遷移を書いた Bash の直後に compile を走らせる。
//!
//! 固定本家 2.7.1 `a277af21` の `hooks/aidlc-rebuild-stage-graph.ts:110-259` と
//! `tools/aidlc-runtime.ts:782-808` に対応する。**静的な `stage-graph.json` の読取りとは
//! 別物**であり、compile は runtime-graph を書くだけでなく、承認済みで日誌が空の位置へ
//! 重複を抑えた `MEMORY_EMPTY` を記録する。
//!
//! # 発火の 2 段
//!
//! 1. コマンドの語彙的な判定 ([`harness_claude::RuntimeCompileEnvelope`]) — 遷移を書く
//!    公開面だけを通し、`aidlc-runtime` 自身は落とす (再帰ガード)。
//! 2. 監査末尾 3 ブロックの遷移判定 — 承認 1 回が 1 度の Bash で 3 行まで書くので、
//!    最後の 3 ブロックのどれかに遷移が現れる。
//!
//! Kiro の `ide-audit-sync` 分岐は写していない — 他ハーネスは本 slice の範囲外である。
use super::{Completion, Layout, Utc};
use core_command_domain::orchestration::{MemoryJournal, MemoryJournalSurvey, StageMemoryJournal};
use core_command_domain::workflow_definition::StageSlug;
use core_command_interface_adapter::orchestration::IntentExecutionRepositoryImpl;
use core_command_use_case::orchestration::ObserveMemoryJournalsUseCase;
use core_read_model_updater::orchestration::{RuntimeGraphReadModelUpdater, RuntimeGraphTargets};
use std::path::Path;

/// 承認 1 回が 1 度の Bash で書く監査行の上限 (本家の tail-read 幅)。
const TRANSITION_TAIL_BLOCKS: usize = 3;
/// compile を発火させる遷移クラスのイベント名。
const TRANSITION_EVENTS: [&str; 6] = [
    "GATE_APPROVED",
    "STAGE_STARTED",
    "STAGE_AWAITING_APPROVAL",
    "AUDIT_MERGED",
    "UNIT_MERGED",
    "WORKFLOW_COMPLETED",
];
/// 記録の直下に置かれる phase ディレクトリ (日誌 `memory.md` の親)。
const PHASE_DIRS: [&str; 5] = [
    "initialization",
    "ideation",
    "inception",
    "construction",
    "operation",
];

pub(super) async fn run(layout: &Layout, input: &str) -> Completion {
    let Some(envelope) = harness_claude::RuntimeCompileEnvelope::parse(input) else {
        return Completion::silent();
    };
    if !envelope.compiles() {
        return Completion::silent();
    }
    let (Some(record), Some(audit_dir)) = (layout.record_dir(), layout.audit_dir()) else {
        return Completion::silent();
    };
    if layout.state_file().is_none_or(|path| !path.exists()) {
        return Completion::silent();
    }
    let audit = read_audit(&audit_dir);
    if audit.is_empty() {
        return Completion::silent();
    }
    let health = super::observe_hook_health(layout, "rebuild-stage-graph").await;
    if health.code() != 0 {
        return health;
    }
    if !tail_carries_transition(&audit) {
        return Completion::silent();
    }
    let record = record.to_path_buf();
    if let Err(error) = compile(layout, &record).await {
        let _ = super::record_hook_drop(layout, "rebuild-stage-graph", &error).await;
    }
    Completion::silent()
}

/// 観測 → 更新 → 投影 → runtime-graph の順に確定する。
///
/// 監査行 (`MEMORY_EMPTY`) を先に確定してから成果物を書くのは本家と同じ順序である。
async fn compile(layout: &Layout, record: &Path) -> Result<(), String> {
    let Some(cursor) = super::active_execution(layout).map_err(|e| e.to_string())? else {
        return Ok(());
    };
    let store = super::store_path(layout)?;
    let repository = IntentExecutionRepositoryImpl::open(&store).map_err(|e| e.to_string())?;
    ObserveMemoryJournalsUseCase::new(repository)
        .execute(cursor.execution_id(), survey(record), Utc::now())
        .await
        .map_err(|error| error.to_string())?;
    super::catch_up(layout).await?;
    let prefix = record_prefix(layout, record)?;
    RuntimeGraphReadModelUpdater::open(&store)
        .map_err(|error| error.to_string())?
        .catch_up(
            cursor.execution_id(),
            &RuntimeGraphTargets::new(record, prefix),
        )
        .await
        .map_err(|error| format!("runtime graph: {error}"))
}

/// 記録ディレクトリのプロジェクト相対の綴り (`memory_path` の前置)。
fn record_prefix(layout: &Layout, record: &Path) -> Result<String, String> {
    let name = record
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "record directory name is not UTF-8".to_string())?;
    Ok(format!("aidlc/spaces/{}/intents/{name}", layout.space()))
}

/// 監査シャードをファイル名順に連結する。
fn read_audit(dir: &Path) -> String {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return String::new();
    };
    let mut shards: Vec<std::path::PathBuf> = entries
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

/// 監査末尾 3 ブロックのどれかが遷移クラスか。
fn tail_carries_transition(audit: &str) -> bool {
    let blocks: Vec<&str> = audit.split("\n---\n").collect();
    blocks
        .iter()
        .skip(blocks.len().saturating_sub(TRANSITION_TAIL_BLOCKS))
        .any(|block| {
            block.lines().any(|line| {
                line.strip_prefix("**Event**: ")
                    .is_some_and(|event| TRANSITION_EVENTS.contains(&event.trim()))
            })
        })
}

/// 記録の下にある `memory.md` を読んで観測列を組む。
///
/// 日誌が**無い**位置は列に載せない — 本家の `readMemory` がファイル不在を `null` として
/// `MEMORY_EMPTY` の対象から外すのと同じ区別である。
fn survey(record: &Path) -> MemoryJournalSurvey {
    let mut observations = Vec::new();
    for phase in PHASE_DIRS {
        let Ok(entries) = std::fs::read_dir(record.join(phase)) else {
            continue;
        };
        let mut stages: Vec<std::path::PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        stages.sort();
        for stage in stages {
            let Some(slug) = stage
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| StageSlug::parse(name).ok())
            else {
                continue;
            };
            let Ok(raw) = std::fs::read_to_string(stage.join("memory.md")) else {
                continue;
            };
            observations.push(StageMemoryJournal::new(slug, MemoryJournal::parse(&raw)));
        }
    }
    MemoryJournalSurvey::new(observations)
}

#[cfg(test)]
mod tests {
    use super::tail_carries_transition;

    fn block(event: &str) -> String {
        format!("\n## H\n**Timestamp**: 2026-09-09T00:00:00Z\n**Event**: {event}\n\n---\n")
    }

    #[test]
    fn a_transition_in_the_last_three_blocks_fires_the_compile() {
        let audit = format!(
            "# AI-DLC Audit Log\n{}{}{}",
            block("GATE_APPROVED"),
            block("STAGE_COMPLETED"),
            block("STAGE_STARTED")
        );
        assert!(tail_carries_transition(&audit));
    }

    #[test]
    fn a_transition_pushed_out_of_the_tail_does_not_fire() {
        let audit = format!(
            "# AI-DLC Audit Log\n{}{}{}{}",
            block("STAGE_STARTED"),
            block("HUMAN_TURN"),
            block("HUMAN_TURN"),
            block("HUMAN_TURN")
        );
        assert!(!tail_carries_transition(&audit));
    }

    /// compile 自身が書く `MEMORY_EMPTY` は遷移クラスではない (再帰ガードの後段)。
    #[test]
    fn the_compile_s_own_rows_are_not_transitions() {
        let audit = format!("# AI-DLC Audit Log\n{}", block("MEMORY_EMPTY"));
        assert!(!tail_carries_transition(&audit));
    }
}
