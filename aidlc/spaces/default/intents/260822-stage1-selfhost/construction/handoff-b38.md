# ハンドオフ — 是正 Bolt 1 / b38: 判断の集約復帰（2026-09-02）

## やったこと（domain のみ — クエリ側・RMU は触っていない）

- `IntentExecution::next_decision(&NextRequest) -> NextDecision` を復帰（仕様 10 §2.3 の所在）。
  b26 以前の `&Intent` / `&WorkflowDefinition` 引数と取り違えガードは撤去 — 計画は `Started` で
  自己完結し、RMU が `replay` した集約自身に問う形では他集約の参照が要らない。
- `NextRequest` / `NextDecision` / `EngineSignal` / `StateBinding` を orchestration に、
  `ScopeCost` を workflow_definition に新設（1 ファイル 1 公開型）。
- `IntentExecution::state_binding()`（`execution_id` + `seq_nr` の CompactRaw ダイジェスト）と
  `WorkflowDefinition::scope_cost(scope)`（upstream `gridCostSummary` の写し）を集約のクエリとして追加。
- Quint `engine_loop` の**観測面**（`lastDirective`）の ITF 照合を domain 側
  `tests/engine_loop_conformance.rs` へ復帰（`assert_signal`）。クエリ側の同名 ITF は Bolt 3 で消える。
- 集約のモジュール doc・仕様 10 §2.3 / §3 を更新。

## 移行期の状態（意図的な重複）

クエリ側の `ExecutionStateView::next_decision` / `DefinitionView` の述語 / `scope_cost` は
**Bolt 3 まで残る**。判断の正本は集約側であり、Bolt 2 で RMU が集約のクエリを呼んで
`read_*` へ投影し、Bolt 3 でクエリ側の複製を削除する（`read-model-spec.md` §7 / §9）。

## 次（Bolt 2 — RMU の構造化投影、`read-model-spec.md` §4 / §6）

1. RMU が `WorkflowDefinition` ストリームを購読する（現状は読み飛ばし）。
2. 取得ループが intent / execution / definition の集約を `replay` で起こし、投影核へ渡す
   （投影核の入口はイベント列のまま — 規則 3 の 2026-09-02 追記）。
3. SQLite `read_*` 表（§4.1〜4.4）をチェックポイントと同一 Tx で差し替える。
4. steering 参照入力（memory 規則ファイル）のダイジェスト比較リフレッシュ。
5. 契約テスト: 各表 = 集約クエリの答え（`read_next_answer` は `next_decision` × 4 request_kind）。
