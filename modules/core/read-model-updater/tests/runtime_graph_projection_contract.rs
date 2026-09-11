//! runtime-graph の投影契約 — 監査台帳の対・センサー発火・学びの件数を、固定本家
//! `compile` と同じ形の JSON へ描く。台帳が無ければ何も書かず、材料が壊れていれば拒否する。
// テストコードでは unwrap / expect / panic を許可 (オーナー規約)。integration test は
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::needless_pass_by_value,
    clippy::type_complexity
)]
mod support;

use core_command_domain::orchestration::{MemoryJournal, MemoryJournalSurvey, StageMemoryJournal};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{SpaceName, StorePath};
use core_read_model_updater::orchestration::{
    CatchUpError, RuntimeGraphReadModelUpdater, RuntimeGraphTargets,
};
use std::path::PathBuf;

struct Fixture {
    root: tempfile::TempDir,
    store: StorePath,
    upstream: support::UpstreamStore,
    writer: support::JournalWriter,
}

impl Fixture {
    /// intent の誕生と実行の genesis（`Started` / `GateOpened` / `GateApproved`）を本家
    /// ストアへ書く。索引 1 = intent-capture が承認済み、索引 2 = scope-definition が進行中。
    async fn seeded() -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = StorePath::for_space(&root.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(store.as_path().parent().unwrap()).unwrap();
        support::seed_intent(&store).await;
        let mut upstream = support::open_store(&store);
        let mut writer =
            support::JournalWriter::start(&mut upstream, support::execution_id()).await;
        writer
            .advance(&mut upstream, |aggregate| {
                aggregate.open_gate(
                    &support::intent(),
                    core_command_domain::orchestration::ArtifactPaths::new(vec![
                        "intent.md".to_string(),
                    ]),
                    support::at(),
                )
            })
            .await;
        writer
            .advance(&mut upstream, |aggregate| {
                aggregate.approve_gate(
                    &support::intent(),
                    None,
                    Some("ok".to_string()),
                    support::at(),
                )
            })
            .await;
        Self {
            root,
            store,
            upstream,
            writer,
        }
    }
    /// 日誌観測を 1 件書く。
    async fn observe(&mut self, survey: MemoryJournalSurvey) {
        self.writer
            .advance(&mut self.upstream, |aggregate| {
                aggregate.observe_memory_journals(survey, support::at())
            })
            .await;
    }
    fn record(&self) -> PathBuf {
        let record = self.root.path().join("record");
        std::fs::create_dir_all(record.join("audit")).unwrap();
        record
    }
    fn targets(&self) -> RuntimeGraphTargets {
        RuntimeGraphTargets::new(self.record(), "aidlc/spaces/default/intents/record")
    }
    fn write_shard(&self, name: &str, audit: &str) {
        std::fs::write(self.record().join("audit").join(name), audit).unwrap();
    }
    async fn catch_up(&self) -> Result<(), CatchUpError> {
        RuntimeGraphReadModelUpdater::open(&self.store)
            .unwrap()
            .catch_up(&support::execution_id(), &self.targets())
            .await
    }
    fn graph(&self) -> Option<String> {
        std::fs::read_to_string(self.record().join("runtime-graph.json")).ok()
    }
}

fn block(timestamp: &str, event: &str, fields: &[(&str, &str)]) -> String {
    let mut text = format!("\n## {event}\n**Timestamp**: {timestamp}\n**Event**: {event}\n");
    for (key, value) in fields {
        text.push_str(&format!("**{key}**: {value}\n"));
    }
    text.push_str("\n---\n");
    text
}

fn ledger(blocks: &[String]) -> String {
    format!("# AI-DLC Audit Log\n{}", blocks.concat())
}

#[tokio::test]
async fn the_graph_pairs_the_ledger_and_reads_the_scope_from_the_workflow_start() {
    let mut fixture = Fixture::seeded().await;
    // 日誌観測を 1 件書く — intent-capture に 2 件（interpretations 1・tradeoffs 1）。
    fixture
        .observe(MemoryJournalSurvey::new(vec![StageMemoryJournal::new(
            StageSlug::parse("intent-capture").unwrap(),
            MemoryJournal::new(1, 0, 1, 0),
        )]))
        .await;
    let audit = ledger(&[
        block(
            "2026-09-09T00:00:00Z",
            "WORKFLOW_STARTED",
            &[("Scope", "classic")],
        ),
        // 前のワークフローの行は窓の外。
        block(
            "2026-09-08T23:59:59Z",
            "STAGE_STARTED",
            &[("Stage", "state-init")],
        ),
        // Agent 欄を欠く開始行は計画の担当で埋める。
        block(
            "2026-09-09T00:00:01Z",
            "STAGE_STARTED",
            &[("Stage", "intent-capture")],
        ),
        // Stage 欄を欠く行・計画に無い slug・文法外の slug は描かない。
        block(
            "2026-09-09T00:00:02Z",
            "STAGE_STARTED",
            &[("Agent", "orchestrator")],
        ),
        block(
            "2026-09-09T00:00:02Z",
            "STAGE_STARTED",
            &[("Stage", "unplanned-stage")],
        ),
        block(
            "2026-09-09T00:00:02Z",
            "STAGE_STARTED",
            &[("Stage", "Not A Slug")],
        ),
        block(
            "2026-09-09T00:00:03Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "aaaa0001"),
                ("Sensor ID", "aidlc-linter"),
                ("Stage slug", "intent-capture"),
            ],
        ),
        block(
            "2026-09-09T00:00:04Z",
            "SENSOR_PASSED",
            &[("Fire id", "aaaa0001")],
        ),
        block(
            "2026-09-09T00:00:05Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "aaaa0002"),
                ("Sensor ID", "aidlc-traceability"),
                ("Stage slug", "intent-capture"),
            ],
        ),
        block(
            "2026-09-09T00:00:06Z",
            "SENSOR_FAILED",
            &[
                ("Fire id", "aaaa0002"),
                ("Detail path", ".aidlc-sensors/intent-capture/x.md"),
            ],
        ),
        // 終端が無い発火は、承認済みの窓の中なら incomplete。
        block(
            "2026-09-09T00:00:07Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "aaaa0003"),
                ("Sensor ID", "aidlc-type-check"),
                ("Stage slug", "intent-capture"),
            ],
        ),
        // Fire id を欠く終端・発火は無視される。
        block("2026-09-09T00:00:07Z", "SENSOR_PASSED", &[]),
        block(
            "2026-09-09T00:00:07Z",
            "SENSOR_FIRED",
            &[("Stage slug", "intent-capture")],
        ),
        // 他ステージの発火は数えない。
        block(
            "2026-09-09T00:00:08Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "aaaa0004"),
                ("Sensor ID", "aidlc-linter"),
                ("Stage slug", "state-init"),
            ],
        ),
        block(
            "2026-09-09T00:00:09Z",
            "RULE_LEARNED",
            &[("Stage", "intent-capture"), ("Source", "user_addition")],
        ),
        block(
            "2026-09-09T00:00:09Z",
            "RULE_LEARNED",
            &[("Stage", "intent-capture")],
        ),
        block(
            "2026-09-09T00:00:09Z",
            "SENSOR_PROPOSED",
            &[("Stage", "scope-definition")],
        ),
        block(
            "2026-09-09T00:00:10Z",
            "STAGE_COMPLETED",
            &[("Stage", "intent-capture")],
        ),
        // 窓の外（完了後）の学びは数えない。
        block(
            "2026-09-09T00:00:11Z",
            "RULE_LEARNED",
            &[("Stage", "intent-capture")],
        ),
        block(
            "2026-09-09T00:00:12Z",
            "STAGE_STARTED",
            &[
                ("Stage", "scope-definition"),
                ("Agent", "aidlc-product-agent"),
            ],
        ),
        // 進行中の位置の孤児は、基準時刻（開始行とセンサー行の最大 = 00:02:31）から
        // 60 秒未満なら描かない。
        block(
            "2026-09-09T00:02:00Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "bbbb0001"),
                ("Sensor ID", "aidlc-linter"),
                ("Stage slug", "scope-definition"),
            ],
        ),
        // 60 秒以上前の孤児は incomplete になる。基準は開始行とセンサー行の最大時刻。
        block(
            "2026-09-09T00:01:20Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "bbbb0002"),
                ("Sensor ID", "aidlc-type-check"),
                ("Stage slug", "scope-definition"),
            ],
        ),
        block(
            "2026-09-09T00:02:30Z",
            "SENSOR_BUDGET_OVERRIDE",
            &[("Fire id", "bbbb0003")],
        ),
        block(
            "2026-09-09T00:02:31Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "bbbb0003"),
                ("Sensor ID", "aidlc-linter"),
                ("Stage slug", "scope-definition"),
            ],
        ),
    ]);
    fixture.write_shard("host-clone.md", &audit);
    fixture.catch_up().await.unwrap();
    let expected = r#"{
  "workflow_id": "2026-09-09T00:00:00Z",
  "scope": "classic",
  "started_at": "2026-09-09T00:00:00Z",
  "stages": [
    {
      "stage_slug": "intent-capture",
      "started_at": "2026-09-09T00:00:01Z",
      "completed_at": "2026-09-09T00:00:10Z",
      "agent": "orchestrator",
      "memory_path": "aidlc/spaces/default/intents/record/ideation/intent-capture/memory.md",
      "memory_entries": 2,
      "memory_breakdown": {
        "interpretations": 1,
        "deviations": 0,
        "tradeoffs": 1,
        "open_questions": 0
      },
      "sensor_firings": [
        {
          "id": "aidlc-linter",
          "fire_id": "aaaa0001",
          "result": "passed",
          "ts": "2026-09-09T00:00:03Z"
        },
        {
          "id": "aidlc-traceability",
          "fire_id": "aaaa0002",
          "result": "failed",
          "ts": "2026-09-09T00:00:05Z",
          "detail_path": ".aidlc-sensors/intent-capture/x.md"
        },
        {
          "id": "aidlc-type-check",
          "fire_id": "aaaa0003",
          "result": "incomplete",
          "ts": "2026-09-09T00:00:07Z"
        }
      ],
      "outcome": "approved",
      "learnings_captured": {
        "from_orchestrator": 1,
        "from_user_addition": 1
      }
    },
    {
      "stage_slug": "scope-definition",
      "started_at": "2026-09-09T00:00:12Z",
      "completed_at": null,
      "agent": "aidlc-product-agent",
      "memory_path": "aidlc/spaces/default/intents/record/ideation/scope-definition/memory.md",
      "memory_entries": null,
      "memory_breakdown": null,
      "sensor_firings": [
        {
          "id": "aidlc-type-check",
          "fire_id": "bbbb0002",
          "result": "incomplete",
          "ts": "2026-09-09T00:01:20Z"
        },
        {
          "id": "aidlc-linter",
          "fire_id": "bbbb0003",
          "result": "budget-override",
          "ts": "2026-09-09T00:02:31Z"
        }
      ],
      "outcome": "pending",
      "learnings_captured": null
    }
  ]
}
"#;
    assert_eq!(fixture.graph().as_deref(), Some(expected));
}

#[tokio::test]
async fn the_state_file_scope_outranks_the_ledger_and_an_unreadable_baseline_keeps_orphans_pending()
{
    let fixture = Fixture::seeded().await;
    let record = fixture.record();
    std::fs::write(
        record.join("aidlc-state.md"),
        "# AI-DLC State Tracking\n- **Scope**: express\n",
    )
    .unwrap();
    let audit = ledger(&[
        block(
            "2026-09-09T00:00:00Z",
            "WORKFLOW_STARTED",
            &[("Scope", "classic")],
        ),
        block(
            "2026-09-09T00:00:01Z",
            "STAGE_STARTED",
            &[("Stage", "intent-capture")],
        ),
        block(
            "2026-09-09T00:00:02Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "cccc0001"),
                ("Sensor ID", "aidlc-linter"),
                ("Stage slug", "intent-capture"),
            ],
        ),
        // 読めない綴りの時刻が基準になると、経過秒は 0 として扱う（孤児にしない）。
        block(
            "2026-09-09T99:99:99Z",
            "SENSOR_PASSED",
            &[("Fire id", "zzzz")],
        ),
    ]);
    fixture.write_shard("host-clone.md", &audit);
    fixture.catch_up().await.unwrap();
    let graph: serde_json::Value = serde_json::from_str(&fixture.graph().unwrap()).unwrap();
    assert_eq!(graph["scope"], "express");
    assert_eq!(graph["stages"][0]["sensor_firings"], serde_json::json!([]));
    assert_eq!(graph["stages"][0]["outcome"], "pending");
}

#[tokio::test]
async fn shards_are_concatenated_in_name_order_and_crlf_is_normalised() {
    let fixture = Fixture::seeded().await;
    fixture.write_shard(
        "b-second.md",
        &block(
            "2026-09-09T00:00:05Z",
            "STAGE_COMPLETED",
            &[("Stage", "intent-capture")],
        )
        .replace('\n', "\r\n"),
    );
    fixture.write_shard(
        "a-first.md",
        &ledger(&[
            block(
                "2026-09-09T00:00:00Z",
                "WORKFLOW_STARTED",
                &[("Scope", "classic")],
            ),
            block(
                "2026-09-09T00:00:01Z",
                "STAGE_STARTED",
                &[("Stage", "intent-capture")],
            ),
        ]),
    );
    std::fs::write(
        fixture.record().join("audit").join("notes.txt"),
        "**Event**: STAGE_STARTED\n",
    )
    .unwrap();
    fixture.catch_up().await.unwrap();
    let graph: serde_json::Value = serde_json::from_str(&fixture.graph().unwrap()).unwrap();
    assert_eq!(graph["stages"][0]["completed_at"], "2026-09-09T00:00:05Z");
    assert_eq!(graph["stages"][0]["outcome"], "approved");
}

#[tokio::test]
async fn a_ledger_without_a_workflow_start_writes_nothing() {
    let fixture = Fixture::seeded().await;
    fixture.write_shard(
        "host-clone.md",
        &ledger(&[block(
            "2026-09-09T00:00:01Z",
            "STAGE_STARTED",
            &[("Stage", "intent-capture")],
        )]),
    );
    fixture.catch_up().await.unwrap();
    assert_eq!(
        fixture.graph(),
        None,
        "WORKFLOW_STARTED が無い台帳は描かない"
    );
}

#[tokio::test]
async fn a_missing_audit_directory_writes_nothing() {
    let fixture = Fixture::seeded().await;
    let record = fixture.record();
    std::fs::remove_dir_all(record.join("audit")).unwrap();
    let targets = RuntimeGraphTargets::new(&record, "aidlc/spaces/default/intents/record");
    RuntimeGraphReadModelUpdater::open(&fixture.store)
        .unwrap()
        .catch_up(&support::execution_id(), &targets)
        .await
        .unwrap();
    assert!(!record.join("runtime-graph.json").exists());
}

#[tokio::test]
async fn an_execution_without_a_birth_record_has_no_plan() {
    let fixture = Fixture::seeded().await;
    fixture.write_shard(
        "host-clone.md",
        &ledger(&[block(
            "2026-09-09T00:00:00Z",
            "WORKFLOW_STARTED",
            &[("Scope", "classic")],
        )]),
    );
    let error = RuntimeGraphReadModelUpdater::open(&fixture.store)
        .unwrap()
        .catch_up(&support::other_execution_id(), &fixture.targets())
        .await
        .unwrap_err();
    assert!(matches!(error, CatchUpError::PlanUnavailable), "{error:?}");
    assert_eq!(fixture.graph(), None);
}

#[tokio::test]
async fn a_shard_that_is_not_utf8_is_refused_as_publication_io() {
    let fixture = Fixture::seeded().await;
    let shard = fixture.record().join("audit").join("host-clone.md");
    std::fs::write(&shard, [0xff, 0xfe, 0x00]).unwrap();
    let error = fixture.catch_up().await.unwrap_err();
    match error {
        CatchUpError::PublicationIo { path, kind } => {
            assert_eq!(path, shard);
            assert_eq!(kind, std::io::ErrorKind::InvalidData);
        }
        other => panic!("PublicationIo を期待した: {other:?}"),
    }
}

#[tokio::test]
async fn a_graph_file_that_cannot_be_replaced_is_refused_with_its_path() {
    let fixture = Fixture::seeded().await;
    fixture.write_shard(
        "host-clone.md",
        &ledger(&[block(
            "2026-09-09T00:00:00Z",
            "WORKFLOW_STARTED",
            &[("Scope", "classic")],
        )]),
    );
    let graph_path = fixture.record().join("runtime-graph.json");
    std::fs::create_dir_all(graph_path.join("occupied")).unwrap();
    let error = fixture.catch_up().await.unwrap_err();
    match error {
        CatchUpError::PublicationIo { path, .. } => assert_eq!(path, graph_path),
        other => panic!("PublicationIo を期待した: {other:?}"),
    }
}

#[test]
fn opening_a_store_without_a_journal_table_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let store = StorePath::for_space(&root.path().join("aidlc"), &SpaceName::default());
    std::fs::create_dir_all(store.as_path().parent().unwrap()).unwrap();
    rusqlite::Connection::open(store.as_path())
        .unwrap()
        .execute_batch("CREATE TABLE unrelated(x)")
        .unwrap();
    assert!(RuntimeGraphReadModelUpdater::open(&store).is_err());
}

/// 同じ発火に終端が 2 つ以上あれば、時刻の遅いほうが結果になる（先に読んだ順ではない）。
#[tokio::test]
async fn the_latest_terminal_of_a_firing_wins_regardless_of_ledger_order() {
    let fixture = Fixture::seeded().await;
    let audit = ledger(&[
        block(
            "2026-09-09T00:00:00Z",
            "WORKFLOW_STARTED",
            &[("Scope", "classic")],
        ),
        block(
            "2026-09-09T00:00:01Z",
            "STAGE_STARTED",
            &[("Stage", "intent-capture"), ("Agent", "orchestrator")],
        ),
        block(
            "2026-09-09T00:00:03Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "aaaa0001"),
                ("Sensor ID", "aidlc-linter"),
                ("Stage slug", "intent-capture"),
            ],
        ),
        // 台帳上は「後に書かれた」が時刻は早い失敗 — 遅い成功が勝つ。
        block(
            "2026-09-09T00:00:06Z",
            "SENSOR_PASSED",
            &[("Fire id", "aaaa0001")],
        ),
        block(
            "2026-09-09T00:00:04Z",
            "SENSOR_FAILED",
            &[
                ("Fire id", "aaaa0001"),
                ("Detail path", ".aidlc-sensors/intent-capture/x.md"),
            ],
        ),
        block(
            "2026-09-09T00:00:07Z",
            "SENSOR_FIRED",
            &[
                ("Fire id", "aaaa0002"),
                ("Sensor ID", "aidlc-traceability"),
                ("Stage slug", "intent-capture"),
            ],
        ),
        // 早い成功のあとに遅い失敗 — 失敗が勝ち、詳細パスも残る。
        block(
            "2026-09-09T00:00:08Z",
            "SENSOR_PASSED",
            &[("Fire id", "aaaa0002")],
        ),
        block(
            "2026-09-09T00:00:09Z",
            "SENSOR_FAILED",
            &[
                ("Fire id", "aaaa0002"),
                ("Detail path", ".aidlc-sensors/intent-capture/y.md"),
            ],
        ),
        block(
            "2026-09-09T00:00:10Z",
            "STAGE_COMPLETED",
            &[("Stage", "intent-capture")],
        ),
    ]);
    fixture.write_shard("host-a.md", &audit);
    fixture.catch_up().await.unwrap();
    let graph: serde_json::Value = serde_json::from_str(&fixture.graph().unwrap()).unwrap();
    let firings = &graph["stages"][0]["sensor_firings"];
    assert_eq!(firings[0]["fire_id"], "aaaa0001");
    assert_eq!(firings[0]["result"], "passed");
    assert!(firings[0].get("detail_path").is_none(), "{firings}");
    assert_eq!(firings[1]["fire_id"], "aaaa0002");
    assert_eq!(firings[1]["result"], "failed");
    assert_eq!(
        firings[1]["detail_path"],
        ".aidlc-sensors/intent-capture/y.md"
    );
}
