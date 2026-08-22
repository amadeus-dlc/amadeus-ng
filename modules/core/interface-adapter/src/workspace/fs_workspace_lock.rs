//! `WorkspaceLock` の実 Gateway — mkdir-EEXIST ロック (upstream `acquireAuditLock`,
//! 03 §6.8, 11-workspace §4)。
//!
//! - lock dir: `<tmpdir>/.aidlc-audit-<md5(identity)[..8]>.lock` (md5 の 16 進先頭 8 文字 —
//!   upstream 互換, 11-workspace §9)。
//! - owner stamp: lock dir 内 `owner.json` = `{pid, startedAtMs, reapLiveOwnerAfterStale}`
//!   (キー順・インデント無し・末尾改行無し・mode `0o600` まで upstream `writeOwnerStamp`
//!   `:6832-6855` に一致)。書込は **best-effort** — 失敗しても acquire は成功する。
//! - reap 条件はスタンプの有無で 2 系統:
//!   - スタンプ済み: `core_domain::workspace::reap_eligible` (upstream `:7036-7040` の 3 枝 —
//!     死亡なら無条件、生存なら `reapLiveOwnerAfterStale` が真かつ `now - startedAtMs`
//!     が `stale_ms` (既定 600_000ms) を厳密超過したときのみ)。
//!   - 未スタンプ dir (mkdir 直後・owner.json 未書込): `grace_ms` (既定 5_000ms — 対象
//!     ディレクトリの mtime で近似する。ファイルシステム間で可搬な `birthtime` に依存
//!     しない) を超えた古いものだけがリークとして奪取対象。
//! - reap は CAS: 私有 `<lockDir>.dead.<pid>-<counter>` へ rename (失敗 = 他者が先に奪ったか
//!   保持者が解放した → `false`) → 移動先が読取時と同一の識別かを再検証 (未スタンプ判定なら
//!   「今も未スタンプ **かつ** 今も grace 超過」の 2 条件) → 一致すれば rm -rf で確定、
//!   不一致なら rename で復元 (復元が EEXIST で失敗すれば第三者が既に新しいロックを握って
//!   いるので私有 dir を破棄する)。
//! - release は識別子ごとの深度カウンタで管理し、深度 0 に戻るときのみ rm -rf する。

use core_domain::workspace::{LockIdentity, reap_eligible};
use core_use_case::workspace::{AcquireBudget, AcquireError, LockGuard, WorkspaceLock};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::clock::{Clock, SystemClock};
use crate::process_probe::{OsProcessProbe, ProcessProbe};

/// stale reap の既定閾値 (upstream `DEFAULT_LOCK_STALE_MS`, 03 §6.8)。
pub const DEFAULT_LOCK_STALE_MS: u64 = 600_000;
/// 未スタンプ dir の既定猶予 (upstream `unstampedGraceMs()`, 03 §6.8)。
pub const DEFAULT_UNSTAMPED_GRACE_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy)]
struct OwnerStamp {
    pid: i32,
    started_at_ms: u64,
    reap_live_owner_after_stale: bool,
}

/// 等価 = rename-CAS の `stampMatches` が判定する保持者の同一性。upstream
/// `aidlc-lib.ts` `stampMatches` (`:6972-6977` @ `3c3146cf`) の逐語:
///
/// ```text
/// 6972    return (
/// 6973      now.pid === judged.pid &&
/// 6974      now.startedAtMs === judged.startedAtMs &&
/// 6975      now.reapLiveOwnerAfterStale === judged.reapLiveOwnerAfterStale &&
/// 6976      now.token === judged.token
/// 6977    );
/// ```
///
/// すなわち **`reapLiveOwnerAfterStale` は upstream では同一性の一部**であり、フラグ違いは
/// 別スタンプ (= 隙間で別の acquire が起きた) と読む。`token` は `acquireActiveDirectiveLock`
/// (`:7118`) だけが渡す任意フィールドで、audit ロック経路では常に不在 (両辺 `undefined` で
/// 一致) のため本型はフィールド自体を持たない。ドメイン上の同値関係は名前付きメソッドでは
/// なく `Eq`/`PartialEq` で表現する (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-equality.md)。
impl PartialEq for OwnerStamp {
    fn eq(&self, other: &OwnerStamp) -> bool {
        self.pid == other.pid
            && self.started_at_ms == other.started_at_ms
            && self.reap_live_owner_after_stale == other.reap_live_owner_after_stale
    }
}

impl Eq for OwnerStamp {}

fn serialize_owner_stamp(stamp: &OwnerStamp) -> String {
    format!(
        "{{\"pid\":{},\"startedAtMs\":{},\"reapLiveOwnerAfterStale\":{}}}",
        stamp.pid, stamp.started_at_ms, stamp.reap_live_owner_after_stale
    )
}

/// 手書きの最小 JSON パーサ (owner.json はこの Gateway 自身が書いた 3 フィールド固定形の
/// み読む — 汎用 JSON パーサへの依存を避けるための最小実装)。`pid` / `startedAtMs` が数値
/// として読めなければ `None` (upstream `readOwnerStamp` `:6860` の `typeof === "number"`
/// ガードと同じく「未スタンプ扱い」へ落とす)。
fn parse_owner_stamp(s: &str) -> Option<OwnerStamp> {
    let pid = extract_i64_field(s, "\"pid\":")?;
    let started_at_ms = extract_i64_field(s, "\"startedAtMs\":")?;
    Some(OwnerStamp {
        pid: i32::try_from(pid).ok()?,
        started_at_ms: u64::try_from(started_at_ms).ok()?,
        reap_live_owner_after_stale: parse_reap_live_owner_after_stale(s),
    })
}

/// upstream `readOwnerStamp` (`:6866`) の `reapLiveOwnerAfterStale: parsed.reapLiveOwnerAfterStale !== false`
/// — **明示 `false` のときだけ `false`**。キー欠落・`true`・boolean 以外はすべて `true` と読む。
/// 逐語の理由 (`:6865`): *"Older stamps have no field and retain the historical over-age reaping."*
/// (キー欠落を `false` と読むと、フラグ導入前に書かれた旧スタンプが「永久に奪えないロック」に
/// なってしまう)。
fn parse_reap_live_owner_after_stale(s: &str) -> bool {
    extract_raw_field(s, "\"reapLiveOwnerAfterStale\":").is_none_or(|raw| raw != "false")
}

/// `key` に続く値トークンを `,` / `}` まで切り出して trim したもの (未出現なら `None`)。
fn extract_raw_field<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let start = s.find(key)? + key.len();
    let rest = &s[start..];
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    Some(rest[..end].trim())
}

fn extract_i64_field(s: &str, key: &str) -> Option<i64> {
    extract_raw_field(s, key)?.parse::<i64>().ok()
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

    /// mkdir に成功した直後の確定処理 — スタンプを best-effort で書き、深度台帳に登録して
    /// guard を発行する。`acquire` は通常経路と reap 後のインライン mkdir の 2 箇所から
    /// 同じ手順に入るので、その 1 実装。
    fn claim_created(&mut self, identity: &LockIdentity, lock_dir: &Path) -> LockGuard {
        self.write_owner_stamp(lock_dir);
        self.depths.insert(identity.clone(), 1);
        LockGuard::new(identity.clone())
    }

    /// owner stamp を書く。**best-effort** — 失敗しても acquire は成功させる。upstream
    /// `writeOwnerStamp` (`:6850-6853` @ `3c3146cf`) の逐語:
    ///
    /// ```text
    /// 6850    } catch {
    /// 6851      // Best-effort: a missing stamp degrades the reaper to age-only on the next
    /// 6852      // waiter (it can't read a PID), never to incorrectness.
    /// 6853      return null;
    /// ```
    ///
    /// スタンプ書込失敗で acquire を失敗させると、直前に mkdir 済みのロック dir が
    /// 誰にも保持されないまま残置され、以後の待機者全員が grace 経過まで待たされる。
    ///
    /// パーミッションは upstream の `writeFileSync(..., { mode: 0o600 })` と同じく `0o600`。
    /// `mode` は**作成時にのみ**適用される点も同じ意味論だが、owner.json は直前に mkdir した
    /// 空 dir の中にあるので既存ファイルへの上書きは起こらない。
    fn write_owner_stamp(&self, lock_dir: &Path) {
        let stamp = OwnerStamp {
            pid: std::process::id() as i32,
            started_at_ms: self.clock.now_ms(),
            reap_live_owner_after_stale: true,
        };
        let _: io::Result<()> = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(lock_dir.join("owner.json"))
            .and_then(|mut f| f.write_all(serialize_owner_stamp(&stamp).as_bytes()));
    }

    /// 未スタンプ dir の年齢が猶予を超えているか (upstream `lockDirMtimeMs` `:6937-6943` +
    /// `unstampedGraceMs` `:6925-6932`)。stat できない (消えた) 場合は「奪わない」側へ倒す。
    fn unstamped_is_over_grace(&self, dir: &Path) -> bool {
        match fs::metadata(dir).and_then(|m| m.modified()) {
            Ok(modified) => {
                let age = self
                    .clock
                    .now_ms()
                    .saturating_sub(system_time_to_ms(modified));
                age > self.grace_ms
            }
            Err(_) => false,
        }
    }

    /// reap を試みる。`true` を返したら呼出側は mkdir を 1 回だけ即再試行してよい
    /// (それでも周回の予算は消費する — `acquire` を参照)。
    fn try_reap(&mut self, lock_dir: &Path) -> bool {
        let owner_path = lock_dir.join("owner.json");
        let stamp_before = fs::read_to_string(&owner_path)
            .ok()
            .and_then(|s| parse_owner_stamp(&s));

        let reapable = match &stamp_before {
            Some(stamp) => {
                // reap 適格判定はドメインの単一実装 (lock_protocol::reap_eligible) に委譲する。
                // 保持者が宣言した reapLiveOwnerAfterStale もそのまま渡す — 生存保持者が
                // false を宣言していれば経過時間によらず奪わない (upstream :7037)。
                let alive = self.process_probe.is_alive(stamp.pid);
                let age = self.clock.now_ms().saturating_sub(stamp.started_at_ms);
                reap_eligible(alive, stamp.reap_live_owner_after_stale, age, self.stale_ms)
            }
            None => self.unstamped_is_over_grace(lock_dir),
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
            // upstream :7047-7049 `catch { return false; }` — 別の待機者が先に奪ったか、
            // 保持者が解放したかで rename 対象が消えている。ここで `true` を返すと (旧実装)
            // rename が恒常的に失敗する条件 (EACCES 等) で「reap 成功」を主張し続けてしまう。
            return false;
        }

        let stamp_after = fs::read_to_string(dead_dir.join("owner.json"))
            .ok()
            .and_then(|s| parse_owner_stamp(&s));

        let matches = match (&stamp_before, &stamp_after) {
            // upstream `stampMatches(dir, null)` (:6962-6970): 未スタンプ判定の再検証は
            // 「今も未スタンプ」だけでは足りず「今も grace 超過」も要る。renameSync は inode の
            // mtime を保存する (逐語 :6952-6954 *"renameSync preserves the inode's mtime, so a
            // genuine old leak keeps its over-grace mtime through the move"*) ので、本物の古い
            // リークは移動後も超過のまま一致する。一方、隙間で競合者が作り直した dir は mtime が
            // リセットされて猶予内 → 不一致 → 復元され、生きた保持者は奪われない。
            (None, None) => self.unstamped_is_over_grace(&dead_dir),
            (Some(a), Some(b)) => a == b,
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
        let mut attempts_left = budget.max_retries();
        loop {
            match fs::create_dir(&lock_dir) {
                Ok(()) => return Ok(self.claim_created(identity, &lock_dir)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    // upstream `acquireAuditLock` (:7156-7163): reap に成功したら `continue`
                    // せず、**同一周回でインライン mkdir を 1 回だけ**試みる。成功すれば sleep
                    // を 1 回省けるが、失敗したらそのまま下へ落ちて予算を消費する。つまり
                    // 「reap 成功で予算を温存し続ける」ことはできず、ループ上限
                    // (`max_retries + 1` 回の mkdir) が常に有効 = ライブロックしない。
                    if self.try_reap(&lock_dir) && fs::create_dir(&lock_dir).is_ok() {
                        return Ok(self.claim_created(identity, &lock_dir));
                    }
                    if attempts_left == 0 {
                        return Err(AcquireError::Exhausted);
                    }
                    attempts_left -= 1;
                    std::thread::sleep(budget.retry_interval());
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
    use crate::clock::FakeClock;
    use crate::process_probe::FakeProcessProbe;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;
    use std::time::Duration;
    use tempfile::tempdir;

    fn budget(max_retries: u32, retry_interval_ms: u64) -> AcquireBudget {
        AcquireBudget::new(max_retries, Duration::from_millis(retry_interval_ms))
    }

    /// 台本どおりの時刻を 1 呼出ごとに返す偽時計 (使い切ったら最後の値を返し続ける)。
    /// `try_reap` は未スタンプ経路で now を 2 回読む — ① rename 前の grace 判定、
    /// ② rename 後の CAS 再検証 — ので、この 2 回に別の値を与えれば
    /// 「隙間で作り直されて mtime がリセットされた dir」を決定的に再現できる。
    #[derive(Debug)]
    struct ScriptedClock {
        remaining: Mutex<Vec<u64>>,
        last: u64,
    }

    impl ScriptedClock {
        fn new(script: &[u64]) -> ScriptedClock {
            let mut remaining = script.to_vec();
            remaining.reverse(); // pop() で台本の先頭から取り出す
            ScriptedClock {
                remaining: Mutex::new(remaining),
                last: script.last().copied().unwrap_or(0),
            }
        }
    }

    impl Clock for ScriptedClock {
        fn now_ms(&self) -> u64 {
            let mut remaining = self
                .remaining
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            remaining.pop().unwrap_or(self.last)
        }
    }

    /// `now_ms()` の副作用でロック dir を書込不能にする偽時計。`write_owner_stamp` は
    /// スタンプを書く直前に now を読むので、書込失敗を決定的に起こせる。
    #[derive(Debug)]
    struct StampSabotagingClock {
        lock_dir: PathBuf,
    }

    impl Clock for StampSabotagingClock {
        fn now_ms(&self) -> u64 {
            if let Ok(meta) = fs::metadata(&self.lock_dir) {
                let mut perms = meta.permissions();
                perms.set_mode(0o500); // 読み取り・実行のみ = 中にファイルを作れない
                let _ = fs::set_permissions(&self.lock_dir, perms);
            }
            0
        }
    }

    /// `now_ms()` の副作用でロック dir を作り直す偽時計。「reap には成功するが直後の
    /// インライン mkdir は必ず EEXIST」という最悪ケースを決定的に再現する。
    #[derive(Debug)]
    struct DirRecreatingClock {
        lock_dir: PathBuf,
        now_ms: u64,
    }

    impl Clock for DirRecreatingClock {
        fn now_ms(&self) -> u64 {
            let _ = fs::create_dir(&self.lock_dir);
            self.now_ms
        }
    }

    /// `now_ms()` の副作用でロック dir を消す偽時計。奪取適格と判定した直後に dir が
    /// 消える = 「別の待機者が先に奪った / 保持者が解放した」状況を決定的に作り、
    /// CAS の rename を ENOENT で失敗させる。
    #[derive(Debug)]
    struct DirVanishingClock {
        lock_dir: PathBuf,
        now_ms: u64,
    }

    impl Clock for DirVanishingClock {
        fn now_ms(&self) -> u64 {
            let _ = fs::remove_dir_all(&self.lock_dir);
            self.now_ms
        }
    }

    /// `<base>/*.dead.*` の私有 dir が残っていないこと (reap の後片付けの検査)。
    fn dead_dirs(base: &Path) -> Vec<String> {
        fs::read_dir(base)
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|name| name.contains(".dead."))
            .collect()
    }

    #[test]
    fn owner_stamp_equality_compares_every_field_upstream_stamp_matches_compares() {
        let a = OwnerStamp {
            pid: 42,
            started_at_ms: 1_000,
            reap_live_owner_after_stale: true,
        };
        assert_eq!(a, OwnerStamp { ..a });
        // upstream stampMatches (:6975) は reapLiveOwnerAfterStale も比較する —
        // フラグ違いは「隙間で別の acquire が起きた」= 別スタンプ (CAS 不一致 → 復元)。
        assert_ne!(
            a,
            OwnerStamp {
                reap_live_owner_after_stale: false,
                ..a
            },
            "ポリシーフラグ違いは upstream では別スタンプ"
        );
        // pid / startedAtMs 違いも当然に別スタンプ
        assert_ne!(a, OwnerStamp { pid: 43, ..a });
        assert_ne!(
            a,
            OwnerStamp {
                started_at_ms: 2_000,
                ..a
            }
        );
    }

    #[test]
    fn the_reap_flag_is_read_as_upstream_does_missing_means_true() {
        // upstream readOwnerStamp (:6866) の `parsed.reapLiveOwnerAfterStale !== false`
        let flag = |s: &str| parse_owner_stamp(s).map(|st| st.reap_live_owner_after_stale);

        // キー欠落 = true (フラグ導入前の旧スタンプは従来どおり over-age reap の対象)
        assert_eq!(flag(r#"{"pid":7,"startedAtMs":1}"#), Some(true));
        // 明示 true / false
        assert_eq!(
            flag(r#"{"pid":7,"startedAtMs":1,"reapLiveOwnerAfterStale":true}"#),
            Some(true)
        );
        assert_eq!(
            flag(r#"{"pid":7,"startedAtMs":1,"reapLiveOwnerAfterStale":false}"#),
            Some(false)
        );
        // boolean 以外は `!== false` が真なので true
        assert_eq!(
            flag(r#"{"pid":7,"startedAtMs":1,"reapLiveOwnerAfterStale":"false"}"#),
            Some(true)
        );
        assert_eq!(
            flag(r#"{"pid":7,"startedAtMs":1,"reapLiveOwnerAfterStale":0}"#),
            Some(true)
        );
        // pid / startedAtMs が数値でなければ未スタンプ扱い (upstream :6860 のガード)
        assert_eq!(flag(r#"{"startedAtMs":1}"#), None);
        assert_eq!(flag(r#"{"pid":"7","startedAtMs":1}"#), None);
    }

    #[test]
    fn the_serialized_stamp_round_trips_through_the_parser() {
        let stamp = OwnerStamp {
            pid: 4_242,
            started_at_ms: 1_755_800_000_000,
            reap_live_owner_after_stale: false,
        };
        let json = serialize_owner_stamp(&stamp);
        assert_eq!(
            json,
            r#"{"pid":4242,"startedAtMs":1755800000000,"reapLiveOwnerAfterStale":false}"#
        );
        assert_eq!(parse_owner_stamp(&json), Some(stamp));
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

    /// A3 — CAS 再検証: 未スタンプ dir を掴んだあと「今も grace 超過」を再検査する
    /// (upstream `stampMatches(dir, null)` `:6962-6970`)。rename の隙間で競合者が dir を作り
    /// 直すと mtime がリセットされて猶予内に戻るため、私有 dir は元の名前へ**復元**され、
    /// 生きた保持者は奪われない。
    #[test]
    fn a_grabbed_unstamped_dir_is_restored_when_it_is_no_longer_over_grace() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
        fs::create_dir(&lock_dir).unwrap(); // 未スタンプ dir (owner.json 無し)

        let mtime = system_time_to_ms(fs::metadata(&lock_dir).unwrap().modified().unwrap());
        let grace = 5_000;
        // ① rename 前 = grace 超過 (奪取判定は成立する) → ② rename 後 = 猶予内
        //    (= 隙間で作り直された新鮮な dir を掴んだ状況)。
        let clock = Arc::new(ScriptedClock::new(&[mtime + grace + 1, mtime]));
        let mut contender = FsWorkspaceLock::with_clock_and_probe(
            dir.path().to_path_buf(),
            clock,
            Arc::new(FakeProcessProbe::new()),
        )
        .with_thresholds(1_000, grace);

        let result = contender.acquire(&identity, budget(1, 1));
        assert_eq!(
            result.unwrap_err(),
            AcquireError::Exhausted,
            "再検査で猶予内に戻った dir は奪ってはならない"
        );
        assert!(
            lock_dir.exists(),
            "CAS 再検証に失敗した dir は元の名前へ復元される"
        );
        assert!(
            dead_dirs(dir.path()).is_empty(),
            "私有 .dead.* dir が残置されている"
        );
    }

    /// A4 — CAS の rename に失敗したら reap は成功を主張しない
    /// (upstream `:7047-7049` `catch { return false; }` — *"another waiter already reclaimed
    /// (or the holder released)"*)。`try_reap` の戻り値そのものが契約なので直接叩く。
    #[test]
    fn try_reap_reports_failure_when_the_cas_rename_fails() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
        fs::create_dir(&lock_dir).unwrap(); // 未スタンプ dir

        // 適格判定 (metadata → now) の「now」を読んだ瞬間に dir が消える → rename が ENOENT。
        let clock = Arc::new(DirVanishingClock {
            lock_dir: lock_dir.clone(),
            now_ms: 1_000_000_000_000_000,
        });
        let mut lock = FsWorkspaceLock::with_clock_and_probe(
            dir.path().to_path_buf(),
            clock,
            Arc::new(FakeProcessProbe::new()),
        )
        .with_thresholds(1_000, 0);

        assert!(
            !lock.try_reap(&lock_dir),
            "rename に失敗した reap は成功を主張しない"
        );
        assert!(
            dead_dirs(dir.path()).is_empty(),
            "私有 .dead.* dir が残置されている"
        );
    }

    /// A5 — reap が毎回成功しても予算は消費される。旧実装の `continue` (予算非消費) では
    /// 「reap 成功 → mkdir が EEXIST」を繰り返すとループが終わらなかった。
    /// 別スレッド + タイムアウトで回し、ライブロックを hang ではなく失敗として観測する。
    ///
    /// 仕掛け: 時計は rename **後**の grace 再検査 (A3) でもう一度読まれるので、その副作用で
    /// ロック dir を作り直す。したがって本テストは A3 の再検査経路にも依存している。
    #[test]
    fn a_repeatedly_reapable_lock_exhausts_the_budget_instead_of_livelocking() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
        fs::create_dir(&lock_dir).unwrap(); // 未スタンプ dir

        // 時刻を読むたびにロック dir を作り直す = reap 直後のインライン mkdir が必ず EEXIST。
        let clock = Arc::new(DirRecreatingClock {
            lock_dir,
            now_ms: 1_000_000_000_000_000,
        });
        let base = dir.path().to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut lock = FsWorkspaceLock::with_clock_and_probe(
                base,
                clock,
                Arc::new(FakeProcessProbe::new()),
            )
            .with_thresholds(1_000, 0);
            let _ = tx.send(lock.acquire(&identity, budget(3, 1)));
        });

        let result = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("reap が毎回成功しても周回の予算は消費される — 終わらなければライブロック");
        assert_eq!(result.unwrap_err(), AcquireError::Exhausted);
    }

    /// A6 — owner stamp の書込は best-effort (upstream `writeOwnerStamp` `:6850-6853`)。
    /// 書けなくても acquire は成功し、ロック dir は保持されたまま (旧実装は
    /// `AcquireError::Io` を返し、作成済みの dir を誰にも保持されないまま残置していた)。
    #[test]
    fn acquire_succeeds_even_when_the_owner_stamp_cannot_be_written() {
        let dir = tempdir().unwrap();
        let identity = LockIdentity::for_workspace(dir.path().to_str().unwrap());
        let lock_dir = FsWorkspaceLock::lock_dir_path(dir.path(), &identity);
        let clock = Arc::new(StampSabotagingClock {
            lock_dir: lock_dir.clone(),
        });
        let mut lock = FsWorkspaceLock::with_clock_and_probe(
            dir.path().to_path_buf(),
            clock,
            Arc::new(FakeProcessProbe::new()),
        );

        let guard = lock
            .acquire(&identity, budget(1, 1))
            .expect("スタンプ書込の失敗は acquire を失敗させない");
        assert!(lock_dir.exists(), "ロックは保持されたまま");
        assert!(
            !lock_dir.join("owner.json").exists(),
            "前提の確認: スタンプは実際に書けていない (未スタンプ保持)"
        );

        // 後片付け: 書込不能のままだと tempdir の削除が失敗する
        let mut perms = fs::metadata(&lock_dir).unwrap().permissions();
        perms.set_mode(0o700);
        fs::set_permissions(&lock_dir, perms).unwrap();
        lock.release(guard);
        assert!(!lock_dir.exists(), "未スタンプでも release は効く");
    }
}
