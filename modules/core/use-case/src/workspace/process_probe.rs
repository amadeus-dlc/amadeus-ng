//! `ProcessProbe` ポート — 11-workspace §3。`kill(pid,0)` の ESRCH 判定 (reap 用)。
//! 実装は `core-interface-adapter` (`OsProcessProbe` が infra-io `process_probe::process_alive`
//! に委譲する実プロセス判定、`testing::FakeProcessProbe` がテスト用の注入可能な偽判定)。

/// プロセス生存判定の抽象。ロックの reap 判定 (W2: 生きた保持者から奪わない) の材料。
pub trait ProcessProbe {
    #[must_use]
    fn is_alive(&self, pid: i32) -> bool;
}
