AIDLC-UNIT: u9-canon-docs
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

# developer-report-4 — 派遣 B: 仕様 11 号 + 12 号 + 共有契約 3 本 + ADR-010 注記（U9 再走、2026-09-07）

> `developer-brief-4.md` に対する作成報告。所有ファイル 6 本（`docs/specs/11-workspace.md` /
> `docs/specs/12-workflow-definition.md` / `inception/domain-design/components.md` /
> `inception/contract-design/contract-summary.md` / `inception/units-generation/unit-of-work.md` /
> `inception/domain-design/decisions.md`）を改訂した。コード・`formal/`・`docs/specs/research/**`・
> `docs/specs/deviations.md`・派遣 A の所有ファイル（`docs/specs/10-*.md` / `01-*.md` / `coding-rules/`）には触れていない。
> `git add` / `git commit` / push / PR / GitHub 書込・`.claude/` 配下ツールの実行・`AIDLC_*` の設定はいずれも行っていない。

## 1. Red 基線（Step 4、改訂前の実測）

### (a) sentinel 10 語 grep（履歴マーカー除外）— 所有仕様 2 本

```bash
grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start|WorkflowExecution|RehydratedWorkflowExecution|message-catalog' docs/specs/11-workspace.md docs/specs/12-workflow-definition.md | grep -vE '~~|旧|失効|是正済み|改名|履歴'
```

**13 件**（ブリーフの見込み「11 号 5 + 12 号 8 = 13」と一致）:

| ファイル | 行 | 語 |
|---|---|---|
| `11-workspace.md` | 6 | 冒頭 B12 注記の `WorkflowExecution` |
| `11-workspace.md` | 47 | 「監査台帳は集約 `WorkflowExecution` のイベントログ」 |
| `11-workspace.md` | 95 | §3 の 01 §3.2 引用「遷移動詞 11 個の唯一の所有者は `WorkflowExecution`」 |
| `11-workspace.md` | 126 | §4 Gateways「並行制御は `WorkflowExecutionRepositoryImpl` の SQLite Tx」 |
| `11-workspace.md` | 165 | §5「チョークポイントは `WorkflowExecutionRepositoryImpl` の `store`」 |
| `12-workflow-definition.md` | 6 | 冒頭 B12 注記の `WorkflowExecution` |
| `12-workflow-definition.md` | 34 | §1 B1「集約 `WorkflowExecution` の `effective_plan`」 |
| `12-workflow-definition.md` | 89 | §2.3 `grid().action()` 行「畳み込みの責務は呼び出し側 = 集約 `WorkflowExecution`」 |
| `12-workflow-definition.md` | 91 | §2.3「集約 `WorkflowExecution` が `Started` で確定させた `stages`」 |
| `12-workflow-definition.md` | 164 | §4 #7「畳み込みは集約 `WorkflowExecution` の `effective_plan`」 |
| `12-workflow-definition.md` | 165 | §4 #8「`WorkflowExecution::start` は initialization が EXECUTE でなければ拒否」 |
| `12-workflow-definition.md` | 215 | §8 F2「文書順の前進走査は集約 `WorkflowExecution` の `stages` 上」 |
| `12-workflow-definition.md` | 221 | §8 F8「畳み込みは集約 `WorkflowExecution` の `effective_plan`」 |

### (b) `予定（未実装` の件数

```
docs/specs/11-workspace.md:0
docs/specs/12-workflow-definition.md:0
```

### (c) `## Review` の行番号と sha256（共有契約 3 本）

```
components.md:430   b954159a08ef6a312306efa0ec9ddaeecd0403e7b103b12a7eca13614bdc1dbb
contract-summary.md:486   7374b976335767b3356452648e3ad92fddf1f8730e9f5f070d8aabc75f8f3662
unit-of-work.md:207   b8d0e4a122bab43d0123c1a0515536e2866d05eb7609f6abcbb87c91dd503880
```

いずれもブリーフに書かれた基線 3 値と一致した。

### (d) 共有契約の sentinel 出現（参考。受入 (2) の grep 範囲外）

| ファイル | 総数 | 履歴マーカー無し |
|---|---|---|
| `components.md` | 13 | 10 |
| `contract-summary.md` | 36 | 28 |
| `unit-of-work.md` | 14 | 12 |
| `decisions.md` | 18 | 16 |

### (e) 用語「同期」（RMU 文脈）

改訂前から **0 件**（`grep -nE '同期' <所有 4 ファイル> | grep -iE 'rmu|catch_up|投影'` が空）。

### (f) `decisions.md:476-478` の現文

```text
  楽観 `version` は集約と memento（`WorkflowExecutionState`）から削除し、**集約の外**を持ち回る
  形にした — `find_by_id` は再水和レコード `RehydratedWorkflowExecution`（集約 + ストア採番
  version）を返し、`store` は `expected_version: usize` を引数に取る。
```

ブリーフの引用と 1 バイト違わないことを確認した。

## 2. 改訂一覧（1 件 1 行。根拠列 = コードの所在 / テストの有無 / 仕様の該当節）

> **番号の統一（メイン指示 2026-09-07、Step 6 再検査の前に適用）**: workspace の集約 3（`Intent` / `Space` / `Worktree`）・供給面 4（`WorktreeService` / `OpaqueFlagStore` / `ScopedStorage` / `SessionStampStore`）・`intents.json` の直列化機構の予定表記を、当初書いた `予定（未実装、クリティカルパス 2 = workspace 実装スライス）` から **`予定（未実装、クリティカルパス 4 = マルチコール CLI + 文言カタログ配線）`** へ改めた（対象 9 箇所 = `11-workspace.md` 8 + `components.md` 1）。いずれも CLI 動詞の配線で着地する未実装であり、派遣 A が 01 号 §3.3 / 10 号 §3・§7.1 で採った形にそろえた。他の番号は指示どおりで変更していない — unpark / jump / recompose のユースケースは 4、フック 4 本は 5、doctor は 6、Bolt / SwarmBatch は本派遣の所有ファイルに言及が無いため対象外。下表の該当行（B-1 #2 / #3 / #8、B-3 #3）はこの統一後の値で読むこと。

### B-1. `docs/specs/11-workspace.md`（改訂 11 件、+57 行）

| # | 節 | BR | 改訂内容（何を何に） | 出典注記 | 根拠: コードの所在 | テスト | 仕様の該当節 |
|---|---|---|---|---|---|---|---|
| 1 | 冒頭 :3-15 | BR3.3 (i) | B12 読み替え注記・B13 優先順位注記の 2 本を「**追従済み（2026-09-07 / U9 再走）**」の 1 ブロックへ畳み込み、旧注記 2 本は `~~…~~` の 1 行ずつに圧縮して残置 | B12 2026-08-30 / B13 2026-08-30 / 実測 | `command/domain/src/orchestration/{intent.rs,intent_execution.rs}`、`intent_execution_repository_impl.rs:340-438` | 既存 43 テスト（再構成、2026-09-05 実測） | 11 §2.1 / §3 |
| 2 | §1 B5 :35 | BR3.2 | 「`WorkflowExecution` 集約の書込は SQLite Tx」→ `IntentExecution`。加えて登録簿の直列化を `予定（未実装、クリティカルパス 4）` と明記 | B12 2026-08-30 改名 / gap-measurement §4.5 S4 | `orchestration/intent_execution.rs`、登録簿の書込はコードに無し | — | 11 §2.1 `LockIdentity` 行 / §10 |
| 3 | §2.1 冒頭（新規 2 段落） | BR3.2 / BR3.3 (g) | workspace ドメインに**集約は無い**（値オブジェクト + FCC 6 のみ）ことを実装状態として明記し、表の 3 集約を `予定（未実装、クリティカルパス 4）` に。加えて「コードの `Intent` は orchestration の静的 intent 集約であり登録簿の `Intent` とは同名の別物」を注記 | gap-measurement §4.5 S1 / S2 / S4、§4.1 O2 | `command/domain/src/workspace/mod.rs` の `pub use` 一覧（集約ゼロ）、`orchestration/intent.rs`（7 属性） | 既存の値オブジェクト単体テスト（インライン `#[cfg(test)]`） | 11 §2.1 / 01 §3.3 |
| 4 | §2.1 リードモデル段落 :47 | BR3.3 | 「監査台帳は集約 `WorkflowExecution` のイベントログ」→ `IntentExecution` | ADR-001 / ADR-003 / B12 2026-08-30 | `orchestration/intent_execution.rs` | — | 11 §2.1 |
| 5 | §2.1 退役段落 :49 | BR3.3 (g) | `WorkflowExecution` 2 箇所 → `IntentExecution`。**楽観 version は集約の内側**（`version: usize` / `version()` / `with_version()` / `UNPERSISTED_VERSION = 0`、`store` に `expected_version` 引数なし）を追記 | ADR-007 / B12 / B13 2026-08-30 | `intent_execution_repository_impl.rs:455`（`let expected_version = aggregate.version();`）、`port/intent_execution_repository.rs:93-98` | ポート契約テスト 7 本（`a_write_that_presents_a_stale_version_conflicts` 等、同ファイル `#[cfg(test)]`） | 11 §2.1 / §3 |
| 6 | §3 :95 | BR3.3 | 01 §3.2 引用の「遷移動詞 11 個の唯一の所有者は `WorkflowExecution`」→「遷移動詞の唯一の所有者は `IntentExecution`」。現行のコマンド数 15 + genesis `start` と、`unpark` の CLI 配線が `予定（未実装、クリティカルパス 4）` であることを併記 | B12 2026-08-30 / gap-measurement §4.1 O4・§4.2 P9 | `orchestration/intent_execution.rs`（コマンド 15 + `start`）、`app/src/cli/request.rs:112-128`（`unpark` は未配線動詞リスト） | 集約コマンドの単体テスト（インライン） | 11 §3 / 10 §9 S1 |
| 7 | §3 ポート表 :101 | BR3.3 (e) | 行全体を書き直し。ポート名 `WorkflowExecutionRepository` → `IntentExecutionRepository`、署名を `find_by_id(&IntentExecutionId) -> Result<IntentExecution, RepositoryError<IntentExecutionId>>` / `store(&mut self, &IntentExecutionEvent, &IntentExecution)` に。**`expected_version` 引数なし**、`RepositoryError<Id>` 4 変種、`Rehydrated*` / `StatePosition` / `StoreVersion` の撤去を履歴化。実装列は `IntentExecutionRepositoryImpl<S>` にし、**テストダブル 3 層**（公開インメモリ = `in_memory()` / 自作ダブル禁止 / use-case 層の `#[cfg(test)]` フェイクは DIP 制約下の例外）を明記 | C3 / ADR-010 / B12・B13 2026-08-30 / オーナー裁定 2026-08-31 | `port/intent_execution_repository.rs:59-98`、`port/mod.rs:15-18`、`port/repository_error.rs:29-48`、`intent_execution_repository_impl.rs:182`、`use-case/src/orchestration/test_support.rs:1-17` / `orchestration/mod.rs:38-39` | ポート契約テスト 7 本 + アダプタ契約テスト（SQLite / memory 両バックエンド） | 11 §3 / 10 §3 / C3 |
| 8 | §3 供給面表 :111-116 | BR3.2 / BR3.3 (g) | `WorktreeService` / `OpaqueFlagStore` / `ScopedStorage` / `SessionStampStore` の 4 行すべてに `予定（未実装、クリティカルパス 4）` を付与し、表の前に「4 サービスはコードに該当型が無い」と実装状態を明記 | gap-measurement §4.5 S4 | 該当型はコードに無し（`grep` 実測 0 件） | — | 11 §3 供給面 |
| 9 | §4 Gateways :126 | BR3.3 | `WorkflowExecutionRepositoryImpl` → `IntentExecutionRepositoryImpl` | ADR-007 / B12 2026-08-30 | `command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs` | — | 11 §4 |
| 10 | §5 :165 | BR3.3 | `WorkflowExecutionRepositoryImpl` の `store` → `IntentExecutionRepositoryImpl` | ADR-003 / B12 2026-08-30 | 同上 | — | 11 §5 |
| 11 | §7 実装順序 :195 / :196 | BR3.3 | 3 の `WorkflowExecutionRepositoryImpl::in_memory()` → `IntentExecutionRepositoryImpl::in_memory()`、4 の ITF 再生先 `WorkflowExecutionRepositoryImpl ＋ JournalReaderImpl` → `IntentExecutionRepositoryImpl ＋ JournalReaderImpl` | ADR-010 / B12 2026-08-30 | `intent_execution_repository_impl.rs:182` | ITF 準拠テスト（`command/domain/tests/`） | 11 §7 |

### B-2. `docs/specs/12-workflow-definition.md`（改訂 10 件、+58 行）

| # | 節 | BR | 改訂内容（何を何に） | 出典注記 | 根拠: コードの所在 | テスト | 仕様の該当節 |
|---|---|---|---|---|---|---|---|
| 1 | 冒頭 :3-15 | BR3.3 (i) | 11 号と同形の畳み込み。本コンテキストの集約 `WorkflowDefinition` / `CompiledDefinition` は現行のまま有効である旨を明記 | B12 / B13 2026-08-30 / 実測 | `workflow_definition/workflow_definition.rs`、`compiled_definition.rs` | 既存の集約単体テスト | 12 §2.1 |
| 2 | §1 B1 :34 | BR3.3 | 「集約 `WorkflowExecution` の `effective_plan`」→「集約 `IntentExecution` の `effective_plan(StageIndex) -> Option<PlanAction>`」 | B12 2026-08-30 / 設計監査 R2 | `orchestration/intent_execution.rs` の `effective_plan` | 集約クエリの単体テスト | 12 §1 / §2.3 |
| 3 | §2.3 表の後（新規 表 + 段落） | BR3.3 (f) | 集約のクエリ **3 件を追記** — `scope_cost(&str) -> Option<ScopeCost>` / `review_policy(&StageSlug, &str, Option<ReviewCapValue>) -> Result<Option<ReviewPolicy>, UnknownStage>` / `stage_route(&str, &StageNode) -> StageRoute`。あわせて旧述語 `effective_plan_action` / `next_in_scope_stage` が**存在しない**ことを履歴として明記 | gap-measurement §4.4 W3 / W4、b41、設計監査 R2 | `workflow_definition.rs:433`（scope_cost）/ `:483`（review_policy）/ `:522`（stage_route） | 各クエリのインライン単体テスト（同ファイル `#[cfg(test)]`） | 12 §2.3 |
| 4 | §2.3 2 経路段落 :91 | BR3.3 | 「集約 `WorkflowExecution` が `Started` で確定させた `stages`」→「集約 `Intent` が保持する `stages: StageEntries`（`StageEntry` 5 欄）。`Started` はその写しを運び `IntentExecution` が自ストリームだけで再生する」 | B12 2026-08-30 分割 / b39 2026-09-02 / 設計監査 R2 | `orchestration/intent.rs` の `stages`、`orchestration/intent_execution_event/started.rs` | 投影ゴールデンテスト（`tests/projection_golden_test.rs`） | 12 §2.3 / 11 §4.1 |
| 5 | §2.3 `grid().action()` 行 :89 | BR3.3 | 「畳み込みの責務は呼び出し側 = 集約 `WorkflowExecution`」→ `IntentExecution` | B12 2026-08-30 / 設計監査 R2 | `orchestration/intent_execution.rs` | — | 12 §2.3 |
| 6 | §4 #7 :164 | BR3.3 | 同上の名称改訂 | B12 2026-08-30 / 設計監査 R2 | 同上 | — | 12 §4 |
| 7 | §4 #8 :165 | BR3.3 | 「`WorkflowExecution::start` は initialization が EXECUTE でなければ `InitializationMustExecute` で拒否」→「`Intent::create` が拒否する（計画 `StageEntries` を確定させるのは `Intent` 側で、`IntentExecution::start` はその写しを受け取る）」 | B12 2026-08-30 分割 | `orchestration/intent.rs` の `create`（`StageEntries` 確定）、`intent_execution.rs` の `start` | 集約の拒否ガード単体テスト | 12 §4 / 10 §2.1 |
| 8 | §5 ユースケース段落 :178 | BR3.3 (f) | 「`LoadStageGraph` / `LoadScopeCatalog` / `ResolveScopePlan`（読み取り専用）」を打消し線で失効させ、**3 層の現行面**へ置換 — コマンド側 1 本（`DefineWorkflowUseCase`）/ クエリ側 5 本（`FindDefinitionUseCase` / `FindDefinitionStageUseCase` / `FindScopeUseCase` / `FindScopeKeywordUseCase` / `FindPhaseEntryUseCase` と対応 DAO 5）/ RMU の `read_definition*` 6 表。`recompose` の CLI 配線は `予定（未実装、クリティカルパス 4）` | オーナー裁定 2026-08-30 / 2026-08-31 / 2026-09-02、gap-measurement §4.2 P5・P6、§4.3 R3 | `query/use-case/src/`（`Find*UseCase` 実測 13 本のうち定義系 5）、`query/use-case/src/`（`*Dao` 14）、`read-model-updater/src/read_tables/`（`read_definition` 系 6 表を `CREATE TABLE` 実測） | クエリ側ユースケースの単体テスト（`InMemory*Dao` 13 を使用） | 12 §5 / 11 §4.1 |
| 9 | §5 ポート箇条 :183 | BR3.3 (f) | `WorkflowDefinitionRepository` の動詞 2 → **3 本**（`find_by_id` / `find_for_intent(&Intent)` / `store`）。`find_for_intent` の趣旨（ユースケースでドメインの getter を呼ばないための関連取得、アダプタが参照 ID を読み `find_by_id` へ委譲）を追記 | オーナー裁定 2026-09-05、gateway-taxonomy §2 | `port/workflow_definition_repository.rs:63`（trait）/ `:77`（find_by_id）/ `:91-94`（find_for_intent）/ `:112`（store） | ポート契約テスト（同ファイル `#[cfg(test)]`） | 12 §5 / C4 |
| 10 | §8 F2 :215 / F8 :221 | BR3.3 | F2「文書順の前進走査は集約 `WorkflowExecution` の `stages` 上」→ 集約 `Intent` の `stages: StageEntries` 上。F8「畳み込みは集約 `WorkflowExecution`」→ `IntentExecution` | B12 2026-08-30 / 設計監査 R2 | `orchestration/intent.rs` / `intent_execution.rs` | — | 12 §8 |

### B-3. `inception/domain-design/components.md`（全面改訂。コンポーネント 12 = 1 行 1 件、+739 行相当の書き直し）

冒頭注記の訂正 1 件（「ジャーナル全再生」→ **最新スナップショット + 差分再生**。打消し線 + 失効注記で履歴保持）に加え、YAML のコンポーネントを現行 10 クレートへ対応させた。

| # | コンポーネント | 対応クレート / モジュール | 改訂内容（何を何に） | 根拠: コードの所在 | テスト | 仕様の該当節 |
|---|---|---|---|---|---|---|
| 1 | `OrchestrationEngine` | `core-command-domain::orchestration` | 集約 `WorkflowExecution` 1 つ → **`Intent`（7 属性 / `IntentEvent::Created` 1 変種）+ `IntentExecution`（12 属性 / コマンド 15 + genesis `start`）**。イベント 11 → **16 変種**（列挙を全数差し替え）。`version` は集約の内側、`Rehydrated*` は撤去を履歴化。`next_decision` の署名を `(&self, &Intent, &NextRequest) -> Result<NextDecision, CommandError>` に。FCC 8 型を responsibilities へ追加 | `orchestration/{intent.rs, intent_execution.rs, intent_execution_event/*（16 ファイル）}`、`intent_event.rs` | ITF 準拠テスト 2 本 + 集約インライン単体テスト | 10 §2.1 / 01 §3.2 / C5 |
| 2 | `WorkflowDefinitionModel` | `core-command-domain::workflow_definition` | 「ES 対象外・find のみ・`next_in_scope_stage`」を失効させ、**集約 2 つ**（`WorkflowDefinition` 8 属性 / イベント 2、`CompiledDefinition` 5 属性 / イベント 4）に。述語 6 + `grid().action()` + クエリ 3（`scope_cost` / `review_policy` / `stage_route`）を明記し、`effective_plan_action` / `next_in_scope_stage` の不在を履歴化 | `workflow_definition/workflow_definition.rs:349-529`、`compiled_definition.rs`、`workflow_definition_event.rs`、`compiled_definition_event.rs` | 集約インライン単体テスト + FCC 契約テスト | 12 §2.1 / §2.3 |
| 3 | `WorkspaceModel` | `core-command-domain::workspace` | 値オブジェクト列を現行の `pub use` へ更新し、**FCC 6 型**を明記。「集約は無い」ことと、11 号 §2.1 の集約 3 / 供給面 4 が `予定（未実装、クリティカルパス 4）` であることを追記 | `workspace/mod.rs:50-91`（`pub use` 一覧） | 値オブジェクトのインライン単体テスト + `collection_contract_test.rs` | 11 §2.2 / §2.3 |
| 4 | `CommandUseCases`（~~`EngineUseCases`~~ — 改名） | `core-command-use-case` | `NextUseCase` / `ReportUseCase` / `ContinueUseCase` / `DoctorUseCase` / フック 4 → **実測 9 本**（`CommitVerdict` / `CreateIntent` / `DefineWorkflow` / `Park` / `PromotePractices` / `RecordReview` / `RecordSingleStageRun` / `RecordSkeletonStance` / `SwitchAutonomy`）。`Next` / `Continue` はクエリ側移設を履歴化、`Doctor` / フック 4 / unpark / jump / recompose は `予定（未実装、クリティカルパス 4〜6）`。ポート 4 とその署名、`RepositoryError<Id>` 4 変種、**テストダブル 3 層**を追記 | `use-case/src/orchestration/{*_use_case.rs（9）, port/*.rs（4）, test_support.rs:1-17, mod.rs:38-39}` | ユースケース単体テスト（`#[cfg(test)]` フェイク 4 を使用） | 10 §3 / 11 §3 / C3 |
| 5 | `CommandGateways`（~~`PersistenceGateways`~~ — 改名） | `core-command-interface-adapter` | ローカル `EventStoreImpl` 同形 trait を失効させ、**本家 event-store-adapter-rs `=3.0.0` を内包**へ。`find_by_id` = 最新スナップショット + 差分再生、`store` の `aggregate_id` 照合と `Corrupt(WriteContract)`、版は `aggregate.version()`、`SnapshotStrategy::every(n)` 既定 10、`in_memory()` を追記。`*Dto` の所有と `wire` 全廃も反映 | `interface-adapter/src/orchestration/{intent_execution_repository_impl.rs:182,336-438,455, dto/}` | アダプタ契約テスト（SQLite / memory 両バックエンド）、43 テスト（再構成） | 11 §3 / §4 / C3 / C6 |
| 6 | `QueryUseCases`（**新設**） | `core-query-use-case` | 旧版に存在しなかったクエリ側を新設。ユースケース 13 本の実名、DAO ポート 14 本の実名、`find` のみの型保証、`*View` の `port/` 同居、**ドメイン非依存**、`Directive` / `DirectiveKind` / `ContinueToken` / `StateBinding` の所在を明記（kind の列挙と JSON 形は逐語契約として不変） | `query/use-case/src/`（`Find*UseCase` 13 / `*Dao` 14 / `directive.rs` / `directive_schema.rs`） | クエリ側ユースケース単体テスト | 10 §2.2 / 12 §5 / C5 |
| 7 | `QueryGateways`（**新設**） | `core-query-interface-adapter` | SQLite DAO **14 本**（`*DaoImpl`）と公開テストダブル **`InMemory*Dao` 13 本**、1 表 1 引当（`cargo lint dao-single-table`）、1 要求 1 読取専用接続を明記 | `query/interface-adapter/src/`（`*DaoImpl` 14 / `InMemory*Dao` 13 を実測） | DAO 実装テスト | 11 §4.1 / C6 |
| 8 | `ReadModelUpdater` | `core-read-model-updater` | 「Lambda 型の差分関数」から**中間クレートの二層構造**へ書き直し。`JournalReader` の **9 メソッド = `async fn` 8 + 同期 `fn prepare_read_model` 1**、`read_*` **17 表**の全数列挙、`amadeus_projection_checkpoint` / `amadeus_read_model_head` / publication 系 **6 表**、`prepare → publish` の 2 Tx、steering の別 Tx、集約 `replay` + クエリメソッド呼出、逐語文言の所有を追記 | `read-model-updater/src/orchestration/{journal_reader.rs:38, journal_reader_impl.rs, read_model_updater.rs:78,141, publication_store.rs, steering_source.rs}`、`read_tables/`、`workspace/wording.rs` | RMU 単体テスト + 投影ゴールデンテスト | 11 §2.3 / §4.1 / C5 / C6 |
| 9 | `CliDispatcher` | `modules/app/aidlc` | 面 **5 つ**（素の `aidlc`/`aidlc-orchestrate` + `aidlc-utility` / `aidlc-log` / `aidlc-state` / `aidlc-bolt`）と配線済み動詞 13、未配線の `予定（未実装、クリティカルパス 4〜6）` を明記。**RMU 呼出の用語を訂正** — 「ポート・ユースケース・RMU は `async fn`、駆動ループ・`tokio::spawn` を持たず、合成ルートが読取前と書込後に `catch_up` を await で直列に呼ぶ」と書き、「同期呼出」とは書かない | `app/src/{main.rs:12, runtime.rs:126-165,194,200,238, wording.rs, cli/face.rs, cli/request.rs:112-128}` | `runtime.rs` のインライン単体テスト（`a_blocked_store_stops_the_catch_up_before_reading` 等） | 10 §3 / 11 §3 / ADR-006 |
| 10 | `CoreInfrastructure`（~~`CanonJson`~~ + ~~`InfraIo`~~ — 統合） | `core-infrastructure` | 2 コンポーネントを 1 クレートへ統合し、モジュール **7 つ**（`canon_json` / `collections` / `atomic` / `append_only` / `fs_meta` / `codec` / `secret_file`）を列挙。`FirstClassCollection` trait と `Collection<T>` / `NonEmptyCollection<T>`（#114）、`ObjectMembers` の適合を追記（T4） | `infrastructure/src/lib.rs:20-26`、`collections/`、`canon_json/value/object_members.rs:124` | `collection_contract_test.rs` / `collections_test.rs` / hash-canonical 受入表 | 01 §7 / K4 / K5 |
| 11 | `HarnessClaude` | `harness-claude` | **実測 3 行のスタブ**であることと、中身が `予定（未実装、クリティカルパス 5）` であることを明記 | `harness/claude/src/lib.rs`（3 行） | — | 01 §7 |
| 12 | `HarnessInfrastructure`（**新設**） | `harness-infrastructure` | 旧版に無かったクレートを追加。**25 行の憲章 doc のみ**で実体は `予定（未実装、クリティカルパス 5）` | `harness/infrastructure/src/lib.rs`（25 行） | — | infrastructure-layer.md |
| — | ~~`PublishedLanguage`~~ | （解消） | 独立クレート `message-catalog` の解体（2026-08-29）に追随してコンポーネントを解消。文言は出す側の `wording`、監査語彙は `WorkspaceModel`、`Directive` 系は `QueryUseCases` が所有することを `## Component Summary` の直後に履歴つきで明記 | `app/src/wording.rs`、`read-model-updater/src/workspace/wording.rs`、`workspace/audit_events.rs`、`query/use-case/src/directive.rs` | — | C1 / C5 |

付随して `## Component Diagram`（mermaid + テキスト代替）・`## Component Summary`（Crate 列を追加した 12 行）・`## Entity Ownership`（6 エンティティ + 旧 `WorkflowExecution` 行を打消し線で履歴保持）・`## External Dependencies`・`## Rationale` をすべて現行へ書き直した。`## Review` 見出し以降は 1 バイトも変えていない。

### B-4. `inception/contract-design/contract-summary.md`（改訂 12 件、+139 行）

| # | 節 | 改訂内容（何を何に） | 出典注記 | 根拠: コードの所在 | テスト | 仕様の該当節 |
|---|---|---|---|---|---|---|
| 1 | §1 一覧 C3 行 | 「`WorkflowExecutionRepository` / `JournalReader`」→ コマンド側ポート 4 本の実名 + RMU 所有の `JournalReader` | B12 2026-08-30 改名 | `use-case/src/orchestration/port/mod.rs` | ポート契約テスト | C3 |
| 2 | §1 一覧 C4 行 | 動詞が現行 3 本（`find_by_id` / `find_for_intent` / `store`）である旨を追記 | 実測 2026-09-07 | `port/workflow_definition_repository.rs:63-114` | ポート契約テスト | C4 |
| 3 | §1 一覧 C5 行 | 「同一プロセスの型（`WorkflowExecutionEvent`）」→ `IntentExecutionEvent` 16 変種 + `IntentEvent` 1 / `WorkflowDefinitionEvent` 2 / `CompiledDefinitionEvent` 4 | B12 2026-08-30 | `orchestration/intent_execution_event.rs:45-60`（16）、`intent_event.rs`、`workflow_definition_event.rs`、`compiled_definition_event.rs` | イベント族の単体テスト | C5 |
| 4 | §1 一覧 C6 行 | 我々の表に `amadeus_read_model_head` / publication 系 6 表 / `read_*` 17 表を追記 | 実測 2026-09-07 / ADR-011 | `read-model-updater/src/` の `CREATE TABLE IF NOT EXISTS` 全件 | RMU 単体テスト | C6 / 11 §4.1 |
| 5 | C3 見出し | 旧題を打消し線で残し「コマンド側 Repository 4 本と `JournalReader`」へ | B12 2026-08-30 | 同 #1 | — | C3 |
| 6 | C3 冒頭（新規ブロック） | **現行のポート 4 本を表で提示**し、v2 / v3 世代の trait 全文と 4 層の追記を履歴と位置づけ。改名・`expected_version` 引数の消失・`Rehydrated*` 等の撤去（型として 0 件）・`RepositoryError<Id>` 1 本・`JournalReader` の 9 メソッド（`async fn` 8 + 同期 1）・テストダブル 3 層を明記 | ADR-010 / B12・B13 2026-08-30 / オーナー裁定 2026-08-31 | `port/*.rs`、`port/mod.rs:15-18`、`journal_reader.rs:38`、`intent_execution_repository_impl.rs:182,455`、`test_support.rs:1-17` | ポート契約テスト 7 本 + アダプタ契約テスト | C3 |
| 7 | C3 本文 2 行 | 「`core-use-case` が所有する trait は 2 本」を打消し線で失効させ、現行（ポート 4 本 + `JournalReader` は RMU 所有）へ | B8 2026-08-29 / B12 2026-08-30 | `port/mod.rs`、`read-model-updater/src/orchestration/mod.rs:42` | — | C3 |
| 8 | C3 コードフェンス直前 | trait 全文が **v2 世代の履歴**である旨を明示し、フェンス内コメントにも現行名を注記 | B12 2026-08-30 | 同 #6 | — | C3 |
| 9 | C3 約束 ④ | テスト構成の型名を `IntentExecutionRepositoryImpl<EventStoreForMemory<…>>` へ改名し、**「テストダブル型は無く」の訂正**（use-case 層には DIP 制約下の `pub(crate)` フェイク 4 つが実在する）を追記 | T1 / 実測 2026-09-07 | `test_support.rs:1-17`（フェイク 4）、`intent_execution_repository_impl.rs:182` | use-case 単体テスト | C3 |
| 10 | C3 末尾 2026-08-30 追記 ① | 「ジャーナル全再生」を打消し線にし、**最新スナップショット + 差分再生**への失効注記を追加（②〜④は現行のまま有効と明記） | オーナー裁定 2026-09-05 / 実測 | `intent_execution_repository_impl.rs:336-438` | 再構成 43 テスト | C3 / 11 §2.1 |
| 11 | C4 冒頭 + 本文 | 動詞 3 本化・`store` 追加（b30）・`GraphReadError` 廃止 →`RepositoryError<WorkflowDefinitionId>` 1 本・`WorkflowExecution::definition_id()` → **`Intent::definition_id()`** を明記 | ADR-008 / b30 2026-08-31 / b26 2026-08-31 / B12 2026-08-30 | `port/workflow_definition_repository.rs:63-114`、`orchestration/intent.rs` | ポート契約テスト | C4 / 12 §5 |
| 12 | C5 冒頭 / C6 冒頭 / C1 / §4 | C5: イベント族 4 と変種 16 / 1 / 2 / 4 の表、`StageCompleted` 撤去、イベント = エンティティ（`id: XxxEventId` + `aggregate_id`）、`Started` の内容を明記。C6: `read_*` 17 表の全数・`amadeus_read_model_head`・publication 系 6 表・`PRAGMA user_version` を明記し、`payload` コメントの「16 属性」→ **12 属性**。C1（:48 / :68）と §4（:477）の `message-catalog` 3 件 → 出す側の `wording`（解体 2026-08-29） | b42 / オーナー裁定 2026-09-02 / b39 / 実測 2026-09-07 | `intent_execution_event.rs:45-60`、`read-model-updater/src/` の `CREATE TABLE` 全件、`intent_execution.rs`（12 属性）、`app/src/wording.rs` / `read-model-updater/src/workspace/wording.rs` | イベント単体テスト・RMU 単体テスト・投影ゴールデン | C1 / C5 / C6 / §4 |

「同一シャード内の直接行と投影行の順序」は §4 に**未決のまま残した**。打消し線つきの旧 manifest 綴り（:285 / :305 / :388 相当）には触れていない。`## Review` 見出し以降は 1 バイトも変えていない。

### B-5. `inception/units-generation/unit-of-work.md`（注記 5 件のみ、+10 行。本文は書き換えていない）

| 行 | 現在の本文（不変） | 添えた注記 | 根拠: コードの所在 | テスト | 仕様の該当節 |
|---|---|---|---|---|---|
| :34 | U3 行「SQLite EventStore と WorkflowExecutionRepository」 | 改名（`IntentExecutionRepository` — B12 2026-08-30） | `port/intent_execution_repository.rs` | ポート契約テスト | C3 / 11 §3 |
| :64 | 「`version` は失効（2026-08-29 / ADR-010・B7）: 楽観 version は集約の外へ — `RehydratedWorkflowExecution` が持ち回る」 | **この失効注記自体が B13（2026-08-30）で再失効** — version は集約の内側、`Rehydrated*` / `StatePosition` / `StoreVersion` 撤去、`expected_version` 引数なし | `port/mod.rs:15-18`、`intent_execution.rs`、`intent_execution_repository_impl.rs:455` | ポート契約テスト 7 本 | C3 / ADR-010 |
| :83 | 「独自スキーマ …（ADR-007）。`InMemoryWorkflowExecutionRepository` を先に書く（gateway-taxonomy §6）」 | 失効 — スキーマは本家 event-store-adapter-rs（`=3.0.0`）、インメモリは `IntentExecutionRepositoryImpl::in_memory()` が唯一の公開形 | `intent_execution_repository_impl.rs:182`、`Cargo.toml` のピン | アダプタ契約テスト（両バックエンド） | C6 / gateway-taxonomy §5 / §6 |
| :91 | 「ポート trait（`WorkflowExecutionRepository`、EventStore 同形 trait）はユースケース層に置く」 | 改名（`IntentExecutionRepository`）。置き場（use-case 層 `port/`）は現行どおり。EventStore 同形 trait は ADR-010 で廃止 | `use-case/src/orchestration/port/` | — | C3 |
| :144 | 「テストは `XxxUseCase<InMemoryWorkflowExecutionRepository>` の素の値で組む」 | 改名（`InMemoryIntentExecutionRepository`、`#[cfg(test)]` の crate 私有フェイク）。**公開のインメモリ実装はアダプタ層の `in_memory()`**、クエリ側の公開ダブルは `InMemoryXxxDao` 13 本 | `test_support.rs:1-17` / `orchestration/mod.rs:38-39`、`intent_execution_repository_impl.rs:182`、`query/interface-adapter/src/`（`InMemory*Dao` 13） | use-case 単体テスト | T1 / C3 |

`## Review` 見出し以降は 1 バイトも変えていない（行番号 207 も不変）。

### B-6. `inception/domain-design/decisions.md`（ADR-010 の 1 段落のみ、+6 / −2 行）

| 行 | 改訂内容 | 根拠: コードの所在 | テスト | 仕様の該当節 |
|---|---|---|---|---|
| :476-478 | 当該 3 行の段落を `~~…~~` で打ち消し、直後に失効注記を追記（**削除していない**）。注記の内容 = 楽観 version は集約の内側（`version: usize` / `version()` / `with_version()` / `UNPERSISTED_VERSION = 0`）、`Rehydrated*` / `StatePosition` / `StoreVersion` は撤去、`store(&mut self, &event, &aggregate)` に `expected_version` 引数なし | `port/mod.rs:15-18`、`intent_execution_repository_impl.rs:455` | ポート契約テスト 7 本 | ADR-010 / C3 |

他の ADR・他の段落（TOCTOU の経緯を含む）には触れていない。

## 3. Green / Refactor 後の検査結果（Step 6、実測をそのまま貼る）

> 下記はすべて、§2 冒頭の**クリティカルパス番号の統一（2 → 4）を適用したあと**に再実行した実測である。番号の書き換えは予定表記の文字列内だけなので、(1) sentinel 0 件・(2) 予定表記 10 / 1 件・(3) `## Review` の sha256 3 値・(4) 「同期」0 件・(5) 表整形 `tables ok`・(6) `decisions.md` の diff 範囲はいずれも統一の前後で同一だった。

### (1) sentinel 10 語 grep（履歴マーカー除外）— 所有仕様 2 本

```
$ grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start|WorkflowExecution|RehydratedWorkflowExecution|message-catalog' docs/specs/11-workspace.md docs/specs/12-workflow-definition.md | grep -vE '~~|旧|失効|是正済み|改名|履歴'
（出力なし）
count: 0
```

**13 件 → 0 件**（合格）。

### (2) `予定（未実装` の件数

```
docs/specs/11-workspace.md:10
docs/specs/12-workflow-definition.md:1
```

**0 / 0 → 10 / 1**（各号 1 件以上、合格）。

### (3) `## Review` 節のバイト同一性（3 本）

```
components.md :661        b954159a08ef6a312306efa0ec9ddaeecd0403e7b103b12a7eca13614bdc1dbb
contract-summary.md :593  7374b976335767b3356452648e3ad92fddf1f8730e9f5f070d8aabc75f8f3662
unit-of-work.md :207      b8d0e4a122bab43d0123c1a0515536e2866d05eb7609f6abcbb87c91dd503880
```

行番号は 430 → 661 / 486 → 593 / 207 → 207 と動いたが、**sha256 の 3 値はいずれも Red 基線と同一**（合格）。

### (4) 用語「同期」（RMU 文脈）

```
$ grep -nE '同期' docs/specs/11-workspace.md docs/specs/12-workflow-definition.md <components.md> <contract-summary.md> | grep -iE 'rmu|catch_up|投影'
（出力なし）
0
```

**0 件**（合格）。Refactor で 1 件の擬陽性を解消した — `components.md` の `CliDispatcher` で ADR-006 の引用「ドメインは同期」と「両側を知ってよいのは RMU と…」が同一行に載っていたため行単位 grep に掛かっていた。引用を「ドメイン（集約）は I/O を持たず純粋・同期」に改め、RMU の文を次行へ送って解消した（意味は変えていない）。

改訂後に残る「同期」は次の 4 件で、いずれも規約が認める用法である:

| 所在 | 用法 |
|---|---|
| `components.md`:60 | 「集約は I/O を持たず純粋・同期である」（ドメインの純粋性 — 許容） |
| `components.md`:373 / `contract-summary.md`:122 | 「**同期の** `fn prepare_read_model(&mut self)` 1 本」（`async fn` でない実在メソッドの事実） |
| `components.md`:434 | ADR-006 の引用「async は初期化から、ドメイン（集約）は I/O を持たず純粋・同期」 |
| `components.md`:446 | 「…という意味であって関数が同期という意味ではないので、ここを『同期呼出』とは書かない」（規約そのものの記述） |

（`contract-summary.md`:606 の「非同期タスク」は `## Review` 節内の既存記述であり、触れていない。）

### (5) 表の列数・見出し重複（`unit-test-instructions.md` §1 (4) の python を所有 6 ファイルへ絞って実行）

```
tables ok
```

### (6) `decisions.md` の diff（ADR-010 の段落だけであること）

```
$ git diff --stat -- .../inception/domain-design/decisions.md
 .../260822-stage1-selfhost/inception/domain-design/decisions.md | 8 ++++++--
 1 file changed, 6 insertions(+), 2 deletions(-)

$ git diff -- .../decisions.md | grep -E '^[-+]' | grep -vE '^(\+\+\+|---)'
-  楽観 `version` は集約と memento（`WorkflowExecutionState`）から削除し、**集約の外**を持ち回る
+  ~~楽観 `version` は集約と memento（`WorkflowExecutionState`）から削除し、**集約の外**を持ち回る
-  version）を返し、`store` は `expected_version: usize` を引数に取る。
+  version）を返し、`store` は `expected_version: usize` を引数に取る。~~
+  — 失効（2026-08-30 / B13: 楽観 version は集約の内側 — `version: usize` フィールド、
+  `version()` / `with_version()`、`UNPERSISTED_VERSION = 0`。`Rehydrated*` / `StatePosition` /
+  `StoreVersion` は撤去、`store(&mut self, &event, &aggregate)` に `expected_version` 引数は無い。
+  実測 `port/mod.rs:15-18` / `intent_execution_repository_impl.rs:455`）
```

diff は ADR-010 の当該段落だけである。**`-` 行が 2 本ある点**は保留 1 に記す（内容は削除されておらず、`~~` を付けた同一行が `+` 側に現れている）。

### (7) 書込スコープ（`git status --porcelain` のうち本派遣が触った分）

```
 M aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/decisions.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/inception/units-generation/unit-of-work.md
 M docs/specs/11-workspace.md
 M docs/specs/12-workflow-definition.md
```

所有ファイル 6 本と新規の本報告のみ。コード（`modules/` / `tools/` / `scripts/` / `.github/` / `Cargo.*`）・`formal/`・`docs/specs/research/**`・`docs/specs/deviations.md`・派遣 A の所有ファイルには一切触れていない。

### (8) diffstat（所有ファイル）

```
 .../inception/contract-design/contract-summary.md  | 139 +++-
 .../inception/domain-design/components.md          | 739 ++++++++++++++-------
 .../inception/domain-design/decisions.md           |   8 +-
 .../inception/units-generation/unit-of-work.md     |  10 +-
 docs/specs/11-workspace.md                         |  57 +-
 docs/specs/12-workflow-definition.md               |  58 +-
 6 files changed, 686 insertions(+), 325 deletions(-)
```

## 4. 保留（メインの裁定を仰ぐ。勝手に決めていない）

1. **受入 (6) の「削除行（`-`）が無い」は、ブリーフが指示する打消し線と両立しない。** 既存の 3 行を `~~…~~` で囲むには、その行自体を書き換えるしかないため、`git diff` は必ず `-` / `+` の対を出す。実測の `-` 行は 2 本で、**どちらも同じ内容が `+` 側に `~~` 付きで残っており、文字は 1 つも失われていない**（上記 (6) の diff 全文で確認できる）。「削除行が無い」を「内容の削除が無い」と読むなら合格、「`-` 行が 0 本」と読むなら不合格になる。`unit-test-instructions.md` §1 (6) の文言をどちらに寄せるかを裁定してほしい。ブリーフ本文（「この 3 行の段落を `~~…~~` で打ち消し」）を優先して打消し線を採用した。

2. **publication 系の表数が台帳と食い違う（台帳 4 表 / gap-measurement §1 は 5 表 / 実測 6 表）。** `CREATE TABLE IF NOT EXISTS` の実測は `amadeus_publication` / `amadeus_publication_file` / `amadeus_publication_history` / `amadeus_publication_history_file` / `amadeus_publication_snapshot` / `amadeus_publication_snapshot_file` の **6 表**である（`modules/core/read-model-updater/src/` を全走査）。§4.3 R4 は「`amadeus_publication` 系 4 表」、gap-measurement §1 は「`amadeus_publication*` 5 表」と書いており、いずれとも一致しない。BR5.3（コードが裁く）に従い **6 表** と書き、表名を全数列挙した（`components.md` の `ReadModelUpdater` と `contract-summary.md` の C6）。台帳側の数字の訂正が要るかどうかはメインの判断に委ねる。

3. **`gap-measurement` §2.3 の表に無い 2 行を、受入 (2) を通すために改訂した。** `11-workspace.md` の :95（01 §3.2 の引用）と :165（§5 のチョークポイント）は §2.3 の行単位表に載っていないが、履歴マーカーを持たない sentinel 行なので、改訂しなければ受入 (2) が 0 件にならない。どちらも**名称のみの改訂**（`WorkflowExecution` → `IntentExecution` / `WorkflowExecutionRepositoryImpl` → `IntentExecutionRepositoryImpl`）に留め、出典注記を添えた。:95 については引用元の「遷移動詞 11 個」が現行 15 + genesis と食い違うため、文の構造は変えずに現行の件数を括弧で併記した（引用元の 01 号は派遣 A の所有なので、こちらでは書き換えていない）。

4. **`11-workspace.md` の :35（§1 B5）と :196（§7 実装順序 4）も、表に無いが名称を改訂した。** どちらも同一行に `~~` を持つため受入 (2) の grep には掛からないが、隣接する :49 / :195 が改訂対象であり、同じ文意の行に旧名と新名が混在すると読者を誤らせる。名称のみの改訂に留めている。範囲逸脱と判断されるなら差し戻してほしい。

5. **`components.md` の面の数が台帳と 1 つ違う。** `gap-measurement` §4.2 P9 は「面 `aidlc-bolt` / `aidlc-log` / `aidlc-state` / `aidlc-utility`」の 4 つを挙げるが、実測の `Face` enum は素の `aidlc` / `aidlc-orchestrate` を含めて **5 変種**である（`app/src/cli/face.rs`）。矛盾ではなく数え方の差と判断し、「面は 5 つ — 素の aidlc / aidlc-orchestrate、aidlc-utility、aidlc-log、aidlc-state、aidlc-bolt」と両方が読める形で書いた。

6. **`CompiledDefinitionRepositoryImpl` には `in_memory()` が無い。** 他の 3 実装（`IntentExecutionRepositoryImpl` / `IntentRepositoryImpl` / `WorkflowDefinitionRepositoryImpl`）は `in_memory()` を持つが、`CompiledDefinitionRepositoryImpl` は媒体が配布ファイルなので持たない。`components.md` では「`XxxRepositoryImpl<S>::in_memory()`」を ES リポジトリの一般形として書き、この 1 件の例外は書いていない。明示が要るなら追記する。

7. **`contract-summary.md` の C3 の trait 全文（v2 世代）は、書き換えずに履歴として残した。** ブリーフは「trait 全文が v2 世代 … 節単位の現行化」を求めているが、`## Review` 節の所見 3 がこのコードブロックの `&self` / `&mut self` を名指しで参照しているため、ブロックを削除・置換すると Review 節の指摘が宙に浮く。代わりに **節の冒頭に現行 4 ポートの表を置き**、コードフェンス直前に「以下の trait 全文は v2 世代の履歴である」と明示する形にした。ブロック自体の削除が望ましいなら指示してほしい。

## 5. 触っていないもの

- **『維持』行**: `11-workspace.md` §2.2 Domain Primitive 表（:51-69 相当）と §2.3 ドメインサービス節（:70-85 相当）、`12-workflow-definition.md` §2.1 集約表（:42-56 相当）はいずれも gap-measurement が「維持」としており、1 バイトも変えていない。§2.3 の RMU 呼出の用語は「同期」を含まないことを確認済み（Red 基線 (e)）。
- **履歴行**: 打消し線つきの旧 manifest 綴り（`contract-summary.md` の :285 / :305 / :388 相当）、`decisions.md` の ADR-010 の TOCTOU 段落と他の全 ADR、`11-workspace.md` §6 の不変条件表 J1〜J6 と W6〜W16、§8 の Quint 実験記録、§9 / §10 の未決事項。
- **`## Review` 節 3 本**: `components.md` / `contract-summary.md` / `unit-of-work.md` のいずれも、改訂前後で `tail -n +<Review 行> | shasum -a 256` が同一であることを実測で確認した（上記 3-(3)）。
- **派遣 A の所有ファイル**: `docs/specs/10-orchestration.md`、`docs/specs/01-domain-model.md`、`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` 配下の全ファイル。読取のみ（規則の適用のため README・cqrs-boundaries・gateway-taxonomy 等を参照した）。
- **コードと形式仕様**: `modules/` / `tools/` / `scripts/` / `.github/` / `Cargo.toml` / `Cargo.lock` / `formal/`。すべて読取のみで、`git status` に現れていない。
- **`docs/specs/research/**` と `docs/specs/deviations.md`**: 読んでも書いてもいない。
- **計画・指示文書**: `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md` / `gap-measurement-20260907.md` / `rules.md` / `security-design.md` は読取のみ。
- **`construction/code-generation/memory.md`**: `git status` に `M` で現れるが、本派遣は書き込んでいない（エンジンが維持するステージ日誌であり、他の主体による変更である）。
