# developer-brief-4 — 委任 4: Quint モデル `journal_protocol.qnt` + ITF 準拠 + quint-gate（U3 / Bolt B5）

Conversation language: 日本語（モデルのコメント・報告はすべて日本語。Quint の識別子・固定トークンは英語）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（Bolt B5）の委任 4。リポジトリルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、ブランチ
`bolt/b5-u3-event-store-repository`（委任 1・2 はコミット済み。`InMemoryEventStore` / `InMemoryWorkflowExecutionRepository` / ポート / 契約テストが使える）。委任 3（SQLite）と
委任 5（docs）が**並行**して走る — 所有外のファイルには触れない。Quint は `quint` CLI 0.32.0（`npx`/グローバル — `quint --version` で確認。無ければ `npm i -g @informalsystems/quint@0.32.0`）。

所有ファイル: `formal/orchestration/journal_protocol.qnt`（新規）、`tests/conformance/fixtures/journal_protocol/**`（新規）、
`modules/core/interface-adapter/tests/journal_protocol_conformance.rs`（新規）、`scripts/quint-gate.sh`（journal_protocol のステップ追加のみ — 既存 engine_loop / stop_hook の
ステップは変えない）、`formal/README.md` があれば journal_protocol の行追加、報告 `developer-report-4.md`（新規）。

触らないもの: `modules/**/src/**`、`docs/**`、計画・検査手順・質問票、他の委任の所有ファイル。`git add` / `git commit` はしない。`.claude/` のツールは実行しない。

## 先に読むもの（順に）

1. `.../u3-event-store-repository/code-generation/code-generation-plan.md`（§5.4 Step 9〜11、§7）
2. `.../u3-event-store-repository/functional-design/rules.md`（BR3.3 / BR3.4 / BR3.5）、`functional-spec.md` §5、`entities.md`（JournalProtocolModel）
3. `docs/adr/0003-quint-operations.md`（DoD: named invariant ごとの mutation、状態遷移レベル不変条件、in-module witness、負形式 run、ITF 採取・`#meta` 正規化）
4. 既存モデルと ITF の規約: `formal/orchestration/engine_loop.qnt`（prev スナップショット方式・witness 定義の書き方）、`modules/core/domain/tests/engine_loop_conformance.rs`
   （lastAction 駆動の再生・状態射影の突合・fixture の読み方・アクション網羅 assert）、`tests/conformance/fixtures/engine_loop/`（ファイル名 `trace-0x<seed>.itf.json`、
   `#meta` の正規化の仕方 — 既存 fixture と同じ形に）、`scripts/quint-gate.sh`（typecheck / invariants run（seed 固定 + --max-samples 明示）/ witness の負形式 run の書き方）。
5. 委任 2 の成果: `modules/core/interface-adapter/src/orchestration/memory/in_memory_event_store.rs`、`memory/workflow_execution_repository.rs`、`tests/support/**`
   （集約の生成方法）、`modules/core/use-case/src/orchestration/{journal_reader.rs,event_store.rs,global_seq_nr.rs,projection_name.rs}`。

## 作業（計画 Step 9〜11）

### Step 9 — モデル `formal/orchestration/journal_protocol.qnt`
- 定数 `WRITERS = 2`。var: `journalLen`, `snapVersion`, `snapSeq`, `checkpoint`, `readModelSeq`, `loadedVersion: int -> int`, `lastAction: str`, `lastActor: int` + `prev*`
  （engine_loop v2 / 旧 audit_lock v2 と同じ prev スナップショット方式）。
- action: `load(w)`（`loadedVersion[w] := snapVersion`）、`store_ok(w)`（ガード `loadedVersion[w] == snapVersion`: `journalLen' = journalLen + 1`, `snapVersion' = snapVersion + 1`,
  `snapSeq' = journalLen + 1`, `loadedVersion[w]' = snapVersion + 1`）、`store_conflict(w)`（ガード `loadedVersion[w] != snapVersion`: 状態不変、`lastAction = "store_conflict"`）、
  `catchup`（`readModelSeq' = journalLen`, `checkpoint' = journalLen`）、`crash`（状態不変、`lastAction = "crash"` — Tx 済み・投影未反映を表す）、`idle`。`step = any {…}`。
- invariant（8 本、状態遷移レベルで書く）: `conflict_rejected`（`lastAction == "store_conflict" implies journalLen == prevJournalLen and snapVersion == prevSnapVersion and snapSeq == prevSnapSeq`）、
  `snapshot_tracks_journal`（`snapSeq == journalLen`）、`version_equals_journal`（`snapVersion == journalLen`）、`checkpoint_monotone`（`checkpoint >= prevCheckpoint`）、
  `checkpoint_bounded`（`checkpoint <= journalLen`）、`projection_idempotent`（`lastAction == "catchup" and prevCheckpoint == prevJournalLen implies readModelSeq == prevReadModelSeq`）、
  `truth_is_journal`（`readModelSeq <= journalLen`）、`no_lost_update`（`lastAction == "store_ok" implies prevLoadedVersion[lastActor] == prevSnapVersion`）。
- witness（4 本、in-module）: `w_conflict`（`lastAction == "store_conflict"`）、`w_crash_then_catchup`（crash の後に catchup で `readModelSeq == journalLen` になった経路 —
  prev で表現）、`w_interleaved_writers`（writer 0 の store_ok の後に writer 1 が store_conflict、またはその逆 — 2 writer の交錯）、`w_idempotent_catchup`
  （`lastAction == "catchup" and prevCheckpoint == prevJournalLen and prevJournalLen > 0`）。
- `quint typecheck` → `quint run --seed 0x<固定> --max-samples 3000 --max-steps 50 --invariants <8 本>` 緑。

### Step 10 — mutation（DoD）
- 不変条件ごとに 1 変異（例: `store_conflict` が `journalLen` を増やす → `conflict_rejected` 違反、`store_ok` のガード除去 → `no_lost_update` 違反、`catchup` が `checkpoint`
  を減らす → `checkpoint_monotone` 違反、`catchup` が `readModelSeq = journalLen + 1` → `truth_is_journal` 違反、`store_ok` が `snapSeq` を進めない → `snapshot_tracks_journal`
  違反、…）を一時ファイル（`/tmp` 相当 — リポジトリに残さない）で作って `quint run` が violation を出すことを確認。表（invariant / 変異 / 結果）を報告に。
- witness 4 本を負形式（`--invariant "not(w_x)"` → violation = 経路実在 = pass）で確認。

### Step 11 — ITF fixture + 準拠テスト + quint-gate
- `quint run formal/orchestration/journal_protocol.qnt --seed 0x<seed> --max-steps 40 --out-itf tests/conformance/fixtures/journal_protocol/trace-0x<seed>.itf.json`
  で 6 本以上（全アクション — load / store_ok / store_conflict / catchup / crash / idle — が少なくとも 1 本に現れる seed を選ぶ）、`#meta` を既存 fixture と同じ形に正規化。
- `modules/core/interface-adapter/tests/journal_protocol_conformance.rs`: fixture を読み、`InMemoryEventStore`（`JournalReader` / `EventStore`）+ 2 writer 分の「ロード済み集約」
  + フェイク投影（`readModelSeq: u64` を持つだけ）に `lastAction × lastActor` で再生: `load(w)` → `find_by_id` 相当（writer w の集約を再水和し version を保持）、
  `store_ok(w)` → writer w の集約にコマンド（`complete_stage` 等、ゲートが要る所は `open_gate` → `approve_gate` の組で 1 イベントずつ — `step` のたびに 1 イベント）→
  `store` が `Ok`、`store_conflict(w)` → `store` が `Err(Conflict)`、`catchup` → `events_after(checkpoint)` → フェイク投影の `readModelSeq = 最後の global`、
  `advance_checkpoint`、`crash` / `idle` → 何もしない。各ステップで射影（journalLen = 総イベント数、snapVersion / snapSeq、checkpoint、readModelSeq）を突合。
  全アクション網羅の assert（engine_loop 準拠テストと同型）。`#[tokio::test]`（dev-dependency `tokio` は委任 2 が追加済み）。
- `scripts/quint-gate.sh`: `JOURNAL_PROTOCOL="formal/orchestration/journal_protocol.qnt"` を加え、typecheck ループ・`invariants run: journal_protocol`（seed 固定 +
  `--max-samples` 明示 + 8 不変条件）・witness 4 本の負形式 run を追加。冒頭コメントのモデル一覧も更新。`bash scripts/quint-gate.sh` 全 PASS。

## 作法

- Quint 側の識別子は設計どおり（rules.md BR3.3）。設計に無い判断は報告の「設計質問」に。テストコードの `unwrap` は許容。添字アクセスはテストでも極力 `get()`。
- `tests/conformance/fixtures/journal_protocol/` に `README` は不要（既存規約に合わせる）。

## 報告（`developer-report-4.md`）

「モデル概要（var / action / invariant / witness）」「quint run の出力」「mutation 表」「witness 負形式の結果」「fixture 一覧（seed・アクション網羅）」
「conformance の射影規則と結果」「quint-gate の差分と実行結果」「設計質問」「未了」。最終応答は要約（日本語、10 行以内）。
