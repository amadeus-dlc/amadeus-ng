# code-generation-plan — U2 ドメイン ES コア（`u2-domain-es-core`）

> Code Generation（Construction 3.5）の計画（Unit: U2、kind: library、Bolt: B3、規模 L）。出典:
> `../functional-design/functional-spec.md`（§2 インターフェイス、W1〜W7、§4 状態遷移、§5 エラー）、`../functional-design/rules.md`
> （BR1.0〜BR1.9 / BR2.1〜BR2.6 / BR3.1〜BR3.3 / BR4.1〜BR4.2 / BR5.1〜BR5.4）、`../functional-design/entities.md`（エンティティ正本）、
> `../nfr-requirements/security-requirements.md`（NFR1.1〜NFR4.5）、`../nfr-requirements/tech-stack-decisions.md`、
> `../nfr-design/security-design.md`、`../nfr-design/logical-components.md`（モジュール分割・テスト配置・B3 の範囲拡張）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C4（find_by_id）/ C5 / C6）、`../../../inception/domain-design/
> decisions.md`（ADR-001〜008）、`../../../inception/units-generation/unit-of-work.md`（U2）、`../../../inception/requirements-analysis/
> requirements.md`（FR1.3 / FR2.1 / FR3.1 / FR3.3 / FR8.3 / FR8.4、NFR1〜NFR4）、`../../../inception/delivery-planning/bolt-plan.md`（B3）、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（全規則）、`code-generation-questions.md`（Q1）。
>
> 実装はワークスペースルート（`modules/core/domain/`、`modules/core/use-case/`、`modules/core/interface-adapter/`、`tests/golden/`）に
> 書く。記録ディレクトリにはコードを置かない。brownfield: 既存ファイルはその場で変更し、複製ファイルを作らない。**後方互換の
> ための旧 API（`report_forward` / `gate_start` / `find()` 等）は残さない**（オーナー裁定 2026-08-23）。

## 1. 前提と範囲

- **作るもの**: (1) `core-domain` の `WorkflowExecution` をイベントソーシング形 FSM に全面改訂（decide / apply_event / snapshot /
  from_snapshot / with_version、12 イベント、`StageIndex` / `StageEntry`、`NextRequest` / `NextDecision` / `EngineSignal`、エラー 4 型）。
  (2) `PlanAction` の `workflow_definition` への完全移動（FR8.3）と `effective_plan_action` / `next_in_scope_stage` の削除（FR8.4）。
  (3) `WorkflowDefinition` のエンティティ化 — `WorkflowDefinitionId` / `DefinitionRevision` の新設と `id()` / `revision()`（ADR-008）。
  (4) C4 改訂の波及: `core-use-case` の `WorkflowDefinitionRepository::find_by_id(&WorkflowDefinitionId)`（`find()` 削除）と
  `GraphReadError::{NotFound, HarnessIdentity}` の追加、`core-interface-adapter` の `WorkflowDefinitionRepositoryImpl`（id は
  `<data_dir>/harness.json` の `name`、revision は 3 入力の正準 JSON の `sha256:`）と `InMemoryWorkflowDefinitionRepository`、
  既存テスト（repository impl test / golden parity test）、ゴールデン dir への `harness.json`（upstream ピンの実バイト）。
  (5) ITF 準拠テスト `engine_loop_conformance.rs` の新 API への書き換え。
- **作らないもの**: Repository（`WorkflowExecutionRepository`、SQLite、EventStore — U3）、投影（U4）、ユースケース（U5 / U6）、
  CLI（U7）、Quint モデルの改訂（不要 — BR2.5 の射影で 1:1）、仕様文書の改訂（12 号 §2.1 の識別子追記は U9）。
- **ブランチ**: `origin/main`（#26 のスカッシュ `0092761`）から `bolt/b3-u2-domain-es-core` を切る。最初のコミットは aidlc 記録
  （`aidlc/` 配下）、以降はコードのコミット（意味単位）。PR は Bolt ゲート承認後にコンダクタが 1 本だけ開く（直列運用、
  squash-merge、コミット名 = Bolt slug）。開発エージェントは push / PR を行わない。
- **定義の識別子（Q1）**: `WorkflowDefinitionId` の値は `<data_dir>/harness.json` の `name`（upstream ピンにも同ファイルあり —
  `claude`）。Q1 = B なら `aidlc:<name>`。`DefinitionRevision` = `canon_json::hash_canonical(JsonValue{ "stage_graph": <stage-graph.json
  の値>, "scope_grid": <scope-grid.json の値（欠損時は導出グリッドを直列化）>, "scopes": [<identity frontmatter を name 昇順>] })`
  （`sha256:<hex64>`）。revision は値属性であって ID ではない。
- **コーディング規則**（正本 `coding-rules/`）: フィールド既定 private（アクセサ公開）、型ファイル mod 既定 private（公開はコンテキスト
  直下 mod.rs の `pub use` 列挙のみ、利便再エクスポート禁止 — `orchestration` は `PlanAction` を再輸出しない）、ドメイン同値は
  `PartialEq` / `Eq`、`unwrap` / `expect` はプロダクトコード禁止、`missing_docs` deny、手実装エラー enum + `Display` + `Error`、
  Tell-Don't-Ask（checkbox の分類は `CheckboxState` の述語、ゲート前提集合は `// amadeus-lint: allow(checkbox-vocabulary)` + 不変条件番号）、
  集約は Repository を呼ばない、ユースケース層は trait のみに依存。

## 2. 公開 API（設計の写し — 実装の契約）

```text
// core_domain::orchestration（ファサード pub use のみ）
WorkflowExecution::start(id: IntentId, def: &WorkflowDefinition, scope: &str, request: String)
    -> Result<(WorkflowExecution, WorkflowExecutionEvent), StartError>     // Started を返す。def.id()/revision() を記録（検査しない）
complete_stage(&mut self) / open_gate(&mut self, artifacts: Vec<String>) / approve_gate(&mut self, user_input: Option<String>, phase_boundary: Option<PhaseBoundary>)
reject_gate(&mut self, feedback: Option<String>) / revise_stage(&mut self) / skip_stage(&mut self, reason: String)
jump(&mut self, target: StageIndex) / park(&mut self) / unpark(&mut self) / recompose(&mut self, flips: &[StageIndex]) / set_autonomy(&mut self, AutonomyMode)
    -> Result<WorkflowExecutionEvent, CommandError>                       // 1 コマンド 1 イベント、Err は状態不変
apply_event(&mut self, &WorkflowExecutionEvent) -> Result<(), ApplyError>  // seq_nr 連続性 / UnknownStage / 不変条件
next_decision(&self, &WorkflowDefinition, &NextRequest) -> Result<NextDecision, CommandError>   // DefinitionMismatch を検査
jump_resolve(&self, StageIndex) -> Result<JumpDirection, CommandError>     stale_report(&self, StageIndex) -> Result<NextDecision, CommandError>
snapshot(&self) -> WorkflowExecutionSnapshot    from_snapshot(WorkflowExecutionSnapshot) -> Result<Self, SnapshotError>   with_version(self, u64) -> Self
stage_index(&self, usize) -> Option<StageIndex>  accepts_commands(&self) -> bool  definition_id() / definition_revision() / intent_id() / stages() / cursor() / checkbox(StageIndex) / approved(StageIndex) / effective_plan(StageIndex) / gated(StageIndex) / status() / parked_at() / autonomy() / revision_count(StageIndex) / seq_nr() / version() / stage_count()
EngineSignal::from(&NextDecision)   // RunStage / Done / Parked / EngineError の導出（BR3.1）
WorkflowExecutionEvent { intent_id, seq_nr, schema_version = 1, occurred_at, payload: 12 変種 }   // 封筒 + ペイロード、アクセサ公開
StageIndex（集約だけが構築）、StageEntry { slug, phase, plan_action, conditional }、NextRequest { resume, reentry, free_text }、NextDecision（8 値）
StartError { UnknownScope, Empty, InitializationMustExecute, InitializationMustBeUnconditional }
CommandError { NotRunning, CheckboxPrecondition { stage, actual }, NotSkippable(StageIndex), NotStale(StageIndex), InvalidTarget(StageIndex), RefusedUnderAutonomy, DefinitionMismatch { expected, actual } }
ApplyError { SequenceGap { expected, actual }, UnknownStage(StageSlug), InvariantViolation(String) }   SnapshotError { InvariantViolation(String) }

// core_domain::workflow_definition
PlanAction（移動）、WorkflowDefinitionId（parse、非空）、DefinitionRevision（parse、`sha256:<hex64>`）
WorkflowDefinition::new(id, revision, graph, grid, scopes)、id()、revision()   // effective_plan_action / next_in_scope_stage は削除

// core_use_case::orchestration
trait WorkflowDefinitionRepository { fn find_by_id(&self, id: &WorkflowDefinitionId) -> Result<WorkflowDefinition, GraphReadError>; }
GraphReadError += NotFound { expected: WorkflowDefinitionId, actual: WorkflowDefinitionId } / HarnessIdentity { path, cause }   // harness.json 欠落・不正
```

設計からの差分（記録）: `occurred_at` はコマンド引数ではなく `WorkflowExecution::start` / 各 decide が受け取る `occurred_at: &str`（ISO 8601
UTC の文字列、呼出側が時計から渡す）— 集約は時計を持たない（NFR3.1）。`IntentId` は既存 `workspace` コンテキストに無ければ
`orchestration` に Domain Primitive として新設（`<kebab-slug>-<id8>` を parse）。`PhaseBoundary` は C5 の `phase_boundary` 投影材料の
値レコード（`from_phase` / `to_phase`、呼出側供給 — 集約は検証しない）。`Status` は `workflow_execution.rs` のインライン定義から private mod
`status.rs` に切り出す（module-visibility）。`skeleton_stance` / `verdict` は触らない。

## 3. 規則の実装方針（BR → コード）

| 規則 | 実装 |
|---|---|
| BR1.0 accepts_commands | `fn accepts_commands(&self) -> bool { self.status == Running && self.parked_at != Some(self.cursor) }`。unpark 以外の decide は先頭でこれを検査し `NotRunning` |
| BR1.1 1 コマンド 1 イベント | decide = ガード → イベント構築（`self.next_event(payload, occurred_at)`）→ `self.apply_event(&ev)`（Ok 前提）→ `Ok(ev)`。ガード不成立で `self` に触れない。PBT (a) decide 後 == 旧 + apply |
| BR1.2 / BR1.3 / BR1.4 / BR1.5 | `gated(s) = stages[s].phase != PhaseId::Initialization`。`complete_stage` は非ゲートのみ（ゲートで呼ぶと `InvalidTarget`）、`approve_gate` は gated のみ、前提 checkbox は現行 FSM と同じ集合。`skip_stage` は InProgress / Revising ∧（conditional ∨ 実効 SKIP） |
| BR1.6 jump | `jump_resolve` で検証（target < stage_count、非 initialization、in-scope、redo は cursor 非 initialization）→ `Jumped{direction, source, target, stages_reset, stages_skipped}`（slug 列）を構築、apply 側が direction / target から approved 消去を導出（backward: target 以降、redo: source） |
| BR1.7 / BR1.8 | park は gated のみ → `Parked{stage}`、unpark は park 中のみ → `Unparked{}`、recompose は全件検査してから `Recomposed{skipped, added, stages_in_scope}`、set_autonomy → `AutonomyModeSet{mode}` |
| BR1.9 | `stale_report(&self, s)`: accepts_commands ∧ s < cursor ∧ Completed ⇒ `Ok(NextDecision::Done)`、それ以外 `NotStale` / `NotRunning` |
| BR2.1 / BR2.3 | 封筒 seq_nr = 現在値 + 1 でなければ `SequenceGap`。apply は一時状態に適用して不変条件を検証してから差し替え（Err で状態不変）。PBT (b) replay == execute |
| BR2.2 Started | `start` は `is_valid_scope` → `stages_in_scope(scope)`（文書順・全ステージ・PhaseId）+ `graph().nodes()[i].execution()` の索引 zip で `StageEntry` 列。None → SKIP。initialization が SKIP / conditional なら Err。Started = {definition_id, definition_revision, scope, request, stages} |
| BR2.4 / C5 | 変種 12（Started / StageCompleted / GateOpened / GateApproved / GateRejected / StageRevised / StageSkipped / Jumped / Parked / Unparked / Recomposed / AutonomyModeSet）、ペイロードは C5 + `c5_revision_proposal`。`revision_count` は集約フィールド（reject で +1）。網羅 match（`#[non_exhaustive]` 無し） |
| BR2.5 ITF | `engine_loop_conformance.rs`: 合成 `WorkflowDefinitionId("itf")` / `DefinitionRevision("sha256:0…0")`、Quint の plan / conditional からステージ列を合成（索引 0 = initialization、他 = 任意の非 init フェーズ）。`report_forward` → 索引 0 は `complete_stage`、gated は `approve_gate`；`report_awaiting_approval` → `open_gate`；`report_rejected` → `reject_gate`；`report_revised` → `revise_stage`；`report_skipped` → `skip_stage`；`jump_*` → `jump`；`park` / `unpark`；`recompose`（1 要素）；`set_autonomy`（反転）；`next*` → `next_decision` + `EngineSignal::from`。合成定義の作り方は `WorkflowDefinition` を組み立てずに `WorkflowExecution::from_snapshot` 相当の合成 Started で集約を作る（ITF 用コンストラクタ `start_from_entries(...)` を `#[cfg(test)]`…ではなく、テスト側が `StageEntry` 列を直接与える公開関数 `WorkflowExecution::start_with_entries(id, definition_id, definition_revision, scope, request, entries)` を使う — `start` はこれに委譲） |
| BR2.6 / ADR-008 | `start` は def.id()/revision() を記録、`next_decision` は id 不一致で `DefinitionMismatch`。Repository: `find_by_id(id)` は harness.json の name と一致しなければ `NotFound{expected: harness 側, actual: 要求}`… 注: `expected` = Repository が提供できる id、`actual` = 要求された id |
| BR3.1 / BR3.3 | `next_decision` の優先順 (0) 定義 id → (1) park → (2) resume → (3) free_text → (4) completed → (5) in-flight ∧ SKIP → (6) in-flight → (7) next in-scope / Done。`EngineSignal::from` で 4 値へ導出。`jump_resolve` と `jump` の分離 |
| BR4.1 / BR4.2 | `plan_action.rs` を `workflow_definition/` へ移動、`orchestration/mod.rs` から `mod plan_action` / `pub use` を削除、呼出側 10 ファイル（現行の `use crate::orchestration::PlanAction` → `crate::workflow_definition::PlanAction`）を一斉修正。`WorkflowDefinition::effective_plan_action` / `next_in_scope_stage` と対応テストを削除（テストは集約側 / `grid().action()` 照会に書き換え）。合格 grep: `grep -rnE 'enum PlanAction\|pub use .*PlanAction' modules/core/domain/src/orchestration` = 0 |
| BR5.1 / BR5.2 / BR5.3 / BR5.4 | `StageIndex`（`usize` newtype、`Copy`、`Ord`、集約だけが構築 — `pub(crate)` コンストラクタ + `WorkflowExecution::stage_index`）。snapshot は全 16 属性、serde なし。`with_version` は値を置くだけ。エラーは手実装 + `std::error::Error` |
| NFR2.2 PBT | 既存 PBT（`quint_invariants_hold_under_random_command_sequences` / `stale_report_never_mutates`）を新 API に移植し、(a) decide = 旧 + apply、(b) replay == execute、(c) seq_nr 単調 / SequenceGap、(d) Quint 不変条件、(e) Err 無副作用、(f) `from_snapshot(snapshot()) == self` を追加。既定 256 ケース・コマンド列 ≤ 60・合成定義（stage_count 2〜8、initialization 1〜3） |
| NFR2.3 | Bolt 着手時に `cargo llvm-cov -p core-domain --summary-only` を 1 回取り、code-summary に基準値を記録（以後の下限） |

## 4. 棚卸し（code-generation で確定し code-summary に記録する事項）

- [ ] I1. ドメインクレート単独カバレッジの着手前基準値（`cargo llvm-cov -p core-domain --summary-only`）。
- [ ] I2. `WorkflowExecution` / `EngineSignal` / `Status` / `PlanAction` の外部利用箇所（実測: ドメイン外の利用は doc コメントのみ、
      `PlanAction` は 10 ファイル）— 実装後に再 grep して差分ゼロを確認。
- [ ] I3. upstream ピン `3c3146cf` の `dist/claude/.claude/tools/data/harness.json` の実バイト（`{ "name": "claude", "harnessDir": ".claude",
      "rulesSubdir": "rules" }`、HTTP 200 実測）をゴールデン dir に追加し README 表に行を足す（バイト不変規則は既存行に適用、追加は可）。
- [ ] I4. `DefinitionRevision` の入力順序と JSON 形（§1）— 同一入力で 2 回 load して一致、`scope-grid.json` を 1 文字変えて不一致、のテスト。
- [ ] I5. `IntentId` の既存有無（`workspace` コンテキストに `IntentSlug` 等があれば再利用し、無ければ新設）。
- [ ] I6. `orchestration/mod.rs` の公開面の最終形（`pub use` 列挙 = entities.md / logical-components の公開面と一致）。

## 5. 実装ステップ（TDD、レイヤーごとに Red → Green → Refactor）

Testing Contract の `plan_profile.steps` を基線とし、ライブラリに存在しない層（Frontend）は省く。「Data model」= Domain Primitive と
値オブジェクト・イベント・スナップショットの型、「Repository」= `WorkflowDefinitionRepository`（C4 改訂）、「Business logic」= 集約の
decide / apply / クエリ、「API」= ファサードと ITF 準拠・実グラフテスト。各 Red では失敗するコマンド出力（失敗テスト名と要約行）を
`code-summary.md` に記録してから Green に進む。

### 5.0 コンダクタ（承認後・委任前）

- [ ] Step 0. Bolt 開始とブランチ: `bun .claude/tools/aidlc-bolt.ts start --name B3 --batch 1` → `git switch -c bolt/b3-u2-domain-es-core origin/main`
      → aidlc 記録を 1 コミット（`chore(aidlc): record U2 design, ADR-008 and the B3 plan`）。基準値 I1 を取得。

### 5.1 workflow_definition 側（開発エージェント — 委任 1）

- [ ] Step 1. 骨格: `plan_action.rs` を `workflow_definition/` へ移動し `workflow_definition/mod.rs` の `pub use` に追加、`orchestration/mod.rs`
      から `mod plan_action` / `pub use plan_action::PlanAction` を削除、呼出側 10 ファイルの `use` を一斉修正。`cargo build --workspace`
      緑、合格 grep = 0。`WorkflowDefinition::effective_plan_action` / `next_in_scope_stage` と依存テストを削除（`grid().action()` /
      `stages_in_scope` への書き換え）。
- [ ] Step 2. テストランナー確認: `cargo test -p core-domain`（実測 126 + ITF 2）、`cargo test -p core-use-case`、
      `cargo test -p core-interface-adapter --test workflow_definition_repository_impl_test --test golden_parity_test` が走ることを確認し
      `unit-test-instructions.md` のコマンドを確定。
- [ ] Step 3. Data model — Red: `WorkflowDefinitionId`（非空・trim・`parse` 往復）、`DefinitionRevision`（`sha256:` + hex64 の形式検証、
      `Display`）、`WorkflowDefinition::new(id, revision, …)` + `id()` / `revision()` のテスト（各 5〜8 本）。失敗出力を記録。
- [ ] Step 4. Data model — Green: 最小実装（private フィールド + アクセサ、`PartialEq` / `Eq` / `Hash` / `Ord`、手実装 Display / Error）。
- [ ] Step 5. Data model — Refactor: rustdoc、`must_use`、ファサード列挙。
- [ ] Step 6. Repository — Red: `core-use-case` の trait を `find_by_id(&WorkflowDefinitionId)` に改訂（`find()` 削除）、
      `GraphReadError::NotFound { expected, actual }` / `HarnessIdentity { path, cause }` を追加。`core-interface-adapter` のテスト:
      (a) `find_by_id(id)` が harness.json の name と一致すれば id / revision 付きの定義を返す、(b) 不一致なら `NotFound`、
      (c) harness.json 欠落 → `HarnessIdentity`、(d) revision は同一入力で安定・入力変更で変わる（I4）、(e) `InMemory…` も同じ契約、
      (f) golden parity test が `find_by_id(WorkflowDefinitionId::parse("claude"))` で実グラフを読む。失敗出力を記録。
- [ ] Step 7. Repository — Green: impl に `load_harness_identity()`（`<data_dir>/harness.json` → `name`）と revision 計算（`canon_json::to_value`
      / `hash_canonical`、依存は既存）、`InMemory…` に id / revision の保持、ゴールデン dir に `harness.json`（I3）。
- [ ] Step 8. Repository — Refactor: 逐語文言の材料（`HarnessIdentity` / `NotFound` の Display は材料のみ）、rustdoc、既存テストの
      `find()` 呼出 13 箇所を `find_by_id` へ。品質ゲート（§5.4 Step 20 と同じ）を一度通してコミット。

### 5.2 orchestration 側 — Data model（開発エージェント — 委任 2）

- [ ] Step 9. Data model — Red: `StageIndex`（範囲保証、`Ord`）、`StageEntry`、`IntentId`（I5）、`WorkflowExecutionEvent`（封筒 + 12 変種の
      構築・アクセサ・`PartialEq`）、`WorkflowExecutionSnapshot`（16 属性）、`NextRequest` / `NextDecision` / `EngineSignal::from`、
      エラー 4 型の `Display` / `Error`（各 5〜8 本）。失敗出力を記録。
- [ ] Step 10. Data model — Green: 最小実装（private + アクセサ、手実装エラー）。`Status` を `status.rs` に切り出し。
- [ ] Step 11. Data model — Refactor: rustdoc、ファサード `pub use` 列挙の更新（旧 API 名は残さない）。

### 5.3 orchestration 側 — Business logic（委任 2 続き）

- [ ] Step 12. Business logic — Red: `start`（W1: 正常 / UnknownScope / InitializationMustExecute / Unconditional、Started の内容、
      definition_id / revision の記録）、12 コマンドのガードと遷移（現行ユニットテスト 9 本を新 API へ移植 + 新規: complete_stage の
      initialization 限定、approve_gate の open 省略経路、reject の revision_count、jump の stages_reset / stages_skipped、recompose 複数件、
      unpark）、`apply_event`（SequenceGap / UnknownStage / 不変条件）、`from_snapshot` の各不変条件、`next_decision`（W4 の優先順 8 分岐 +
      DefinitionMismatch + revision 差で Ok）、`jump_resolve` / `stale_report`。実グラフ索引テスト（initialization 3 ステージの合成 StageEntry
      列で索引 0〜2 非ゲート / 3 ゲート / jump(1) = InvalidTarget）。失敗出力を記録。
- [ ] Step 13. Business logic — Green: 集約本体（decide / apply / クエリ）。現行 FSM のガード集合は維持（`// amadeus-lint: allow(checkbox-vocabulary)`
      + 不変条件番号）。
- [ ] Step 14. Business logic — Refactor: apply の一時コピー方式の整理、重複ガードの関数化、rustdoc。テスト緑のまま。
- [ ] Step 15. PBT（`workflow_execution.rs` 同居、`PROPTEST_RNG_SEED` 固定）: 性質 (a)〜(f)（§3 NFR2.2）。

### 5.4 orchestration 側 — API（ファサード・ITF・品質ゲート）

- [ ] Step 16. API — Red: `engine_loop_conformance.rs` を新 API に書き換え（BR2.5 の対応表）— 8 fixture 全緑になるまで Red。
      `orchestration/mod.rs` の公開面がロジカル設計の列挙と一致することのテスト（`pub use` 行の読取 — canon-json と同じ方式）。
- [ ] Step 17. API — Green: 不足のアクセサ / 変換（`EngineSignal::from`）、ITF 用 `start_with_entries`。
- [ ] Step 18. API — Refactor: クレート rustdoc（`//!`）に ES 形・イベント 12 変種・射影表・gated = phase の説明、BR2.5 の注記。
- [ ] Step 19. 棚卸し I2 / I6 と `cargo llvm-cov -p core-domain --summary-only`（I1 の基準値と比較）。
- [ ] Step 20. 品質ゲート: `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` →
      `cargo test --workspace` → `bash scripts/coverage.sh`（絶対床）→ 合格 grep（BR4.1）= 0。コミットは意味単位
      （`feat(core-domain): …` / `refactor(workflow-definition): …` / `test(itf): …`）。

## 6. トレーサビリティ（要求 → ステップ）

| 要求 / 規則 | ステップ | 主な成果物 |
|---|---|---|
| FR8.3 PlanAction 完全移動 | 1, 19, 20 | `workflow_definition/plan_action.rs`、両 mod.rs、呼出側 10 ファイル |
| FR8.4 畳み込み移設 | 1, 12〜14 | `workflow_definition.rs`（削除）、`workflow_execution.rs`（effective_plan） |
| FR2.1 / FR3.1 / FR3.3 の土台（decide / next_decision） | 9〜18 | `orchestration/*.rs` |
| FR1.3 の集約側（snapshot / replay） | 9, 12〜15 | `workflow_execution_snapshot.rs`、`apply_event` |
| BR1.0〜BR1.9 / BR3.1〜BR3.3 | 12〜14 | `workflow_execution.rs` |
| BR2.1〜BR2.4 | 9, 12〜15 | `workflow_execution_event.rs`、`apply_event`、PBT |
| BR2.5 | 16, 17 | `tests/engine_loop_conformance.rs` |
| BR2.6 / ADR-008 / C4 | 3〜8, 12 | `workflow_definition_id.rs`、`definition_revision.rs`、`workflow_definition_repository.rs`（trait）、`workflow_definition_repository_impl.rs`、`memory/workflow_definition_repository.rs`、`tests/golden/upstream-3c3146cf/harness.json` |
| BR4.1 / BR4.2 | 1 | 同上 |
| BR5.1〜BR5.4 | 9〜11 | `stage_index.rs`、`workflow_execution_snapshot.rs`、エラー 4 型 |
| NFR1.1 / NFR1.2 / NFR1.3 | 12, 16, 17 | ITF、実グラフ索引テスト、網羅 match |
| NFR2.1〜NFR2.4 | 全 Red/Green/Refactor、15, 19, 20 | Red 記録、PBT、カバレッジ基準値、品質ゲート |
| NFR3.1〜NFR3.4 | 12〜15 | apply / from_snapshot / next_decision / snapshot |
| NFR4.1〜NFR4.5 | 1, 4, 7, 20 | 依存不変（core-domain）、StageIndex、素通し、serde なし |

## 7. 委任の形

- 委任 1（Step 1〜8: workflow_definition / Repository 側）と委任 2（Step 9〜20: orchestration 側）を同じ承認済み計画・同じ指紋の下で
  **直列に** aidlc-developer-agent へ委任する。各委任の冒頭行は `AIDLC-UNIT: u2-domain-es-core` と `AIDLC-TESTING-CONTRACT: <contract_sha256>`。
  委任 1 の終わりでワークスペースが緑（ビルド・テスト・lint）であること。
- 開発エージェントは計画のチェックボックスを更新しない（計画バイトは承認後凍結 — 進捗はエージェントの報告ファイル
  `developer-report-<n>.md` に書き、コンダクタが `code-summary.md` に統合する）。
- 失敗時はコンダクタが halt-and-ask（retry / skip / abort）を出す。規模 L: 委任 1 が 1 日相当を超えそうならオーナーと分割を相談。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "新規プロダクションコードはレイヤーごとに red-green-refactor",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 新規プロダクションコードはレイヤーごとに red-green-refactor\n  （失敗するテストを先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・\n  ゴールデンパリティは TDD サイクルの外側の受け入れゲートとして維持し、\n  TDD の red を代替しない。（インタビュー Q2、選択肢 A で確定——品質レビュー\n  の自己完結化置換案どおり）\n\nテストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識した配分とする\n（オーナー明言）。比率は**定性のみ**とし、数値目標は定めない（インタビュー\nQ3、選択肢 A）: 単体テスト優位・統合テストは境界ごと・E2E は最小、という\n配置規則で充足する。\n\nこのプロジェクトは TDD の上に **3層の品質保証** を重ねている点が特徴的で、\nそれぞれ役割が異なる（`code-quality-assessment.md` §品質保証の全体像より）:\n\n1. **Quint 形式検証**（毎 PR）— 決定論コアの状態機械契約そのものを検証。\n   不変条件 run 27本・到達性 witness 12本の反転判定・決定的シナリオ。\n   モデルの検査力自体も mutation テストで証明済み（engine_loop 3/3、\n   audit_lock 10/10 + witness 7/7、stop_hook 7/7）。\n2. **ITF 準拠テスト**（`modules/core/domain/tests/`、engine_loop / audit_lock\n   の2モデル・2ファイル）— Quint モデルのトレースを集約に再生し状態射影を\n   突き合わせることで、モデルと実装の乖離を検出。TDD の「テストを先に書く」\n   対象は実装コードだが、契約の正本は Quint 側にあるため、ITF 準拠テストは\n   実装後に契約適合を機械確認する位置づけ（TDD サイクルの red-green-refactor\n   そのものではなく、その外側のゲート）。なお stop_hook は ITF 準拠テストが\n   未整備（既知の穴、`evidence.md` インタビュー未確定事項 (e) 参照）。\n3. **PBT（proptest）+ ゴールデンパリティ**— upstream 配布実バイト33ノードの\n   全数 load パリティを固定し、upstream 互換の逸脱を検出。\n\nしたがって TDD サイクルは主にユニットテスト層（インライン `#[cfg(test)]`、\n実測**40ファイル**——集計方法: `modules/` 配下・`tests/` ディレクトリを除いた\nインライン `#[cfg(test)]` 数。`tests/` 配下6本（ITF準拠2 + 統合4）を含めると\n46、`tools/lint/src/check.rs` を含めても47であり、いずれの集計でも48には\nならない。開発者レビュー指摘どおり40へ訂正した）に適用し、ITF 準拠テスト・\nゴールデンパリティはレイヤー横断の受け入れ確認として TDD サイクルの外側に\n位置づける。\n\n- **カバレッジ**: 絶対ゲート90%床 + PR 相対ゲート（head が base を下回ったら\n  fail、許容誤差 0.5pp。PBT のシード非固定に起因するノイズ較正値であり、\n  stage-1 スコープで**シード固定により 0.01 へ引き締める**——インタビュー\n  Q7、選択肢 A/B。除外設定は現状無いが、**composition root（`main.rs` の\n  配線部分）のみカバレッジ除外を許可**し、それ以外は床を維持する\n  （インタビュー Q5、選択肢 B。除外設定は `scripts/coverage.sh` への確定\n  アクション、`evidence.md` 参照）。実測 94.87〜95.29%（`scripts/coverage.sh`）。\n- **ツーリング**: `cargo test --workspace`（234テスト全緑、実測）、\n  `cargo-llvm-cov`、Quint 0.32.0（Node 22 経由）。\n- **テスト種別**: ユニット（インライン `#[cfg(test)]`）、PBT（proptest、集約\n  本体同居）、ITF 準拠（`modules/core/domain/tests/` 2本）、統合（\n  `modules/core/interface-adapter/tests/` 4本 — ゴールデンパリティ・FS ロック・\n  Repository 実装・シンボリックリンク防御）。\n- **CI ゲート**（`main` へのマージ条件、実測）: `check` ジョブ（`cargo fmt\n  --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →\n  `cargo lint` → `cargo test --workspace`）、`quint` ジョブ\n  （`scripts/quint-gate.sh`）、`coverage` ジョブ（`scripts/coverage.sh`、\n  絶対90%床 + PR 相対ゲート）の3ジョブすべてを緑にする。この3ジョブは\n  **stage-1 スコープで branch protection の required status checks として\n  機械強制する**（インタビュー Q4、選択肢 A——現状は運用規律のみで機械強制が\n  無いという品質レビューの重大指摘を受けての裁定。設定作業は\n  `evidence.md` の確定アクションに記載）。\n- **スコープ注記**: `tools/lint`（`cargo lint` の実装クレート）は workspace\n  非メンバーの detached クレートであり、CI の fmt/clippy/test がまだ届いて\n  いない（設計監査 C27）。**stage-1 スコープに含める**: `tools/lint` への\n  CI 3ステップ（fmt/clippy/自己テスト）追加（インタビュー Q7、選択肢 A）。\n  macOS CI ジョブ追加・`main` への push トリガー追加は本 intent には\n  含めず、後続 intent へ繰り延べる（インタビュー Q7、選択肢 E 相当の一部\n  不採択）。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:e4f36aa113753d3604df570f5ec3a0cb465d4b29d82a17a16efbb2ea8b993111",
  "contract_sha256": "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
}
```
