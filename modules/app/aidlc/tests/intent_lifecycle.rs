//! 合成ルートの縦切り — 鋳造から報告まで（コマンド側 → RMU → クエリ側の一巡）。
//!
//! `intent-create` が 2 つの集約をジャーナルへ書き、RMU がそれを `aidlc-state.md` と
//! 監査シャードへ投影し、`next` がその投影を読んで directive を出し、`report` が遷移を
//! コミットして再び投影される。**両側と中間が実際に噛み合うか**を見る唯一のテストである。
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use aidlc::execution_cursor::ExecutionCursor;
use chrono::Utc;
use core_command_domain::orchestration::AutonomyMode;
use core_command_domain::workspace::{HumanTurns, SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, IntentRepositoryImpl,
};
use core_command_use_case::orchestration::{IntentExecutionRepository as _, IntentRepository as _};
use core_infrastructure::canon_json::{JsonValue, parse};

/// 合成グラフが宣言するレビュアー (b48)。
const REVIEWER: &str = "aidlc-architecture-reviewer-agent";

/// ジャーナルに居ない実行の識別子 (b49 — ポートの失敗を見る用)。
const ABSENT_EXECUTION: &str = "018f3b2c-4d5e-7f60-8abc-def012345678";

/// `practices-discovery` が宣言する支援エージェント (b49)。
const PRACTICES_SUPPORT_AGENTS: [&str; 2] = ["aidlc-developer-agent", "aidlc-quality-agent"];

struct Workspace {
    root: tempfile::TempDir,
}

impl Workspace {
    /// 定義 3 入力と memory 層を書き、ストアの置き場だけ用意した fresh なワークスペース。
    ///
    /// **カーソルも record も置かない** — intent がまだ生まれていない状態から始める。
    fn create() -> Workspace {
        Workspace::with_execution("ALWAYS")
    }

    /// 最初のゲート付きステージ (`domain-design`) を CONDITIONAL にした合成グラフ。
    fn conditional() -> Workspace {
        Workspace::with_execution("CONDITIONAL")
    }

    /// Construction フェーズを持つ合成グラフ (walking-skeleton ゲートの往復を見る用)。
    ///
    /// `functional-design` が Construction の最初の EXECUTE — すなわち skeleton-gate である。
    /// `code-generation` は scope グリッドで SKIP にしてあり、`--single` の scope 外ガードの
    /// 的にもなる。
    fn with_construction() -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_construction_definition();
        // memory 層は**置かない** — 規則ファイルが無ければ配信計画が空になり、`next` は
        // steering 連鎖ではなく素の run-stage を出す。この試験が見たいのは `gate` の 3 値
        // だけなので、連鎖を挟まない最短形にする。
        fs::create_dir_all(workspace.path("aidlc/spaces/default/intents")).expect("intents");
        workspace
    }

    /// `domain-design` にレビュアーを宣言した合成グラフ (b48 — 受領証の往復を見る用)。
    ///
    /// `class` は `review_class:` の値 (`None` = 宣言なし = adversarial 扱い)、`cap` は
    /// scope の `review_cap:` である。
    fn with_reviewer(class: Option<&str>, cap: Option<&str>) -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_reviewed_definition(class, cap);
        workspace.write_memory_and_store_dir();
        workspace
    }

    /// `practices-discovery` を持つ 3 段の合成グラフ (b49 — 昇格の往復を見る用)。
    ///
    /// 支援エージェントは 2 本宣言する (contributions の検査を見るため)。memory 層には
    /// `team.md` / `project.md` の正本を置き、ドラフト 2 本と contributions は
    /// `<root>/drafts/` に置く。
    fn with_practices() -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_practices_definition();
        workspace.write_memory_and_store_dir();
        workspace.write_memory_targets();
        workspace.write_drafts();
        workspace
    }

    fn with_execution(execution: &str) -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_definition(execution);
        workspace.write_memory_and_store_dir();
        workspace
    }

    fn write_memory_and_store_dir(&self) {
        let memory = self.path("aidlc/spaces/default/memory");
        fs::create_dir_all(&memory).expect("memory");
        fs::write(memory.join("org.md"), "# Org\n\n規則なし。\n").expect("org.md");
        // ストアの親 (`intents/`) は upstream の既存ディレクトリ扱いなので先に作る。
        fs::create_dir_all(self.path("aidlc/spaces/default/intents")).expect("intents");
    }

    /// メモリ層の正本 2 本 (b49 — 昇格の書込先)。
    fn write_memory_targets(&self) {
        let memory = self.path("aidlc/spaces/default/memory");
        fs::write(
            memory.join("team.md"),
            "# Team\n\n## Way of Working\nold way.\n\n## Walking Skeleton\nold skeleton.\n\n## Testing Posture\nold posture.\n\n## Deployment\nold deployment.\n\n## Code Style\nold style.\n",
        )
        .expect("team.md");
        fs::write(
            memory.join("project.md"),
            "# Project\n\n## Mandated\n\n## Forbidden\n\n## Corrections\n",
        )
        .expect("project.md");
    }

    /// ドラフト 2 本と contributions 2 本 (b49 — 昇格の材料)。
    fn write_drafts(&self) {
        let drafts = self.path("drafts");
        fs::create_dir_all(drafts.join("contributions")).expect("contributions");
        fs::write(
            drafts.join("team-practices.md"),
            "## Way of Working\ntrunk-based.\n",
        )
        .expect("team-practices.md");
        fs::write(
            drafts.join("discovered-rules.md"),
            "## Mandated\nALWAYS review.\n\n## Forbidden\nNEVER force-push.\n",
        )
        .expect("discovered-rules.md");
        for agent in PRACTICES_SUPPORT_AGENTS {
            fs::write(
                drafts.join("contributions").join(format!("{agent}.md")),
                format!("**Collaborator:** {agent}\n\n所見。\n"),
            )
            .expect("contribution");
        }
    }

    fn draft(&self, name: &str) -> String {
        self.path("drafts")
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    fn memory_file(&self, name: &str) -> String {
        fs::read_to_string(self.path("aidlc/spaces/default/memory").join(name))
            .expect("memory のファイルは読める")
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn project_dir(&self) -> &Path {
        self.root.path()
    }

    fn record_dir(&self) -> Option<PathBuf> {
        let cursor =
            fs::read_to_string(self.path("aidlc/spaces/default/intents/active-intent")).ok()?;
        Some(
            self.path("aidlc/spaces/default/intents")
                .join(cursor.trim()),
        )
    }

    fn state_file(&self) -> Option<String> {
        fs::read_to_string(self.record_dir()?.join("aidlc-state.md")).ok()
    }

    fn execution_cursor(&self) -> Option<String> {
        fs::read_to_string(self.record_dir()?.join(".aidlc-execution")).ok()
    }

    fn audit_shard(&self) -> Option<String> {
        let audit = self.record_dir()?.join("audit");
        let entry = fs::read_dir(audit).ok()?.filter_map(Result::ok).next()?;
        fs::read_to_string(entry.path()).ok()
    }

    /// 監査シャードへ `HUMAN_TURN` を 1 行追記する（b50 — フックが書く一次の事実の代役）。
    ///
    /// この行は**我々のイベントの投影ではない**（フック `aidlc-record-human-turn.ts` が
    /// シャードへ直接書く）。テストでも同じ形で、投影を経由せずファイルへ追記する。
    fn append_human_turn(&self, timestamp: &str) {
        let audit = self
            .record_dir()
            .expect("カーソルが据わっている")
            .join("audit");
        let entry = fs::read_dir(&audit)
            .expect("監査ディレクトリは在る")
            .filter_map(Result::ok)
            .next()
            .expect("シャードは 1 つ以上ある");
        let mut content = fs::read_to_string(entry.path()).expect("シャードは読める");
        content.push_str(&format!(
            "\n## Human Turn\n**Timestamp**: {timestamp}\n**Event**: HUMAN_TURN\n\n---\n"
        ));
        fs::write(entry.path(), content).expect("シャードは書ける");
    }

    /// 状態ファイルから 1 行を落とす（欄の手編集の再現）。
    fn strip_state_line(&self, prefix: &str) {
        let path = self
            .record_dir()
            .expect("カーソルが据わっている")
            .join("aidlc-state.md");
        let content = fs::read_to_string(&path).expect("状態ファイルは読める");
        let stripped: Vec<&str> = content
            .lines()
            .filter(|line| !line.starts_with(prefix))
            .collect();
        fs::write(&path, format!("{}\n", stripped.join("\n"))).expect("状態ファイルは書ける");
    }

    /// ジャーナル行数を 2 つに割って数える — (定義ストリーム, それ以外)。
    ///
    /// 定義 id はハーネス名 (`claude`) なので、UUID を名乗る intent / 実行の 2 集約と同じ表に
    /// 居ても `aid` で分かれる。
    fn journal_rows(&self) -> (usize, usize) {
        let store = self.path("aidlc/spaces/default/intents/.aidlc-store.sqlite");
        let connection = rusqlite::Connection::open(&store).expect("ストアは開ける");
        let count = |predicate: &str| -> usize {
            connection
                .query_row(
                    &format!("SELECT COUNT(*) FROM journal WHERE aid {predicate} 'claude'"),
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("行数は数えられる")
                .try_into()
                .expect("行数は非負")
        };
        (count("="), count("<>"))
    }

    /// この record が握る実行を autonomous へ切り替える（`set-autonomy` は未配線 — #72）。
    ///
    /// テストダブルではなく**実物のリポジトリ**を開いて `switch_autonomy` を store する。
    /// 集約側の遷移とジャーナルへの書込みを実駆動で通すことで、その後の `park` が読むのは
    /// 「本当に autonomous になった歴史」になる。
    async fn switch_to_autonomous(&self) {
        let record = self.record_dir().expect("カーソルが据わっている");
        let execution_id = ExecutionCursor::read(&record)
            .expect("実行カーソルは読める")
            .expect("実行カーソルは据わっている")
            .execution_id()
            .clone();
        let space = SpaceName::parse("default").expect("既定の空間名");
        let store = StorePath::for_space(&self.path("aidlc"), &space);
        let mut execution_repository =
            IntentExecutionRepositoryImpl::open(&store).expect("ストアは開ける");
        let intent_repository = IntentRepositoryImpl::open(&store).expect("ストアは開ける");
        let mut aggregate = execution_repository
            .find_by_id(&execution_id)
            .await
            .expect("実行は再構成できる");
        let intent = intent_repository
            .find_by_id(aggregate.intent_id())
            .await
            .expect("計画は引ける");
        let event = aggregate
            .switch_autonomy(
                &intent,
                AutonomyMode::Autonomous,
                &HumanTurns::default(),
                false,
                Utc::now(),
            )
            .expect("Running な実行はモードを切り替えられる");
        execution_repository
            .store(&event, &aggregate)
            .await
            .expect("切替は書ける");
    }

    fn write_definition(&self, execution: &str) {
        let data = self.path(".claude/tools/data");
        let scopes = self.path(".claude/scopes");
        fs::create_dir_all(&data).expect("data");
        fs::create_dir_all(&scopes).expect("scopes");
        fs::write(
            data.join("harness.json"),
            r#"{"name":"claude","harnessDir":".claude","rulesSubdir":"rules"}"#,
        )
        .expect("harness.json");
        let node = |slug: &str, number: &str, name: &str, phase: &str, execution: &str| {
            format!(
                r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                     "execution":"{execution}","mode":"inline","lead_agent":"orchestrator",
                     "scopes":["classic"]}}"#
            )
        };
        fs::write(
            data.join("stage-graph.json"),
            format!(
                "[{},{},{}]",
                // initialization は CONDITIONAL にできない (計画の不変条件)。
                node(
                    "state-init",
                    "0.1",
                    "State Init",
                    "initialization",
                    "ALWAYS"
                ),
                node(
                    "domain-design",
                    "1.1",
                    "Domain Design",
                    "inception",
                    execution
                ),
                node(
                    "contract-design",
                    "1.2",
                    "Contract Design",
                    "inception",
                    "ALWAYS"
                ),
            ),
        )
        .expect("stage-graph.json");
        fs::write(
            data.join("scope-grid.json"),
            r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"EXECUTE"}}}"#,
        )
        .expect("scope-grid.json");
        fs::write(
            scopes.join("aidlc-classic.md"),
            "---\nname: classic\n---\n\n# Classic\n",
        )
        .expect("scope identity");
    }

    /// `domain-design` にレビュアーを宣言した 3 段の合成グラフ (b48)。
    fn write_reviewed_definition(&self, class: Option<&str>, cap: Option<&str>) {
        let data = self.path(".claude/tools/data");
        let scopes = self.path(".claude/scopes");
        fs::create_dir_all(&data).expect("data");
        fs::create_dir_all(&scopes).expect("scopes");
        fs::write(
            data.join("harness.json"),
            r#"{"name":"claude","harnessDir":".claude","rulesSubdir":"rules"}"#,
        )
        .expect("harness.json");
        let node = |slug: &str, number: &str, name: &str, phase: &str, extra: &str| {
            format!(
                r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                     "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator",
                     "scopes":["classic"]{extra}}}"#
            )
        };
        let class = class.map_or(String::new(), |class| {
            format!(r#","review_class":"{class}""#)
        });
        let reviewed = format!(r#","reviewer":"{REVIEWER}"{class}"#);
        fs::write(
            data.join("stage-graph.json"),
            format!(
                "[{},{},{}]",
                node("state-init", "0.1", "State Init", "initialization", ""),
                node(
                    "domain-design",
                    "1.1",
                    "Domain Design",
                    "inception",
                    &reviewed
                ),
                node("contract-design", "1.2", "Contract Design", "inception", ""),
            ),
        )
        .expect("stage-graph.json");
        fs::write(
            data.join("scope-grid.json"),
            r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","contract-design":"EXECUTE"}}}"#,
        )
        .expect("scope-grid.json");
        let cap = cap.map_or(String::new(), |cap| format!("review_cap: {cap}\n"));
        fs::write(
            scopes.join("aidlc-classic.md"),
            format!("---\nname: classic\n{cap}---\n\n# Classic\n"),
        )
        .expect("scope identity");
    }

    /// `practices-discovery` を持つ 3 段の合成グラフ (b49)。
    fn write_practices_definition(&self) {
        let data = self.path(".claude/tools/data");
        let scopes = self.path(".claude/scopes");
        fs::create_dir_all(&data).expect("data");
        fs::create_dir_all(&scopes).expect("scopes");
        fs::write(
            data.join("harness.json"),
            r#"{"name":"claude","harnessDir":".claude","rulesSubdir":"rules"}"#,
        )
        .expect("harness.json");
        let agents = PRACTICES_SUPPORT_AGENTS
            .map(|agent| format!("\"{agent}\""))
            .join(",");
        let node = |slug: &str, number: &str, name: &str, phase: &str, extra: &str| {
            format!(
                r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                     "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator",
                     "scopes":["classic"]{extra}}}"#
            )
        };
        fs::write(
            data.join("stage-graph.json"),
            format!(
                "[{},{},{}]",
                node("state-init", "0.1", "State Init", "initialization", ""),
                node(
                    "practices-discovery",
                    "1.1",
                    "Practices Discovery",
                    "inception",
                    &format!(r#","support_agents":[{agents}]"#)
                ),
                node("contract-design", "1.2", "Contract Design", "inception", ""),
            ),
        )
        .expect("stage-graph.json");
        fs::write(
            data.join("scope-grid.json"),
            r#"{"classic":{"stages":{"state-init":"EXECUTE","practices-discovery":"EXECUTE","contract-design":"EXECUTE"}}}"#,
        )
        .expect("scope-grid.json");
        fs::write(
            scopes.join("aidlc-classic.md"),
            "---\nname: classic\n---\n\n# Classic\n",
        )
        .expect("scope identity");
    }

    /// Construction を含む 4 段の合成グラフ (b47 — walking-skeleton ゲートの往復)。
    fn write_construction_definition(&self) {
        let data = self.path(".claude/tools/data");
        let scopes = self.path(".claude/scopes");
        fs::create_dir_all(&data).expect("data");
        fs::create_dir_all(&scopes).expect("scopes");
        fs::write(
            data.join("harness.json"),
            r#"{"name":"claude","harnessDir":".claude","rulesSubdir":"rules"}"#,
        )
        .expect("harness.json");
        let node = |slug: &str, number: &str, name: &str, phase: &str| {
            format!(
                r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                     "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator",
                     "scopes":["classic"]}}"#
            )
        };
        fs::write(
            data.join("stage-graph.json"),
            format!(
                "[{},{},{},{}]",
                node("state-init", "0.1", "State Init", "initialization"),
                node("domain-design", "1.1", "Domain Design", "inception"),
                node(
                    "functional-design",
                    "3.1",
                    "Functional Design",
                    "construction"
                ),
                node("code-generation", "3.2", "Code Generation", "construction"),
            ),
        )
        .expect("stage-graph.json");
        fs::write(
            data.join("scope-grid.json"),
            r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE","functional-design":"EXECUTE","code-generation":"SKIP"}}}"#,
        )
        .expect("scope-grid.json");
        fs::write(
            scopes.join("aidlc-classic.md"),
            "---\nname: classic\n---\n\n# Classic\n",
        )
        .expect("scope identity");
    }
}

async fn invoke(workspace: &Workspace, argv0: &str, args: &[&str]) -> aidlc::runtime::Completion {
    let mut owned: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
    owned.push("--project-dir".to_string());
    owned.push(workspace.project_dir().to_string_lossy().into_owned());
    aidlc::runtime::run(argv0, &owned, workspace.project_dir()).await
}

fn line_of(completion: &aidlc::runtime::Completion) -> JsonValue {
    let line = completion
        .line()
        .unwrap_or_else(|| panic!("stdout に 1 行が要る: {completion:?}"));
    assert!(!line.contains('\n'), "1 行である: {line}");
    parse(line).expect("JSON として読める")
}

/// `report` を 1 回叩き、directive の `kind` と本文（`message` か `reason`）を返す。
async fn report_directive(workspace: &Workspace, args: &[&str]) -> (String, String) {
    let mut argv = vec!["report"];
    argv.extend_from_slice(args);
    let completion = invoke(workspace, "aidlc-orchestrate", &argv).await;
    assert_eq!(
        completion.code(),
        0,
        "ビジネス拒否も成功も exit 0: {completion:?}"
    );
    let directive = line_of(&completion);
    let kind = string_of(&directive, "kind");
    let body = if kind == "done" {
        string_of(&directive, "reason")
    } else {
        string_of(&directive, "message")
    };
    (kind, body)
}

/// 自己防衛拒否は成功面を出さず、障害の分類と完全な所在を保つ。
fn assert_refused_at(completion: &aidlc::runtime::Completion, prefix: &str, path: &Path) {
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(completion.line(), None, "拒否では成功directiveを出さない");
    assert_eq!(
        completion.diagnostic(),
        Some(format!("{prefix}{}", path.display()).as_str())
    );
}

fn string_of(value: &JsonValue, key: &str) -> String {
    match value {
        JsonValue::Object(members) => match members.get(key) {
            Some(JsonValue::String(text)) => text.clone(),
            other => panic!("{key} は文字列であるべき: {other:?}"),
        },
        other => panic!("オブジェクトであるべき: {other:?}"),
    }
}

/// 鋳造は record を作り、カーソルを据え、リードモデル 2 面を投影する。
#[tokio::test]
async fn creating_an_intent_projects_both_read_model_faces() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    // 記録名の形は `<yymmdd>-<kebab ラベル>-<id8>`。3 つの成分を**それぞれ独立に**
    // 確かめる（記録名から取り出した値を記録名に突き合わせても何も検査したことにならない）。
    let record = string_of(&line_of(&completion), "record");
    let (head, id8) = record.rsplit_once('-').expect("`-<id8>` で終わる");
    assert_eq!(id8.len(), 8, "{record}");
    assert!(
        id8.chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase() && c.is_ascii_hexdigit()),
        "id8 は小文字 16 進: {record}"
    );
    let date = head.get(..6).unwrap_or_default();
    assert!(
        date.len() == 6 && date.chars().all(|c| c.is_ascii_digit()),
        "先頭は yymmdd: {record}"
    );
    assert_eq!(head.get(6..), Some("-demo-run"), "{record}");

    // カーソルが据わり、record が実在する。
    let record = workspace.record_dir().expect("カーソルが据わっている");
    assert!(record.is_dir());

    // RMU が 2 面を描いた。
    let state = workspace.state_file().expect("状態ファイルが投影された");
    assert!(state.contains("- **Scope**: classic"), "{state}");
    assert!(state.contains("state-init"), "{state}");
    let audit = workspace.audit_shard().expect("監査シャードが投影された");
    assert!(audit.contains("WORKFLOW_STARTED"), "{audit}");
}

/// 鋳造の直後に `next` が同じワークスペースで進める（A-1 追加条項の受入）。
///
/// カーソルと record が用意されていなければ、`next` は record を解決できずここで倒れる。
#[tokio::test]
async fn next_runs_against_the_freshly_created_intent() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;

    let completion = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    let directive = line_of(&completion);
    // 規則束が小さいので 1 部で収まり、そのまま load-steering が届く。
    assert_eq!(string_of(&directive, "kind"), "load-steering");
    // 誕生の投影が initialization を完了させ、最初のゲート付きステージへ着地している
    // ので、`next` が届けるのは `state-init` ではなく `domain-design` の規則である。
    assert_eq!(string_of(&directive, "stage"), "domain-design");
}

/// 破損した復旧計画は読取・書込の全入口を止め、真実記録や公開位置を変えない。
#[tokio::test]
async fn next_rejects_a_corrupt_restoration_plan_without_recreating_files() {
    let workspace = Workspace::with_practices();
    let created = invoke(
        &workspace,
        "aidlc-utility",
        &[
            "intent-create",
            "--scope",
            "classic",
            "--label",
            "corrupt restoration",
        ],
    )
    .await;
    assert_eq!(created.code(), 0, "{created:?}");
    let ready = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;
    assert_eq!(ready.code(), 0, "{ready:?}");
    let ready_directive = line_of(&ready);
    assert_eq!(string_of(&ready_directive, "kind"), "load-steering");
    let token = string_of(&ready_directive, "continue_token");
    let before_journal = workspace.journal_rows();
    let audit = workspace.audit_shard().expect("公開済み監査");
    let record = workspace.record_dir().expect("記録先");
    let path = StorePath::for_space(&workspace.path("aidlc"), &SpaceName::default());
    let database = rusqlite::Connection::open(path.as_path()).expect("実ストアを開く");
    let position = || {
        database.query_row(
        "SELECT last_global_seq FROM amadeus_projection_checkpoint WHERE projection='orchestration'",
        [], |row| row.get::<_, i64>(0),
    ).expect("公開位置")
    };
    let before = position();
    assert_eq!(database.execute(
        "UPDATE amadeus_publication_snapshot SET plan_digest='corrupt' WHERE projection='orchestration'", [],
    ).expect("保存計画の破損を注入"), 1);
    fs::remove_file(record.join("aidlc-state.md")).expect("状態を失わせる");
    let expected = format!(
        "aidlc-orchestrate: projection restoration: read: io: InvalidData at {}",
        path.as_path().display()
    );
    for args in [vec!["next"], vec!["continue", token.as_str()]] {
        let completion = invoke(&workspace, "aidlc-orchestrate", &args).await;
        assert_eq!(
            completion.code(),
            0,
            "読み取り拒否はerror directive: {completion:?}"
        );
        assert_eq!(completion.diagnostic(), None);
        let directive = line_of(&completion);
        assert_eq!(string_of(&directive, "kind"), "error");
        assert_eq!(string_of(&directive, "message"), expected);
        assert!(!record.join("aidlc-state.md").exists());
        assert_eq!(workspace.journal_rows(), before_journal);
        assert_eq!(workspace.audit_shard().as_deref(), Some(audit.as_str()));
        assert_eq!(position(), before);
    }
    for args in [
        vec!["report", "--result", "awaiting-approval"],
        vec!["report", "--result", "resumed", "--user-input", "1"],
        vec![
            "report",
            "--single",
            "--result",
            "approved",
            "--stage",
            "contract-design",
        ],
        vec!["report", "--skeleton-stance", "on"],
    ] {
        let completion = invoke(&workspace, "aidlc-orchestrate", &args).await;
        assert_refused_at(
            &completion,
            "aidlc-orchestrate: projection restoration: read: io: InvalidData at ",
            path.as_path(),
        );
        assert!(!record.join("aidlc-state.md").exists());
        assert_eq!(workspace.journal_rows(), before_journal);
        assert_eq!(workspace.audit_shard().as_deref(), Some(audit.as_str()));
        assert_eq!(position(), before);
    }
    for completion in [
        promote(&workspace).await,
        set_autonomy(&workspace, "gated").await,
    ] {
        assert_refused_at(
            &completion,
            "aidlc-orchestrate: projection restoration: read: io: InvalidData at ",
            path.as_path(),
        );
        assert!(!record.join("aidlc-state.md").exists());
        assert_eq!(workspace.journal_rows(), before_journal);
        assert_eq!(workspace.audit_shard().as_deref(), Some(audit.as_str()));
        assert_eq!(position(), before);
    }
}

/// イベント保存後だけpublicationを失敗させ、次の読み取りで一度だけ追いつく。
#[tokio::test]
async fn a_committed_report_survives_publication_failure_and_is_recovered_once() {
    let workspace = minted().await;
    let store = workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let database = rusqlite::Connection::open(&store).expect("実SQLiteストア");
    let checkpoint = || {
        database.query_row("SELECT last_global_seq FROM amadeus_projection_checkpoint WHERE projection='orchestration'", [], |row| row.get::<_, i64>(0)).expect("公開位置")
    };
    let before_position = checkpoint();
    let before_journal = workspace.journal_rows();
    let before_state = workspace.state_file().expect("公開済み状態");
    let before_audit = workspace.audit_shard().expect("公開済み監査");
    database.execute_batch(&format!("CREATE TRIGGER fail_new_publication BEFORE INSERT ON amadeus_publication WHEN NEW.target_position > {before_position} BEGIN SELECT RAISE(ABORT,'publication unavailable after event commit'); END;")).expect("新イベントに対応する公開だけを失敗させる");

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "awaiting-approval"],
    )
    .await;
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: projection: read: io: Other at ",
        &store,
    );
    let committed_journal = workspace.journal_rows();
    assert_eq!(
        committed_journal,
        (before_journal.0, before_journal.1 + 1),
        "イベント保存後の失敗である"
    );
    assert_eq!(checkpoint(), before_position, "公開完了位置を先行させない");
    assert_eq!(
        workspace.state_file().as_deref(),
        Some(before_state.as_str())
    );
    assert_eq!(
        workspace.audit_shard().as_deref(),
        Some(before_audit.as_str())
    );

    database
        .execute_batch("DROP TRIGGER fail_new_publication")
        .expect("公開障害を除去");
    let recovered = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;
    assert_eq!(recovered.code(), 0, "{recovered:?}");
    assert_eq!(recovered.diagnostic(), None);
    assert_ne!(string_of(&line_of(&recovered), "kind"), "error");
    assert_eq!(
        workspace.journal_rows(),
        committed_journal,
        "復旧は新イベントを作らない"
    );
    assert_eq!(checkpoint(), before_position + 1);
    let after_state = workspace.state_file().expect("復旧済み状態");
    assert!(after_state.contains("- [?] domain-design"), "{after_state}");
    let after_audit = workspace.audit_shard().expect("復旧済み監査");
    assert_eq!(
        after_audit.matches("STAGE_AWAITING_APPROVAL").count(),
        before_audit.matches("STAGE_AWAITING_APPROVAL").count() + 1
    );
    let repeated = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;
    assert_eq!(repeated.code(), 0, "{repeated:?}");
    assert_ne!(string_of(&line_of(&repeated), "kind"), "error");
    assert_eq!(workspace.journal_rows(), committed_journal);
    assert_eq!(checkpoint(), before_position + 1);
    assert_eq!(
        workspace.state_file().as_deref(),
        Some(after_state.as_str())
    );
    assert_eq!(
        workspace.audit_shard().as_deref(),
        Some(after_audit.as_str())
    );
}

#[tokio::test]
async fn next_restores_missing_projection_files_without_repeating_audit() {
    let workspace = Workspace::create();
    let created = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "recovery"],
    )
    .await;
    assert_eq!(created.code(), 0, "{created:?}");
    let state = workspace.state_file().expect("公開済み状態");
    let audit = workspace.audit_shard().expect("公開済み監査");
    let record = workspace.record_dir().expect("記録先");
    fs::remove_file(record.join("aidlc-state.md")).expect("状態を失わせる");
    fs::remove_dir_all(record.join("audit")).expect("監査出力を失わせる");
    for _ in 0..2 {
        let next = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;
        assert_eq!(next.code(), 0, "{next:?}");
        assert_eq!(workspace.state_file().as_deref(), Some(state.as_str()));
        assert_eq!(workspace.audit_shard().as_deref(), Some(audit.as_str()));
    }
}

/// `report --result` が遷移をコミットし、投影が読み面へ落ちる。
///
/// # issue #76 是正済み — 書き面と読み面は誕生時から一致する
///
/// かつては誕生の**投影**だけが初期化ステージを `[x]` にして最初のゲート付きステージへ
/// カーソルを進め、**集約はそれをしていなかった**（`IntentExecution::start` が出すのは
/// `Started` 1 本で、カーソルは 0 = `state-init` のまま）。読み面が書き面より先へ進んで
/// いたため、直後の `report --result completed` が `state-init` を**もう一度**完了させ、
/// 監査シャードに `STAGE_COMPLETED` が 2 度現れていた。
///
/// 誕生 = 初期化完了済み（オーナー裁定 2026-09-01）でこの乖離は閉じた。集約も誕生の
/// 時点で initialization を completed にし、カーソルは最初のゲート付きステージ
/// （`domain-design`）に立つ。したがってこの報告が完了させるのは `state-init` ではなく
/// `domain-design` であり、しかもそれは**ゲート付き**なので `Forward` は承認
/// （`GATE_APPROVED` + ゲート経由の `STAGE_COMPLETED`）になる。
///
/// 本テストはその両面一致を逐語で固定する — `state-init` の完了行が最後まで 1 本の
/// ままであることが、二重記録が戻っていないことの証拠である。
#[tokio::test]
async fn reporting_a_verdict_commits_and_projects() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    // 誕生の時点で initialization は完了済み、最初のゲート付きステージが in-flight。
    // これは読み面（投影）だけでなく書き面（集約のカーソル）でも同じである。
    let before = workspace.state_file().expect("投影済み");
    assert!(before.contains("- [x] state-init"), "{before}");
    assert!(before.contains("- [-] domain-design"), "{before}");
    let before_audit = workspace.audit_shard().expect("監査シャード");
    let before_completions = before_audit.matches("STAGE_COMPLETED").count();
    assert_eq!(
        before_audit.matches("GATE_APPROVED").count(),
        0,
        "誕生の投影はゲートを承認しない: {before_audit}"
    );

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--result",
            "completed",
            "--user-input",
            "A",
            "--stage",
            "domain-design",
        ],
    )
    .await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    assert_eq!(string_of(&line_of(&completion), "kind"), "done");
    // 遷移がコミットされ、投影が監査へ足した行の対象は `domain-design` である。
    let after_audit = workspace.audit_shard().expect("監査シャード");
    assert_eq!(
        after_audit.matches("STAGE_COMPLETED").count(),
        before_completions + 1,
        "報告が監査へ完了行を 1 本足す: {after_audit}"
    );
    assert_eq!(
        after_audit.matches("GATE_APPROVED").count(),
        1,
        "カーソルはゲート付きなので Forward は承認になる: {after_audit}"
    );
    assert!(
        after_audit.contains("Stage Domain Design approved by gate"),
        "完了したのは domain-design（ゲート経由の逐語）: {after_audit}"
    );
    // 二重記録が戻っていないことの証拠 — state-init の完了行は誕生の 1 本のままである。
    assert_eq!(
        after_audit.matches("State initialized:").count(),
        1,
        "state-init を 2 度完了させない（issue #76 の症状）: {after_audit}"
    );
    // 承認でカーソルが次のゲートへ進み、読み面もそこへ動く。
    let after_state = workspace.state_file().expect("投影済み");
    assert!(after_state.contains("- [x] domain-design"), "{after_state}");
    assert!(
        after_state.contains("- [-] contract-design"),
        "{after_state}"
    );
}

/// 受理されない `--result` は逐語で拒否される（ビジネス拒否 — stdout・exit 0）。
#[tokio::test]
async fn an_unknown_result_is_refused_verbatim() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "ok"],
    )
    .await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "error");
    assert_eq!(
        string_of(&directive, "message"),
        "Unknown --result \"ok\". accepted outcomes: approved, completed, complete, done, awaiting-approval, rejected, revised, resume, resumed, skipped."
    );
}

/// `park` はカーソル位置にマーカーを据え、投影が読み面 2 面へ落ちる（#74）。
///
/// 続けてもう一度 `park` を打つと**再スタンプ**になる — upstream の `handlePark` は park 済み
/// でも成功し、`Parked` 時刻を上書き・`WORKFLOW_PARKED` を再 emit する。状態ファイルの
/// `Parked` 行は置き直しなので 1 本のままで、監査ブロックだけが 2 つになる。
#[tokio::test]
async fn parking_stamps_the_marker_and_projects_both_faces() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;

    let completion = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "parked");
    // 誕生のカーソルは最初のゲート付きステージなので、park の位置はそこである。
    assert_eq!(string_of(&directive, "stage"), "domain-design");
    assert_eq!(
        string_of(&directive, "reason"),
        "Workflow parked at \"domain-design\". Resume with /aidlc --resume."
    );

    // 読み面 1: 状態ファイルの `## Runtime State` に 2 行が入る。
    let state = workspace.state_file().expect("投影済み");
    assert!(state.contains("- **Parked**: "), "{state}");
    assert!(
        state.contains("- **Parked At Stage**: domain-design"),
        "{state}"
    );
    // 読み面 2: 監査シャードに `WORKFLOW_PARKED` ブロックが 1 つ追記される。
    let audit = workspace.audit_shard().expect("監査シャード");
    assert_eq!(audit.matches("WORKFLOW_PARKED").count(), 1, "{audit}");
    assert!(audit.contains("**Stage**: domain-design"), "{audit}");

    // 再スタンプ — park 済みでも成功する。
    let again = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(again.code(), 0, "{again:?}");
    assert_eq!(string_of(&line_of(&again), "kind"), "parked");
    let audit = workspace.audit_shard().expect("監査シャード");
    assert_eq!(
        audit.matches("WORKFLOW_PARKED").count(),
        2,
        "再 emit されて 2 ブロックになる: {audit}"
    );
    let state = workspace.state_file().expect("投影済み");
    assert_eq!(
        state.matches("- **Parked**: ").count(),
        1,
        "マーカーは置き直しなので 1 行のまま: {state}"
    );
}

/// 完了済みのワークフローは park できない（upstream 逐語を中継形で包む）。
#[tokio::test]
async fn parking_a_completed_workflow_is_refused_verbatim() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    // ゲート付き 2 段（domain-design / contract-design）を畳んで Completed にする。
    // `[-]` のゲートは明示 `--stage` を要し（forward 表）、前進は人間の選択を要する（段 13）。
    for stage in ["domain-design", "contract-design"] {
        let completion = invoke(
            &workspace,
            "aidlc-orchestrate",
            &[
                "report",
                "--result",
                "completed",
                "--user-input",
                "A",
                "--stage",
                stage,
            ],
        )
        .await;
        assert_eq!(completion.code(), 0, "{completion:?}");
    }
    // 計画を使い切った証拠 — ゲート付き 2 段が両方 `[x]` になっている。
    // （読み面の `- **Status**:` はまだ `Running` のまま — ワークフロー完了の投影は
    //   未実装であり、b45 のスコープ外である。書き面が Completed であることは、下の
    //   park が upstream 逐語 2「already Completed」で拒まれることが示す。）
    let state = workspace.state_file().expect("投影済み");
    assert!(state.contains("- [x] domain-design"), "{state}");
    assert!(state.contains("- [x] contract-design"), "{state}");

    let completion = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "error");
    assert_eq!(
        string_of(&directive, "message"),
        "Cannot park the workflow: Workflow is already Completed - nothing to park."
    );
}

/// autonomous な構築ランは park を拒む（無人のランには再開する人間が居ない）。
///
/// `set-autonomy` はまだ配線されていない（#72）ので、モードの切替はリポジトリを直接開いて
/// `switch_autonomy` を store する — テストダブルではなく**実物のリポジトリ経由の実駆動**で
/// ある。
#[tokio::test]
async fn parking_an_autonomous_run_is_refused_verbatim() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    workspace.switch_to_autonomous().await;

    let completion = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "error");
    assert_eq!(
        string_of(&directive, "message"),
        "Cannot park the workflow: Refusing to park: Construction Autonomy Mode is autonomous. \
An unattended autonomous run has no human to resume it and must keep moving - do not park it."
    );
}

/// 実行がまだ鋳造されていなければ park する対象が無い（ビジネス拒否）。
#[tokio::test]
async fn parking_without_an_execution_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "error");
    assert_eq!(
        string_of(&directive, "message"),
        "Cannot park the workflow: No workflow execution to park. Run `next` first."
    );
}

/// ユーティリティ面の未知動詞は**自己防衛拒否**（stderr・exit 1・stdout は無音）。
#[tokio::test]
async fn an_unknown_utility_verb_is_refused_on_stderr() {
    let workspace = Workspace::create();

    let completion = invoke(&workspace, "aidlc-utility", &["teleport"]).await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert_eq!(
        completion.diagnostic(),
        Some("aidlc-orchestrate: Unknown subcommand: teleport")
    );
}

/// 段 5 — `report` は `--result` が要る（受理 10 語を提示順で並べる）。
#[tokio::test]
async fn reporting_without_a_result_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(&workspace, "aidlc-orchestrate", &["report"]).await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "report requires --result <outcome>. Accepted: approved, completed, complete, done, \
awaiting-approval, rejected, revised, resume, resumed, skipped (the verdict for the stage just \
acted on)."
    );
}

/// 段 5 — 受理 10 語の外は硬いエラーである。
#[tokio::test]
async fn reporting_an_unknown_result_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "ok"],
    )
    .await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Unknown --result \"ok\". accepted outcomes: approved, completed, complete, done, \
awaiting-approval, rejected, revised, resume, resumed, skipped."
    );
}

/// 段 4 — resume 系の結末は遷移ではない。`--user-input` が無ければそこで止まる。
#[tokio::test]
async fn a_resume_result_without_the_humans_choice_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "resume"],
    )
    .await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "report --result resumed requires --user-input with the human's resume choice."
    );
}

/// 段 4 — 再開の報告はステージ遷移ではないので `--stage` を受け付けない。
#[tokio::test]
async fn a_resume_result_with_a_stage_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--result",
            "resumed",
            "--user-input",
            "1",
            "--stage",
            "domain-design",
        ],
    )
    .await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "A resume-choice report is not a stage transition; omit --stage."
    );
}

/// 段 4 — 状態ファイルが無ければ再開する対象が無い（ダッシュは ASCII の `-`）。
#[tokio::test]
async fn a_resume_result_without_a_state_file_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "resumed", "--user-input", "1"],
    )
    .await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "No active intent workflow state found (aidlc-state.md is absent) - nothing to resume."
    );
}

/// 段 4 — 4 つの選択肢は数字でも語でも同じ経路に落ち、5 つ目は拒否になる。
#[tokio::test]
async fn the_four_resume_choices_route_and_anything_else_is_refused() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    let (kind, message) =
        report_directive(&workspace, &["--result", "resumed", "--user-input", "1"]).await;
    assert_eq!(kind, "print");
    assert_eq!(
        message,
        "Resume choice accepted at \"domain-design\". Re-run `next` to continue from the last checkpoint."
    );
    let (_, message) =
        report_directive(&workspace, &["--result", "resumed", "--user-input", "2"]).await;
    assert_eq!(
        message,
        "Redo accepted at \"domain-design\". Run `aidlc-jump execute --target domain-design \
--direction redo --scope classic` to reset the current stage, then re-run `next` to start it over."
    );
    let (_, message) =
        report_directive(&workspace, &["--result", "resumed", "--user-input", "3"]).await;
    assert_eq!(
        message,
        "Jump accepted. Ask the human which stage to jump to, then re-run `next --stage <slug>`; \
the direction and the target are worked out and checked for you."
    );
    let (_, message) =
        report_directive(&workspace, &["--result", "resumed", "--user-input", "4"]).await;
    assert_eq!(
        message,
        "Start-fresh accepted. Confirm the new work's scope and description with the human, then \
run `next --new-intent --scope <scope> \"<description>\"` — the existing workflow stays in place \
and the new intent starts alongside it."
    );
    // 語での応答も同じ経路に落ちる（数字は正規化してから意味で照合する）。
    let (_, message) = report_directive(
        &workspace,
        &[
            "--result",
            "resume",
            "--user-input",
            "REDO the current stage",
        ],
    )
    .await;
    assert!(message.starts_with("Redo accepted at"), "{message}");
    // 当たらない応答は**正規化前の生値**を埋めて拒む。
    let (kind, message) = report_directive(
        &workspace,
        &["--result", "resumed", "--user-input", "  Maybe Later  "],
    )
    .await;
    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "Unrecognized resume choice \"  Maybe Later  \". Accepted choices: 1/resume from last \
checkpoint, 2/redo the current stage, 3/jump to a stage, or 4/start fresh."
    );
}

/// 鋳造は record に**実行カーソル**を据える — `report` はこれで実行を解決する。
///
/// かつて `report` はジャーナル先頭の実行行を実行の識別子と決め打っていた（リードモデルが
/// 実行の識別子を記録していないため）。それは「実行はワークスペースにただ 1 つ」という
/// 仮定に乗っており、2 本目が生まれた瞬間に静かに別の実行へ報告する。record が指す実行を
/// record 自身に書くことでその仮定を外した。
#[tokio::test]
async fn minting_writes_the_execution_cursor_into_the_record() {
    let workspace = Workspace::create();

    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;

    let cursor = workspace
        .execution_cursor()
        .expect("record に実行カーソルが据わっている");
    let mut lines = cursor.lines();
    let execution_id = lines.next().expect("1 行目 = 実行の識別子");
    let intent_id = lines.next().expect("2 行目 = intent の識別子");
    assert_eq!(lines.next(), None, "2 行だけである: {cursor:?}");
    // どちらも UUIDv7 の正準表記で、**互いに違う**（実行は intent の識別子を借りない）。
    assert_eq!(execution_id.len(), 36, "{cursor:?}");
    assert_eq!(intent_id.len(), 36, "{cursor:?}");
    assert_ne!(execution_id, intent_id, "{cursor:?}");
    // 2 行目は record 名の id8 と一致する — record とカーソルが同じ intent を指す証拠。
    let record = workspace.record_dir().expect("record");
    let name = record
        .file_name()
        .and_then(|n| n.to_str())
        .expect("record 名")
        .to_string();
    let id8: String = intent_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect();
    assert!(
        name.ends_with(&id8),
        "record 名 {name} と intent {intent_id}"
    );
}

/// 実行カーソルが壊れていれば**不在と混ぜず**に拒む。
///
/// 「まだ鋳造していない」に畳むと、壊れた record の上で作業が続いてしまう。
#[tokio::test]
async fn reporting_against_a_broken_execution_cursor_is_refused() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    let record = workspace.record_dir().expect("record");
    fs::write(record.join(".aidlc-execution"), "not-an-id\nalso-not\n").expect("カーソルを壊す");

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "completed"],
    )
    .await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let message = string_of(&line_of(&completion), "message");
    assert!(
        message.starts_with("The execution cursor cannot be read"),
        "{message}"
    );
    assert!(
        message.contains("malformed execution cursor at"),
        "{message}"
    );
}

/// 進行中の実行が無ければ報告先が無い。
#[tokio::test]
async fn reporting_before_any_intent_exists_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "completed"],
    )
    .await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "No active intent workflow state found (aidlc-state.md is absent) — nothing to report a transition for."
    );
}

/// 記録ディレクトリを作れないときは**何も作らず**拒む（半端な record を残さない）。
#[tokio::test]
async fn a_record_directory_that_cannot_be_created_is_refused() {
    let workspace = Workspace::create();
    // `intents/` の場所をファイルが占めていれば、その下にディレクトリは作れない。
    let intents = workspace.path("aidlc/spaces/default/intents");
    fs::remove_dir_all(&intents).expect("既存の intents を退ける");
    fs::write(&intents, "not a directory\n").expect("同名のファイル");

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    assert_eq!(completion.code(), 1);
    assert!(
        completion
            .diagnostic()
            .unwrap_or_default()
            .starts_with("aidlc-orchestrate: cannot create the record directory:"),
        "{completion:?}"
    );
}

/// ストアを開けないときも拒む（イベントを書けないのに record だけ増やさない）。
#[tokio::test]
async fn an_unopenable_store_is_refused() {
    let workspace = Workspace::create();
    // ストアファイルの場所をディレクトリが占めていれば SQLite は開けない。
    fs::create_dir_all(workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite"))
        .expect("同名のディレクトリ");

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some("aidlc-orchestrate: cannot open the event store")
    );
}

/// `--depth` / `--test-strategy` も `--review` と同じ形で状態ファイルまで届く。
#[tokio::test]
async fn the_scope_configuration_flags_reach_the_projected_state_file() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &[
            "intent-create",
            "--scope",
            "classic",
            "--label",
            "demo run",
            "--depth",
            "standard",
            "--test-strategy",
            "minimal",
        ],
    )
    .await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    let state = workspace.state_file().expect("状態ファイルが投影された");
    assert!(state.contains("- **Depth**: standard"), "{state}");
    assert!(state.contains("- **Test Strategy**: minimal"), "{state}");
}

/// 鍵が読めなければ `next` は**ビジネス拒否**として逐語で止まる（fail-closed）。
#[tokio::test]
async fn an_unreadable_steering_key_stops_next_with_its_wording() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    // 鍵の場所をディレクトリが塞いでいれば「在るのに読めない」になる。
    let key = workspace
        .record_dir()
        .expect("カーソル")
        .join(".aidlc-steering-token-key");
    fs::create_dir_all(&key).expect("同名のディレクトリ");

    let completion = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let message = string_of(&line_of(&completion), "message");
    assert!(message.contains("local key file at"), "{message}");
}

/// `continue` 側も同じ鍵の失敗で止まる（こちらは鋳造しないので読取の失敗がそのまま出る）。
#[tokio::test]
async fn an_unreadable_steering_key_stops_continue_too() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    let key = workspace
        .record_dir()
        .expect("カーソル")
        .join(".aidlc-steering-token-key");
    fs::create_dir_all(&key).expect("同名のディレクトリ");

    let completion = invoke(&workspace, "aidlc-orchestrate", &["continue", "token"]).await;

    assert_eq!(completion.code(), 0);
    let message = string_of(&line_of(&completion), "message");
    assert!(message.contains("local key file at"), "{message}");
}

/// 上限を超える directive は**出さずに拒む**（1 行 JSON の transport 契約）。
#[tokio::test]
async fn an_oversize_directive_is_refused_instead_of_emitted() {
    let workspace = Workspace::create();
    // 拒否文言は与えられた `--result` を逐語で引用するので、巨大な値は巨大な directive になる。
    let huge = "x".repeat(30 * 1024);

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", &huge],
    )
    .await;

    assert_eq!(completion.code(), 1, "自己防衛拒否");
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert_eq!(
        completion.diagnostic(),
        Some("aidlc-orchestrate: refusing to emit a directive larger than 28672 bytes")
    );
}

/// 配布物が読めなければ鋳造は止まる（壊れたインストールで空の intent を作らない）。
///
/// 止まる位置は 2026-08-31 の ES 転換で**手前へ動いた**。以前は鋳造の途中で Repository が
/// ファイルを読んで失敗していたが、いまは鋳造の前段の ensure-defined（取込 → 定義の確立）が
/// 失敗する。観測は同じ「stdout には何も出さず、カーソルも据わらない」である。
#[tokio::test]
async fn a_definition_that_cannot_be_read_stops_the_mint() {
    let workspace = Workspace::create();
    fs::remove_file(workspace.path(".claude/tools/data/stage-graph.json")).expect("定義を欠く");

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert!(
        completion.diagnostic().unwrap_or_default().starts_with(
            "aidlc-orchestrate: cannot read the compiled definition: \
                 compiled definition repository: io: NotFound at"
        ),
        "{completion:?}"
    );
    assert!(workspace.record_dir().is_none(), "カーソルは据わらない");
}

/// 壊れた定義の診断は**原因の末端まで**届く（PR #78 レビュー指摘）。
///
/// `RepositoryError::Corrupt` は「壊れていた」としか `Display` に書かない（裁定 6 —
/// 分類を契約に載せない）。どのファイルがどう壊れていたかという実材料は `Error::source` の
/// 連鎖に載るので、診断はそれを末端まで辿らないと利用者に届かない。
#[tokio::test]
async fn a_corrupt_definition_names_the_file_and_the_reason_in_the_diagnostic() {
    let workspace = Workspace::create();
    let graph = workspace.path(".claude/tools/data/stage-graph.json");
    fs::write(&graph, "{ not json").expect("壊れた定義を置く");

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    let diagnostic = completion.diagnostic().unwrap_or_default();
    assert!(
        diagnostic.contains("caused by"),
        "原因連鎖を辿っていない: {diagnostic}"
    );
    assert!(
        diagnostic.contains(&format!(
            "stage graph at {} is not valid JSON",
            graph.display()
        )),
        "壊れたファイルと理由が届いていない: {diagnostic}"
    );
}

/// 定義の取込は**冪等**である — 2 度目の鋳造は定義のストリームに 1 行も足さない。
///
/// ensure-defined は毎回の鋳造の前段で走る（配布物を読んで定義をストアへ合わせる）。
/// 配布物が 1 バイトも変わっていなければ、集約が `Unchanged` ガードで改訂を拒否し、
/// ユースケースがそれを `Ok(())` へ畳むので**店に行かない**（`store` を呼ばない）。
/// ジャーナルを直接数えて、2 度目に行が増えていないことを固定する
/// （オーナー裁定 2026-08-31 の受入条件）。
#[tokio::test]
async fn a_second_mint_does_not_write_the_definition_again() {
    let workspace = Workspace::create();

    let first = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "one"],
    )
    .await;
    assert_eq!(first.code(), 0, "{first:?}");
    let (definitions, intents) = workspace.journal_rows();
    assert_eq!(definitions, 1, "定義は誕生の 1 行だけ");

    // 2 つ目の鋳造。**この invoke 自体は投影で倒れる**（1 ワークスペースに 2 intent が
    // 同居する形は RMU がまだ受け付けない）が、それは ensure-defined と鋳造の**後段**で
    // ある。intent 側の行が増えたことが「そこまで到達した」証拠になるので、それを併せて
    // 確かめる — 手前で倒れていたら定義の行数が変わらないのは当たり前になってしまう。
    let second = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "two"],
    )
    .await;

    let (definitions_after, intents_after) = workspace.journal_rows();
    assert!(
        intents_after > intents,
        "2 度目の鋳造は ensure-defined を抜けて集約を書いている: {second:?}"
    );
    assert_eq!(
        definitions_after, definitions,
        "内容版が同じなら改訂イベントは書かれない（取込は冪等）"
    );
}

/// 書いた後の投影に失敗したら**それも拒否する**（書けたのに読み面へ落ちない状態を
/// 「成功」と言わない）。クローン ID の場所をディレクトリが塞いでいると投影は始まれない。
#[tokio::test]
async fn a_projection_that_cannot_run_after_the_write_is_refused() {
    let workspace = Workspace::create();
    fs::create_dir_all(workspace.path("aidlc/.aidlc-clone-id")).expect("同名のディレクトリ");

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    assert_eq!(completion.code(), 1);
    assert!(
        completion
            .diagnostic()
            .unwrap_or_default()
            .starts_with("aidlc-orchestrate: clone id:"),
        "{completion:?}"
    );
}

/// `--stage` は slug でなければ受け付けない（遷移先を取り違えないための門）。
#[tokio::test]
async fn a_stage_value_that_is_not_a_slug_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "completed", "--stage", "NOT A SLUG"],
    )
    .await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Internal: reported stage \"NOT A SLUG\" is not in the compiled graph — cannot commit its transition."
    );
}

/// 段 8 — slug の文法内でも計画に無ければ同じ逐語で断る。
#[tokio::test]
async fn a_stage_outside_the_plan_is_refused_with_the_same_wording() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--result",
            "completed",
            "--user-input",
            "A",
            "--stage",
            "not-in-the-plan",
        ],
    )
    .await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Internal: reported stage \"not-in-the-plan\" is not in the compiled graph — cannot commit its transition."
    );
}

/// カーソル以外のステージを報告すると集約が拒み、その理由が中継される。
#[tokio::test]
async fn reporting_a_stage_that_is_not_the_cursor_is_rejected() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--result",
            "completed",
            "--user-input",
            "A",
            "--stage",
            "contract-design",
        ],
    )
    .await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Stage \"contract-design\" is still pending. Run the stage before reporting it complete."
    );
}

/// 空間名として成立しないカーソルは**既定へ落とさず**拒む（record とイベントが散るため）。
#[tokio::test]
async fn an_invalid_active_space_is_refused_rather_than_defaulted() {
    let workspace = Workspace::create();
    fs::write(workspace.path("aidlc/active-space"), "../escape\n").expect("カーソル");

    let created = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    assert_eq!(created.code(), 1);
    assert_eq!(
        created.diagnostic(),
        Some(
            "The active space \"../escape\" is not a valid space name. Fix aidlc/active-space (or remove it to use the default space), then run the command again."
        )
    );

    // 読み側（report）も同じ判断で止まる。こちらはビジネス拒否層。
    let reported = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "completed"],
    )
    .await;
    assert_eq!(reported.code(), 0);
    assert!(
        string_of(&line_of(&reported), "message").starts_with("The active space \"../escape\""),
        "{reported:?}"
    );

    // `park` も同じ判断で止まる（書込動詞なので `report` と同じ形である）。
    let parked = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;
    assert_eq!(parked.code(), 0);
    assert!(
        string_of(&line_of(&parked), "message").starts_with("The active space \"../escape\""),
        "{parked:?}"
    );
}

/// `--review` は鋳造から状態ファイルまで貫通する。
#[tokio::test]
async fn a_review_override_reaches_the_projected_state_file() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &[
            "intent-create",
            "--scope",
            "classic",
            "--label",
            "demo run",
            "--review",
            "advisory",
        ],
    )
    .await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    let state = workspace.state_file().expect("状態ファイルが投影された");
    assert!(state.contains("- **Review Override**: advisory"), "{state}");
}

/// 閉集合外の `--review` は upstream 逐語で拒み、**何も作らない**。
#[tokio::test]
async fn an_unknown_review_class_is_refused_verbatim() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--review", "strict"],
    )
    .await;

    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some("Unknown review class: \"strict\". Valid: adversarial, advisory, none.")
    );
    assert!(workspace.record_dir().is_none(), "何も作らない");
}

/// print が名指しした命令行を、シェルと同じ規則で argv へ割る（テスト用の最小トークナイザ）。
///
/// 単一引用符（`shellArg` が出す形。内側は `'"'"'` で綴られる）と二重引用符（`--label`
/// のプレースホルダ）の両方を剥がす。エスケープ記号は upstream の綴りに現れないので扱わない。
fn shell_split(command: &str) -> Vec<String> {
    #[derive(PartialEq, Eq)]
    enum Quote {
        None,
        Single,
        Double,
    }
    let mut argv = Vec::new();
    let mut token = String::new();
    let mut open = false;
    let mut quote = Quote::None;
    for character in command.chars() {
        match (&quote, character) {
            (Quote::None, ' ') => {
                if open {
                    argv.push(std::mem::take(&mut token));
                    open = false;
                }
            }
            (Quote::None, '\'') => {
                quote = Quote::Single;
                open = true;
            }
            (Quote::None, '"') => {
                quote = Quote::Double;
                open = true;
            }
            (Quote::Single, '\'') | (Quote::Double, '"') => quote = Quote::None,
            _ => {
                token.push(character);
                open = true;
            }
        }
    }
    if open {
        argv.push(token);
    }
    argv
}

/// 名指し側（クエリ側 `next` の誕生 print）と受け口（`intent-create`）が噛み合う。
///
/// 命令行はテストが組み立てず、**print が出した文字列から切り出して**そのまま走らせる。
/// 置き換えるのは `--label` のプレースホルダだけで、これは upstream が明示的に
/// 「conductor が 2〜3 語のケバブに畳め」と指示している継ぎ目である（`:889-890`）。
/// 逸脱3（MintIntent の引数面が upstream 完全形か）が壊れたら、受け口側が値を取り違えて
/// ここが落ちる。
#[tokio::test]
async fn the_birth_print_names_a_command_the_receiving_surface_accepts() {
    let workspace = Workspace::create();

    let named = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["next", "--scope", "classic", "build the auth service"],
    )
    .await;
    assert_eq!(named.code(), 0, "{named:?}");
    let directive = line_of(&named);
    assert_eq!(string_of(&directive, "kind"), "print");
    let message = string_of(&directive, "message");
    let command = message
        .split('`')
        .nth(1)
        .expect("print はコマンドをバッククォートで括る");
    assert_eq!(
        command,
        "aidlc-utility intent-create --scope classic --arguments='build the auth service' --label \"<2-3 word kebab essence>\""
    );

    // conductor がラベルを畳む（唯一の置換）。
    let argv = shell_split(&command.replace("<2-3 word kebab essence>", "auth service"));
    let (face, rest) = argv.split_first().expect("argv0 がある");
    assert_eq!(face, "aidlc-utility");
    let borrowed: Vec<&str> = rest.iter().map(String::as_str).collect();

    let created = invoke(&workspace, face, &borrowed).await;

    assert_eq!(created.code(), 0, "{created:?}");
    let record = string_of(&line_of(&created), "record");
    assert!(record.contains("-auth-service-"), "{record}");
    // 自由記述は intent の Project 欄へ通っている（`--arguments` が届いた証拠）。
    let state = workspace.state_file().expect("状態ファイルが投影された");
    assert!(state.contains("build the auth service"), "{state}");

    // 名指し → 実行 → 再び `next`、で最初のステージへ着地する。
    let resumed = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;
    assert_eq!(string_of(&line_of(&resumed), "stage"), "domain-design");
}

/// `--scope` の無い鋳造は拒否される（bare invocation で既定の intent を作らない）。
#[tokio::test]
async fn creating_an_intent_without_a_scope_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(&workspace, "aidlc-utility", &["intent-create"]).await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None);
    assert!(workspace.record_dir().is_none(), "何も作らない");
}

/// 定義が名乗らない scope では鋳造できない — 自己防衛拒否（stderr・exit 1）で止まる。
#[tokio::test]
async fn minting_with_a_scope_the_definition_does_not_declare_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "nope", "--label", "demo"],
    )
    .await;

    assert_eq!(completion.code(), 1, "自己防衛拒否: {completion:?}");
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert!(
        completion
            .diagnostic()
            .unwrap_or_default()
            .starts_with("aidlc-orchestrate: "),
        "{completion:?}"
    );
    assert_eq!(workspace.record_dir(), None, "記録は残らない");
}

/// 定義の下準備でストアを開けなければ、最初の `next` はその失敗を逐語で運ぶ。
#[tokio::test]
async fn a_first_next_that_cannot_open_the_store_reports_the_failure() {
    let workspace = Workspace::create();
    fs::create_dir_all(workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite"))
        .expect("塞ぐ");

    let completion = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;

    assert_eq!(
        completion.code(),
        0,
        "ビジネス経路は exit 0: {completion:?}"
    );
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "error");
    assert!(
        string_of(&directive, "message").contains("cannot open the workflow definition repository"),
        "{directive:?}"
    );
}

/// `--project-dir` が無ければ、渡された作業ディレクトリがワークスペース根になる。
#[tokio::test]
async fn the_working_directory_is_the_workspace_root_when_no_project_dir_is_given() {
    let workspace = Workspace::create();

    let completion = aidlc::runtime::run(
        "aidlc-utility",
        &[
            "intent-create".to_string(),
            "--scope".to_string(),
            "classic".to_string(),
            "--label".to_string(),
            "demo".to_string(),
        ],
        workspace.project_dir(),
    )
    .await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    assert!(
        workspace.record_dir().is_some(),
        "cwd の下に record が生まれる"
    );
}

/// ゲート付きステージの往復 — 承認待ち → 差戻し → 改訂 → 承認を実 CLI で歩き、集約の
/// 遷移ガードが実際に噛み合うことを見る（`report` は結末ごとに別の遷移へ写る）。
#[tokio::test]
async fn a_gated_stage_walks_through_rejection_and_revision_before_it_is_approved() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    // gate 系 3 語は `print` で `Recorded <result> for "<slug>".` を返す（コミットは
    // するが、ワークフローは前へ進まない）。
    for outcome in ["awaiting-approval", "rejected", "revised"] {
        let next = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;
        assert_eq!(next.code(), 0, "{outcome} の直前の next: {next:?}");

        let completion = invoke(
            &workspace,
            "aidlc-orchestrate",
            &[
                "report",
                "--result",
                outcome,
                "--user-input",
                "Sharpen the testing posture.",
            ],
        )
        .await;

        assert_eq!(
            completion.code(),
            0,
            "{outcome} は受理される: {completion:?}"
        );
        let directive = line_of(&completion);
        assert_eq!(string_of(&directive, "kind"), "print", "{directive:?}");
        assert_eq!(
            string_of(&directive, "message"),
            format!("Recorded {outcome} for \"domain-design\".")
        );
    }

    // 承認だけが `done` で、コミットした段と scope を名乗る。
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "approved", "--user-input", "A"],
    )
    .await;
    assert_eq!(completion.code(), 0, "{completion:?}");
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "done");
    assert_eq!(
        string_of(&directive, "reason"),
        "Committed approve for \"domain-design\" (scope: classic). State advanced; run next to continue."
    );

    let state = workspace.state_file().expect("骨格");
    assert!(
        state.contains("contract-design"),
        "承認でカーソルが次のステージへ進む: {state}"
    );
}

/// 段 10 — 既に開いているゲートへの `awaiting-approval` は**何もコミットしない**成功である。
#[tokio::test]
async fn a_repeated_awaiting_approval_report_is_an_idempotent_print() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "awaiting-approval"],
    )
    .await;
    let before = workspace.audit_shard().expect("監査シャード");

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "awaiting-approval"],
    )
    .await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "print");
    assert_eq!(
        string_of(&directive, "message"),
        "Stage \"domain-design\" is already awaiting approval."
    );
    assert_eq!(
        workspace.audit_shard().expect("監査シャード"),
        before,
        "監査行も状態差分も空である（upstream `awaiting-approval-repeat`）"
    );
}

/// 段 10 — gate 系 3 語の前提違反は語ごとに別の逐語で断る。
#[tokio::test]
async fn each_gate_verdict_refuses_with_its_own_precondition_wording() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    // `[-]` から `revised` は通らない（再入できるのは revising だけ）。
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "revised"],
    )
    .await;
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Stage \"domain-design\" is in-progress; only a revising stage can re-enter its gate."
    );

    // 差し戻して `[R]` にすると、今度は `awaiting-approval` が通らない。
    invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "rejected", "--reason", "直して"],
    )
    .await;
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "awaiting-approval"],
    )
    .await;
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Stage \"domain-design\" is revising; only an in-progress stage can open a gate."
    );
    // `[R]` は差し戻しの前提集合にも入らない。
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "rejected", "--reason", "もう一度"],
    )
    .await;
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Stage \"domain-design\" is revising; only an active or awaiting-approval stage can be rejected."
    );
}

/// 段 10 — `rejected` は非空のフィードバックを要する。
#[tokio::test]
async fn a_rejection_without_feedback_is_refused() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "rejected", "--reason", "   "],
    )
    .await;

    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "report --result rejected for \"domain-design\" requires nonblank --user-input or --reason feedback."
    );
}

/// 段 13 — ゲート付き未完了の前進は人間の選択を要する。
///
/// 抜け道 2 つ（autonomous な実行・env `AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1`）は集約の
/// ユニットテストが固定する — env を差し替えるには `unsafe` が要り、workspace lint が
/// `unsafe_code` を forbid しているのでプロセス内では踏めない。
#[tokio::test]
async fn the_human_presence_guard_refuses_a_blank_approval() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "awaiting-approval"],
    )
    .await;

    let (kind, message) = report_directive(&workspace, &["--result", "approved"]).await;

    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "report --result approved for \"domain-design\" requires --user-input with the human's exact approval choice."
    );
}

/// forward 表 — `[-]` のゲートは明示 `--stage` を要し、名乗れば 2 段でコミットする。
#[tokio::test]
async fn approving_an_unopened_gate_needs_the_explicit_stage_and_then_recovers_it() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "approved", "--user-input", "A"],
    )
    .await;
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Stage \"domain-design\" is still in-progress. To approve a gated stage that has not \
entered awaiting-approval, report the acted directive explicitly with --stage \"domain-design\" \
so the engine cannot mistake a freshly advanced Current Stage for the completed one."
    );

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--result",
            "approved",
            "--user-input",
            "A",
            "--stage",
            "domain-design",
        ],
    )
    .await;
    assert_eq!(
        string_of(&line_of(&completion), "reason"),
        "Committed gate-start + approve for \"domain-design\" (scope: classic). State advanced; run next to continue."
    );
}

/// forward 表 — 未着手の `[ ]` と、通過済み `[x]` の再報告。
#[tokio::test]
async fn the_forward_table_refuses_a_pending_stage_and_folds_a_stale_re_report() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--result",
            "approved",
            "--user-input",
            "A",
            "--stage",
            "contract-design",
        ],
    )
    .await;
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "Stage \"contract-design\" is still pending. Run the stage before reporting it complete."
    );

    // 最初のゲートを承認してカーソルを進めてから、通過済みステージを再報告する。
    invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--result",
            "approved",
            "--user-input",
            "A",
            "--stage",
            "domain-design",
        ],
    )
    .await;
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--result",
            "approved",
            "--user-input",
            "A",
            "--stage",
            "domain-design",
        ],
    )
    .await;
    assert_eq!(
        string_of(&line_of(&completion), "reason"),
        "Stage \"domain-design\" is already completed and the workflow has moved on to \
\"contract-design\" (scope: classic); idempotent re-report, no transition needed."
    );
}

/// 段 9 — `skipped` の受理条件 4 形（明示 stage / CONDITIONAL / reason / カーソル一致）。
#[tokio::test]
async fn the_skipped_arm_refuses_each_missing_condition_verbatim() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    invoke(&workspace, "aidlc-orchestrate", &["next"]).await;

    // (a) 明示・非空の `--stage` が要る（集約より前の構文的な段）。
    let (kind, message) = report_directive(
        &workspace,
        &["--result", "skipped", "--reason", "out of scope"],
    )
    .await;
    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "report --result skipped requires an explicit nonblank --stage <slug>."
    );
    let (_, message) = report_directive(
        &workspace,
        &[
            "--result",
            "skipped",
            "--reason",
            "out of scope",
            "--stage",
            "   ",
        ],
    )
    .await;
    assert_eq!(
        message,
        "report --result skipped requires an explicit nonblank --stage <slug>."
    );
    // (b) 計画が EXECUTE と言っている ALWAYS のステージは飛ばせない。
    let (_, message) = report_directive(
        &workspace,
        &[
            "--result",
            "skipped",
            "--reason",
            "out of scope",
            "--stage",
            "domain-design",
        ],
    )
    .await;
    assert_eq!(
        message,
        "Stage \"domain-design\" is execution: ALWAYS; only a CONDITIONAL stage can report skipped."
    );
    // (c) カーソル以外を名指しした場合も (b) が先に立つ — 名指し先の宣言を先に見るからで
    //     ある（ピンの判定順 `:5613-5633`）。
    let (_, message) = report_directive(
        &workspace,
        &[
            "--result",
            "skipped",
            "--reason",
            "out of scope",
            "--stage",
            "contract-design",
        ],
    )
    .await;
    assert_eq!(
        message,
        "Stage \"contract-design\" is execution: ALWAYS; only a CONDITIONAL stage can report skipped."
    );
}

/// 実行カーソルが在るのに読めなければ、park も**不在と混ぜず**に読取失敗を答える。
#[tokio::test]
async fn parking_against_a_broken_execution_cursor_is_refused() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    let record = workspace.record_dir().expect("record");
    fs::write(record.join(".aidlc-execution"), "not-an-id\nalso-not\n").expect("カーソルを壊す");

    let completion = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let message = string_of(&line_of(&completion), "message");
    assert!(
        message.starts_with("Cannot park the workflow: The execution cursor cannot be read"),
        "{message}"
    );
    assert!(
        message.contains("malformed execution cursor at"),
        "{message}"
    );
}

/// イベントストアが開けなければ park は**自己防衛拒否**で止まる（媒体の失敗）。
#[tokio::test]
async fn parking_against_an_unopenable_event_store_is_refused() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    // カーソルは据わったまま、ストアの置き場だけをディレクトリで塞ぐ。
    let store = workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    fs::remove_file(&store).expect("ストア");
    fs::create_dir(&store).expect("塞ぐ");

    let completion = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert_eq!(
        completion.diagnostic(),
        Some("aidlc-orchestrate: cannot open the event store")
    );
}

/// 書いたあとに投影が回らなければ、park は**自己防衛拒否**で止まる（握り潰さない）。
///
/// 描けなければ利用者には park した位置が何も見えないままになるので、`report` と同じ規律で
/// 失敗を surface する。
#[tokio::test]
async fn a_projection_that_cannot_run_after_the_park_is_refused() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    // clone id の置き場をディレクトリで塞ぐと、投影のシャード名が解決できなくなる。
    let clone_id = workspace.path("aidlc/.aidlc-clone-id");
    fs::remove_file(&clone_id).expect("clone id");
    fs::create_dir(&clone_id).expect("塞ぐ");

    let completion = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert!(
        completion
            .diagnostic()
            .unwrap_or_default()
            .starts_with("aidlc-orchestrate: clone id:"),
        "{completion:?}"
    );
}

// ---------------------------------------------------------------------------
// `report` の 13 段ガード — 段 1 / 2 / 3 と、コミットの残り 2 形
// ---------------------------------------------------------------------------

/// 段 1 — `State Version` が現行でない状態ファイルは**どの report 経路でも**先に断る。
#[tokio::test]
async fn the_state_version_guard_refuses_every_report_path() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let state_file = workspace
        .record_dir()
        .expect("record")
        .join("aidlc-state.md");
    let original = fs::read_to_string(&state_file).expect("状態ファイル");

    // 3 形 (unparseable / past / future) を書き換えて 1 つずつ踏む。
    let cases: [(&str, &str); 3] = [
        (
            "8 garbage",
            "Incompatible workflow state: the State Version field is missing, empty, or \
unparseable in aidlc-state.md, so this state cannot be matched to the current v8 stage graph and \
cannot be advanced safely. Archive your workspace ('mv aidlc aidlc.archive') and start a fresh \
workflow (describe what to build), or finish this workflow on the prior shell. Run `/aidlc \
--doctor` for the full diagnosis.",
        ),
        (
            "7",
            "Incompatible workflow state: State Version 7 predates the current v8 stage graph. \
v8 renamed the Inception `application-design` stage to `domain-design` and inserted \
`contract-design`, so this state's stage rows no longer match the graph and cannot be advanced \
safely. Archive your workspace ('mv aidlc aidlc.v7-archive') and start a fresh workflow (describe \
what to build), or finish this workflow on the prior shell. Run `/aidlc --doctor` for the full \
diagnosis.",
        ),
        (
            "9",
            "Incompatible workflow state: State Version 9 is newer than the current v8 stage \
graph this build understands, so it cannot be advanced safely. Upgrade the framework to a build \
that ships state schema v9 (or newer), or finish this workflow on the shell that produced it. Run \
`/aidlc --doctor` for the full diagnosis.",
        ),
    ];
    for (version, expected) in cases {
        fs::write(
            &state_file,
            original.replace(
                "- **State Version**: 8",
                &format!("- **State Version**: {version}"),
            ),
        )
        .expect("版を書き換える");
        // 主経路も `--single` も `--skeleton-stance` も、段 1 で同じ文言に落ちる。
        for args in [
            vec!["--result", "approved"],
            vec![
                "--single",
                "--result",
                "approved",
                "--stage",
                "domain-design",
            ],
            vec!["--skeleton-stance", "on"],
        ] {
            let (kind, message) = report_directive(&workspace, &args).await;
            assert_eq!(kind, "error", "{version}: {args:?}");
            assert_eq!(message, expected, "{version}: {args:?}");
        }
    }
}

/// 段 1 — 0 バイトの状態ファイルは「不在」ではなく「版が読めない」である。
#[tokio::test]
async fn a_zero_byte_state_file_is_refused_as_an_unreadable_version() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let state_file = workspace
        .record_dir()
        .expect("record")
        .join("aidlc-state.md");
    fs::write(&state_file, "").expect("空にする");

    let (kind, message) = report_directive(&workspace, &["--result", "approved"]).await;

    assert_eq!(kind, "error");
    assert!(
        message.starts_with("Incompatible workflow state: the State Version field is missing"),
        "{message}"
    );
}

/// 段 2 — `--single` は構文を検証し、**本流を一歩も進めずに**対をコミットする（I10 / #73）。
#[tokio::test]
async fn the_single_report_commits_the_pair_without_advancing_the_main_workflow() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let before = workspace.state_file().expect("投影済み");

    let (_, message) = report_directive(&workspace, &["--single"]).await;
    assert_eq!(
        message,
        "report --single requires --result <outcome>. Accepted: approved, completed, complete, \
done (the verdict for the single stage just run)."
    );

    let (_, message) = report_directive(&workspace, &["--single", "--result", "rejected"]).await;
    assert_eq!(
        message,
        "Unknown --result \"rejected\". report commits forward outcomes only; accepted: approved, \
completed, complete, done."
    );

    let (_, message) = report_directive(&workspace, &["--single", "--result", "approved"]).await;
    assert_eq!(
        message,
        "report --single must not advance the main workflow. Pass --stage <slug> to commit the \
single stage's synthetic-id pair; --single never writes the main workflow's Current Stage."
    );

    // 計画に無い slug は未知として断る。
    let (kind, message) = report_directive(
        &workspace,
        &["--single", "--result", "approved", "--stage", "nowhere"],
    )
    .await;
    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "Unknown stage \"nowhere\". Run /aidlc --help for the full list."
    );

    // initialization は隔離実行の対象にならない。
    let (kind, message) = report_directive(
        &workspace,
        &["--single", "--result", "approved", "--stage", "state-init"],
    )
    .await;
    assert_eq!(kind, "error");
    assert!(
        message.starts_with("Cannot run an initialization stage with --single."),
        "{message}"
    );

    // 成功 — 監査 2 行だけが増え、状態ファイルは 1 バイトも動かない。
    let (kind, message) = report_directive(
        &workspace,
        &[
            "--single",
            "--result",
            "approved",
            "--stage",
            "contract-design",
        ],
    )
    .await;
    assert_eq!(kind, "done");
    assert_eq!(
        message,
        "Single-stage run of \"contract-design\" committed under synthetic workflow \
\"single-stage:contract-design\". The main workflow's Current Stage is untouched."
    );
    assert_eq!(
        workspace.state_file().expect("投影済み"),
        before,
        "`--single` は本流の状態を 1 バイトも動かさない"
    );
    let audit = workspace.audit_shard().expect("監査シャードは在る");
    assert!(
        audit.contains("**Workflow**: single-stage:contract-design"),
        "疑似ワークフロー ID で名乗る: {audit}"
    );
    assert!(
        audit.contains("**Details**: Single-stage run of contract-design completed"),
        "{audit}"
    );
}

/// 段 2 — 鋳造前のワークスペースには隔離実行の対を書けない。
#[tokio::test]
async fn a_single_report_without_an_intent_record_names_the_missing_cursor() {
    let workspace = Workspace::create();

    let (kind, message) = report_directive(
        &workspace,
        &[
            "--single",
            "--result",
            "approved",
            "--stage",
            "contract-design",
        ],
    )
    .await;

    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "Failed to record single-stage lifecycle pair for \"contract-design\": \
no active intent record"
    );
}

/// 段 3 — `--skeleton-stance` は値を検証し、state を要求し、現在地を照合してから記録する。
#[tokio::test]
async fn the_skeleton_stance_report_validates_its_value_and_the_current_stage() {
    let workspace = Workspace::create();

    // state がまだ無い段階でも、値の検証が先に立つ（順序は upstream と同じ）。
    let (_, message) = report_directive(&workspace, &["--skeleton-stance", "maybe"]).await;
    assert_eq!(
        message,
        "Unknown --skeleton-stance \"maybe\". Accepted: on, off, scope-dependent (the \
walking-skeleton stance classified from the team's ## Walking Skeleton prose)."
    );

    let (_, message) = report_directive(&workspace, &["--skeleton-stance", "on"]).await;
    assert_eq!(
        message,
        "No active intent workflow state found (aidlc-state.md is absent) — nothing to record a skeleton stance for."
    );

    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let before = workspace.state_file().expect("投影済み");
    // この合成グラフに Construction は無い — 現在地は skeleton-gate ではありえない。
    let (kind, message) =
        report_directive(&workspace, &["--skeleton-stance", "scope-dependent"]).await;
    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "Current stage \"domain-design\" is not the skeleton-gate stage for scope \"classic\" — \
a skeleton stance is only reported for the first Construction Bolt's gate."
    );
    assert_eq!(
        workspace.state_file().expect("投影済み"),
        before,
        "拒否された段は状態を 1 バイトも動かさない"
    );
}

/// walking-skeleton の分類往復 — `unresolved` → stance 報告 → 決まったゲート（#73）。
#[tokio::test]
async fn the_skeleton_gate_round_trip_turns_unresolved_into_a_determined_gate() {
    let workspace = Workspace::with_construction();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    // 誕生直後のカーソルは inception の `domain-design` — ゲートは決まっている。
    let directive = line_of(&invoke(&workspace, "aidlc-orchestrate", &["next"]).await);
    assert_eq!(string_of(&directive, "stage"), "domain-design");
    assert_eq!(gate_of(&directive), JsonValue::Bool(true));

    // 承認して Construction の最初の EXECUTE (= skeleton gate) へ進む。
    let (kind, message) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "approve",
        ],
    )
    .await;
    assert_eq!(kind, "done", "{message}");

    let directive = line_of(&invoke(&workspace, "aidlc-orchestrate", &["next"]).await);
    assert_eq!(string_of(&directive, "stage"), "functional-design");
    assert_eq!(
        gate_of(&directive),
        JsonValue::String("unresolved".to_string()),
        "分類の往復が済むまでゲートは決まらない"
    );

    // conductor が分類を返す。
    let (kind, message) = report_directive(&workspace, &["--skeleton-stance", "on"]).await;
    assert_eq!(kind, "print");
    assert_eq!(
        message,
        "Recorded walking-skeleton stance \"on\" for \"functional-design\". \
Re-run `next` to continue — the gate is now determined."
    );
    assert!(
        workspace
            .state_file()
            .expect("投影済み")
            .contains("- **Skeleton Stance**: on\n"),
        "状態ファイルの `## Runtime State` に載る"
    );

    // 記録後はゲートが決まる。
    let directive = line_of(&invoke(&workspace, "aidlc-orchestrate", &["next"]).await);
    assert_eq!(string_of(&directive, "stage"), "functional-design");
    assert_eq!(gate_of(&directive), JsonValue::Bool(true));
}

/// 分岐 4b — `next --single` の 5 つの拒否と、隔離 run-stage の形（#73）。
#[tokio::test]
async fn next_single_refuses_five_ways_and_emits_an_isolated_run_stage() {
    let workspace = Workspace::with_construction();
    // record が無いと run-stage の相対パスに基準を前置できないので、先に鋳造しておく。
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    // `--phase` との併用は排他である。
    let directive = line_of(
        &invoke(
            &workspace,
            "aidlc-orchestrate",
            &["next", "--single", "--phase", "construction"],
        )
        .await,
    );
    assert_eq!(
        string_of(&directive, "message"),
        "Cannot use --single with --phase. --single runs one stage; pass --stage <slug>."
    );

    let directive = line_of(&invoke(&workspace, "aidlc-orchestrate", &["next", "--single"]).await);
    assert_eq!(
        string_of(&directive, "message"),
        "--single requires --stage <slug>. A stage-runner runs exactly one named stage."
    );

    let directive = line_of(
        &invoke(
            &workspace,
            "aidlc-orchestrate",
            &["next", "--single", "--stage", "nowhere"],
        )
        .await,
    );
    assert_eq!(
        string_of(&directive, "message"),
        "Unknown stage \"nowhere\". Run /aidlc --help for the full list."
    );

    let directive = line_of(
        &invoke(
            &workspace,
            "aidlc-orchestrate",
            &["next", "--single", "--stage", "state-init"],
        )
        .await,
    );
    assert!(
        string_of(&directive, "message")
            .starts_with("Cannot run an initialization stage with --single."),
        "{directive:?}"
    );

    // scope グリッドで SKIP のステージは走らせない。
    let directive = line_of(
        &invoke(
            &workspace,
            "aidlc-orchestrate",
            &["next", "--single", "--stage", "code-generation"],
        )
        .await,
    );
    assert_eq!(
        string_of(&directive, "message"),
        "Stage \"code-generation\" is skipped for scope \"classic\". \
Choose a different stage or change scope."
    );

    // 成功 — `single: true` / `gate: false` / `next_stage` 不在。
    let directive = line_of(
        &invoke(
            &workspace,
            "aidlc-orchestrate",
            &["next", "--single", "--stage", "functional-design"],
        )
        .await,
    );
    assert_eq!(string_of(&directive, "kind"), "run-stage");
    assert_eq!(string_of(&directive, "stage"), "functional-design");
    assert_eq!(member_of(&directive, "single"), Some(JsonValue::Bool(true)));
    assert_eq!(gate_of(&directive), JsonValue::Bool(false));
    assert_eq!(
        member_of(&directive, "next_stage"),
        None,
        "隔離実行は次のステージを名乗らない"
    );
}

/// directive の `gate` フィールド (boolean か `"unresolved"` の 3 値)。
fn gate_of(directive: &JsonValue) -> JsonValue {
    member_of(directive, "gate").unwrap_or_else(|| panic!("gate が要る: {directive:?}"))
}

/// directive のメンバを 1 つ引く (不在は `None`)。
fn member_of(directive: &JsonValue, key: &str) -> Option<JsonValue> {
    match directive {
        JsonValue::Object(members) => members.get(key).cloned(),
        other => panic!("オブジェクトであるべき: {other:?}"),
    }
}

/// 記録そのものが失敗したら、中継形に材料を載せて断る。
///
/// カーソルは文法内だがストアに居ない実行を指す — 再構成が `NotFound` になる形である。
#[tokio::test]
async fn a_record_failure_is_relayed_as_material_on_the_single_face() {
    let absent = "018f3b2c-4d5e-7f60-8abc-def012345678";
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let record = workspace.record_dir().expect("record");
    let intent_id = workspace
        .execution_cursor()
        .expect("カーソルは在る")
        .lines()
        .nth(1)
        .expect("2 行目は intent id")
        .to_string();
    fs::write(
        record.join(".aidlc-execution"),
        format!("{absent}\n{intent_id}\n"),
    )
    .expect("居ない実行を指す");

    let (kind, message) = report_directive(
        &workspace,
        &[
            "--single",
            "--result",
            "approved",
            "--stage",
            "contract-design",
        ],
    )
    .await;

    assert_eq!(kind, "error");
    assert!(
        message.starts_with(
            "Failed to record single-stage lifecycle pair for \"contract-design\": repository:"
        ),
        "{message}"
    );
}

/// 投影された実行の行が引けなければ、stance は「記録する先が無い」として断る。
///
/// カーソルが歴史の無い実行を指す形（壊れた record）でだけ起きる — 状態ファイルが在っても
/// 記録できる実行が無いことに変わりはないので、不在と同じ逐語で断る。
#[tokio::test]
async fn a_skeleton_stance_report_without_a_projected_execution_is_refused() {
    let absent = "018f3b2c-4d5e-7f60-8abc-def012345678";
    let workspace = Workspace::with_construction();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let record = workspace.record_dir().expect("record");
    let intent_id = workspace
        .execution_cursor()
        .expect("カーソルは在る")
        .lines()
        .nth(1)
        .expect("2 行目は intent id")
        .to_string();
    fs::write(
        record.join(".aidlc-execution"),
        format!("{absent}\n{intent_id}\n"),
    )
    .expect("居ない実行を指す");

    let (kind, message) = report_directive(&workspace, &["--skeleton-stance", "on"]).await;

    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "No active intent workflow state found (aidlc-state.md is absent) — nothing to record a skeleton stance for."
    );
}

/// 文法外の `--stage` は、計画を引くまでもなく未知として断る。
#[tokio::test]
async fn a_single_report_with_a_malformed_stage_is_unknown() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    let (kind, message) = report_directive(
        &workspace,
        &["--single", "--result", "approved", "--stage", "Not A Slug"],
    )
    .await;

    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "Unknown stage \"Not A Slug\". Run /aidlc --help for the full list."
    );
}

/// state ファイルは在るのに実行カーソルが無い形も「記録する先が無い」で断る。
#[tokio::test]
async fn a_skeleton_stance_report_without_an_execution_cursor_is_refused() {
    let workspace = Workspace::with_construction();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let record = workspace.record_dir().expect("record");
    fs::remove_file(record.join(".aidlc-execution")).expect("カーソルを外す");

    let (kind, message) = report_directive(&workspace, &["--skeleton-stance", "on"]).await;

    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "No active intent workflow state found (aidlc-state.md is absent) — nothing to record a skeleton stance for."
    );
}

/// `--single` は媒体の失敗を握り潰さない（カーソル破損・ストア不通・投影不能）。
#[tokio::test]
async fn a_single_report_surfaces_every_medium_failure() {
    // (1) 壊れた実行カーソルは「不在」と混ぜない。
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let record = workspace.record_dir().expect("record");
    fs::write(record.join(".aidlc-execution"), "not-an-id\nalso-not\n").expect("カーソルを壊す");
    let (kind, message) = report_directive(
        &workspace,
        &[
            "--single",
            "--result",
            "approved",
            "--stage",
            "contract-design",
        ],
    )
    .await;
    assert_eq!(kind, "error");
    assert!(
        message.starts_with(
            "Failed to record single-stage lifecycle pair for \"contract-design\": \
The execution cursor cannot be read"
        ),
        "{message}"
    );

    // (2) ストアが開けなければ自己防衛拒否。
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let store = workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let before = workspace.journal_rows();
    let saved = store.with_extension("saved.sqlite");
    fs::rename(&store, &saved).expect("真実記録は保持して置き場を塞ぐ");
    fs::create_dir(&store).expect("塞ぐ");
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--single",
            "--result",
            "approved",
            "--stage",
            "contract-design",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: journal: io: NotFound at ",
        &store,
    );
    fs::remove_dir(&store).expect("障害を除去");
    fs::rename(&saved, &store).expect("真実記録を戻す");
    assert_eq!(workspace.journal_rows(), before);

    // (3) 事前復旧が回らなければ、隔離実行を記録せず自己防衛拒否。
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let before = workspace.journal_rows();
    let clone_id = workspace.path("aidlc/.aidlc-clone-id");
    fs::remove_file(&clone_id).expect("clone id");
    fs::create_dir(&clone_id).expect("塞ぐ");
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &[
            "report",
            "--single",
            "--result",
            "approved",
            "--stage",
            "contract-design",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert!(
        completion
            .diagnostic()
            .is_some_and(|message| message.starts_with("aidlc-orchestrate: clone id:")),
        "{completion:?}"
    );
    assert_eq!(
        workspace.journal_rows(),
        before,
        "事前復旧で止まり隔離実行を記録しない"
    );
}

/// `--skeleton-stance` も媒体の失敗を握り潰さない。
#[tokio::test]
async fn a_skeleton_stance_report_surfaces_every_medium_failure() {
    // (1) 壊れた実行カーソル — 状態ファイルは在るので値検証と state 検査は抜ける。
    let workspace = Workspace::with_construction();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let record = workspace.record_dir().expect("record");
    fs::write(record.join(".aidlc-execution"), "not-an-id\nalso-not\n").expect("カーソルを壊す");
    let (kind, message) = report_directive(&workspace, &["--skeleton-stance", "off"]).await;
    assert_eq!(kind, "error");
    assert!(
        message.starts_with("The execution cursor cannot be read"),
        "{message}"
    );

    // (2) ストアが開けなければ自己防衛拒否。
    let workspace = Workspace::with_construction();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let store = workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let before = workspace.journal_rows();
    let saved = store.with_extension("saved.sqlite");
    fs::rename(&store, &saved).expect("真実記録は保持して置き場を塞ぐ");
    fs::create_dir(&store).expect("塞ぐ");
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--skeleton-stance", "off"],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: journal: io: NotFound at ",
        &store,
    );
    fs::remove_dir(&store).expect("障害を除去");
    fs::rename(&saved, &store).expect("真実記録を戻す");
    assert_eq!(workspace.journal_rows(), before);
}

/// リードモデルの `read_execution` を**引けない形**に置き換える（イベントストアは無傷）。
///
/// 実 CLI からは作れない形なので、ここだけストアへ直接 SQL を打つ。表ごと落とすと
/// `catch_up` の `CREATE TABLE IF NOT EXISTS` が空の表を建て直してしまい「行が無い」に
/// なるので、**列が足りない表**を残して SELECT 自体を失敗させる。
fn break_read_execution(workspace: &Workspace) {
    let store = workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let connection = rusqlite::Connection::open(&store).expect("ストアは開ける");
    connection
        .execute_batch(
            "DROP TABLE read_execution; CREATE TABLE read_execution (id TEXT PRIMARY KEY);",
        )
        .expect("リードモデルの表は置き換えられる");
}

/// 引けないリードモデルは「記録する先が無い」と混ぜない（PR #103 レビュー指摘）。
///
/// 実行カーソルは読めてイベントストアも開けるのに、リードモデルの引当だけが落ちる形。
/// ここを `None` へ畳むと「まだ鋳造していない」と同じ答えになり、壊れた媒体の上で
/// 作業が続いてしまう。所在と分類を材料に「引けない」と言わせる。
#[tokio::test]
async fn a_skeleton_stance_report_names_the_unreadable_read_model() {
    let workspace = Workspace::with_construction();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    // 「開けるが引けない」形にする — イベントストアの表は無傷のまま、リードモデルの
    // `read_execution` だけを引けなくする。ファイル全体を非 SQLite バイト列にすると
    // イベントストアを開く段で止まってしまい、この経路には届かない。
    let before = workspace.journal_rows();
    break_read_execution(&workspace);
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--skeleton-stance", "off"],
    )
    .await;
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: journal: io: Other at ",
        &workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite"),
    );
    assert_eq!(
        workspace.journal_rows(),
        before,
        "復旧に失敗したらイベントは書かない"
    );
}

/// 事前復旧が回らなければ stance は記録する前に自己防衛拒否で止まる。
#[tokio::test]
async fn a_restoration_failure_prevents_the_stance_from_being_recorded() {
    let workspace = Workspace::with_construction();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    // skeleton-gate まで進めてから投影を塞ぐ（記録そのものは受理される位置にする）。
    report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "approve",
        ],
    )
    .await;
    let before = workspace.journal_rows();
    let clone_id = workspace.path("aidlc/.aidlc-clone-id");
    fs::remove_file(&clone_id).expect("clone id");
    fs::create_dir(&clone_id).expect("塞ぐ");

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--skeleton-stance", "on"],
    )
    .await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert_eq!(
        workspace.journal_rows(),
        before,
        "事前復旧の失敗ではコミットしない"
    );
}

/// `skipped` の受理 — CONDITIONAL なステージはルーティングされ、完了数には入らない。
#[tokio::test]
async fn a_conditional_stage_is_routed_forward_by_a_skipped_report() {
    let workspace = Workspace::conditional();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;

    let (kind, reason) = report_directive(
        &workspace,
        &[
            "--result",
            "skipped",
            "--reason",
            "  out of scope  ",
            "--stage",
            "domain-design",
        ],
    )
    .await;

    assert_eq!(kind, "done");
    assert_eq!(
        reason,
        "Committed skip for \"domain-design\" (scope: classic). State routed forward; run next to continue."
    );
    let state = workspace.state_file().expect("投影済み");
    assert!(state.contains("- [S] domain-design"), "{state}");
    assert!(state.contains("- [-] contract-design"), "{state}");
    let audit = workspace.audit_shard().expect("監査シャード");
    // 理由は trim して運ぶ（upstream も `flags.reason?.trim()` を渡す）。
    assert!(audit.contains("**Reason**: out of scope"), "{audit}");
}

/// 最後のゲートを畳むとワークフローが完了し、その再報告は冪等な `done` になる。
#[tokio::test]
async fn completing_the_last_gate_projects_the_workflow_completion_and_folds_a_re_report() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    for stage in ["domain-design", "contract-design"] {
        let (kind, _) = report_directive(
            &workspace,
            &[
                "--result",
                "approved",
                "--user-input",
                "A",
                "--stage",
                stage,
            ],
        )
        .await;
        assert_eq!(kind, "done");
    }

    // 完了の投影 — 状態 7 欄とフェーズ行、監査 3 行。
    let state = workspace.state_file().expect("投影済み");
    for field in [
        "- **Status**: Completed",
        "- **In Progress**: none",
        "- **Next Stage**: none",
        "- **Next Action**: Workflow complete",
        "- **Last Completed Stage**: contract-design",
        "- **Completed**: 3",
        "- **Inception**: Verified",
    ] {
        assert!(state.contains(field), "{field}:\n{state}");
    }
    let audit = workspace.audit_shard().expect("監査シャード");
    for row in [
        "**Event**: PHASE_COMPLETED",
        "**To phase**: (end)",
        "**Phase boundary**: inception → end",
        "**Event**: WORKFLOW_COMPLETED",
        "**Details**: Scope: classic, 3 stages completed",
    ] {
        assert!(audit.contains(row), "{row}:\n{audit}");
    }

    // 完了済みへの再報告は何もコミットせず、新規作業の出口案内が付く。
    let (kind, reason) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--user-input",
            "A",
            "--stage",
            "contract-design",
        ],
    )
    .await;
    assert_eq!(kind, "done");
    assert!(
        reason.starts_with(
            "Workflow is already completed at \"contract-design\" (scope: classic); no transition was needed."
        ),
        "{reason}"
    );
    assert!(
        reason.contains("If this input is genuinely NEW, unrelated work"),
        "{reason}"
    );
    assert_eq!(
        workspace.state_file().expect("投影済み"),
        state,
        "冪等な再報告は状態を 1 バイトも動かさない"
    );
}

/// 段 1 — 状態ファイルが**在るのに読めない**なら、版が読めないのとは別に材料を出して断る。
#[tokio::test]
async fn an_unreadable_state_file_is_reported_with_its_place_and_cause() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let state_file = workspace
        .record_dir()
        .expect("record")
        .join("aidlc-state.md");
    // 骨格の置き場をディレクトリで塞ぐ（在るが文字列として読めない）。
    let before = workspace.journal_rows();
    fs::remove_file(&state_file).expect("骨格");
    fs::create_dir(&state_file).expect("塞ぐ");

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "approved"],
    )
    .await;
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: projection restoration: publication conflict: ",
        &state_file,
    );
    assert_eq!(
        workspace.journal_rows(),
        before,
        "復旧拒否でイベントを増やさない"
    );
    assert!(state_file.is_dir(), "障害箇所を勝手に上書きしない");
}

/// 段 4 — 状態ファイルは在るのに現在地が引けなければ、再開先を名乗れない。
#[tokio::test]
async fn a_resume_without_a_resolvable_cursor_is_refused() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    // 実行カーソルだけを外す — 状態ファイルは残るので段 4 の state 判定は通り、
    // 現在地の引当だけが空になる。
    fs::remove_file(
        workspace
            .record_dir()
            .expect("record")
            .join(".aidlc-execution"),
    )
    .expect("実行カーソル");

    let (kind, message) =
        report_directive(&workspace, &["--result", "resumed", "--user-input", "1"]).await;

    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "State file has no Current Stage field - cannot resume from the last checkpoint."
    );
}

/// 段 4 — 引けないリードモデルは「現在地が無い」と混ぜない（PR #103 レビュー指摘）。
#[tokio::test]
async fn a_resume_whose_read_model_cannot_be_read_names_the_store() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    // 「開けるが引けない」形にする（上と同じ細工）。状態ファイルは在るので段 4 の
    // state 判定は通り、現在地の引当だけが落ちる。
    let before = workspace.journal_rows();
    break_read_execution(&workspace);
    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "resumed", "--user-input", "1"],
    )
    .await;
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: journal: io: Other at ",
        &workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite"),
    );
    assert_eq!(
        workspace.journal_rows(),
        before,
        "復旧に失敗したらイベントは書かない"
    );
}

/// 段 4 — 壊れた実行カーソルは「現在地が無い」と混ぜない（PR #103 レビュー指摘）。
#[tokio::test]
async fn a_resume_with_a_broken_execution_cursor_names_the_cursor() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    let record = workspace.record_dir().expect("record");
    fs::write(record.join(".aidlc-execution"), "not-an-id\nalso-not\n").expect("カーソルを壊す");

    let (kind, message) =
        report_directive(&workspace, &["--result", "resumed", "--user-input", "1"]).await;

    assert_eq!(kind, "error");
    assert!(
        message.starts_with("The execution cursor cannot be read"),
        "{message}"
    );
}

/// 段 4 — リードモデルを**開けない**ときも所在を名指す（引けないときと同じ規律）。
#[tokio::test]
async fn a_resume_whose_store_cannot_be_opened_names_the_store() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    // 置き場をディレクトリで塞ぐ — 開くことすらできない形である。
    let store = workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let before = workspace.journal_rows();
    let saved = store.with_extension("saved.sqlite");
    fs::rename(&store, &saved).expect("真実記録は保持して置き場を塞ぐ");
    fs::create_dir(&store).expect("塞ぐ");

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "resumed", "--user-input", "1"],
    )
    .await;
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: journal: io: NotFound at ",
        &store,
    );
    fs::remove_dir(&store).expect("障害を除去");
    fs::rename(&saved, &store).expect("真実記録を戻す");
    assert_eq!(workspace.journal_rows(), before);
}

/// forward 表 — 差し戻し中 (`[R]`) のステージは前進の完了ではない。
#[tokio::test]
async fn a_revising_stage_cannot_be_reported_as_a_forward_completion() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "rejected", "--reason", "直して"],
    )
    .await;

    let (kind, message) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--user-input",
            "A",
            "--stage",
            "domain-design",
        ],
    )
    .await;

    assert_eq!(kind, "error");
    assert_eq!(
        message,
        "Stage \"domain-design\" is revising; report commits forward completions only."
    );
}

// ---------------------------------------------------------------------------
// b48: レビュー受領証 (#51 / B10)
// ---------------------------------------------------------------------------

/// `aidlc-log review` を 1 回叩く。
async fn log_review(workspace: &Workspace, args: &[&str]) -> aidlc::runtime::Completion {
    let mut argv = vec!["review"];
    argv.extend_from_slice(args);
    invoke(workspace, "aidlc-log", &argv).await
}

/// 依頼 1 件（成功を確かめて stdout の 1 行を返す）。
async fn request_review(workspace: &Workspace, iteration: &str) -> String {
    let completion = log_review(
        workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            iteration,
        ],
    )
    .await;
    assert_eq!(completion.code(), 0, "依頼は通る: {completion:?}");
    completion.line().expect("stdout に 1 行が要る").to_string()
}

/// 判定 1 件（成功を確かめて stdout の 1 行を返す）。
async fn record_verdict(workspace: &Workspace, iteration: &str, verdict: &str) -> String {
    let completion = log_review(
        workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            iteration,
            "--verdict",
            verdict,
        ],
    )
    .await;
    assert_eq!(completion.code(), 0, "判定は通る: {completion:?}");
    completion.line().expect("stdout に 1 行が要る").to_string()
}

/// 依頼 → 判定 → 承認の一巡が通り、監査台帳に受領証 2 行が並ぶ。
#[tokio::test]
async fn the_review_round_trip_lets_the_gate_be_approved_and_records_both_rows() {
    let workspace = Workspace::with_reviewer(None, None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    assert_eq!(
        request_review(&workspace, "1").await,
        r#"{"emitted":"REVIEW_REQUESTED","stage":"domain-design"}"#
    );
    assert_eq!(
        record_verdict(&workspace, "1", "READY").await,
        r#"{"emitted":"REVIEW_COMPLETED","stage":"domain-design"}"#
    );

    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "done", "{body}");
    assert!(
        body.starts_with("Committed gate-start + approve for \"domain-design\""),
        "{body}"
    );

    // 監査台帳に受領証 2 行が upstream のフィールド順で並ぶ。
    let audit = workspace.audit_shard().expect("監査シャードは在る");
    assert!(audit.contains("**Event**: REVIEW_REQUESTED\n"), "{audit}");
    assert!(
        audit.contains(&format!(
            "**Stage**: domain-design\n**Reviewer**: {REVIEWER}\n**Iteration**: 1\n"
        )),
        "{audit}"
    );
    assert!(
        audit.contains(&format!(
            "**Stage**: domain-design\n**Reviewer**: {REVIEWER}\n**Iteration**: 1\n**Verdict**: READY\n"
        )),
        "{audit}"
    );
}

/// 受領証が無い承認は段 11 で拒まれる（`aidlc-state.ts approve` の逐語を包み文に入れて）。
#[tokio::test]
async fn approving_a_reviewer_bearing_stage_without_a_receipt_is_refused_verbatim() {
    let workspace = Workspace::with_reviewer(None, None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "error", "{body}");
    assert_eq!(
        body,
        format!(
            "Transition rejected by aidlc-state.ts approve for \"domain-design\": \
Refusing to complete \"domain-design\": it declares a reviewer ({REVIEWER}) but no fresh \
REVIEW_COMPLETED is recorded for it. Invoke the reviewer (stage-protocol-reviewer.md §12a) and \
record the verdict with `aidlc-log.ts review --stage domain-design --reviewer {REVIEWER} \
--verdict <READY|NOT-READY>` before completing. Terminal ordering: apply any fixes FIRST, then \
run the reviewer, record the receipt, and stop editing produces[] artifacts - a later write to \
one invalidates the receipt and re-opens this refusal. Do not apply suggestions riding on a \
READY verdict; surface them at the gate instead."
        )
    );
}

/// adversarial の NOT-READY は 1 回目では終端にならず、上限の 2 回目で終端になる。
#[tokio::test]
async fn an_adversarial_not_ready_only_becomes_terminal_at_the_iteration_cap() {
    let workspace = Workspace::with_reviewer(Some("adversarial"), None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    request_review(&workspace, "1").await;
    record_verdict(&workspace, "1", "NOT-READY").await;
    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "error", "1 回目の NOT-READY は終端ではない: {body}");

    request_review(&workspace, "2").await;
    record_verdict(&workspace, "2", "NOT-READY").await;
    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "done", "上限到達の NOT-READY は終端である: {body}");
}

/// advisory は 1 パスで終端になる（NOT-READY でも承認が通る）。
#[tokio::test]
async fn an_advisory_pass_is_terminal_at_the_first_verdict() {
    let workspace = Workspace::with_reviewer(Some("advisory"), None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    request_review(&workspace, "1").await;
    record_verdict(&workspace, "1", "NOT-READY").await;
    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "done", "{body}");

    // 予算 1 なので 2 回目の依頼は断られる（advisory の言い回し）。
    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "contract-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "1",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some("Cannot record review: stage \"contract-design\" has no declared reviewer.")
    );
}

/// scope の `review_cap: none` は受領証の要求そのものを解く。
#[tokio::test]
async fn a_scope_cap_of_none_waives_the_receipt_entirely() {
    let workspace = Workspace::with_reviewer(None, Some("none"));
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "done", "実効 none は受領証を要らない: {body}");
}

/// 呼び直しは `retry` を載せた JSON を返し、依頼の回数には数えない。
#[tokio::test]
async fn a_retry_pending_request_reports_the_retry_and_does_not_spend_the_budget() {
    let workspace = Workspace::with_reviewer(Some("adversarial"), None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    request_review(&workspace, "1").await;
    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "1",
            "--retry-pending",
        ],
    )
    .await;
    assert_eq!(completion.code(), 0, "{completion:?}");
    assert_eq!(
        completion.line(),
        Some(r#"{"emitted":"REVIEW_REQUESTED","stage":"domain-design","retry":"pending-request"}"#)
    );
    // 呼び直しは数えないので、次の通常依頼は依然として 2 番である。
    request_review(&workspace, "2").await;
    // 監査台帳に `Retry` 行が並ぶ。
    let audit = workspace.audit_shard().expect("監査シャードは在る");
    assert!(audit.contains("**Retry**: pending-request\n"), "{audit}");
}

/// 予算超過・順序違反・対にならない判定・宣言不一致の 4 拒否（逐語）。
#[tokio::test]
async fn the_review_refusals_are_verbatim() {
    let workspace = Workspace::with_reviewer(Some("adversarial"), None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    // 順序違反 — 1 番から始まらない。
    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "2",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some(
            "Refusing REVIEW_REQUESTED for \"domain-design\": iteration 2 is out of sequence; \
expected 1 from the current audit attempt."
        )
    );

    // 予算超過 — 上限 2 を超える通し番号。
    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "3",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some(
            "Refusing REVIEW_REQUESTED for \"domain-design\": review request 3 exceeds this \
stage's review budget (2). The review loop is exhausted - present the gate with the unresolved \
findings for the human's decision instead of another review pass."
        )
    );

    // 対にならない判定。
    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "1",
            "--verdict",
            "READY",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some(
            "Refusing REVIEW_COMPLETED for \"domain-design\": no unmatched REVIEW_REQUESTED \
iteration 1 exists in the current audit attempt."
        )
    );

    // 対にならない呼び直し。
    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "1",
            "--retry-pending",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some(
            "Refusing review retry for \"domain-design\": no unmatched REVIEW_REQUESTED \
iteration 1 exists in the current audit attempt."
        )
    );

    // 宣言不一致。
    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            "someone-else",
            "--iteration",
            "1",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some(&*format!(
            "Cannot record review for \"domain-design\": reviewer \"someone-else\" does not \
match the declared reviewer \"{REVIEWER}\"."
        ))
    );
}

/// 構文段の拒否（フラグ文法・必須フラグ・セレクタ・未配線 2 面・値の閉集合）。
#[tokio::test]
async fn the_review_syntax_guards_refuse_in_the_upstream_order() {
    let workspace = Workspace::with_reviewer(None, None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    async fn refusal(workspace: &Workspace, args: &[&str]) -> String {
        let completion = log_review(workspace, args).await;
        assert_eq!(completion.code(), 1, "{completion:?}");
        completion
            .diagnostic()
            .expect("stderr に逐語が要る")
            .to_string()
    }

    // フラグ文法 — 値が必要なフラグに値が無い。
    assert_eq!(
        refusal(&workspace, &["--stage"]).await,
        "--stage expects a value, got end of arguments."
    );
    assert_eq!(
        refusal(&workspace, &["--stage", "--reviewer"]).await,
        "--stage expects a value, got another flag: \"--reviewer\". Did you forget the value?"
    );
    // 必須フラグ。
    assert_eq!(refusal(&workspace, &[]).await, "Missing --stage <slug>");
    assert_eq!(
        refusal(&workspace, &["--stage", "domain-design"]).await,
        "Missing --reviewer <agent>"
    );
    // セレクタは受け付けない。
    assert_eq!(
        refusal(
            &workspace,
            &[
                "--stage",
                "domain-design",
                "--reviewer",
                REVIEWER,
                "--intent",
                "x"
            ]
        )
        .await,
        "The review command does not accept --intent/--space selectors. Switch to the target \
workspace first."
    );
    // 未配線の 2 面（own wording）。
    assert!(
        refusal(
            &workspace,
            &[
                "--stage",
                "domain-design",
                "--reviewer",
                REVIEWER,
                "--unit",
                "b48"
            ]
        )
        .await
        .contains("the --unit receipt is not wired in this build")
    );
    assert!(
        refusal(
            &workspace,
            &[
                "--stage",
                "domain-design",
                "--reviewer",
                REVIEWER,
                "--single"
            ]
        )
        .await
        .contains("the --single receipt is not wired in this build")
    );
    // 通し番号は正整数（依頼形 / 判定形で文言が分かれる）。
    assert_eq!(
        refusal(
            &workspace,
            &["--stage", "domain-design", "--reviewer", REVIEWER]
        )
        .await,
        "REVIEW_REQUESTED requires --iteration <positive integer>."
    );
    assert_eq!(
        refusal(
            &workspace,
            &[
                "--stage",
                "domain-design",
                "--reviewer",
                REVIEWER,
                "--iteration",
                "0"
            ]
        )
        .await,
        "REVIEW_REQUESTED requires --iteration <positive integer>."
    );
    assert_eq!(
        refusal(
            &workspace,
            &[
                "--stage",
                "domain-design",
                "--reviewer",
                REVIEWER,
                "--verdict",
                "READY"
            ]
        )
        .await,
        "REVIEW_COMPLETED requires --iteration <positive integer>."
    );
    // `--retry-pending` と `--verdict` の併用。
    assert_eq!(
        refusal(
            &workspace,
            &[
                "--stage",
                "domain-design",
                "--reviewer",
                REVIEWER,
                "--iteration",
                "1",
                "--verdict",
                "READY",
                "--retry-pending"
            ]
        )
        .await,
        "--retry-pending cannot be combined with --verdict."
    );
    // 判定の閉集合。
    assert_eq!(
        refusal(
            &workspace,
            &[
                "--stage",
                "domain-design",
                "--reviewer",
                REVIEWER,
                "--iteration",
                "1",
                "--verdict",
                "maybe"
            ]
        )
        .await,
        "Unknown --verdict \"maybe\". Accepted: READY, NOT-READY."
    );
    // 未知の slug は「宣言が無い」と同じ答えである（upstream の `find` が空振りするだけ）。
    assert_eq!(
        refusal(
            &workspace,
            &[
                "--stage",
                "nowhere",
                "--reviewer",
                REVIEWER,
                "--iteration",
                "1"
            ]
        )
        .await,
        "Cannot record review: stage \"nowhere\" has no declared reviewer."
    );
    // slug が文法違反でも、依頼形の `--iteration` 検査が先に出る（upstream は
    // `handleReview` の `:983-985` で `loadContext` より前に通し番号を検査する）。
    assert_eq!(
        refusal(
            &workspace,
            &["--stage", "Not A Slug", "--reviewer", REVIEWER]
        )
        .await,
        "REVIEW_REQUESTED requires --iteration <positive integer>."
    );
}

/// 記録面の未知動詞と未配線動詞は stderr + exit 1 である。
#[tokio::test]
async fn the_log_face_refuses_unknown_and_unwired_verbs() {
    let workspace = Workspace::with_reviewer(None, None);

    let completion = invoke(&workspace, "aidlc-log", &["frobnicate"]).await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some("Unknown subcommand: frobnicate. Valid: decision, answer, link, review")
    );

    let completion = invoke(&workspace, "aidlc-log", &[]).await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some("Unknown subcommand: undefined. Valid: decision, answer, link, review")
    );

    for verb in ["decision", "answer", "link"] {
        let completion = invoke(&workspace, "aidlc-log", &[verb]).await;
        assert_eq!(completion.code(), 1);
        assert_eq!(
            completion.diagnostic(),
            Some(&*format!(
                "Cannot record a {verb} event: the aidlc-log {verb} verb is not wired in this \
build. Only `review` is available."
            ))
        );
    }
}

/// 鋳造前のワークスペースは「アクティブな intent が無い」で断る。
#[tokio::test]
async fn a_review_without_an_active_intent_is_refused() {
    let workspace = Workspace::with_reviewer(None, None);
    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "1",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some("Cannot resolve the active intent for review logging.")
    );
}

/// 差し戻しは試行のフロアである — 受領証を積み直さないと再承認できない。
#[tokio::test]
async fn a_gate_rejection_resets_the_attempt_and_the_receipt_must_be_recorded_again() {
    let workspace = Workspace::with_reviewer(Some("advisory"), None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    request_review(&workspace, "1").await;
    record_verdict(&workspace, "1", "READY").await;
    report_directive(
        &workspace,
        &["--result", "rejected", "--reason", "Sharpen the design."],
    )
    .await;
    report_directive(&workspace, &["--result", "revised"]).await;

    // 差し戻しで試行が空に戻っているので、承認は再び拒まれる。
    let (kind, _) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "error", "差し戻し後は受領証を積み直す");

    // 通し番号も数え直しなので 1 番から始まる。
    request_review(&workspace, "1").await;
    record_verdict(&workspace, "1", "READY").await;
    let (kind, _) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "done", "積み直した受領証で承認が通る");
}

/// 壊れた実行カーソルは「不在」と混ぜない（`report` 段 6 と同じ規律）。
#[tokio::test]
async fn a_review_on_a_broken_execution_cursor_names_the_medium_failure() {
    let workspace = Workspace::with_reviewer(None, None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;
    let record = workspace.record_dir().expect("カーソルが据わっている");
    fs::write(record.join(".aidlc-execution"), "not-an-id\nalso-not\n").expect("カーソルを壊す");

    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "1",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert!(
        completion
            .diagnostic()
            .expect("stderr に逐語が要る")
            .starts_with("The execution cursor cannot be read"),
        "{completion:?}"
    );
}

/// 文法外の slug も「宣言が無い」と同じ答えである（upstream の `find` は空振りするだけ）。
#[tokio::test]
async fn a_review_stage_outside_the_slug_grammar_reads_as_no_declared_reviewer() {
    let workspace = Workspace::with_reviewer(None, None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "Not A Slug",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "1",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some("Cannot record review: stage \"Not A Slug\" has no declared reviewer.")
    );
}

/// 空の `--iteration` も正整数の文法から外れる。
#[tokio::test]
async fn an_empty_iteration_is_outside_the_positive_integer_grammar() {
    let workspace = Workspace::with_reviewer(None, None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;

    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some("REVIEW_REQUESTED requires --iteration <positive integer>.")
    );
}

/// 空間名が壊れていれば、レビューの記録もストアを開かずに断る。
#[tokio::test]
async fn a_review_under_an_invalid_active_space_is_refused_before_the_store() {
    let workspace = Workspace::with_reviewer(None, None);
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "review"],
    )
    .await;
    fs::write(workspace.path("aidlc/active-space"), "../escape\n").expect("空間カーソルを壊す");

    let completion = log_review(
        &workspace,
        &[
            "--stage",
            "domain-design",
            "--reviewer",
            REVIEWER,
            "--iteration",
            "1",
        ],
    )
    .await;
    assert_eq!(completion.code(), 1);
    assert!(
        completion
            .diagnostic()
            .expect("stderr に逐語が要る")
            .starts_with("The active space \"../escape\""),
        "{completion:?}"
    );
}

// ---------------------------------------------------------------------------
// b49: 昇格受領証 (#7 キュー 5 の残り / B10)
// ---------------------------------------------------------------------------

/// `aidlc-state <verb>` を 1 回叩く。
async fn state_verb(workspace: &Workspace, args: &[&str]) -> aidlc::runtime::Completion {
    invoke(workspace, "aidlc-state", args).await
}

/// 昇格を 1 回打つ（既定のドラフト 2 本）。
async fn promote(workspace: &Workspace) -> aidlc::runtime::Completion {
    let team = workspace.draft("team-practices.md");
    let rules = workspace.draft("discovered-rules.md");
    state_verb(
        workspace,
        &[
            "practices-promote",
            "--team-practices",
            &team,
            "--discovered-rules",
            &rules,
            "--affirming-user",
            "owner",
        ],
    )
    .await
}

/// 鋳造して昇格の準備が整ったワークスペース。
async fn minted_practices() -> Workspace {
    let workspace = Workspace::with_practices();
    invoke(
        &workspace,
        "aidlc-utility",
        &[
            "intent-create",
            "--scope",
            "classic",
            "--label",
            "practices",
        ],
    )
    .await;
    workspace
}

/// 昇格 → 4 面 → 承認の一巡が通る。
#[tokio::test]
async fn the_promotion_writes_the_memory_layer_and_opens_the_gate() {
    let workspace = minted_practices().await;

    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 0, "昇格は通る: {completion:?}");
    let line = completion.line().expect("stdout に 1 行が要る").to_string();
    assert!(
        line.starts_with(
            r#"{"emitted":"PRACTICES_AFFIRMED","sections_written":["Way of Working"],"mandated_appended":1,"forbidden_appended":1,"affirmed_at":""#
        ),
        "{line}"
    );
    assert!(
        line.contains(r#""team_md":"#) && line.contains(r#""project_guardrails":"#),
        "{line}"
    );

    // team.md の節が置き換わる（他の 4 節は据え置き）。
    let team = workspace.memory_file("team.md");
    assert!(team.contains("## Way of Working\ntrunk-based.\n"), "{team}");
    assert!(team.contains("## Code Style\nold style.\n"), "{team}");

    // project.md に印付きの行が並ぶ。
    let project = workspace.memory_file("project.md");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    assert!(
        project.contains(&format!("ALWAYS review. (affirmed {today})")),
        "{project}"
    );
    assert!(
        project.contains(&format!("NEVER force-push. (affirmed {today})")),
        "{project}"
    );

    // 状態ファイルのタイムスタンプは stdout の `affirmed_at` と同じ値である
    // （upstream の 2 部受領証は状態ファイルの値と監査行の時刻を突き合わせる）。
    let affirmed_at = string_of(&parse(&line).expect("JSON として読める"), "affirmed_at");
    let state = workspace.state_file().expect("状態ファイルは在る");
    assert!(
        state.contains(&format!(
            "- **Practices Affirmed Timestamp**: {affirmed_at}\n"
        )),
        "{state}"
    );
    assert!(
        state.contains(&format!("- **Last Updated**: {affirmed_at}\n")),
        "{state}"
    );

    // 監査行は upstream のフィールド順で 1 行。
    let audit = workspace.audit_shard().expect("監査シャードは在る");
    assert!(audit.contains("**Event**: PRACTICES_AFFIRMED\n"), "{audit}");
    assert!(
        audit.contains(
            "**Affirming User**: owner\n**Sections Written**: Way of Working\n**Mandated Rules Appended**: 1\n**Forbidden Rules Appended**: 1\n"
        ),
        "{audit}"
    );

    // 受領証が立ったので承認が通る。
    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "practices-discovery",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "done", "{body}");
}

/// 受領証が無い承認は段 12 で断られる（orchestrate 自身の error directive）。
#[tokio::test]
async fn approving_practices_discovery_without_a_promotion_is_refused_verbatim() {
    let workspace = minted_practices().await;

    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "practices-discovery",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "error", "{body}");
    assert_eq!(
        body,
        "Cannot approve \"practices-discovery\" before practices-promote succeeds. Run \
aidlc-state.ts practices-promote after the human approves; it records Practices Affirmed \
Timestamp and a fresh PRACTICES_AFFIRMED receipt for this stage attempt, then report --result \
approved --user-input \"<exact choice>\"."
    );
}

/// 差し戻しは受領証を落とす — 積み直さないと承認できない。
#[tokio::test]
async fn a_gate_rejection_floors_the_receipt_and_the_promotion_must_be_replayed() {
    let workspace = minted_practices().await;
    promote(&workspace).await;

    // 差し戻しも再入も `print` の directive である（遷移は 1 つコミットされる）。
    let (kind, _) = report_directive(
        &workspace,
        &["--result", "rejected", "--reason", "Sharpen the practices."],
    )
    .await;
    assert_eq!(kind, "print");
    let (kind, _) = report_directive(&workspace, &["--result", "revised"]).await;
    assert_eq!(kind, "print");

    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "practices-discovery",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "error", "差し戻しは受領証を落とす: {body}");

    // 積み直せば通る。
    assert_eq!(promote(&workspace).await.code(), 0);
    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "practices-discovery",
            "--user-input",
            "A",
        ],
    )
    .await;
    assert_eq!(kind, "done", "{body}");
}

/// 再昇格は正本に重複を積まない（印付き行の trim 一致で除かれる）。
#[tokio::test]
async fn a_second_promotion_does_not_duplicate_the_stamped_rules() {
    let workspace = minted_practices().await;
    assert_eq!(promote(&workspace).await.code(), 0);
    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 0, "{completion:?}");
    // 2 回目は足す規則が無い（既在なので除かれる）。
    let line = completion.line().expect("stdout に 1 行").to_string();
    assert!(
        line.contains(r#""mandated_appended":0,"forbidden_appended":0"#),
        "{line}"
    );
    let project = workspace.memory_file("project.md");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    assert_eq!(
        project
            .matches(&format!("ALWAYS review. (affirmed {today})"))
            .count(),
        1,
        "{project}"
    );
    // 節の置換は毎回行われる（同じ本文なので結果は同じ）。
    assert!(
        workspace
            .memory_file("team.md")
            .contains("## Way of Working\ntrunk-based.\n")
    );
}

/// 昇格の拒否 — 構文段と Step 1〜4 の逐語。
#[tokio::test]
async fn the_promotion_refusals_are_verbatim() {
    let workspace = minted_practices().await;
    let team = workspace.draft("team-practices.md");
    let rules = workspace.draft("discovered-rules.md");

    let refusal = |completion: aidlc::runtime::Completion| -> String {
        assert_eq!(completion.code(), 1, "{completion:?}");
        completion.diagnostic().expect("stderr に 1 行").to_string()
    };

    // 必須フラグの欠落（2 形とも同じ usage）。
    assert_eq!(
        refusal(state_verb(&workspace, &["practices-promote"]).await),
        "Usage: aidlc-state.ts practices-promote --team-practices <path> --discovered-rules <path> [--affirming-user <name>] [--target-dir <path>]"
    );
    assert_eq!(
        refusal(
            state_verb(
                &workspace,
                &["practices-promote", "--team-practices", &team]
            )
            .await
        ),
        "Usage: aidlc-state.ts practices-promote --team-practices <path> --discovered-rules <path> [--affirming-user <name>] [--target-dir <path>]"
    );

    // `--target-dir` は未配線。
    assert_eq!(
        refusal(
            state_verb(
                &workspace,
                &[
                    "practices-promote",
                    "--team-practices",
                    &team,
                    "--discovered-rules",
                    &rules,
                    "--target-dir",
                    "/tmp/elsewhere",
                ]
            )
            .await
        ),
        "Cannot redirect the promotion: --target-dir is not wired in this build. The affirmed practices are written to the active space's memory directory."
    );

    // ドラフト 2 本の親ディレクトリが違う。
    let elsewhere = workspace
        .path("elsewhere.md")
        .to_string_lossy()
        .into_owned();
    fs::write(&elsewhere, "## Mandated\n").expect("elsewhere");
    assert_eq!(
        refusal(
            state_verb(
                &workspace,
                &[
                    "practices-promote",
                    "--team-practices",
                    &team,
                    "--discovered-rules",
                    &elsewhere,
                ]
            )
            .await
        ),
        "practices-promote failed: team-practices and discovered-rules drafts must share one stage directory"
    );

    // contributions の 2 形（不在 / identity marker 違い）。
    let contributions = workspace.path("drafts/contributions");
    fs::remove_file(contributions.join("aidlc-developer-agent.md")).expect("消す");
    fs::write(contributions.join("aidlc-quality-agent.md"), "所見。\n").expect("書く");
    assert_eq!(
        refusal(promote(&workspace).await),
        "practices-promote failed: ensemble evidence is incomplete: \
aidlc-developer-agent (no contribution file); \
aidlc-quality-agent (missing identity-marker first line)"
    );
    workspace.write_drafts();

    // ドラフトが無い。
    fs::remove_file(&team).expect("消す");
    assert_eq!(
        refusal(promote(&workspace).await),
        format!("practices-promote failed: team-practices draft not found: {team}")
    );
    workspace.write_drafts();
    fs::remove_file(&rules).expect("消す");
    assert_eq!(
        refusal(promote(&workspace).await),
        format!("practices-promote failed: discovered-rules draft not found: {rules}")
    );
    workspace.write_drafts();

    // 正本が無い。
    let team_md = workspace.path("aidlc/spaces/default/memory/team.md");
    fs::remove_file(&team_md).expect("消す");
    assert_eq!(
        refusal(promote(&workspace).await),
        format!(
            "practices-promote failed: team.md not found at {}",
            team_md.display()
        )
    );
    workspace.write_memory_targets();
    let project_md = workspace.path("aidlc/spaces/default/memory/project.md");
    fs::remove_file(&project_md).expect("消す");
    assert_eq!(
        refusal(promote(&workspace).await),
        format!(
            "practices-promote failed: project.md not found at {}",
            project_md.display()
        )
    );
    workspace.write_memory_targets();

    // 正本に置換先の見出しが無い。
    fs::write(&team_md, "# Team\n\n## Deployment\nx.\n").expect("書く");
    assert_eq!(
        refusal(promote(&workspace).await),
        "practices-promote failed: replaceSection failed on team.md for \"## Way of Working\": \
replaceSection: heading not found: ## Way of Working"
    );
    workspace.write_memory_targets();

    // 正本に追記先の見出しが無い。
    fs::write(&project_md, "# Project\n\n## Corrections\n").expect("書く");
    assert_eq!(
        refusal(promote(&workspace).await),
        "practices-promote failed: appendUnderHeading failed on Mandated: \
appendUnderHeading: heading not found: ## Mandated"
    );
}

/// practices-discovery を持たないグラフでは、昇格は Step 1 で断られる。
#[tokio::test]
async fn a_graph_without_practices_discovery_refuses_the_promotion() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &[
            "intent-create",
            "--scope",
            "classic",
            "--label",
            "no-practices",
        ],
    )
    .await;
    let drafts = workspace.path("drafts");
    fs::create_dir_all(&drafts).expect("drafts");
    fs::write(drafts.join("team-practices.md"), "## Way of Working\nx.\n").expect("draft");
    fs::write(drafts.join("discovered-rules.md"), "## Mandated\n").expect("draft");

    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(
        completion.diagnostic(),
        Some(
            "practices-promote failed: practices-discovery is absent from the compiled stage graph"
        )
    );
}

/// 鋳造前の昇格は「アクティブ intent が無い」で断られる。
#[tokio::test]
async fn a_promotion_without_an_active_intent_is_refused() {
    let workspace = Workspace::with_practices();
    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(
        completion.diagnostic(),
        Some("Cannot resolve the active intent for practices promotion.")
    );
}

/// 状態面の未知動詞と、認識はするが未配線の動詞。
#[tokio::test]
async fn the_state_face_refuses_unknown_and_unwired_verbs() {
    let workspace = Workspace::with_practices();

    let completion = state_verb(&workspace, &["frobnicate"]).await;
    assert_eq!(completion.code(), 1);
    assert_eq!(
        completion.diagnostic(),
        Some(
            "Unknown subcommand: frobnicate. Valid: get, set, set-skeleton-stance, \
set-construction-iteration, checkbox, count, advance, finalize, complete-workflow, gate-start, \
approve, reject, revise, skip, resume, acknowledge-compaction, reuse-artifact, lookup, \
practices-event, practices-promote, fork, merge, park, unpark"
        )
    );

    let completion = state_verb(&workspace, &[]).await;
    assert_eq!(completion.code(), 1);
    assert!(
        completion
            .diagnostic()
            .is_some_and(|line| line.starts_with("Unknown subcommand: undefined. Valid: ")),
        "{completion:?}"
    );

    for verb in ["approve", "practices-event", "park", "unit"] {
        let completion = state_verb(&workspace, &[verb]).await;
        assert_eq!(completion.code(), 1);
        assert_eq!(
            completion.diagnostic(),
            Some(
                format!(
                    "Cannot run aidlc-state {verb}: the {verb} subcommand is not wired in this \
build. Only `practices-promote` is available."
                )
                .as_str()
            )
        );
    }
}

/// 実行カーソルが壊れているのは「不在」と混ぜない。
#[tokio::test]
async fn a_promotion_on_a_broken_execution_cursor_names_the_medium_failure() {
    let workspace = minted_practices().await;
    let record = workspace.record_dir().expect("record");
    fs::write(record.join(".aidlc-execution"), "not-a-uuid\n").expect("壊す");

    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert!(
        completion
            .diagnostic()
            .is_some_and(|line| line.starts_with("The execution cursor cannot be read")),
        "{completion:?}"
    );
}

/// 昇格の媒体失敗 3 形 — 壊れた active-space、読めないドラフト、読めない正本。
#[tokio::test]
async fn a_promotion_surfaces_every_medium_failure() {
    // active-space が空間名の文法外なら、ストアを開く前に断る。
    let workspace = minted_practices().await;
    fs::write(workspace.path("aidlc/active-space"), "Not A Space\n").expect("空間名を壊す");
    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert!(
        completion
            .diagnostic()
            .is_some_and(|line| line
                .starts_with("The active space \"Not A Space\" is not a valid space name.")),
        "{completion:?}"
    );

    // ドラフトが「在るのに読めない」（位置にディレクトリが居る）。
    let workspace = minted_practices().await;
    let team = workspace.draft("team-practices.md");
    fs::remove_file(&team).expect("消す");
    fs::create_dir(&team).expect("ディレクトリを置く");
    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert!(
        completion.diagnostic().is_some_and(
            |line| line.starts_with("practices-promote failed: could not read drafts: ")
        ),
        "{completion:?}"
    );

    // 正本が「在るのに読めない」（同上）。
    let workspace = minted_practices().await;
    let team_md = workspace.path("aidlc/spaces/default/memory/team.md");
    let before = workspace.journal_rows();
    fs::remove_file(&team_md).expect("消す");
    fs::create_dir(&team_md).expect("ディレクトリを置く");
    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: projection restoration: publication conflict: ",
        &team_md,
    );
    assert_eq!(workspace.journal_rows(), before);
    assert!(team_md.is_dir(), "正本の障害を上書きしない");
}

/// カーソルが指す実行がジャーナルに居なければ、ユースケースの失敗が材料ごと上がる。
#[tokio::test]
async fn a_promotion_against_an_absent_execution_relays_the_repository_failure() {
    let workspace = minted_practices().await;
    let record = workspace.record_dir().expect("record");
    let cursor = fs::read_to_string(record.join(".aidlc-execution")).expect("カーソルは読める");
    let intent_line = cursor.lines().nth(1).expect("2 行目は intent id");
    fs::write(
        record.join(".aidlc-execution"),
        format!("{ABSENT_EXECUTION}\n{intent_line}\n"),
    )
    .expect("別の実行を指す");

    let completion = promote(&workspace).await;
    assert_eq!(completion.code(), 1, "{completion:?}");
    assert!(
        completion
            .diagnostic()
            .is_some_and(|line| line.starts_with("practices-promote failed: repository: ")),
        "{completion:?}"
    );
}

/// 事前復旧が回らなければ、昇格を記録する前に拒否する。
#[tokio::test]
async fn a_restoration_failure_prevents_the_promotion_from_being_recorded() {
    let workspace = minted_practices().await;
    // clone id の置き場をディレクトリで塞ぐと、投影のシャード名が解決できなくなる。
    let before = workspace.journal_rows();
    let clone_id = workspace.path("aidlc/.aidlc-clone-id");
    fs::remove_file(&clone_id).expect("clone id");
    fs::create_dir(&clone_id).expect("塞ぐ");

    let completion = promote(&workspace).await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert!(
        completion
            .diagnostic()
            .unwrap_or_default()
            .starts_with("aidlc-orchestrate: clone id:"),
        "{completion:?}"
    );
    assert_eq!(
        workspace.journal_rows(),
        before,
        "事前復旧の失敗ではコミットしない"
    );
}

/// 定義の面が読めなければ、ストアの置き場を名指して断る（グラフの不在と混ぜない）。
#[tokio::test]
async fn a_promotion_names_the_unreadable_read_model_instead_of_an_absent_stage() {
    let workspace = minted_practices().await;
    let store = workspace.path("aidlc/spaces/default/intents/.aidlc-store.sqlite");
    let before = workspace.journal_rows();
    let connection = rusqlite::Connection::open(&store).expect("ストアは開ける");
    connection
        .execute_batch(
            "DROP TABLE read_definition_stage; \
             CREATE TABLE read_definition_stage (id TEXT PRIMARY KEY);",
        )
        .expect("リードモデルの表は置き換えられる");

    let completion = promote(&workspace).await;
    assert_refused_at(
        &completion,
        "aidlc-orchestrate: journal: io: Other at ",
        &store,
    );
    assert_eq!(workspace.journal_rows(), before);
}

// ---------------------------------------------------------------------------
// b50: `aidlc-bolt set-autonomy` と human presence ガード (#72 / I11)
// ---------------------------------------------------------------------------

/// `aidlc-bolt <verb>` を 1 回叩く。
async fn bolt_verb(workspace: &Workspace, args: &[&str]) -> aidlc::runtime::Completion {
    invoke(workspace, "aidlc-bolt", args).await
}

/// `set-autonomy --mode <mode>` を 1 回打つ。
async fn set_autonomy(workspace: &Workspace, mode: &str) -> aidlc::runtime::Completion {
    bolt_verb(workspace, &["set-autonomy", "--mode", mode]).await
}

/// 鋳造だけ済ませたワークスペース（自律モードは既定の `gated`）。
async fn minted() -> Workspace {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "autonomy"],
    )
    .await;
    workspace
}

/// 直近のゲート解決より後の人間の turn（実時刻より確実に後ろ）。
const FRESH_TURN: &str = "2099-01-01T00:00:00Z";
/// 直近のゲート解決より前の人間の turn。
const STALE_TURN: &str = "2020-01-01T00:00:00Z";

/// 人間の turn が無ければ昇格は逐語で断られる（I11）。
#[tokio::test]
async fn an_escalation_without_a_human_turn_is_refused_verbatim() {
    let workspace = minted().await;

    let completion = set_autonomy(&workspace, "autonomous").await;

    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(
        completion.diagnostic(),
        Some(
            "Refusing to switch Construction to autonomous: a real human has not acted since the last gate resolution, and autonomous mode is granted only by the human's ladder-prompt answer (it waives every later gate, so the grant itself needs a fresh human turn). Ask the human to confirm autonomous mode in a typed message, then retry. Do not log the ladder choice via aidlc-log answer; the choice is recorded by set-autonomy itself."
        )
    );
    // 断られた実行は状態ファイルも監査台帳も動かさない。
    let state = workspace.state_file().expect("状態ファイルは在る");
    assert!(
        state.contains("- **Construction Autonomy Mode**: gated\n"),
        "{state}"
    );
    let audit = workspace.audit_shard().expect("監査シャードは在る");
    assert!(!audit.contains("AUTONOMY_MODE_SET"), "{audit}");
}

/// 新しい turn が在れば昇格が通り、両面（状態ファイル・監査台帳）へ落ちる。
#[tokio::test]
async fn a_fresh_human_turn_grants_autonomy_and_lands_on_both_faces() {
    let workspace = minted().await;
    workspace.append_human_turn(FRESH_TURN);

    let completion = set_autonomy(&workspace, "autonomous").await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    assert_eq!(
        completion.line(),
        Some(r#"{"emitted":"AUTONOMY_MODE_SET","mode":"autonomous","state_updated":true}"#)
    );
    let state = workspace.state_file().expect("状態ファイルは在る");
    assert!(
        state.contains("- **Construction Autonomy Mode**: autonomous\n"),
        "{state}"
    );
    let audit = workspace.audit_shard().expect("監査シャードは在る");
    assert!(audit.contains("**Event**: AUTONOMY_MODE_SET\n"), "{audit}");
    assert!(audit.contains("**Mode**: autonomous\n"), "{audit}");
}

/// ゲート解決より古い turn しか無ければ断られる（付与がその turn を消費する）。
#[tokio::test]
async fn a_human_turn_older_than_the_last_gate_resolution_is_refused() {
    let workspace = minted().await;
    workspace.append_human_turn(STALE_TURN);
    // 承認 = ゲート解決。ここで解決時刻が「いま」になる。
    let (kind, body) = report_directive(
        &workspace,
        &[
            "--result",
            "approved",
            "--stage",
            "domain-design",
            "--user-input",
            "looks good",
        ],
    )
    .await;
    assert_ne!(kind, "error", "{body}");

    let completion = set_autonomy(&workspace, "autonomous").await;

    assert_eq!(completion.code(), 1, "{completion:?}");
    assert!(
        completion
            .diagnostic()
            .is_some_and(|line| line.starts_with("Refusing to switch Construction to autonomous:")),
        "{completion:?}"
    );
}

/// 降格は人間の turn を要さない（ゲートを戻すのに人手は要らない）。
#[tokio::test]
async fn a_de_escalation_needs_no_human_turn() {
    let workspace = minted().await;

    let completion = set_autonomy(&workspace, "gated").await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    assert_eq!(
        completion.line(),
        Some(r#"{"emitted":"AUTONOMY_MODE_SET","mode":"gated","state_updated":true}"#)
    );
    let audit = workspace.audit_shard().expect("監査シャードは在る");
    assert!(audit.contains("**Mode**: gated\n"), "{audit}");
}

/// 受理集合は状態に依らない — park 中でも切替は通り、park マーカーは残る（裁定 A）。
#[tokio::test]
async fn the_switch_is_accepted_while_the_workflow_is_parked() {
    let workspace = minted().await;
    let parked = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;
    assert_eq!(parked.code(), 0, "{parked:?}");
    workspace.append_human_turn(FRESH_TURN);

    let completion = set_autonomy(&workspace, "autonomous").await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    let state = workspace.state_file().expect("状態ファイルは在る");
    assert!(
        state.contains("- **Construction Autonomy Mode**: autonomous\n"),
        "{state}"
    );
    assert!(state.contains("- **Parked**: "), "{state}");
    assert!(
        state.contains("- **Parked At Stage**: domain-design"),
        "{state}"
    );
}

/// `--mode` の 3 形の拒否（欠落・閉集合外・値なし）は逐語である。
#[tokio::test]
async fn the_mode_flag_refusals_are_verbatim() {
    let workspace = minted().await;

    let missing = bolt_verb(&workspace, &["set-autonomy"]).await;
    assert_eq!(missing.code(), 1, "{missing:?}");
    assert_eq!(
        missing.diagnostic(),
        Some("Missing --mode <autonomous|gated>")
    );

    let invalid = set_autonomy(&workspace, "turbo").await;
    assert_eq!(invalid.code(), 1, "{invalid:?}");
    assert_eq!(
        invalid.diagnostic(),
        Some("Invalid --mode: turbo. Must be 'autonomous' or 'gated'.")
    );

    let no_value = bolt_verb(&workspace, &["set-autonomy", "--mode"]).await;
    assert_eq!(no_value.code(), 1, "{no_value:?}");
    assert_eq!(
        no_value.diagnostic(),
        Some("--mode expects a value, got end of arguments.")
    );
}

/// 状態ファイルの欄が消えていれば `setFieldStrict` の逐語で断る（逸脱台帳 #2 の M12）。
#[tokio::test]
async fn a_state_file_without_the_autonomy_field_is_refused_verbatim() {
    let workspace = minted().await;
    workspace.append_human_turn(FRESH_TURN);
    workspace.strip_state_line("- **Construction Autonomy Mode**:");

    let completion = set_autonomy(&workspace, "autonomous").await;

    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(
        completion.diagnostic(),
        Some(
            "State update failed: Field not found in state file: \"Construction Autonomy Mode\". Cannot update — refusing to silently no-op."
        )
    );
}

/// 鋳造前の切替はアクティブ intent が無いので断られる（own wording）。
#[tokio::test]
async fn switching_before_any_intent_exists_is_refused() {
    let workspace = Workspace::create();

    let completion = set_autonomy(&workspace, "gated").await;

    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(
        completion.diagnostic(),
        Some("Cannot resolve the active intent for the autonomy switch.")
    );
}

/// Bolt 面は未知動詞と未配線の 7 動詞を stderr で断る。
#[tokio::test]
async fn the_bolt_face_refuses_unknown_and_unwired_verbs_on_stderr() {
    let workspace = minted().await;

    let unknown = bolt_verb(&workspace, &["frobnicate"]).await;
    assert_eq!(unknown.code(), 1, "{unknown:?}");
    assert_eq!(
        unknown.diagnostic(),
        Some(
            "Unknown subcommand: frobnicate. Valid: start, complete, fail, abort, set-autonomy, dispatch-event, hold-merge, release-merge"
        )
    );

    let not_wired = bolt_verb(&workspace, &["start", "--name", "b1", "--batch", "1"]).await;
    assert_eq!(not_wired.code(), 1, "{not_wired:?}");
    assert_eq!(
        not_wired.diagnostic(),
        Some(
            "Cannot run aidlc-bolt start: the start subcommand is not wired in this build. Only `set-autonomy` is available."
        )
    );
}

/// 集約の拒否**以外**の失敗は own wording の中継形で材料を運ぶ。
#[tokio::test]
async fn a_switch_against_an_absent_execution_relays_the_repository_failure() {
    let workspace = minted().await;
    workspace.append_human_turn(FRESH_TURN);
    let record = workspace.record_dir().expect("record");
    let cursor = fs::read_to_string(record.join(".aidlc-execution")).expect("カーソルは読める");
    let intent_line = cursor.lines().nth(1).expect("2 行目は intent id");
    fs::write(
        record.join(".aidlc-execution"),
        format!("{ABSENT_EXECUTION}\n{intent_line}\n"),
    )
    .expect("別の実行を指す");

    let completion = set_autonomy(&workspace, "autonomous").await;

    assert_eq!(completion.code(), 1, "{completion:?}");
    assert!(
        completion
            .diagnostic()
            .is_some_and(|line| line.starts_with("Failed to switch autonomy: repository: ")),
        "{completion:?}"
    );
}

/// 実行カーソルが壊れているのは「不在」と混ぜない（`report` 段 6 と同じ規律）。
#[tokio::test]
async fn switching_against_a_broken_execution_cursor_is_refused() {
    let workspace = minted().await;
    let record = workspace.record_dir().expect("record");
    fs::write(record.join(".aidlc-execution"), "not-an-id\nalso-not\n").expect("カーソルを壊す");

    let completion = set_autonomy(&workspace, "gated").await;

    assert_eq!(completion.code(), 1, "{completion:?}");
    assert!(
        completion
            .diagnostic()
            .is_some_and(|line| line.starts_with("The execution cursor cannot be read")),
        "{completion:?}"
    );
}

/// 状態ファイルが読めなければ、その所在を名指して断る（欄の不在と混ぜない）。
#[tokio::test]
async fn switching_names_the_unreadable_state_file_instead_of_an_absent_field() {
    let workspace = minted().await;
    workspace.append_human_turn(FRESH_TURN);
    let state = workspace
        .record_dir()
        .expect("record")
        .join("aidlc-state.md");
    let before = workspace.journal_rows();
    fs::remove_file(&state).expect("状態ファイル");
    fs::create_dir(&state).expect("塞ぐ");

    let completion = set_autonomy(&workspace, "autonomous").await;

    assert_refused_at(
        &completion,
        "aidlc-orchestrate: projection restoration: publication conflict: ",
        &state,
    );
    assert_eq!(workspace.journal_rows(), before);
    assert!(state.is_dir());
}

/// 事前復旧が回らなければ、自律モードを切り替える前に拒否する。
#[tokio::test]
async fn a_restoration_failure_prevents_the_autonomy_switch_from_being_recorded() {
    let workspace = minted().await;
    workspace.append_human_turn(FRESH_TURN);
    // clone id の置き場をディレクトリで塞ぐと、投影のシャード名が解決できなくなる。
    let before = workspace.journal_rows();
    let clone_id = workspace.path("aidlc/.aidlc-clone-id");
    fs::remove_file(&clone_id).expect("clone id");
    fs::create_dir(&clone_id).expect("塞ぐ");

    let completion = set_autonomy(&workspace, "autonomous").await;

    assert_eq!(completion.code(), 1, "{completion:?}");
    assert_eq!(completion.line(), None, "stdout には何も出さない");
    assert!(
        completion
            .diagnostic()
            .unwrap_or_default()
            .starts_with("aidlc-orchestrate: clone id:"),
        "{completion:?}"
    );
    assert_eq!(
        workspace.journal_rows(),
        before,
        "事前復旧の失敗ではコミットしない"
    );
}

/// 空間名として成立しないカーソルは既定へ落とさず断る（指定と違う置き場へ書かない）。
#[tokio::test]
async fn switching_under_an_invalid_active_space_is_refused_by_name() {
    let workspace = minted().await;
    fs::write(workspace.path("aidlc/active-space"), "../escape\n").expect("空間カーソル");

    let completion = set_autonomy(&workspace, "gated").await;

    assert_eq!(completion.code(), 1, "{completion:?}");
    assert!(
        completion
            .diagnostic()
            .is_some_and(|line| line.contains("../escape")),
        "{completion:?}"
    );
}
