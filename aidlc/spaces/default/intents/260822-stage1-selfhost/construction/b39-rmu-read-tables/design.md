# b39 設計 — RMU の構造化投影（是正 Bolt 2・前半）: `read_*` 表とジャーナル 3 ストリームの再生

**日付**: 2026-09-02 / **前提**: `query-side-audit/read-model-spec.md`（§10 オーナー裁定済み）、
`handoff-b38.md`「次」/ **PR**: 直列運用の 1 本目（後半は b40）

## 0. 分割と射程

Bolt 2（RMU の構造化投影）を PR 2 本に分ける。**b39（本書）** = ジャーナル由来の表とトランザクション、
**b40** = 要求時材料の表（`read_run_stage` / `read_scope_change` / `read_config_current`）と steering の
参照入力ダイジェスト（`read_steering_plan` / `read_steering_part`、`SteeringPlan::pack` の RMU 移設）。
クエリ側は触らない（Bolt 3）。upstream 互換の Markdown 面（`aidlc-state.md` / 監査シャード）は現状維持。

## 1. 原則からの導出（何を・なぜ）

1. **ES の基本 — 集約の歴史は自ストリームだけで再生できる。** 現状の `Started` は `intent_id` しか
   運ばず、genesis は `&Intent` を要する（`genesis_state(id, &intent, at)`）。つまり `IntentExecution` の
   ストリームは自己完結していない（b38 ハンドオフの「計画は `Started` で自己完結」は**誤り**だった）。
   是正: `Started` が**集約 id と計画（各ステージの slug / phase / plan_action）**を運び、
   `impl From<(Started, DateTime<Utc>)> for IntentExecution` が genesis イベントから状態を導出する。
   `start` はこの `From` を通る（`Intent` / `WorkflowDefinition` と同型 —
   `coding-rules/aggregate-commands.md`「genesis イベントから集約を導出する変換がリプレイの種」）。
   イベントが intent の材料の複製を運ぶのは**歴史**であり違反ではない（`aggregate-references.md`）。
2. **投影核の入口はイベント列のまま**（cqrs-boundaries 規則 3 の 2026-09-02 追記）。RMU は
   `replay` で集約を起こし、**集約のクエリメソッドを呼んだ答えをそのまま行に写す**。RMU に判断を
   書かない — RMU が持つのは「どのキーでどのクエリを呼ぶか」の列挙だけ。
3. **リードモデルは非正規化**（裁定 §10-1）— 読取コマンドが 1 回の引当で答えを得る形。
4. **原子性** — 行の差し替えとチェックポイント前進は 1 Tx（裁定 §3）。
5. **冪等・決定性** — 壁時計を読まない。`as_of_global_seq` = 走査済み最終通番。全履歴からの
   再計算なので何度走らせても同じ行（差分投影ではなく**全再計算 + 全差し替え**。ジャーナルは
   1 ワークスペース分で小さく、定義イベントも数版しか無い。増分化は必要になった時点で行う）。

## 2. ドメイン変更（スライス A）

- `Started { id: IntentExecutionId, intent_id: IntentId, stages: Vec<StageEntry> }` —
  `new(id, intent_id, stages)`、アクセサ `id()` / `intent_id()` / `stages()`。
- `impl From<(Started, DateTime<Utc>)> for IntentExecution` — 旧 `genesis_state` の本体を移す
  （構造体リテラルは引き続きこの 1 箇所）。`start(id, &intent, at)` は
  `Started::new(id, intent.id().clone(), intent.stages().to_vec())` を作り `From` で状態を起こして対を返す。
- 追加クエリ `IntentExecution::first_in_scope_of_phase(&self, PhaseId) -> Option<StageIndex>`
  （`--phase` ジャンプの目的地 = そのフェーズで**実効プラン** EXECUTE の最初のステージ。b40 / Bolt 3
  が `read_next_jump_phase` で使う。判断は集約に置く）。
- 両側 DTO の追随: コマンド側 `StartedDto`、RMU 側 `StartedDto`（`id` + `stages`。stage の写しは
  それぞれの側の `IntentDto` が持つ stage 表現と同型で**側ごとに複製**）。
- 影響: ローカルの `.aidlc-store.sqlite`（gitignore・機械ローカル）は旧 `Started` を復号できなくなる
  （`Corrupt`）。未配布なので互換口は作らない（`no-backward-compatibility.md`）。再鋳造で対応。

## 3. RMU — 定義ストリームの購読（スライス B）

- `DefinitionEntry { global_seq, definition_id: WorkflowDefinitionId, seq_nr, occurred_at, event: WorkflowDefinitionEvent }`。
- `JournalBatch::new(executions, intents, definitions, scanned_to)` + `definitions()`。
- RMU 側 DTO（読む側の複製）: `WorkflowDefinitionEventDto { Defined(DefinedDto), Redefined(RedefinedDto) }`、
  `DefinitionContentDto { graph, grid, scopes }`、`StageNodeDto` / `ConsumeDeclDto` / `RuleInContextDto` /
  `SensorRefDto` / `ScopeMetadataDto` — コマンド側アダプタ（`workflow_definition_dto.rs` 731 行）と
  **同じワイヤ形式**。`of` / `to_domain` を持つ。両側の一致は横断適合テストで固定
  （`modules/app/aidlc/tests/journal_protocol_conformance.rs` に定義ストリームの節を追加）。
- `journal_reader_impl.rs`: `DEFINITION_EVENT_MANIFEST` の行を `decode_definition_row` で復号する
  （`Redefined` は id を運ばないので行の `aid` が定義 id）。「暫定の読み飛ばし」を撤去。

## 4. RMU — 構造化投影核 `read_tables`（スライス C）

新しい公開 mod `read_tables`（系統 (2) の構造化リードモデル）。1 ファイル 1 公開型。

- `ReadTables::project(history: &JournalBatch) -> Result<ReadTables, ReadTablesError>` — 純粋。
  1. `definitions()` を id ごとに群化 → 先頭 `Defined` から `WorkflowDefinition::from((defined, at))`、
     以降を `replay`。
  2. `intents()` は集約値のまま。
  3. `executions()` を id ごとに群化 → 先頭 `Started` から `IntentExecution::from((started, at))`、以降を
     `replay`。genesis が先頭に無い / `Started.intent_id` の `Intent` が無い → `ReadTablesError`
     （壊れた歴史。`apply_event` 自体の不変条件違反はキャノンどおりクラッシュ）。
  4. 下表の行を計算。`as_of` = `history.scanned_to()`。
- `RequestKind { Bare, Resume, FreeText, Reentry }` ↔ `NextRequest::new(resume, reentry, free_text)` =
  Bare(f,f,f) / Resume(t,f,f) / FreeText(f,f,t) / Reentry(f,t,f)。列値は kebab-case。
- 行型は基本データ型のみ（`String` / `i64` / `bool` / `Option<_>`。配列・構造は canon_json
  `ContractCompact` の JSON 文字列）。

### 4.1 表カタログ（b39 で作る 13 表。PK = 太字）

| 表 | 列 | 計算元（集約のクエリ） |
| --- | --- | --- |
| `read_definition` | **definition_id**, revision, stage_count, scope_count, as_of | `WorkflowDefinition::{id, revision, graph().len, scopes().len}` |
| `read_definition_stage` | **definition_id, stage_slug**, position（文書順）, number, name, phase, execution, condition, lead_agent, support_agents(JSON), mode, for_each, workspace_requires(JSON), produces(JSON), optional_produces(JSON), produces_kinds(JSON), consumes(JSON), requires_stage(JSON), sensors(JSON), scopes(JSON), reviewer, reviewer_max_iterations, review_class, summary_confirmation, plugin, enabled, gated, inputs(JSON), outputs(JSON), rules_in_context(JSON), sensors_applicable(JSON), as_of | `StageNode` の全アクセサ（29 フィールド）。gated = phase ≠ initialization（`StageKey::is_gated` と同じ規則を `StageNode` 側の既存述語で） |
| `read_definition_scope` | **definition_id, scope**, depth, keywords(JSON), skeleton, review_cap, freeform_default, has_grid_column, cost_total, cost_execute, cost_gates, cost_per_unit_stages, as_of | `scopes()` / `grid().contains_scope` / `scope_cost(scope)`（`None` は NULL） |
| `read_definition_scope_keyword` | **definition_id, keyword**, scope, as_of | `ScopeMetadata::keywords`。同じ語を複数 scope が宣言したら scope 名の辞書順で最初 |
| `read_definition_scope_stage` | **definition_id, scope, stage_slug**, action, in_scope_order, as_of | `stages_in_scope(scope)`（action = EXECUTE/SKIP、in_scope_order は EXECUTE のみ 0 始まりの文書順、SKIP は NULL） |
| `read_definition_scope_phase_entry` | **definition_id, scope, phase**, first_stage_slug, as_of | `first_in_scope_stage_of_phase(phase, scope)`（`Some` の行だけ） |
| `read_intent` | **intent_id**, definition_id, definition_revision, scope, request, depth, test_strategy, review, created_at(RFC3339), project_type, project_kind, languages(JSON), frameworks(JSON), build_system, as_of | `Intent` のアクセサ / `WorkspaceScan` |
| `read_intent_stage` | **intent_id, stage_index**, slug, phase, plan_action, conditional, number, name, lead_agent, gated, as_of | `Intent::stages()` の `StageEntry` / `StageDisplay` |
| `read_execution` | **execution_id**, intent_id, status, cursor_index, cursor_slug, parked_at_index, parked_at_slug, parked_active, accepts_commands, autonomy, seq_nr, last_updated_at(RFC3339), state_binding, as_of | `IntentExecution::{status, cursor, parked_at, parked_active, accepts_commands, autonomy, seq_nr, last_updated_at, state_binding}`。slug は `stage_keys()` |
| `read_execution_stage` | **execution_id, stage_index**, slug, phase, checkbox, effective_plan, approved, revision_count, gated, as_of | `checkbox / effective_plan / approved / revision_count / gated(&intent, i)` |
| `read_next_answer` | **execution_id, request_kind**, decision_kind, stage_index, stage_slug, gated, checkbox, as_of | `next_decision(&request)` × 4 kind。decision_kind = run-stage / done / parked / unpark-then-resume / resume-menu / new-work-routing / recover-skip-inconsistency / inconsistent-skip |
| `read_next_jump` | **execution_id, target_index**, target_slug, outcome, refusal, as_of | `jump_resolve(&intent, target)` を全 index で。outcome = forward / backward / redo / refused、refusal = not-running / invalid-target（受理時 NULL） |
| `read_next_jump_phase` | **execution_id, phase**, target_index, target_slug, as_of | `first_in_scope_of_phase(phase)`（`Some` の行だけ） |

DDL は `CREATE TABLE IF NOT EXISTS`（checkpoint 表と同じ流儀）、`JournalReaderImpl::open` で作る。

### 4.2 実装時の差分（b39 実測 — 型は集約の答えに従う）

| 列 | §4.1 | 実装 | 理由 |
| --- | --- | --- | --- |
| `read_definition_stage.workspace_requires` / `inputs` / `outputs` | JSON | INTEGER(bool) / TEXT / TEXT | `StageNode` の答えが `bool` と散文 `&str` |
| `read_intent.languages` / `frameworks` | JSON | TEXT | `WorkspaceScan` の答えが `&str` |
| `read_definition_scope_stage.action` | EXECUTE / SKIP | NULL 許容 | `stages_in_scope` はグリッド列の無い有効スコープで `None` を返す。丸めない |
| `read_execution.cursor_slug` / `read_next_jump_phase.target_slug` | NOT NULL | NULL 許容 | 添字帳を `get` で引くので型上 `Option`（実運用は常に値あり） |
| `read_next_jump.refusal` | 2 綴り | `CommandError` 全変種の綴り | `jump_resolve` の戻り型に `IntentMismatch` 等も含まれる。起きない値を既存に寄せない |
| `read_definition_stage.produces_kinds` | JSON | `[{"artifact","kinds"}]` の配列 | オブジェクトのキーに畳むと同名成果物が潰れる |
| `as_of` | 各行型のフィールド | SQL 側で 1 値を全表に書く | スナップショット全体で 1 つの値を 13 型に複製しない |

ドメインに無かったもの（RMU で導出せず、既存の述語に問うた）: `StageNode` のゲート付き述語は
`StageKey::new(slug, phase).is_gated()` に委ね、`Status` / `CheckboxState` の読取面の綴り（kebab-case）は
`read_tables/spelling.rs` 1 箇所に置いた（`PhaseId` / `PlanAction` / `ExecutionKind` / `StageMode` /
`ReviewClass` / `AutonomyMode` はドメインの `as_str` 系をそのまま使用）。

## 5. 取得ループと Tx（スライス C）

- ポート変更: `JournalReader::advance_checkpoint(&mut self, projection, to, tables: &ReadTables)` —
  **行の全差し替えとチェックポイント前進を 1 Tx**（`BEGIN IMMEDIATE`）。単調性・アンカー照合は
  従来どおり。テストの Fake は行を保持する。
- `catch_up`: 差分空 → 従来どおり何もしない。非空 → Markdown 面を従来どおり描く → 全履歴
  （チェックポイントが ZERO なら差分 = 全履歴、そうでなければ `events_after(ZERO)`）から
  `ReadTables::project` → `advance_checkpoint(projection, last, &tables)`。
- `CatchUpError::ReadTables(ReadTablesError)` を追加。

## 6. テスト

- domain: `IntentExecution::from((Started, at))` = `start(..).0`；Started だけの再生 = start；
  `first_in_scope_of_phase` は overlay 反映後を見る（recompose 後に変わる）。既存 ITF は不変。
- DTO: `StartedDto` 往復（両側）・横断適合（app の `journal_protocol_conformance.rs`）。定義 DTO の
  ワイヤ JSON をゴールデン文字列で釘留め（コマンド側 `dto/tests.rs` と同一の文字列）。
- `read_tables` 契約テスト: フィクスチャ履歴（定義 Defined + Redefined、intent Created、実行 数イベント）
  から各行 == 同じ再生で得た集約のクエリの答え（`read_next_answer` は 4 kind、`read_next_jump` は全 index）。
- SQLite 往復: `advance_checkpoint` → SELECT で戻して等値。チェックポイント後退の拒否で行が変わらない。
- 取得ループ: Fake が受け取る `tables.as_of()` == 前進後のチェックポイント。

## 7. 正本の更新（本 Bolt）

- 仕様 11 workspace: 「構造化リードモデル（`read_*` 表）」の節（媒体・表・更新契機）。
- decisions.md: ADR-011 構造化リードモデルは SQLite `read_*` 表（オーナー裁定 2026-09-02）。
- `aggregate-commands.md`: genesis イベントは集約 id と genesis の材料を運ぶ（`Started` の是正）。
- `cqrs-boundaries.md` 規則 6: 表カタログの所在（仕様 11）。
