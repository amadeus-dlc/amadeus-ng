// stop_hook.qnt — orchestration コンテキスト第一陣モデル (第一陣 2 号, v1)
//
// 対象: Stop フック forwarding loop — 「エンジンがすべてのルーティングを所有する」規律。
//       conductor がターンを終えようとすると next をプローブし、PENDING なら decision:block、
//       carve-out (固定順の許可経路) が勝てば allow、無限ブロックは no-progress cap が解放する。
//       upstream aidlc-workflows v2 @ 3c3146cf の 07-hooks.md §7 を抽象化
//       (根拠: docs/specs/research/orchestration-directives-verbs.md §8)。
//
// モデル型 ↔ Rust Domain Primitive 対応表 (ADR 0003 決定 6):
//   sig            ↔ ProgressSignature (stage::stateSha(Last Updated 除外)::directiveFingerprint) の抽象 (int)
//   dirKind        ↔ プローブが返す Directive の観測射影 (RunStage/Done/Parked/Ask)
//   recSig/recCount ↔ <record>/.aidlc-stop-hook/block-count.json {signature, count} (-1 = 記録なし)
//   stopHookActive ↔ payload の stop_hook_active (前 block の産物である信号)
//   autonomous     ↔ Construction Autonomy Mode (cap 8 / 2 の分岐と carve-out の autonomy ガード)
//   humanWait      ↔ carve-out 5: 現ステージ checkbox が [?] awaiting-approval / [R] revising
//   sideCarve      ↔ carve-out 0/1/6/7/8/9 の抽象 (下記 v1 抽象化参照)
//   decision       ↔ フックの出力 (decision:block + reason / allow)
//   lastAction     ↔ ITF リプレイ駆動用 (ADR 0003 決定 5)
//   prev*          ↔ (モデル専用) 遷移条件検査用の前状態スナップショット
//
// v1 の抽象化 (精密化は slice 2 候補):
//   - carve-out 0 (handoff TTL)・1 (resume wait — probe より先に読む順序)・6 (未回答質問と
//     code-generation Plan Approval 例外)・7 (pending decision)・8 (compose marker TTL)・
//     9 (会話ターン判定の 2 証拠経路) は sideCarve 1 ビットに畳む。これらは全て autonomy
//     ガード付きとして扱う (0 は実際には n/a だが安全側に倒す)
//   - CLAUDE_CODE_STOP_HOOK_BLOCK_CAP の env 上書きは未モデル化 (既定 8/2 のみ)
//   - プローブ 10s タイムアウト・Copilot session-owned path・usage folding は対象外
//   - 継続文 5 形の選択は対象外 (Block の一様観測)
//
// 検査力の確認 (mutation テスト — 7 変異 / 7 不変条件、全て検出済み):
//   - cap 判定除去 (常に block)              → blocked_below_cap
//   - block 時に記録を書かない               → record_before_decision
//   - done で resetGuard しない              → reset_on_terminal
//   - done でも block する (carve 順崩し)     → carve_beats_forwarding
//   - 署名変化でもカウント継続               → signature_resets_count
//   - parked の autonomy ガード除去          → parked_autonomous_guard
//   - seed-2 を 1 にする                     → seed2_on_active_chain
//
// 到達性 witness (w_*): CI は負形式 (--invariant "not(w_x)") で実行し violation = 経路実在 = pass。
// ランダムウォークでは深すぎる経路 (autonomous cap 8 の連続 block 等) は決定的シナリオ
// (run r_* — `quint test --match 'r_.*'`) で検査する。w_cap_release_autonomous はシナリオ側が正。
//
// E4 トレーサビリティ (10-orchestration.md §6 から参照される定義名):
//   stop_hook::blocked_below_cap / cap_is_mode_dependent / record_before_decision /
//   reset_on_terminal / carve_beats_forwarding / signature_resets_count /
//   parked_autonomous_guard / seed2_on_active_chain

module stop_hook {
  pure val INTERACTIVE_CAP = 2
  pure val AUTONOMOUS_CAP = 8
  pure val MAX_SIG = 4

  type DirKind = KRunStage | KDone | KParked | KAsk
  type Decision = DBlock | DAllow | DNone

  var sig: int
  var dirKind: DirKind
  var autonomous: bool
  var humanWait: bool
  var sideCarve: bool
  var recSig: int          // -1 = 記録なし
  var recCount: int
  var stopHookActive: bool
  var decision: Decision
  var lastAction: str

  var prevSig: int
  var prevDirKind: DirKind
  var prevAutonomous: bool
  var prevHumanWait: bool
  var prevSideCarve: bool
  var prevRecSig: int
  var prevRecCount: int
  var prevStopHookActive: bool

  def cap: int = if (autonomous) AUTONOMOUS_CAP else INTERACTIVE_CAP

  action init = all {
    sig' = 0,
    dirKind' = KRunStage,
    autonomous' = false,
    humanWait' = false,
    sideCarve' = false,
    recSig' = -1,
    recCount' = 0,
    stopHookActive' = false,
    decision' = DNone,
    lastAction' = "init",
    prevSig' = 0,
    prevDirKind' = KRunStage,
    prevAutonomous' = false,
    prevHumanWait' = false,
    prevSideCarve' = false,
    prevRecSig' = -1,
    prevRecCount' = 0,
    prevStopHookActive' = false,
  }

  action snapshot = all {
    prevSig' = sig,
    prevDirKind' = dirKind,
    prevAutonomous' = autonomous,
    prevHumanWait' = humanWait,
    prevSideCarve' = sideCarve,
    prevRecSig' = recSig,
    prevRecCount' = recCount,
    prevStopHookActive' = stopHookActive,
  }

  // === 環境アクション (エンジン・人間・状態の変化) ===

  action envProgress = all {
    sig' = (sig + 1) % MAX_SIG,   // 意味的変更 (state sha / directive fingerprint の移動)
    dirKind' = dirKind, autonomous' = autonomous, humanWait' = humanWait,
    sideCarve' = sideCarve, recSig' = recSig, recCount' = recCount,
    stopHookActive' = stopHookActive, decision' = decision,
    lastAction' = "env_progress",
  }

  action envDirective = {
    nondet k = Set(KRunStage, KDone, KParked, KAsk).oneOf()
    all {
      dirKind' = k,
      sig' = sig, autonomous' = autonomous, humanWait' = humanWait,
      sideCarve' = sideCarve, recSig' = recSig, recCount' = recCount,
      stopHookActive' = stopHookActive, decision' = decision,
      lastAction' = "env_directive",
    }
  }

  action envToggleAutonomy = all {
    autonomous' = not(autonomous),
    sig' = sig, dirKind' = dirKind, humanWait' = humanWait,
    sideCarve' = sideCarve, recSig' = recSig, recCount' = recCount,
    stopHookActive' = stopHookActive, decision' = decision,
    lastAction' = "env_autonomy",
  }

  action envToggleHumanWait = all {
    humanWait' = not(humanWait),
    sig' = sig, dirKind' = dirKind, autonomous' = autonomous,
    sideCarve' = sideCarve, recSig' = recSig, recCount' = recCount,
    stopHookActive' = stopHookActive, decision' = decision,
    lastAction' = "env_human_wait",
  }

  action envToggleSideCarve = all {
    sideCarve' = not(sideCarve),
    sig' = sig, dirKind' = dirKind, autonomous' = autonomous,
    humanWait' = humanWait, recSig' = recSig, recCount' = recCount,
    stopHookActive' = stopHookActive, decision' = decision,
    lastAction' = "env_side_carve",
  }

  // fresh-session handoff 境界: 耐久カウンタはクリアされるが、payload の
  // stop_hook_active (per-invocation のハーネス信号) は独立に残りうる — seed-2 の入口
  action envHandoffReset = all {
    recSig' = -1,
    recCount' = 0,
    sig' = sig, dirKind' = dirKind, autonomous' = autonomous,
    humanWait' = humanWait, sideCarve' = sideCarve,
    stopHookActive' = stopHookActive, decision' = decision,
    lastAction' = "env_handoff_reset",
  }

  // === フック発火 (conductor がターンを終えようとした) ===
  //
  // 判定順: 終端 directive (done / parked※非autonomous / ask) と human-wait、
  // side carve-out (autonomy ガード付き) が勝てば allow。
  // それ以外 (PENDING な作業 directive、および autonomous 下の parked) は
  // no-progress cap: nextCount = 同署名なら +1 / 記録なし & stop_hook_active なら 2 / それ以外 1。
  // 記録は判定「前」に書く。nextCount >= cap で解放 (allow)、未満なら block。
  // resetGuard (記録ゼロ化) は done / parked 許可時のみ。

  action actHookStop = {
    val carved = or {
      dirKind == KDone,
      and { dirKind == KParked, not(autonomous) },
      dirKind == KAsk,
      humanWait,
      and { sideCarve, not(autonomous) },
    }
    val terminalReset = or {
      dirKind == KDone,
      and { dirKind == KParked, not(autonomous) },
    }
    val nextCount =
      if (recSig == sig) recCount + 1
      else if (recSig == -1 and stopHookActive) 2
      else 1
    all {
      recSig' = if (carved) (if (terminalReset) -1 else recSig)
                else sig,
      recCount' = if (carved) (if (terminalReset) 0 else recCount)
                  else nextCount,
      decision' = if (carved) DAllow
                  else (if (nextCount >= cap) DAllow else DBlock),
      stopHookActive' = and { not(carved), nextCount < cap },
      sig' = sig, dirKind' = dirKind, autonomous' = autonomous,
      humanWait' = humanWait, sideCarve' = sideCarve,
      lastAction' = "hook",
    }
  }

  action step = any {
    all { snapshot, envProgress },
    all { snapshot, envDirective },
    all { snapshot, envToggleAutonomy },
    all { snapshot, envToggleHumanWait },
    all { snapshot, envToggleSideCarve },
    all { snapshot, envHandoffReset },
    all { snapshot, actHookStop },
  }

  // === 不変条件 (DoD — ADR 0003 決定 7) ===

  def hookFired: bool = lastAction == "hook"
  def prevCarved: bool = or {
    prevDirKind == KDone,
    and { prevDirKind == KParked, not(prevAutonomous) },
    prevDirKind == KAsk,
    prevHumanWait,
    and { prevSideCarve, not(prevAutonomous) },
  }

  // block するのは cap 未満のときだけ — cap 到達で必ず解放 (停止できないトレースが存在しない)
  val blocked_below_cap = (hookFired and decision == DBlock) implies
    (recCount < (if (autonomous) AUTONOMOUS_CAP else INTERACTIVE_CAP))

  // cap はモード依存 (interactive では 2 以上のカウントで block しない)
  val cap_is_mode_dependent = (hookFired and decision == DBlock and not(autonomous)) implies
    (recCount < INTERACTIVE_CAP)

  // 記録は判定前に書かれる: block したステップの記録は現署名に一致
  val record_before_decision = (hookFired and decision == DBlock) implies (recSig == sig)

  // done / parked (非 autonomous) の許可は resetGuard (記録ゼロ化)
  val reset_on_terminal = (hookFired and or {
    prevDirKind == KDone,
    and { prevDirKind == KParked, not(prevAutonomous) },
  }) implies (recSig == -1 and recCount == 0)

  // carve-out は固定順で forwarding に勝つ: carve 条件が立っていれば決して block しない
  val carve_beats_forwarding = (hookFired and prevCarved) implies (decision == DAllow)

  // 署名が変わったら streak はリセット (block 時のカウントは 1、seed-2 例外は記録なし時のみ)
  val signature_resets_count = (hookFired and decision == DBlock
    and prevRecSig != sig and prevRecSig != -1) implies (recCount == 1)

  // autonomous 下の parked は carve-out にならない: 許可は他の carve か cap 解放のみ
  val parked_autonomous_guard = (hookFired and prevDirKind == KParked
    and prevAutonomous and decision == DAllow) implies (or {
      prevHumanWait,
      recCount >= AUTONOMOUS_CAP,
    })

  // 進行中シーケンスへの合流: 記録なし & stop_hook_active の block は 2 から数える
  val seed2_on_active_chain = (hookFired and decision == DBlock
    and prevRecSig == -1 and prevStopHookActive) implies (recCount == 2)

  // === 到達性 witness (負形式で実行: violation = 経路実在 = pass) ===

  // block が起きる
  val w_block = hookFired and decision == DBlock

  // interactive の cap 解放 (2 回ブロック後の解放) が起きる
  val w_cap_release_interactive = and {
    hookFired, decision == DAllow, not(prevCarved), not(autonomous),
    prevRecSig == sig, prevRecCount + 1 >= INTERACTIVE_CAP,
  }

  // autonomous の cap 解放 (8) が起きる
  val w_cap_release_autonomous = and {
    hookFired, decision == DAllow, not(prevCarved), autonomous,
    prevRecSig == sig, prevRecCount + 1 >= AUTONOMOUS_CAP,
  }

  // autonomous 下の parked が block される (carve-out 3 の拒否)
  val w_parked_auto_block = and {
    hookFired, prevDirKind == KParked, prevAutonomous, decision == DBlock,
  }

  // seed-2 経路が起きる
  val w_seed2 = and {
    hookFired, decision == DBlock, prevRecSig == -1, prevStopHookActive, recCount == 2,
  }

  // 署名リセット経路が起きる
  val w_sig_reset = and {
    hookFired, decision == DBlock, prevRecSig != -1, prevRecSig != sig, recCount == 1,
  }

  // === 決定的シナリオ (quint test で実行 — ランダムウォークでは深すぎる経路) ===

  // autonomous cap (8): 7 回 block が続き、8 回目で必ず解放される
  run r_cap_release_autonomous = init
    .then(all { snapshot, envToggleAutonomy })
    .then((AUTONOMOUS_CAP - 1).reps(_ => all { snapshot, actHookStop }))
    .then(all { snapshot, actHookStop })
    .expect(and { w_cap_release_autonomous, decision == DAllow })

  // interactive cap (2): 1 回 block、2 回目で解放
  run r_cap_release_interactive = init
    .then(all { snapshot, actHookStop })
    .then(all { snapshot, actHookStop })
    .expect(and { w_cap_release_interactive, decision == DAllow })

}
