# developer-report-2 — U2 ドメイン ES コア（Bolt B3 / 委任 2: orchestration 側の ES 化 = 計画 Step 9〜20）

> 開発エージェント（aidlc-developer-agent）の作業記録。計画ファイル（`code-generation-plan.md` /
> `unit-test-instructions.md` / `code-generation-questions.md`）は読み取り専用として一切編集していない。
> Testing Contract: `sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`（tdd /
> standard / classic / brownfield）。ブランチ `bolt/b3-u2-domain-es-core`、push / PR は行っていない。

## 1. 結論

計画 Step 9〜20 を完了した。`WorkflowExecution` をイベントソーシング形 FSM に全面改訂し、12 イベント・
snapshot / from_snapshot・`StageIndex` / `StageEntry` / `IntentId`・`NextDecision`・エラー 4 型を新設、
ITF 準拠テストを新 API へ書き換えた。旧 API（`report_forward` / `gate_start` / `reject` / `revise` /
`report_skipped` / `recompose_flip` / `next`）は後方互換を残さず削除した。

品質ゲートは 5 つすべて緑（§7）。カバレッジは `core-domain` 単独 **lines 96.53%**（着手前基準 94.70% を
+1.83pp 上回る）、ワークスペース **97.38%**（絶対床 90%）。

## 2. 変更したファイル

| ファイル | 種別 | 内容 |
|---|---|---|
| `modules/core/domain/src/orchestration/intent_id.rs` | 新規 | `IntentId` / `IntentIdError`（kebab 表記の Always Valid な集約識別子） |
| `modules/core/domain/src/orchestration/stage_index.rs` | 新規 | `StageIndex`（集約だけが構築する E1 型、`pub(crate)` コンストラクタ） |
| `modules/core/domain/src/orchestration/stage_entry.rs` | 新規 | `StageEntry`（slug / phase / plan_action / conditional、`is_gated()` を所有） |
| `modules/core/domain/src/orchestration/phase_boundary.rs` | 新規 | `PhaseBoundary`（`GateApproved` の投影材料） |
| `modules/core/domain/src/orchestration/status.rs` | 新規（切り出し） | `Status`（`workflow_execution.rs` のインライン定義から private mod へ） |
| `modules/core/domain/src/orchestration/next_decision.rs` | 新規 | `NextRequest` / `NextDecision`（8 値）/ `EngineSignal` と `From<&NextDecision>` |
| `modules/core/domain/src/orchestration/workflow_execution_event.rs` | 新規 | 封筒 + 12 変種のペイロード型（C5 の形） |
| `modules/core/domain/src/orchestration/workflow_execution_snapshot.rs` | 新規 | `WorkflowExecutionSnapshot`（16 属性）+ ビルダー |
| `modules/core/domain/src/orchestration/start_error.rs` | 新規 | `StartError`（4 変種、`UnknownScope` は定義側の材料型を運ぶ） |
| `modules/core/domain/src/orchestration/command_error.rs` | 新規 | `CommandError`（7 変種、`DefinitionMismatch` を含む） |
| `modules/core/domain/src/orchestration/apply_error.rs` | 新規 | `ApplyError`（`SequenceGap` / `UnknownStage` / `InvariantViolation`） |
| `modules/core/domain/src/orchestration/snapshot_error.rs` | 新規 | `SnapshotError::InvariantViolation` |
| `modules/core/domain/src/orchestration/workflow_execution.rs` | 全面改訂 | 集約本体（12 コマンド・`apply_event`・クエリ・snapshot・PBT 4 本） |
| `modules/core/domain/src/orchestration/mod.rs` | 改訂 | 公開面の `pub use` 列挙、コンテキスト rustdoc（射影表・ゲート判定・イベント表） |
| `modules/core/domain/src/lib.rs` | 追記 | クレート rustdoc に ES 形と 3 コンテキストの役割を追記 |
| `modules/core/domain/tests/engine_loop_conformance.rs` | 書き換え | 新 API（`start_with_entries` → decide → apply）でトレース再生 |

`skeleton_stance.rs` / `verdict.rs` / `autonomy_mode.rs` / `jump_direction.rs` は変更なし。`core-domain` の
`Cargo.toml` は不変（依存追加なし）。use-case / interface-adapter / app / harness には一切触れていない。

## 3. TDD の記録（各 Red の失敗出力）

Rust の静的型付けでは、まだ存在しない型に対するテストの Red はコンパイルエラーとして現れる。各 Red は
テストモジュールだけを書いた状態で `cargo test -p core-domain --lib` を実行し、失敗出力を記録してから
Green に進んだ。

### Red A — Data model（leaf 型: `IntentId` / `StageIndex` / `StageEntry` / `PhaseBoundary` / `Status` / エラー 4 型）

```
error[E0432]: unresolved import `crate::orchestration::StageIndex`
 --> modules/core/domain/src/orchestration/command_error.rs:6:9
error[E0433]: cannot find type `CommandError` in this scope
  --> modules/core/domain/src/orchestration/command_error.rs:12:20
error[E0425]: cannot find type `IntentId` in this scope
  --> modules/core/domain/src/orchestration/intent_id.rs:69:29
error[E0425]: cannot find type `StageEntry` in this scope
 --> modules/core/domain/src/orchestration/stage_entry.rs:8:72
error[E0425]: cannot find type `StageIndex` in this scope
  --> modules/core/domain/src/orchestration/stage_index.rs:35:27
error[E0433]: cannot find type `StartError` in this scope
  --> modules/core/domain/src/orchestration/start_error.rs:10:19
...
error: could not compile `core-domain` (lib test) due to 91 previous errors; 9 warnings emitted
```

**Green A**: `test result: ok. 169 passed; 0 failed`（着手前 141 → +28）。

### Red B — Data model（`WorkflowExecutionEvent` / `WorkflowExecutionSnapshot` / `NextDecision`）

```
   |                 ^^^^^^^^^^^^ use of undeclared type `NextDecision`
Some errors have detailed explanations: E0425, E0433.
error: could not compile `core-domain` (lib test) due to 78 previous errors; 3 warnings emitted
```

**Green B**: `test result: ok. 187 passed; 0 failed`（+18）。

### Red C — Business logic（集約本体）

```
error: could not compile `core-domain` (lib test) due to 16 previous errors; 2 warnings emitted
```
（`WorkflowExecution` / `start_with_entries` / `complete_stage` 等がいずれも未定義。集約ファイルは
Red C の時点で doc コメントのみのプレースホルダにしてあった。）

**Green C**: `test result: ok. 233 passed; 0 failed`（+46）。

### Red D — PBT（性質 (a)〜(f) + 移設 2 性質）

PBT の Red は「性質を書いた時点で走らない」= コンパイルエラーで観測した（`prop_assert!` の
フォーマット文字列展開）:

```
2415 |                 prop_assert!(matches!(gap, Err(ApplyError::SequenceGap { .. })));
     |                 ^^^^ expected `}` in format string
error: could not compile `core-domain` (lib test) due to 1 previous error
```

**Green D**: `test result: ok. 237 passed; 0 failed`。4 本の性質テストはいずれも初回実行で緑
（既存実装のバグを検出しなかった）。**注記**: これは「PBT が弱い」ことを意味しうるため、性質の実効性は
§5 の「PBT が実際に踏んでいる経路」で確認した。

### Red E — API（ITF 準拠テスト）

Green C 直後の `cargo clippy --workspace --all-targets` で、ライブラリは緑・ITF 準拠テストだけが赤という
状態を確認した（これが API 層の Red）:

```
error: could not compile `core-domain` (test "engine_loop_conformance") due to 26 previous errors
error[E0599]: no method named `report_forward` found for tuple `(WorkflowExecution, WorkflowExecutionEvent)`
error[E0599]: no method named `gate_start` found for tuple ...
error[E0599]: no method named `recompose_flip` found for tuple ...
error[E0277]: the trait bound `i64: TryFrom<StageIndex>` is not satisfied
error[E0061]: this function takes 5 arguments but 2 arguments were supplied
（他 10 件の E0308 mismatched types）
```

**Green E**: `test result: ok. 1 passed; 0 failed`（8 fixture 全数 + アクション網羅 16 本）。

### Refactor

- Step 11 / 14 / 18: `cargo clippy -p core-domain --all-targets` の指摘（`missing_const_for_fn` 5 件・
  `useless_vec` 3 件）を解消し、`cargo fmt --all` を適用。rustdoc をクレート（`lib.rs`）とコンテキスト
  （`orchestration/mod.rs`）に追記（イベント対応表・射影表・ゲート判定の説明）。テストは緑のまま。
- 追加の Red/Green（コミット `1d035f5`）: カバレッジ実測で `next_decision` の防御腕 2 本が未到達と
  判明したため、到達経路を特定してテストを追加した（§5）。

## 4. 設計との差分（判断）と設計質問

### D1. スナップショットの `stages` は `StageEntry` 列にした（設計の内部矛盾の解消）

`entities.md` の `WorkflowExecution` 属性表は `stages: list<StageSlug>` としつつ、BR1.3 は
`gated(s) = phase(s) ≠ initialization` を「`Started` の `StageEntry.phase` から」判定すると定める。前者の
ままでは phase が再水和で失われ、`from_snapshot` した集約がゲート判定できない（**実装不能**）。

**採った最小の解釈**: 集約とスナップショットの `stages` を `Vec<StageEntry>`（slug + phase + plan_action +
conditional）にし、16 属性の `plan` / `conditional` は独立した列として保持したうえで、`from_snapshot` が
`plan[i] == stages[i].plan_action()` / `conditional[i] == stages[i].is_conditional()` の整合を検査する
（重複を二重状態にせず検査済みの冗長にする）。属性数は 16 のまま。

### D2. `Started` の `depth` / `test_strategy` は載せていない

`entities.md` の payloads は `depth?, test_strategy?` を任意フィールドとして挙げるが、計画 §2 / BR2.2 の
`Started` の定義は `{definition_id, definition_revision, scope, request, stages}` である。集約はこの 2 つに
対応する状態を持たず、載せても素通しにしかならないため、計画 §2 の定義に従った。

**設計質問 Q-A**: `Started.depth` / `test_strategy` は U4（投影）が状態ファイルの
`Scope Configuration` を描くのに要るか。要るなら U3 のワイヤ構造体で持つのか、集約のイベントに載せるのか。

> **裁定済み（2026-08-23）→ 載せる。§12 で実装済み**。本 D2 の「載せていない」は追加作業前の記述であり、
> 現在の実装は `StartRequest` 経由で `Started` に `depth` / `test_strategy` を載せる。

### D3. `IntentId` は `-<id8>` の 16 進サフィックスを検査していない（実データとの不一致）

`entities.md` は `IntentId` を「`<kebab-slug>-<id8>`（id8 = 16 進 8 桁）。構築時に形を検証」と定める。
しかし本ワークスペースの実データは `aidlc/spaces/default/intents/intents.json` の
`dirName = "260822-stage1-selfhost"`（`slug = "stage1-selfhost"`、`uuid = 01a02785-…`）であり、
`-<id8>` サフィックスを持たない（`<YYMMDD>-<slug>` の形）。設計どおりに検査すると本 intent 自身の
識別子が構築できず、U5 / U7 で実行時に破綻する。

**採った最小の解釈**: kebab 表記（`[a-z0-9]` の 1 文字以上の区間を `-` で連結、前後空白は trim）だけを
検査する。`<kebab-slug>-<id8>` はこの形の部分集合なので、設計が意図する値はすべて受理される。

**設計質問 Q-B**: `IntentId` の正本の形はどちらか。(a) 実データどおり `<YYMMDD>-<slug>` を含む一般の
kebab（現実装）、(b) `entities.md` どおり `-<id8>` 必須（この場合は記録ディレクトリの命名規約の是正が
先に要る）。

> **裁定済み（2026-08-23）→ (a) 現実装どおり**。コード変更なし。`entities.md` の記述訂正は設計側の作業。

### D4. `EngineSignal::from` は `UnparkThenResume` / `ResumeMenu` / `NewWorkRouting` を `Done` へ畳む

BR3.1 の導出規則は 5 分岐（RunStage / Done / Parked / 2 つの不整合）しか定めていない。残る 3 分岐は
Quint の `DirectiveKind`（4 値）に対応語を持たない。「ステージを走らせない・park でもエラーでもない停止」
として `Done` に畳んだ（`next_decision.rs` の rustdoc とテスト
`the_decisions_outside_the_model_vocabulary_stop_the_loop` に明記）。ITF 準拠テストはこの 3 分岐を踏まない
（モデルに resume / 自由記述が無い）ため、射影の突合せには影響しない。

### D5. `start` の「先頭ステージは EXECUTE」を独立のガードにした

BR2.2 は「`stages[0]` は EXECUTE かつ非 conditional でなければ Err」と書き、BR1.3 はゲート判定を
フェーズに移した。両者を素直に合成すると「initialization フェーズの**すべて**のステージが EXECUTE かつ
非 CONDITIONAL」＋「索引 0 は（フェーズによらず）EXECUTE」の 2 条件になる。後者が無いと
`cursor_in_scope`（初期カーソル = 索引 0）が破れるため、独立のガードとして実装し、いずれも
`StartError::InitializationMustExecute` を返す。

### D6. `apply_event(Started)` は genesis 専用（`InvariantViolation`）

集約の genesis は `start` / `start_with_entries` が担うため、既存集約への `Started` 適用は
`ApplyError::InvariantViolation("Started applies only at genesis")` で拒否する。BR2.3 が定めるリプレイは
「`from_snapshot(S)` + seq_nr 以降の apply」なので、イベント列だけからの genesis 再構成は U2 の契約外である
（PBT (b) も `from_snapshot` 起点で検証している）。U3 が seq_nr = 1 からの再構成を要するなら、
`from_started` 相当の入口を U3 の設計で足すのが自然。

### D7. `commit` の到達不能な `Err` 腕

decide はガードを全部通してからイベントを構築し `apply_event` に渡すため、封筒・slug・不変条件のいずれも
破れない。しかし型としては `Result` なので、`unwrap` / `expect` / `panic!` を使わずに扱う必要がある
（NFR4.3: `# Panics` 0 件）。到達不能な `Err` は状態を変えないまま `InvalidTarget(cursor)` として返し、
理由をコメントに明記した。この腕はカバレッジ上も未到達である（§6）。

### D8. `WorkflowExecutionSnapshotBuilder` を公開面に追加

スナップショットは 16 属性あり、単一の `new(...)` は `clippy::too_many_arguments`（既定 7）に抵触する。
`workflow_definition` の `StageNodeBuilder` と同じ house style でビルダーを用意し、既定値は解決済み計画から
導ける birth 時の状態にした。スナップショットのフィールドは `pub(crate)`（同一クレート内の再水和のための
実装詳細共有 — `field-visibility.md` が許す範囲）で、クレート外へは 16 本のアクセサだけを公開する。

### D9. 公開面への追加 3 型

`logical-components` §1 の列挙に対する追加は 3 つ: `WorkflowExecutionEventPayload`（12 変種の enum 本体 —
網羅 match に必須）、`WorkflowExecutionSnapshotBuilder`（D8）、`IntentIdError`（`IntentId::parse` の材料）。
いずれも列挙された型を使うために不可欠なもので、利便再エクスポートではない。

## 5. PBT が実際に踏んでいる経路（性質の実効性）

4 本の性質テストが初回で緑になったため、生成器が意味のある経路を踏んでいるかを確認した。

- 生成器は合成計画（stage_count 2〜8、initialization 1〜3 ステージ、残りは inception。initialization は
  常に EXECUTE・非 CONDITIONAL）と 13 種のコマンド列（1〜59 個）。`PROPTEST_RNG_SEED=20260823` 固定、
  既定 256 ケース。
- `drive()` が `Err` を受けたときは「発火しないアクション」として**その場で状態不変をアサート**する
  （性質 (e) は全ケース・全ステップで検査されている）。
- 性質 (a) は `Ok` のたびに「旧状態 + `apply_event`」と実状態の一致を検査し、同じループで Quint 不変条件
  （cursor_in_scope / no_gate_bypass / at_most_one_active / parked_position）と `from_snapshot(snapshot())`
  の往復（性質 (d)(f)）も毎ステップ検査する。
- 性質 (b)(c) は genesis スナップショット + 全イベント列の再生が実行結果と一致すること、`seq_nr` が
  1 イベントにつきちょうど 1 増えること、順序違反が `SequenceGap` で拒否され状態も動かないことを検査する。
- 移設した 2 性質: 実効プランが「グリッド + recompose サフィックス」であり静的 `plan` が動かないこと、
  `next_decision` の先読み先がカーソルより後ろで**最初**の in-scope ステージであること（`Done` のときは
  後続に in-scope が無いこと）。

実際にこの PBT はカバレッジで裏が取れている: `workflow_execution.rs` の未到達行は 31 行のみで、その大半は
到達不能な防御腕（§6）である。

## 6. カバレッジの実測と未到達行

| 対象 | 着手前 | 完了時 |
|---|---|---|
| `cargo llvm-cov -p core-domain --summary-only`（lines） | **94.70%**（ブリーフ記載の I1 基準値。委任 1 完了時点の実測は 95.02%） | **96.53%**（regions 97.05% / functions 95.39%） |
| `bash scripts/coverage.sh`（ワークスペース、絶対床 90%） | — | **97.38%** [PASS] |

計測は `cargo llvm-cov clean --workspace` 後・`PROPTEST_RNG_SEED=20260823` 固定で行った（clean を挟まないと
削除済みの `orchestration/plan_action.rs` の古いプロファイルが 0% で混入し、値が 91.5% 前後に見える）。

新規モジュール別 lines: `apply_error` / `command_error` / `intent_id` / `phase_boundary` / `snapshot_error` /
`stage_entry` / `stage_index` / `start_error` / `status` / `workflow_execution_snapshot` = 100.00%、
`workflow_execution` = 98.16%、`workflow_execution_event` = 96.93%、`next_decision` = 94.00%。

**未到達行の内訳**（いずれも意図的に残した防御腕、またはテストコード）:

- `workflow_execution.rs:439` — `commit` の到達不能な `Err` 腕（D7）。
- `workflow_execution.rs:602/608/895` — `checkbox()` が `None` を返したときの `continue`。`StageIndex` は
  集約が範囲内でのみ構築するため到達しない（`unwrap` を書かないための分岐）。
- `workflow_execution.rs:695` — `recompose` で `effective_plan` が `None` の腕（同上）。
- `next_decision.rs` / `workflow_execution_event.rs` の未到達 6 / 10 行 — 網羅 match をコンパイル時に固定する
  テストヘルパ（`name()` 関数）の腕。関数自体は呼ばれるが全腕は踏まない。

初回のカバレッジ実測で `next_decision` の `RecoverSkipInconsistency` / `InconsistentSkip` の 2 腕が未到達と
判明した。これは「実効 SKIP のカーソル」が `cursor_in_scope` により通常のコマンド列では作れないためで、
到達可能な唯一の経路（park 中の状態を再水和し、再入フラグで park 分岐を外す）を特定してテストを追加した
（コミット `1d035f5`）。設計の防御腕が本当に到達可能であることをここで確認できたのは収穫である。

## 7. 品質ゲートの結果

すべて緑（2026-08-23 実測、最終コミット `1d035f5` 時点）。

| ゲート | 結果 |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0（warning 0 件） |
| `cargo lint` | exit 0 |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | **464 passed, 0 failed**（着手前 368） |
| `bash scripts/coverage.sh` | head 97.38% ≥ 90.0% [PASS] |
| `cargo llvm-cov -p core-domain --summary-only` | lines 96.53% ≥ 94.70%（着手前基準） |

**合格 grep**

- BR4.1 / FR8.3: `grep -rnE 'enum PlanAction|pub use .*PlanAction' modules/core/domain/src/orchestration`
  → **0 件**
- 旧 API（`report_forward` / `gate_start` / `recompose_flip` / `report_skipped`）: Rust の識別子としては
  **0 件**。残る 5 件は ITF 準拠テスト内の Quint アクション名（文字列リテラルと網羅アサートの配列）で、
  モデル側の語彙なので正当。

## 8. 棚卸し I2 / I6

### I2 — `WorkflowExecution` / `EngineSignal` / `Status` / `PlanAction` の外部利用箇所

- `WorkflowExecution` / `EngineSignal` の**ドメイン外での利用は doc コメントのみ**（`core-use-case` の
  `workspace/mod.rs`、`core-interface-adapter` の `workspace/mod.rs` / `state_file_io.rs`、
  `tests/workflow_definition_repository_impl_test.rs` のコメント 1 行）。実コードの利用はゼロで、委任 1 の
  実測と**差分ゼロ**。集約の全面改訂はドメイン外へ波及していない（use-case / interface-adapter / app /
  harness のファイルは 1 つも変更していない）。
- `PlanAction` を含むファイルは 10 → **13 ファイル**。増分 3 は本委任の新規ファイル
  （`orchestration/stage_entry.rs` / `workflow_execution_event.rs` / `workflow_execution_snapshot.rs`）で、
  いずれも `crate::workflow_definition::PlanAction` を**利用**しているだけであり、定義も再輸出も無い
  （合格 grep = 0 件がそれを機械的に保証する）。

### I6 — `orchestration/mod.rs` の公開面の最終形

```
// Domain Primitive
AutonomyMode, IntentId, JumpDirection, PhaseBoundary, SkeletonStance, StageEntry, StageIndex, Verdict
// 集約
WorkflowExecution
// 集約の観測結果
EngineSignal, NextDecision, NextRequest, Status, WorkflowExecutionSnapshot
// ドメインイベント (C5 の語彙 — 12 変種)
WorkflowExecutionEvent, WorkflowExecutionEventPayload,
Started, StageCompleted, GateOpened, GateApproved, GateRejected, StageRevised, StageSkipped,
Jumped, Parked, Recomposed, AutonomyModeSet        // Unparked は C5 が payload: {} なので単位変種
// ビルダー
WorkflowExecutionSnapshotBuilder
// 純関数ドメインサービス
parse_mode_arg
// エラー
ApplyError, CommandError, IntentIdError, InvalidModeArg, SnapshotError, StartError,
UnknownStance, UnknownVerdict
// 逐語定数
ACCEPTED_RESULTS
```

`logical-components` §1 の列挙との照合: 列挙された型はすべて存在し、`PlanAction` は再輸出していない。
追加は `WorkflowExecutionEventPayload` / `WorkflowExecutionSnapshotBuilder` / `IntentIdError` の 3 型（§4 D9）。
旧 API（`report_forward` / `gate_start` / `reject` / `revise` / `report_skipped` / `recompose_flip` / `next`）は
公開面から消えている。

## 9. 受入基準 12 項目の自己照合

| # | 内容 | 結果 |
|---|---|---|
| 1 | 公開面が logical-components §1 と一致、旧 API 削除、`PlanAction` 再輸出なし | OK（§8 I6。追加 3 型は §4 D9 に記録） |
| 2 | 12 コマンドが 1 コマンド 1 イベント、Err は状態不変、`occurred_at` を封筒に載せる、apply が同一経路 | OK（PBT (a)(e) が全ケースで固定） |
| 3 | `gated = phase ≠ Initialization`、`complete_stage` は非ゲートのみ / `approve_gate` はゲートのみ | OK（`every_initialization_stage_is_non_gated_and_the_rest_are_gated`） |
| 4 | `next_decision` の (0) `DefinitionMismatch` + BR3.1 (1)〜(7)、`EngineSignal::from`、`jump_resolve` / `stale_report` はクエリ | OK（`next_decision_walks_the_branches_in_priority_order` ほか。D4 を記録） |
| 5 | `snapshot()` 16 属性、`from_snapshot` の不変条件検査、`with_version` | OK（D1 を記録） |
| 6 | `apply_event` の `SequenceGap` / `UnknownStage` / `InvariantViolation`、Err で状態不変 | OK（一時コピー方式） |
| 7 | `StageIndex` は集約だけが構築、内部添字はすべて `StageIndex` 経由、`# Panics` 0 件、`unwrap`/`expect`/`panic!` なし | OK（`orchestration/` のプロダクトコードに `unwrap` / `expect` / `panic!` / `todo!` /
`unimplemented!` / `# Panics` 節はいずれも 0 件 — 実測） |
| 8 | PBT 6 性質 + 委任 1 が削除した 2 性質の等価物 | OK（4 本のテストに集約。§5） |
| 9 | ITF 準拠 8 fixture 全緑 + アクション網羅アサート | OK（1 test / 8 fixture / 16 アクション） |
| 10 | 実グラフ索引テスト（initialization 3 ステージ、索引 0〜2 非ゲート / 3 ゲート / `jump(1)` = `InvalidTarget`） | OK |
| 11 | 品質ゲート緑、合格 grep 0 件 | OK（§7） |
| 12 | 報告ファイル | 本ファイル |

## 10. コミット一覧（ブランチ `bolt/b3-u2-domain-es-core`）

| SHA | メッセージ |
|---|---|
| `55e9384` | `feat(core-domain): event-sourced WorkflowExecution — events, snapshot, StageIndex` |
| `ded4c0d` | `feat(core-domain): decide/apply commands on WorkflowExecution` |
| `f4910dc` | `test(itf): replay engine_loop traces through the event-sourced aggregate` |
| `1d035f5` | `test(core-domain): cover the defensive branches of next_decision and from_snapshot` |

`aidlc/` 配下はコミットしていない（記録コミットはコンダクタの担当）。`git add -A` は使わず、ファイルを
明示して staging した。push / PR は行っていない。

コミットの分割は**意味単位**（データモデル / 集約本体 / ITF / 防御腕のテスト）で行った。集約の全面改訂は
本質的に不可分なので、`55e9384` 単独ではライブラリは緑だが ITF 準拠テストがコンパイルできない（`f4910dc`
で解消）。最終状態は全ゲート緑である。

## 11. 後続への申し送り

1. **設計質問 2 件**（§4 Q-A: `Started.depth` / `test_strategy` の要否、Q-B: `IntentId` の正本の形）は
   オーナー裁定が要る。とくに Q-B は記録ディレクトリの命名規約に波及する。
2. **U3 への引き渡し**: genesis からのイベント列リプレイ（`Started` を seq_nr = 1 から適用する経路）は
   U2 の契約外にした（§4 D6）。U3 が最新スナップショット + 差分 replay で足りるなら追加は不要。
3. **`WorkflowExecutionSnapshot` のフィールドは `pub(crate)`**。U3 は公開ビルダー
   （`WorkflowExecutionSnapshotBuilder`）から組む。
4. **`GateOpened.artifacts` / `GateApproved.phase_boundary` / `GateRejected.feedback`** は集約が検証しない
   素通しの投影材料。導出は U5 / U6 の責務である。
5. **`revision_count`** は集約の状態になった（`reject_gate` で +1）。C6 のスナップショット列に 1 列
   追加が要る。

## 12. 追加作業（Q-A の裁定 — `Started.depth` / `test_strategy`）

コンダクタの裁定（2026-08-23）を反映した是正作業。**Q-A = 載せる**（C5 の Started payload と
`entities.md` の payloads がどちらも `depth?` / `test_strategy?` を含み、U4 の Started 投影が状態ファイルの
`Scope Configuration`（`Depth` / `Test Strategy` 行）を描くのに要る。Started の自己完結 = 投影が定義を
読み直さない。計画 §2 の定義で落ちていたのが誤り）。**Q-B = 現実装どおり一般の kebab を受理**（コード変更
なし。`entities.md` 側の訂正は設計側の作業）。§4 の D2 / D3 は本節で更新される。

### 12.1 Red

`StartRequest` のテスト 6 本（新規ファイル `start_request.rs`、テストモジュールのみ）と、`Started` に
`depth` / `test_strategy` が載ることを確認するテスト 1 本を先に書いて実行した:

```
error[E0433]: cannot find type `StartRequest` in this scope        （13 件）
error[E0061]: this function takes 5 arguments but 4 arguments were supplied   （2 件）
error[E0599]: no method named `depth` found for reference `&workflow_execution_event::Started`   （2 件）
error[E0599]: no method named `test_strategy` found for reference `&workflow_execution_event::Started`   （2 件）
error: could not compile `core-domain` (lib test) due to 19 previous errors; 1 warning emitted
```

### 12.2 Green / Refactor

| ファイル | 変更 |
|---|---|
| `modules/core/domain/src/orchestration/start_request.rs` | 新規。`StartRequest { scope, request, depth, test_strategy }`。`new(scope, request)` + `with_depth(..)` / `with_test_strategy(..)` のビルダー風、フィールド private + アクセサ（`depth()` / `test_strategy()` は `Option<&str>`）。検証はしない（素通しの投影材料） |
| `modules/core/domain/src/orchestration/workflow_execution_event.rs` | `Started` に `depth: Option<String>` / `test_strategy: Option<String>` を追加しアクセサを公開。`Started::new(definition_id, definition_revision, &StartRequest, stages)` が C5 の平坦なレコードへ展開する（引数 4 個、`too_many_arguments` の閾値に触れない） |
| `modules/core/domain/src/orchestration/workflow_execution.rs` | `start(intent_id, &definition, &StartRequest, occurred_at)` / `start_with_entries(intent_id, definition_id, definition_revision, &StartRequest, entries, occurred_at)` に改訂（旧シグネチャは残さない）。rustdoc に「2 値は集約状態にならず素通し」を明記。テストヘルパ・PBT の呼出しを移行 |
| `modules/core/domain/src/orchestration/mod.rs` | `pub use start_request::StartRequest;` を追加（Domain Primitive 群） |
| `modules/core/domain/tests/engine_loop_conformance.rs` | `StartRequest::new("itf", "conformance")` へ移行（depth / test_strategy は None） |

**設計上の判断（追加）**: `apply_event(Started)` は `depth` / `test_strategy` を無視する。集約状態にすると
`WorkflowExecutionSnapshot` の 16 属性と C6 のスナップショット列へ波及するため、裁定どおり
**pass-through に留めた**（16 属性は不変）。`StartRequest` は集約が検証しない — 「フラグ上書き or scope
metadata の既定」の解決は呼出側（birth ユースケース）の責務である旨を型の rustdoc とテスト
`the_domain_does_not_validate_the_request` に明記した。

### 12.3 品質ゲート（再実行）

| ゲート | 結果 |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0（warning 0 件） |
| `cargo lint` | exit 0 |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | **471 passed, 0 failed**（+7: `StartRequest` 6 本 + `Started` の depth 1 本） |
| `bash scripts/coverage.sh` | head **97.39%** ≥ 90.0% [PASS] |
| `cargo llvm-cov -p core-domain --summary-only` | lines **96.55%**（着手前基準 94.70%。`start_request.rs` は 100.00%） |
| 合格 grep（BR4.1 / FR8.3） | **0 件** |

### 12.4 コミット

| SHA | メッセージ |
|---|---|
| `fa6bf64` | `feat(core-domain): carry depth / test_strategy on Started via StartRequest (C5)` |

`aidlc/` 配下はコミットしていない。

### 12.5 公開面の更新（I6 の差分）

Domain Primitive 群に `StartRequest` が 1 型加わった（`StageIndex` の次）。ほかの公開型に変更はない。
§8 I6 の列挙に対する追加は合計 4 型（`WorkflowExecutionEventPayload` / `WorkflowExecutionSnapshotBuilder` /
`IntentIdError` / `StartRequest`）となる。

### 12.6 後続への申し送り（更新）

- §11-1 の設計質問 2 件は本節で解消済み（Q-A = 載せる → 実装済み、Q-B = 現実装どおり → コード変更なし。
  `entities.md` の `IntentId` 記述の訂正だけが設計側に残る）。
- U3 は `Started` を journal から再構成するとき `StartRequest` を組んでから `Started::new` を呼ぶ。
- U4 は `Started.depth()` / `test_strategy()` を `Scope Configuration` 行の材料として読む（`None` は
  「指定なし」であり、既定値の解決は投影側ではなく birth ユースケース側で済んでいる前提）。
