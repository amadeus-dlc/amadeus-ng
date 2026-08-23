// journal_protocol.qnt — orchestration コンテキストの永続化協定モデル (U3 / Bolt B5, v1)
//
// 対象: 集約 1・writer 2・投影 1 の抽象で、ADR-007 の「ロック退役後の並行制御」を検査する。
//       真実源はジャーナル、スナップショットは毎 store 更新 (ADR-001)、投影はチェックポイント
//       から冪等にキャッチアップする。相互排他をロックで担保していた旧 audit_lock.qnt の
//       置換であり、担保手段は SQLite Tx + 楽観 version だけになる (FD BR3.2 / BR3.3)。
//
// モデル型 ↔ Rust Domain Primitive 対応表 (ADR 0003 決定 6):
//   journalLen    ↔ journal 表の行数 (単一集約なので global 通番 GlobalSeqNr の最大値と一致)
//   snapVersion   ↔ snapshot 表の version 列 = WorkflowExecution::version() (楽観 version)
//   snapSeq       ↔ snapshot 表の seq_nr 列 = WorkflowExecution::seq_nr() (適用済みイベント数)
//   checkpoint    ↔ checkpoint 表の global 通番 = JournalReader::checkpoint(ProjectionName)
//   readModelSeq  ↔ 投影 (U4) が描き終えた最後の global 通番。本モデルは値だけを持つ
//   loadedVersion ↔ writer ごとの「再水和した集約が載せている版」
//                   = WorkflowExecutionRepository::find_by_id が with_version で載せた値
//   lastAction    ↔ ITF リプレイ用アクション記録 (ADR 0003 決定 5 の lastAction 規約)
//   lastActor     ↔ 同上。writer を一意に選ぶための添字 (lastAction × lastActor で駆動する)
//   prev*         ↔ (モデル専用) 状態遷移レベル不変条件のための前状態スナップショット。
//                   Rust 対応物なし
//
// アクション ↔ Rust ポート操作:
//   load(w)           ↔ WorkflowExecutionRepository::find_by_id (未書込なら genesis を持つ)
//   store_ok(w)       ↔ WorkflowExecutionRepository::store が Ok (Tx 内で journal + snapshot)
//   store_conflict(w) ↔ 同 store が Err(RepositoryError::Conflict) — 状態を変えない
//   catchup           ↔ JournalReader::events_after + advance_checkpoint
//   crash             ↔ Tx 済み・投影未反映のままプロセスが落ちる (状態不変のマーカー)
//   idle              ↔ 何も起きない (デッドロック回避のスタッタ)
//
// v1 の抽象化 (第二陣以降で精密化):
//   - 集約は 1 つ。複数集約にまたがる global 通番の交錯・投影の複数化は未モデル化
//   - store の失敗は Conflict のみ。Io / Corrupt / Schema は状態不変という点で crash と同型
//   - checkpoint の後退 (CheckpointRegression) はポートに口が無いためモデルにも無い
//   - 投影の中身 (state ファイルの内容) は持たず、進捗を readModelSeq だけで表す
//
// 検査力の確認 (mutation テスト、developer-report-4.md の表を参照):
//   named invariant 8 本それぞれについて、対応するガード・遷移を壊した変異モデルが
//   violation になることを確認済み (ADR 0003 決定 7 の DoD)。
//
// E4 トレーサビリティ (10-orchestration.md / 11-workspace.md から参照される定義名):
//   journal_protocol::conflict_rejected / snapshot_tracks_journal / version_equals_journal /
//   checkpoint_monotone / checkpoint_bounded / projection_idempotent / truth_is_journal /
//   no_lost_update

module journal_protocol {
  /// 並行して書く writer の数 (FD BR3.3)。2 で「相互排他をロック無しで担保できるか」を検査できる。
  pure val WRITERS = 2
  pure val WRITER_IDS = 0.to(WRITERS - 1)

  var journalLen: int                 // journal 表の行数
  var snapVersion: int                // snapshot 表の version 列
  var snapSeq: int                    // snapshot 表の seq_nr 列
  var checkpoint: int                 // 投影のチェックポイント (global 通番)
  var readModelSeq: int               // 投影が描き終えた最後の global 通番
  var loadedVersion: int -> int       // writer ごとの再水和済み版
  var lastAction: str                 // ITF リプレイ駆動用
  var lastActor: int                  // ITF リプレイ駆動用 (writer 添字)

  // 前状態スナップショット (状態遷移レベル不変条件用。全アクションが snapshot と合成される)
  var prevJournalLen: int
  var prevSnapVersion: int
  var prevSnapSeq: int
  var prevCheckpoint: int
  var prevReadModelSeq: int
  var prevLoadedVersion: int -> int
  var prevLastAction: str
  var prevLastActor: int

  action init = {
    val zero = WRITER_IDS.mapBy(w => 0)
    all {
      journalLen' = 0,
      snapVersion' = 0,
      snapSeq' = 0,
      checkpoint' = 0,
      readModelSeq' = 0,
      loadedVersion' = zero,
      lastAction' = "init",
      lastActor' = 0,
      prevJournalLen' = 0,
      prevSnapVersion' = 0,
      prevSnapSeq' = 0,
      prevCheckpoint' = 0,
      prevReadModelSeq' = 0,
      prevLoadedVersion' = zero,
      prevLastAction' = "init",
      prevLastActor' = 0,
    }
  }

  action snapshot = all {
    prevJournalLen' = journalLen,
    prevSnapVersion' = snapVersion,
    prevSnapSeq' = snapSeq,
    prevCheckpoint' = checkpoint,
    prevReadModelSeq' = readModelSeq,
    prevLoadedVersion' = loadedVersion,
    prevLastAction' = lastAction,
    prevLastActor' = lastActor,
  }

  // load: 再水和。writer w は現在のスナップショット版を載せる (未書込なら 0 = genesis の期待版)
  action actLoad = {
    nondet w = WRITER_IDS.oneOf()
    all {
      journalLen' = journalLen,
      snapVersion' = snapVersion,
      snapSeq' = snapSeq,
      checkpoint' = checkpoint,
      readModelSeq' = readModelSeq,
      loadedVersion' = loadedVersion.set(w, snapVersion),
      lastAction' = "load",
      lastActor' = w,
    }
  }

  // store 成功: 楽観 version が一致するときだけ 1 イベント追記 + スナップショット更新。
  // 書いた writer だけが新しい版を手にする (他の writer は stale のまま)
  action actStoreOk = {
    nondet w = WRITER_IDS.oneOf()
    all {
      loadedVersion.get(w) == snapVersion,
      journalLen' = journalLen + 1,
      snapVersion' = snapVersion + 1,
      snapSeq' = journalLen + 1,
      checkpoint' = checkpoint,
      readModelSeq' = readModelSeq,
      loadedVersion' = loadedVersion.set(w, snapVersion + 1),
      lastAction' = "store_ok",
      lastActor' = w,
    }
  }

  // store 衝突: 版が一致しない書込は拒否され、状態は 1 ビットも変わらない (Tx の rollback)
  action actStoreConflict = {
    nondet w = WRITER_IDS.oneOf()
    all {
      loadedVersion.get(w) != snapVersion,
      journalLen' = journalLen,
      snapVersion' = snapVersion,
      snapSeq' = snapSeq,
      checkpoint' = checkpoint,
      readModelSeq' = readModelSeq,
      loadedVersion' = loadedVersion,
      lastAction' = "store_conflict",
      lastActor' = w,
    }
  }

  // catchup: チェックポイント以降を読んで描き、チェックポイントを進める (冪等)
  action actCatchup = all {
    journalLen' = journalLen,
    snapVersion' = snapVersion,
    snapSeq' = snapSeq,
    checkpoint' = journalLen,
    readModelSeq' = journalLen,
    loadedVersion' = loadedVersion,
    lastAction' = "catchup",
    lastActor' = lastActor,
  }

  // crash: Tx は済んでいるが投影は未反映、という瞬間にプロセスが落ちる。永続状態は変わらない
  action actCrash = all {
    journalLen' = journalLen,
    snapVersion' = snapVersion,
    snapSeq' = snapSeq,
    checkpoint' = checkpoint,
    readModelSeq' = readModelSeq,
    loadedVersion' = loadedVersion,
    lastAction' = "crash",
    lastActor' = lastActor,
  }

  action actIdle = all {
    journalLen' = journalLen,
    snapVersion' = snapVersion,
    snapSeq' = snapSeq,
    checkpoint' = checkpoint,
    readModelSeq' = readModelSeq,
    loadedVersion' = loadedVersion,
    lastAction' = "idle",
    lastActor' = lastActor,
  }

  action step = any {
    all { snapshot, actLoad },
    all { snapshot, actStoreOk },
    all { snapshot, actStoreConflict },
    all { snapshot, actCatchup },
    all { snapshot, actCrash },
    all { snapshot, actIdle },
  }

  // === 不変条件 (DoD — ADR 0003 決定 7。状態遷移レベルで書く) ===

  // 衝突した書込は journal / snapshot を 1 ビットも変えない (Tx の rollback — NFR3.3)
  val conflict_rejected = (lastAction == "store_conflict") implies and {
    journalLen == prevJournalLen,
    snapVersion == prevSnapVersion,
    snapSeq == prevSnapSeq,
  }

  // スナップショットの seq_nr はジャーナルの長さに追随する (毎 store 更新 — ADR-001)
  val snapshot_tracks_journal = snapSeq == journalLen

  // 楽観 version は「永続化済みの最後の seq_nr」— ジャーナル長と一致する
  val version_equals_journal = snapVersion == journalLen

  // チェックポイントは後退しない (BR1.4 の単調性)
  val checkpoint_monotone = checkpoint >= prevCheckpoint

  // チェックポイントはジャーナルを追い越さない
  val checkpoint_bounded = checkpoint <= journalLen

  // 追いついた状態からの catchup は投影を動かさない (冪等 — NFR3.4)
  val projection_idempotent =
    (lastAction == "catchup" and prevCheckpoint == prevJournalLen)
      implies (readModelSeq == prevReadModelSeq)

  // 真実源はジャーナル — 投影がジャーナルより先に進むことはない
  val truth_is_journal = readModelSeq <= journalLen

  // 更新の消失が起きない: 書けたのは、書く直前に最新版を握っていた writer だけ
  val no_lost_update = (lastAction == "store_ok")
    implies (prevLoadedVersion.get(lastActor) == prevSnapVersion)

  // === 到達性 witness (負形式で実行: violation = 経路実在 = pass) ===

  // 衝突が実際に起きる (ロック無しでも並行書込が拒否される経路が存在する)
  val w_conflict = lastAction == "store_conflict"

  // クラッシュ後の catchup で投影が追いつく (投影未反映のまま落ちても回復できる)
  val w_crash_then_catchup = and {
    lastAction == "catchup",
    prevLastAction == "crash",
    prevReadModelSeq < journalLen,
    readModelSeq == journalLen,
  }

  // 2 writer の交錯: 片方の成功の直後にもう片方が衝突する (またはその逆)
  val w_interleaved_writers = and {
    lastActor != prevLastActor,
    or {
      and { prevLastAction == "store_ok", lastAction == "store_conflict" },
      and { prevLastAction == "store_conflict", lastAction == "store_ok" },
    },
  }

  // 追いついた状態からの再 catchup (冪等性の前提が空でないことの証明)
  val w_idempotent_catchup = and {
    lastAction == "catchup",
    prevCheckpoint == prevJournalLen,
    prevJournalLen > 0,
  }
}
