//! 群 A の読取 DAO とユースケースの契約 — **コンパイル済み定義（ファイル面リードモデル）を
//! 鍵で引き、当たらなければ空**。
//!
//! ここが固定するのは:
//! 1. `StageGraphDao` は slug で、無ければ番号で引く（`resolveStage`）。`enabled: false` は
//!    `find_all` から落ち、順序はグラフ順で保たれる。
//! 2. `ScopeGridDao` は 1 scope の割当を返し、`action_of` / 件数を答える。未知 scope は `None`。
//! 3. `ScopeMetadataDao` は scope 名の綴り順で `depth` / `testStrategy` を返す（未宣言は `None`）。
//! 4. `FindNextInScopeStageUseCase` は静的グリッドを歩き、状態ファイルが在れば per-stage 上書きと
//!    済/skip チェックボックスを尊重する。
//! 5. `ListScopeCatalogUseCase` はメタとグリッドを scope 名で突合して行を組む。
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::fs;

use core_query_interface_adapter::{
    ScopeGridDaoImpl, ScopeMetadataDaoImpl, StageGraphDaoImpl, StateFileDaoImpl,
};
use core_query_use_case::orchestration::{
    FindNextInScopeStageUseCase, ListScopeCatalogUseCase, ListStageGraphUseCase,
    ResolveStageUseCase, ScopeGridDao, ScopeMetadataDao, StageGraphDao,
};

/// 4 ステージ + 無効 1 のグラフ、2 scope のグリッド、2 scope 定義を書いた使い捨てツリー。
struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn create() -> Fixture {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let data = root.path().join("tools/data");
        let scopes = root.path().join("scopes");
        fs::create_dir_all(&data).expect("data");
        fs::create_dir_all(&scopes).expect("scopes");
        fs::write(
            data.join("stage-graph.json"),
            r#"[
              {"slug":"state-init","number":"0.3","name":"State Initialization","phase":"initialization","execution":"ALWAYS","lead_agent":"orchestrator","support_agents":[],"mode":"inline"},
              {"slug":"intent-capture","number":"1.1","name":"Intent Capture & Framing","phase":"ideation","execution":"ALWAYS","lead_agent":"aidlc-product-agent","support_agents":["aidlc-architect-agent"],"mode":"inline"},
              {"slug":"domain-design","number":"2.6","name":"Domain Design","phase":"inception","execution":"CONDITIONAL","lead_agent":"aidlc-architect-agent","support_agents":["aidlc-developer-agent","aidlc-product-agent"],"mode":"subagent"},
              {"slug":"deployment-execution","number":"4.2","name":"Deployment Execution","phase":"operation","execution":"ALWAYS","lead_agent":"aidlc-pipeline-deploy-agent","support_agents":[],"mode":"subagent"},
              {"slug":"disabled-stage","number":"9.9","name":"Disabled","phase":"operation","execution":"ALWAYS","lead_agent":"orchestrator","support_agents":[],"mode":"inline","enabled":false}
            ]"#,
        )
        .expect("stage-graph.json");
        fs::write(
            data.join("scope-grid.json"),
            r#"{
              "alpha": {"stages": {"state-init":"EXECUTE","intent-capture":"EXECUTE","domain-design":"SKIP","deployment-execution":"EXECUTE"}},
              "beta": {"stages": {"state-init":"EXECUTE","intent-capture":"SKIP","domain-design":"EXECUTE","deployment-execution":"SKIP"}}
            }"#,
        )
        .expect("scope-grid.json");
        fs::write(
            scopes.join("aidlc-alpha.md"),
            "---\nname: alpha\ndepth: Standard\ntestStrategy: standard\nskeleton: off\n---\n\n# alpha\n",
        )
        .expect("alpha");
        fs::write(
            scopes.join("aidlc-beta.md"),
            "---\nname: beta\ndepth: Minimal\nskeleton: on\n---\n\n# beta\n",
        )
        .expect("beta");
        Fixture { root }
    }

    fn data_dir(&self) -> std::path::PathBuf {
        self.root.path().join("tools/data")
    }

    fn scopes_dir(&self) -> std::path::PathBuf {
        self.root.path().join("scopes")
    }

    fn write_state(&self, body: &str) -> std::path::PathBuf {
        let path = self.root.path().join("aidlc-state.md");
        fs::write(&path, body).expect("state");
        path
    }

    fn absent_state(&self) -> std::path::PathBuf {
        self.root.path().join("no-state.md")
    }

    fn stage_graph(&self) -> StageGraphDaoImpl {
        StageGraphDaoImpl::new(&self.data_dir())
    }

    fn scope_grid(&self) -> ScopeGridDaoImpl {
        ScopeGridDaoImpl::new(&self.data_dir())
    }

    fn scope_metadata(&self) -> ScopeMetadataDaoImpl {
        ScopeMetadataDaoImpl::new(&self.scopes_dir())
    }
}

#[test]
fn resolve_stage_finds_by_slug() {
    let fx = Fixture::create();
    let stage = ResolveStageUseCase::new(fx.stage_graph())
        .execute("domain-design")
        .expect("読取")
        .expect("在る");
    assert_eq!(stage.phase(), "inception");
    assert_eq!(stage.lead_agent(), "aidlc-architect-agent");
    assert_eq!(stage.number(), "2.6");
}

#[test]
fn resolve_stage_falls_back_to_the_number() {
    let fx = Fixture::create();
    let stage = ResolveStageUseCase::new(fx.stage_graph())
        .execute("4.2")
        .expect("読取")
        .expect("番号で引ける");
    assert_eq!(stage.slug(), "deployment-execution");
}

#[test]
fn resolve_stage_returns_none_for_an_unknown_key() {
    let fx = Fixture::create();
    assert_eq!(
        ResolveStageUseCase::new(fx.stage_graph())
            .execute("no-such-stage")
            .expect("読取"),
        None
    );
}

#[test]
fn a_disabled_node_is_excluded_and_order_is_preserved() {
    let fx = Fixture::create();
    let stages = ListStageGraphUseCase::new(fx.stage_graph())
        .execute()
        .expect("読取");
    let slugs: Vec<&str> = stages.iter().map(|s| s.slug()).collect();
    assert_eq!(
        slugs,
        vec![
            "state-init",
            "intent-capture",
            "domain-design",
            "deployment-execution"
        ]
    );
    // 無効ノードは slug でも引けない。
    assert_eq!(fx.stage_graph().find("disabled-stage").expect("読取"), None);
}

#[test]
fn the_scope_grid_answers_actions_and_counts() {
    let fx = Fixture::create();
    let alpha = fx.scope_grid().find("alpha").expect("読取").expect("在る");
    assert_eq!(alpha.action_of("intent-capture"), Some("EXECUTE"));
    assert_eq!(alpha.action_of("domain-design"), Some("SKIP"));
    assert_eq!(alpha.action_of("absent"), None);
    assert_eq!(alpha.execute_count(), 3);
    assert_eq!(alpha.total_count(), 4);
    assert_eq!(fx.scope_grid().find("nope").expect("読取"), None);
}

#[test]
fn scope_metadata_is_sorted_and_carries_optional_test_strategy() {
    let fx = Fixture::create();
    let rows = fx.scope_metadata().find_all().expect("読取");
    let names: Vec<&str> = rows.iter().map(|r| r.scope()).collect();
    assert_eq!(names, vec!["alpha", "beta"]);
    assert_eq!(rows[0].depth(), "Standard");
    assert_eq!(rows[0].test_strategy(), Some("standard"));
    assert_eq!(rows[1].depth(), "Minimal");
    assert_eq!(rows[1].test_strategy(), None);
}

#[test]
fn the_scope_catalog_joins_metadata_and_grid_by_scope() {
    let fx = Fixture::create();
    let rows = ListScopeCatalogUseCase::new(fx.scope_metadata(), fx.scope_grid())
        .execute()
        .expect("読取");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].scope(), "alpha");
    assert_eq!(rows[0].execute(), 3);
    assert_eq!(rows[0].total(), 4);
    assert_eq!(rows[1].scope(), "beta");
    assert_eq!(rows[1].execute(), 2);
    assert_eq!(rows[1].total(), 4);
    assert_eq!(rows[1].test_strategy(), None);
}

#[test]
fn next_in_scope_walks_the_static_grid_without_a_state_file() {
    let fx = Fixture::create();
    let use_case = FindNextInScopeStageUseCase::new(
        fx.stage_graph(),
        fx.scope_grid(),
        StateFileDaoImpl::new(&fx.absent_state()),
    );
    // alpha: state-init -> intent-capture (EXECUTE)。
    assert_eq!(
        use_case.execute("state-init", "alpha").expect("読取"),
        Some("intent-capture".to_string())
    );
    // alpha: intent-capture -> (domain-design SKIP) -> deployment-execution。
    assert_eq!(
        use_case.execute("intent-capture", "alpha").expect("読取"),
        Some("deployment-execution".to_string())
    );
    // 末尾 -> None。
    assert_eq!(
        use_case
            .execute("deployment-execution", "alpha")
            .expect("読取"),
        None
    );
    // 未知 scope -> None。
    assert_eq!(use_case.execute("state-init", "nope").expect("読取"), None);
}

#[test]
fn next_in_scope_honours_state_suffix_overrides() {
    let fx = Fixture::create();
    // 承認済み計画の上書き: intent-capture を SKIP へ、domain-design を EXECUTE へ。
    let state = fx.write_state(
        "- [x] state-init — EXECUTE\n\
         - [ ] intent-capture — SKIP: excluded by recompose\n\
         - [ ] domain-design — EXECUTE\n\
         - [ ] deployment-execution — EXECUTE\n",
    );
    let next = FindNextInScopeStageUseCase::new(
        fx.stage_graph(),
        fx.scope_grid(),
        StateFileDaoImpl::new(&state),
    )
    .execute("state-init", "alpha")
    .expect("読取");
    // グリッドだけなら intent-capture だが、上書きで SKIP になり、SKIP-grid の domain-design が
    // EXECUTE へ昇格して次になる。
    assert_eq!(next, Some("domain-design".to_string()));
}

#[test]
fn next_in_scope_skips_completed_and_skipped_checkboxes() {
    let fx = Fixture::create();
    // intent-capture は済 (`x`) なので飛ばす。domain-design は grid SKIP。次は deployment-execution。
    let state = fx.write_state(
        "- [x] state-init — EXECUTE\n\
         - [x] intent-capture — EXECUTE\n\
         - [ ] domain-design — SKIP\n\
         - [ ] deployment-execution — EXECUTE\n",
    );
    let next = FindNextInScopeStageUseCase::new(
        fx.stage_graph(),
        fx.scope_grid(),
        StateFileDaoImpl::new(&state),
    )
    .execute("state-init", "alpha")
    .expect("読取");
    assert_eq!(next, Some("deployment-execution".to_string()));
}
