//! 統合テスト:
//! (b) 並行 2 スレッドの acquire で相互排他 (同時保持なし)
//! (c) 死んだ owner.json (存在しない PID) を書いた lock dir が reap される
//! (d) 生きた PID・新しい stamp は奪えない
//!
//! upstream `acquireAuditLock` / reap 契約 (03 §6.8, 11-workspace §4・W2)。
#![allow(clippy::unwrap_used)]

use core_domain::workspace::lock_identity::LockIdentity;
use core_interface_adapter::workspace::fs_workspace_lock::FsWorkspaceLock;
use core_use_case::workspace::workspace_lock::{AcquireBudget, AcquireError, WorkspaceLock};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

const fn budget(max_retries: u32, retry_interval_ms: u64) -> AcquireBudget {
    AcquireBudget::new(max_retries, Duration::from_millis(retry_interval_ms))
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

    let owner_json =
        format!("{{\"pid\":{dead_pid},\"startedAtMs\":0,\"reapLiveOwnerAfterStale\":true}}");
    std::fs::write(lock_dir.join("owner.json"), owner_json).unwrap();

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

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let owner_json = format!(
        "{{\"pid\":{},\"startedAtMs\":{now_ms},\"reapLiveOwnerAfterStale\":true}}",
        std::process::id()
    );
    std::fs::write(lock_dir.join("owner.json"), owner_json).unwrap();

    let mut lock = FsWorkspaceLock::new(dir.path().to_path_buf());
    let result = lock.acquire(&identity, budget(3, 5));
    assert_eq!(result.unwrap_err(), AcquireError::Exhausted);
    assert!(lock_dir.exists(), "生きた保持者のロック dir が残っていない");

    std::fs::remove_dir_all(&lock_dir).unwrap();
}
