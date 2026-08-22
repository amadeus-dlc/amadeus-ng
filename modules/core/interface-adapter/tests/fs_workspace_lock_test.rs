//! 統合テスト:
//! (b) 並行 2 スレッドの acquire で相互排他 (同時保持なし)
//! (c) 死んだ owner.json (存在しない PID) を書いた lock dir が reap される
//! (d) 生きた PID・新しい stamp は奪えない
//! (e) `reapLiveOwnerAfterStale: false` を宣言した生きた保持者は閾値超過でも奪えない
//!     (フラグ true なら奪える / 死亡なら奪える — upstream `:7036-7040` の 3 枝)
//! (f) owner.json は mode `0o600` + upstream 逐語のバイト形で書かれる
//!
//! upstream `acquireAuditLock` / reap 契約 (03 §6.8, 11-workspace §4・W2)。
#![allow(clippy::unwrap_used)]

use core_domain::workspace::LockIdentity;
use core_interface_adapter::workspace::FsWorkspaceLock;
use core_interface_adapter::{FakeClock, FakeProcessProbe};
use core_use_case::workspace::{AcquireBudget, AcquireError, WorkspaceLock};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

const fn budget(max_retries: u32, retry_interval_ms: u64) -> AcquireBudget {
    AcquireBudget::new(max_retries, Duration::from_millis(retry_interval_ms))
}

/// upstream `writeOwnerStamp` と同じバイト形の owner.json を手で書く (テストの下ごしらえ)。
fn write_stamp(lock_dir: &Path, pid: u32, started_at_ms: u64, reap_live_owner_after_stale: bool) {
    std::fs::write(
        lock_dir.join("owner.json"),
        format!(
            "{{\"pid\":{pid},\"startedAtMs\":{started_at_ms},\"reapLiveOwnerAfterStale\":{reap_live_owner_after_stale}}}"
        ),
    )
    .unwrap();
}

/// (b) 4 スレッド × 各 15 回の acquire/release を、同一 identity・同一 base_dir に対して
/// 別々の `FsWorkspaceLock` インスタンス (= 別プロセス相当) から行う。クリティカルセクション
/// 内で保持者数を数え、同時に 2 以上を観測しないことを確認する。
#[test]
fn concurrent_acquire_never_grants_two_holders_at_once() {
    let dir = tempdir().unwrap();
    let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
    let holders = Arc::new(AtomicUsize::new(0));
    let max_seen = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let base = dir.path().to_path_buf();
            let identity = identity.clone();
            let holders = Arc::clone(&holders);
            let max_seen = Arc::clone(&max_seen);
            thread::spawn(move || {
                let mut lock = FsWorkspaceLock::new(base);
                for _ in 0..15 {
                    let guard = lock.acquire(&identity, budget(500, 5)).unwrap();
                    let n = holders.fetch_add(1, Ordering::SeqCst) + 1;
                    max_seen.fetch_max(n, Ordering::SeqCst);
                    thread::sleep(Duration::from_micros(200));
                    holders.fetch_sub(1, Ordering::SeqCst);
                    lock.release(guard);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(
        max_seen.load(Ordering::SeqCst),
        1,
        "同時に 2 以上のロック保持者が観測された (相互排他違反)"
    );
}

/// (c) 存在しない PID を owner.json に持つ lock dir は reap され、後続の acquire が成功する。
#[test]
fn a_dead_owner_lock_dir_is_reaped() {
    let dir = tempdir().unwrap();
    let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
    let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
    std::fs::create_dir(&lock_dir).unwrap();

    // 確実に存在しない PID を得るため、子プロセスを spawn して wait する
    // (wait 後の PID は再利用されない限り存在しない — 短時間の統合テストでは十分安全)。
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let dead_pid = child.id();
    child.wait().unwrap();

    write_stamp(&lock_dir, dead_pid, 0, true);

    let mut lock = FsWorkspaceLock::new(dir.path().to_path_buf());
    let guard = lock.acquire(&identity, budget(20, 5)).unwrap();
    lock.release(guard);
    assert!(!lock_dir.exists());
}

/// (c') 死んだ保持者は `reapLiveOwnerAfterStale: false` を宣言していても奪える —
/// upstream `reapStaleLock` の死亡枝 (`else if (ownerAlive(owner))` に入らない) には
/// フラグのガードも年齢のガードも無い。
#[test]
fn a_dead_owner_is_reaped_even_when_it_declared_reap_live_owner_after_stale_false() {
    let dir = tempdir().unwrap();
    let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
    let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
    std::fs::create_dir(&lock_dir).unwrap();

    let mut child = std::process::Command::new("true").spawn().unwrap();
    let dead_pid = child.id();
    child.wait().unwrap();
    write_stamp(&lock_dir, dead_pid, 0, false);

    let mut lock = FsWorkspaceLock::new(dir.path().to_path_buf());
    let guard = lock.acquire(&identity, budget(20, 5)).unwrap();
    lock.release(guard);
    assert!(!lock_dir.exists());
}

/// (d) 生きた PID (自プロセス自身) と新しい stamp を持つロックは、予算を使い切っても
/// 奪えない (W2 — 生きている閾値未満の保持者から決して奪わない)。
#[test]
fn a_live_pid_with_a_fresh_stamp_is_never_reaped() {
    let dir = tempdir().unwrap();
    let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
    let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
    std::fs::create_dir(&lock_dir).unwrap();

    let now_ms = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    write_stamp(&lock_dir, std::process::id(), now_ms, true);

    let mut lock = FsWorkspaceLock::new(dir.path().to_path_buf());
    let result = lock.acquire(&identity, budget(3, 5));
    assert_eq!(result.unwrap_err(), AcquireError::Exhausted);
    assert!(lock_dir.exists(), "生きた保持者のロック dir が残っていない");

    std::fs::remove_dir_all(&lock_dir).unwrap();
}

/// (e) `reapLiveOwnerAfterStale: false` を宣言した**生きた**保持者は、stale 閾値を大きく
/// 超えていても奪えない (upstream `:7037` `if (!owner.reapLiveOwnerAfterStale) return false;`)。
/// 同じ条件でフラグが `true` なら奪える — 差分がフラグだけであることを対で示す。
#[test]
fn a_live_owner_declaring_reap_live_owner_after_stale_false_is_never_reaped() {
    let dir = tempdir().unwrap();
    let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
    let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
    std::fs::create_dir(&lock_dir).unwrap();

    // FakeProcessProbe は既定で全 pid を alive と報告する = 「生きた保持者」。
    // 時刻は閾値 (1_000ms) を遥かに超える 100_000ms に置く。
    let contender = |base: &Path| {
        FsWorkspaceLock::with_clock_and_probe(
            base.to_path_buf(),
            Arc::new(FakeClock::new(100_000)),
            Arc::new(FakeProcessProbe::new()),
        )
        .with_thresholds(1_000, 5_000)
    };

    write_stamp(&lock_dir, std::process::id(), 0, false);
    let mut blocked = contender(dir.path());
    assert_eq!(
        blocked.acquire(&identity, budget(2, 1)).unwrap_err(),
        AcquireError::Exhausted,
        "フラグ false の生存保持者は閾値超過でも守られる"
    );
    assert!(lock_dir.exists(), "守られた dir は消えていない");

    // 同一条件でフラグだけ true にすると奪える (対照)
    write_stamp(&lock_dir, std::process::id(), 0, true);
    let mut allowed = contender(dir.path());
    let guard = allowed
        .acquire(&identity, budget(2, 1))
        .expect("フラグ true の生存保持者は閾値超過なら奪える");
    allowed.release(guard);
    assert!(!lock_dir.exists());
}

/// (f) owner.json は upstream `writeFileSync(..., { mode: 0o600 })` と同じパーミッションで、
/// `JSON.stringify` と同じバイト形 (キー順 `pid` → `startedAtMs` → `reapLiveOwnerAfterStale`、
/// インデント無し、末尾改行無し) で書かれる。
#[test]
fn the_owner_stamp_is_written_with_mode_0600_in_the_upstream_byte_form() {
    let dir = tempdir().unwrap();
    let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
    let mut lock = FsWorkspaceLock::new(dir.path().to_path_buf());
    let guard = lock.acquire(&identity, budget(5, 1)).unwrap();

    let stamp_path = FsWorkspaceLock::lock_dir_path(dir.path(), &identity).join("owner.json");
    let mode = std::fs::metadata(&stamp_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "upstream は mode 0o600 で owner.json を書く");

    let body = std::fs::read_to_string(&stamp_path).unwrap();
    assert_eq!(
        body,
        format!(
            "{{\"pid\":{},\"startedAtMs\":{},\"reapLiveOwnerAfterStale\":true}}",
            std::process::id(),
            body.split("\"startedAtMs\":")
                .nth(1)
                .unwrap()
                .split(',')
                .next()
                .unwrap()
        ),
        "キー順・インデント無し・末尾改行無しまで upstream 逐語"
    );

    lock.release(guard);
}
