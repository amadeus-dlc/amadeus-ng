//! クエリ側の縦切り結合テスト — ディスク上の実バイトから `next` / `continue` の連鎖まで。
//!
//! 単体テストはビューを直接組んで分岐を押さえるが、ここは**実 DAO 3 本が噛み合うか**を見る:
//! `WorkflowDefinitionDaoImpl` が 3 入力を、`ExecutionStateDaoImpl` が `aidlc-state.md` を、
//! `MemoryRulesDaoImpl` が memory 層を実ファイルから読み、ユースケースが directive を出し、
//! continue_token が封緘 → 開封を往復して次の部が届く。
//!
//! b26 まではここで読取結果を値として組み立てていたが、2026-08-31 のオーナー裁定でユースケースが
//! リードモデルを**ポート経由で**読むようになったため、フェイクを挟まず一時ディレクトリの
//! 実ファイルを通す真の縦切りになった。
// 統合テストは別クレートなので `clippy.toml` の allow-*-in-tests が届かない。expect
// (フィクスチャ組み立ての失敗は即時失敗させる) と panic (想定外バリアントの検証) は
// 同クレートの他の統合テストと同じ理由で file 単位に allow する。
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use core_query_interface_adapter::{
    DefinitionPaths, ExecutionStateDaoImpl, MemoryRulesDaoImpl, WorkflowDefinitionDaoImpl,
    mint_continue_token, verify_continue_token,
};
use core_query_use_case::orchestration::{
    ContinueUseCase, Directive, ExecutionStateDao, GateField, LoadSteeringDirective, NextTurnInput,
    NextUseCase, WorkspaceLayout,
};

/// 封緘鍵 (マシンローカル鍵の代わり — 合成ルートが用意するバイト列)。
const KEY: &[u8] = &[7u8; 32];

/// ワークスペース一式 (定義 3 入力・record・memory 層) を書いた一時ディレクトリ。
struct Workspace {
    root: tempfile::TempDir,
}

impl Workspace {
    /// 3 ステージの定義と memory 層 2 ファイルを書く (状態ファイルは呼び手が置く)。
    fn create() -> Workspace {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let workspace = Workspace { root };
        workspace.write_definition();
        workspace.write_memory_layer();
        fs::create_dir_all(workspace.record_dir()).expect("record ディレクトリ");
        workspace
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn record_dir(&self) -> PathBuf {
        self.path("record")
    }

    fn memory_dir(&self) -> PathBuf {
        self.path("aidlc/spaces/default/memory")
    }

    /// 3 ステージのワークフロー定義リードモデルをディスクへ書く。
    fn write_definition(&self) {
        let data = self.path("tools/data");
        let scopes = self.path("scopes");
        fs::create_dir_all(&data).expect("data ディレクトリ");
        fs::create_dir_all(&scopes).expect("scopes ディレクトリ");
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

    /// 2 部に割れる大きさの memory 層 (org.md + team.md、いずれも 12KiB の 1 セクション)。
    ///
    /// project.md は置かない — 欠落が正常スキップされることも同時に固定する。
    fn write_memory_layer(&self) {
        let memory = self.memory_dir();
        fs::create_dir_all(memory.join("phases")).expect("memory ディレクトリ");
        let big = "x".repeat(12 * 1024);
        fs::write(memory.join("org.md"), format!("# Org\n{big}\n")).expect("org.md");
        fs::write(memory.join("team.md"), format!("# Team\n{big}\n")).expect("team.md");
    }

    fn write_state(&self, text: &str) {
        fs::write(self.record_dir().join("aidlc-state.md"), text).expect("状態ファイル");
    }

    fn definition_dao(&self) -> WorkflowDefinitionDaoImpl {
        WorkflowDefinitionDaoImpl::new(DefinitionPaths::new(
            self.path("tools/data"),
            self.path("scopes"),
        ))
    }

    fn state_dao(&self) -> ExecutionStateDaoImpl {
        ExecutionStateDaoImpl::new(self.record_dir())
    }

    fn rules_dao(&self) -> MemoryRulesDaoImpl {
        MemoryRulesDaoImpl::new(self.memory_dir())
    }

    fn next(
        &self,
    ) -> NextUseCase<WorkflowDefinitionDaoImpl, ExecutionStateDaoImpl, MemoryRulesDaoImpl> {
        NextUseCase::new(self.definition_dao(), self.state_dao(), self.rules_dao())
    }

    fn continuing(
        &self,
    ) -> ContinueUseCase<WorkflowDefinitionDaoImpl, ExecutionStateDaoImpl, MemoryRulesDaoImpl> {
        ContinueUseCase::new(self.definition_dao(), self.state_dao(), self.rules_dao())
    }
}

/// 出荷テンプレートと同じ節構成の状態ファイル (カーソルは inception の `domain-design`)。
fn state_file() -> String {
    [
        "# AI-DLC State Tracking",
        "",
        "## Project Information",
        "- **Scope**: classic",
        "- **State Version**: 8",
        "",
        "## Stage Progress",
        "",
        "### INITIALIZATION PHASE",
        "- [x] state-init — EXECUTE",
        "",
        "### INCEPTION PHASE",
        "- [-] domain-design — EXECUTE",
        "- [ ] contract-design — EXECUTE",
        "",
        "## Current Status",
        "- **Current Stage**: domain-design",
        "- **Status**: Running",
        "- **Last Updated**: 2026-08-29T16:36:24Z",
        "",
    ]
    .join("\n")
}

fn layout() -> WorkspaceLayout {
    WorkspaceLayout::new(
        "aidlc/spaces/default/intents/demo".to_string(),
        ".claude/aidlc-common/stages".to_string(),
        ".claude/agents".to_string(),
    )
}

fn expect_load_steering(directive: Directive) -> LoadSteeringDirective {
    match directive {
        Directive::LoadSteering(part) => part,
        other => panic!("load-steering を期待したが {:?}", other.kind()),
    }
}

/// ルール台帳のパスを memory ディレクトリからの相対に畳む (前置は一時ディレクトリなので)。
fn relative_to(paths: &[String], memory_dir: &Path) -> Vec<String> {
    let prefix = format!("{}/", memory_dir.display());
    paths
        .iter()
        .map(|path| path.strip_prefix(&prefix).unwrap_or(path).to_string())
        .collect()
}

/// `next` → (封緘 → 開封) → `continue` ×2 で run-stage まで到達する縦切り。
#[test]
fn the_chain_walks_from_the_files_to_the_run_stage_through_sealed_tokens() {
    let workspace = Workspace::create();
    workspace.write_state(&state_file());

    let state = workspace
        .state_dao()
        .find()
        .expect("状態ファイルの読取")
        .expect("状態ファイルは書いたので読めるはず");
    assert_eq!(state.scope().as_str(), "classic");
    assert_eq!(state.cursor().to_usize(), 1, "カーソルは domain-design");

    let input = NextTurnInput::new().with_layout(layout());

    // 第 1 部 — カーソルのステージについて load-steering が出る。
    let part1 = expect_load_steering(workspace.next().execute(&input));
    assert_eq!(part1.stage().as_str(), "domain-design");
    assert_eq!(part1.part().as_u32(), 1);
    assert_eq!(part1.parts().as_u32(), 2, "org.md + team.md で 2 部");

    // トークンは封緘 → 開封を往復しても同じ中身である (プロセスをまたいだ継続の本体)。
    let sealed = mint_continue_token(KEY, part1.continue_token());
    let opened = verify_continue_token(KEY, &sealed).expect("同じ鍵なら開封できる");
    assert_eq!(&opened, part1.continue_token(), "封緘の往復は無損失");
    assert!(
        opened.bindings().state().is_some(),
        "state ありの連鎖は state 束縛を運ぶ"
    );

    // 第 2 部。
    let part2 = expect_load_steering(workspace.continuing().execute(Some(opened), &input));
    assert_eq!(part2.part().as_u32(), 2);

    // 終端 — run-stage がルール台帳つきで届く。
    let sealed = mint_continue_token(KEY, part2.continue_token());
    let opened = verify_continue_token(KEY, &sealed).expect("同じ鍵なら開封できる");
    let directive = workspace.continuing().execute(Some(opened), &input);
    let Directive::RunStage(run_stage) = directive else {
        panic!("run-stage を期待した")
    };
    assert_eq!(run_stage.stage().as_str(), "domain-design");
    assert_eq!(
        run_stage.gate(),
        GateField::Gated,
        "inception はゲート付き (BR1.3)"
    );
    assert_eq!(
        relative_to(run_stage.rules_in_context(), &workspace.memory_dir()),
        ["org.md", "team.md"],
        "配信済みルールのパス台帳 — 解決順どおりで、無い project.md は現れない"
    );
    assert_eq!(
        run_stage.next_stage(),
        Some("Contract Design"),
        "次の in-scope EXECUTE ステージの表示名"
    );
    assert_eq!(
        run_stage.stage_file(),
        ".claude/aidlc-common/stages/inception/domain-design.md"
    );
    assert_eq!(
        run_stage.memory_path(),
        "aidlc/spaces/default/intents/demo/inception/domain-design/memory.md"
    );
}

/// 状態ファイルが無いターンは誕生分岐へ落ちる (読取失敗ではない)。
#[test]
fn a_record_without_a_state_file_falls_through_to_the_birth_group() {
    let workspace = Workspace::create();
    assert_eq!(
        workspace.state_dao().find().expect("不在は失敗ではない"),
        None
    );
    let directive = workspace.next().execute(
        &NextTurnInput::new()
            .with_layout(layout())
            .with_scope("classic"),
    );
    let Directive::Print { message } = directive else {
        panic!("print を期待した")
    };
    assert!(
        message.contains("aidlc-utility intent-create --scope classic"),
        "{message}"
    );
}

/// 状態が動いたあとの継続は fail-closed で止まる (state 束縛の照合)。
///
/// 同じ DAO を作り直して読むので、**ディスクの更新がそのまま継続の入力になる** — b26 まで
/// テスト側で 2 つのビューを持ち替えていた部分が、実際の再読取に置き換わった。
#[test]
fn a_continuation_after_the_state_moved_on_fails_closed() {
    let workspace = Workspace::create();
    workspace.write_state(&state_file());
    let before = workspace
        .state_dao()
        .find()
        .expect("状態ファイルの読取")
        .expect("読めるはず");
    let input = NextTurnInput::new().with_layout(layout());
    let part1 = expect_load_steering(workspace.next().execute(&input));

    // 連鎖の途中でカーソルが進んだ (投影がリードモデルを更新した)。
    workspace.write_state(
        &state_file()
            .replace(
                "- [-] domain-design — EXECUTE",
                "- [x] domain-design — EXECUTE",
            )
            .replace("- [ ] contract-design", "- [-] contract-design")
            .replace(
                "- **Current Stage**: domain-design",
                "- **Current Stage**: contract-design",
            ),
    );
    let after = workspace
        .state_dao()
        .find()
        .expect("更新後も読める")
        .expect("読めるはず");
    assert_ne!(
        before.state_binding(),
        after.state_binding(),
        "state が動けば束縛も動く"
    );

    let sealed = mint_continue_token(KEY, part1.continue_token());
    let opened = verify_continue_token(KEY, &sealed).expect("開封はできる");
    let directive = workspace.continuing().execute(Some(opened), &input);
    assert_eq!(
        directive,
        Directive::Error {
            message: "The saved position moved on: the workflow state changed while this stage's \
                      rules were being loaded. Run a fresh `next` to restart delivery from part 1."
                .to_string()
        }
    );
}

/// 必須ルールファイルが在るのに読めないターンは、ステージを始めずに逐語で止まる。
#[test]
fn an_unreadable_rule_file_blocks_the_stage_verbatim() {
    let workspace = Workspace::create();
    workspace.write_state(&state_file());
    // org.md の位置をディレクトリに差し替える — read_to_string は EISDIR で失敗する。
    let org = workspace.memory_dir().join("org.md");
    fs::remove_file(&org).expect("org.md の除去");
    fs::create_dir(&org).expect("org.md をディレクトリに差し替え");

    let directive = workspace
        .next()
        .execute(&NextTurnInput::new().with_layout(layout()));
    let Directive::Error { message } = directive else {
        panic!("error を期待した")
    };
    assert!(
        message.starts_with(&format!(
            "Cannot load required stage rule \"{}\" (",
            org.display()
        )),
        "{message}"
    );
    assert!(
        message.ends_with(
            "The stage has not started. Restore the file or fix its permissions/UTF-8 encoding, then run `next` again."
        ),
        "{message}"
    );
}

/// 定義 3 入力が読めないターンは 12 §4 の逐語文言で止まる (文言の所有はユースケース側)。
#[test]
fn an_unreadable_stage_graph_stops_the_turn_with_the_upstream_wording() {
    let workspace = Workspace::create();
    workspace.write_state(&state_file());
    let graph = workspace.path("tools/data/stage-graph.json");
    fs::remove_file(&graph).expect("stage-graph.json の除去");

    let directive = workspace
        .next()
        .execute(&NextTurnInput::new().with_layout(layout()));
    let Directive::Error { message } = directive else {
        panic!("error を期待した")
    };
    assert!(
        message.starts_with(&format!(
            "Stage graph not readable at {}: ",
            graph.display()
        )),
        "{message}"
    );
    assert!(
        message.ends_with("Reinstall the framework or re-run setup to restore the data file."),
        "{message}"
    );
}

/// memory 層が空でも正常 — ルール未整備は bare run-stage になる。
#[test]
fn an_empty_memory_layer_delivers_a_bare_run_stage() {
    let workspace = Workspace::create();
    workspace.write_state(&state_file());
    fs::remove_file(workspace.memory_dir().join("org.md")).expect("org.md の除去");
    fs::remove_file(workspace.memory_dir().join("team.md")).expect("team.md の除去");

    let directive = workspace
        .next()
        .execute(&NextTurnInput::new().with_layout(layout()));
    let Directive::RunStage(run_stage) = directive else {
        panic!("run-stage を期待した")
    };
    assert_eq!(run_stage.stage().as_str(), "domain-design");
    assert!(
        run_stage.rules_in_context().is_empty(),
        "配信するルールが無い"
    );
}
