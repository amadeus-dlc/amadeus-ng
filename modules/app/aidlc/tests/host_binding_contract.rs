//! C8 のホスト識別と切替準備 — `scripts/aidlc-selfhost/host-binding.json` が、
//! ホスト (検証済みの安定版) とターゲット (開発版) を取り違えない形で識別し、
//! 復帰が検証済みホストへ戻り、準備の完了を達成と混同しないことの検査。
//!
//! **ここが green でも、実地スモーク (FR7)・自己診断・CI 全ジョブ成功・切替 (FR8) は
//! 未達である** (C8 `not_proof_of_completion`)。この検査が見るのは準備の形だけである。
//! 安定タグはここでは作らない (作成は切替工程)。
// 契約テストは固定の添字参照と panic を検証の合図として使う (既存の契約テストと同じ許容)。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn manifest() -> serde_json::Value {
    let path = repo_root().join("scripts/aidlc-selfhost/host-binding.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("ホスト識別が読めない ({}): {error}", path.display()));
    serde_json::from_str(&raw).expect("ホスト識別は JSON")
}

/// ホスト識別の原文 (逐語の検査に使う — 直列化し直さない)。
fn manifest_text() -> String {
    fs::read_to_string(repo_root().join("scripts/aidlc-selfhost/host-binding.json"))
        .expect("ホスト識別の原文")
}

fn field<'a>(value: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("{key} が無い: {value}"))
}

fn text(value: &serde_json::Value, key: &str) -> String {
    field(value, key)
        .as_str()
        .unwrap_or_else(|| panic!("{key} は文字列であるべき: {value}"))
        .to_string()
}

/// ホストとターゲットは、役割・実体・識別の材料が別々に書かれている。
#[test]
fn the_manifest_separates_the_host_from_the_target() {
    let manifest = manifest();
    let host = field(&manifest, "host");
    let target = field(&manifest, "target");
    assert_eq!(text(host, "role"), "host");
    assert_eq!(text(target, "role"), "target");
    assert_ne!(
        text(host, "role"),
        text(target, "role"),
        "ホストとターゲットの役割が同じ"
    );
    // タグ・コミット・バイナリ実体の 3 つを、どちらの側も同じ鍵で語る。
    for side in [host, target] {
        for key in ["tag", "commit", "binary", "sha256", "status"] {
            assert!(
                side.get(key).is_some(),
                "{} に {key} が無い",
                text(side, "role")
            );
        }
    }
    assert_eq!(
        text(target, "binary"),
        "target/release/aidlc",
        "ターゲットの実体は開発版のビルド成果物である"
    );
}

/// 未検証の版を安定版と呼ばない — タグもコミットもバイナリの指紋も持たせない。
#[test]
fn an_unverified_side_carries_no_tag_and_is_never_called_stable() {
    let manifest = manifest();
    for key in ["host", "target"] {
        let side = field(&manifest, key);
        let status = text(side, "status");
        assert!(
            ["verified", "unverified", "development"].contains(&status.as_str()),
            "{key}: 未知の状態 {status}"
        );
        if status == "verified" {
            for required in ["tag", "commit", "binary", "sha256"] {
                assert!(
                    !field(side, required).is_null(),
                    "{key}: 検証済みなのに {required} が空である"
                );
            }
            continue;
        }
        assert!(
            field(side, "tag").is_null(),
            "{key}: 未検証の版にタグを与えている"
        );
        assert!(
            field(side, "sha256").is_null(),
            "{key}: 未検証の版に確定した指紋を与えている"
        );
        assert_ne!(
            text(side, "status"),
            "stable",
            "{key}: 未検証の版を安定版と呼んでいる"
        );
    }
    // 安定タグはこの工程で作らない。
    assert_eq!(
        text(&manifest, "tag_creation"),
        "deferred-to-switch-stage",
        "タグの作成をこの工程へ引き込まない"
    );
}

/// 切替の前提条件は C8 の 3 つで、済みと書けるのはローカルで実測した自己診断だけである。
///
/// 自己診断はこの工程で実際に走らせるので、実行したコマンド・日時・結果の要約を伴って
/// `met` になる。実地スモークと CI 全ジョブは人が別途行うため `unmet` のまま残る —
/// ローカル検証を CI 全ジョブ成功へ読み替えない。
#[test]
fn only_the_locally_measured_doctor_precondition_is_marked_met() {
    let manifest = manifest();
    let preconditions = field(&manifest, "switch_preconditions")
        .as_array()
        .expect("switch_preconditions は配列");
    let ids: BTreeSet<String> = preconditions
        .iter()
        .map(|entry| text(entry, "id"))
        .collect();
    assert_eq!(
        ids,
        [
            "all_ci_jobs_pass".to_string(),
            "doctor_pass".to_string(),
            "live_bugfix_smoke_pass".to_string(),
        ]
        .into_iter()
        .collect::<BTreeSet<_>>(),
        "C8 の切替前提と一致しない"
    );
    for entry in preconditions {
        let id = text(entry, "id");
        assert!(
            !text(entry, "how").is_empty(),
            "{id}: 確かめ方が書かれていない"
        );
        if id == "doctor_pass" {
            assert_eq!(
                text(entry, "status"),
                "met",
                "{id}: ローカルで実測した自己診断が済みになっていない"
            );
            let evidence = field(entry, "evidence");
            assert!(
                evidence.is_object(),
                "{id}: 証拠がコマンド・日時・結果の記録になっていない"
            );
            let command = text(evidence, "command");
            assert!(
                command.contains("--doctor"),
                "{id}: 証拠が実行したコマンドを名指していない ({command})"
            );
            assert!(
                !text(evidence, "ran_at").is_empty(),
                "{id}: 証拠に実行日時が無い"
            );
            assert!(
                !text(evidence, "result").is_empty(),
                "{id}: 証拠に結果の要約が無い"
            );
            continue;
        }
        assert_eq!(
            text(entry, "status"),
            "unmet",
            "{id}: 人が別途行う前提を済みと書いている"
        );
        assert!(
            field(entry, "evidence").is_null(),
            "{id}: 証拠が無いのに証拠欄が埋まっている"
        );
    }
}

/// 復帰は検証済みホストへ戻す。ホストが未検証のあいだは復帰先が無いと明示する。
#[test]
fn the_rollback_restores_the_verified_host_and_never_points_at_the_target() {
    let manifest = manifest();
    let rollback = field(&manifest, "rollback");
    assert_eq!(text(rollback, "restores"), "host", "復帰先がホストではない");
    let steps = field(rollback, "steps").as_array().expect("steps は配列");
    assert!(!steps.is_empty(), "復帰手順が空である");
    for step in steps {
        let command = text(step, "command");
        assert!(
            !command.contains("target/release/aidlc\" hook"),
            "復帰手順がターゲットの接続を張り直している: {command}"
        );
    }
    // ホストが未検証なら、復帰は「使える」と書かない。
    let host_status = text(field(&manifest, "host"), "status");
    let available = field(rollback, "available")
        .as_bool()
        .expect("available は真偽値");
    assert_eq!(
        available,
        host_status == "verified",
        "検証されていないホストへ復帰できると書いている"
    );
    assert!(
        !text(rollback, "when_unavailable").is_empty(),
        "復帰先が無いときの扱いが書かれていない"
    );
}

/// 切替手順はフック接続定義を名指し、各段に戻し方がある。
#[test]
fn the_switch_procedure_names_the_hook_binding_definition_and_every_step_is_reversible() {
    let manifest = manifest();
    let steps = field(&manifest, "switch_procedure")
        .as_array()
        .expect("switch_procedure は配列");
    assert!(steps.len() >= 3, "切替手順が短すぎる");
    for step in steps {
        assert!(
            !text(step, "command").is_empty() || !text(step, "action").is_empty(),
            "手順に実行内容が無い: {step}"
        );
        assert!(
            !text(step, "undo").is_empty(),
            "戻し方の無い手順がある: {step}"
        );
    }
    let joined = manifest_text();
    assert!(
        joined.contains("scripts/aidlc-selfhost/hook-binding.json"),
        "切替手順がフック接続定義を名指していない"
    );
    assert!(
        joined.contains(".claude/settings.json"),
        "切替手順が接続先の設定を名指していない"
    );
}

/// ターゲットの識別は、このリポジトリから実際に解決できる。
#[test]
fn the_target_identity_resolves_from_this_repository() {
    let manifest = manifest();
    let target = field(&manifest, "target");
    let command = text(target, "commit_command");
    assert_eq!(command, "git rev-parse HEAD", "コミットの取り方が違う");
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root())
        .output()
        .expect("git");
    assert!(output.status.success(), "対象コミットが解決できない");
    let commit = String::from_utf8(output.stdout).unwrap();
    assert_eq!(commit.trim().len(), 40, "コミットの綴りが 40 桁でない");
    let fingerprint = text(target, "sha256_command");
    assert!(
        fingerprint.contains("target/release/aidlc"),
        "バイナリ実体の指紋の取り方が実体を指していない"
    );
    assert!(
        !text(target, "build_command").is_empty(),
        "ターゲットの作り方が書かれていない"
    );
}

/// 準備の完了を達成と混同しない。
#[test]
fn preparation_is_never_recorded_as_achievement() {
    let manifest = manifest();
    let note = text(&manifest, "not_proof_of_completion");
    assert!(!note.is_empty(), "準備と達成の区別が書かれていない");
    let joined = manifest_text();
    for claim in [
        "\"smoke_passed\": true",
        "\"switched\": true",
        "\"doctor_passed\": true",
        "\"status\": \"verified\"",
    ] {
        assert!(
            !joined.contains(claim),
            "達成を先取りした記述がある: {claim}"
        );
    }
    // 済みと書けるのは、この工程でローカルに実測した自己診断だけである。
    let met: BTreeSet<String> = field(&manifest, "switch_preconditions")
        .as_array()
        .expect("switch_preconditions は配列")
        .iter()
        .filter(|entry| text(entry, "status") == "met")
        .map(|entry| text(entry, "id"))
        .collect();
    assert_eq!(
        met,
        ["doctor_pass".to_string()]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        "ローカル検証だけで済ませられない前提まで達成と書いている"
    );
    assert_eq!(
        text(&manifest, "binding_selected"),
        "distributed-typescript",
        "まだ切り替えていないのに接続先を native と書いている"
    );
}
