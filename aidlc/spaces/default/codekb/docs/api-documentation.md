# api-documentation — 公開 API 面

> リバースエンジニアリング成果物（2026-08-22 実施、`c4d8d95` 時点）。Web API は存在しない。公開面は CLI（現状スタブ）と Rust ライブラリ公開 API のみ。一次情報は開発者スキャン結果（公開シグネチャ全数抽出を含む）。

## CLI 面

- **`aidlc`（`modules/app/aidlc/src/main.rs`）— スタブ。サブコマンド 0 個**。本体は `const fn main()` のみで、マルチコールバイナリ + ディスパッチャ語彙 `<executable> <noun> <verb>`（ADR 0002 / 逸脱台帳 #1 のコマンド綴り写像）が計画済み・未実装。upstream 互換の CLI サブコマンド語彙・JSON エンベロープ・exit code 契約は D6 の互換対象であり、実装はフェーズ A の残作業。
- **`amadeus-lint`（`tools/lint`、`cargo lint [root]` エイリアスで起動）** — 開発ツール CLI。検査 1 動詞のみ。所見は rustc 風 3 行形式で出力し、所見ありで exit 1。

## Rust 公開 API — core クレート

公開は一貫して「private mod + ファサード `pub use`」方式で、mod.rs の列挙が公開 API 宣言そのもの。

### core_domain::orchestration

- 集約 `WorkflowExecution` — コマンド/クエリ 13 面: `start` / `next` / `report_forward` / `gate_start` / `reject` / `revise` / `report_skipped` / `stale_report` / `jump` / `park` / `unpark` / `recompose_flip` / `set_autonomy`。観測面 9 面。
- `EngineSignal` / `Status` / エラー型 2 種 / Domain Primitive 5 種（`AutonomyMode` / `JumpDirection` / `PlanAction` / `SkeletonStance` / `Verdict`）。

### core_domain::workflow_definition

- 集約 `WorkflowDefinition` — `effective_plan_action` / `subgraph_for_scope` / `next_in_scope_stage` / `first_in_scope_stage_of_phase` / `stages_in_scope` / `valid_scopes` / `is_valid_scope` 等。
- `ScopeGrid`（転置導出 `derive_from_graph`）、`StageNode`（28 フィールド + Builder）、エラー型 10 種。
- 注意: `effective_plan_action` は R2 裁定により orchestration 側ドメインサービスへ移設予定（未履行）— この面を新規消費する設計は移設後の形を前提にすること。

### core_domain::workspace

- 状態ファイル純関数サービス 10 本: `get_field` / `set_field` / `set_field_strict` / `set_or_insert_field` / `remove_field` / `parse_checkboxes` / `set_checkbox` / `count_completed` / `classify_state_version` / `reap_eligible`。
- `LockProtocol`（純粋ステップ関数）、逐語定数 3 種、Always Valid newtype 群（`CloneId` / `LockIdentity` / `ShardName` / `SpaceName` 等）。

### core_use_case（ポート trait 2 本）

- `WorkflowDefinitionRepository::find_by_id(&WorkflowDefinitionId)` — 読取専用（`save` なし）。失敗は `NotFound { expected, actual }` / `HarnessIdentity { path, cause }` ほか `GraphReadError`（Bolt B3 で `find()` を廃止 — C4 改訂 / ADR-008）。
- `WorkspaceLock::{acquire, release}` — `acquire(&LockIdentity, AcquireBudget) -> Result<LockGuard, AcquireError>`。`LockGuard` は非 Clone（二重解放不能を型で表現）。**ADR-007 で退役決定**（コード上は Bolt B5（U3）で削除予定 — 現行コードのスナップショットとして記載）。

### core_interface_adapter

- Gateway 実装 2: `WorkflowDefinitionRepositoryImpl`（PL 3 入力読取）、`FsWorkspaceLock`（mkdir-EEXIST ロック + reap CAS）。
- テストダブル 1: `InMemoryWorkflowDefinitionRepository`。
- 機構 trait 2: `Clock` / `ProcessProbe`（各 Fake 付き — ユースケースが消費しないため use-case 層のポートではなく、アダプタ層の注入シーム）。
- 逐語文言関数 3（R4 裁定により `message-catalog` へ移設予定の残存 7 形とは別に、公開面として存在）。

## Rust 公開 API — 共有クレート（Published Language の閉集合）

| クレート | 公開面 | 集合の規模 |
| --- | --- | --- |
| `audit_events` | `EventType` / `EventCategory` | イベント 86 語 / 22 カテゴリの閉集合。MANDATORY 8、CLI_PROTECTED 18。CLI_RESERVED(8)・MERGE_PROTECTED は意図的未定義（upstream 読解待ち） |
| `directive_schema` | `DirectiveKind` | 10 種の閉集合（placeholder 2 種にマーク）。Directive 本体・28KiB 上限・continue_token は後続スライス |
| `message_catalog` | 逐語文言関数 | 7 形、全数 `Captured`（ピン留めソースでバイト一致確認済み、JS `toFixed(1)` 丸め再現含む） |
| `canon_json` | — | スタブ（ADR 0001 の正準 JSON シリアライザ予定地） |

## Rust 公開 API — infra-io（低水準プリミティブ）

ポリシーを持たない I/O 部品の閉集合: `write_file_atomic`（tmp + rename + fsync）、`open_append_only`（O_APPEND \| O_NOFOLLOW）、`is_symlink` / `dev_ino`（lstat）、W_OK 検査、`process_alive`（kill(pid,0) の ESRCH 判定）。依存できるのはアダプタ層と composition root のみ（D4）。

## 契約上の注意

- ドメイン層 API は serde 非依存・I/O ゼロ。シリアライズが要る消費者はアダプタ層のワイヤ構造体を経由する。
- Repository のメソッド語彙は `find_by_id` / `find` / `save` / `remove` + ドメイン概念の named retrieval のみ（gateway-taxonomy.md §2b）。`load` / `get` / `fetch` は禁止語彙であり、doc 内の残滓 5 箇所が C19/C20 として修正待ち。
- 読取専用の型保証（10-orchestration I8）: 書込を禁じたいユースケースには Repository を注入せず、load 済み集約を `&` 参照で渡す設計が既定。
