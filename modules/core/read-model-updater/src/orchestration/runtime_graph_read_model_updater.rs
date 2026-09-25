//! runtime-graph の投影 — ジャーナルと監査台帳から `<record>/runtime-graph.json` を描く。
//!
//! 固定本家 2.7.1 `a277af21` の `tools/aidlc-runtime.ts` の `compile` に対応する
//! (`:319-812`)。**静的な `stage-graph.json` の読取りとは別物**である — こちらは
//! 「いま何がいつ始まって、いつ承認され、日誌が何件あるか」という実行の観測であり、
//! 配布グラフはその材料ではない。
//!
//! # なぜ独立した投影なのか
//!
//! runtime-graph は再生成可能な派生物であり、状態ファイル・監査シャードのような正本では
//! ない (配布の `.gitignore` が除外している)。心拍リードモデルと同じく、専用の取得ループを
//! 持つ形にしてある — 通常の取得ループは差分が無いと何も書かないが、日誌 `memory.md` は
//! ジャーナルの外で書き換わるので、compile は差分の有無に依らず描き直す必要がある。
//!
//! # 対の材料
//!
//! 位置ごとの `started_at` / `completed_at` / `agent` は**監査台帳の対**から読む
//! (本家 `pairStartedCompleted`)。台帳は本 RMU が描いた読取面であり、そこに現れた対こそが
//! 観測可能な契約である。ジャーナルからは計画 (位置の phase と担当) と日誌観測を取る。
use std::collections::BTreeMap;
use std::path::Path;

use core_command_domain::orchestration::{IntentExecutionEvent, IntentExecutionId};
use core_command_domain::workspace::{EventType, OrderedAuditEvents};
use serde::Serialize;

use super::{
    GlobalSeqNr, JournalBatch, JournalReadError, JournalReader as _, ReadModelUpdateError,
    ReadModelUpdater,
};
use super::{JournalReaderImpl, RuntimeGraphTargets};
use crate::workspace::ResolvedPlan;
use core_command_domain::workspace::StorePath;

/// 位置ごとの日誌の内訳 (4 見出し)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct MemoryBreakdownDto {
    interpretations: u64,
    deviations: u64,
    tradeoffs: u64,
    open_questions: u64,
}

/// 位置 1 行。キーの並びが契約である (本家 `RuntimeStage`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RuntimeStageDto {
    stage_slug: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    agent: Option<String>,
    memory_path: String,
    memory_entries: Option<u64>,
    memory_breakdown: Option<MemoryBreakdownDto>,
    sensor_firings: Vec<SensorFiringDto>,
    outcome: &'static str,
    learnings_captured: Option<LearningsCapturedDto>,
}

/// センサー発火の対 (本家 `SensorFiring`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SensorFiringDto {
    id: String,
    fire_id: String,
    result: &'static str,
    ts: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail_path: Option<String>,
}

/// 承認済みの位置で数えた学びの件数 (本家 `learnings_captured`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct LearningsCapturedDto {
    from_orchestrator: u64,
    from_user_addition: u64,
}

/// 投影する runtime-graph 全体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RuntimeGraphDto {
    workflow_id: String,
    scope: String,
    started_at: String,
    stages: Vec<RuntimeStageDto>,
}

/// 監査の対 1 件 (位置ごとの開始・完了・担当)。
#[derive(Debug, Clone)]
struct PairingEntry {
    started_at: String,
    completed_at: Option<String>,
    agent: String,
    started_index: usize,
}

/// runtime-graph リードモデルの取得ループ。
///
/// 描く対象の実行と書込先は構築時に束ねる（[`ReadModelUpdater`] の契約）。
#[derive(Debug)]
pub struct RuntimeGraphReadModelUpdater {
    reader: JournalReaderImpl,
    execution: IntentExecutionId,
    targets: RuntimeGraphTargets,
}

impl RuntimeGraphReadModelUpdater {
    /// 既にあるストアを読取用に開き、描く実行と書込先を束ねる。
    ///
    /// # Errors
    /// ストアを開けない・本家の `journal` 表がまだ無い場合。
    pub fn open(
        store: &StorePath,
        execution: IntentExecutionId,
        targets: RuntimeGraphTargets,
    ) -> Result<Self, JournalReadError> {
        Ok(Self {
            reader: JournalReaderImpl::open(store)?,
            execution,
            targets,
        })
    }
}

impl ReadModelUpdater for RuntimeGraphReadModelUpdater {
    type Error = ReadModelUpdateError;

    /// 対象実行の runtime-graph を描き直す。
    ///
    /// 監査台帳が空 (まだ 1 行も無い) なら**何も書かない** — 本家 compile も
    /// `WORKFLOW_STARTED` が無い履歴では書かないからである。
    ///
    /// # Errors
    /// ジャーナル読取、計画の不在、監査・出力の I/O 失敗。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        let execution = &self.execution;
        let targets = &self.targets;
        // 全履歴を読む — 差分ではない。日誌 `memory.md` はジャーナルの外で書き換わるので、
        // compile は毎回すべての対を数え直す。
        let history = self
            .reader
            .events_after(GlobalSeqNr::ZERO)
            .await
            .map_err(ReadModelUpdateError::Read)?;
        let plan = resolve_plan(&history, execution)?;
        let audit = read_audit(targets.audit_dir())?;
        let Some(graph) = build_graph(&audit, &history, execution, &plan, targets) else {
            return Ok(());
        };
        // 契約 JSON の直列化は `ContractPretty` (2 スペース + 宣言順 + 末尾改行) — 固定本家の
        // `JSON.stringify(graph, null, 2)` に改行を足した体裁と同じである (BR1.7)。
        let value = core_infrastructure::canon_json::to_value(&graph).map_err(|_| {
            ReadModelUpdateError::PublicationIo {
                path: targets.graph_file().to_path_buf(),
                kind: std::io::ErrorKind::InvalidData,
            }
        })?;
        let body = core_infrastructure::canon_json::serialize(
            &value,
            core_infrastructure::canon_json::SerializationProfile::ContractPretty,
        );
        core_infrastructure::atomic::write_file_atomic(targets.graph_file(), body.as_bytes())
            .map_err(|error| ReadModelUpdateError::PublicationIo {
                path: targets.graph_file().to_path_buf(),
                kind: error.kind(),
            })
    }
}

/// 誕生記録から計画を起こす (`OrchestrationReadModelUpdater::resolve_plan` と同じ規則)。
fn resolve_plan(
    history: &JournalBatch,
    execution: &IntentExecutionId,
) -> Result<ResolvedPlan, ReadModelUpdateError> {
    let intent_id = history
        .executions()
        .iter()
        .filter(|entry| entry.execution_id() == execution)
        .find_map(|entry| match entry.event() {
            IntentExecutionEvent::Started(started) => Some(started.intent_id().clone()),
            _ => None,
        })
        .ok_or(ReadModelUpdateError::PlanUnavailable)?;
    history
        .intents()
        .iter()
        .find(|intent| intent.id() == &intent_id)
        .map(ResolvedPlan::of)
        .ok_or(ReadModelUpdateError::PlanUnavailable)
}

/// 監査シャードをファイル名順に連結する (本家 `readAllAuditShards`)。
fn read_audit(dir: &Path) -> Result<String, ReadModelUpdateError> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(String::new());
    };
    let mut shards: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    shards.sort();
    let mut buffer = String::new();
    for shard in shards {
        let text = std::fs::read_to_string(&shard).map_err(|error| {
            ReadModelUpdateError::PublicationIo {
                path: shard.clone(),
                kind: error.kind(),
            }
        })?;
        buffer.push_str(&text.replace("\r\n", "\n"));
    }
    Ok(buffer)
}

/// ブロック本文から `**<key>**: <value>` を読む。
fn field_of(block: &str, key: &str) -> Option<String> {
    let prefix = format!("**{key}**: ");
    block
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(|value| value.trim().to_string())
}

/// 隔離実行 (`--single`) の合成行か (本家 `isSingleStageRow`)。
fn single_stage_row(block: &str) -> bool {
    field_of(block, "Workflow").is_some_and(|value| value.starts_with("single-stage:"))
}

/// 台帳を対にする (本家 `pairStartedCompleted`)。
fn pair_started_completed(
    audit: &str,
    blocks: &[&str],
    since: &str,
) -> BTreeMap<String, PairingEntry> {
    let events = OrderedAuditEvents::find_in(audit).filter(|record| {
        matches!(
            record.event(),
            EventType::StageStarted | EventType::StageCompleted
        ) && record.timestamp() >= since
            && blocks
                .get(record.position())
                .is_some_and(|block| !single_stage_row(block))
    });
    // 同一秒の並びは**開始が先**である。本家は 2 つの列を 1 本へ混ぜるとき、完了側の
    // タイブレーク添字に定数を足して必ず後ろへ送る (`aidlc-runtime.ts:200-209`)。同じ秒に
    // 「完了 → 再開始」が並ぶ台帳で、素のバッファ順だと再開始が対を持てなくなる。
    let mut stream: Vec<(&str, u8, usize)> = events.fold_left(Vec::new(), |mut rows, record| {
        let rank = u8::from(record.event() == EventType::StageCompleted);
        rows.push((record.timestamp(), rank, record.position()));
        rows
    });
    stream.sort_by(|left, right| left.0.cmp(right.0).then(left.1.cmp(&right.1)));
    let mut pairs: BTreeMap<String, PairingEntry> = BTreeMap::new();
    let mut source_index = 0;
    for (timestamp, rank, position) in stream {
        let Some(block) = blocks.get(position) else {
            continue;
        };
        let Some(slug) = field_of(block, "Stage") else {
            continue;
        };
        if rank == 0 {
            pairs.insert(
                slug,
                PairingEntry {
                    started_at: timestamp.to_string(),
                    completed_at: None,
                    agent: field_of(block, "Agent").unwrap_or_default(),
                    started_index: source_index,
                },
            );
            source_index += 1;
        } else if let Some(entry) = pairs.get_mut(&slug)
            && entry.completed_at.is_none()
        {
            entry.completed_at = Some(timestamp.to_string());
        }
    }
    pairs
}

/// 学びの件数を窓で数える (本家 `countLearnings`)。
fn count_learnings(
    audit: &str,
    blocks: &[&str],
    slug: &str,
    window_start: &str,
    window_end: Option<&str>,
) -> LearningsCapturedDto {
    OrderedAuditEvents::find_in(audit)
        .filter(|record| {
            matches!(
                record.event(),
                EventType::RuleLearned | EventType::SensorProposed
            ) && record.timestamp() >= window_start
                && window_end.is_none_or(|end| record.timestamp() < end)
        })
        .fold_left(
            LearningsCapturedDto {
                from_orchestrator: 0,
                from_user_addition: 0,
            },
            |mut counts, record| {
                let Some(block) = blocks.get(record.position()) else {
                    return counts;
                };
                if field_of(block, "Stage").as_deref() != Some(slug) {
                    return counts;
                }
                if field_of(block, "Source").as_deref() == Some("user_addition") {
                    counts.from_user_addition += 1;
                } else {
                    counts.from_orchestrator += 1;
                }
                counts
            },
        )
}

/// センサー発火の対を窓で拾う (本家 `pairFirings` の親パス)。
///
/// 発火行そのものが無ければ空になる — 空の既定値を書いているのではなく、台帳に対が
/// 無いという観測である。
fn pair_firings(
    audit: &str,
    blocks: &[&str],
    slug: &str,
    window_start: &str,
    window_end: Option<&str>,
    baseline: &str,
) -> Vec<SensorFiringDto> {
    /// 孤児を `incomplete` と見なすまでの秒数 (本家 `ORPHAN_CUTOFF_SECONDS`)。
    const ORPHAN_CUTOFF_SECONDS: i64 = 60;
    let events = OrderedAuditEvents::find_in(audit);
    let mut terminals: BTreeMap<String, (&'static str, String, Option<String>)> = BTreeMap::new();
    for (event, kind) in [
        (EventType::SensorPassed, "passed"),
        (EventType::SensorFailed, "failed"),
        (EventType::SensorBudgetOverride, "budget-override"),
    ] {
        events
            .filter(|record| record.event() == event)
            .fold_left((), |(), record| {
                let Some(block) = blocks.get(record.position()) else {
                    return;
                };
                let Some(fire_id) = field_of(block, "Fire id") else {
                    return;
                };
                let detail = if kind == "failed" {
                    field_of(block, "Detail path")
                } else {
                    None
                };
                let replace = terminals
                    .get(&fire_id)
                    .is_none_or(|(_, ts, _)| ts.as_str() < record.timestamp());
                if replace {
                    terminals.insert(fire_id, (kind, record.timestamp().to_string(), detail));
                }
            });
    }
    let mut firings: Vec<SensorFiringDto> = Vec::new();
    events
        .filter(|record| {
            record.event() == EventType::SensorFired
                && record.timestamp() >= window_start
                && window_end.is_none_or(|end| record.timestamp() < end)
        })
        .fold_left((), |(), record| {
            let Some(block) = blocks.get(record.position()) else {
                return;
            };
            if field_of(block, "Stage slug").as_deref() != Some(slug) {
                return;
            }
            let (Some(fire_id), Some(sensor_id)) =
                (field_of(block, "Fire id"), field_of(block, "Sensor ID"))
            else {
                return;
            };
            if let Some((kind, _, detail)) = terminals.get(&fire_id) {
                firings.push(SensorFiringDto {
                    id: sensor_id,
                    fire_id,
                    result: kind,
                    ts: record.timestamp().to_string(),
                    detail_path: detail.clone(),
                });
                return;
            }
            let orphan = window_end.is_some()
                || seconds_between(record.timestamp(), baseline) >= ORPHAN_CUTOFF_SECONDS;
            if orphan {
                firings.push(SensorFiringDto {
                    id: sensor_id,
                    fire_id,
                    result: "incomplete",
                    ts: record.timestamp().to_string(),
                    detail_path: None,
                });
            }
        });
    firings.sort_by(|left, right| left.ts.cmp(&right.ts));
    firings
}

/// 秒精度 ISO の 2 点間の秒数 (読めない綴りは 0 秒差として扱う)。
fn seconds_between(from: &str, to: &str) -> i64 {
    let instant = |text: &str| {
        chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%SZ")
            .ok()
            .map(|naive| naive.and_utc().timestamp())
    };
    match (instant(from), instant(to)) {
        (Some(from), Some(to)) => to - from,
        _ => 0,
    }
}

/// 実行の最後の日誌観測 (無ければ観測なし)。
fn latest_survey<'a>(
    history: &'a JournalBatch,
    execution: &IntentExecutionId,
) -> Option<&'a core_command_domain::orchestration::MemoryJournalSurvey> {
    history
        .executions()
        .iter()
        .filter(|entry| entry.execution_id() == execution)
        .filter_map(|entry| match entry.event() {
            IntentExecutionEvent::MemoryJournalsObserved(observed) => Some(observed.survey()),
            _ => None,
        })
        .next_back()
}

fn build_graph(
    audit: &str,
    history: &JournalBatch,
    execution: &IntentExecutionId,
    plan: &ResolvedPlan,
    targets: &RuntimeGraphTargets,
) -> Option<RuntimeGraphDto> {
    let blocks: Vec<&str> = audit.split("\n---\n").collect();
    let ordered = OrderedAuditEvents::find_in(audit);
    let started = ordered.filter(|record| record.event() == EventType::WorkflowStarted);
    let latest = started.latest()?;
    let scope = state_scope(targets.state_file())
        .or_else(|| {
            blocks
                .get(latest.position())
                .and_then(|b| field_of(b, "Scope"))
        })
        .unwrap_or_default();
    // 孤児の打ち切りに使う決定的な「いま」。本家は **stage 開始行とセンサー行だけ**の最大時刻を
    // 使う (`aidlc-runtime.ts:623-633`)。compile 自身が書く `MEMORY_EMPTY` の時刻を混ぜると、
    // 再発火のたびに基準が動いて出力が非決定的になる。
    let baseline = ordered
        .filter(|record| {
            matches!(
                record.event(),
                EventType::StageStarted
                    | EventType::SensorFired
                    | EventType::SensorPassed
                    | EventType::SensorFailed
                    | EventType::SensorBudgetOverride
            )
        })
        .fold_left(String::new(), |newest, record| {
            if record.timestamp() > newest.as_str() {
                record.timestamp().to_string()
            } else {
                newest
            }
        });
    let pairs = pair_started_completed(audit, &blocks, latest.timestamp());
    let survey = latest_survey(history, execution);
    let mut rows: Vec<(usize, RuntimeStageDto)> = Vec::new();
    for (slug, entry) in &pairs {
        let Some(parsed) = core_command_domain::workflow_definition::StageSlug::parse(slug).ok()
        else {
            continue;
        };
        let Some(stage) = plan.find(&parsed) else {
            continue; // 計画に無い slug は描かない (本家も phaseMap 不在で読み飛ばす)。
        };
        let journal = survey.and_then(|survey| survey.find(&parsed));
        let approved = entry.completed_at.is_some();
        rows.push((
            entry.started_index,
            RuntimeStageDto {
                stage_slug: slug.clone(),
                started_at: Some(entry.started_at.clone()),
                completed_at: entry.completed_at.clone(),
                agent: Some(if entry.agent.is_empty() {
                    stage.display().lead_agent().to_string()
                } else {
                    entry.agent.clone()
                }),
                memory_path: format!(
                    "{}/{}/{slug}/memory.md",
                    targets.record_prefix(),
                    stage.phase().as_str()
                ),
                memory_entries: journal.map(|observed| observed.journal().total()),
                memory_breakdown: journal.map(|observed| {
                    let counts = observed.journal();
                    MemoryBreakdownDto {
                        interpretations: counts.interpretations(),
                        deviations: counts.deviations(),
                        tradeoffs: counts.tradeoffs(),
                        open_questions: counts.open_questions(),
                    }
                }),
                sensor_firings: pair_firings(
                    audit,
                    &blocks,
                    slug,
                    &entry.started_at,
                    entry.completed_at.as_deref(),
                    &baseline,
                ),
                outcome: if approved { "approved" } else { "pending" },
                learnings_captured: approved.then(|| {
                    count_learnings(
                        audit,
                        &blocks,
                        slug,
                        &entry.started_at,
                        entry.completed_at.as_deref(),
                    )
                }),
            },
        ));
    }
    rows.sort_by_key(|(index, _)| *index);
    Some(RuntimeGraphDto {
        workflow_id: latest.timestamp().to_string(),
        scope,
        started_at: latest.timestamp().to_string(),
        stages: rows.into_iter().map(|(_, row)| row).collect(),
    })
}

/// 状態ファイルの `Scope` 欄 (無ければ `None`)。
fn state_scope(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("- **Scope**: "))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{field_of, pair_started_completed, single_stage_row};

    fn block(event: &str, timestamp: &str, stage: &str, extra: &str) -> String {
        format!(
            "\n## H\n**Timestamp**: {timestamp}\n**Event**: {event}\n**Stage**: {stage}\n{extra}"
        )
    }

    #[test]
    fn a_field_is_read_verbatim_and_a_missing_one_is_absent() {
        let text = block(
            "STAGE_STARTED",
            "2026-09-09T00:00:00Z",
            "state-init",
            "**Agent**: orchestrator\n",
        );
        assert_eq!(field_of(&text, "Stage").as_deref(), Some("state-init"));
        assert_eq!(field_of(&text, "Agent").as_deref(), Some("orchestrator"));
        assert_eq!(field_of(&text, "Workflow"), None);
    }

    #[test]
    fn an_isolated_run_row_is_recognised_by_its_workflow_field() {
        let single = block(
            "STAGE_STARTED",
            "2026-09-09T00:00:00Z",
            "domain-design",
            "**Workflow**: single-stage:domain-design\n",
        );
        assert!(single_stage_row(&single));
        assert!(!single_stage_row(&block(
            "STAGE_STARTED",
            "2026-09-09T00:00:00Z",
            "domain-design",
            ""
        )));
    }

    #[test]
    fn the_pairing_keeps_the_start_the_completion_and_the_agent() {
        let audit = format!(
            "# AI-DLC Audit Log\n{}\n---\n{}\n---\n",
            block(
                "STAGE_STARTED",
                "2026-09-09T00:00:00Z",
                "state-init",
                "**Agent**: orchestrator\n"
            ),
            block("STAGE_COMPLETED", "2026-09-09T00:00:05Z", "state-init", "")
        );
        let blocks: Vec<&str> = audit.split("\n---\n").collect();
        let pairs = pair_started_completed(&audit, &blocks, "2026-09-09T00:00:00Z");
        let entry = pairs.get("state-init").expect("対がある");
        assert_eq!(entry.started_at, "2026-09-09T00:00:00Z");
        assert_eq!(entry.completed_at.as_deref(), Some("2026-09-09T00:00:05Z"));
        assert_eq!(entry.agent, "orchestrator");
    }

    #[test]
    fn a_row_older_than_the_workflow_start_is_left_out() {
        let audit = format!(
            "# AI-DLC Audit Log\n{}\n---\n",
            block("STAGE_STARTED", "2026-09-08T00:00:00Z", "state-init", "")
        );
        let blocks: Vec<&str> = audit.split("\n---\n").collect();
        assert!(
            pair_started_completed(&audit, &blocks, "2026-09-09T00:00:00Z").is_empty(),
            "前のワークフローの行は対にしない"
        );
    }

    #[test]
    fn an_isolated_run_row_is_left_out_of_the_pairing() {
        let audit = format!(
            "# AI-DLC Audit Log\n{}\n---\n",
            block(
                "STAGE_STARTED",
                "2026-09-09T00:00:00Z",
                "domain-design",
                "**Workflow**: single-stage:domain-design\n"
            )
        );
        let blocks: Vec<&str> = audit.split("\n---\n").collect();
        assert!(pair_started_completed(&audit, &blocks, "2026-09-09T00:00:00Z").is_empty());
    }

    /// 同一秒に「完了 → 再開始」が並ぶ台帳でも、再開始が対の左半分になる。
    ///
    /// 本家は開始側を必ず先に処理する (`aidlc-runtime.ts:200-209` の添字オフセット)。
    /// バッファ順のまま畳むと、完了行が先に来た瞬間に対が壊れる。
    #[test]
    fn at_the_same_second_the_start_is_processed_before_the_completion() {
        let audit = format!(
            "# AI-DLC Audit Log\n{}\n---\n{}\n---\n{}\n---\n",
            block(
                "STAGE_STARTED",
                "2026-09-09T00:00:00Z",
                "code-generation",
                ""
            ),
            block(
                "STAGE_COMPLETED",
                "2026-09-09T00:00:09Z",
                "code-generation",
                ""
            ),
            block(
                "STAGE_STARTED",
                "2026-09-09T00:00:09Z",
                "code-generation",
                "**Agent**: aidlc-developer-agent\n"
            )
        );
        let blocks: Vec<&str> = audit.split("\n---\n").collect();
        let pairs = pair_started_completed(&audit, &blocks, "2026-09-09T00:00:00Z");
        let entry = pairs.get("code-generation").expect("対がある");
        assert_eq!(entry.started_at, "2026-09-09T00:00:09Z", "再開始が左半分");
        assert_eq!(
            entry.completed_at.as_deref(),
            Some("2026-09-09T00:00:09Z"),
            "同じ秒の完了は再開始の対を埋める"
        );
        assert_eq!(entry.agent, "aidlc-developer-agent");
    }
}
