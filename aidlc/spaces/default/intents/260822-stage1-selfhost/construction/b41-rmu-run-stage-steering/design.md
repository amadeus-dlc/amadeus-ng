# b41 設計 — 是正 Bolt 2 後半: run-stage 材料・steering・scope-change の `read_*` 表（2026-09-03）

**前提**: b39（13 表・同一 Tx）/ b40（イベント ID）マージ済み。`read-model-spec.md` §4.3〜4.5 と、
クエリ側の現状調査（2026-09-02、下記 §0）に基づく。クエリ側は触らない（Bolt 3 で `read_*` の引当へ切替）。

## 0. 調査で分かった事実（クエリ側の現状 = b41 が事前計算すべきもの）

- run-stage directive は 21 フィールド。出所: 定義（StageView）13、layout パス 4（`stage_file` = `{stage_library_dir}/{phase_dir}/{slug}.md`、
  `memory_path` = `{record}/{phase_dir}/{slug}/memory.md`、`consumes` = `{record}/{artifact}`、`produces` = `{record}/{phase_dir}/{slug}/{artifact}`、
  `inline_context_paths` = `{agent_dir}/{agent}.md`（mode 依存: Inline → lead+support、Mob → lead のみ、他 → 空））、
  グリッド 1（`next_stage` = scope の文書順で自ノードの次以降・最初の EXECUTE の**表示名**）、gate 1（分岐 10 は `is_gated(cursor)` だが実体は phase ≠ initialization = `default_gate` と同じ）、
  steering 1（`rules_in_context` = 配信済みパス台帳）、要求 1（`single`）、`unit`、`narration`（常に未設定）。
  `protocol_modules` は導出: reviewer あり → "reviewer"、mode ≠ Inline または support 非空 → "ensemble"、phase == Construction → "construction"。
  現状パスは**絶対**（Layout から前置）。「ハーネス相対」変換はどこにも無い。
- `directive_digest` = hash_compact(ContractCompact) の素材 7 キー: stage / gate / stage_file / memory_path / next_stage / unit / single。
  `route_digest` の素材: stage / stages（`stages_in_scope(scope)` の slug 全列、EXECUTE で絞らない）。
  `bundle_digest` の素材: チャンク入れ子配列 `[[{path,text}]]`（分割境界がダイジェストの一部）。
  `state_binding`（クエリ側）の素材: scope / cursor / status / parked_at / last_updated / stages — **b38 の集約 `state_binding`（execution_id + seq_nr）と素材が違う**（Bolt 3 で集約側へ統一）。
- steering: 束 = `org.md` → `team.md` → `project.md` → `phases/<phase>.md`（Initialization はスキップ、欠損は正常、読めないのは blocking）。
  **ステージの `rules_in_context` は束の選択に使わない** → 束は **phase の関数**（stage ではない）。`pack` は 20KiB 目標、見出し境界 → コードポイント分割。
  空計画 → bare run-stage（`rules_in_context: []`）。
- continue の照合: token(`h`) → state_binding、route_digest 再計算、run-stage 再構築 + pins 再適用 → directive_digest、bundle_digest。不一致は STATE_MOVED_ON / ROUTE_CHANGED / STALE。
  token は serde DTO（18 キー）+ HMAC 封筒 + base64url（canon_json 不使用）。pins = gate / next_stage / unit / single。
- 分岐 5: scope-change は「`--scope` の値 ≠ state の scope」だけ。config-change は**現在値を見ない**（depth / test_strategy / review のフラグが 1 つでも来たら出す）。
  → `read_config_current` は不要（upstream パリティどおり構文分岐のまま）。`read_execution` に `scope` 列が無い → 追加が要る。

## 1. b41 の表（確定案）

| 表 | キー | 列 | 備考 |
| --- | --- | --- | --- |
| `read_run_stage` | definition_id × scope × stage_slug | phase, lead_agent, support_agents(JSON), mode, gate_default, inline_context_paths_rel(JSON: `agents/{x}.md`), stage_file_rel(`{phase_dir}/{slug}.md`), memory_path_rel(`{phase_dir}/{slug}/memory.md`), consumes_rel(JSON), produces_rel(JSON), sensors_applicable(JSON), reviewer, reviewer_max_iterations(既定 1), review_class, protocol_modules(JSON), next_stage_name, route_digest, directive_digest, as_of | **定義 × scope で決まる**（実行依存なし）。パスは相対、プレゼンタが Layout の各 dir を前置。`directive_digest` の素材から pins（gate/unit/single）を外し環境由来（stage/stage_file_rel/memory_path_rel/next_stage）だけにする — pins は HMAC 封筒内の token 自身の主張なので環境ドリフト検出の素材ではない（token は内部形式で観測互換外） |
| `read_steering_plan` | phase | bundle_digest, part_count, delivered_paths(JSON), source_digest, as_of | 束 = phase の関数。`source_digest` = 規則ファイル群（path+text）のダイジェスト。`catch_up` ごとに比較し変化時だけ再パック |
| `read_steering_part` | phase × part_index | rules_content(JSON `[{path,text}]`) | 1 始まり |
| `read_scope_change` | execution_id × scope | kind（scope-change / same-as-state） | 有効 scope ごとに 1 行（intent の scope と一致なら same-as-state）。無効 scope は行無し |
| `read_execution` | （既存に列追加） | scope | intent から非正規化 |

steering の参照入力: `ProjectionTargets` とは別の読取入力型（`SteeringSource { memory_dir }`）を RMU に渡す。パック（`SteeringPlan::pack` 約 120 行）は RMU の投影ヘルパへ複製し、Bolt 3 でクエリ側から削除。
`rules_in_context`（配信済みパス台帳）は `delivered_paths` を絶対化して載せる（プレゼンタ）。

## 2. continue（Bolt 3 での形、b41 が行を用意）

token の bindings（bundle / directive / route / state）は行の同名列と `WHERE` で等値照合するだけ。行が返れば次部（`read_steering_part`）か終端 run-stage、返らなければ固定文言（fail-closed）。

## 3. RMU の構造

- `read_tables` に行型 4 種を追加（`RunStageRow` / `SteeringPlanRow` / `SteeringPartRow` / `ScopeChangeRow`）、
  `ExecutionRow` に `scope` 列を追加。DDL は `sql.rs` に追加（`read_run_stage` / `read_steering_plan` /
  `read_steering_part` / `read_scope_change`）。
- **ジャーナル由来**（`read_run_stage` / `read_scope_change` / `read_execution.scope`）は `ReadTables::project`
  の一部として従来どおり全再計算・全差し替え。`route_digest` / `directive_digest` は canon_json `hash_compact`
  （`ContractCompact` → sha256 → 生 hex）で、素材キーは §1 のとおり（`directive_digest` は環境由来 4 キー
  `stage` / `stage_file` / `memory_path` / `next_stage` — pins は含めない）。
- **参照入力由来**（steering）は別の投影単位 `SteeringTables::pack(rules: &[RuleContent]) -> SteeringTables`
  （純粋。`SteeringPlan::pack` の分割・パック約 120 行を RMU へ複製 — クエリ側の複製は Bolt 3 で削除）。
  取得ループは `SteeringSource { memory_dir }`（`ProjectionTargets` とは別の**読取入力**型）から
  `org.md` → `team.md` → `project.md` → `phases/<phase>.md`（Initialization はスキップ、欠損は正常、
  読めないのは `CatchUpError::SteeringRead`）を読み、`source_digest`（path + text の `hash_compact`）を
  `read_steering_plan.source_digest` と比べ、**違うときだけ**再パックして差し替える。ジャーナル差分が空でも
  この比較は毎回行う（`catch_up` の早期 return の**前**に参照入力を見る）。
- Tx: ジャーナル由来はチェックポイント前進と同一 Tx（既存 `advance_checkpoint(.., &tables)`）。steering は
  `JournalReader::replace_steering(&mut self, &SteeringTables)` を別 Tx（journal 位置と独立 — `source_digest`
  が整合性の鍵）。Fake は行を保持。
- app: `catch_up(layout)` が `SteeringSource::new(layout.memory_dir())` を渡す。

## 4. テスト
- `read_run_stage`: 各列 = 定義（`StageNode`）/ scope グリッド（`next_stage_name` は文書順で次以降の最初の EXECUTE
  の**表示名**）/ 相対パス規則 / `protocol_modules` の導出規則 / `gate_default`（phase ≠ initialization）。
  `route_digest` の素材 = `stages_in_scope(scope)` の slug 全列（EXECUTE で絞らない）。
- `read_scope_change`: 有効 scope ごとに 1 行、intent の scope と一致なら `same-as-state`。
- steering: パックの無損失性（全チャンク結合 = 入力）、20KiB 目標、見出し境界、コードポイント分割、
  `bundle_digest` はチャンク入れ子配列、`source_digest` 不変なら再投影しない（フェイクで検出）、
  Initialization スキップ、欠損は正常、読めないのはエラー。
- SQLite 往復と Tx。

## 5. 正本の更新
仕様 11 §4.1 の表カタログに 4 表 + `read_execution.scope` を追記。`read-model-spec.md` §4.4 のキーを
「stage_slug」から「phase」へ訂正（束は phase の関数）、§4.3 の `read_config_current` を「不要（upstream は
現在値を見ない）」に訂正。
