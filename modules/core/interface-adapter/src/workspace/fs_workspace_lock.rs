//! `WorkspaceLock` の実 Gateway — mkdir-EEXIST ロック (upstream `acquireAuditLock`,
//! 03 §6.8, 11-workspace §4)。
//!
//! - lock dir: `<tmpdir>/.aidlc-audit-<md5(identity)[..8]>.lock` (md5 の 16 進先頭 8 文字 —
//!   upstream 互換, 11-workspace §9)。
//! - owner stamp: lock dir 内 `owner.json` = `{pid, startedAtMs, reapLiveOwnerAfterStale}`。
//! - reap 条件: owner.json が読めて `process_alive(pid) == false` または
//!   `now - startedAtMs > stale_ms` (既定 600_000ms)。未スタンプ dir (mkdir 直後・owner.json
//!   未書込) は `grace_ms` (既定 5_000ms — 対象ディレクトリの mtime で近似する。ファイルシス
//!   テム間で可搬な `birthtime` に依存しない) で保護する。
//! - reap は CAS: 私有 `<lockDir>.dead.<pid>-<counter>` へ rename → 移動先の owner.json が
//!   読取時のものと一致するか再検証 → 一致すれば rm -rf で確定、不一致なら rename で復元
//!   (復元が EEXIST で失敗すれば第三者が既に新しいロックを握っているので私有 dir を破棄する)。
//! - release は識別子ごとの深度カウンタで管理し、深度 0 に戻るときのみ rm -rf する。

use core_domain::workspace::lock_identity::LockIdentity;
use core_domain::workspace::lock_protocol::reap_eligible;
use core_use_case::workspace::clock::Clock;
use core_use_case::workspace::process_probe::ProcessProbe;
use core_use_case::workspace::workspace_lock::{
    AcquireBudget, AcquireError, LockGuard, WorkspaceLock,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::clock::SystemClock;
use super::process_probe::OsProcessProbe;

/// stale reap の既定閾値 (upstream `DEFAULT_LOCK_STALE_MS`, 03 §6.8)。
pub const DEFAULT_LOCK_STALE_MS: u64 = 600_000;
/// 未スタンプ dir の既定猶予 (upstream `unstampedGraceMs()`, 03 §6.8)。
pub const DEFAULT_UNSTAMPED_GRACE_MS: u64 = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnerStamp {
    pid: i32,
    started_at_ms: u64,
    reap_live_owner_after_stale: bool,
}

impl OwnerStamp {
    /// CAS の同一性 (`stampMatches` 相当): 保持者の同一性は `pid` と `startedAtMs` で決まる。
    /// `reapLiveOwnerAfterStale` はポリシー宣言であり同一性の一部ではない。
    const fn names_same_holder(&self, other: &OwnerStamp) -> bool {
        self.pid == other.pid && self.started_at_ms == other.started_at_ms
    }
}

fn serialize_owner_stamp(stamp: &OwnerStamp) -> String {
    format!(
        "{{\"pid\":{},\"startedAtMs\":{},\"reapLiveOwnerAfterStale\":{}}}",
        stamp.pid, stamp.started_at_ms, stamp.reap_live_owner_after_stale
    )
}

/// 手書きの最小 JSON パーサ (owner.json はこの Gateway 自身が書いた 3 フィールド固定形の
/// み読む — 汎用 JSON パーサへの依存を避けるための最小実装)。
fn parse_owner_stamp(s: &str) -> Option<OwnerStamp> {
    let pid = extract_i64_field(s, "\"pid\":")?;
    let started_at_ms = extract_i64_field(s, "\"startedAtMs\":")?;
    let reap_live_owner_after_stale = s.contains("\"reapLiveOwnerAfterStale\":true");
    Some(OwnerStamp {
        pid: i32::try_from(pid).ok()?,
        started_at_ms: u64::try_from(started_at_ms).ok()?,
        reap_live_owner_after_stale,
    })
}

fn extract_i64_field(s: &str, key: &str) -> Option<i64> {
    let start = s.find(key)? + key.len();
    let rest = &s[start..];
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    rest[..end].trim().parse::<i64>().ok()
}

fn system_time_to_ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// mkdir-EEXIST ロックの実 Gateway。1 インスタンス = 1 プロセス相当 (再入深度カウンタは
/// インスタンス内にのみ存在する — upstream の in-process depth/handler map に対応)。
pub struct FsWorkspaceLock {
    base_dir: PathBuf,
    depths: HashMap<LockIdentity, u32>,
    clock: Arc<dyn Clock + Send + Sync>,
    process_probe: Arc<dyn ProcessProbe + Send + Sync>,
    stale_ms: u64,
    grace_ms: u64,
    reap_counter: u64,
}

impl FsWorkspaceLock {
    /// 実時計・実プロセス判定で構築する。`base_dir` はロック dir の置き場
    /// (upstream `os.tmpdir()` 相当。テストでは tempdir を渡す)。
    #[must_use]
    pub fn new(base_dir: PathBuf) -> FsWorkspaceLock {
        FsWorkspaceLock::with_clock_and_probe(
            base_dir,
            Arc::new(SystemClock::new()),
            Arc::new(OsProcessProbe::new()),
        )
    }

    /// clock / process-probe を注入して構築する (テスト用 — 11-workspace §4)。
    #[must_use]
    pub fn with_clock_and_probe(
        base_dir: PathBuf,
        clock: Arc<dyn Clock + Send + Sync>,
        process_probe: Arc<dyn ProcessProbe + Send + Sync>,
    ) -> FsWorkspaceLock {
        FsWorkspaceLock {
            base_dir,
            depths: HashMap::new(),
            clock,
            process_probe,
            stale_ms: DEFAULT_LOCK_STALE_MS,
            grace_ms: DEFAULT_UNSTAMPED_GRACE_MS,
            reap_counter: 0,
        }
    }

    /// stale / grace 閾値を上書きする (テスト用ビルダー)。
    #[must_use]
    pub const fn with_thresholds(mut self, stale_ms: u64, grace_ms: u64) -> FsWorkspaceLock {
        self.stale_ms = stale_ms;
        self.grace_ms = grace_ms;
        self
    }

    /// ロック dir パスの算出 — `<base_dir>/.aidlc-audit-<md5(identity)[..8]>.lock`
    /// (md5 の 16 進先頭 8 文字 — upstream 互換, 11-workspace §9)。`base_dir` と identity から
    /// 決定論的に導出するので、Gateway インスタンスの外 (テストの下ごしらえ・診断) からも
    /// 同じ値を再計算できるよう関連関数として公開する。
    #[must_use]
    pub fn lock_dir_path(base_dir: &Path, identity: &LockIdentity) -> PathBuf {
        let digest = md5::compute(identity.as_bytes());
        let hex = format!("{digest:x}");
        let short = &hex[..8];
        base_dir.join(format!(".aidlc-audit-{short}.lock"))
    }

    fn write_owner_stamp(&self, lock_dir: &Path) -> io::Result<()> {
        let stamp = OwnerStamp {
            pid: std::process::id() as i32,
            started_at_ms: self.clock.now_ms(),
            reap_live_owner_after_stale: true,
        };
        fs::write(lock_dir.join("owner.json"), serialize_owner_stamp(&stamp))
    }

    /// reap を試みる。`true` を返したら呼出側は mkdir を即再試行してよい (予算を消費しない)。
    fn try_reap(&mut self, lock_dir: &Path) -> bool {
        let owner_path = lock_dir.join("owner.json");
        let stamp_before = fs::read_to_string(&owner_path)
            .ok()
            .and_then(|s| parse_owner_stamp(&s));

        let reapable = match &stamp_before {
            Some(stamp) => {
                // reap 適格判定はドメインの単一実装 (lock_protocol::reap_eligible) に委譲する
                let alive = self.process_probe.is_alive(stamp.pid);
                let age = self.clock.now_ms().saturating_sub(stamp.started_at_ms);
                reap_eligible(alive, age, self.stale_ms)
            }
            None => match fs::metadata(lock_dir).and_then(|m| m.modified()) {
                Ok(modified) => {
                    let age = self
                        .clock
                        .now_ms()
                        .saturating_sub(system_time_to_ms(modified));
                    age > self.grace_ms
                }
                Err(_) => false,
            },
        };

        if !reapable {
            return false;
        }

        self.reap_counter += 1;
        let dead_dir = lock_dir.with_file_name(format!(
            "{}.dead.{}-{}",
            lock_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("lock"),
            std::process::id(),
            self.reap_counter,
        ));

        if fs::rename(lock_dir, &dead_dir).is_err() {
            // 既に他者が reap/解放済み — lock_dir が空いた可能性が高いので mkdir 再試行させる。
            return true;
        }

        let stamp_after = fs::read_to_string(dead_dir.join("owner.json"))
            .ok()
            .and_then(|s| parse_owner_stamp(&s));

        let matches = match (&stamp_before, &stamp_after) {
            (None, None) => true,
            (Some(a), Some(b)) => a.names_same_holder(b),
            _ => false,
        };

        if matches {
            let _ = fs::remove_dir_all(&dead_dir);
            true
        } else if fs::rename(&dead_dir, lock_dir).is_ok() {
            false
        } else {
            // 第三者が隙間で再 mkdir 済み — 私有 dir は単に破棄する。
            let _ = fs::remove_dir_all(&dead_dir);
            false
        }
    }
}

impl WorkspaceLock for FsWorkspaceLock {
    fn acquire(
        &mut self,
        identity: &LockIdentity,
        budget: AcquireBudget,
    ) -> Result<LockGuard, AcquireError> {
        if let Some(depth) = self.depths.get_mut(identity) {
            *depth += 1;
            return Ok(LockGuard::new(identity.clone()));
        }

        let lock_dir = FsWorkspaceLock::lock_dir_path(&self.base_dir, identity);
        let mut attempts_left = budget.max_retries;
        loop {
            match fs::create_dir(&lock_dir) {
                Ok(()) => {
                    self.write_owner_stamp(&lock_dir)
                        .map_err(|e| AcquireError::Io {
                            message: e.to_string(),
                        })?;
                    self.depths.insert(identity.clone(), 1);
                    return Ok(LockGuard::new(identity.clone()));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    if self.try_reap(&lock_dir) {
                        continue;
                    }
                    if attempts_left == 0 {
                        return Err(AcquireError::Exhausted);
                    }
                    attempts_left -= 1;
                    std::thread::sleep(budget.retry_interval);
                }
                Err(e) => {
                    return Err(AcquireError::Io {
                        message: e.to_string(),
                    });
                }
            }
        }
    }

    fn release(&mut self, guard: LockGuard) {
        let identity = guard.into_identity();
        if let Some(depth) = self.depths.get_mut(&identity)
            && *depth > 1
        {
            *depth -= 1;
            return;
        }
        // 深度台帳に無い identity の guard (ポート外で偽造された LockGuard) では lock dir に
        // 触れない — 他プロセスが保持中のロックを消す経路をここで遮断する
        // (audit_lock.qnt の release_requires_ownership に相当する防御)。
        if self.depths.remove(&identity).is_none() {
            return;
        }
        let lock_dir = FsWorkspaceLock::lock_dir_path(&self.base_dir, &identity);
        let _ = fs::remove_dir_all(&lock_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::testing::{FakeClock, FakeProcessProbe};
    use std::time::Duration;
    use tempfile::tempdir;

    fn budget(max_retries: u32, retry_interval_ms: u64) -> AcquireBudget {
        AcquireBudget::new(max_retries, Duration::from_millis(retry_interval_ms))
    }

    #[test]
    fn acquire_then_release_frees_the_lock_dir() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let mut lock = FsWorkspaceLock::new(dir.path().to_path_buf());
        let guard = lock.acquire(&identity, budget(5, 1)).unwrap();
        let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
        assert!(lock_dir.exists());
        lock.release(guard);
        assert!(!lock_dir.exists());
    }

    #[test]
    fn reentrant_acquire_increments_depth_and_release_only_frees_at_depth_zero() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let mut lock = FsWorkspaceLock::new(dir.path().to_path_buf());
        let outer = lock.acquire(&identity, budget(5, 1)).unwrap();
        let inner = lock.acquire(&identity, budget(5, 1)).unwrap();
        let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
        assert!(lock_dir.exists());
        lock.release(inner);
        assert!(lock_dir.exists(), "深度 1 では解放されない");
        lock.release(outer);
        assert!(!lock_dir.exists(), "深度 0 で解放される");
    }

    #[test]
    fn a_fresh_live_owner_is_never_reaped_and_acquire_exhausts_the_budget() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let clock = Arc::new(FakeClock::new(0));
        let probe = Arc::new(FakeProcessProbe::new());
        let mut holder = FsWorkspaceLock::with_clock_and_probe(
            dir.path().to_path_buf(),
            clock.clone(),
            probe.clone(),
        );
        let _guard = holder.acquire(&identity, budget(5, 1)).unwrap();

        let mut contender =
            FsWorkspaceLock::with_clock_and_probe(dir.path().to_path_buf(), clock, probe);
        let result = contender.acquire(&identity, budget(2, 1));
        assert_eq!(result.unwrap_err(), AcquireError::Exhausted);
    }

    #[test]
    fn a_dead_owner_is_reaped_and_the_contender_acquires() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let clock = Arc::new(FakeClock::new(0));
        let probe = Arc::new(FakeProcessProbe::new());
        let mut holder = FsWorkspaceLock::with_clock_and_probe(
            dir.path().to_path_buf(),
            clock.clone(),
            probe.clone(),
        );
        let guard = holder.acquire(&identity, budget(5, 1)).unwrap();
        // 保持者の pid を dead としてマークする (プロセスは実際には生きているが、reap 判定は
        // process_probe のみを見るので fake で「死んだ」と偽装できる)。
        probe.mark_dead(std::process::id() as i32);

        let mut contender =
            FsWorkspaceLock::with_clock_and_probe(dir.path().to_path_buf(), clock, probe);
        let new_guard = contender.acquire(&identity, budget(5, 1)).unwrap();
        assert_eq!(new_guard.identity(), &identity);

        // 元の guard は既に奪取されたロック dir を指しているが、release は自身の深度台帳
        // (holder インスタンス側) から見て一貫しているので panic しない。
        drop(guard);
        contender.release(new_guard);
    }

    #[test]
    fn acquire_maps_a_missing_base_dir_to_an_io_error() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace("x");
        let mut lock = FsWorkspaceLock::new(dir.path().join("missing-base"));
        // base_dir 不在: mkdir は ENOENT (EEXIST ではない) で失敗し Io へ写像される
        let err = lock.acquire(&identity, budget(1, 1)).unwrap_err();
        assert!(matches!(err, AcquireError::Io { .. }));
    }

    #[test]
    fn release_with_a_forged_guard_does_not_touch_a_lock_it_never_acquired() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let mut holder = FsWorkspaceLock::new(dir.path().to_path_buf());
        let _guard = holder.acquire(&identity, budget(5, 1)).unwrap();
        let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);

        let mut stranger = FsWorkspaceLock::new(dir.path().to_path_buf());
        stranger.release(LockGuard::new(identity.clone()));
        assert!(
            lock_dir.exists(),
            "非保持者の release はロック dir に触れない"
        );
    }

    #[test]
    fn a_stale_owner_past_the_threshold_is_reaped() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let clock = Arc::new(FakeClock::new(0));
        let probe = Arc::new(FakeProcessProbe::new());
        let mut holder = FsWorkspaceLock::with_clock_and_probe(
            dir.path().to_path_buf(),
            clock.clone(),
            probe.clone(),
        )
        .with_thresholds(1_000, 5_000);
        let _guard = holder.acquire(&identity, budget(5, 1)).unwrap();

        clock.advance(1_001);

        let mut contender =
            FsWorkspaceLock::with_clock_and_probe(dir.path().to_path_buf(), clock, probe)
                .with_thresholds(1_000, 5_000);
        let result = contender.acquire(&identity, budget(5, 1));
        assert!(result.is_ok());
    }
}
