//! 統合テスト: memory 層 loader (`load_memory_rules`) + ドメインの分割・パック
//! (`MemoryRules::plan_for`) の合成 (issue #46 で旧 `RuleBundleSourceImpl` から移設)。
//!
//! 分割・パックの規則そのもの (見出し境界・コードポイント分割・無損失) はドメインの
//! `SteeringPlan::pack` のユニットテストが持つ。ここが見るのは **I/O の態度** — 解決順・
//! 欠落スキップ・initialization 特例・blocking な読取失敗である。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use core_command_domain::workflow_definition::PhaseId;

use aidlc::load_memory_rules;

#[test]
fn the_bundle_reads_in_resolution_order_and_skips_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("org.md"), "# Org\n").unwrap();
    std::fs::create_dir_all(dir.path().join("phases")).unwrap();
    std::fs::write(dir.path().join("phases/inception.md"), "# Inception\n").unwrap();
    // team.md / project.md は無い — 正常スキップ。
    let rules = load_memory_rules(dir.path()).unwrap();
    let plan = rules.plan_for(PhaseId::Inception).unwrap();
    let pieces: Vec<_> = plan.chunks().iter().flatten().collect();
    assert_eq!(pieces.len(), 2);
    let first = pieces.first().unwrap();
    let second = pieces.get(1).unwrap();
    assert!(first.path().ends_with("org.md"));
    assert!(second.path().ends_with("phases/inception.md"));
    assert_eq!(first.text(), "# Org\n");
}

#[test]
fn initialization_reads_no_phase_rule() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("org.md"), "# Org\n").unwrap();
    std::fs::create_dir_all(dir.path().join("phases")).unwrap();
    // phases/initialization.md が置かれていても loader は読まない (initialization は
    // フェーズルールファイルを持たない唯一のフェーズ — 02 §10)。
    std::fs::write(dir.path().join("phases/initialization.md"), "# Init\n").unwrap();
    let rules = load_memory_rules(dir.path()).unwrap();
    let plan = rules.plan_for(PhaseId::Initialization).unwrap();
    assert_eq!(
        plan.chunks().iter().flatten().count(),
        1,
        "base の org.md だけ"
    );
}

#[test]
fn an_empty_memory_dir_plans_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let rules = load_memory_rules(dir.path()).unwrap();
    assert!(rules.plan_for(PhaseId::Inception).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn an_unreadable_file_is_blocking() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("org.md");
    std::fs::write(&path, "# Org\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let error = load_memory_rules(dir.path()).unwrap_err();
    assert!(error.path().ends_with("org.md"));
    assert!(!error.cause().is_empty(), "OS 由来の理由を材料として運ぶ");
}
