# logical-components — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Design（Construction 3.3）成果物（Unit: U2、kind: library）。出典: `../nfr-requirements/security-requirements.md`、
> `../nfr-requirements/tech-stack-decisions.md`（依存・定義の識別子・B3 の範囲拡張）、`../functional-design/functional-spec.md`
> （§2 インターフェイス、W1〜W7）、`../functional-design/entities.md`、`../functional-design/rules.md`（BR4.1 / BR5.x）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C4 / C5 / C6）、`../../../inception/domain-design/components.md`
> （OrchestrationEngine / WorkflowDefinitionModel）、`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md`、
> 確認事項 `nfr-design-questions.md`（P3 / P4）。
>
> 本 Unit はインフラを持たないため、「論理コンポーネント」= `core-domain` クレート内のモジュール境界とテスト支援の置き場。
> 障害ドメインは「呼出側へ返す `Err`」の 1 つだけで、ブラストラディウスは呼出側の 1 コマンド実行に閉じる。

## 1. コンポーネント一覧

| コンポーネント | 置き場（`modules/core/domain/src/`） | 責務 | 公開面 | 依存 |
|---|---|---|---|---|
| `workflow_execution` | `orchestration/workflow_execution.rs`（private mod、全面改訂） | 集約ルート — 状態・decide（12 コマンド）・`apply_event`・クエリ（`next_decision` / `jump_resolve` / `stale_report` / アクセサ）・`snapshot` / `from_snapshot` / `with_version`・PBT 同居 | `WorkflowExecution` | event, snapshot, stage_index, stage_entry, next_decision, errors, checkbox, autonomy_mode, jump_direction, status, `workflow_definition::{WorkflowDefinition, WorkflowDefinitionId, DefinitionRevision, PlanAction}` |
| `workflow_execution_event` | `orchestration/workflow_execution_event.rs`（新規、private） | 封筒（intent_id / seq_nr / schema_version / occurred_at）+ 12 変種のペイロード（C5 の形、StageSlug 参照） | `WorkflowExecutionEvent`（+ 変種ごとのペイロード型） | stage_entry, `workflow_definition::{StageSlug, WorkflowDefinitionId, DefinitionRevision}`, autonomy_mode, jump_direction |
| `workflow_execution_snapshot` | `orchestration/workflow_execution_snapshot.rs`（新規、private） | 全状態の値オブジェクト（C6 snapshot.payload の論理形、アクセサ公開） | `WorkflowExecutionSnapshot` | checkbox, autonomy_mode, `workflow_definition::PlanAction` |
| `stage_index` / `stage_entry` | `orchestration/stage_index.rs` / `stage_entry.rs`（新規、private） | `StageIndex`（範囲を型保証、集約だけが構築）/ `StageEntry`（slug / phase / plan_action / conditional） | `StageIndex`, `StageEntry` | `workflow_definition::{StageSlug, PhaseId, PlanAction}` |
| `next_decision` | `orchestration/next_decision.rs`（新規、private） | `NextRequest` / `NextDecision`（8 値）/ `EngineSignal` 導出 | `NextRequest`, `NextDecision`, `EngineSignal` | stage_index, checkbox |
| エラー 4 型 | `orchestration/{start_error, command_error, apply_error, snapshot_error}.rs`（private） | 手実装 enum + Display + Error（材料のみ） | `StartError`, `CommandError`, `ApplyError`, `SnapshotError` | stage_index, `workflow_definition::WorkflowDefinitionId` |
| 既存 | `orchestration/{checkbox, autonomy_mode, jump_direction, status}.rs` | 変更なし（`CheckboxState` の分類述語、`AutonomyMode`、`JumpDirection::of`、`Status`） | 既存どおり | — |
| `plan_action` | `workflow_definition/plan_action.rs`（**移動先**、private） | `PlanAction`（EXECUTE / SKIP）— FR8.3 の完全移動。`orchestration` から定義・再輸出を削除 | `workflow_definition::PlanAction` | — |
| `workflow_definition_id` / `definition_revision` | `workflow_definition/workflow_definition_id.rs` / `definition_revision.rs`（新規、private） | Domain Primitive（parse-don't-validate）。id は空・不正形を拒否、revision は `sha256:<hex64>` 形を検証 | `WorkflowDefinitionId`, `DefinitionRevision` | — |
| `workflow_definition` | `workflow_definition/workflow_definition.rs`（改訂） | `id()` / `revision()` の追加、`new(id, revision, graph, grid, scopes)`、`effective_plan_action` / `next_in_scope_stage` の削除（FR8.4）。残す述語は BR4.2 | 既存 + `id` / `revision` | graph, grid, scopes, id, revision |
| ファサード | `orchestration/mod.rs` / `workflow_definition/mod.rs` | 公開 API の列挙（`pub use` のみ。利便再エクスポート無し — module-visibility。`orchestration` は `PlanAction` を再輸出しない） | 上記の公開型 | — |
| ITF 準拠テスト | `modules/core/domain/tests/engine_loop_conformance.rs`（書き換え） | Quint トレースの再生（decide → apply）、射影表の突合せ、合成定義・合成 id | テストのみ | core-domain, serde_json（dev） |

## 2. 境界と隔離

- **クレート境界**: `core-domain` の依存は不変（内部 3 クレート）。serde / canon-json を入れない。JSON 化と revision 計算は
  `core-interface-adapter`（U3 の `WorkflowDefinitionRepositoryImpl` / ワイヤ構造体）。
- **モジュール境界**: 型ファイル mod はすべて private、公開はコンテキスト直下 mod.rs の `pub use` 列挙のみ（`unreachable_pub` deny
  で再輸出漏れはビルドエラー）。`orchestration` → `workflow_definition` の依存は一方向（`PlanAction` / `StageSlug` / `PhaseId` /
  `WorkflowDefinitionId` / `DefinitionRevision` / `WorkflowDefinition` を参照）。逆向き依存は作らない。
- **集約境界**: `WorkflowExecution` は `WorkflowDefinition` のオブジェクトを保持せず `definition_id` で参照（BR2.6）。`start` /
  `next_decision` は `&WorkflowDefinition` を引数で受け取るだけ。
- **Bolt B3 の範囲拡張（C4 改訂の帰結 — ADR-008）**: `core-use-case` の `WorkflowDefinitionRepository` trait を
  `find_by_id(&WorkflowDefinitionId)` に改訂（`find()` 削除）、`core-interface-adapter` の `WorkflowDefinitionRepositoryImpl` に
  id（harness.json `name`）/ revision（canon-json）の付与、`InMemoryWorkflowDefinitionRepository`、既存テスト（golden parity /
  repository impl test / 呼出側 10 ファイル — BR4.1）の同時修正。後方互換の `find()` は残さない。

## 3. 障害ドメインとブラストラディウス

| 障害 | 影響範囲 | 手当て |
|---|---|---|
| ガード不成立（`CommandError`） | 呼出側の 1 コマンド実行（状態不変、イベントなし） | Err で返す。文言はアダプタ層（message-catalog）。ユースケースは中断して上位へ |
| 封筒・ステージ違反（`ApplyError`）/ スナップショット不変条件違反（`SnapshotError`） | 再水和の失敗（U3 が `Corrupt` に写す） | Err で返す。ジャーナルの健全性は U3 の Tx と C6 の UNIQUE 制約で守る |
| 定義の不一致（`DefinitionMismatch`） | `next` 1 回の失敗 | Err。別定義での駆動は契約違反として上位へ |
| 設計と Quint の乖離 | テスト失敗（リリース前に検出） | ITF 準拠テスト + PBT（実装を直す、モデルは不変） |
| 依存の脆弱性 | ビルド全体 | 依存追加なし。既存 `cargo audit`（U10） |

共有資源: なし（I/O・グローバル状態・時計を持たない）。

## 4. テストの配置（NFR2.x）

| 種別 | 置き場 | 内容 |
|---|---|---|
| ユニット（インライン `#[cfg(test)]`） | 各モジュール | ガード境界値（各 Err 変種: happy path + エラー 2 件以上）、`StageIndex` 構築、`StageEntry` 解決（None → SKIP、phase）、実グラフ索引 0〜2 非ゲート、`DefinitionMismatch`、`from_snapshot` の各不変条件 |
| PBT（proptest、`PROPTEST_RNG_SEED` 固定） | `workflow_execution.rs` 同居 | 5 性質（decide = 旧 + apply / replay = execute / seq_nr 単調 / Quint 不変条件 / Err 無副作用）、`from_snapshot(snapshot()) == self` |
| ITF 準拠（受け入れゲート） | `tests/engine_loop_conformance.rs` | Quint トレース再生 + 射影表（BR2.5）。モデル不変 |
| 受入手順（Bolt B3） | PR の受入チェック | `grep -rnE 'enum PlanAction\|pub use .*PlanAction' modules/core/domain/src/orchestration` = 0、CI 3 ジョブ + audit 緑、`cargo llvm-cov --package core-domain` の基準値記録 |

## 5. Infrastructure Design への橋渡し

infrastructure-design は本 intent でスコープ外（SKIP）。U2 はインフラ資源を持たないため引き渡し事項なし。CI（U10）側の関係:
`cargo audit` / `unsafe_code` forbid / カバレッジ床の対象に `core-domain` が含まれること。
