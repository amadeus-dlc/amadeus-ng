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

/// park は**ビジネス拒否**として stdout に出る（未配線でも自己防衛拒否ではない）。
#[tokio::test]
async fn park_is_refused_as_a_business_error_on_stdout() {
    let workspace = Workspace::create();

    let completion = invoke(&workspace, "aidlc-orchestrate", &["park"]).await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    let directive = line_of(&completion);
    assert_eq!(string_of(&directive, "kind"), "error");
    assert_eq!(
        string_of(&directive, "message"),
        "Cannot park the workflow: park is not wired in this build."
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
