# nfr-design-questions — U2 ドメイン ES コア（`u2-domain-es-core`）

> NFR Design（Construction 3.3）の質問票（Unit: U2、kind: library）。出典: `../nfr-requirements/security-requirements.md`
> （NFR1.1〜1.3 / NFR2.1〜2.4 / NFR3.1〜3.4 / NFR4.1〜4.5、STRIDE）、`../nfr-requirements/tech-stack-decisions.md`（依存追加なし、
> 定義の識別子、PBT / ITF、エラー型）、`../functional-design/functional-spec.md`（§2 インターフェイス、W1〜W7、§5 エラー）、
> `../functional-design/rules.md`（BR1.x〜BR5.x、BR2.6）、`../functional-design/entities.md`、`../../../inception/contract-design/
> contract-summary.md`（C3 / C4 / C5 / C6）、`../../../inception/domain-design/decisions.md`（ADR-001〜008）、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（module-visibility / field-visibility / tell-dont-ask / domain-equality）。
> performance / scalability / reliability / observability の要求・設計は kind = library のため存在せず、本ステージの成果物は
> `security-design.md` / `logical-components.md` / `traceability.json` の 3 つ。
>
> **質問なし。** 耐障害・スケール・キャッシュ・観測のパターン選択は純粋な集約には無く、セキュリティ設計（不変条件の検査点・
> Err 境界・ペイロードの素通し・依存ゼロ）と論理コンポーネント分割（`core-domain` 内のモジュール境界・公開面・テスト配置）は
> NFR 要求・技術選定・機能設計・coding-rules から一意に決まる。次の前提を確認して成果物へ進む。

## 前提（確認事項）

- P1. セキュリティ設計 = **不変条件の検査点を 3 か所に集約**する: (a) decide（各コマンドのガード — BR1.x、Err は状態不変）、
  (b) `apply_event`（封筒 seq_nr の連続性 — BR2.1、未知ステージ — UnknownStage）、(c) `from_snapshot`（長さ一致 / cursor in-scope /
  active ≤ 1 / gated Completed ⇒ approved / parked_at = cursor / definition_id の存在 — SnapshotError）。`next_decision` は
  definition_id の一致検査（BR2.6）。どの検査も panic せず Err（NFR3.2 / NFR4.3）。
- P2. ペイロード・秘密情報: 人間入力は `String` の素通し（加工・切詰め・ログなし — NFR4.4）。集約はログ基盤・時計・乱数・環境変数を
  持たない（NFR3.1）。`DefinitionRevision` の計算（canon-json）はアダプタ層、ドメインは値を運ぶだけ（NFR4.1 / NFR4.5）。
- P3. 論理コンポーネント（`core-domain` 内、module-visibility 準拠 — 型ファイル mod は private、公開はコンテキスト直下の
  mod.rs の `pub use` 列挙のみ）:
  - `orchestration/`: `workflow_execution`（集約 — 状態・decide・apply・クエリ）/ `workflow_execution_event`（封筒 + 12 変種）/
    `workflow_execution_snapshot` / `stage_index` / `stage_entry` / `next_decision`（NextRequest / NextDecision / EngineSignal 導出）/
    `command_error`・`apply_error`・`snapshot_error`・`start_error`（手実装 enum + Display + Error）。既存の `checkbox` /
    `autonomy_mode` / `jump_direction` / `status` は残す。
  - `workflow_definition/`: `plan_action`（完全移動 — FR8.3）/ `workflow_definition_id` / `definition_revision`（新設、Domain Primitive）、
    `workflow_definition` に `id()` / `revision()` を追加。`effective_plan_action` / `next_in_scope_stage` は削除（FR8.4）。
  - 公開面: `core_domain::orchestration::{WorkflowExecution, WorkflowExecutionEvent, WorkflowExecutionSnapshot, StageIndex,
    StageEntry, NextRequest, NextDecision, EngineSignal, CommandError, ApplyError, SnapshotError, StartError, …}`、
    `core_domain::workflow_definition::{PlanAction, WorkflowDefinitionId, DefinitionRevision, …}`。利便再エクスポート無し。
- P4. 障害ドメインとテスト配置: 障害は「呼出側へ返す `Err`」の 1 ドメイン（ブラストラディウスは 1 コマンド実行）。ユニットは
  各モジュールのインライン `#[cfg(test)]`、PBT（5 性質 — NFR2.2）は `workflow_execution` 同居、ITF 準拠は
  `tests/engine_loop_conformance.rs`（書き換え、合成 WorkflowDefinitionId / 合成計画）。infrastructure-design は SKIP（引き渡し
  なし）。

## Consolidated Summary Confirmation

- U2 に固有の NFR 設計質問はなし。耐障害・スケール・キャッシュ・観測のパターンは純粋な集約に不要
- セキュリティ設計（P1 / P2）: 不変条件の検査点は decide / apply_event / from_snapshot の 3 か所（+ next_decision の definition_id 検査）、すべて Err で panic なし。人間入力は素通し、時計・乱数・環境・ログなし、revision 計算はアダプタ層
- 論理コンポーネント（P3）: `orchestration/` に集約・イベント・スナップショット・StageIndex / StageEntry・NextDecision・エラー 4 型の private mod、`workflow_definition/` に PlanAction（移動）と WorkflowDefinitionId / DefinitionRevision（新設）、公開はファサードの `pub use` 列挙のみ
- 障害ドメインとテスト配置（P4）: Err の 1 ドメイン、ユニット同居 + PBT 同居 + ITF は engine_loop_conformance.rs、infrastructure-design は SKIP

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
