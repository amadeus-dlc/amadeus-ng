//! `WorkspaceLock` ポート — 11-workspace §3 `LockService` の最小面。acquire（予算つき）/
//! 再入 / 解放 / reap を供給する。identity は `core_domain::workspace::lock_identity::LockIdentity`
//! (upstream `auditLockIdentity` — 2/3 成分の keying 規約はドメイン層が既に強制している)。
//! 実装は `core-interface-adapter` の Gateway (`fs_workspace_lock`)。

use core_domain::workspace::lock_identity::LockIdentity;
use std::time::Duration;

/// acquire の retry 予算 (回数 × 間隔)。reap による即時再試行は予算を消費しない
/// (upstream `acquireAuditLock(maxRetries, retryMs)` — 03 §6.8)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcquireBudget {
    max_retries: u32,
    retry_interval: Duration,
}

impl AcquireBudget {
    #[must_use]
    pub const fn new(max_retries: u32, retry_interval: Duration) -> AcquireBudget {
        AcquireBudget {
            max_retries,
            retry_interval,
        }
    }

    /// reap による即時再試行を除く retry 回数。
    #[must_use]
    pub const fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// retry 1 回あたりの待機時間。
    #[must_use]
    pub const fn retry_interval(&self) -> Duration {
        self.retry_interval
    }
}

/// 保持中のロックを表す不透明トークン。再入時は呼出しごとに 1 つ返す (深度 1 単位)。
/// `release` に渡すまで生存し、`Clone`/`Copy` は実装しない (二重解放を型で抑止する)。
///
/// この型は `WorkspaceLock` 実装が `acquire` 内でのみ構築すべき値であり、`new` は実装向けの
/// 構築口として公開する (ポートの外形上の制約であり、型システムでは強制しない)。
#[derive(Debug)]
#[must_use = "解放されないロックは release() に渡すこと"]
pub struct LockGuard {
    identity: LockIdentity,
}

impl LockGuard {
    pub const fn new(identity: LockIdentity) -> LockGuard {
        LockGuard { identity }
    }

    #[must_use]
    pub const fn identity(&self) -> &LockIdentity {
        &self.identity
    }

    #[must_use]
    pub fn into_identity(self) -> LockIdentity {
        self.identity
    }
}

/// acquire 失敗の理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireError {
    /// retry 予算を使い切っても取得できなかった
    /// (upstream `Failed to acquire audit lock after retries`)。
    Exhausted,
    /// mkdir 等の I/O エラー (EEXIST 以外)。
    Io { message: String },
}

/// ワークスペースロック (mkdir-EEXIST ミューテックス) を供給するポート。
pub trait WorkspaceLock {
    /// 予算内でロックを取得する。identity ごとに再入可 (深度カウンタ) — 同一 identity への
    /// 再帰的 acquire は新しい `LockGuard` を返す (奪取ではない)。
    ///
    /// # Errors
    ///
    /// 予算を使い切っても取得できなければ `AcquireError::Exhausted`。mkdir 等の I/O エラーは
    /// `AcquireError::Io`。
    fn acquire(
        &mut self,
        identity: &LockIdentity,
        budget: AcquireBudget,
    ) -> Result<LockGuard, AcquireError>;

    /// 保持中のロックを解放する。深度が 0 に戻るときのみ実際に手放す (再入的な release)。
    fn release(&mut self, guard: LockGuard);
}
