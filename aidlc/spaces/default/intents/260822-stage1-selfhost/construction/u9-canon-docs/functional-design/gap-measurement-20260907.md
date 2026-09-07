# gap-measurement — 現行コードを基準にした正本・仕様・共有契約のずれ（U9 再走、2026-09-07）

> オーナー指示「あいまいな提案するな。コードを実測して提案しろ。現状のコードを基準だろ」（2026-09-07）に応じた実測記録。
> 基準 = `main` `02cacea2`（b51 マージ後）の作業ツリー。測り方 = `grep` / `awk` による型・関数・変種の列挙と、各文書の該当行の突合。
> 本書は U9 functional-design の補助記録であり、正本（rules.md / entities.md）の代替ではない。

## 1. コードの現状（基準）

| 項目 | 実測 |
|---|---|
| クレート（10） | `core-command-domain` / `core-command-use-case` / `core-command-interface-adapter` / `core-query-use-case` / `core-query-interface-adapter` / `core-read-model-updater` / `core-infrastructure` / `aidlc`（app）/ `harness-claude` / `harness-infrastructure`（`Cargo.toml` members） |
| 集約（4） | `Intent`（`orchestration/intent.rs`、7 属性: id / definition_id / definition_revision / start_request / stages: `StageEntries` / scan / created_at。genesis `create` + `replay`、イベント `IntentEvent::Created` 1 変種）、`IntentExecution`（`intent_execution.rs`、12 属性: id / intent_id / slots: `StageSlots` / cursor / status / parked_at / autonomy / skeleton_stance / last_gate_resolution_at / seq_nr / **version** / last_updated_at）、`WorkflowDefinition`（8 属性: id / revision / graph / grid / scopes / seq_nr / version / last_updated_at。`define` / `redefine` / `replay`、イベント `Defined` / `Redefined`）、`CompiledDefinition`（5 属性: id / revision / graph / grid / scopes。`compile` / `recompile` / `register_scope` / `apply_plugin_selection`、イベント `Compiled` / `Recompiled` / `ScopeRegistered` / `PluginSelectionApplied`） |
| `IntentExecution` のコマンド（`&mut self`、15 + genesis） | `start`（static、`(id, &Intent, at) -> (IntentExecution, IntentExecutionEvent)`）/ `open_gate` / `approve_gate` / `reject_gate` / `revise_stage` / `skip_stage` / `jump` / `park` / `unpark` / `recompose` / `switch_autonomy` / `record_single_stage_run` / `record_skeleton_stance` / `request_review` / `record_review_verdict` / `affirm_practices`（＋ `apply_event` / `apply_report`） |
| `IntentExecutionEvent`（16 変種） | `Started` / `GateOpened` / `GateApproved` / `GateRejected` / `StageRevised` / `StageSkipped` / `Jumped` / `Parked` / `Unparked` / `Recomposed` / `AutonomyModeSet` / `SingleStageRunCommitted` / `SkeletonStanceRecorded` / `ReviewRequested` / `ReviewCompleted` / `PracticesAffirmed` |
| `IntentExecution` のクエリ（`&self`） | `next_decision(&self, &Intent, &NextRequest) -> Result<NextDecision, CommandError>`（別 intent は `IntentMismatch` — b51）/ `report_dispatch` / `jump_resolve(&self, &Intent, StageIndex)` / `state_binding` / `human_acted_since_gate` / `effective_plan` / `gated` / `parked_active` / `accepts_commands` / `stale_report` / `checkbox` / `approved` / `revision_count` / `stage_key` / `stage_index` / `review_attempt` / `first_in_scope_of_phase` ほか |
| 再構成 | `IntentExecution::replay(snapshot: IntentExecution, events) -> IntentExecution`（最新スナップショットを基底に、その `seq_nr` より後のイベントを再生）。Repository 実装 `intent_execution_repository_impl.rs:340-438` は `get_latest_snapshot_by_id` → `get_events_by_id_since_seq_nr` → `replay(base, events).with_version(version)`（2026-09-05 実測 43 テスト、オーナー裁定で確定）。memento 型・`state()` / `from_state()` は存在しない。DTO ↔ 集約の変換はアダプタ層（`IntentExecution::new(...) -> Result<_, IntentExecutionError>`、11 引数） |
| 楽観 version の置き場 | **集約の内側**（`version: usize` フィールド、`version()` / `with_version()`、`UNPERSISTED_VERSION = 0`）。`Rehydrated*` 型はコードに 0 件。`store(&mut self, &event, &aggregate)` は `aggregate.version()` を期待版として本家へ渡す（`impl:455`） |
| `# Panics` | `intent_execution.rs` 3 か所（`From<(Started, at)>` / `apply_event` / `replay`）+ `workflow_definition.rs` 1 か所。error-handling.md「再構成は失敗を返さない」の例外どおり |
| ファーストクラスコレクション（16 型） | orchestration 8: `StageEntries` / `StageSlots` / `StageIndexSet` / `StageSlugSet` / `ArtifactPaths` / `TransitionSteps` / `ReviewClosures` / `PendingIterations`。workspace 6: `Checkboxes` / `AuditFields` / `BoltRefs` / `OrderedAuditEvents` / `PromotedSections` / `RuleLines`。workflow-definition 2: `StageGraph` / `ScopeGrid`（契約ハーネス `tests/collection_contract_test.rs`） |
| コマンド側ポート（4、`core-command-use-case/orchestration/port/`） | `IntentExecutionRepository { find_by_id(&IntentExecutionId), store(&mut self, &IntentExecutionEvent, &IntentExecution) }`、`IntentRepository { find_by_id, find_for_execution(&IntentExecution), store }`、`WorkflowDefinitionRepository { find_by_id, find_for_intent(&Intent), store }`、`CompiledDefinitionRepository { find_by_id, store }`。エラーは `RepositoryError<Id>` 1 本。`expected_version` 引数は無い |
| コマンド側ユースケース（9） | `CommitVerdict`（= report）/ `CreateIntent` / `DefineWorkflow` / `Park` / `PromotePractices` / `RecordReview` / `RecordSingleStageRun` / `RecordSkeletonStance` / `SwitchAutonomy`。テストダブル型 `InMemoryXxxRepository` は無く、`XxxRepositoryImpl::in_memory()`（本家 memory バックエンド）で組む |
| クエリ側 | ユースケース 13（`FindNextAnswer` / `FindContinuation` / `FindRunStage` / `FindSteering` / `FindDefinition` / `FindDefinitionStage` / `FindScope` / `FindScopeKeyword` / `FindPhaseEntry` / `FindJump` / `FindExecution` / `FindScopeChange` / `FindStateFile`）、DAO ポート 14（`ExecutionDao` / `NextAnswerDao` / … 1 表 1 引当）、実装 = SQLite DAO 14 + `InMemoryXxxDao` 13。`Directive` / `DirectiveKind` / `ContinueToken` / `StateBinding` の型は `core-query-use-case` に住む（コマンド側ドメインではない） |
| RMU（中間クレート） | `JournalReader { events_after / events_through / checkpoint / advance_checkpoint / publish / pending_publication / steering_source_digest / replace_steering / prepare_read_model }`。`IntentExecution::replay` / `WorkflowDefinition::replay` で集約を起こして投影。`read_*` 17 表（definition 6 / execution 2 / intent 2 / next 3 / run_stage / scope_change / steering 2）+ `amadeus_projection_checkpoint` / `amadeus_read_model_head` / `amadeus_publication*` 5 表 |
| CLI（`aidlc` マルチコール） | 面 `aidlc-bolt` / `aidlc-log` / `aidlc-state` / `aidlc-utility`、動詞 `next` / `continue` / `report` / `park` / `compose` / `init` / `intent-create` / `link` / `decision` / `answer` / `review` / `set-autonomy` / `practices-promote`。読取前に RMU `catch_up`（`runtime.rs:194,200`）。`unpark` / `jump` / `recompose` / フック 4 本 / `doctor` は未配線（クリティカルパス 4〜6 の予定どおり — 文書の誤りではない） |
| BR5.1 の sentinel 8 語 | `docs/specs/*.md` = 0 件。`coding-rules/*.md` は gateway-taxonomy の履歴行 3 件（20 / 290 / 291）のみ |

## 2. 文書側のずれ（文書の主張 ≠ コード）

凡例: **要改訂** = 現行規範として書かれていてコードと食い違う。**予定** = 未実装の計画（食い違いではないが「予定」と明記が要る）。**履歴** = 打消し線・「旧」注記で既に履歴化済み（触らない）。

### 2.1 `docs/specs/10-orchestration.md`（263 行、`WorkflowExecution` 13 件）

| 行 | 現在の主張 | コード | 処置 |
|---|---|---|---|
| 3-15 | 冒頭の B12 読み替え注記・B13 優先順位注記（「全文追従は後続 Bolt」） | 該当 Bolt 未実施 | 本文へ畳み込み、注記を「追従済み（2026-09-07 / U9 再走）」の 1 行に縮める |
| 43-46 | §2.1 見出し「集約: `WorkflowExecution`」、identity `IntentId`、実装パス `modules/core/domain/` | `Intent`（`IntentId`）+ `IntentExecution`（`IntentExecutionId`、`intent_id` で参照）、`modules/core/command/domain/` | 要改訂（2 集約へ分割して記述） |
| 48 | 「16 属性。`version` 列を除去、`RehydratedWorkflowExecution` が持ち回る」、`stages: Vec<StageEntry>` / `plan` / `conditional` 展開列 | `IntentExecution` 12 属性（`version` は集約内）、計画は `Intent.stages: StageEntries`、実行側は `slots: StageSlots`（FCC）、`Rehydrated*` 型なし | 要改訂 |
| 50 | コマンド（11） | 15 + `start` | 要改訂（5 件追加: record_single_stage_run / record_skeleton_stance / request_review / record_review_verdict / affirm_practices） |
| 51 | ドメインイベント（11） | 16 変種 | 要改訂（5 件追加） |
| 52 | メメント `state()` / `from_state()`、`WorkflowExecutionState`、serde は memento 経由 | memento 型なし。復号検査点は `IntentExecution::new`（アダプタの DTO から）、再生は `replay(snapshot, events)` | 要改訂（B13 注記で非規範化済みだが本文に残る） |
| 55 | 「`next_decision` は失敗しないクエリ `(&self, &NextRequest) -> NextDecision`」 | `(&self, &Intent, &NextRequest) -> Result<NextDecision, CommandError>`（`IntentMismatch`、b51 = 2026-09-07 マージ） | 要改訂 |
| 56 | Tx 境界「ジャーナル追記 + スナップショット更新を同一 Tx」 | 一致（本家 `persist_event_and_snapshot`） | 維持 |
| 70 | §2.2 `EffectivePlan` 行「集約 `WorkflowExecution` の `effective_plan`」 | `IntentExecution::effective_plan(&self, ..)` | 名称のみ改訂 |
| 66-76 | §2.2 `DirectiveKind` / `Directive` / `ContinueToken` / `ProgressSignature` / `DirectiveMaxBytes` を orchestration の Domain Primitive として列挙 | 実装は `core-query-use-case`（クエリ側の View / DTO）。コマンド側ドメインには `NextDecision` / `NextRequest` / `StateBinding` 材料 | 「所在 = クエリ側」を列に追加（構造の規範、逐語契約は不変） |
| 82-87 | §2.3 表・注記の `WorkflowExecution`（3 件）、`next_decision(&NextRequest)` | `IntentExecution`、`(&Intent, &NextRequest)` | 名称・署名を改訂 |
| 93 | §3 ユースケース列（`Next` / `Continue` / `Report` / `Park` / `Unpark` / `JumpResolve` / `JumpExecute` / `SetAutonomy` / `Recompose` / …） | コマンド側 9（表記は `CommitVerdict` / `SwitchAutonomy` 等）、`Next` / `Continue` はクエリ側 `Find*` 13、`Unpark` / `Jump*` / `Recompose` は未実装 | 実装名へ揃え、未実装は**予定**と明記 |
| 99-101 | 「1 trait 1 Impl（`XxxRepositoryImpl` + `InMemoryXxxRepository`）」「`WorkflowDefinitionRepository` は `InMemoryWorkflowDefinitionRepository` を持つ」 | テストダブル型は無い（`in_memory()` バックエンド）。クエリ側だけ `InMemoryXxxDao` | 要改訂 |
| 105 | ポート表 `WorkflowExecutionRepository` 行（`store(.., expected_version: usize)`、`Rehydrated…` を返す、`find_by_id(&IntentId)`） | `IntentExecutionRepository { find_by_id(&IntentExecutionId) -> IntentExecution, store(&mut self, &event, &aggregate) }` | 要改訂 |
| 106 | `WorkflowDefinitionRepository` 行「`save` は持たない」「`GraphReadError`」 | `find_by_id` / `find_for_intent` / `store`、エラーは `RepositoryError<WorkflowDefinitionId>` | 要改訂 |
| （欠落） | — | `IntentRepository` / `CompiledDefinitionRepository` の 2 ポート | 2 行追加 |
| 110 | 「監査台帳は `WorkflowExecution` のイベントログ」 | `IntentExecution` | 名称のみ |
| 137 / 205-206 / 236 | I8・実装順序・S1 の `WorkflowExecution` / `InMemory…` | 同上 | 名称のみ（205-206 は履歴化） |

### 2.2 `docs/specs/12-workflow-definition.md`（279 行、`WorkflowExecution` 9 件）

| 行 | 現在の主張 | コード | 処置 |
|---|---|---|---|
| 3-15 | 冒頭 2 注記 | — | 畳み込み |
| 42-56 | §2.1 `WorkflowDefinition` / `CompiledDefinition` | 一致（属性・イベント・ID の別型） | 維持 |
| 77-93 | §2.3 述語 6 + `grid().action()`、`WorkflowExecution` の `effective_plan` / `stages`（89, 91） | 述語は一致（加えて `scope_cost` / `review_policy` / `stage_route` が集約のクエリ）。所有者は `IntentExecution::effective_plan`、計画は `Intent.stages: StageEntries` | 名称改訂 + クエリ 3 件追記 |
| 164-165 / 215 / 221 | §4 #7・§8 F2 / F8 の `WorkflowExecution` | `IntentExecution` / `Intent` | 名称のみ |
| 178 | §5 ユースケース「`LoadStageGraph` / `LoadScopeCatalog` / `ResolveScopePlan`（読み取り専用）」 | コマンド側 `DefineWorkflowUseCase`、クエリ側 `FindDefinition` / `FindDefinitionStage` / `FindScope` / `FindScopeKeyword` / `FindPhaseEntry`、RMU `read_definition*` 6 表 | 要改訂 |
| 183 | `WorkflowDefinitionRepository` 動詞 2 | `find_for_intent(&Intent)` もある | 1 動詞追記 |

### 2.3 `docs/specs/11-workspace.md`（227 行、`WorkflowExecution` 7 件）

| 行 | 現在の主張 | コード | 処置 |
|---|---|---|---|
| 3-15 | 冒頭 2 注記 | — | 畳み込み |
| 41-49 | §2.1 集約 `Intent` / `Space` / `Worktree`（登録簿 `intents.json` の Intent） | workspace モジュールに集約は無い（値オブジェクト + FCC のみ）。コードの `Intent` は orchestration の静的 intent 集約であり登録簿の Intent とは別物 | **予定**と明記し、orchestration の `Intent` との関係（同名・別物）を注記 |
| 47 / 49 | 「監査台帳は `WorkflowExecution` のイベントログ」「`WorkflowExecution` 集約の書込は SQLite Tx」 | `IntentExecution` | 名称のみ |
| 51-69 | §2.2 Domain Primitive（`IntentId` UUIDv7 / `IntentDirName` / `CheckboxState` / `BoltRefs` / `StateVersion` / `AuditFieldKey` …） | 一致（`intent_id.rs` は UUIDv7 検証、`intent_dir_name.rs` あり） | 維持 |
| 70-85 | §2.3 描画関数は RMU へ移動済み、順序付け純関数はドメイン | 一致（`OrderedAuditEvents` FCC / `StateVersionClassification`） | 維持 |
| 101 | §3 ポート表 `WorkflowExecutionRepository` 行（`expected_version` 引数） | `IntentExecutionRepository`（署名は 2.1 と同じ） | 要改訂 |
| 111-116 | 供給面 `WorktreeService` / `OpaqueFlagStore` / `ScopedStorage` / `SessionStampStore` | 未実装 | **予定**と明記 |
| 126 / 195 | §4 Gateways 末尾・§7 実装順序の `WorkflowExecutionRepositoryImpl` / `InMemory…` | `IntentExecutionRepositoryImpl`、`in_memory()` | 名称のみ |

### 2.4 `docs/specs/01-domain-model.md`（290 行、`WorkflowExecution` 8 件）

| 行 | 現在の主張 | コード | 処置 |
|---|---|---|---|
| 3-15 | 冒頭 2 注記 | — | 畳み込み |
| 96 | §3.1 状態機械「所有者は集約 `WorkflowExecution`」 | `IntentExecution::effective_plan` | 名称のみ |
| 102 | §3.2 集約「`WorkflowExecution`、遷移動詞 11、decide 12 コマンド、16 属性、イベント 12 種、メメント `WorkflowExecutionState`、`Rehydrated…`」 | `Intent`（7 属性、`Created`）+ `IntentExecution`（12 属性、15 コマンド、16 変種、memento なし、`version` は集約内） | 要改訂（段落を書き直す） |
| 106 | Domain Primitive 候補「`WorkflowExecution::start` が焼き込む」 | `Intent::create(id, &WorkflowDefinition, StartRequest, WorkspaceScan, at)` が `StageEntries` を確定、`IntentExecution::start(id, &Intent, at)` | 要改訂 |
| 116 | §3.3 集約 `Intent` / `Space` / `Worktree` | 未実装（11 号 §2.1 と同じ） | **予定**と明記 |
| 121 | 脚注「U2 実装の `IntentId::parse` は kebab を受理、B5 で是正」 | 是正済み（UUIDv7 検証） | 履歴化（「是正済み 2026-08-23〜」） |
| 184 / 231 | §4 B1・§6 の `WorkflowExecution` | `IntentExecution` | 名称のみ |
| 278 | §7.1 原則 5「`WorkflowExecution` が `WorkflowDefinition` を `definition_id` で参照」 | `Intent` が `WorkflowDefinition` を `definition_id` で、`IntentExecution` が `Intent` を `intent_id` で参照 | 要改訂（例を 2 段に） |

### 2.5 `coding-rules/`（FR8.1 の範囲 — 語彙の自己矛盾）

| ファイル:行 | 現在 | コード | 処置 |
|---|---|---|---|
| `README.md:50` | error-handling 行「利用者向け文言はアダプタ層（message-catalog）」 | message-catalog は 2026-08-29 解体、出す側の `wording` | 要改訂（本文 12 行目と同期） |
| `error-handling.md:4` | 適用例「core-domain `CommandError` / `ApplyError` / `StartError` / `SnapshotError`」 | `StartError` / `SnapshotError` は存在しない。現行 `CommandError` / `ApplyError` / `IntentError` / `IntentExecutionError` / `PlanError` / `StageSlotsError` / `TransitionStepsError` / `PromotionPlanError` …（`core-command-domain`） | 要改訂 |
| `factory-naming.md:47` | 反例「`WorkflowExecutionState` はリテラル 2 箇所 … 次 Bolt で是正」 | 型ごと消滅（是正済み） | 履歴化 |
| `factory-naming.md:84` | 例「`WorkflowExecution::with_version`」 | `IntentExecution::with_version` / `WorkflowDefinition::with_version` | 名称のみ |
| `gateway-taxonomy.md:223` | 改訂注記の本文「書込モデル（集約 `WorkflowExecution` + `EventStore`）」 | `IntentExecution` | 名称のみ |
| `gateway-taxonomy.md:299` | 「`core-domain` に存在する集約ルート型名」 | `core-command-domain` | クレート名 |
| `module-visibility.md:11,21` | `core_domain::{workspace, …}` / `core_domain::workspace::CheckboxState` | `core_command_domain::…` | クレート名 |
| `module-visibility.md:12` | 例「`message_catalog::{state, lock, bolt}`」 | 解体済み。現行の共有語彙は `core_infrastructure::canon_json` 等 | 例を差し替え |
| `use-case-rules.md:11` | 「`core-use-case` の Cargo.toml に `core-interface-adapter` が無い」 | `core-command-use-case` / `core-command-interface-adapter` | クレート名 |
| `use-case-rules.md:36` | 「実装は実質 2 つ（Impl + InMemory）… `XxxUseCase<InMemoryXxxRepository>`」 | コマンド側は `XxxRepositoryImpl<S>` の `in_memory()`、クエリ側は `InMemoryXxxDao` | 要改訂 |

`command-query-separation.md:5` / `interior-mutability.md:5` / `factory-naming.md:5` / `domain-equality.md:4` / `tell-dont-ask.md:46` の `WorkflowExecution…` / `OwnerStamp` / `reap_eligible` は是正の経緯・退役の履歴として書かれており**触らない**。gateway-taxonomy §1b（旧 BR1.5）は一般形へ改訂済み。

### 2.6 共有契約（Inception 成果物）

| ファイル | ずれ | 規模 |
|---|---|---|
| `inception/domain-design/components.md`（458 行） | 冒頭注記「再構成はジャーナル全再生」（R-02、コードは差分再生）。YAML: `OrchestrationEngine` の集約 `WorkflowExecution` + イベント 11 / `EngineUseCases`（`NextUseCase` / `ReportUseCase` / `ContinueUseCase` / `DoctorUseCase` / フック 4）/ `WorkflowDefinitionModel`「ES 対象外・find のみ・`next_in_scope_stage`」/ `PublishedLanguage` の message-catalog / `RehydratedWorkflowExecution`。**クエリ側（`core-query-*`、DAO 14、`Find*` 13）と `CompiledDefinition`・`Intent` 集約・`read_*` 17 表がコンポーネントとして存在しない** | 全面改訂（M） |
| `inception/contract-design/contract-summary.md`（523 行） | C3: trait 全文が v2 世代（`WorkflowExecutionRepository`、`find_by_id(&IntentId)`）+ 追記 4 層、2026-08-30 追記「ジャーナル全再生」（R-02）、`Rehydrated…` 3 件。C4: `WorkflowExecution::definition_id()`（→ `Intent`）。C5: `WorkflowExecutionEvent` 11 変種列挙（→ `IntentExecutionEvent` 16）。C6: 「16 属性」注記、`read_*` / `amadeus_read_model_head` / publication 表なし。C1・§4: message-catalog 3 件 | 節単位の現行化（M） |

## 3. 作業パッケージ（すべて文書のみ・コード変更なし）

| # | 内容 | 対象 | 規模 |
|---|---|---|---|
| P1 | U9 設計記録の同期（本ステージの成果物） | `rules.md`（BR3.3 / BR4.1 を現行へ、BR1.5 新設、BR2.5 範囲、BR5.1 grep・diff 範囲、P2〜P4 の BR 新設）、`entities.md`、`functional-spec.md` §6、`traceability.json`（FR8 親行） | S |
| P2 | coding-rules 10 箇所（§2.5） | 6 ファイル、約 12 行 | S |
| P3 | 仕様 4 号の全文追従（§2.1〜2.4） | 10 号 約 20 行 / 12 号 約 9 行 / 11 号 約 8 行 / 01 号 約 9 行 + 冒頭注記 4 × 2 | M |
| P4 | 共有契約 2 本の現行化（§2.6） | `components.md` 全面、`contract-summary.md` C1 / C3 / C4 / C5 / C6 / §4 | M |

## 4. Bolt / Unit 記録の全数探索から得た裁定台帳（検証済み — 「正しい姿」の正本）

オーナー質問「bolt か unit を全部探索して正しい姿に仕様としてる？？」（2026-09-07）への応答。construction 配下 29 ディレクトリ・224 ファイル、
handoff 16 本、`inception/domain-design/decisions.md`（ADR-001〜011）を 4 区画に分けて全数読み（`survey-20260907/{shift-bolts,late-bolts,u2-u3,other-units}.md`、
計 292 行の一次台帳）、構造の規範に当たる裁定を抽出したうえで、**1 件ずつ現行コード（`main` `02cacea2`）で実否を確認**した。
本節は検証済みの統合版であり、後続の裁定が前の裁定を上書きしている場合は現行だけを載せ、上書き関係は §4.8 に記す。
「検証」列: ✓ = コードで一致を確認、予定 = 記録上の設計であり未実装（仕様には「予定」と明記する）、— = 文書のみの裁定。

### 4.1 orchestration — 集約・イベント・クエリ（10 号 §2、01 号 §3.2、C5）

| # | 裁定（現行） | 出典（一次台帳） | 検証 |
|---|---|---|---|
| O1 | 集約 `WorkflowExecution` は **`Intent`（静的 intent）+ `IntentExecution`（1 回の実行）** に分割。1 intent : n 実行。実行は `intent_id: IntentId` で intent を ID 参照し、計画が要るコマンド・クエリは `&Intent` を引数で受け、`intent.id() != self.intent_id` は `Err(CommandError::IntentMismatch)` | shift #50-52、u2u3 #1 | ✓ `intent.rs` / `intent_execution.rs`、`matches(intent)` ガード |
| O2 | `Intent` 7 属性（id / definition_id / definition_revision / start_request{scope, request, depth?, test_strategy?, review?} / stages: `StageEntries` / scan: `WorkspaceScan` / created_at）。genesis `Intent::create(id, &WorkflowDefinition, StartRequest, WorkspaceScan, at) -> Result<(Intent, IntentEvent), IntentError>`、イベント `IntentEvent::Created` 1 変種（intent の全属性を運ぶ）、再構成 `Intent::replay` | shift #54、late #1 | ✓ |
| O3 | `IntentExecution` 12 属性（id / intent_id / slots: `StageSlots` / cursor / status / parked_at / autonomy / skeleton_stance / last_gate_resolution_at / seq_nr / **version** / last_updated_at）。旧 7 並列列は `StageSlots` に統合、要素 `StageSlot` 7 欄（key / plan_action / checkbox / approved / revision_count / review_attempt / practices_affirmed） | u2u3 #11、late #41 #51 #61 | ✓ |
| O4 | コマンド（`&mut self`、15）+ genesis `start(id, &Intent, at) -> (IntentExecution, IntentExecutionEvent)`: open_gate / approve_gate / reject_gate / revise_stage / skip_stage / jump / park / unpark / recompose / switch_autonomy / record_single_stage_run / record_skeleton_stance / request_review / record_review_verdict / affirm_practices。1 コマンド 1 イベント、拒否は `Err` で状態不変 | shift #2、u2u3 #19、late #39 #40 #48 #61 | ✓ |
| O5 | `IntentExecutionEvent` **16 変種**: Started / GateOpened / GateApproved / GateRejected / StageRevised / StageSkipped / Jumped / Parked / Unparked / Recomposed / AutonomyModeSet / SingleStageRunCommitted / SkeletonStanceRecorded / ReviewRequested / ReviewCompleted / PracticesAffirmed。`StageCompleted` は撤去（#85 = A）。監査語彙の `EventType::StageCompleted` は残る | late #16 #41 #48 #62、u2u3 #14 | ✓ 16、`audit_events.rs` に `StageCompleted` 残存 |
| O6 | ドメインイベントはエンティティ: 全変種 `{ id: XxxEventId（UUIDv7、集約のコマンド内で採番）, aggregate_id: XxxId, .. }`。`seq_nr` / `occurred_at` は本家封筒が運び `apply_event(seq_nr, at, &event)` へ渡す。復号境界（Repository / RMU）は行 `aid` と payload `aggregate_id` を全変種で照合し不一致は `Corrupt` | shift #5 #6、late #8-10 | ✓ |
| O7 | `Started` は `id` / `aggregate_id` / `intent_id` / `stages: StageEntries`（解決済み計画の写し）を運び、`From<(Started, at)>` が誕生状態を導く（initialization は誕生時 Completed・approved=false）。scan と表示属性は `Created` が運ぶ | shift #18、late #1、u2u3 #16 #17 | ✓ `started.rs` / `created.rs` |
| O8 | 楽観 version は**集約の内側**（`version: usize`、`version()` / `with_version()`、`UNPERSISTED_VERSION = 0`）。ストアが採番する不透明トークンで `seq_nr` と別物。`Rehydrated*` / `StatePosition` / `StoreVersion` は廃止 | u2u3 #21 #24、late #74 | ✓ port/mod.rs に「すべて廃止済み」 |
| O9 | 再構成 = **最新スナップショット + その seq_nr より後の差分イベント**（`IntentExecution::replay(snapshot, events)`）。memento 型・`state()` / `from_state()` は無く、DTO → 集約は `IntentExecution::new(...) -> Result`（アダプタ）。壊れた歴史（`replay` / `apply_event` / 誕生変換）は panic が正（`# Panics` 3 か所 + `WorkflowDefinition` 1） | u2u3 #20 #22、shift #67-68 | ✓ `impl:340-438`、`intent_execution.rs:40` |
| O10 | クエリ（`&self`）: `next_decision(&Intent, &NextRequest) -> Result<NextDecision, CommandError>`（8 変種: RunStage{stage, gate: GateDecision} / Done / Parked / UnparkThenResume / ResumeMenu / NewWorkRouting / RecoverSkipInconsistency / InconsistentSkip。`NextRequest` = resume / reentry / free_text）、`report_dispatch(&Intent, &ReportRequest)`（→ `ReportDecision::Commit{stage, steps: TransitionSteps, scope}` / `NoOp`、拒否 `ReportRefusal` 13 変種）、`jump_resolve(&Intent, StageIndex) -> Result<JumpDirection, _>`、`first_in_scope_of_phase(PhaseId)`、`skeleton_gate_stage(&Intent)` = `stages.first_of(Construction, EXECUTE)`、`human_acted_since_gate(&HumanTurns)`（同秒 fail-closed）、`effective_plan(StageIndex) -> Option<PlanAction>`、`gated(&Intent, StageIndex) -> Option<bool>`（phase ≠ initialization）、`parked_active`、`stale_report(StageIndex) -> Result<(), _>`、`state_binding`、`checkbox` / `approved` / `revision_count` / `review_attempt` | u2u3 #13 #42-43 #64、late #4 #31-34 #43 #67 #70 | ✓ 署名すべて実測 |
| O11 | `GateDecision` = Gated / Ungated / Unresolved。`TransitionStep` 8（GateStartRecovered / GateStart / Approve / Reject / Revise / Skip / Advance / CompleteWorkflow）。`Status` = Running / Completed。`AutonomyMode` = Autonomous / Gated。`SkeletonStance` = On / Off / ScopeDependent。`JumpDirection` = Forward / Backward / Redo | late #33 #43 | ✓ |
| O12 | ガード順: `park` = 取り違え → RefusedUnderAutonomy → NotRunning（`parked_active` を見ない = 再スタンプ可）。`switch_autonomy` は Running ガードなし、Autonomous かつ guard 有効なら `human_acted_since_gate` で HumanPresenceRequired。`approve_gate` は checkbox 前提 → 段 12 昇格受領証 → 段 11 レビュー終端受領証（`ReviewPolicy` 引数）。jump は全ステージのレビュー試行を消す（`reset_attempts_all`） | late #27 #53 #61 #68、u2u3 #38 #39 #41 #63 | ✓ |
| O13 | ファーストクラスコレクション（orchestration 8）: `StageEntries`（計画検査の唯一の場所）/ `StageSlots` / `StageIndexSet` / `StageSlugSet`（辞書順。文書順が要る監査行は RMU `in_document_order` で並べ直す — 本番呼出 1 箇所）/ `ArtifactPaths` / `TransitionSteps` / `ReviewClosures` / `PendingIterations`（`pub(crate)`）。`combine` / `divide` は集合型のみ（orchestration 2 + workspace 2 = 4 型） | u2u3 #6-#12 #65-69、late #79 | ✓ |
| O14 | `PlanAction` の所有は workflow-definition。orchestration に定義も再輸出も無い | shift #9、u2u3 #45 | ✓ |
| O15 | `Directive`（構築可能 7: LoadSteering / RunStage / Ask / Print / Error / Done / Parked）、`DirectiveKind`（10 — dispatch-subagent / invoke-swarm / present-gate は placeholder）、`ContinueToken`（HMAC 封筒、codec はアダプタ）、`StateBinding` は **クエリ側 `core-query-use-case`** に住む。コマンド側ドメインは `state_binding()` の材料だけ | shift #71-72 | ✓ `directive.rs` / `directive_schema.rs` |

### 4.2 ポート・ユースケース・アダプタ（10 号 §3、11 号 §3、12 号 §5-6、C3 / C4）

| # | 裁定（現行） | 出典 | 検証 |
|---|---|---|---|
| P1 | コマンド側ポート 4（`core-command-use-case/orchestration/port/`）: `IntentExecutionRepository { find_by_id(&IntentExecutionId) -> IntentExecution, store(&mut self, &IntentExecutionEvent, &IntentExecution) }`、`IntentRepository { find_by_id, find_for_execution(&IntentExecution), store }`、`WorkflowDefinitionRepository { find_by_id, find_for_intent(&Intent), store }`、`CompiledDefinitionRepository { find_by_id, store }`。`expected_version` 引数は無い（版は集約が運ぶ）。関連取得は `find_for_*` に限る | u2u3 #24、late #76、shift #53 | ✓ |
| P2 | `RepositoryError<Id>` 1 本、4 変種 NotFound{id} / Conflict{expected, actual} / Io{kind, path} / Corrupt{id, seq_nr, source}。`Corrupt` の分類はアダプタ私有（`Error::source` 連鎖） | u2u3 #26 #63 | ✓ |
| P3 | `store` は I/O 前に `event.aggregate_id == aggregate.id` を照合し不一致は `Corrupt(WriteContract)`（3 実装とも）。スナップショットは `SnapshotStrategy::every(n)`、既定 10、初回必須。更新も `persist_event_and_snapshot` | u2u3 #25 #27 #75、shift #34 | ✓ |
| P4 | テストダブル型 `InMemoryXxxRepository` は無い。`XxxRepositoryImpl<S>` の `in_memory()`（本家 memory バックエンド）と SQLite に同じ契約テストを課す。内部可変性なし、`store` は `&mut self` | u2u3 #28 #71、shift #26 | ✓ |
| P5 | コマンド側ユースケース 9: CommitVerdict（= report。`execute(&IntentExecutionId, ReportRequest, at) -> Result<CommitOutcome{Committed / NoOp}, CommitError>`、Approve 段だけ定義を読む、競合は再構成から 1 回だけ再試行）/ CreateIntent / DefineWorkflow / Park / PromotePractices / RecordReview / RecordSingleStageRun / RecordSkeletonStance / SwitchAutonomy。`execute` の引数は ID と値オブジェクトのみ。`Next` / `Continue` はコマンド側に存在しない | shift #58-59 #64、late #38 #55 | ✓ |
| P6 | クエリ側: ユースケース 13（`Find*`: NextAnswer / Continuation / RunStage / Steering / Definition / DefinitionStage / Scope / ScopeKeyword / PhaseEntry / Jump / Execution / ScopeChange / StateFile）は `dao.find(key) → View` のみで判断・導出・選択・文言を持たない。DAO ポート 14（動詞 `find`）、SQL はリテラルで 1 表 1 引当（`cargo lint dao-single-table`）、実装 = SQLite DAO 14 + `InMemoryXxxDao` 13、1 要求 1 読取専用接続を `Rc` 共有。クエリ側クレートはドメインに依存しない | late #17-#21 #25、shift #38 | ✓ |
| P7 | 文言（利用者向け逐語）は出す側の `wording` モジュール（app / RMU）。要求の形で決まる分岐はコントローラ、状態の値で決まる分岐は行の kind をプレゼンタが描く。app は状態の値で決まる分岐を持たない | late #21 #35、shift #24 | ✓ `app/src/wording.rs`、`rmu/workspace/wording.rs` |
| P8 | 合成ルート（`aidlc`）は読取前と書込後に RMU `catch_up` を**同期**呼出する（駆動ループ・spawn は持たない）。実行カーソル `<record>/.aidlc-execution` を intent-create が書き、`definition_id` は `"claude"` 固定 | late #22、other #27 | ✓ `runtime.rs:194,200,238`、`execution_cursor.rs:32` |
| P9 | CLI 面: `aidlc` マルチコール。動詞 next / continue / report / park / compose / init / intent-create / link / decision / answer / review / set-autonomy / practices-promote、面 `aidlc-bolt`（set-autonomy のみ）/ `aidlc-log`（review のみ）/ `aidlc-state`（practices-promote のみ）/ `aidlc-utility`。未配線: unpark / jump / recompose / フック 4 本 / doctor / 他動詞 | late #56 #63 #71 | ✓ 動詞表実測 |

### 4.3 RMU とリードモデル（11 号 §2.3 / §4.1、C5 / C6）

| # | 裁定（現行） | 出典 | 検証 |
|---|---|---|---|
| R1 | RMU は中間クレート `core-read-model-updater`（コマンド側でもクエリ側でもない）。`JournalReader`（events_after / events_through / checkpoint / advance_checkpoint / publish / pending_publication / steering_source_digest / replace_steering / prepare_read_model）と `JournalReaderImpl`（別接続、rowid カーソル、`BEGIN IMMEDIATE`）を所有 | shift #20 #22 #27、other #4 #7 | ✓ |
| R2 | 二層: 取得ループ（`catch_up` / `catch_up_structured`）と純粋投影核（`project`）。集約を `IntentExecution::replay` / `WorkflowDefinition::replay` で起こしクエリメソッドの答えを写す。順序 = 投影 → 書込 → `advance_checkpoint`（at-least-once） | late #2、other #12 #15 | ✓ `read_model_updater.rs:5` |
| R3 | `read_*` 17 表（definition / definition_stage / definition_scope / definition_scope_keyword / definition_scope_stage / definition_scope_phase_entry / intent / intent_stage / execution / execution_stage / next_answer / next_jump / next_jump_phase / run_stage / scope_change / steering_plan / steering_part）。主キーは 1 列 `id`、自然キーは UNIQUE、関連は FK 列。全差し替えとチェックポイント前進は同一 Tx。`read_next_answer.gate TEXT`（3 綴り）、`read_run_stage`（定義 × scope × stage、`in_scope`、相対パス）、`read_config_current` は作らない。読み面スキーマ版は `PRAGMA user_version` | late #3 #5 #12 #14 #23 #45 #46、shift #37 #41-43 | ✓ `sql.rs` |
| R4 | 我々の表は `amadeus_projection_checkpoint`（`(aid, seq_nr)` アンカー併記、不一致は `Corrupt(CheckpointAnchorMismatch)`）/ `amadeus_read_model_head` / `amadeus_publication` 系 4 表。`journal` / `snapshot` は本家所有（ピン `=3.0.0`、manifest 列 = `intent-execution-event/1` / `intent-event/1` / `workflow-definition-event/1`） | shift #28-29 #35、other #26-#31 | ✓ |
| R5 | steering は別投影単位・別 Tx。束は phase の関数（org → team → project → phases/<phase>）、`source_digest` が変わったときだけ再パック | late #11 #15、shift #43-44 | ✓ `steering_source.rs` |
| R6 | 監査行はイベントの `occurred_at` を刻む。状態ファイルの骨格は合成ルートの成果物で RMU は差分適用のみ（欠落は `ProjectionError::ScaffoldMissing`）。`PhaseBoundary` は RMU が導出。メモリ層 2 本（team.md / project.md）も RMU の投影面（`MemoryFaces`） | other #14 #21、shift #25、late #57 | ✓ |
| R7 | 公開は `prepare` → `publish` の 2 Tx（`publication` 表）、1 回の `catch_up` は最大 2 計画 | other #26-#27 | ✓ `publication_store.rs` |

### 4.4 workflow-definition（12 号、01 号 §3.1、C4）

| # | 裁定（現行） | 出典 | 検証 |
|---|---|---|---|
| W1 | `WorkflowDefinition` は ES 集約（8 属性: id / revision / graph / grid / scopes / seq_nr / version / last_updated_at）。`define(id, &CompiledDefinition, at) -> Result<(Self, Defined), LineageMismatch>`、`redefine(&CompiledDefinition, at) -> Result<Redefined, RedefineError>`、`replay`。「読取専用集約」の呼称は廃止 | shift #13 #16 #47 #55、u2u3 #2 | ✓ |
| W2 | `CompiledDefinition`（配布束の集約、5 属性: id: CompiledDefinitionId / revision / graph / grid / scopes）。遷移 compile / recompile / register_scope / apply_plugin_selection、イベント 4。`DefinitionRevision::of_content` はドメインが導出 | shift #15-16、u2u3 #5 | ✓ |
| W3 | 述語面 6（is_valid_scope / valid_scopes / scope_metadata / subgraph_for_scope / stages_in_scope / first_in_scope_stage_of_phase）+ `grid().action()` 3 値 + クエリ `scope_cost` / `review_policy(slug, scope, override)` / `stage_route`。`effective_plan_action` / `next_in_scope_stage` は無い | u2u3 #46、late #49、shift #40 | ✓ |
| W4 | `ReviewPolicy`: cap と override は下げるだけ、reviewer 宣言ありでクラス無しは adversarial、budget = advisory 1 / adversarial max_iterations / none 0。`PRACTICES_DISCOVERY_SLUG` はリテラル定数 | late #49 #58 | ✓ `review_policy.rs` |
| W5 | `StageGraph` / `ScopeGrid` は FCC（`Filtered = Self`）。`StageEntry` 5 欄（slug / phase / plan_action / conditional / display: StageDisplay） | u2u3 #10 | ✓ |

### 4.5 workspace（11 号 §2、01 号 §3.3）

| # | 裁定（現行） | 出典 | 検証 |
|---|---|---|---|
| S1 | 値オブジェクト: `SpaceName` / `CloneId` / `ShardName` / `IntentDirName`（`^[0-9]{6}-slug`、64 字以下）/ `StateFieldValue` / `CheckboxState` / `CheckboxEntry` / `StateVersionClassification`（`CURRENT_STATE_VERSION = "8"`）/ `AuditFieldKey` / `AuditFieldValue` / `AuditEventRecord` / `EventType`（86）/ `EventCategory` / `StorePath::for_space` / `HumanTurns::find_in`。`IntentId` は UUIDv7（`orchestration`） | u2u3 #3 #32-33 #51、late #36 #69、other #11 #16 | ✓ `workspace/mod.rs` |
| S2 | FCC 6: `Checkboxes` / `AuditFields`（挿入順、combine / divide）/ `BoltRefs`（combine / divide）/ `OrderedAuditEvents`（順序付け純関数の出力）/ `PromotedSections` / `RuleLines`。`PracticesPromotion::plan` と `markdown_sections` 3 関数はドメイン workspace | other #16-17、late #59-60 | ✓ |
| S3 | 描画（`render_audit_block` / `state_writers`）とシャード列挙・読取 I/O は RMU。ロック機構（`WorkspaceLock` / `FsWorkspaceLock` / `LockProtocol` / `reap_eligible` / `OwnerStamp` / `ProcessProbe`）は退役 | shift #12、other #18 | ✓ 退役語 0 件 |
| S4 | workspace の集約 `Intent`（登録簿）/ `Space` / `Worktree`、供給面 `WorktreeService` / `OpaqueFlagStore` / `ScopedStorage` / `SessionStampStore`、`intents.json` の直列化機構は**未実装**（コードに登録簿の書込は無い）。orchestration の `Intent` は同名の別物 | other #48、shift #30 | 予定 |

### 4.6 クレート構成・依存・基盤（01 号 §7、components.md）

| # | 裁定（現行） | 出典 | 検証 |
|---|---|---|---|
| K1 | クレート 10: `core-command-domain` / `core-command-use-case` / `core-command-interface-adapter` / `core-query-use-case` / `core-query-interface-adapter` / `core-read-model-updater` / `core-infrastructure` / `aidlc` / `harness-claude`（3 行のスタブ）/ `harness-infrastructure`。`modules/shared` は解体。後方互換の再輸出・shim・`#[deprecated]` はゼロ | other #1-#10、shift #22-24 | ✓ |
| K2 | CQRS 境界はクレート分離で物理強制: コマンド側とクエリ側は互いの `Cargo.toml` に現れず、RMU だけが両側に依存できる。ドメインはコマンド側の持ち物 | shift #19 #23、late #72 | ✓ |
| K3 | ドメインは永続化知識から中立: `core-command-domain` の依存は chrono / uuid（v7）/ core-infrastructure のみ。serde・event-store-adapter-rs・manifest はアダプタと RMU が持つ | shift #56-57、u2u3 #23 #50 | ✓ `Cargo.toml` |
| K4 | infrastructure 層は言語拡張のみ（`core-infrastructure`: canon_json / collections（`FirstClassCollection`）/ atomic / append_only / fs_meta / codec / secret_file）。RPC / DB を置かない | other #8 #32 | ✓ |
| K5 | 契約 JSON は canon_json 経由（`serde_json::to_*` 直接呼出禁止、3 プロファイル、ダイジェスト 2 族）。ゴールデンは `tests/golden/upstream-3c3146cf/`（cli / hooks / hash-canonical / normalization.json / 骨格 `state-full.md`） | other #22 #33-#37 | ✓ |
| K6 | Quint モデル 3（`engine_loop.qnt` v2.7 / `journal_protocol.qnt`（保証は every(1) 限定）/ `stop_hook.qnt`）。`journal_protocol.qnt` の対応表コメント 5 行に旧名 `WorkflowExecution` が残る | u2u3 #35-36、shift 未了 | ✓ 残存確認 |

### 4.7 未実装（仕様には「予定」と明記するもの）

unpark / jump / recompose のユースケースと CLI 配線、フック 4 本、doctor、`aidlc-state` 他 24 動詞・`aidlc-bolt` 他 7 動詞、workspace 集約 3 つと供給面 4 つ、`intents.json` 直列化、Bolt / SwarmBatch（slice 2）、`PRACTICES_OVERRIDE` / `practices-event`、deviations #5 / #6 / #7 の繰延（fingerprint 2 欄・stale-receipt recovery・`--unit` / `--single`・revision backstop・`QUESTION_ANSWERED` 等）。

### 4.8 記録間の矛盾と、コードで裁いた結果

| 論点 | 記録 A | 記録 B | コードの現状（採用） |
|---|---|---|---|
| イベント変種数 | decisions.md:55「12 → 11」（b42） | nfr-requirements/memory.md:5「16」 | **16**（b42 で 11 → b47 +2 → b48 +2 → b49 +1） |
| 楽観 version の置き場 | ADR-010 2026-08-29「集約の外、Rehydrated が持ち回る」 | U3 rules 2026-08-30「版は返す集約が運ぶ」 | **集約の内側**（`version` / `with_version`） |
| 再構成方式 | 2026-08-30「ジャーナル全再生」 | 2026-09-05「最新スナップショット + 差分」 | **差分再生**（実測 43 テスト） |
| `DefinitionRevision` の計算主体 | command-domain-audit 2026-08-29「Repository / 投影側」 | b36 2026-09-02「ドメインが導出」 | **ドメイン**（`of_content`） |
| `StageSlugSet` の順序 | U2 entities「文書順」 | b51 計画「辞書順」 | **辞書順**、RMU で並べ直し |
| skeleton 対象 | U2 rules §3 射影表「最初の非 init EXECUTE」 | rules BR「`first_of(Construction, EXECUTE)`」 | **Construction の最初の EXECUTE** |
| RMU の駆動 | u5 decisions-1「非同期タスク、join 2 箇所」 | b39 以降「合成ルートが `catch_up` を呼ぶだけ」 | **同期呼出**（spawn なし） |
| Directive の kind 数 | 10 号 §2.2 / C1「10 種」 | — | `DirectiveKind` 10、`Directive` 構築可能 7 |
| RMU とジャーナル | ADR-009 初稿「RMU はジャーナルを読まない」 | 2026-08-28 改訂「RMU が取得ループ」 | **RMU が読む**（`JournalReader` 所有） |
| `CommitVerdictUseCase` の戻り値 | u5 decisions-1「`Result<(), CommitError>`」 | b46「`CommitOutcome` 3 形」 | `Result<CommitOutcome{2 形}, CommitError>` |
| レビュー試行の置き場 | b48「`review_attempts: Vec<ReviewAttempt>`」 | b51「`StageSlot.review_attempt`」 | **`StageSlot` 内** |
| `next_decision` の署名 | b38「`(&NextRequest) -> NextDecision`、失敗しない」 | b47「`&Intent` 追加」/ b51「`Result`」 | `(&Intent, &NextRequest) -> Result<_, CommandError>` |
| `Started` の内容 | b8「scan と display を焼き込む」 | b39「`Created` が運び、`Started` は intent_id と stages」 | **b39 の形** |

## 5. 記録に「未了」と明記された箇所の扱い

**U9 再走で扱う**（P1〜P4 に含める）: 仕様 4 号の全文追従（§2.1〜2.4 + §4 の O/P/R/W/S/K を本文へ）と冒頭注記 2 種の畳み込み、10 号 :51 のイベント数 11 → 16、coding-rules 12 行・6 ファイル（§2.5 の 10 行 + `README.md:115` の `WorkflowExecutionState` 是正対象表記の履歴化。2026-09-07 追加実測）、`components.md` 全面、`contract-summary.md` C1 / C3 / C4 / C5 / C6 / §4（「同一シャード内の直接行と投影行の順序」は未決のまま §4 に残す）、`unit-of-work.md` U3 の独自スキーマ・`InMemoryWorkflowExecutionRepository` 記述への失効注記（:34 / :83 / :91 / :144、本文は書き換えない）、U9 設計記録の同期（pending-revision 5 項目、R-01〜R-03、BR5.3）。

**訂正（2026-09-07 追加実測）**: 旧 manifest 綴り `workflow-execution-event/1` の残存 4 件（10 号 :51、`decisions.md:473`、`contract-summary.md` :285 / :305 / :388）は**すべて打消し線つきの履歴**（「B12 改名追従 2026-08-30」注記あり）であり、改訂対象ではない。要約確認で「直す」と述べた点を本行で訂正する。また coding-rules で `WorkflowExecution` を含む行のうち aggregate-references:62 / cqrs-boundaries:30 / field-visibility:46 / good-examples:84 / ubiquitous-language:21 は履歴（「旧」「~~」「実測」）であり触らない。

**U9 では扱わない**（別 Unit / 別 Bolt の記録として残す）: U2 / U3 / U1 / U10 の設計本文の折り戻し（各 Unit のゲート）、`query-side-audit/read-model-spec.md` と `inventory.md`（Bolt 記録）、`formal/orchestration/journal_protocol.qnt` のコメント 5 行（コード扱い — 別途 1 行 PR か U6 / U7 の Bolt で）、codekb 2 文書、`docs/CLAUDE.md`、CI 設定の裁定（cargo doc 等）。
