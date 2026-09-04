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
use core_command_domain::workspace::{SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, IntentRepositoryImpl,
};
use core_command_use_case::orchestration::{IntentExecutionRepository as _, IntentRepository as _};
use core_infrastructure::canon_json::{JsonValue, parse};

struct Workspace {
    root: tempfile::TempDir,
}

impl Workspace {
    /// 定義 3 入力と memory 層を書き、ストアの置き場だけ用意した fresh なワークスペース。
    ///
    /// **カーソルも record も置かない** — intent がまだ生まれていない状態から始める。
    fn create() -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_definition();
        let memory = workspace.path("aidlc/spaces/default/memory");
        fs::create_dir_all(&memory).expect("memory");
        fs::write(memory.join("org.md"), "# Org\n\n規則なし。\n").expect("org.md");
        // ストアの親 (`intents/`) は upstream の既存ディレクトリ扱いなので先に作る。
        fs::create_dir_all(workspace.path("aidlc/spaces/default/intents")).expect("intents");
        workspace
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
            .switch_autonomy(&intent, AutonomyMode::Autonomous, Utc::now())
            .expect("Running な実行はモードを切り替えられる");
        execution_repository
            .store(&event, &aggregate)
            .await
            .expect("切替は書ける");
    }

    fn write_definition(&self) {
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
                "[{},{},{}]",
                node("state-init", "0.1", "State Init", "initialization"),
                node("domain-design", "1.1", "Domain Design", "inception"),
                node("contract-design", "1.2", "Contract Design", "inception"),
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
        &["report", "--result", "completed"],
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
    for _ in 0..2 {
        let completion = invoke(
            &workspace,
            "aidlc-orchestrate",
            &["report", "--result", "completed"],
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

/// `report` は `--result` が要る（ビジネス拒否）。
#[tokio::test]
async fn reporting_without_a_result_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(&workspace, "aidlc-orchestrate", &["report"]).await;

    assert_eq!(completion.code(), 0);
    assert_eq!(
        string_of(&line_of(&completion), "message"),
        "report requires --result <outcome>."
    );
}

/// resume 系の結末は遷移ではない — コミットせずに経路を返す。
#[tokio::test]
async fn a_resume_result_is_routed_rather_than_committed() {
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
        "Resume is routed, not committed. Run a fresh `next --resume`."
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
        "No workflow execution to report against. Run `next` first."
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
        "The --stage value is not a stage slug."
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
            "--stage",
            "contract-design",
        ],
    )
    .await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let message = string_of(&line_of(&completion), "message");
    assert!(message.starts_with("Transition rejected: "), "{message}");
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

    for outcome in ["awaiting-approval", "rejected", "revised", "approved"] {
        let next = invoke(&workspace, "aidlc-orchestrate", &["next"]).await;
        assert_eq!(next.code(), 0, "{outcome} の直前の next: {next:?}");

        let completion = invoke(
            &workspace,
            "aidlc-orchestrate",
            &["report", "--result", outcome],
        )
        .await;

        assert_eq!(
            completion.code(),
            0,
            "{outcome} は受理される: {completion:?}"
        );
        let directive = line_of(&completion);
        assert_eq!(string_of(&directive, "kind"), "done");
        assert_eq!(
            string_of(&directive, "reason"),
            format!("reported {outcome}")
        );
    }

    let state = workspace.state_file().expect("骨格");
    assert!(
        state.contains("contract-design"),
        "承認でカーソルが次のステージへ進む: {state}"
    );
}

/// 計画が EXECUTE と言っているステージは飛ばせない — 集約の拒否を逐語で中継する。
#[tokio::test]
async fn skipping_a_stage_the_plan_marks_execute_is_rejected() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo"],
    )
    .await;
    invoke(&workspace, "aidlc-orchestrate", &["next"]).await;

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "skipped", "--reason", "out of scope"],
    )
    .await;

    assert_eq!(
        completion.code(),
        0,
        "ビジネス拒否は exit 0: {completion:?}"
    );
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "error");
    assert_eq!(
        string_of(&directive, "message"),
        "Transition rejected: command: stage 1 is not skippable"
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
