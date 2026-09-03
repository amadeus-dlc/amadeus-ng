//! CLI ゴールデン — 採取済みの `next` / `continue` の実行出力と突き合わせる。
//!
//! 入力は `tests/golden/upstream-3c3146cf/cli/{next,continue}/` — ピン留めコミット
//! `3c3146cf` (v2.6.40) の配布シェルを bun で実行して採った stdout である
//! (`scripts/goldens/recapture-cli.sh`)。ゴールデンの更新は**upstream ピン更新の intent で
//! のみ**行う (BR2.5) ので、ここは読むだけである。
//!
//! # 何をバイトで固定できるか
//!
//! | ケース | 突き合わせ | 理由 |
//! | --- | --- | --- |
//! | `continue/invalid-token` | **stdout をバイト一致**で固定 | 逐語文言だけの directive で、フィクスチャにも配置にも依らない |
//! | `next/start` (`load-steering`) | **キー集合**を固定 | 中身 (`rules_content` / `bundle`) は採取時のワークスペースの memory 層に依存する |
//! | `continue/load-steering` (終端 `run-stage`) | **キー集合**を固定 | 同上 (パスは配置に依存する) |
//!
//! # 駆動できないケース (黙って飛ばさない)
//!
//! - **`next/no-active-intent`** — fresh なワークスペースでは構造化リードモデルに定義の行が
//!   まだ無い (定義をジャーナルへ入れるのは `intent-create` の前段だけで、`next` は読むだけ
//!   なので取り込めない)。state なしの誕生分岐そのものが新経路で成立しないため、駆動しても
//!   比較にならない。RMU が配布束を参照入力として投影するようになったら再訪する。
//! - **`next/stage-jump-print`** — **逸脱台帳 #1**。upstream は
//!   `bun .claude/tools/aidlc-jump.ts execute --target <slug> --direction forward --scope <scope>`
//!   を名指すが、こちらはマルチコール正準形 (`aidlc-jump resolve --stage <slug>`) を名指す。
//!   バイト一致は設計上ありえない。
//! - **`next/after-approval`** — 採取時のワークスペース (upstream 配布の 11 個の scope identity
//!   ファイルとその metadata) が vendored されていないため、同じ計画を再現できない。
//!   `stage-graph.json` / `scope-grid.json` / `harness.json` は在るが、有効 scope の権威は
//!   `.claude/scopes/aidlc-<name>.md` であり (12 §4 #6)、それが無い。
//!
//! # 既知の欠落 (キー集合の差はここで固定する)
//!
//! upstream の `run-stage` は `conductor_persona` と `narration` を載せるが、こちらは
//! どちらも載せない (b44 以前からの欠落 — `RunStageDirective` は `narration` の欄を持つが
//! 設定する経路が無く、`conductor_persona` は欄すら無い)。**差を明示的に固定**しておく
//! ことで、別のキーが黙って落ちたらここが赤くなる。
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use core_infrastructure::canon_json::{JsonValue, parse};

/// 採取済みケースの置き場 (リポジトリ根からの相対)。
fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("tests/golden/upstream-3c3146cf/cli")
}

/// 採取済みケースの stdout (1 行 JSON) をそのまま読む。
fn recorded(case: &str) -> String {
    let path = golden_dir().join(case).join("stdout.json");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("ゴールデン {} が読めない: {error}", path.display()))
        .trim_end_matches('\n')
        .to_string()
}

/// オブジェクトのキー集合 (順序は upstream の挿入順であって契約ではない — §キー順)。
fn keys(line: &str) -> BTreeSet<String> {
    match parse(line).expect("ゴールデンは JSON") {
        JsonValue::Object(members) => members.iter().map(|(key, _)| key.to_string()).collect(),
        other => panic!("オブジェクトであるべき: {other:?}"),
    }
}

/// 定義 3 入力と memory 層を書いた fresh なワークスペース。
///
/// キー集合の比較は計画の大きさに依らないので、採取時の 33 ノードのグラフではなく最小の
/// 合成グラフでよい (upstream 配布の scope identity ファイルは vendored されていない —
/// モジュール doc の「駆動できないケース」を参照)。
struct Workspace {
    root: tempfile::TempDir,
}

impl Workspace {
    fn create() -> Workspace {
        let workspace = Workspace {
            root: tempfile::tempdir().expect("一時ディレクトリ"),
        };
        workspace.write_definition();
        let memory = workspace.path("aidlc/spaces/default/memory");
        fs::create_dir_all(&memory).expect("memory");
        fs::write(
            memory.join("org.md"),
            "# Org\n\n## Way of Working\n\n規則。\n",
        )
        .expect("org.md");
        fs::create_dir_all(workspace.path("aidlc/spaces/default/intents")).expect("intents");
        workspace
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn project_dir(&self) -> &Path {
        self.root.path()
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
        // 採取済みの `run-stage` が載せるキーを全部出させるため、reviewer と support_agents を
        // 持つノードにする (任意キーは値が在るときだけ載る)。
        let node = |slug: &str, number: &str, name: &str, phase: &str, extra: &str| {
            format!(
                r#"{{"slug":"{slug}","number":"{number}","name":"{name}","phase":"{phase}",
                     "execution":"ALWAYS","mode":"subagent","lead_agent":"orchestrator",
                     "scopes":["classic"]{extra}}}"#
            )
        };
        let reviewed = r#","support_agents":["aidlc-quality-agent"],
             "reviewer":"aidlc-architecture-reviewer-agent","review_class":"advisory",
             "reviewer_max_iterations":2,"produces":["design.md"]"#;
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
                    reviewed
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
        fs::write(
            scopes.join("aidlc-classic.md"),
            "---\nname: classic\n---\n\n# Classic\n",
        )
        .expect("scope identity");
    }

    async fn invoke(&self, argv0: &str, args: &[&str]) -> aidlc::runtime::Completion {
        let mut owned: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
        owned.push("--project-dir".to_string());
        owned.push(self.project_dir().to_string_lossy().into_owned());
        aidlc::runtime::run(argv0, &owned, self.project_dir()).await
    }

    async fn mint(&self) {
        let completion = self
            .invoke(
                "aidlc-utility",
                &["intent-create", "--scope", "classic", "--label", "golden"],
            )
            .await;
        assert_eq!(completion.code(), 0, "鋳造は通る: {completion:?}");
    }
}

/// 出た 1 行を取り出す。
fn line(completion: &aidlc::runtime::Completion) -> String {
    completion
        .line()
        .unwrap_or_else(|| panic!("stdout に 1 行が要る: {completion:?}"))
        .to_string()
}

/// **バイト一致** — 逐語文言だけの directive はフィクスチャに依らない。
#[tokio::test]
async fn the_invalid_continuation_token_is_byte_identical_to_the_recorded_case() {
    let workspace = Workspace::create();
    workspace.mint().await;

    let completion = workspace
        .invoke(
            "aidlc-orchestrate",
            &["continue", "not-a-continuation-token"],
        )
        .await;

    assert_eq!(completion.code(), 0, "ビジネス拒否は exit 0");
    assert_eq!(
        line(&completion),
        recorded("continue/invalid-token"),
        "採取済みの stdout と 1 バイトも違わない"
    );
}

/// `load-steering` のキー集合が採取済みケースと一致する。
#[tokio::test]
async fn the_load_steering_keys_match_the_recorded_case() {
    let workspace = Workspace::create();
    workspace.mint().await;

    let completion = workspace.invoke("aidlc-orchestrate", &["next"]).await;

    let emitted = keys(&line(&completion));
    let expected = keys(&recorded("next/start"));
    assert_eq!(
        emitted, expected,
        "load-steering のキー集合は採取済みケースと同じである"
    );
}

/// 終端 `run-stage` のキー集合 — 欠落は既知の 2 つ (`conductor_persona` / `narration`) だけ。
///
/// 採取済みの `continue/load-steering` は「最後の部まで配り終えた継続」なので、返るのは
/// 続きの部ではなく台帳付きの `run-stage` である。こちらも同じ形で終端に着く。
#[tokio::test]
async fn the_terminal_run_stage_keys_match_the_recorded_case_except_the_known_gap() {
    let workspace = Workspace::create();
    workspace.mint().await;

    // 1 部で収まる規則束なので、最初の `next` が第 1 部を配って終端に達する。
    let first = workspace.invoke("aidlc-orchestrate", &["next"]).await;
    let token = match parse(&line(&first)).expect("JSON") {
        JsonValue::Object(members) => match members.get("continue_token") {
            Some(JsonValue::String(token)) => token.clone(),
            other => panic!("continue_token は文字列であるべき: {other:?}"),
        },
        other => panic!("オブジェクトであるべき: {other:?}"),
    };
    let completion = workspace
        .invoke("aidlc-orchestrate", &["continue", &token])
        .await;

    let emitted = keys(&line(&completion));
    let expected = keys(&recorded("continue/load-steering"));
    let missing: Vec<&String> = expected.difference(&emitted).collect();
    assert_eq!(
        missing,
        vec![&"conductor_persona".to_string(), &"narration".to_string()],
        "採取済みケースが載せるキーのうち、こちらが載せないのは既知の 2 つだけである"
    );
    // 採取済みのステージ (`practices-discovery`) はレビュアを宣言しないので、任意の 3 キーが
    // 現れない。こちらのフィクスチャは宣言するので現れる — その 3 つ以外は増やさない。
    let extra: Vec<&String> = emitted.difference(&expected).collect();
    assert_eq!(
        extra,
        vec![
            &"review_class".to_string(),
            &"reviewer".to_string(),
            &"reviewer_max_iterations".to_string(),
        ],
        "採取済みケースに無いキーは、レビュア宣言に応じた任意の 3 つだけである"
    );
}
