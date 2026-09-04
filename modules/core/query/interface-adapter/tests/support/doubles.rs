//! 契約テストの in-memory ダブルを、**SQLite 実装が読んだ行そのもの**から組む。
//!
//! 契約は「ジェネリック関数 1 本を 2 実装に同一に走らせる」形で書く
//! (`coding-rules/good-examples.md` §契約テスト)。そのとき両実装が**同じ入力**を見ている
//! ことが要点なので、ダブルの行は期待値をテストに書き下すのではなく、RMU が書いた行を
//! `*DaoImpl` で読み出して写す。写す鍵の並びはこのモジュールが 1 か所で持つ。
//!
//! 引けなかった鍵 (RMU が行を作らなかった鍵) は写さない — 行の不在も両実装で同じになる。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use core_query_interface_adapter::{
    InMemoryDefinitionDao, InMemoryDefinitionStageDao, InMemoryExecutionDao, InMemoryJumpDao,
    InMemoryJumpPhaseDao, InMemoryNextAnswerDao, InMemoryPhaseEntryDao, InMemoryRunStageDao,
    InMemoryScopeChangeDao, InMemoryScopeDao, InMemoryScopeKeywordDao, InMemorySteeringPartDao,
    InMemorySteeringPlanDao, ReadModelDaos,
};
use core_query_use_case::orchestration::{
    DefinitionDao, DefinitionStageDao, ExecutionDao, JumpDao, JumpPhaseDao, NextAnswerDao,
    PhaseEntryDao, RunStageDao, ScopeChangeDao, ScopeDao, ScopeKeywordDao, SteeringPartDao,
    SteeringPlanDao,
};

use super::{DEFINITION, EXECUTION, Fixture};

/// フィクスチャのストアを 1 度だけ開き、12 実装をその上に建てる。
fn daos(fixture: &Fixture) -> ReadModelDaos {
    ReadModelDaos::open(fixture.store()).expect("フィクスチャのストアは開ける")
}

/// フィクスチャの定義が持つステージ (グラフの並び順)。
pub(crate) const STAGES: [&str; 3] = ["state-init", "intent-capture", "requirements-analysis"];
/// フィクスチャの定義が持つ scope (カタログの綴り)。
pub(crate) const SCOPES: [&str; 3] = ["express", "classic", "feature"];
/// 定義が持ちうるフェーズ (入口・ジャンプ目的地の鍵の並び)。
pub(crate) const PHASES: [&str; 5] = [
    "initialization",
    "ideation",
    "inception",
    "construction",
    "operation",
];
/// `next` の要求の形 (行の鍵になる 4 値)。
pub(crate) const REQUEST_KINDS: [&str; 4] = ["bare", "resume", "free-text", "reentry"];
/// 部番号を探る上限 (フィクスチャの計画は 1 部で足りる — 終端は行の不在が表す)。
const MAX_PART_INDEX: u32 = 3;

/// `read_next_answer` の行を写したダブル。
pub(crate) fn next_answer(fixture: &Fixture) -> InMemoryNextAnswerDao {
    let source = daos(fixture).next_answer();
    let mut double = InMemoryNextAnswerDao::empty();
    for kind in REQUEST_KINDS {
        if let Some(view) = source.find(EXECUTION, kind).unwrap() {
            double = double.with_row(kind, view);
        }
    }
    double
}

/// `read_execution` の行を写したダブル。
pub(crate) fn execution(fixture: &Fixture) -> InMemoryExecutionDao {
    let source = daos(fixture).execution();
    let mut double = InMemoryExecutionDao::empty();
    if let Some(view) = source.find(EXECUTION).unwrap() {
        double = double.with_row(view);
    }
    double
}

/// `read_run_stage` の行 (定義 × 全 scope × 全ステージ) を写したダブル。
pub(crate) fn run_stage(fixture: &Fixture) -> InMemoryRunStageDao {
    let source = daos(fixture).run_stage();
    let mut double = InMemoryRunStageDao::empty();
    for scope in SCOPES {
        for slug in STAGES {
            if let Some(view) = source.find(DEFINITION, scope, slug).unwrap() {
                double = double.with_row(view);
            }
        }
    }
    double
}

/// run-stage の行がたどらせる配信計画の識別子 (重複を畳んだ並び)。
fn plan_ids(fixture: &Fixture) -> Vec<String> {
    let source = daos(fixture).run_stage();
    let mut ids: Vec<String> = Vec::new();
    for slug in STAGES {
        if let Some(view) = source.find(DEFINITION, "classic", slug).unwrap() {
            let id = view.steering_plan_id().to_string();
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

/// `read_steering_plan` の行を写したダブル。
pub(crate) fn steering_plan(fixture: &Fixture) -> InMemorySteeringPlanDao {
    let source = daos(fixture).steering_plan();
    let mut double = InMemorySteeringPlanDao::empty();
    for id in plan_ids(fixture) {
        if let Some(view) = source.find(&id).unwrap() {
            double = double.with_row(view);
        }
    }
    double
}

/// `read_steering_part` の行を写したダブル。
pub(crate) fn steering_part(fixture: &Fixture) -> InMemorySteeringPartDao {
    let source = daos(fixture).steering_part();
    let mut double = InMemorySteeringPartDao::empty();
    for id in plan_ids(fixture) {
        for part_index in 1..=MAX_PART_INDEX {
            if let Some(view) = source.find(&id, part_index).unwrap() {
                double = double.with_row(view);
            }
        }
    }
    double
}

/// `read_next_jump` の行を写したダブル。
pub(crate) fn jump(fixture: &Fixture) -> InMemoryJumpDao {
    let source = daos(fixture).jump();
    let mut double = InMemoryJumpDao::empty();
    for slug in STAGES {
        if let Some(view) = source.find(EXECUTION, slug).unwrap() {
            double = double.with_row(EXECUTION, view);
        }
    }
    double
}

/// `read_next_jump_phase` の行を写したダブル。
pub(crate) fn jump_phase(fixture: &Fixture) -> InMemoryJumpPhaseDao {
    let source = daos(fixture).jump_phase();
    let mut double = InMemoryJumpPhaseDao::empty();
    for phase in PHASES {
        if let Some(view) = source.find(EXECUTION, phase).unwrap() {
            double = double.with_row(EXECUTION, phase, view);
        }
    }
    double
}

/// `read_definition_scope` の行を写したダブル。
pub(crate) fn scope(fixture: &Fixture) -> InMemoryScopeDao {
    let source = daos(fixture).scope();
    let mut double = InMemoryScopeDao::empty();
    for name in SCOPES {
        if let Some(view) = source.find(DEFINITION, name).unwrap() {
            double = double.with_row(DEFINITION, view);
        }
    }
    double
}

/// `read_definition_scope_keyword` の行を写したダブル。
pub(crate) fn scope_keyword(fixture: &Fixture) -> InMemoryScopeKeywordDao {
    let source = daos(fixture).scope_keyword();
    let mut double = InMemoryScopeKeywordDao::empty();
    for keyword in ["api", "quick", "unclaimed"] {
        if let Some(name) = source.find(DEFINITION, keyword).unwrap() {
            double = double.with_row(DEFINITION, keyword, &name);
        }
    }
    double
}

/// `read_definition_scope_phase_entry` の行を写したダブル。
pub(crate) fn phase_entry(fixture: &Fixture) -> InMemoryPhaseEntryDao {
    let source = daos(fixture).phase_entry();
    let mut double = InMemoryPhaseEntryDao::empty();
    for name in SCOPES {
        for phase in PHASES {
            if let Some(view) = source.find(DEFINITION, name, phase).unwrap() {
                double = double.with_row(DEFINITION, name, phase, view);
            }
        }
    }
    double
}

/// `read_scope_change` の行を写したダブル。
pub(crate) fn scope_change(fixture: &Fixture) -> InMemoryScopeChangeDao {
    let source = daos(fixture).scope_change();
    let mut double = InMemoryScopeChangeDao::empty();
    for name in SCOPES {
        if let Some(view) = source.find(EXECUTION, name).unwrap() {
            double = double.with_row(EXECUTION, name, view);
        }
    }
    double
}

/// `read_definition` の行を写したダブル。
pub(crate) fn definition(fixture: &Fixture) -> InMemoryDefinitionDao {
    let source = daos(fixture).definition();
    let mut double = InMemoryDefinitionDao::empty();
    if let Some(view) = source.find(DEFINITION).unwrap() {
        double = double.with_row(DEFINITION, view);
    }
    double
}

/// `read_definition_stage` の行を写したダブル。
pub(crate) fn definition_stage(fixture: &Fixture) -> InMemoryDefinitionStageDao {
    let source = daos(fixture).definition_stage();
    let mut double = InMemoryDefinitionStageDao::empty();
    for slug in ["state-init", "intent-capture", "requirements-analysis"] {
        if let Some(view) = source.find(DEFINITION, slug).unwrap() {
            double = double.with_row(DEFINITION, view);
        }
    }
    double
}

/// 契約が使う配信計画の識別子 (ideation の計画 — run-stage の FK が指す先)。
pub(crate) fn ideation_plan_id(fixture: &Fixture) -> String {
    daos(fixture)
        .run_stage()
        .find(DEFINITION, "classic", "intent-capture")
        .unwrap()
        .unwrap()
        .steering_plan_id()
        .to_string()
}
