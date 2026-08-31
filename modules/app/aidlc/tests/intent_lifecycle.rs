//! 合成ルートの縦切り — 鋳造から報告まで（コマンド側 → RMU → クエリ側の一巡）。
//!
//! `intent-create` が 2 つの集約をジャーナルへ書き、RMU がそれを `aidlc-state.md` と
//! 監査シャードへ投影し、`next` がその投影を読んで directive を出し、`report` が遷移を
//! コミットして再び投影される。**両側と中間が実際に噛み合うか**を見る唯一のテストである。
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

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

    fn audit_shard(&self) -> Option<String> {
        let audit = self.record_dir()?.join("audit");
        let entry = fs::read_dir(audit).ok()?.filter_map(Result::ok).next()?;
        fs::read_to_string(entry.path()).ok()
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
    let created = line_of(&completion);
    assert!(string_of(&created, "record").ends_with(&format!(
        "-{}",
        &string_of(&created, "record")
            .rsplit('-')
            .next()
            .unwrap_or_default()
    )));
    assert!(string_of(&created, "record").contains("-demo-run-"));

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
/// # ここが写している既知の乖離（b29 で発見・未修正）
///
/// 誕生の**投影**は初期化ステージを `[x]` にして最初のゲート付きステージへカーソルを
/// 進めるが、**集約はそれをしていない** — `IntentExecution::start` が出すのは `Started`
/// 1 本だけで、カーソルは 0（`state-init`）のままである。つまり読み面は書き面より
/// 先へ進んでいる。
///
/// その結果、`report --result completed` は集約のカーソル（`state-init`）を完了させ、
/// 監査シャードには `STAGE_COMPLETED` が**2 度**現れる（誕生の投影で 1 度、この報告で
/// もう 1 度）。監査証跡は第一級の成果物なので、この重複は是正されるべきである。
///
/// **本テストは現状を逐語で固定している** — 是正が入ればここが落ちるので、乖離が
/// 静かに残ることはない。
#[tokio::test]
async fn reporting_a_verdict_commits_and_projects() {
    let workspace = Workspace::create();
    invoke(
        &workspace,
        "aidlc-utility",
        &["intent-create", "--scope", "classic", "--label", "demo run"],
    )
    .await;
    // 誕生の投影で initialization は完了済み、最初のゲート付きステージが in-flight。
    let before = workspace.state_file().expect("投影済み");
    assert!(before.contains("- [x] state-init"), "{before}");
    assert!(before.contains("- [-] domain-design"), "{before}");
    let before_completions = workspace
        .audit_shard()
        .expect("監査シャード")
        .matches("STAGE_COMPLETED")
        .count();

    let completion = invoke(
        &workspace,
        "aidlc-orchestrate",
        &["report", "--result", "completed"],
    )
    .await;

    assert_eq!(completion.code(), 0, "{completion:?}");
    assert_eq!(string_of(&line_of(&completion), "kind"), "done");
    // 遷移がコミットされ、投影が監査へ 1 行足した。
    let after_completions = workspace
        .audit_shard()
        .expect("監査シャード")
        .matches("STAGE_COMPLETED")
        .count();
    assert_eq!(
        after_completions,
        before_completions + 1,
        "報告が監査へ 1 行足す（乖離により対象は state-init である）"
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

/// `--scope` の無い鋳造は拒否される（bare invocation で既定の intent を作らない）。
#[tokio::test]
async fn creating_an_intent_without_a_scope_is_refused() {
    let workspace = Workspace::create();

    let completion = invoke(&workspace, "aidlc-utility", &["intent-create"]).await;

    assert_eq!(completion.code(), 1);
    assert_eq!(completion.line(), None);
    assert!(workspace.record_dir().is_none(), "何も作らない");
}
