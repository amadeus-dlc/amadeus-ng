# developer-brief-5 — 委任 5: 仕様・正本の同期（U3 / Bolt B5）

Conversation language: 日本語（仕様本文・注記・報告はすべて日本語。型名 / API 名 / ファイル名 / ID は英語のまま）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（Bolt B5）の委任 5。リポジトリルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、ブランチ
`bolt/b5-u3-event-store-repository`（委任 1・2 はコミット済み — ロック系は削除、`IntentId` = UUIDv7、`WorkflowExecutionState`）。委任 3・4 が**並行**して走る — 所有外には触れない。

所有ファイル: `docs/specs/01-domain-model.md`、`docs/specs/10-orchestration.md`、`docs/specs/11-workspace.md`、`docs/specs/deviations.md`、報告 `developer-report-5.md`（新規）。
触らないもの: `docs/specs/research/**`、`docs/specs/12-*.md` 等の他号、`modules/**`、`formal/**`、coding-rules（委任 1 が更新済み）、計画・検査手順・質問票。
`git add` / `git commit` はしない。`.claude/` のツールは実行しない。

## 先に読むもの（順に）

1. `.../u3-event-store-repository/code-generation/code-generation-plan.md`（§5.5 Step 12）
2. `.../u3-event-store-repository/functional-design/rules.md`（BR3.3 の不変条件名、BR5.1）、`functional-spec.md`（§1 配置、§3、§5）、`functional-design-questions.md`（Q1〜Q4 = A）
3. `.../u3-event-store-repository/nfr-design/security-design.md`（§3 / §6）
4. `.../inception/domain-design/decisions.md`（ADR-003 / 007）
5. 対象の現行本文: `docs/specs/10-orchestration.md` §3（ポート表）/ §6（不変条件表、I14）、`docs/specs/11-workspace.md` §2.1 / §2.2（`LockIdentity` 行）/ §3（ポート表・
   「ポートではないもの」）/ §4 / §6（W1〜W5 + 直前の E4 定義名段落 + 2026-08-23 注記）/ §8（Quint 記録）/ §10（未決事項）、`docs/specs/01-domain-model.md` §3.3
   （代表不変条件の段落、状態機械）/ §6（第一陣の項目）、`docs/specs/deviations.md` # 4。
6. Bolt B4 の作法の手本: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/nfr-design/security-design.md` §2（出典注記の形式、逐語契約に
   触れない、旧記述は「旧」明記の比較表のみ、表は列数一致）。

## 作業（計画 Step 12 — 出典注記つき、最小変更）

- **10 号**: §6 I14 行を journal_protocol の不変条件へ差し替え（I14「各遷移は原子的 — ジャーナル追記とスナップショット更新を同一 Tx、楽観 version で直列化、投影は
  チェックポイントから冪等」| E3+E4 | `journal_protocol::{conflict_rejected, snapshot_tracks_journal, version_equals_journal, no_lost_update}`（ADR-007 / Bolt B5））。
  E4 定義名段落の I14 参照を `formal/orchestration/journal_protocol.qnt` に。B4 で入れた「注記（2026-08-23）: … B5 で差し替える」は本文に反映したので削除。
  §3 ポート表の `WorkflowExecutionRepository` 実装欄に「`WorkflowExecutionRepositoryImpl`（`SqliteEventStore` を内包 — `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`、
  C6）」、登録簿の直列化は `SqliteEventStore::within_write_transaction`（Q2 = A）と一言。
- **11 号**: §6 の W1〜W5 行を J1〜J6 へ置換 — J1 `conflict_rejected`（楽観 version 不一致の store はジャーナル / スナップショットを変えない）、J2
  `snapshot_tracks_journal`（snapshot.seq_nr = ジャーナル末尾）、J3 `version_equals_journal`（snapshot.version = 永続化済みイベント数）、J4 `checkpoint_monotone` +
  `checkpoint_bounded`、J5 `projection_idempotent` + `truth_is_journal`（投影はジャーナルを超えない・再実行で変わらない）、J6 `no_lost_update`（store_ok は載せた version が
  現在 version のときのみ）— 強制 E3+E4、E4 定義名は `journal_protocol::*`。直前の E4 定義名段落と B4 の注記を本文反映に合わせて書き換え。§2.2 `LockIdentity` 行 →
  「退役（ADR-007 / Bolt B5）: 登録簿の直列化は `SqliteEventStore::within_write_transaction`（同一 DB の BEGIN IMMEDIATE）」の 1 行に縮退（keying 規範の文は「単一 DB =
  単一バケット」で満たされる旨）。§3「ポートではないもの」の `ProcessProbe` → 退役（reap 消滅）、`Clock` のみ機構、§4 配置の規範も同様。§7-4 ITF の文を
  journal_protocol（InMemory + フェイク投影）へ。§8 Quint 記録に「audit_lock.qnt は ADR-007 で退役し、協定モデル `formal/orchestration/journal_protocol.qnt`（不変条件 8 /
  witness 4）へ置換（Bolt B5）」。§10 未決 2 件を確定に書き換え: stage-0/1 併用期の相互排他 = 「担保しない — stage-1 は単一クローン運用（Q1 = A / P7）、併用は後続 intent」、
  `intents.json` の直列化 = `within_write_transaction`（Q2 = A）。§5 の派生イベント発行チョークポイントは既に store（B4）— 確認のみ。
- **01 号**: §3.3 代表不変条件の段落（B4 の「旧 mkdir ロックモデル … B5 で差し替え」注記）を協定モデルの不変条件（ジャーナルが真実源・楽観 version・チェックポイント
  単調・投影冪等）へ書き換え、§6 第一陣の「～~Audit lock lifecycle~～ → ジャーナル / スナップショット / version / チェックポイント協定」の行を
  「journal_protocol（`formal/orchestration/journal_protocol.qnt`、Bolt B5 で実装・ITF 準拠）」に確定。§3.3 の状態機械の行の `audit_lock.qnt` 言及も journal_protocol へ。
- **deviations.md** # 4: amadeus-ng 欄と理由欄の「`.aidlc-store.sqlite` 相当」→ 確定パス `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`（space 単位 1 ファイル、
  既存 .gitignore `aidlc/spaces/*/intents/.aidlc-*` で git 管理外）、記録欄の「最終パスは U3 で確定するため『相当』と記す — 確定時に本行を更新する」を
  「2026-08-23 Bolt B5 で確定」に。
- 各改訂行に出典注記（`（ADR-007 / Bolt B5）` / `（U3 FD Q1 = A）` 等）。表の列数・見出し重複・逐語契約不変（research/ に触れない）を自分で検査
  （B4 の `unit-test-instructions.md` §2 の表検査スクリプトを流用してよい: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/unit-test-instructions.md`）。
- 退役語の grep: `grep -nE 'WorkspaceLock|LockProtocol|LockIdentity|reap_eligible|audit_lock|ProcessProbe|withAuditLock' docs/specs/01-domain-model.md docs/specs/10-orchestration.md docs/specs/11-workspace.md`
  の残りが「退役」注記・履歴（旧）明記の行だけであることを報告に列挙。

## 報告（`developer-report-5.md`）

「改訂一覧（ファイル:節 → 内容 → 出典注記）」「検査（表・見出し・grep・research 不変）」「設計質問」「未了」。最終応答は要約（日本語、10 行以内）。
