//! ITF 準拠テスト (ADR 0003 決定 5) — `formal/workspace/audit_lock.qnt` のトレースを
//! `LockProtocol` に再生し、全ステップで状態射影を突き合わせる。
//! フィクスチャは `tests/conformance/fixtures/audit_lock/` にコミット済み (#meta 正規化済み)。
//! トレースの各遷移は `lastAction` × `lastActor` で駆動する (lastAction 規約)。
//! モデルのアクションは enabled なときしか発火しないため、ここでのコマンド `Err` は
//! 準拠違反として扱い `unwrap` する (ADR 0003 決定 5 の型)。

// テストコードでは unwrap を許可 (オーナー規約)。integration test のヘルパは
// clippy.toml の allow-unwrap-in-tests の検出対象外のため file-level で明示する。
#![allow(clippy::unwrap_used)]

use core_domain::workspace::lock_protocol::LockProtocol;
use serde_json::Value;

/// `audit_lock.qnt` の `THRESHOLD` (モデル定数そのもの)。
const THRESHOLD: u32 = 3;

fn bigint(v: &Value) -> i64 {
    v["#bigint"].as_str().unwrap().parse().unwrap()
}

/// `int -> bool` の #map を index 順の Vec へ (`alive`)。
fn map_to_bool_vec(v: &Value, n: usize) -> Vec<bool> {
    let pairs = v["#map"].as_array().unwrap();
    let mut out: Vec<Option<bool>> = (0..n).map(|_| None).collect();
    for p in pairs {
        let (k, val) = (&p[0], &p[1]);
        out[usize::try_from(bigint(k)).unwrap()] = Some(val.as_bool().unwrap());
    }
    out.into_iter().map(Option::unwrap).collect()
}

struct ModelState {
    last_action: String,
    last_actor: i64,
    alive: Vec<bool>,
    dir_owner: i64,
    depth: u32,
    held_ticks: u32,
    audit_len: u64,
    state_ver: u64,
    pending: bool,
}

fn parse_state(v: &Value) -> ModelState {
    let n = v["alive"]["#map"].as_array().unwrap().len();
    ModelState {
        last_action: v["lastAction"].as_str().unwrap().to_string(),
        last_actor: bigint(&v["lastActor"]),
        alive: map_to_bool_vec(&v["alive"], n),
        dir_owner: bigint(&v["dirOwner"]),
        depth: u32::try_from(bigint(&v["depth"])).unwrap(),
        held_ticks: u32::try_from(bigint(&v["heldTicks"])).unwrap(),
        audit_len: u64::try_from(bigint(&v["auditLen"])).unwrap(),
        state_ver: u64::try_from(bigint(&v["stateVer"])).unwrap(),
        pending: v["pending"].as_bool().unwrap(),
    }
}

fn assert_projection(lock: &LockProtocol, m: &ModelState, step: usize) {
    assert_eq!(
        lock.process_count(),
        m.alive.len(),
        "step {step}: process count"
    );
    for p in 0..lock.process_count() {
        assert_eq!(lock.alive(p), m.alive[p], "step {step}: alive[{p}]");
    }
    let dir_owner = lock.dir_owner().map_or(-1, |p| i64::try_from(p).unwrap());
    assert_eq!(dir_owner, m.dir_owner, "step {step}: dirOwner");
    assert_eq!(lock.depth(), m.depth, "step {step}: depth");
    assert_eq!(lock.held_ticks(), m.held_ticks, "step {step}: heldTicks");
    assert_eq!(lock.audit_len(), m.audit_len, "step {step}: auditLen");
    assert_eq!(lock.state_ver(), m.state_ver, "step {step}: stateVer");
    assert_eq!(lock.pending(), m.pending, "step {step}: pending");
}

fn actor(m: &ModelState) -> usize {
    usize::try_from(m.last_actor).unwrap()
}

fn replay(path: &std::path::Path, seen: &mut std::collections::BTreeSet<String>) {
    let text = std::fs::read_to_string(path).unwrap();
    let trace: Value = serde_json::from_str(&text).unwrap();
    let states: Vec<ModelState> = trace["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(parse_state)
        .collect();
    let m0 = &states[0];
    assert_eq!(m0.last_action, "init");
    let mut lock = LockProtocol::new(m0.alive.len(), THRESHOLD);
    assert_projection(&lock, m0, 0);

    for (i, m) in states.iter().enumerate().skip(1) {
        seen.insert(m.last_action.clone());
        match m.last_action.as_str() {
            "acquire" => lock.acquire(actor(m)).unwrap(),
            "reenter" => lock.reenter(actor(m)).unwrap(),
            "transition_emit" => lock.transition_emit(actor(m)).unwrap(),
            "transition_write" => lock.transition_write(actor(m)).unwrap(),
            "transition_emit_fail" => lock.transition_emit_fail(actor(m)).unwrap(),
            "release" => lock.release(actor(m)).unwrap(),
            "crash" => lock.crash(actor(m)).unwrap(),
            "tick" => lock.tick().unwrap(),
            "reap" => lock.reap(actor(m)).unwrap(),
            "idle" => {}
            a => panic!("step {i}: unknown action {a}"),
        }
        assert_projection(&lock, m, i);
    }
}

#[test]
fn lock_protocol_conforms_to_every_committed_audit_lock_trace() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/conformance/fixtures/audit_lock");
    let mut count = 0;
    let mut seen = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            replay(&path, &mut seen);
            count += 1;
        }
    }
    assert!(count >= 6, "expected committed fixtures, found {count}");
    // アクション網羅: 全アクションが少なくとも 1 つのコミット済みトレースに現れること。
    // ファイル数だけの検査だと、稀にしか出ないアクション (reap は初回 6 シードに一度も
    // 現れず 0xee を追加した) を含む fixture の消失に気づけないため、和集合で退行を防ぐ。
    for action in [
        "acquire",
        "reenter",
        "transition_emit",
        "transition_write",
        "transition_emit_fail",
        "release",
        "crash",
        "tick",
        "reap",
    ] {
        assert!(
            seen.contains(action),
            "no committed trace exercises action {action}"
        );
    }
}
