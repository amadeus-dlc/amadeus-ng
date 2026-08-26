# contract-summary — Unit 間・外部境界の契約一覧

> Contract Design（Inception 2.8）成果物。出典: `../units-generation/unit-of-work.md`（10 Unit と kind）、
> `../units-generation/unit-of-work-dependency.md`（DAG と §4 統合点 4 境界）、`../domain-design/components.md`
> （ポート・イベント・投影の所在）、`../domain-design/decisions.md`（ADR-001〜007）、
> `../requirements-analysis/requirements.md`（NFR1 upstream 互換・NFR3 監査完全性・FR1〜FR5 の合格基準）、
> `docs/specs/deviations.md`（逸脱台帳）、確認質問 `contract-design-questions.md`（Q1〜Q7 回答済み・Looks correct）。
>
> 契約 = 境界をまたぐ 2 者の取り決め（何が・どんな形で・どの手段で渡り、失敗時にどうなるか）。
> 本ファイルが各契約の**正本の所在**と**形**を固定し、functional-design（3.1）が各 Unit の内側を詳細化する。

## 1. 契約一覧

| # | Provider Unit | Consumer | Mechanism | Owner |
|---|---|---|---|---|
| C1 | U7 `u7-cli-dispatcher-hooks`（CLI バイナリ `aidlc`） | External: Claude Code ハーネス（スキル・フック登録・statusline） | プロセス起動（argv/stdin → stdout の directive JSON・逐語文言 + 終了コード）と、動詞が書く upstream 互換ファイル（状態ファイル・監査シャード） | U7（正本 = upstream 仕様 D6 + U1 ゴールデン。破壊的変更は逸脱台帳 + ADR） |
| C2 | U7（フック 4 本のサブコマンド） | External: Claude Code フック機構（PreToolUse / PostToolUse / Stop / UserPromptSubmit） | stdin JSON → 終了コード（0 許可 / 2 拒否）+ stderr 理由 + 副作用（監査行） | U7（正本 = upstream フック契約 + U1 ゴールデン） |
| C3 | U3 `u3-event-store-repository`（実装） | U5 `u5-report-use-case` / U6 `u6-next-continue-use-case` / U7（composition root） | Rust trait（同一プロセス、静的束縛）: `WorkflowExecutionRepository` / `JournalReader`（~~EventStore 同形 trait~~ → 失効。2026-08-27 / ADR-010 — 本家 `event_store_adapter_rs::types::EventStore` が正本になったため、我々は定義しない） | U5/U6（使う側 = ユースケース層）。U3 は準拠 |
| C4 | U3（既存 `WorkflowDefinitionRepositoryImpl`） | U6（`next` が定義集約を参照） | Rust trait `WorkflowDefinitionRepository`（2026-08-23 改訂: `find()` → `find_by_id(&WorkflowDefinitionId)`、ADR-008） | U6（使う側） |
| C5 | U2 `u2-domain-es-core`（イベント語彙） / U4 `u4-read-model-updater`（投影規則） | U4（投影）/ U3（ジャーナルへ保存）/ U7（コマンド末尾で投影起動） | 同一プロセスの型（`WorkflowExecutionEvent`）+ 投影規則表（イベント → 監査行・状態ファイル差分） | 語彙 = U2、投影規則 = U4 |
| C6 | U3（SQLite ストア） | U4（チェックポイント以降の差分読取・チェックポイント更新） | shared-schema: SQLite DDL（~~journal / snapshot / checkpoint の 3 表~~ → 失効。2026-08-27 / ADR-010 — 本家の `journal` / `snapshot` ＋ 我々の `amadeus_projection_checkpoint`） | U3（我々の表のみ。本家 2 表の正本は upstream） |
| C7 | U1 `u1-canon-json-goldens`（正解データ） | U6 / U7 のテスト（CLI 出力・状態ファイル差分・監査行・hash-canonical 受入表の突合） | 共有フィクスチャ（リポジトリ内の固定ファイル） | U1（更新は upstream ピン更新の別 intent） |

外部契約は C1・C2 のみ（Q1 = A）。SQLite ファイル（C6）と内部ポート（C3〜C5）は外部契約ではない。
Q1 の「CLI 面」はハーネスがバイナリの実行から観測できるもの全体 — stdout/終了コードに加え、動詞が書く
upstream 互換ファイル（`aidlc-state.md`・監査シャード）の形式を含む（NFR1 の D6 範囲と一致）。

## 2. 各契約の仕様

### C1 — CLI 面（外部）

正本は upstream 仕様（`docs/specs/10-orchestration.md` / `11-workspace.md` / `12-workflow-definition.md`、
逸脱は `docs/specs/deviations.md`）と U1 の 0b ゴールデン。ここでは**形**を固定する。

```yaml
contract: cli-surface
version: upstream 3c3146cf (v2.6.40)  # 外部面のバージョンは upstream ピン。変更は逸脱台帳 + ADR
binary: aidlc                         # マルチコール。逸脱台帳 #1 の綴り写像で upstream の `bun <dir>/tools/aidlc-<tool>.ts <sub>` に対応
verbs:                                # ROUTES 表（U7 所有）。stage-1 の最小集合 — 全表は 10 号 §ROUTES
  orchestrate: [next, continue, report, park]
  state: [unpark]                     # print directive が名指す。next は呼ばず提示する（use-case-rules §3）
  log: [decision, answer, review]
  utility: [doctor]                   # stage-1 サブセット（U8）
  hooks: [stop-forwarding-loop, record-human-turn, state-transition-guard, write-audit-log]  # C2
stdout:
  next|continue: directive JSON（下記 directive）を 1 行。28 KiB 上限（超過時は load-steering で分割）
  report: "{ kind: done|print|error, ... }" の JSON
  print系: 逐語文言（message-catalog）— LLM の分岐条件になる文言はバイト一致
directive:
  kinds: [load-steering, run-stage, dispatch-subagent, invoke-swarm, present-gate, ask, print, error, done, parked]  # 10 種の閉集合（directive-schema クレート）
  load-steering: { stage, bundle: "sha256:<hex>", part, parts, rules_content: [{path, text}], continue_token }
  continue_token: 正準 JSON（U1 canon-json）を base64url した不透明トークン。バンドル digest とステージ/試行の識別子を含み、ドリフト時は error directive
  run-stage: { stage, phase, lead_agent, support_agents, mode, inline_context_paths, gate, memory_path, consumes, produces, rules_in_context, stage_file, next_stage, reviewer, review_class, protocol_modules, narration, ... }
exit_codes:
  0: 正常（directive/JSON を出力済み）
  1: エラー（error JSON または逐語エラー文言。upstream の `error()` と同じ）
  2: フックの拒否（C2 のみ）
env:
  AIDLC_*: upstream と同名・同意味（AIDLC_SKIP_HUMAN_PRESENCE_GUARD 等）
  AIDLC_LOG: amadeus-ng 拡張（逸脱台帳 #3、RUST_LOG 互換記法）
files_written:                        # 互換ファイル = リードモデル（U4 が投影）。形式は D6 逐語契約
  state: "<record>/aidlc-state.md"    # フィールド・見出し・チェックボックス記法は upstream と同一
  audit: "<record>/audit/<host>-<clone>.md"  # `---` 区切りブロック、見出し・**Field**: 順序は EVENT_HEADINGS / FIELD_ORDER
  store: "<record>/.aidlc-store.sqlite"      # 外部契約ではない（C6）。git 管理外。パスは functional-design で確定
```

エラー・リトライ（Q6 = A）: 終了コードと文言は upstream 互換。ローカル I/O のみのためタイムアウトは設けない。
内部の楽観 version 競合が 2 回続いた場合は exit 1 + 逐語のエラー文言（文言は message-catalog に新設、
逸脱台帳の対象外 — upstream に同状況が存在しないため）。

### C2 — フック 4 本（外部）

```yaml
contract: hook-subcommands
protocol: Claude Code hooks（stdin に JSON、exit 0 = 許可 / exit 2 = 拒否 + stderr 理由）
hooks:
  - name: stop-forwarding-loop
    event: Stop
    reads: [aidlc-state.md（投影）, <slug>-questions.md の空 [Answer] タグ, 監査台帳の DECISION_RECORDED/QUESTION_ANSWERED 対]
    writes: なし（判定のみ）
    exit: 0 許可 / 2 ブロック（stderr に継続指示の逐語文言）
  - name: record-human-turn
    event: [UserPromptSubmit, PostToolUse(AskUserQuestion)]
    writes: HUMAN_TURN 監査行（ドメインイベントではなく監査専用行 — 投影規則 C5 の「直接行」）
    exit: 0
  - name: state-transition-guard
    event: PreToolUse(Bash)
    reads: 監査台帳・状態（投影）
    exit: 0 許可 / 2 拒否（直接の aidlc-state.ts ライフサイクル動詞呼出を拒否）
  - name: write-audit-log
    event: PostToolUse(Write|Edit)
    writes: ARTIFACT_CREATED / ARTIFACT_UPDATED 監査行（record 配下・codekb 配下の書込のみ）
    exit: 0
compat: 発火条件・stdout/stderr 文言・ブロック挙動は upstream 互換。正本 = upstream フック実装の観測契約 + U1 ゴールデン
```

### C3 — ポート trait: `WorkflowExecutionRepository` と `JournalReader`（内部、Rust trait が正本）

> **2026-08-27 改訂（ADR-010 / Bolt B6 — event-store-adapter-rs v2.0.0 へ乗り換え）**:
> 本節が定義していた**ローカル `EventStore` 同形 trait は失効**した。イベントストアの契約は
> 本家 `event_store_adapter_rs::types::EventStore` が正本であり、我々はそれを**定義せず実装する側**に回る
> （Conformist、腐敗防止層なし）。数値パラメータも本家に従い `usize`（2026-08-23 の `u64` 具体化は撤回）。
> あわせて `InMemoryWorkflowExecutionRepository`（テストダブル）も失効し、
> `WorkflowExecutionRepositoryImpl::in_memory()`（本家の memory バックエンドを内包）に置き換わった。
> 出典: [`decisions.md` ADR-010](../domain-design/decisions.md)、
> [`developer-report-1.md` §6](../../construction/esa-v2-migration/developer-report-1.md)・
> [`developer-report-2.md` §8](../../construction/esa-v2-migration/developer-report-2.md)。

ユースケース層（`core-use-case`）が所有する trait は `WorkflowExecutionRepository` と `JournalReader` の 2 本。
実装 `…Impl` は `core-interface-adapter`（U3）。動詞 `store` は ES 拡張語彙（ADR-006。正本注記は U9 FR8.1）。

```rust
// core-use-case（U5/U6 が所有、U3 が準拠）
pub trait WorkflowExecutionRepository {
    /// 集約を再水和する。最新スナップショット + seq_nr 以降のイベントを replay。
    /// # Errors
    /// `NotFound`（intent_id に対応する集約が無い — 契約上は呼出側の前提違反、fatal）、
    /// `Io { kind: std::io::ErrorKind, .. }`（ErrorKind を保持 — 監査 C24）、`Corrupt`（復号不能）。
    async fn find_by_id(&self, id: &IntentId) -> Result<WorkflowExecution, RepositoryError>;

    /// 1 コマンドが返した単一イベントと、適用後の集約（スナップショット）を同一 Tx で永続化する。
    /// 楽観 version: 期待値は `aggregate.version()`（**ストアが前回載せた不透明トークン**）。
    /// 一致しなければ `Conflict` で、ストアの状態は変わらない。
    /// （2026-08-27 改訂 / ADR-010: 旧「集約の version が保存時点のジャーナル末尾と一致しなければ」は
    ///  **失効**。version は seq_nr から導かない — 採番はストアの責務であり我々は解釈しない（BR5.3）。
    ///  引数は `&` なので呼出側の集約は動かず、続けて書くには再水和が要る）
    /// # Errors
    /// `Conflict { expected, actual }`（再水和して 1 回だけ再試行 — Q6）、`Io`、`Corrupt`。
    // （2026-08-23 改訂: `&self` → `&mut self` に具体化。オーナー裁定。U3（Bolt B5）の実装で内部可変性を
    // 使わない設計を採ったため、`&self` の trait メソッドから `&mut self` の EventStore を呼ぶための
    // 内部可変性の機構（下記 Finding #3）が不要になった — coding-rules/interior-mutability.md /
    // command-query-separation.md。所有 Unit は U5/U6 のままだが、U3 の実装が正）
    async fn store(&mut self, event: &WorkflowExecutionEvent, aggregate: &WorkflowExecution)
        -> Result<(), RepositoryError>;
}

// ---- 失効（2026-08-27 / ADR-010）: ローカル `EventStore` 同形 trait ----
// 旧: pub trait EventStore<AID, A, E> { persist_event / persist_event_and_snapshot /
//     get_latest_snapshot_by_id / get_events_by_id_since_seq_nr }（ローカル定義、ADR-006。
//     2026-08-23 に数値を usize → u64 へ「具体化」した）
// → 口ごと削除した。正本は本家 `event_store_adapter_rs::types::EventStore`（関連型 AID/AG/EV、
//   数値は usize、エラーは EventStoreWriteError / EventStoreReadError の 2 種）であり、
//   我々は定義しない。`WorkflowExecutionRepositoryImpl<S>` が本家 trait を型引数の境界に取り、
//   S = EventStoreForSqlite | EventStoreForMemory を選ぶ（借り物の契約は 1 文字も変えない —
//   coding-rules/upstream-contracts.md）。

// 読取側（U4 が使う差分読取 — C6 の上に立つ。U3 所有の trait、`core-use-case` に置く）
// 本家のイベントストアは集約単位の読み書きだけを担うので、全集約横断の順序読取と投影の
// チェックポイントは我々が持ち続ける（ADR-010 決定 4）。
// （2026-08-27 改訂: エラー型を EventStoreError → JournalReadError に改名・3 変種化。
//  「同一 Tx で更新」は失効 — 本家は接続も Tx も露出しないため、チェックポイントの前進は
//  我々の別接続の単独書込である。単調性は advance_checkpoint 自身が保証する）
pub trait JournalReader {
    /// グローバル通番が `after` より大きいイベントを昇順に返す（チェックポイント以降の差分）。
    async fn events_after(&self, after: GlobalSeqNr) -> Result<Vec<(GlobalSeqNr, WorkflowExecutionEvent)>, JournalReadError>;
    /// 投影のチェックポイントを読む/進める（単調。現在値未満は CheckpointRegression）。
    async fn checkpoint(&self, projection: &ProjectionName) -> Result<GlobalSeqNr, JournalReadError>;
    async fn advance_checkpoint(&mut self, projection: &ProjectionName, to: GlobalSeqNr) -> Result<(), JournalReadError>;
}
```

契約上の約束: ① `find_by_id` は集約を完全に再構成して返す（部分データを返さない）。② `store` の Tx 所有は
Repository 実装（ユースケースはトランザクションを持たない。2026-08-27 補足: Tx を実際に張るのは
**本家のイベントストア**であり、我々のコードは接続も Tx も持たない）。③ `Conflict` 以外のエラーはリトライしない。
④ ~~`InMemoryWorkflowExecutionRepository` は同じ trait を満たし、テストは `XxxUseCase<InMemory…>` で組む~~
→ **失効**（2026-08-27 / ADR-010）。テストダブル型は無く、`WorkflowExecutionRepositoryImpl::in_memory()` が
**本家の memory バックエンド**を内包する（実装コードは SQLite と同一で、バックエンドだけが違う）。
テストは `XxxUseCase<WorkflowExecutionRepositoryImpl<EventStoreForMemory<…>>>` で組む。
⑤ `dyn` は使わない（静的束縛、use-case-rules §2）。
⑥ **楽観 version はストアが採番する不透明トークン**であり、ドメインも Repository も解釈・比較しない
（2026-08-27 追加 / ADR-010 追記 (1)・BR5.3）。genesis（`Event::is_created()` が真）だけは
Gateway が**ストアへ渡す写しにのみ**初期値 1 を載せる — 呼出側の集約は動かない。

### C4 — ポート trait: `WorkflowDefinitionRepository`（2026-08-23 改訂 — ADR-008）

`WorkflowDefinition` はエンティティ（集約ルート、12 号 §2.1）なので識別子 `WorkflowDefinitionId`（内容が変わっても不変の系譜 ID — Repository 実装が
harness.json の `name` から付与）と内容版 `DefinitionRevision`（3 入力の正準 JSON の `sha256:` — 値属性、識別子ではない）を持つ。既存の引数なし `find()` は
**廃止**（後方互換の併存なし — オーナー裁定 2026-08-23）。`WorkflowExecution` は定義を `definition_id` で間接参照する（C5 `Started`）。

```rust
pub trait WorkflowDefinitionRepository {
    /// 3 入力（stage-graph / scope-grid / scopes）を読み、id / revision を付与した `WorkflowDefinition` を返す。
    /// # Errors
    /// `NotFound`（要求 id がこのハーネスの定義 id と異なる — 契約上 fatal）、読取・解析失敗は `GraphReadError` の既存変種。
    fn find_by_id(&self, id: &WorkflowDefinitionId) -> Result<WorkflowDefinition, GraphReadError>;
}
```

呼出側は `WorkflowDefinitionId` を (a) 稼働中のワークフローでは `WorkflowExecution::definition_id()` から、(b) birth（intent-create）では composition root が
harness.json から組み立てた値から渡す。`next_decision` は引数の定義の id が `definition_id` と一致しなければ `Err(DefinitionMismatch)`（U2 BR2.6）。

### C5 — ドメインイベント語彙と投影規則（内部）

コマンドと 1:1 のドメインイベント（U2 所有）。`schema_version: 1` を全イベントに予約（Q5 = A）。
投影（U4 所有）は 1 イベント → upstream 監査行 N 行 + 状態ファイル差分。監査行の見出し・フィールド順は
EVENT_HEADINGS / FIELD_ORDER（audit-events クレート、86 語）に従う。

> **2026-08-27 改訂（ADR-010 / Bolt B6）— 語彙は不変、封筒の**形**とワイヤ表現が変わった**:
> ① 封筒の `intent_id` / `seq_nr` は `WorkflowExecutionEventId`（両者の組の Domain Primitive、新設）に
> まとまった。② ~~ジャーナル行は封筒を列（`event_type` / `schema_version` / `occurred_at`）に持ち、
> payload は正準 JSON。未知の `type` は `Corrupt(UnknownEventType)`、対応外の版は `Corrupt(SchemaVersion)`~~
> → **失効**。本家のジャーナルは `payload` 1 列に**封筒ごと serde で**書く（本家の既定シリアライザ
> `serde_json::to_vec`）。未知の変種は serde の復号失敗に畳まれ `Corrupt(UndecodablePayload)` になる
> （`CorruptCause::UnknownEventType` / `SchemaVersion` は削除）。**2026-08-27 訂正**:
> 対応外の `schema_version` の検査は**復元済み** — `decode_event`（`journal_reader_impl.rs`）が
> 復号後に `SCHEMA_VERSION`（1）以外を `Corrupt(UndecodablePayload)` で拒否する（B6 CodeRabbit #466
> が発見した実装ギャップの即時是正。回帰テストあり）。
> ③ ストアの payload は**契約 JSON（BR1.7 / canon-json）ではない** — 本家が書くのであって我々は
> 呼んでいない。契約 JSON の射程は upstream 観測面（監査行・状態ファイル・directive）に限られる。
> **投影規則（`projects_to`）と監査行の逐語性は 1 文字も変わっていない。**

```yaml
asyncapi-like: workflow-execution-events
schema_version: 1                      # 予約。追加フィールドは消費側が無視（additive-safe）
# 2026-08-27 改訂: intent_id + seq_nr → id（WorkflowExecutionEventId）。値は同じ 2 つ組である
envelope: { id: { intent_id, seq_nr }, occurred_at, schema_version, payload }
events:
  - name: Started
    command: start (intent-create)
    payload: { definition_id, definition_revision, scope, request, stages, depth, test_strategy }   # 2026-08-23: definition_id / definition_revision を追加（ADR-008、U2 BR2.6）。stages_in_scope（list<StageSlug>）→ stages（list<StageEntry> = slug + phase + plan_action + conditional、文書順の全ステージ）に改名・型変更 — U2 実装（Bolt B3）の `Started::stages()` と一致させた。in-scope の集合は各 StageEntry の plan_action = EXECUTE から導く
    projects_to:
      audit: [WORKFLOW_STARTED, PHASE_STARTED(initialization), STAGE_STARTED×3 + STAGE_COMPLETED×3（init 3 stage）, PHASE_SKIPPED（scope 外 phase）]
      state: 全フィールド初期化（Project Information / Scope Configuration / Stage Progress / Current Status）
  - name: GateOpened
    command: report --result awaiting-approval
    payload: { stage, artifacts }
    projects_to: { audit: [STAGE_AWAITING_APPROVAL], state: "checkbox [-] → [?]" }
  - name: GateApproved
    command: report --result approved
    payload: { stage, user_input, next_stage?, phase_boundary? }
    projects_to:
      audit: [GATE_APPROVED, STAGE_COMPLETED, (PHASE_COMPLETED, PHASE_VERIFIED?, PHASE_STARTED)?, STAGE_STARTED(next) | WORKFLOW_COMPLETED]
      state: "checkbox [?] → [x]、Current/Next Stage、Completed 数、Phase Progress"
  - name: GateRejected
    command: report --result rejected
    payload: { stage, feedback, revision_count }
    projects_to: { audit: [GATE_REJECTED, STAGE_REVISING], state: "checkbox [?] → [R]、Revision Count" }
  - name: StageRevised
    command: report --result revised
    payload: { stage }
    projects_to: { audit: [STAGE_AWAITING_APPROVAL], state: "checkbox [R] → [?]" }
  - name: StageSkipped
    command: report --result skipped
    payload: { stage, reason, next_stage? }
    projects_to: { audit: [STAGE_SKIPPED, STAGE_STARTED(next) | WORKFLOW_COMPLETED], state: "checkbox → [S]、Current/Next Stage" }
  - name: Jumped
    command: jump execute
    payload: { direction, source, target, stages_reset, stages_skipped }
    projects_to: { audit: [STAGE_JUMPED, STAGE_SKIPPED×n], state: "対象以降の checkbox リセット、Current Stage" }
  - name: Parked
    command: park
    payload: { stage }
    projects_to: { audit: [WORKFLOW_PARKED], state: "park マーカー" }
  - name: Unparked
    command: unpark
    payload: {}
    projects_to: { audit: [WORKFLOW_UNPARKED], state: "park マーカー除去" }
  - name: Recomposed
    command: recompose
    payload: { skipped: [slug], added: [slug], stages_in_scope }
    projects_to: { audit: [RECOMPOSED], state: "Stage Progress の EXECUTE/SKIP 接尾辞、Total Stages" }
  - name: AutonomyModeSet
    command: set-autonomy
    payload: { mode: autonomous|gated }
    projects_to: { audit: [AUTONOMY_MODE_SET], state: "Construction Autonomy Mode" }
direct_audit_rows:                    # ドメインイベントを経ない監査専用行（フック・ログ・センサー）。投影ではなく append
  - [HUMAN_TURN, DECISION_RECORDED, QUESTION_ANSWERED, SUMMARY_CONFIRMATION_RECORDED, REVIEW_REQUESTED, REVIEW_COMPLETED, ARTIFACT_CREATED, ARTIFACT_UPDATED, ARTIFACT_REUSED, SENSOR_*, SESSION_*, ERROR_LOGGED]
rules:
  - 行の見出し・フィールド順は audit-events の EVENT_HEADINGS / FIELD_ORDER（逐語）
  - 1 イベントが複数行を描くとき、行順は upstream の emit 順（approve: GATE_APPROVED → STAGE_COMPLETED → PHASE_* → STAGE_STARTED）
  - 投影は冪等: 同じ seq_nr を 2 度描かない（チェックポイント C6 で保証）。再生成は空の投影先から全イベントを再適用
  - 他クローンのシャードは読み取り専用の外部入力。位置付き横断読取の順序 = timestamp ソート + バッファ位置 tiebreak（FR1.1）
```

### C6 — SQLite スキーマ（内部、shared-schema）

> **2026-08-27 全面改訂（ADR-010 / Bolt B6 — event-store-adapter-rs v2.0.0 へ乗り換え）**:
> ~~我々が定義した 3 表（`journal` / `snapshot` / `checkpoint`）~~ → **失効**。`journal` と `snapshot` は
> **本家 v2.0.0 のスキーマ**に置き換わり、**正本は upstream**（我々は所有しない）。我々の表は
> `amadeus_projection_checkpoint` の 1 つだけである。
> ~~版の検査は `PRAGMA user_version`（0 なら DDL を実行して 1 を刻み、知らない版は `Schema` で拒否）~~
> → **失効**（本家は `user_version` を使わない。`EventStoreError::Schema` 変種も削除）。代替は
> **バージョンの完全固定（`event-store-adapter-rs = "=2.0.0"`）＋ スキーマガードテスト**で、
> `journal` の DDL と一意索引の SQL 文字列を実測と突き合わせ、本家スキーマが変わったら明示的に落ちる
> （`modules/core/interface-adapter/src/orchestration/journal_reader_impl.rs` の
> `the_upstream_journal_schema_is_the_pinned_one` / `the_journal_table_keeps_a_rowid_so_the_cursor_is_well_defined`）。
> ~~スナップショット payload の値域検査（JSON の正確整数域 2^53 を超える値を拒否）~~ → **失効**
> （検査を担っていたワイヤ構造体ごと削除。ストアファイルは upstream 非観測なので互換上の実害は無い）。

```sql
-- 本家 event-store-adapter-rs v2.0.0 が接続確立時に冪等に作る 2 表（正本は upstream。我々は
-- 所有せず、DDL も発行しない）。ピン `=2.0.0` とスキーマガードテストで固定する。
CREATE TABLE journal (
  pkey            TEXT    NOT NULL,                  -- 本家の書込アドレス
  skey            TEXT    NOT NULL,
  aid             TEXT    NOT NULL,                  -- 集約 ID（intent_id）
  seq_nr          INTEGER NOT NULL,                  -- 集約内の単調増加（1 コマンド 1 イベント）
  payload         BLOB    NOT NULL,                  -- 封筒ごと serde（本家の serde_json。契約 JSON ではない）
  occurred_at     INTEGER NOT NULL,
  PRIMARY KEY (pkey, skey)
);
CREATE UNIQUE INDEX journal_aid_seq_nr_idx ON journal (aid, seq_nr);
CREATE TABLE snapshot (
  pkey            TEXT    NOT NULL,
  skey            TEXT    NOT NULL,
  aid             TEXT    NOT NULL,
  seq_nr          INTEGER NOT NULL,                  -- このスナップショットが含む最後の seq_nr
  version         INTEGER NOT NULL,                  -- 楽観 version（本家が採番する不透明トークン — BR5.3）
  payload         BLOB    NOT NULL,                  -- 集約の状態の写し（17 属性）を serde
  last_updated_at INTEGER NOT NULL,
  PRIMARY KEY (pkey, skey)
);
CREATE INDEX snapshot_aid_seq_nr_idx ON snapshot (aid, seq_nr);

-- 我々の表（U3 所有）。本家の表と名前が衝突しないよう接頭辞を付ける。U4 が読み書きする。
CREATE TABLE amadeus_projection_checkpoint (
  projection      TEXT    PRIMARY KEY,               -- 例: state-file, audit-shard
  last_global_seq INTEGER NOT NULL                   -- 単調増加。巻き戻しは再生成時のみ（行削除 → 0）
);
-- 2026-08-27: updated_at 列は落とした（誰も読まない列で、押印には時計が要るため。
--             Repository も Clock を持たなくなった — NFR3.1 の再掲は下記制約 (5) を参照）
-- 制約: (1) store（イベント追記 + スナップショット更新 + 楽観 version の CAS）は**本家が**同一 Tx で行う。
--       我々は接続も Tx も持たない（本家は露出しない）。競合はストアが拒否し、状態は変わらない。
--       (2) seq_nr は集約ごとに +1 のみ。(3) amadeus_projection_checkpoint.last_global_seq は
--       advance 時に単調でなければ Err（CheckpointRegression）。(4) ファイルは git 管理外（逸脱台帳へ登録）。
--       (5) 全集約横断のカーソルは本家 journal の **rowid**（追記専用＝コミット順の単調カーソル）。
--       AUTOINCREMENT の専用列は持たない。U4 は同じ DB ファイルへの**別接続**で読む（JournalReader 経由）。
--       (6) 押印時刻はイベントの occurred_at から来る（ストアは時計を持たない — 本家の作法）。
```

### C7 — ゴールデン（共有フィクスチャ）

```yaml
contract: golden-fixtures
owner: U1
layout:
  tests/golden/upstream-3c3146cf/hash-canonical/cases.json   # 受入表 { family, upstream_commit, case_count, cases: [{ id, class, description, input | input_js + construct, expected: { canonical_output, canonical_digest, compact_output, compact_digest_prefixed, compact_digest_hex, pretty_output } }] }（ADR 0001 受入条件 2: 出力文字列とハッシュの両方を固定。2026-08-22 U1 機能設計レビューで 2 フィールド省略表記を訂正、同日 U1 code-generation の実採取でフィールド名を実体に合わせて確定 — 旧表記 expected_output / expected_sha256 は canonical_output / canonical_digest に対応）。来歴は同ディレクトリの provenance.json
  tests/golden/upstream-3c3146cf/cli/<verb>/<case>/{argv,stdin,stdout.json|stdout.txt,stderr,exit,state.diff,audit.md,case.json}  # FR7.2 実行出力（2026-08-22 U1 code-generation の実採取で exit / stderr / case.json を追加 — C1 の終了コード契約とフック拒否理由の逐語性のため。stdout は JSON として読めた場合 stdout.json、それ以外 stdout.txt）。欠落ケースは cli/cases-missing.json に理由付き、来歴は cli/provenance.json
  tests/golden/upstream-3c3146cf/hooks/<hook>/<case>/{stdin.json,stdout,stderr,exit,audit.md,case.json}   # 同上（hooks/cases-missing.json / hooks/provenance.json）
  # 2026-08-22 U1 code-generation Q2 = A（オーナー裁定）: 既存の upstream 配布実バイト置き場 `tests/golden/upstream-3c3146cf/`（README がピン単位ディレクトリ・バイト不変を規定）に統合し、`tests/goldens/` は新設しない。既存の stage-graph.json / scope-grid.json は同ディレクトリ直下のまま不変
provenance: upstream 3c3146cf を bun で実行して採取。再採取スクリプトを同梱（A3）
consumers: U6（continue_token / directive）, U7（CLI・フック・文言）, U4（監査行・状態ファイル差分の突合）
change_policy: upstream ピン更新の intent でのみ更新。差分は逸脱台帳と突き合わせてレビュー
```

## 3. 契約の所有ルール（Q5 / Q7）

- **所有者**: C1/C2 = U7（正本 = upstream 仕様 + U1 ゴールデン）。C3/C4 = 使う側のユースケース層（U5/U6）、
  実装側 U3 は準拠（2026-08-27 補足: C3 のうち**イベントストア trait の所有者は本家**であり、我々は
  実装する側に回る — ADR-010 Conformist）。C5 = 語彙 U2 / 投影規則 U4。
  C6 = ~~U3~~ → **本家 2 表（`journal` / `snapshot`）の正本は upstream、我々の
  `amadeus_projection_checkpoint` のみ U3 所有**（2026-08-27 改訂 / ADR-010）。C7 = U1。
- **破壊的変更の合意**: 所有 Unit の Bolt（PR）で ADR を添えて合意する。外部面（C1/C2）の変更は加えて
  `docs/specs/deviations.md` に登録（D6）。
- **追加的変更の安全**: イベント・スナップショット・directive に追加されたフィールドは消費側が無視する
  （`schema_version` は予約のみ、stage-1 では 1 固定）。trait へのメソッド追加は既定実装を与えるか全実装
  （~~Impl + InMemory~~ → **2026-08-27 訂正 / ADR-010**: 実装は `WorkflowExecutionRepositoryImpl<S>` 1 つで、
  `S`（`open()` = SQLite / `in_memory()` = 本家 memory）の両バックエンドを同じ PR で更新する — 別型の
  InMemory 実装は存在しない）。
- **検証**: C1/C2/C7 はゴールデン一致、C3/C4 はコンパイル（層 = クレート、E0432）+ 契約テスト
  （2026-08-27 改訂: ~~InMemory テスト~~ → `WorkflowExecutionRepositoryImpl::in_memory()` と SQLite の
  **両バックエンドに同じ契約テストを課す**。手順が 1 行も違わないので同じ約束が課せる — BR2.7）、
  C5 は投影テスト（イベント → 行のバイト一致）、C6 は ~~DDL マイグレーションテスト~~ →
  **スキーマガードテスト**（本家 DDL の実測突合。2026-08-27 改訂 / ADR-010）+ ITF 準拠
  （~~改訂版 `audit_lock.qnt`~~ → `journal_protocol.qnt`。ADR-007 で `audit_lock.qnt` は退役）。

## 4. 未解決の契約項目

| Contract | Question | Blocks |
|---|---|---|
| C1 | SQLite ストアファイルの配置（`<record>/.aidlc-store.sqlite` か `aidlc/.aidlc-store/` か）と `.gitignore` への追記先 | U3（ストア初期化）、U9（逸脱台帳の文言） |
| C1 | 楽観 version 競合が 2 回続いた場合の逐語文言（新設。upstream に同状況なし）の文面 | U7（message-catalog 配線） |
| C2 | フック 4 本それぞれの stdin JSON の厳密なスキーマ（upstream の Claude Code フック入力の写し）— ゴールデン採取で確定 | U1（採取）、U7（実装） |
| ~~C3~~ | ~~`EventStore` trait のジェネリクス境界（`AID: Clone + Eq`, `E: Serialize` 等）と `EventStoreError` の変種 — event-store-adapter-rs の同形性をどこまで取るか~~ → **解決（2026-08-27 / ADR-010）**: 同形性を取るのではなく**本家 crate に直接依存して実装する**（Conformist）。境界もエラー型も本家のものをそのまま受け入れる | — |
| C3 / C6 | `within_write_transaction`（登録簿 `intents.json` の read-modify-write を Tx で守る口）は**削除済み** — 本家は接続も Tx も露出しないため実現できない。ADR-010 は「登録簿を SQLite へ移す」を筋と書いているが、**U7 で裁定**する（本 Bolt では未決） | U7 |
| C6 | `busy_timeout` を本家のイベントストア接続に設定できない（我々の `JournalReader` 側の接続には 5000ms を設定済み）。別プロセスの並行書込は待たずに `SQLITE_BUSY` になる。単一プロセス前提の現状は受容し、**U7 の並行モデルと併せて再裁定**する（本 Bolt では未決） | U7 |
| C5 | `Started` の投影（init 3 stage の STAGE_STARTED/COMPLETED をどこまで 1 イベントに含めるか）と `GateApproved` の phase 境界（PHASE_VERIFIED の要否）の厳密な行順 | U4（投影規則）、U1（ゴールデン） |
| C5 | `Jumped` イベントのペイロード（reset/skip 集合をイベントに含めるか、適用時に再計算するか） | U2 |
| C6 | `projection` の名前集合（state-file / audit-shard の 2 つで足りるか、シャード別にするか） | U4 |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-22T10:47:50Z
**Iteration:** 1（advisory）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | C6（SQLite DDL）§2 制約(1) | C6 の楽観 version 制約は「journal INSERT + snapshot **UPDATE** ... WHERE version = :expected を同一 Tx で行い、影響行 0 なら Conflict」とだけ定義している。これは既存スナップショット行がある更新には効くが、集約の**新規作成**（例: `Started` イベントによる intent 生成の初回 `store`）では `snapshot` に対象 `aggregate_id` の行がまだ存在しないため、UPDATE は必ず影響行 0 になり、実際には競合ではないのに `Conflict` を誤って返す。DDL には `INSERT ... ON CONFLICT(aggregate_id) DO UPDATE` のような genesis 経路も、`expected version = 0` のような番人（sentinel）値の扱いも書かれておらず、C3 の trait 契約（`store` の doc コメントは「集約の version が保存時点のジャーナル末尾と一致しなければ Conflict」としか書いていない）にも genesis の特例は無い。実装者は最初の `store` 呼び出しの経路をその場で決めなければならない。 | C6 の DDL コメントか C3 の `store` doc コメントに、集約新規作成時の挙動（例: `INSERT OR IGNORE` 後の条件付き `UPDATE`、または `expected version = 0` は「行が無ければ INSERT、あれば UPDATE...WHERE version=0」と定義する）を明記する。 |
| 2 | Major | C5（ドメインイベント語彙と投影規則） | ステージのチェック観点で名指しされている「監査シャードの HUMAN_TURN 等『直接行』と投影行の順序保証」が、C5 のどこにも記述されていない。`direct_audit_rows`（HUMAN_TURN・DECISION_RECORDED 等、C2 のフックが直接書く行）と、ドメインイベントの投影行（U4 が `report`/`next` 末尾で書く行）は別のプロセス起動（別コマンド呼出）から同一シャードファイルへ追記される。C5 の `rules` はシャード**横断**（他クローン）の順序規約（timestamp ソート + バッファ位置 tiebreak、FR1.1）は定義しているが、同一シャード内で直接行と投影行がどちらの順で現れるべきか（例: ある `report` コマンド実行の直前に発火した `record-human-turn` フックの HUMAN_TURN 行と、その `report` 自体が描く GATE_APPROVED/STAGE_COMPLETED 行の相対順序）についての取り決めは無い。NFR3（監査完全性）の検証対象になり得る点であり、未解決項目表（§4）にも挙がっていない。 | C5 の `rules` に「同一シャード内では直接行と投影行はプロセス起動順（＝ファイルへの追記順）でよく、追加の順序制約は課さない」等、意図的な取り決めを明記するか、§4 の未解決項目表に追加して functional-design（U4）に持ち越す。 |
| 3 | Minor | C3（`EventStore` trait） | `EventStore` の書込系メソッド（`persist_event` / `persist_event_and_snapshot`）は `&mut self` だが、C3 の `WorkflowExecutionRepository::store` は `&self` である。`WorkflowExecutionRepositoryImpl`（U3）が内部で `EventStore` を保持して `store` を呼ぶ構図だとすると、`&self` の中から `&mut self` メソッドを呼ぶには内部可変性（`Mutex`/`RefCell` 等）が要る。契約上は問題ないが、functional-design が実装方針（非同期タスク間で SQLite コネクションをどう共有するか）を明示しないと、複数の実装者が別の内部可変性戦略を選びうる。 | functional-design（U3）で `WorkflowExecutionRepositoryImpl` 内の `EventStore` 保持方法（`tokio::sync::Mutex` 等）を明記する。 |
| 4 | Minor | C6（`checkpoint` テーブル・`JournalReader::checkpoint`） | `checkpoint(projection)` の戻り値型は `Result<GlobalSeqNr, EventStoreError>`（`Option` ではない）だが、`checkpoint` テーブルに当該 `projection` の行がまだ無い場合（初回実行）にどの値を返すかが DDL コメントにも trait doc コメントにも無い。 | 「行が無ければ 0 を返す（未投影の意味）」と C6 の制約コメントか `JournalReader::checkpoint` の doc コメントに明記する。 |
| 5 | Minor | C2 所有ルール（§3） | 本ステージの上流 `unit-of-work.md` の既存 `## Review`（units-generation 段の advisory レビュー、Major 所見#1）は、フック4本・doctor のユースケース層コードの帰属が `components.md` の `EngineUseCases`（クレート分離による DIP 強制点）と U7 の「ロジックを持たない」自己宣言との間で未確定だと指摘済みである。本契約summary は C2 の Owner を無条件に「U7」としているが、もし functional-design でフックのユースケースロジックが別 Unit（新設または U5/U6）に切り出された場合、C2 の Owner 列は改訂が必要になる。現時点では contract-design のスコープ外の carry-over だが、承認前に人間が意識すべき依存関係である。 | functional-design 着手前に units-generation の Major 所見#1（フック/doctor のユースケース帰属）を解消し、その結果に応じて C2 の Owner を確定させる。 |

> （2026-08-23 追記: 所見 3 は前提ごと失効した。U3（Bolt B5）の実装でオーナー裁定により `WorkflowExecutionRepository::store` を `&mut self` へ改訂した（上記 §C3 参照）ため、`&self` の中から `&mut self` を呼ぶための内部可変性の機構そのものが不要になった — 正本 `coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`。あわせて `EventStore` の数値パラメータも `usize` → `u64` へ改訂済み（U3 実装 `modules/core/use-case/src/orchestration/event_store.rs:33,60` 実測）。）
>
> （2026-08-27 追記: 直前の「`usize` → `u64` へ改訂」は**失効した**。ADR-010（Bolt B6）で本家 crate へ乗り換え、ローカル `EventStore` trait を口ごと削除したため、数値は本家の `usize` に戻った — 借り物の契約を我々のドメイン型に合わせて書き換えていたこと自体が `coding-rules/upstream-contracts.md` 違反だった。参照ファイル `event_store.rs` も削除済み。`store` の `&mut self` は不変。）

### Validation Tool Results

| Tool / Check | Result | Interpretation |
|---|---|---|
| §4 統合点 4 境界 ↔ 契約表 C1〜C7 の突合（手動） | 一致 | `unit-of-work-dependency.md` §4 の4境界（ポート trait／ドメインイベント語彙と投影規則／SQLite スキーマ／CLI・directive・フック）はそれぞれ C3+C4／C5／C6／C1+C2 に 1:1 対応。外部契約は Q1=A どおり C1・C2 のみで、内部境界（C3〜C6）と非対称に扱われている点も Q1 回答と整合 |
| DirectiveKind 10 種 ↔ C1 の `directive.kinds` 列挙（`modules/shared/directive-schema/src/lib.rs` 実コード突合） | 一致 | `load-steering, run-stage, dispatch-subagent, invoke-swarm, present-gate, ask, print, error, done, parked` の10種が完全一致 |
| `WorkflowDefinitionRepository::find` ↔ C4 の trait シグネチャ（既存コード `modules/core/use-case/src/orchestration/workflow_definition_repository.rs` 突合） | 一致 | 既存 trait は `fn find(&self) -> Result<WorkflowDefinition, GraphReadError>` で C4 のシグネチャと同一。「既存・変更なし」の記述も正しい |
| `store` 動詞の coding-rules 適合性（`gateway-taxonomy.md` §2b・ADR-006 突合） | 一致（ADR による明示的例外） | §2b の許容動詞（find_by_id/find/save/remove）に `store` は無いが、ADR-006 が ES 拡張語彙として明示的に採用し、正本注記を U9 FR8.1 に同梱すると裁定済み。C3 の記述「動詞 `store` は ES 拡張語彙（ADR-006。正本注記は U9 FR8.1）」は ADR 本文と一致 |
| EventStore trait の4メソッド ↔ ADR-001 Decision 節の event-store-adapter-rs API 列挙 | 一致 | `persist_event` / `persist_event_and_snapshot` / `get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr` が ADR-001 の列挙と1:1 |
| EVENT_HEADINGS/FIELD_ORDER・86語彙 ↔ 実コード（`modules/shared/audit-events/src/lib.rs`）突合 | 一致 | ファイル冒頭コメントが「`EventType` 86 語 22 カテゴリの閉集合」と明記しており、C5 の「86 語彙」記述と一致 |
| C6 楽観 version 制約の genesis（新規集約作成）経路の机上トレース | 不整合（所見1） | DDL コメント #(1) の UPDATE-only 定義では初回 `store` が誤って `Conflict` になる |
| ADR-002（decide は `&mut self`・単一イベント）↔ C3 trait 群の整合 | 一致 | `store`（Repository、`&self`）と集約の `decide`（`&mut self`）は別レイヤーの操作であり矛盾なし。ユースケースが `find_by_id` で取得した所有集約に対し `decide` → `store` の順で呼ぶ運用と整合 |

### Summary

契約表は §4 の4境界（ポート trait・イベント語彙/投影規則・SQLite スキーマ・CLI/フック）を過不足なく形式化しており、DirectiveKind・既存 `WorkflowDefinitionRepository`・EventStore API・EVENT_HEADINGS 語彙数など実コード/ADR との突合はすべて一致した。ただし C6 の楽観 version 制約が集約の新規作成（genesis）経路を書き落としており（所見1、Major）、初回 `store` が実装次第で誤って競合エラーになりうる。また、チェック観点で明示された「直接行と投影行の順序保証」が C5 に欠落している（所見2、Major）。advisory 判定の閾値（Major ≤2）内であり構造的な健全性は保たれているため READY とするが、承認前にこの2件の Major と、内部可変性・checkpoint 初期値・C2所有権 carry-over の Minor 3件を人間が重みづけされたい。
