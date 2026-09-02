# ハンドオフ — b41（Bolt 2 後半）: run-stage 材料・scope-change・steering の `read_*` 表（2026-09-03）

設計書: [`b41-rmu-run-stage-steering/design.md`](b41-rmu-run-stage-steering/design.md)。これで `read-model-spec.md` §4 の表は
`read_config_current`（不要と訂正）を除きすべて揃った（計 17 表）。

## やったこと
- `read_run_stage`（definition_id × scope × stage_slug、19 列）: `StageNode` の属性、相対パス規則、`inline_context_paths_rel`（mode 依存）、
  `protocol_modules` の導出、`next_stage_name`（scope の文書順で次以降の最初の EXECUTE の表示名）、`route_digest` / `directive_digest`
  （環境由来 4 キー: stage / stage_file_rel / memory_path_rel / next_stage — pins は含めない）。
- `read_scope_change`（execution_id × scope → same-as-state / scope-change。定義が履歴に無い実行には行を立てない）、`read_execution.scope`。
- steering: `SteeringTables::pack(&MemoryRules)`（クエリ側 `SteeringPlan::pack` の複製 — Bolt 3 で元を削除）、`read_steering_plan`（phase、
  5 フェーズすべて。base 規則は initialization にも届く）/ `read_steering_part`（phase × part_index）。`SteeringSource { memory_dir }` を
  取得ループが読み、`source_digest` が変わったときだけ `replace_steering`（別 Tx）。`catch_up` の早期 return の前に参照入力を見る。
- app: `catch_up(layout)` が `SteeringSource::new(layout.memory_dir())` を渡す。

## 申し送り（Bolt 3 で扱う）
- **`directive_digest` の素材はクエリ側の現行 7 キー（gate / unit / single 含む）と別物**。Bolt 3 で continue token の束縛を行の同名列との
  等値照合に切り替えるとき、pins は token 自身（HMAC 封筒内）の主張として再適用し、環境ドリフトの検出は 4 キー版で行う。
- `next_stage_name` に対応する集約の静的クエリ（`WorkflowDefinition::next_execute_in_scope(scope, slug)` 相当）が無く、RMU が `stages_in_scope`
  の文書順の列を畳んでいる（b39 の `in_scope_order` と同種の列挙）。集約に生やしてその呼出に置き換えるのが本来形 — Bolt 3 でクエリ側の
  `next_in_scope_name` を消すときに合わせて判断。
- `read_execution.scope` は intent からの非正規化。`MemoryRules`（RMU 側の値型）は steering の入力束。

## 次
2b（#85 = A: 非ゲート完了パイプラインの撤去 — `complete_stage` / `StageCompleted`（両側 DTO）/ RMU 投影と文言 / commit_verdict の非ゲート腕、
イベント 12 → 11 変種）→ Bolt 3（クエリ側縮小: DAO を `read_*` の引当へ、ユースケースは `find` → View、分類と文言はコントローラ /
プレゼンタへ、判断型と 2 パーサ・steering 複製の削除、`cargo lint` ルール追加、golden で外部観測不変を固定）。
