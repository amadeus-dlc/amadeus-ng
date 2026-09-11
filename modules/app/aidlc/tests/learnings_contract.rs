//! §13 の学びの儀式を、固定本家 2.7.1 の公開境界と同じ入出力で検証する。
//!
//! 対象は `tools/aidlc-learnings.ts`（surface `:293-374` / persist `:684-948`）と
//! `aidlc-common/protocols/stage-protocol.md:1055-1110`。逐語の正本はゴールデン
//! `tests/golden/upstream-a277af21/learnings/cases.json`（本家実バイトの採取）である。
// 契約テストは固定フィクスチャの添字参照と panic を検証の合図として使う。
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_methods
)]
use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

/// ゴールデン `learnings/persist-one` が使った学びの本文とその同一性。
const GOLDEN_TEXT: &str = "ALWAYS 採取用の検証結果を記録する。";
const GOLDEN_HASH: &str = "f543ed24a72a9b57b8fac723a95fa0a2c04240a25c8a6a67229322acb3de9bd3";

/// メモリ層の合成入力（配布の正本と同じ見出しを持つ最小形）。
const PROJECT_MD: &str = "# Project-Level Rules\n\n## Forbidden\n\n## Mandated\n\n## Corrections\n";
const TEAM_MD: &str = "# Team-Level Rules\n\n## Way of Working\n\n## Corrections\n";

struct Fixture {
    root: tempfile::TempDir,
    record: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let data = root.path().join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
            fs::copy(
                repo.join("tests/golden/upstream-a277af21/data").join(name),
                data.join(name),
            )
            .unwrap();
        }
        fs::create_dir_all(root.path().join(".claude/scopes")).unwrap();
        fs::copy(
            repo.join(".claude/scopes/aidlc-bugfix.md"),
            root.path().join(".claude/scopes/aidlc-bugfix.md"),
        )
        .unwrap();
        // 本家 `intent-create` は Greenfield と走査したワークスペースで reverse-engineering を
        // SKIP する (`aidlc-utility.ts:5895-5904`)。この fixture は reverse-engineering が最初の
        // run-stage であることに依るので、ソースを 1 つ置いて Brownfield にする。
        fs::create_dir_all(root.path().join("src")).unwrap();
        fs::write(root.path().join("src/lib.rs"), "pub fn smoke() {}\n").unwrap();
        let fixture = Self {
            record: PathBuf::new(),
            root,
        };
        let created = fixture.cli(
            "aidlc-utility",
            &[
                "intent-create",
                "--scope",
                "bugfix",
                "--label",
                "learnings",
                "--arguments",
                "Capture learning behavior",
            ],
        );
        assert!(created.status.success(), "{created:?}");
        let record = fixture.active_record();
        let fixture = Self {
            root: fixture.root,
            record,
        };
        // メモリ層 2 本は space の正本である（配布は最初から持っている）。
        let memory = fixture.root.path().join("aidlc/spaces/default/memory");
        fs::create_dir_all(&memory).unwrap();
        fs::write(memory.join("project.md"), PROJECT_MD).unwrap();
        fs::write(memory.join("team.md"), TEAM_MD).unwrap();
        let next = fixture.cli("aidlc-orchestrate", &["next", "bugfix"]);
        assert!(next.status.success(), "{next:?}");
        fixture.compile();
        fixture
    }

    fn active_record(&self) -> PathBuf {
        let intents = self.root.path().join("aidlc/spaces/default/intents");
        intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        )
    }

    /// runtime-graph を作る（learnings surface はこの stage 行を前提にする）。
    fn compile(&self) {
        let envelope = format!(
            r#"{{"session_id":"11111111-2222-4333-8444-555555555555","tool_name":"Bash","tool_input":{{"command":{}}},"tool_response":{{"stdout":""}}}}"#,
            serde_json::to_string(
                "bun .claude/tools/aidlc-orchestrate.ts report --result completed"
            )
            .unwrap()
        );
        let mut child = Command::new(env!("CARGO_BIN_EXE_aidlc"))
            .args(["hook", "rebuild-stage-graph"])
            .current_dir(self.root.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.root.path())
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(envelope.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "{out:?}");
        assert!(
            self.record.join("runtime-graph.json").is_file(),
            "compile が runtime-graph を書いていない"
        );
    }

    /// `mode: pipeline` の reverse-engineering はゲートを開く前に lead/support の受領証を要る
    /// （本家 `aidlc-orchestrate.ts:7198-7227`）。developer リンクは handoff 成果物を要する
    /// （同 `aidlc-log.ts:895-979`）。
    fn complete_reverse_engineering_pipeline(&self) {
        if self.current_stage() != "reverse-engineering" {
            return;
        }
        let handoff = self
            .record
            .join("inception/reverse-engineering/developer-scan.md");
        fs::create_dir_all(handoff.parent().unwrap()).unwrap();
        fs::write(&handoff, "## Developer Code Scan Results\n").unwrap();
        let artifact = handoff
            .strip_prefix(self.root.path())
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        for (agent, artifact) in [
            ("aidlc-developer-agent", Some(artifact.as_str())),
            ("aidlc-architect-agent", None),
        ] {
            let mut args = vec!["link", "--stage", "reverse-engineering", "--link", agent];
            if let Some(artifact) = artifact {
                args.extend(["--artifact", artifact]);
            }
            let linked = self.cli("aidlc-log", &args);
            assert!(linked.status.success(), "{linked:?}");
            assert!(
                String::from_utf8_lossy(&linked.stdout)
                    .contains("\"emitted\":\"PIPELINE_LINK_COMPLETED\""),
                "{linked:?}"
            );
        }
    }

    fn cli(&self, argv0: &str, args: &[&str]) -> Output {
        let binary = self.root.path().join(argv0);
        if !binary.exists() {
            // 起動名だけを変えたいので、まずハードリンクを試す（37MB の複製を 1 フィクスチャに
            // 4 回作ると I/O で詰まる）。跨ぐファイルシステムでは複製へ落とす。
            if fs::hard_link(env!("CARGO_BIN_EXE_aidlc"), &binary).is_err() {
                tool_link::link_tool(&binary).unwrap();
            }
        }
        Command::new(binary)
            .args(args)
            .current_dir(self.root.path())
            .env_clear()
            .envs(coverage_profile_env())
            .env("HOME", self.root.path())
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap()
    }

    fn learnings(&self, args: &[&str]) -> Output {
        self.cli("aidlc-learnings", args)
    }

    fn current_stage(&self) -> String {
        let state = fs::read_to_string(self.record.join("aidlc-state.md")).unwrap();
        state
            .lines()
            .find_map(|line| line.strip_prefix("- **Current Stage**:"))
            .unwrap()
            .trim()
            .to_string()
    }

    fn record_name(&self) -> String {
        self.record
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    fn write_journal(&self, body: &str) {
        let path = self
            .record
            .join(self.phase())
            .join(self.current_stage())
            .join("memory.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    /// runtime-graph が持つその stage の `memory_path` から phase を読む。
    fn phase(&self) -> String {
        let graph: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(self.record.join("runtime-graph.json")).unwrap(),
        )
        .unwrap();
        let slug = self.current_stage();
        let path = graph["stages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["stage_slug"] == slug)
            .unwrap()["memory_path"]
            .as_str()
            .unwrap()
            .to_string();
        let segments: Vec<&str> = path.split('/').collect();
        segments[segments.len() - 3].to_string()
    }

    fn selections(&self, body: &str) -> String {
        let path = self.root.path().join("selections.json");
        fs::write(&path, body).unwrap();
        "selections.json".to_string()
    }

    fn project_md(&self) -> String {
        fs::read_to_string(
            self.root
                .path()
                .join("aidlc/spaces/default/memory/project.md"),
        )
        .unwrap()
    }

    fn team_md(&self) -> String {
        fs::read_to_string(self.root.path().join("aidlc/spaces/default/memory/team.md")).unwrap()
    }

    fn shard(&self) -> PathBuf {
        fs::read_dir(self.record.join("audit"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "md"))
            .unwrap()
    }

    fn audit(&self) -> String {
        fs::read_to_string(self.shard()).unwrap()
    }

    fn rule_learned_blocks(&self) -> Vec<String> {
        self.audit()
            .split("\n---\n")
            .filter(|block| block.contains("**Event**: RULE_LEARNED\n"))
            .map(str::to_string)
            .collect()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// 学び 1 件の選択ファイル。
fn one_learning(fixture: &Fixture, scope: &str, heading: &str, text: &str) -> String {
    format!(
        r#"{{"stage_slug":"{}","space":"default","intent":"{}","selections":[{{"candidate_id":"fixture-1","type":"learning","scope":"{scope}","heading":"{heading}","text":{},"source":"user_addition"}}]}}"#,
        fixture.current_stage(),
        fixture.record_name(),
        serde_json::to_string(text).unwrap()
    )
}

/// 何も選ばなかった選択ファイル。
fn no_learning(fixture: &Fixture) -> String {
    format!(
        r#"{{"stage_slug":"{}","space":"default","intent":"{}","selections":[]}}"#,
        fixture.current_stage(),
        fixture.record_name()
    )
}

// ---------------------------------------------------------------------------
// surface
// ---------------------------------------------------------------------------

/// ゴールデン `learnings/surface` と同じ封筒を出す（キーの並びまで逐語）。
#[test]
fn a_surfaced_stage_emits_the_upstream_envelope() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let result = fixture.learnings(&["surface", "--slug", &slug]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        format!(
            "{{\"schema_version\":1,\"stage_slug\":\"{slug}\",\"phase\":\"{}\",\"space\":\"default\",\"intent\":\"{}\",\"memory_entries_total\":0,\"candidates\":[],\"parked_open_questions\":[]}}\n",
            fixture.phase(),
            fixture.record_name()
        )
    );
}

/// 記録は候補になり、`Open questions` は昇格せず別配列に残る。
#[test]
fn entries_become_candidates_and_open_questions_stay_parked() {
    let fixture = Fixture::new();
    fixture.write_journal(
        "# Stage Memory\n\n## Interpretations\n\n- 2026-09-09T00:00:00Z — 解釈; 文脈\n\n## Deviations\n\n- 2026-09-09T00:01:00Z — 逸脱; 理由\n\n## Tradeoffs\n\n## Open questions\n\n- 2026-09-09T00:02:00Z — 確認; 次回まで\n",
    );
    let slug = fixture.current_stage();
    let result = fixture.learnings(&["surface", "--slug", &slug]);
    assert!(result.status.success(), "{result:?}");
    let surfaced: serde_json::Value = serde_json::from_str(&stdout(&result)).unwrap();
    assert_eq!(surfaced["memory_entries_total"], 3);
    let candidates = surfaced["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2, "Open questions は候補に入らない");
    assert_eq!(
        candidates[0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "id",
            "source_heading",
            "ts",
            "summary",
            "context",
            "default_scope"
        ]
    );
    assert_eq!(candidates[0]["id"], "c1");
    assert_eq!(candidates[0]["source_heading"], "Interpretations");
    assert_eq!(candidates[0]["ts"], "2026-09-09T00:00:00Z");
    assert_eq!(candidates[0]["summary"], "解釈");
    assert_eq!(candidates[0]["context"], "文脈");
    assert_eq!(candidates[0]["default_scope"], "project");
    assert_eq!(candidates[1]["id"], "c2");
    assert_eq!(candidates[1]["source_heading"], "Deviations");
    let parked = surfaced["parked_open_questions"].as_array().unwrap();
    assert_eq!(parked.len(), 1);
    assert_eq!(
        parked[0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["ts", "summary"]
    );
    assert_eq!(parked[0]["summary"], "確認");
}

/// 日誌が無い位置は 0 件で出す（ゲートを落とさない）。
#[test]
fn a_stage_without_a_journal_surfaces_zero_candidates() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let result = fixture.learnings(&["surface", "--slug", &slug]);
    assert!(result.status.success(), "{result:?}");
    assert!(stdout(&result).contains("\"memory_entries_total\":0"));
}

/// 現在位置でない slug は拒否する（逐語）。
#[test]
fn a_slug_that_is_not_the_current_stage_is_refused() {
    let fixture = Fixture::new();
    let result = fixture.learnings(&["surface", "--slug", "code-generation"]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert_eq!(
        stderr(&result),
        format!(
            "slug mismatch: requested \"code-generation\" but Current Stage is \"{}\"\n",
            fixture.current_stage()
        )
    );
    assert!(stdout(&result).is_empty());
}

/// runtime-graph が無ければ拒否する（compile が前提である）。
#[test]
fn a_missing_runtime_graph_is_refused() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.record.join("runtime-graph.json")).unwrap();
    let slug = fixture.current_stage();
    let result = fixture.learnings(&["surface", "--slug", &slug]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert!(
        stderr(&result).starts_with("runtime-graph.json not found: "),
        "{}",
        stderr(&result)
    );
}

/// runtime-graph に行が無ければ拒否する。
#[test]
fn a_stage_absent_from_the_runtime_graph_is_refused() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let graph = fixture.record.join("runtime-graph.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&graph).unwrap()).unwrap();
    value["stages"] = serde_json::Value::Array(Vec::new());
    fs::write(&graph, value.to_string()).unwrap();
    let result = fixture.learnings(&["surface", "--slug", &slug]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert_eq!(
        stderr(&result),
        format!("stage \"{slug}\" not found in runtime-graph.json\n")
    );
}

/// `--slug` が無ければ使い方を出して止まる。
#[test]
fn surface_without_a_slug_prints_its_usage() {
    let fixture = Fixture::new();
    let result = fixture.learnings(&["surface"]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert_eq!(
        stderr(&result),
        "Usage: aidlc-learnings.ts surface --slug <stage-slug> [--project-dir <path>]\n"
    );
}

// ---------------------------------------------------------------------------
// persist
// ---------------------------------------------------------------------------

/// 何も選ばれなかった回は 1 バイトも書かない（ゴールデン `learnings/persist-empty`）。
#[test]
fn an_empty_selection_writes_nothing() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&no_learning(&fixture));
    let before = fixture.project_md();
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        format!(
            "{{\"stage_slug\":\"{slug}\",\"rule_learned\":0,\"sensor_proposed\":0,\"notes\":[]}}\n"
        )
    );
    assert_eq!(fixture.project_md(), before);
    assert!(fixture.rule_learned_blocks().is_empty());
}

/// 残した学びは実践行と監査行になる（ゴールデン `learnings/persist-one` の逐語）。
#[test]
fn a_kept_learning_lands_as_a_practice_line_and_an_audit_row() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Corrections",
        GOLDEN_TEXT,
    ));
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        format!(
            "{{\"stage_slug\":\"{slug}\",\"rule_learned\":1,\"sensor_proposed\":0,\"notes\":[]}}\n"
        )
    );
    assert_eq!(
        fixture.project_md(),
        format!(
            "{PROJECT_MD}- {GOLDEN_TEXT} (learned {}) <!-- cid:{}:{slug}:{GOLDEN_HASH} -->\n",
            chrono::Utc::now().format("%Y-%m-%d"),
            fixture.record_name()
        )
    );
    assert_eq!(fixture.team_md(), TEAM_MD, "team.md は触らない");
    let rows = fixture.rule_learned_blocks();
    assert_eq!(rows.len(), 1, "{}", fixture.audit());
    assert!(rows[0].contains("## Rule Learned\n"));
    assert!(rows[0].contains(&format!("**Stage**: {slug}\n")));
    assert!(rows[0].contains("**Candidate-ID**: fixture-1\n"));
    assert!(rows[0].contains(&format!("**Content-Hash**: {GOLDEN_HASH}\n")));
    assert!(
        rows[0].contains("**Destination**: <project-dir>/aidlc/spaces/default/memory/project.md\n")
    );
    assert!(rows[0].contains("**Heading**: ## Corrections\n"));
    assert!(rows[0].contains("**Source**: user_addition\n"));
}

/// 同じ選択をもう一度流しても増えない（ゴールデン `learnings/persist-repeat`）。
#[test]
fn a_repeated_persist_adds_nothing() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Corrections",
        GOLDEN_TEXT,
    ));
    fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    let after_first = fixture.project_md();
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        format!(
            "{{\"stage_slug\":\"{slug}\",\"rule_learned\":0,\"sensor_proposed\":0,\"notes\":[]}}\n"
        )
    );
    assert_eq!(fixture.project_md(), after_first);
    assert_eq!(fixture.rule_learned_blocks().len(), 1);
}

/// 実践行だけが消えた状態からは、監査行を増やさずに行を戻す。
#[test]
fn a_deleted_practice_line_is_restored_without_a_second_audit_row() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Corrections",
        GOLDEN_TEXT,
    ));
    fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    let written = fixture.project_md();
    fs::write(
        fixture
            .root
            .path()
            .join("aidlc/spaces/default/memory/project.md"),
        PROJECT_MD,
    )
    .unwrap();
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        format!(
            "{{\"stage_slug\":\"{slug}\",\"rule_learned\":0,\"sensor_proposed\":0,\"notes\":[]}}\n"
        ),
        "監査行は既に在るので数えない"
    );
    assert_eq!(fixture.project_md(), written, "実践行が戻る");
    assert_eq!(fixture.rule_learned_blocks().len(), 1);
}

/// 監査行だけが消えた状態からは、実践行を増やさずに監査行を戻す。
#[test]
fn a_deleted_audit_row_is_restored_without_a_second_practice_line() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Corrections",
        GOLDEN_TEXT,
    ));
    fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    let written = fixture.project_md();
    let stripped: String = fixture
        .audit()
        .split("\n---\n")
        .filter(|block| !block.contains("**Event**: RULE_LEARNED\n"))
        .collect::<Vec<_>>()
        .join("\n---\n");
    fs::write(fixture.shard(), stripped).unwrap();
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        format!(
            "{{\"stage_slug\":\"{slug}\",\"rule_learned\":1,\"sensor_proposed\":0,\"notes\":[]}}\n"
        )
    );
    assert_eq!(fixture.project_md(), written, "実践行は二重にならない");
    assert_eq!(fixture.rule_learned_blocks().len(), 1);
}

/// `team` の学びは team.md へ落ちる（project.md は触らない）。
#[test]
fn a_team_scoped_learning_lands_in_the_team_method_file() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "team",
        "Way of Working",
        "ALWAYS squash-merge",
    ));
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(fixture.project_md(), PROJECT_MD);
    // 追記は**次の `## ` 見出しの直前**に入る（本家 `appendUnderHeading`）。節末尾の空行は
    // 残るので、見出しの直後の行になるとは限らない。
    let team = fixture.team_md();
    assert!(team.contains("\n- ALWAYS squash-merge (learned "), "{team}");
    assert!(
        team.find("- ALWAYS squash-merge")
            .zip(team.find("## Corrections"))
            .is_some_and(|(line, next)| line < next),
        "`## Way of Working` の節の中に入る: {team}"
    );
    assert!(
        fixture.rule_learned_blocks()[0]
            .contains("**Destination**: <project-dir>/aidlc/spaces/default/memory/team.md\n")
    );
}

/// 正本に無い見出しは作ってから足す。
#[test]
fn a_routed_heading_the_method_file_lacks_is_created_first() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Testing Posture",
        "ALWAYS run the suite",
    ));
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert!(
        fixture
            .project_md()
            .ends_with("\n## Testing Posture\n- ALWAYS run the suite (learned ")
            || fixture
                .project_md()
                .contains("\n## Testing Posture\n- ALWAYS run the suite (learned "),
        "{}",
        fixture.project_md()
    );
    assert!(fixture.rule_learned_blocks()[0].contains("**Heading**: ## Testing Posture\n"));
}

/// 素性は surface の時点で固定される — 後からカーソルが動いても書込先は変わらない。
#[test]
fn the_pinned_intent_wins_over_a_later_cursor_move() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Corrections",
        GOLDEN_TEXT,
    ));
    let pinned = fixture.record_name();
    // 別の作業を作ってカーソルを移す（選択ファイルは前の作業を指したまま）。
    let created = fixture.cli(
        "aidlc-utility",
        &[
            "intent-create",
            "--scope",
            "bugfix",
            "--label",
            "other",
            "--arguments",
            "Another piece of work",
        ],
    );
    assert!(created.status.success(), "{created:?}");
    assert_ne!(
        fixture
            .active_record()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
        pinned,
        "カーソルは新しい作業を指す"
    );
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert!(
        fixture
            .project_md()
            .contains(&format!("cid:{pinned}:{slug}:{GOLDEN_HASH}")),
        "固定した作業の印で書く: {}",
        fixture.project_md()
    );
    assert_eq!(
        fixture.rule_learned_blocks().len(),
        1,
        "監査は固定した作業のシャードへ落ちる"
    );
}

/// `--slug` と選択ファイルの食い違いは拒否する（ゴールデン `learnings/wrong-stage` の逐語）。
#[test]
fn a_slug_that_differs_from_the_selections_file_is_refused() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Corrections",
        GOLDEN_TEXT,
    ));
    let result = fixture.learnings(&[
        "persist",
        "--slug",
        "code-generation",
        "--selections-json",
        &path,
    ]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert_eq!(
        stderr(&result),
        format!(
            "slug mismatch: selections were surfaced for \"{slug}\" but persist requested \"code-generation\"\n"
        )
    );
    assert!(fixture.rule_learned_blocks().is_empty());
}

/// `space` の欄が無い選択ファイルは拒否する（ゴールデン `learnings/malformed-selection` の逐語）。
#[test]
fn a_selections_file_without_a_space_is_refused() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&format!(r#"{{"stage_slug":"{slug}","selections":[]}}"#));
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert_eq!(
        stderr(&result),
        "selections-json is malformed: missing or non-string space (bind it from surface's output)\n"
    );
}

/// 壊れた選択ファイル・不在・不正な要素はそれぞれ拒否する。
#[test]
fn malformed_selection_files_are_refused_one_reason_at_a_time() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let record = fixture.record_name();
    let cases: Vec<(String, String)> = vec![
        (
            "not json".to_string(),
            "selections-json is malformed: ".to_string(),
        ),
        (
            r#"[]"#.to_string(),
            "selections-json is malformed: expected { stage_slug, space, intent, selections[] }"
                .to_string(),
        ),
        (
            format!(
                r#"{{"stage_slug":"{slug}","space":"Default","intent":"{record}","selections":[]}}"#
            ),
            "selections-json is malformed: space must be a lowercase slug".to_string(),
        ),
        (
            format!(r#"{{"stage_slug":"{slug}","space":"default","intent":7,"selections":[]}}"#),
            "selections-json is malformed: intent must be a string or null".to_string(),
        ),
        (
            format!(r#"{{"stage_slug":"{slug}","space":"default","selections":[]}}"#),
            "selections-json is malformed: intent must be a string or null".to_string(),
        ),
        (
            format!(r#"{{"stage_slug":"{slug}","space":"default","intent":null,"selections":[]}}"#),
            "This build has no flat workspace layout".to_string(),
        ),
        (
            format!(
                r#"{{"stage_slug":"{slug}","space":"default","intent":"../escape","selections":[]}}"#
            ),
            "selections-json is malformed: intent must be a non-empty record-directory name"
                .to_string(),
        ),
        (
            format!(
                r#"{{"stage_slug":"{slug}","space":"default","intent":"{record}","selections":[7]}}"#
            ),
            "selections-json malformed: each selection must be an object".to_string(),
        ),
        (
            format!(
                r#"{{"stage_slug":"{slug}","space":"default","intent":"{record}","selections":[{{}}]}}"#
            ),
            "selections-json malformed: selection missing candidate_id".to_string(),
        ),
        (
            format!(
                r#"{{"stage_slug":"{slug}","space":"default","intent":"{record}","selections":[{{"candidate_id":"c1"}}]}}"#
            ),
            "selections-json malformed: learning selection needs heading + text".to_string(),
        ),
    ];
    for (body, expected) in cases {
        let path = fixture.selections(&body);
        let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
        assert_eq!(result.status.code(), Some(1), "{body}: {result:?}");
        assert!(
            stderr(&result).starts_with(&expected),
            "{body}\n実測: {}",
            stderr(&result)
        );
    }
    // 選択ファイル自体が無い場合。
    let missing = fixture.learnings(&[
        "persist",
        "--slug",
        &slug,
        "--selections-json",
        "no-such-file.json",
    ]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(stderr(&missing).starts_with("selections-json not found: "));
    assert!(fixture.rule_learned_blocks().is_empty());
}

/// センサーの選択はこの build に無い（黙って飛ばさず拒否する）。
#[test]
fn a_sensor_selection_is_refused_because_this_build_does_not_wire_it() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&format!(
        r#"{{"stage_slug":"{slug}","space":"default","intent":"{}","selections":[{{"candidate_id":"c1","type":"sensor","origin_stage":"{slug}","manifest_fields":{{"id":"x","kind":"script","command":"true","default_severity":"advisory","description":"d","matches":"**/*.md"}}}}]}}"#,
        fixture.record_name()
    ));
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert!(
        stderr(&result).contains("the sensor selection type is not wired"),
        "{}",
        stderr(&result)
    );
    assert!(fixture.rule_learned_blocks().is_empty());
}

/// 固定した記録が消えていれば拒否する。
#[test]
fn a_selections_file_naming_a_missing_record_is_refused() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&format!(
        r#"{{"stage_slug":"{slug}","space":"default","intent":"260101-gone-0000","selections":[]}}"#
    ));
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert_eq!(
        stderr(&result),
        "cannot persist selections for missing intent record \"260101-gone-0000\" in space \"default\". Re-run the stage's surface step and regenerate the selections file, then retry.\n"
    );
}

/// 同じ本文が 2 度あっても実践行は 1 本だけになる。
#[test]
fn the_same_text_twice_in_one_batch_is_written_once() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let record = fixture.record_name();
    let body = format!(
        r#"{{"stage_slug":"{slug}","space":"default","intent":"{record}","selections":[{{"candidate_id":"c1","type":"learning","scope":"project","heading":"Corrections","text":{0}}},{{"candidate_id":"c2","type":"learning","scope":"project","heading":"Corrections","text":{0}}}]}}"#,
        serde_json::to_string(GOLDEN_TEXT).unwrap()
    );
    let path = fixture.selections(&body);
    let result = fixture.learnings(&["persist", "--slug", &slug, "--selections-json", &path]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        stdout(&result),
        format!(
            "{{\"stage_slug\":\"{slug}\",\"rule_learned\":1,\"sensor_proposed\":0,\"notes\":[]}}\n"
        )
    );
    assert_eq!(fixture.project_md().matches(GOLDEN_HASH).count(), 1);
    assert_eq!(fixture.rule_learned_blocks().len(), 1);
}

/// 学びの記録は runtime-graph の `learnings_captured` を実データで駆動する。
///
/// 儀式は「完了メッセージ（ゲート開放）と承認ゲートのあいだ」に走るので、`RULE_LEARNED` は
/// その stage の開始〜完了の窓に入る。前スライスで空・ゼロしか観測できていなかった対が、
/// ここで実データに動く。
#[test]
fn a_persisted_learning_drives_the_runtime_graph_counter() {
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    fixture.complete_reverse_engineering_pipeline();
    let opened = fixture.cli(
        "aidlc-orchestrate",
        &["report", "--result", "awaiting-approval"],
    );
    assert!(opened.status.success(), "{opened:?}");
    assert!(
        !stdout(&opened).contains("\"kind\":\"error\""),
        "ゲートが開かなかった: {}",
        stdout(&opened)
    );
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Corrections",
        GOLDEN_TEXT,
    ));
    assert!(
        fixture
            .learnings(&["persist", "--slug", &slug, "--selections-json", &path])
            .status
            .success()
    );
    let approved = fixture.cli(
        "aidlc-orchestrate",
        &["report", "--result", "completed", "--user-input", "A"],
    );
    assert!(approved.status.success(), "{approved:?}");
    // ビジネス拒否は exit 0 の `error` directive として出る — 終了コードだけでは通らない。
    assert!(
        !stdout(&approved).contains("\"kind\":\"error\""),
        "承認が拒否された: {}",
        stdout(&approved)
    );
    fixture.compile();
    let graph: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture.record.join("runtime-graph.json")).unwrap(),
    )
    .unwrap();
    let row = graph["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["stage_slug"] == slug)
        .unwrap()
        .clone();
    assert_eq!(row["outcome"], "approved", "{row}");
    assert_eq!(row["learnings_captured"]["from_user_addition"], 1, "{row}");
    assert_eq!(row["learnings_captured"]["from_orchestrator"], 0, "{row}");
}

// ---------------------------------------------------------------------------
// ゴールデン照合
// ---------------------------------------------------------------------------

/// 固定本家の実測（`learnings/cases.json`）と同じ拒否文言を出す。
#[test]
fn the_refusals_match_the_upstream_capture() {
    let corpus: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tests/golden/upstream-a277af21/learnings/cases.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let observation = |id: &str| -> serde_json::Value {
        corpus["observations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap()
            .clone()
    };
    let fixture = Fixture::new();
    let slug = fixture.current_stage();
    let path = fixture.selections(&one_learning(
        &fixture,
        "project",
        "Corrections",
        GOLDEN_TEXT,
    ));

    let wrong_stage = fixture.learnings(&[
        "persist",
        "--slug",
        "code-generation",
        "--selections-json",
        &path,
    ]);
    // ゴールデンは `requirements-analysis` を surface している。stage 名だけを合わせる。
    let expected = observation("learnings/wrong-stage")["output"]["stderr"]
        .as_str()
        .unwrap()
        .replace("requirements-analysis", &slug);
    assert_eq!(stderr(&wrong_stage), expected);
    assert_eq!(
        wrong_stage.status.code(),
        observation("learnings/wrong-stage")["output"]["exit_code"]
            .as_i64()
            .map(|code| code as i32)
    );

    let malformed_path =
        fixture.selections(&format!(r#"{{"stage_slug":"{slug}","selections":[]}}"#));
    let malformed = fixture.learnings(&[
        "persist",
        "--slug",
        &slug,
        "--selections-json",
        &malformed_path,
    ]);
    assert_eq!(
        stderr(&malformed),
        observation("learnings/malformed-selection")["output"]["stderr"]
            .as_str()
            .unwrap()
    );
}

#[path = "../../../../tests/support/coverage_profile_env.rs"]
mod coverage_profile_env;
use coverage_profile_env::coverage_profile_env;

#[path = "../../../../tests/support/tool_link.rs"]
mod tool_link;
