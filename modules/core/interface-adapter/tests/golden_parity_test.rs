//! ゴールデンパリティ: `FsStageGraphReader` が **upstream の配布実バイト**を本家と同じに読むこと。
//!
//! 入力は `tests/golden/upstream-3c3146cf/{stage-graph.json,scope-grid.json}` — ピン留めコミット
//! `3c3146cf` (v2.6.40) の `dist/claude/.claude/tools/data/` からバイト無変更で持ってきたもの。
//! 期待値はすべて採取レポート `docs/specs/research/golden-3c3146cf-graph-dist.md` の実測に由来する
//! （推測値は 1 つも無い）。
//!
//! 本テストが閉じる仕様上の宿題:
//! - 12 §10 表 #3（全列挙を load 時厳格にする裁定）の残条件「ゴールデン採取で正規データが
//!   **全数 load** できること」。33 ノードすべてが `PhaseId` / `ExecutionKind` / `StageMode` /
//!   `ReviewClass` / `RuleScope` / `BrownfieldGreenfield` / `StageSlug` / `StageNumber` の
//!   厳密パースを通ることを、実バイトで示す。
//! - 12 §8 F2（文書順保持）の観測差確認。正規データでは配列順 = 数値順なので、文書順保持と
//!   数値順正規化に観測差が無いことを実データで固定する。
//! - 12 §11 の `enabled` 意味論。有効ノードでは `enabled` キーが JSON に出ない
//!   （`applyPluginSelection` が毎回 `delete` してから無効時のみ `= false` を立てるため）。
//!
//! scopes ディレクトリには**空の tempdir** を渡す。identity ファイルが 1 つも無い場合の挙動
//! （空カタログ = `valid_scopes()` が空）は `fs_stage_graph_reader_test.rs` で確立済みで、
//! ここでの関心はグラフとグリッドの実バイトだけだからである。グリッドが 11 列を持っていても
//! `valid_scopes()` が空になることは 12 §4 #6（グリッド列は有効スコープの権威ではない）の
//! 帰結であり、下の `the_grid_is_not_the_authority_for_valid_scopes` で明示する。
// ヘルパは `#[test]` の外にあるため clippy.toml の `allow-*-in-tests` が効かない。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use core_domain::orchestration::PlanAction;
use core_domain::workflow_definition::{ReviewClass, WorkflowDefinition};
use core_interface_adapter::orchestration::FsStageGraphReader;
use core_use_case::orchestration::StageGraphReader;
use std::path::PathBuf;
use tempfile::TempDir;

/// 採取レポート §5(b): 配布グラフのノード数。
const EXPECTED_NODE_COUNT: usize = 33;

/// 採取レポート §5(b): 配布グリッドのスコープ列数。
const EXPECTED_SCOPE_COLUMN_COUNT: usize = 11;

/// 採取レポート「VERIFIED COUNTS」: 列ごとの EXECUTE 数（as-built 01 §5.3 の Total 行と一致）。
const EXPECTED_EXECUTE_COUNTS: [(&str, usize); EXPECTED_SCOPE_COLUMN_COUNT] = [
    ("bugfix", 7),
    ("classic", 26),
    ("enterprise", 33),
    ("express", 10),
    ("feature", 33),
    ("infra", 13),
    ("mvp", 23),
    ("poc", 8),
    ("refactor", 8),
    ("security-patch", 10),
    ("workshop", 26),
];

/// 採取レポート §5(b): 配列順に並んだ `number` の実測列（= `numericStageOrder` 昇順）。
const EXPECTED_NUMBERS: [&str; EXPECTED_NODE_COUNT] = [
    "0.1", "0.2", "0.3", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "2.1", "2.2", "2.3",
    "2.4", "2.5", "2.6", "2.7", "2.8", "2.9", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7",
    "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.7",
];

/// 採取レポート「VERIFIED COUNTS」: `reviewer` を宣言するステージ数（as-built 01:259 / 01:1124）。
const EXPECTED_REVIEWER_STAGES: usize = 13;

/// 採取レポート §7.1: `review_class` の実測内訳。
const EXPECTED_ADVERSARIAL: usize = 5;
/// 同上。
const EXPECTED_ADVISORY: usize = 8;

/// ゴールデンフィクスチャの置き場（`tests/golden/upstream-3c3146cf/`）。
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../tests/golden/upstream-3c3146cf")
}

/// フィクスチャの `data_dir` と空の `scopes_dir` を与えたリーダ。
///
/// `TempDir` は返り値で保持する — drop すると scopes ディレクトリが消えてしまうため。
fn reader() -> (FsStageGraphReader, TempDir) {
    let scopes = TempDir::new().unwrap();
    let reader = FsStageGraphReader::new(golden_dir(), scopes.path().to_path_buf());
    (reader, scopes)
}

/// 3 入力を読んだ `WorkflowDefinition`。
fn load() -> (WorkflowDefinition, TempDir) {
    let (reader, scopes) = reader();
    let definition = reader
        .load()
        .expect("ピン留め配布物は 33 ノード全数が厳密パースを通るはず");
    (definition, scopes)
}

// ---------------------------------------------------------------------------
// (a) 33 ノード全パース成功
// ---------------------------------------------------------------------------

#[test]
fn every_node_of_the_shipped_graph_parses() {
    let (definition, _scopes) = load();
    assert_eq!(
        definition.graph().nodes().len(),
        EXPECTED_NODE_COUNT,
        "配布グラフは 33 ノード（as-built 01 §5.3）"
    );
    // slug は `StageGraph::new` が一意性を検査済み。ここでは 33 件が重複なく索引できることを見る。
    for node in definition.graph().nodes() {
        assert!(
            definition.graph().get(node.slug()).is_some(),
            "slug {:?} が索引から引けない",
            node.slug().as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// (b) 文書順 = 数値順（"0.1"〜"4.7"）
// ---------------------------------------------------------------------------

#[test]
fn the_document_order_is_already_the_numeric_order() {
    let (definition, _scopes) = load();
    let graph = definition.graph();

    let numbers: Vec<&str> = graph
        .nodes()
        .iter()
        .map(|node| node.number().as_str())
        .collect();
    assert_eq!(
        numbers,
        EXPECTED_NUMBERS.to_vec(),
        "配列順の number 列が採取実測と一致しない"
    );

    // F2 の観測差確認: 数値順ソートしても並びが変わらない = 正規データでは文書順保持と
    // 数値順正規化に観測差が無い（差が出るのは手編集グラフのみ）。
    let by_number: Vec<&str> = graph
        .numeric_order()
        .iter()
        .map(|node| node.slug().as_str())
        .collect();
    let by_document: Vec<&str> = graph
        .nodes()
        .iter()
        .map(|node| node.slug().as_str())
        .collect();
    assert_eq!(by_number, by_document);
}

// ---------------------------------------------------------------------------
// (c) グリッド 11 列・列ごとの EXECUTE 数
// ---------------------------------------------------------------------------

#[test]
fn the_grid_has_eleven_columns_with_the_measured_execute_counts() {
    let (definition, _scopes) = load();
    let grid = definition.grid();

    let expected_names: Vec<&str> = EXPECTED_EXECUTE_COUNTS
        .iter()
        .map(|(scope, _)| *scope)
        .collect();
    assert_eq!(
        grid.scope_names(),
        expected_names,
        "配布グリッドは 11 スコープ列（辞書順）"
    );

    for (scope, expected) in EXPECTED_EXECUTE_COUNTS {
        assert_eq!(
            grid.execute_slugs(scope).len(),
            expected,
            "スコープ {scope} の EXECUTE 数が採取実測と一致しない"
        );
    }

    // 全セルが 2 値のいずれか、かつ 33 × 11 = 363 セルが揃っていること（転置導出に倒れていない）。
    let mut cells = 0usize;
    for scope in grid.scope_names() {
        for node in definition.graph().nodes() {
            let action = grid
                .action(scope, node.slug())
                .expect("配布グリッドは全ステージを明示的に EXECUTE / SKIP で埋める");
            assert!(matches!(action, PlanAction::Execute | PlanAction::Skip));
            cells += 1;
        }
    }
    assert_eq!(cells, EXPECTED_NODE_COUNT * EXPECTED_SCOPE_COLUMN_COUNT);
}

#[test]
fn the_grid_is_not_the_authority_for_valid_scopes() {
    // identity ファイルが 1 つも無いので、グリッドが 11 列を持っていても有効スコープは 0 件
    // （12 §4 #6 / F7 — 権威は `.md` の存在）。
    let (definition, _scopes) = load();
    assert_eq!(
        definition.grid().scope_names().len(),
        EXPECTED_SCOPE_COLUMN_COUNT
    );
    assert!(definition.valid_scopes().is_empty());
}

// ---------------------------------------------------------------------------
// (d) reviewer 宣言 13 ステージ・adversarial 5 / advisory 8
// ---------------------------------------------------------------------------

#[test]
fn thirteen_stages_declare_a_reviewer_split_five_adversarial_and_eight_advisory() {
    let (definition, _scopes) = load();
    let nodes = definition.graph().nodes();

    let with_reviewer = nodes.iter().filter(|n| n.reviewer().is_some()).count();
    assert_eq!(with_reviewer, EXPECTED_REVIEWER_STAGES);

    let adversarial = nodes
        .iter()
        .filter(|n| n.review_class() == Some(ReviewClass::Adversarial))
        .count();
    let advisory = nodes
        .iter()
        .filter(|n| n.review_class() == Some(ReviewClass::Advisory))
        .count();
    assert_eq!(adversarial, EXPECTED_ADVERSARIAL);
    assert_eq!(advisory, EXPECTED_ADVISORY);
    assert_eq!(adversarial + advisory, EXPECTED_REVIEWER_STAGES);

    // B10 の 3 フィールドは束で立つ: reviewer があれば review_class も max_iterations もある。
    for node in nodes {
        assert_eq!(
            node.reviewer().is_some(),
            node.review_class().is_some(),
            "stage {:?} で reviewer と review_class の宣言が食い違う",
            node.slug().as_str()
        );
        assert_eq!(
            node.reviewer().is_some(),
            node.reviewer_max_iterations().is_some(),
            "stage {:?} で reviewer と reviewer_max_iterations の宣言が食い違う",
            node.slug().as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// (e) `enabled` キーの出現 0（全ノード有効）
// ---------------------------------------------------------------------------

#[test]
fn the_shipped_graph_carries_no_enabled_key_at_all() {
    // 生バイト側: プラグイン無選択の配布物では `enabled` キーが 1 つも emit されない
    // （`applyPluginSelection` は毎回 `delete stage.enabled` してから無効時のみ `= false`)。
    let raw = std::fs::read_to_string(golden_dir().join("stage-graph.json")).unwrap();
    assert!(
        !raw.contains("\"enabled\""),
        "配布グラフに enabled キーが現れた（採取実測は 0/33）"
    );

    // ドメイン側: キー不在は `None` として運ばれ、`is_enabled()` は全ノード true。
    let (definition, _scopes) = load();
    for node in definition.graph().nodes() {
        assert_eq!(node.enabled(), None, "stage {:?}", node.slug().as_str());
        assert!(node.is_enabled());
    }
}
