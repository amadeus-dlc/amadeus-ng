//! `LockProtocol` — 監査ロックの純粋ステップ関数 (単一 identity)。プロセスは `0..process_count`
//! のインデックスで扱う (モデルは `PROCS = 0.to(2)` すなわち N=3)。
//!
//! 意味論の形式的正本は `formal/workspace/audit_lock.qnt` (v3 — green・mutation 10/10・
//! witness 7/7)。本実装は同モデルの純粋ステップ関数であり、ITF 準拠テスト
//! (`tests/audit_lock_conformance.rs`) がモデルトレースを再生して突き合わせる (ADR 0003 決定 5)。
//!
//! モデル型 ↔ フィールド対応 (audit_lock.qnt ヘッダの対応表そのもの):
//!   `alive`      ↔ プロセス生存 (`kill(pid,0)` の ESRCH 判定の抽象)
//!   `dir_owner`  ↔ mkdir-EEXIST ロックディレクトリの保持者 (`None` = 不在)
//!   `depth`      ↔ `withAuditLock` の identity ごとの再入深度カウンタ
//!   `held_ticks` ↔ owner.json スタンプ経過時間の抽象 (`threshold` = 既定 10 分の抽象化)
//!   `audit_len`  ↔ 監査シャードの行数 (追記のみ)
//!   `state_ver`  ↔ 状態ファイルの版 (audit-first: 監査 emit 済みトランザクション内でのみ進む)
//!   `pending`    ↔ クリティカルセクション内「監査 emit 済み・state 未書込」の中間状態 (R6 の窓)
//!
//! ガード不成立はモデルの enabled 条件と同型で `Err` を返す (アクションが発火しない)。

/// コマンドのガード不成立 (モデルの enabled 条件と同型)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockError {
    /// 指定プロセスインデックスがモデル外 (`process_count` 以上)。
    UnknownProcess(usize),
    /// 指定プロセスは生存していない (crash 済み)。
    ProcessNotAlive(usize),
    /// ロックは既に他プロセスが保持中 (`acquire` の mkdir-EEXIST 相当)。
    LockHeld,
    /// ロックは誰にも保持されていない。
    LockNotHeld,
    /// 呼出プロセスはロック保持者ではない。
    NotOwner,
    /// 監査 emit 済み・state 未書込のトランザクションが既に開いている。
    TransactionAlreadyOpen,
    /// 監査 emit 済みトランザクションが開いていない (state 書込は emit 後のみ許可 — audit-first)。
    NoOpenTransaction,
    /// reap は自分自身が保持しているロックには適用できない。
    CannotReapOwnLock,
    /// reap 対象の保持者はまだ生存中かつ閾値以内 (奪取不可 — 厳密超過のみ許容)。
    ReapNotEligible,
}

/// 監査ロックの並行モデル (`audit_lock.qnt` v3) の純粋ステップ関数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockProtocol {
    alive: Vec<bool>,
    dir_owner: Option<usize>,
    depth: u32,
    held_ticks: u32,
    audit_len: u64,
    state_ver: u64,
    pending: bool,
    threshold: u32,
}

/// reap 適格性 (audit_lock.qnt `actReap` のガードそのもの) — 死んだ所有者、または保持経過が
/// しきい値を**厳密に**超えた保持者だけが奪取可能。W2 (生きた閾値以内の保持者からは決して
/// 奪わない) の単一実装であり、モデル (`LockProtocol::reap`) と実 Gateway
/// (`FsWorkspaceLock::try_reap`) の両方がこの関数を呼ぶ — 境界規約 (`>`) を 2 箇所に
/// 重複させないための Tell-Don't-Ask 装置。時間の単位は呼出側で揃える (モデルは tick、
/// 実 Gateway は ms)。
#[must_use]
pub const fn reap_eligible(owner_alive: bool, held_elapsed: u64, stale_threshold: u64) -> bool {
    !owner_alive || held_elapsed > stale_threshold
}

impl LockProtocol {
    /// `process_count` 個のプロセス (全員生存) と閾値でロックを初期化する
    /// (`audit_lock.qnt` の `init` — `dirOwner=-1`, `depth=0`, その他すべて 0/false)。
    #[must_use]
    pub fn new(process_count: usize, threshold: u32) -> Self {
        LockProtocol {
            alive: vec![true; process_count],
            dir_owner: None,
            depth: 0,
            held_ticks: 0,
            audit_len: 0,
            state_ver: 0,
            pending: false,
            threshold,
        }
    }

    const fn validate_process(&self, p: usize) -> Result<(), LockError> {
        if p >= self.alive.len() {
            return Err(LockError::UnknownProcess(p));
        }
        Ok(())
    }

    // ---- 観測 (read model) ----

    /// モデルのプロセス数 N。コマンドが受け付けるインデックスの定義域は `0..process_count`
    /// (外は `UnknownProcess`)。
    #[must_use]
    pub const fn process_count(&self) -> usize {
        self.alive.len()
    }

    /// プロセス `p` の生存 (`kill(pid,0)` の ESRCH 判定の抽象)。`p` は `0..process_count` の範囲で
    /// あること。
    #[must_use]
    pub fn alive(&self, p: usize) -> bool {
        self.alive[p]
    }

    /// mkdir-EEXIST ロックディレクトリの保持者。`None` = 不在 (モデルの `dirOwner = -1`)。
    /// 保持者の生存は含意しない — crash 後も stale なまま残る (W5)。
    #[must_use]
    pub const fn dir_owner(&self) -> Option<usize> {
        self.dir_owner
    }

    /// `withAuditLock` の identity ごとの再入深度カウンタ。`dir_owner` が `None` ⇔ 0、
    /// `Some` ⇒ 1 以上 (`depth_consistent`)。
    #[must_use]
    pub const fn depth(&self) -> u32 {
        self.depth
    }

    /// 現保持者の保持経過。単位は tick (owner.json スタンプ経過時間の抽象。実 Gateway では ms)。
    /// `threshold` との比較にのみ意味がある値で、取得・reap で 0 に戻る。
    #[must_use]
    pub const fn held_ticks(&self) -> u32 {
        self.held_ticks
    }

    /// 監査シャードの行数。追記のみのため単調非減少 (減る遷移はモデルに存在しない)。
    #[must_use]
    pub const fn audit_len(&self) -> u64 {
        self.audit_len
    }

    /// 状態ファイルの版。audit-first により、監査 emit 済みトランザクション内でのみ進む (W1)。
    #[must_use]
    pub const fn state_ver(&self) -> u64 {
        self.state_ver
    }

    /// 「監査 emit 済み・state 未書込」の中間状態 (R6 の窓) が開いているか。真のときのみ
    /// `transition_write` が通る。
    #[must_use]
    pub const fn pending(&self) -> bool {
        self.pending
    }

    /// stale 判定のしきい値。単位は `held_ticks` と同じ tick で、**厳密超過** (`>`) のみが
    /// 生きた保持者からの reap を許す (W2)。
    #[must_use]
    pub const fn threshold(&self) -> u32 {
        self.threshold
    }

    // ---- コマンド (アクションと 1:1) ----

    /// mkdir-EEXIST: ロック不在のときのみ成功する。取得で `depth=1, held_ticks=0, pending=false`
    /// に初期化する (`actAcquire`)。
    /// # Errors
    ///
    /// プロセス未知 (`UnknownProcess`)・非生存 (`ProcessNotAlive`)・ロック既保持 (`LockHeld`) を拒否する。
    pub fn acquire(&mut self, p: usize) -> Result<(), LockError> {
        self.validate_process(p)?;
        if !self.alive[p] {
            return Err(LockError::ProcessNotAlive(p));
        }
        if self.dir_owner.is_some() {
            return Err(LockError::LockHeld);
        }
        self.dir_owner = Some(p);
        self.depth = 1;
        self.held_ticks = 0;
        self.pending = false;
        Ok(())
    }

    /// 再入: 同一 identity の保持者はネスト可 (深度カウンタを増やす — `actReenter`)。
    /// # Errors
    ///
    /// プロセス未知・非生存・非保持者 (`NotOwner`) を拒否する。
    pub fn reenter(&mut self, p: usize) -> Result<(), LockError> {
        self.validate_process(p)?;
        if !self.alive[p] {
            return Err(LockError::ProcessNotAlive(p));
        }
        if self.dir_owner != Some(p) {
            return Err(LockError::NotOwner);
        }
        self.depth += 1;
        Ok(())
    }

    /// audit-first の第 1 段: 監査 emit 成功。state はまだ書かれない (`pending` が開く —
    /// `actTransitionEmit`)。
    /// # Errors
    ///
    /// プロセス未知・非生存・非保持者・既にトランザクションが開いている場合
    /// (`TransactionAlreadyOpen`) を拒否する。
    pub fn transition_emit(&mut self, p: usize) -> Result<(), LockError> {
        self.validate_process(p)?;
        if !self.alive[p] {
            return Err(LockError::ProcessNotAlive(p));
        }
        if self.dir_owner != Some(p) {
            return Err(LockError::NotOwner);
        }
        if self.pending {
            return Err(LockError::TransactionAlreadyOpen);
        }
        self.audit_len += 1;
        self.pending = true;
        Ok(())
    }

    /// audit-first の第 2 段: state 書込 (emit 済みトランザクション内でのみ — `actTransitionWrite`)。
    /// # Errors
    ///
    /// プロセス未知・非生存・非保持者・開いたトランザクションがない場合 (`NoOpenTransaction`) を拒否する。
    pub fn transition_write(&mut self, p: usize) -> Result<(), LockError> {
        self.validate_process(p)?;
        if !self.alive[p] {
            return Err(LockError::ProcessNotAlive(p));
        }
        if self.dir_owner != Some(p) {
            return Err(LockError::NotOwner);
        }
        if !self.pending {
            return Err(LockError::NoOpenTransaction);
        }
        self.state_ver += 1;
        self.pending = false;
        Ok(())
    }

    /// 監査 emit が throw した場合: 何も起きない (audit-first — state は書かれない)。ガードは
    /// `transition_emit` と同一 (保持者・not pending) — 成立すれば状態は不変のまま成功する
    /// (`actTransitionEmitFail`)。
    /// # Errors
    ///
    /// プロセス未知・非生存・非保持者・既にトランザクションが開いている場合を拒否する。
    pub fn transition_emit_fail(&mut self, p: usize) -> Result<(), LockError> {
        self.validate_process(p)?;
        if !self.alive[p] {
            return Err(LockError::ProcessNotAlive(p));
        }
        if self.dir_owner != Some(p) {
            return Err(LockError::NotOwner);
        }
        if self.pending {
            return Err(LockError::TransactionAlreadyOpen);
        }
        Ok(())
    }

    /// 解放: 深度が 0 に戻ったときのみロックを手放す (exit ハンドラ経由も同一視 — `actRelease`)。
    /// # Errors
    ///
    /// プロセス未知・非生存・非保持者を拒否する。
    pub fn release(&mut self, p: usize) -> Result<(), LockError> {
        self.validate_process(p)?;
        if !self.alive[p] {
            return Err(LockError::ProcessNotAlive(p));
        }
        if self.dir_owner != Some(p) {
            return Err(LockError::NotOwner);
        }
        let was_outermost = self.depth == 1;
        self.depth -= 1;
        if was_outermost {
            self.dir_owner = None;
            self.held_ticks = 0;
            self.pending = false;
        }
        Ok(())
    }

    /// クラッシュ: exit ハンドラなしの消滅。**ロック状態は一切変えない** (stale 化 — R6 の核心 —
    /// `actCrash`)。
    /// # Errors
    ///
    /// プロセス未知・既に非生存 (二重クラッシュ) を拒否する。
    pub fn crash(&mut self, p: usize) -> Result<(), LockError> {
        self.validate_process(p)?;
        if !self.alive[p] {
            return Err(LockError::ProcessNotAlive(p));
        }
        self.alive[p] = false;
        Ok(())
    }

    /// 保持中の時間経過 (owner スタンプの経過時間の抽象 — `actTick`)。
    /// # Errors
    ///
    /// ロックが不在のとき (`LockNotHeld`) を拒否する。
    pub const fn tick(&mut self) -> Result<(), LockError> {
        if self.dir_owner.is_none() {
            return Err(LockError::LockNotHeld);
        }
        self.held_ticks += 1;
        Ok(())
    }

    /// reap (rename CAS の抽象): 死んだ所有者 or 閾値**厳密超過**のみ。生きている閾値以内の
    /// 保持者からは決して奪わない (`actReap`)。
    /// # Errors
    ///
    /// プロセス未知・非生存・ロック不在 (`LockNotHeld`)・自ロックへの reap
    /// (`CannotReapOwnLock`)・保持者が生存かつ閾値以内 (`ReapNotEligible`) を拒否する。
    pub fn reap(&mut self, p: usize) -> Result<(), LockError> {
        self.validate_process(p)?;
        if !self.alive[p] {
            return Err(LockError::ProcessNotAlive(p));
        }
        let owner = self.dir_owner.ok_or(LockError::LockNotHeld)?;
        if owner == p {
            return Err(LockError::CannotReapOwnLock);
        }
        let owner_alive = self.alive[owner];
        if !reap_eligible(
            owner_alive,
            u64::from(self.held_ticks),
            u64::from(self.threshold),
        ) {
            return Err(LockError::ReapNotEligible);
        }
        self.dir_owner = Some(p);
        self.depth = 1;
        self.held_ticks = 0;
        self.pending = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 例示テスト (ユビキタス言語の文書化 — 単体テストは基本 PBT というオーナー規約に
    //      対する少数の例外) ----

    #[test]
    fn acquire_blocks_a_second_process_until_release() {
        let mut lock = LockProtocol::new(3, 3);
        lock.acquire(0).unwrap();
        assert_eq!(lock.acquire(1), Err(LockError::LockHeld));
        lock.release(0).unwrap();
        lock.acquire(1).unwrap();
        assert_eq!(lock.dir_owner(), Some(1));
    }

    #[test]
    fn reenter_increases_depth_and_release_only_frees_at_depth_zero() {
        let mut lock = LockProtocol::new(3, 3);
        lock.acquire(0).unwrap();
        lock.reenter(0).unwrap();
        assert_eq!(lock.depth(), 2);
        lock.release(0).unwrap();
        assert_eq!(lock.depth(), 1);
        assert_eq!(lock.dir_owner(), Some(0)); // まだ保持中 (再入解放は手放さない)
        lock.release(0).unwrap();
        assert_eq!(lock.depth(), 0);
        assert_eq!(lock.dir_owner(), None);
    }

    #[test]
    fn transition_write_without_emit_is_rejected_audit_first() {
        let mut lock = LockProtocol::new(3, 3);
        lock.acquire(0).unwrap();
        assert_eq!(lock.transition_write(0), Err(LockError::NoOpenTransaction));
        lock.transition_emit(0).unwrap();
        lock.transition_write(0).unwrap();
        assert_eq!(lock.state_ver(), 1);
        assert!(!lock.pending());
    }

    #[test]
    fn reap_refuses_live_holder_below_or_at_threshold_and_succeeds_after_strict_excess() {
        let mut lock = LockProtocol::new(3, 3); // threshold=3 (モデルと同一)
        lock.acquire(0).unwrap();
        assert_eq!(lock.reap(1), Err(LockError::ReapNotEligible)); // held_ticks=0
        for _ in 0..3 {
            lock.tick().unwrap();
        }
        assert_eq!(lock.held_ticks(), 3);
        // 閾値ちょうど (3) はまだ「厳密超過」ではない — 拒否されたままであること
        assert_eq!(lock.reap(1), Err(LockError::ReapNotEligible));
        lock.tick().unwrap();
        assert_eq!(lock.held_ticks(), 4);
        lock.reap(1).unwrap();
        assert_eq!(lock.dir_owner(), Some(1));
        assert_eq!(lock.depth(), 1);
        assert_eq!(lock.held_ticks(), 0);
    }

    #[test]
    fn crash_leaves_lock_state_untouched_and_a_dead_owner_is_reapable_immediately() {
        let mut lock = LockProtocol::new(3, 3);
        lock.acquire(0).unwrap();
        lock.crash(0).unwrap();
        assert_eq!(lock.dir_owner(), Some(0)); // stale 化 — ロックは残る
        assert_eq!(lock.depth(), 1);
        assert!(!lock.alive(0));
        // 死んだ所有者からは held_ticks によらず即 reap 可能
        lock.reap(1).unwrap();
        assert_eq!(lock.dir_owner(), Some(1));
    }

    #[test]
    fn reap_eligibility_is_dead_owner_or_strict_threshold_excess() {
        // 生存 ∧ ちょうど閾値 → 不適格 (厳密超過のみ)
        assert!(!reap_eligible(true, 3, 3));
        assert!(reap_eligible(true, 4, 3));
        // 死んだ所有者は経過によらず即適格
        assert!(reap_eligible(false, 0, 3));
    }

    #[test]
    fn commands_reject_a_process_index_outside_the_model() {
        let mut lock = LockProtocol::new(3, 3);
        assert_eq!(lock.acquire(3), Err(LockError::UnknownProcess(3)));
        assert_eq!(lock.threshold(), 3);
    }

    // ---- PBT: ランダムコマンド列の下で audit_lock.qnt の不変条件の Rust 版が全ステップ成立する
    //      (単体テストは基本 PBT — オーナー規約) ----

    use proptest::prelude::*;

    const MODEL_PROCESS_COUNT: usize = 3; // audit_lock.qnt PROCS = 0.to(2)
    const MODEL_THRESHOLD: u32 = 3; // audit_lock.qnt THRESHOLD

    #[derive(Debug, Clone, Copy)]
    enum Cmd {
        Acquire(usize),
        Reenter(usize),
        TransitionEmit(usize),
        TransitionWrite(usize),
        TransitionEmitFail(usize),
        Release(usize),
        Crash(usize),
        Tick,
        Reap(usize),
    }

    fn cmd_strategy(n: usize) -> impl Strategy<Value = Cmd> {
        prop_oneof![
            (0..n).prop_map(Cmd::Acquire),
            (0..n).prop_map(Cmd::Reenter),
            (0..n).prop_map(Cmd::TransitionEmit),
            (0..n).prop_map(Cmd::TransitionWrite),
            (0..n).prop_map(Cmd::TransitionEmitFail),
            (0..n).prop_map(Cmd::Release),
            (0..n).prop_map(Cmd::Crash),
            Just(Cmd::Tick),
            (0..n).prop_map(Cmd::Reap),
        ]
    }

    fn apply(lock: &mut LockProtocol, cmd: Cmd) {
        // ガード違反の Err は「発火しないアクション」— モデルの enabled 条件と同型
        let _ = match cmd {
            Cmd::Acquire(p) => lock.acquire(p),
            Cmd::Reenter(p) => lock.reenter(p),
            Cmd::TransitionEmit(p) => lock.transition_emit(p),
            Cmd::TransitionWrite(p) => lock.transition_write(p),
            Cmd::TransitionEmitFail(p) => lock.transition_emit_fail(p),
            Cmd::Release(p) => lock.release(p),
            Cmd::Crash(p) => lock.crash(p),
            Cmd::Tick => lock.tick(),
            Cmd::Reap(p) => lock.reap(p),
        };
    }

    /// prev* スナップショット (audit_lock.qnt の `snapshot` アクションの Rust 版)。
    #[derive(Clone)]
    struct Snapshot {
        alive: Vec<bool>,
        dir_owner: Option<usize>,
        depth: u32,
        held_ticks: u32,
        pending: bool,
        state_ver: u64,
    }

    fn snapshot(lock: &LockProtocol) -> Snapshot {
        Snapshot {
            alive: (0..lock.process_count()).map(|p| lock.alive(p)).collect(),
            dir_owner: lock.dir_owner(),
            depth: lock.depth(),
            held_ticks: lock.held_ticks(),
            pending: lock.pending(),
            state_ver: lock.state_ver(),
        }
    }

    fn assert_invariants(prev: &Snapshot, lock: &LockProtocol, cmd: Cmd, threshold: u32) {
        // depth_consistent: dirOwner None ⇔ depth==0、Some ⇒ depth>=1
        match lock.dir_owner() {
            None => assert_eq!(lock.depth(), 0, "depth_consistent: None implies depth==0"),
            Some(_) => assert!(lock.depth() >= 1, "depth_consistent: Some implies depth>=1"),
        }
        // pending_only_with_lock: pending ⇒ dirOwner.is_some()
        if lock.pending() {
            assert!(lock.dir_owner().is_some(), "pending_only_with_lock");
        }
        // audit_first: state_ver が進むのは直前ステップが pending だったときのみ
        if lock.state_ver() > prev.state_ver {
            assert!(
                prev.pending,
                "audit_first: state_ver advanced without prior pending"
            );
        }
        // lock_no_steal 相当: 所有者交代 (Some(a)->Some(b), a!=b) の直前状態は
        // a が死んでいたか held_ticks が閾値を厳密超過していたときに限る
        if let (Some(a), Some(b)) = (prev.dir_owner, lock.dir_owner())
            && a != b
        {
            assert!(
                !prev.alive[a] || prev.held_ticks > threshold,
                "lock_no_steal: owner changed {a}->{b} while alive and within threshold"
            );
        }
        // crash_leaves_lock: crash コマンドの前後でロック状態 (dirOwner/depth/heldTicks/pending) は不変
        if matches!(cmd, Cmd::Crash(_)) {
            assert_eq!(
                lock.dir_owner(),
                prev.dir_owner,
                "crash_leaves_lock: dir_owner"
            );
            assert_eq!(lock.depth(), prev.depth, "crash_leaves_lock: depth");
            assert_eq!(lock.pending(), prev.pending, "crash_leaves_lock: pending");
            assert_eq!(
                lock.held_ticks(),
                prev.held_ticks,
                "crash_leaves_lock: held_ticks"
            );
        }
    }

    proptest! {
        #[test]
        fn quint_invariants_hold_under_random_command_sequences(
            cmds in proptest::collection::vec(cmd_strategy(MODEL_PROCESS_COUNT), 1..80),
        ) {
            let mut lock = LockProtocol::new(MODEL_PROCESS_COUNT, MODEL_THRESHOLD);
            let mut prev = snapshot(&lock);
            for cmd in cmds {
                apply(&mut lock, cmd);
                assert_invariants(&prev, &lock, cmd, MODEL_THRESHOLD);
                prev = snapshot(&lock);
            }
        }
    }
}
