// audit_lock.qnt — workspace コンテキスト第一陣モデル (第一陣 3 号, v3)
//
// 対象: 監査ロックの並行モデル — mkdir-EEXIST ミューテックス × 再入深度 × owner スタンプ ×
//       reap (死んだ所有者 or 閾値超過のみ) × audit-first クリティカルセクション ×
//       クラッシュ (exit ハンドラなしの消滅)。方針書 R6 (exit/panic 経路のロック漏れ) の受け皿。
//       upstream aidlc-workflows v2 @ 3c3146cf の 03-state-audit-runtime.md §6.8 を抽象化。
// v2 (2026-08-22): 敵対的レビューの結果を反映 — アクションラベル経由でしか守られていなかった
//       所有権遷移を状態遷移レベルの不変条件 (lock_no_steal / release_requires_ownership /
//       reaper_alive / crash_leaves_lock) で直接守る。到達性 witness をモジュール内に定義。
// v3 (2026-08-22, PR #2 レビュー反映):
//   - audit-first のクリティカルセクションを emit 段階と state 書込段階に分割 (pending フラグ)。
//     「監査済み・state 未書込」が crash で到達可能な中間状態になり (R6 の要求)、
//     audit_first は「state 書込は emit 済みトランザクション内でのみ」を検査する形に精密化
//   - stale 境界を upstream 契約に一致: reap は年齢がしきい値を「厳密に超える」ときのみ (> THRESHOLD)
//   - witness 追加: w_crash_mid_txn (emit 後 crash) / w_recovery_after_mid_txn_crash (その後の reap 回復)
//
// モデル型 ↔ Rust Domain Primitive 対応表 (ADR 0003 決定 6):
//   dirOwner   ↔ os.tmpdir() 内のロックディレクトリ (mkdir-EEXIST。-1 = 不在)
//   depth      ↔ withAuditLock の identity ごとの再入深度カウンタ
//   heldTicks  ↔ owner.json スタンプ経過時間の抽象 (THRESHOLD = 既定 10 分の抽象化)
//   alive      ↔ プロセス生存 (kill(pid,0) の ESRCH 判定の抽象)
//   auditLen   ↔ 監査シャードの行数 (追記のみ)
//   stateVer   ↔ 状態ファイルの版 (audit-first: 監査 emit 済みトランザクション内でのみ進む)
//   pending    ↔ クリティカルセクション内「監査 emit 済み・state 未書込」の中間状態 (R6 の窓)
//   lastAction / lastActor ↔ ITF リプレイ駆動用 (ADR 0003 決定 5)
//   prev*      ↔ (モデル専用) フレーム/遷移条件検査用の前状態スナップショット
//
// v1→v2 でも維持している抽象化 (精密化は slice 2 候補):
//   - ロック識別子は 1 つ (単一 intent)。__workspace__ センチネル (2 成分形) と複数 identity は対象外
//   - 二相 acquire (mkdir → writeOwnerStamp) と未スタンプ猶予 (既定 5000ms) は未モデル化 —
//     acquire は原子で常に即スタンプ扱い。よって no_reap_of_live_fresh の守備範囲は
//     「スタンプ済み保持者」のみで、未スタンプ猶予は E3 (実装検査) の担保
//   - 取得予算 (50×100ms、merge 200×100ms、フック 5×50ms) と acquire 失敗経路は未モデル化
//   - owner.json の reapLiveOwnerAfterStale フラグは未モデル化 (常に true 相当)
//   - reap の rename CAS (renamed-aside 状態・stampMatches・戻し EEXIST レース) は原子アクションに抽象化
//   - appendAuditEntries のバッチ原子性 (全事前検証・1 write) は auditLen +1 に畳んでいる
//   - exit ハンドラによる解放は release と同一視。クラッシュ = ハンドラなしの消滅のみ
//   - 「解放または reap 可能」の liveness (temporal) は quint verify (nightly) 送り
//
// 検査力の確認 (mutation テスト — ADR 0003 決定 7。9 変異 / 9 不変条件、全て検出済み):
//   - reap ガード (ESRCH∨閾値) 除去          → no_reap_of_live_fresh
//   - emit 失敗経路での state 書込注入        → audit_first
//   - transition の所有権ガード除去          → writes_require_ownership
//   - crash がロック状態をリセット           → crash_leaves_lock
//   - acquire の dirOwner==-1 ガード除去 (横取り) → lock_no_steal
//   - release の dirOwner==p ガード除去       → release_requires_ownership
//   - reap の alive(reaper) ガード除去        → reaper_alive
//   - acquire が depth を立てない             → depth_consistent
//   - release が深度に関わらず常時解放        → reentrant_release_keeps_lock
//   - state 書込の pending ガード除去 (emit なし書込) → audit_first
//
// 到達性 witness (w_*): CI は各 witness を負形式 (--invariant "not(w_x)") で実行し、
// violation = 経路が実在 = pass と読む。ここが green のままなら経路がモデルから消えた退行。
//
// E4 トレーサビリティ (11-workspace.md §6 から参照される定義名):
//   audit_lock::audit_first / no_reap_of_live_fresh / lock_no_steal /
//   writes_require_ownership / release_requires_ownership / reaper_alive /
//   crash_leaves_lock / depth_consistent / reentrant_release_keeps_lock /
//   pending_only_with_lock

module audit_lock {
  pure val PROCS = 0.to(2)
  pure val THRESHOLD = 3

  var alive: int -> bool
  var dirOwner: int        // -1 = ロック不在
  var depth: int
  var heldTicks: int
  var auditLen: int
  var stateVer: int
  var pending: bool        // emit 済み・state 未書込のトランザクションが開いている
  var lastAction: str
  var lastActor: int

  var prevAlive: int -> bool
  var prevDirOwner: int
  var prevDepth: int
  var prevHeldTicks: int
  var prevAuditLen: int
  var prevStateVer: int
  var prevPending: bool

  action init = all {
    alive' = PROCS.mapBy(p => true),
    dirOwner' = -1,
    depth' = 0,
    heldTicks' = 0,
    auditLen' = 0,
    stateVer' = 0,
    pending' = false,
    lastAction' = "init",
    lastActor' = -1,
    prevAlive' = PROCS.mapBy(p => true),
    prevDirOwner' = -1,
    prevDepth' = 0,
    prevHeldTicks' = 0,
    prevAuditLen' = 0,
    prevStateVer' = 0,
    prevPending' = false,
  }

  action snapshot = all {
    prevAlive' = alive,
    prevDirOwner' = dirOwner,
    prevDepth' = depth,
    prevHeldTicks' = heldTicks,
    prevAuditLen' = auditLen,
    prevStateVer' = stateVer,
    prevPending' = pending,
  }

  // mkdir-EEXIST: ロック不在のときのみ成功
  action actAcquire = {
    nondet p = PROCS.oneOf()
    all {
      alive.get(p),
      dirOwner == -1,
      dirOwner' = p,
      depth' = 1,
      heldTicks' = 0,
      pending' = false,
      alive' = alive, auditLen' = auditLen, stateVer' = stateVer,
      lastAction' = "acquire",
      lastActor' = p,
    }
  }

  // 再入: 同一 identity の保持者はネスト可 (深度カウンタ)
  action actReenter = {
    nondet p = PROCS.oneOf()
    all {
      alive.get(p),
      dirOwner == p,
      depth' = depth + 1,
      dirOwner' = dirOwner, heldTicks' = heldTicks, pending' = pending,
      alive' = alive, auditLen' = auditLen, stateVer' = stateVer,
      lastAction' = "reenter",
      lastActor' = p,
    }
  }

  // audit-first の第 1 段: 監査 emit 成功。state はまだ書かれない (pending が開く)
  action actTransitionEmit = {
    nondet p = PROCS.oneOf()
    all {
      alive.get(p),
      dirOwner == p,
      not(pending),
      auditLen' = auditLen + 1,
      stateVer' = stateVer,
      pending' = true,
      dirOwner' = dirOwner, depth' = depth, heldTicks' = heldTicks,
      alive' = alive,
      lastAction' = "transition_emit",
      lastActor' = p,
    }
  }

  // audit-first の第 2 段: state 書込 (emit 済みトランザクション内でのみ)
  action actTransitionWrite = {
    nondet p = PROCS.oneOf()
    all {
      alive.get(p),
      dirOwner == p,
      pending,
      auditLen' = auditLen,
      stateVer' = stateVer + 1,
      pending' = false,
      dirOwner' = dirOwner, depth' = depth, heldTicks' = heldTicks,
      alive' = alive,
      lastAction' = "transition_write",
      lastActor' = p,
    }
  }

  // 監査 emit が throw した場合: 何も起きない (audit-first — state は書かれない)
  action actTransitionEmitFail = {
    nondet p = PROCS.oneOf()
    all {
      alive.get(p),
      dirOwner == p,
      not(pending),
      auditLen' = auditLen,
      stateVer' = stateVer,
      pending' = pending,
      dirOwner' = dirOwner, depth' = depth, heldTicks' = heldTicks,
      alive' = alive,
      lastAction' = "transition_emit_fail",
      lastActor' = p,
    }
  }

  // 解放: 深度が 0 に戻ったときのみロックを手放す (exit ハンドラ経由も同一視)
  action actRelease = {
    nondet p = PROCS.oneOf()
    all {
      alive.get(p),
      dirOwner == p,
      depth' = depth - 1,
      dirOwner' = if (depth == 1) -1 else dirOwner,
      heldTicks' = if (depth == 1) 0 else heldTicks,
      pending' = if (depth == 1) false else pending,
      alive' = alive, auditLen' = auditLen, stateVer' = stateVer,
      lastAction' = "release",
      lastActor' = p,
    }
  }

  // クラッシュ: exit ハンドラなしの消滅。ロックは残る (stale 化)
  action actCrash = {
    nondet p = PROCS.oneOf()
    all {
      alive.get(p),
      alive' = alive.set(p, false),
      dirOwner' = dirOwner, depth' = depth, heldTicks' = heldTicks,
      pending' = pending,
      auditLen' = auditLen, stateVer' = stateVer,
      lastAction' = "crash",
      lastActor' = p,
    }
  }

  // 保持中の時間経過 (owner スタンプの経過時間の抽象)
  action actTick = all {
    dirOwner != -1,
    heldTicks' = heldTicks + 1,
    dirOwner' = dirOwner, depth' = depth, pending' = pending,
    alive' = alive, auditLen' = auditLen, stateVer' = stateVer,
    lastAction' = "tick",
    lastActor' = -1,
  }

  // reap (rename CAS): 死んだ所有者 or 閾値超過のみ。生きている閾値未満の保持者からは決して奪わない
  action actReap = {
    nondet p = PROCS.oneOf()
    all {
      alive.get(p),
      dirOwner != -1,
      dirOwner != p,
      or { not(alive.get(dirOwner)), heldTicks > THRESHOLD },
      dirOwner' = p,
      depth' = 1,
      heldTicks' = 0,
      pending' = false,
      alive' = alive, auditLen' = auditLen, stateVer' = stateVer,
      lastAction' = "reap",
      lastActor' = p,
    }
  }

  // 全プロセス死亡等でのデッドロック回避
  action actIdle = all {
    alive' = alive, dirOwner' = dirOwner, depth' = depth, heldTicks' = heldTicks,
    pending' = pending,
    auditLen' = auditLen, stateVer' = stateVer,
    lastAction' = "idle",
    lastActor' = -1,
  }

  action step = any {
    all { snapshot, actAcquire },
    all { snapshot, actReenter },
    all { snapshot, actTransitionEmit },
    all { snapshot, actTransitionWrite },
    all { snapshot, actTransitionEmitFail },
    all { snapshot, actRelease },
    all { snapshot, actCrash },
    all { snapshot, actTick },
    all { snapshot, actReap },
    all { snapshot, actIdle },
  }

  // === 不変条件 (DoD — ADR 0003 決定 7) ===

  // audit-first: state が進むのは「監査 emit 済みで開いたトランザクション」の中でのみ
  // (emit なしの state 書込 — audit にない状態変化 — は表現不能)
  val audit_first = (stateVer > prevStateVer) implies prevPending

  // トランザクションはロック保持中にのみ開いている
  val pending_only_with_lock = pending implies (dirOwner != -1)

  // reap は「死んだ所有者 or 閾値超過」のみ (アクションラベル観測)
  val no_reap_of_live_fresh = (lastAction == "reap") implies (or {
    not(prevAlive.get(prevDirOwner)),
    prevHeldTicks > THRESHOLD,
  })

  // 所有権の移転そのものを守る: 保持者 A → 保持者 B の直接遷移は、A が死んでいるか
  // 閾値超過のときに限る (経路を問わない — acquire 経由の横取りも検出する)。
  // 本モデルにおける相互排他の実質的検査でもある
  val lock_no_steal =
    if (prevDirOwner == -1 or dirOwner == -1 or dirOwner == prevDirOwner) true
    else or { not(prevAlive.get(prevDirOwner)), prevHeldTicks > THRESHOLD }

  // 台帳・状態への書込はロック保持者のみが行う
  val writes_require_ownership = (or {
    lastAction == "transition_emit",
    lastAction == "transition_write",
    lastAction == "transition_emit_fail",
  }) implies (prevDirOwner == lastActor)

  // 解放できるのは保持者のみ (非保持者の rm -rf 相当を検出)
  val release_requires_ownership =
    (lastAction == "release") implies (prevDirOwner == lastActor)

  // reap を実行する側は生きているプロセスのみ
  val reaper_alive =
    if (lastAction != "reap") true
    else if (lastActor == -1) false
    else prevAlive.get(lastActor)

  // クラッシュはロック状態を変えない (ロックは残って stale 化する — R6 の核心)
  val crash_leaves_lock = (lastAction == "crash") implies (and {
    dirOwner == prevDirOwner,
    depth == prevDepth,
    heldTicks == prevHeldTicks,
  })

  // 深度とロック在否の整合
  val depth_consistent = and {
    (dirOwner == -1) implies (depth == 0),
    (dirOwner != -1) implies (depth >= 1),
    depth >= 0,
  }

  // 再入解放はロックを手放さない (深度 1 の解放のみ手放す)
  val reentrant_release_keeps_lock =
    (lastAction == "release" and prevDepth > 1) implies (and {
      dirOwner == prevDirOwner,
      depth == prevDepth - 1,
    })

  // === 到達性 witness (負形式で実行: violation = 経路実在 = pass) ===

  // 生きた保持者からの閾値 reap が起きる
  val w_threshold_reap = and {
    lastAction == "reap",
    prevDirOwner != -1,
    prevAlive.get(prevDirOwner),
    prevHeldTicks > THRESHOLD,
  }

  // 死んだ所有者からの reap が起きる (クラッシュ回復経路)
  val w_dead_owner_reap = and {
    lastAction == "reap",
    prevDirOwner != -1,
    not(prevAlive.get(prevDirOwner)),
  }

  // 深度 3 以上からの多段解放が起きる
  val w_deep_release = and { lastAction == "release", prevDepth >= 3 }

  // 監査 emit 失敗経路が起きる
  val w_emit_fail = lastAction == "transition_emit_fail"

  // emit 済み・state 未書込のまま crash する (R6 の中間状態)
  val w_crash_mid_txn = and { lastAction == "crash", prevPending }

  // その中間状態から reap で回復する
  val w_recovery_after_mid_txn_crash = and {
    lastAction == "reap", prevPending, not(prevAlive.get(prevDirOwner)),
  }

  // 完全解放 (深度 1 → ロック不在) が起きる
  val w_full_unwind = and { lastAction == "release", prevDepth == 1, dirOwner == -1 }
}
