//! クエリ側の縦切り結合テスト — 状態ファイルの実バイトから `next` / `continue` の連鎖まで。
//!
//! 単体テストはビューを直接組んで分岐を押さえるが、ここは**アダプタ層の 3 部品が噛み合うか**
//! を見る: reader が `aidlc-state.md` を読み、parse がクエリモデルへ写し、ユースケースが
//! directive を出し、continue_token が封緘 → 開封を往復して次の部が届く。
//!
//! ワークフロー定義は同じクレートの reader で読む — 3 入力 (`harness.json` /
//! `stage-graph.json` / `scope-grid.json`) と scope identity を一時ディレクトリに書き、
//! b25 の読取経路をそのまま通す。
// 統合テストは別クレートなので `clippy.toml` の allow-*-in-tests が届かない。expect
// (フィクスチャ組み立ての失敗は即時失敗させる) と panic (想定外バリアントの検証) は
// 同クレートの他の統合テストと同じ理由で file 単位に allow する。
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use core_query_interface_adapter::{
    DefinitionPaths, LoadedExecutionState, load_execution_state, load_workflow_definition,
    mint_continue_token, verify_continue_token,
};
use core_query_use_case::orchestration::{
    ContinueUseCase, DefinitionSource, Directive, ExecutionStateSource, GateField,
    LoadSteeringDirective, MemoryRules, NextTurnInput, NextUseCase, RuleContent, SteeringSource,
    WorkspaceLayout,
};
use core_query_use_case::workflow_view::DefinitionView;

/// 封緘鍵 (マシンローカル鍵の代わり — 合成ルートが用意するバイト列)。
const KEY: &[u8] = &[7u8; 32];

/// 2 部に割れる大きさのルール束 (読み終えた値 — ファイル I/O はテストの外)。
fn two_part_rules() -> MemoryRules {
    let big = "x".repeat(12 * 1024);
    MemoryRules::new(
        vec![
            RuleContent::new(
                "aidlc/spaces/default/memory/org.md".to_string(),
                format!("# Org\n{big}\n"),
            ),
            RuleContent::new(
                "aidlc/spaces/default/memory/team.md".to_string(),
                format!("# Team\n{big}\n"),
            ),
        ],
        std::collections::BTreeMap::new(),
    )
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

/// 3 ステージのワークフロー定義リードモデルをディスクへ書き、読み取ったビューを返す。
fn write_definition(root: &Path) -> DefinitionView {
    let data = root.join("tools/data");
    let scopes = root.join("scopes");
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
    load_workflow_definition(&DefinitionPaths::new(data, scopes)).expect("定義リードモデル")
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

/// `next` → (封緘 → 開封) → `continue` ×2 で run-stage まで到達する縦切り。
#[test]
fn the_chain_walks_from_the_state_file_to_the_run_stage_through_sealed_tokens() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let record = dir.path().join("record");
    fs::create_dir_all(&record).expect("record ディレクトリ");
    fs::write(record.join("aidlc-state.md"), state_file()).expect("状態ファイル");

    let definition = write_definition(dir.path());
    let LoadedExecutionState::Loaded(state) =
        load_execution_state(&record).expect("状態ファイルの読取")
    else {
        panic!("状態ファイルは書いたので読めるはず")
    };
    assert_eq!(state.scope().as_str(), "classic");
    assert_eq!(state.cursor().to_usize(), 1, "カーソルは domain-design");

    let input = NextTurnInput::new().with_layout(layout());
    let rules = two_part_rules();
    let steering = SteeringSource::Loaded(&rules);

    // 第 1 部 — カーソルのステージについて load-steering が出る。
    let part1 = expect_load_steering(NextUseCase::execute(
        ExecutionStateSource::Loaded(&state),
        DefinitionSource::Loaded(&definition),
        steering,
        &input,
    ));
    assert_eq!(part1.stage().as_str(), "domain-design");
    assert_eq!(part1.part().as_u32(), 1);
    assert_eq!(part1.parts().as_u32(), 2);

    // トークンは封緘 → 開封を往復しても同じ中身である (プロセスをまたいだ継続の本体)。
    let sealed = mint_continue_token(KEY, part1.continue_token());
    let opened = verify_continue_token(KEY, &sealed).expect("同じ鍵なら開封できる");
    assert_eq!(&opened, part1.continue_token(), "封緘の往復は無損失");
    assert!(
        opened.bindings().state().is_some(),
        "state ありの連鎖は state 束縛を運ぶ"
    );

    // 第 2 部。
    let part2 = expect_load_steering(ContinueUseCase::execute(
        Some(opened),
        ExecutionStateSource::Loaded(&state),
        DefinitionSource::Loaded(&definition),
        steering,
        &input,
    ));
    assert_eq!(part2.part().as_u32(), 2);

    // 終端 — run-stage がルール台帳つきで届く。
    let sealed = mint_continue_token(KEY, part2.continue_token());
    let opened = verify_continue_token(KEY, &sealed).expect("同じ鍵なら開封できる");
    let directive = ContinueUseCase::execute(
        Some(opened),
        ExecutionStateSource::Loaded(&state),
        DefinitionSource::Loaded(&definition),
        steering,
        &input,
    );
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
        run_stage.rules_in_context(),
        [
            "aidlc/spaces/default/memory/org.md",
            "aidlc/spaces/default/memory/team.md"
        ],
        "配信済みルールのパス台帳"
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
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let record = dir.path().join("record");
    fs::create_dir_all(&record).expect("record ディレクトリ");
    let definition = write_definition(dir.path());
    assert_eq!(
        load_execution_state(&record).expect("不在は失敗ではない"),
        LoadedExecutionState::Missing
    );
    let rules = two_part_rules();
    let directive = NextUseCase::execute(
        ExecutionStateSource::Missing,
        DefinitionSource::Loaded(&definition),
        SteeringSource::Loaded(&rules),
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
#[test]
fn a_continuation_after_the_state_moved_on_fails_closed() {
    let dir = tempfile::tempdir().expect("一時ディレクトリ");
    let record = dir.path().join("record");
    fs::create_dir_all(&record).expect("record ディレクトリ");
    fs::write(record.join("aidlc-state.md"), state_file()).expect("状態ファイル");
    let definition = write_definition(dir.path());
    let LoadedExecutionState::Loaded(before) =
        load_execution_state(&record).expect("状態ファイルの読取")
    else {
        panic!("読めるはず")
    };
    let input = NextTurnInput::new().with_layout(layout());
    let rules = two_part_rules();
    let steering = SteeringSource::Loaded(&rules);
    let part1 = expect_load_steering(NextUseCase::execute(
        ExecutionStateSource::Loaded(&before),
        DefinitionSource::Loaded(&definition),
        steering,
        &input,
    ));

    // 連鎖の途中でカーソルが進んだ (投影がリードモデルを更新した)。
    fs::write(
        record.join("aidlc-state.md"),
        state_file()
            .replace(
                "- [-] domain-design — EXECUTE",
                "- [x] domain-design — EXECUTE",
            )
            .replace("- [ ] contract-design", "- [-] contract-design")
            .replace(
                "- **Current Stage**: domain-design",
                "- **Current Stage**: contract-design",
            ),
    )
    .expect("状態ファイルの更新");
    let LoadedExecutionState::Loaded(after) =
        load_execution_state(&record).expect("更新後も読める")
    else {
        panic!("読めるはず")
    };
    assert_ne!(
        before.state_binding(),
        after.state_binding(),
        "state が動けば束縛も動く"
    );

    let sealed = mint_continue_token(KEY, part1.continue_token());
    let opened = verify_continue_token(KEY, &sealed).expect("開封はできる");
    let directive = ContinueUseCase::execute(
        Some(opened),
        ExecutionStateSource::Loaded(&after),
        DefinitionSource::Loaded(&definition),
        steering,
        &input,
    );
    assert_eq!(
        directive,
        Directive::Error {
            message: "The saved position moved on: the workflow state changed while this stage's \
                      rules were being loaded. Run a fresh `next` to restart delivery from part 1."
                .to_string()
        }
    );
}
