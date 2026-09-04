# b47 設計 — `--single`（synthetic-id pair）と `--skeleton-stance`（classify round-trip）を `IntentExecution` のイベントで（2026-09-04）

対象: GitHub #73 の残り（b46 が「not wired in this build」で止めていた 2 面）。契約の正本はピン留め `3c3146cf`
（`aidlc-orchestrate.ts` `emitSingleRunStage` `:4436-4490`、`handleNext` の Branch 4b `:2996-3002`、`handleSingleReport`
`:5261-5330`、`handleSkeletonStanceReport` `:4943-5008`、`readSkeletonStance` / `resolveSkeletonGate` `:1226-1425`、
`buildRunStageDirective` の `gate:"unresolved"` `:4274`、`aidlc-state.ts set-skeleton-stance` `:703-742`）。

## 0. 裁定（オーナー 2026-09-04、着手前に質問）

| 問い | 裁定 | 帰結 |
| --- | --- | --- |
| `report --single` が記録する疑似ワークフロー ID 付きの `STAGE_STARTED` / `STAGE_COMPLETED` 対の置き場 | **B — `IntentExecution` のイベント**（「場当たり的に集約を作らない。よく考えて」） | 隔離実行は「その intent の記録の中で起きた事実」であり、監査台帳はその実行のイベントログの投影。新集約は作らない。仕様 I10（`--single` は本流を進められない）の強制手段は E1（遷移ポート非注入）から **E4 + 単体**へ改訂 — イベントの適用がカーソル・checkbox・`Status`・park・overlay・autonomy・approved を 1 つも動かさない（**フレーム条件**）ことを Quint 不変条件 `single_run_frame`（mutation 検査つき）と集約の単体テストで保証する |
| `report --skeleton-stance` の分類結果の置き場 | **A — `IntentExecution` のコマンドとイベント** | `record_skeleton_stance` → `SkeletonStanceRecorded`。受理ガード（現在地が skeleton-gate stage）と `next_decision` の gate 判定（`unresolved` → 決定）は同じ集約に閉じる |

## 1. 原則からの導出

- **判断は集約**: 隔離実行の受理（計画に在る・initialization でない）、stance の受理（現在地が skeleton-gate stage）、`next` の gate が
  未解決かどうか、はすべて `IntentExecution` のコマンド／クエリ。
- **RMU は投影**: 監査 2 行（`Workflow: single-stage:<slug>`）、`## Runtime State` の `Skeleton Stance` 欄、`read_execution.skeleton_stance`、
  `read_next_answer.gate` の 3 値。
- **クエリ側は読むだけ**、**app は構文段と逐語**。b46 の「not wired」の止め木 2 本を外す。
- **upstream との対応（ピン）**: `next --single` は state を読まず何も記録しない（run-stage を出すだけ — `single: true` / `gate: false` /
  `next_stage: null`）。`report --single` が対を**まとめて**監査へ足す（「開いた試行があるか」の検査はピンには無い — それは 2.7.x の追加）。
  `set-skeleton-stance` は監査行を出さず `Last Updated` も触らない。

## 2. ドメイン（`modules/core/command/domain`）

- **イベント 2 本を追加**（`IntentExecutionEvent` は 11 → 13 変種。DTO・ワイヤ形式・ゴールデンコーパス・RMU 復号を追随）:
  - `SingleStageRunCommitted { id: IntentExecutionEventId, aggregate_id: IntentExecutionId, stage: StageSlug }`
  - `SkeletonStanceRecorded { id, aggregate_id, stance: SkeletonStance }`
- **状態**: `skeleton_stance: Option<SkeletonStance>` を追加（完全コンストラクタ `new` と `IntentExecutionDto` に追随。既存スナップショットは
  `None` で復号 — 後方互換ではなく「欄が無い = 未記録」の正規の意味）。
- **コマンド `record_single_stage_run(&mut self, intent: &Intent, stage: StageIndex, occurred_at)`**: ガードは取り違え → initialization
  （`InvalidTarget(stage)`）だけ。**`guard_running_for` は使わない** — 隔離実行は本流の状態（Completed / park / autonomous）に依らない
  （ピンは監査追記のみ）。計画に無い slug は呼出側（ユースケース）が `slug → index` の解決で断る（`UnknownStage { slug }`）。
  **適用は no-op**（フレーム空）。
- **コマンド `record_skeleton_stance(&mut self, intent: &Intent, stance: SkeletonStance, occurred_at)`**: ガードは取り違え → 現在地が
  skeleton-gate stage でなければ `InvalidTarget(cursor)`。skeleton-gate stage = **静的計画**（`Intent::stages` の grid action、recompose
  overlay は見ない — ピン `isSkeletonGateStage` は `firstInScopeStageOfPhase` = 静的サブグラフ）で Construction フェーズの最初の
  EXECUTE ステージ。再記録は可（上書き — `setOrInsertField`）。running / park のガードは無い（ピンにも無い）。
  適用は `skeleton_stance = Some(stance)`。
- **クエリ**: `skeleton_stance()`、`skeleton_gate_stage(&self, intent) -> Option<StageIndex>`。
- **`next_decision`**: `RunStage { stage, gate: bool }` を `RunStage { stage, gate: GateDecision }` へ（domain の 3 値
  `GateDecision { Gated, Ungated, Unresolved }` — クエリ側 `GateField` とは別型）。`Unresolved` = ゲート付き ∧ cursor が skeleton-gate stage ∧
  `skeleton_stance` が `None`。記録後は `resolveSkeletonGate` がどの stance でも `true` を返すので `Gated`。
  `EngineSignal` / ITF の観測射影（`DRunStage(int)`）は gate を含まないので不変。

## 3. Quint（`formal/orchestration/engine_loop.qnt`、v2.4）

- `var stanceRecorded: bool`（+ `prevStanceRecorded`、`snapshot` に追加）。skeleton-gate stage の抽象は
  **`skeletonGateStage` = 静的計画 `plan` の最初の非 init EXECUTE ステージ**（-1 = 該当なし）。~~`cursor == 1`~~ は不忠実だった —
  実装レビュー時に反例（縮退誕生 `plan=[E,S,S,S,S]` → recompose で overlay[1] を Execute へ → jump で cursor=1 だが
  `plan[1] == SkipPlan`。Rust の `skeleton_gate_stage` は静的計画を見るので `None` を返し準拠再生が赤になる）が 4 シードで
  即座に見つかり、静的計画由来へ改めた（設計の訂正 2026-09-04。再採取フィクスチャの実測でも stance の記録位置は cursor 3 / 2 で
  索引 1 ではない）。合成計画のフェーズ割当は「索引 0 = Initialization、1 以降 = Construction」（`gated(s) = s != 0` の抽象は不変）。
- `actRecordSkeletonStance`: guard `status == Running and cursor == skeletonGateStage`、`stanceRecorded' = true`、他は不変、
  `lastAction' = "record_skeleton_stance"`（`status == Running` はモデルが狭い側 — Rust は park 中・完了後も拒まないが、準拠再生を
  減らすだけで実装の受理集合を偽らない）。
- `actSingleRun`: `nondet s = STAGES.filter(s => s != 0).oneOf()`、**全状態変数不変**（`stanceRecorded` も）、`lastDirective' = DNone`、
  `lastAction' = "single_run"`。
- 不変条件: `single_run_frame`（`lastAction == "single_run"` ⇒ cursor / status / checkbox / overlay / parkedAt / autonomous / approved /
  stanceRecorded がすべて prev と等しい）、`stance_frame`（`lastAction == "record_skeleton_stance"` ⇒ stanceRecorded 以外が prev と等しい）。
  ゲートの `--invariants` に登録。witness `w_single_run`（`lastAction == "single_run"`）と `w_stance_recorded`（`lastAction == "record_skeleton_stance"`）を負形式で登録。
- **mutation 検査**（ADR 0003 DoD）: `actSingleRun` に `cursor' = cursor + 1` を注入 → `single_run_frame` が検出、`actRecordSkeletonStance` に
  `checkbox' = checkbox.set(cursor, CompletedBox)` を注入 → `stance_frame` が検出。結果を §9 に記録。
- **ITF フィクスチャ**: 状態変数が増えるので既存 9 本（`0xa1 0xb2 0xc3 0xd4 0xe5 0xf6 0x101 0x202 0x303`）を**同じ seed・同じ採取条件で
  再採取**し、`#meta` 正規化のうえ差し替える（ADR 0003「.qnt 変更を含む PR ではフィクスチャ再生成」）。採取コマンドは既存フィクスチャの
  採取記録（`tests/conformance/` の README か design 記録）から引く。新規 `trace-0x404`（`--invariant 'not(w_single_run)'`）と
  `trace-0x505`（`not(w_stance_recorded)`）を追加。準拠テストの網羅リストに `single_run` / `record_skeleton_stance` を追加し、
  再生では `single_run` → `record_single_stage_run(任意の非 init ステージ)`、`record_skeleton_stance` → `record_skeleton_stance(On)` を打つ。
  `stanceRecorded` は集約の `skeleton_stance().is_some()` と突き合わせる。

## 4. ユースケース（`modules/core/command/use-case`）

- `RecordSingleStageRunUseCase::execute(execution_id, stage: &StageSlug, occurred_at) -> Result<(), SingleStageRunError>`（CQS。
  find 実行 → find intent → `slug → index`（無ければ `UnknownStage`）→ `record_single_stage_run` → store、`Conflict` は 1 回再試行）。
- `RecordSkeletonStanceUseCase::execute(execution_id, stance, occurred_at) -> Result<(), SkeletonStanceError>`（同じ 3 手）。
- エラー封筒: `SingleStageRunError { Repository, IntentRepository, UnknownStage { slug }, Command { stage: StageSlug, error: CommandError } }`、
  `SkeletonStanceError { Repository, IntentRepository, Command { stage: StageSlug, scope: String, error: CommandError } }`（文言の材料）。
  連鎖を切らない（`CommitError` の規律）。1 ファイル 1 公開型。
- **I10 の補助**: この 2 ユースケースが `IntentExecutionRepository` を持つのは B の帰結。本流を進めないことは §2 のフレーム空適用と
  §3 の不変条件が保証する（設計 §0）。

## 5. RMU（`modules/core/read-model-updater`）

- `SingleStageRunCommitted` → 監査 2 行を**この順**で（ピン `:5326-5341`）: `STAGE_STARTED` {`Stage`, `Agent`（計画の lead_agent）, `Workflow: single-stage:<slug>`} →
  `STAGE_COMPLETED` {`Stage`, `Details: Single-stage run of <slug> completed`, `Workflow`}。状態ファイル・`read_*` 表は**一切動かさない**
  （フレーム空）。`key::WORKFLOW` を追加。
- `SkeletonStanceRecorded` → `## Runtime State` に `- **Skeleton Stance**: <v>` を setOrInsert（監査行なし、`Last Updated` も触らない —
  ピン `set-skeleton-stance`）、`read_execution.skeleton_stance TEXT NULL`。
- `read_next_answer`: `gated INTEGER NULL` を **`gate TEXT NULL`**（綴り `gated` / `ungated` / `unresolved`、run-stage 以外は NULL）に置換。
  綴りの正本は domain の `GateDecision::spelling()`。クエリ側 `NextAnswerView::gate() -> Option<GateField>`（`GateField::parse`）。
- `read_run_stage.gate_default` は不変。

## 6. app（`modules/app/aidlc`）

- **`next --single`**（ピン `emitSingleRunStage` に揃える。現状の `turn.rs::single` を改訂）:
  `--phase` 併用 → `Cannot use --single with --phase. --single runs one stage; pass --stage <slug>.`（分岐 2 の後・`--stage` 検査の前）、
  `--stage` 無し → `--single requires --stage <slug>. A stage-runner runs exactly one named stage.`（現行 `SINGLE_REQUIRES_STAGE` は短い — 訂正）、
  未知 → `Unknown stage "<slug>". Run /aidlc --help for the full list.`、initialization → `SINGLE_INIT_ERROR`
  （`Cannot run an initialization stage with --single. Initialization is bootstrap (it creates the intent + state); it runs automatically when you start a workflow (describe what to build, e.g. /aidlc "build the auth service").`）、
  scope 外 → `Stage "<slug>" is skipped for scope "<scope>". Choose a different stage or change scope.`、
  directive は `single: true` / `gate: false` / `next_stage` 無し（`directive_drawing::run_stage` の single 経路で強制。現状は `gate_default` を使っている — 訂正）。
  state は読まず何も記録しない。
- **`report --single`**: b46 の構文段の後（result 必須 / FORWARD のみ / `--stage` 必須 — 空判定は trim しない、b46 PR #102 の記録どおり）→
  実行カーソル（不在 → `Failed to record single-stage lifecycle pair for "<slug>": no active intent record` — upstream に対応する逐語は無い）→
  `RecordSingleStageRunUseCase` → `catch_up` → `done` `Single-stage run of "<slug>" committed under synthetic workflow "single-stage:<slug>". The main workflow's Current Stage is untouched.`。
  拒否: `UnknownStage` → `Unknown stage "<slug>". Run /aidlc --help for the full list.`、`InvalidTarget` → `SINGLE_INIT_ERROR`、
  ポート失敗 → `Failed to record single-stage lifecycle pair for "<slug>": <chained>`。
  順序はピン: result → FORWARD → stage → unknown → initialization →（evidence は slice 2）→ 記録。
- **`report --skeleton-stance`**: 値検証 → state 必須（b46 のまま）→ `RecordSkeletonStanceUseCase` → `catch_up` → `print`
  `Recorded walking-skeleton stance "<v>" for "<slug>". Re-run \`next\` to continue — the gate is now determined.`。
  拒否 `InvalidTarget` → `Current stage "<slug>" is not the skeleton-gate stage for scope "<scope>" — a skeleton stance is only reported for the first Construction Bolt's gate.`、
  ポート失敗 → `Failed to record skeleton stance for "<slug>": <chained>`。
- **`next` の gate**: `read_next_answer.gate` を `GateField` に写して描く（`"unresolved"` の描画はプレゼンタに既にある）。
- b46 の `single_report_not_wired` / `skeleton_stance_not_wired` と `wording` 定数を撤去（後方互換なし）。

## 7. テスト

- domain: 2 コマンドのガード、`SingleStageRunCommitted` 適用の**フレーム空**（適用前後で集約が `==`）、`SkeletonStanceRecorded` の適用、
  `next_decision` の `Unresolved` → 記録後 `Gated`（Construction ステージを含む合成計画で）、skeleton-gate stage の導出（静的 grid、overlay 無視）。
- DTO: 2 変種の往復、ゴールデンコーパス（`golden_corpus_read.rs` の変種一覧）。
- use-case: 2 本（成功・`UnknownStage`・`Command`・Conflict 再試行）。
- RMU: 監査 2 行の逐語（フィールド順）、Runtime State の setOrInsert（無い → 挿入、有る → 置換）、`read_execution.skeleton_stance`、
  `read_next_answer.gate` の 3 綴り。
- app 結合: `next --single` の 5 拒否 + 成功（`single` / `gate` / `next_stage`）、`report --single` の成功と拒否、round-trip
  「Construction の最初のステージで `next` → `gate: "unresolved"` → `report --skeleton-stance on` → `next` → `gate: true`」、
  stance の拒否文言（Inception のステージで報告）。合成グラフに Construction ステージ（例 `functional-design`）を足す。
- Quint: ゲート全 PASS、mutation 2 件、フィクスチャ再採取 9 + 新規 2、準拠テスト網羅。

## 8. 仕様・記録

- `docs/specs/10-orchestration.md`: §6 I10 行の強制手段を E1 → **E4（`single_run_frame`）+ 単体**（オーナー裁定 2026-09-04 = B）に改訂し、
  §3 のユースケース名（`SingleStageRun` → `RecordSingleStageRun` / `RecordSkeletonStance`）、§9 の「未着手: skeleton stance / gate:"unresolved"」
  を解消、§10 に実装ノート（b47）。`docs/specs/01-domain-model.md` の B11 は不変（アンカー計算は静的計画から）。
- Issue #73 は本 PR で `Closes #73`（残る B10 受領証は b48 として #7 キュー 5 に残す）。

## 9. 検証記録（2026-09-04 実測、実装は Opus サブエージェント + Quint 下請け、統合レビューは Fable 5）

- **ドメイン**: `IntentExecutionEvent` を 13 変種へ（`SingleStageRunCommitted { stage }` / `SkeletonStanceRecorded { stance }`）。
  `IntentExecution` に `skeleton_stance: Option<SkeletonStance>` を追加。コマンド `record_single_stage_run`（取り違え → initialization /
  範囲外 `InvalidTarget` だけ。**本流の状態に依らず受理** — Completed / park 中 / autonomous でも通る。**適用はフレーム空**で、
  通番以外が `==` のまま — I10 の実体）と `record_skeleton_stance`（現在地が**静的計画**の Construction 最初の EXECUTE ステージでなければ
  `InvalidTarget`。recompose overlay は見ない。再記録は上書き）。`next_decision(&self, intent, request)` は `RunStage.gate` を 3 値
  `GateDecision`（`Gated` / `Ungated` / `Unresolved`）で返す — `Unresolved` = ゲート付き ∧ skeleton-gate stage ∧ stance 未記録。
- **DTO**: 2 変種の永続化 DTO、`dto_vocabulary` に stance の綴り、スナップショット行に `skeleton_stance`（欄不在は `None`）。ワイヤ形式
  13 変種を両側のゴールデンコーパスで固定。
- **ユースケース**: `RecordSingleStageRunUseCase` / `RecordSkeletonStanceUseCase`（find → コマンド → store、`Conflict` 1 回再試行）と封筒 2 本。
- **RMU**: `SingleStageRunCommitted` → 監査 2 行だけ（`STAGE_STARTED` {Stage, Agent, Workflow: single-stage:<slug>} → `STAGE_COMPLETED`
  {Stage, Details: Single-stage run of <slug> completed, Workflow}。状態ファイルと `read_*` は不変）。`SkeletonStanceRecorded` →
  `## Runtime State` の `Skeleton Stance` 欄を setOrInsert（監査行なし、`Last Updated` 不変）と `read_execution.skeleton_stance`。
  `read_next_answer.gated INTEGER` → `gate TEXT`（3 綴り、正本は `GateDecision::spelling`）、`read_run_stage.in_scope`（`--single` の scope 外ガードの材料）。
- **クエリ側**: `GateField::parse`、`NextAnswerView::gate() -> Option<GateField>`、`RunStageView::in_scope()`。
- **app**: `next --single` を pinned `emitSingleRunStage` に揃えた（`--phase` 併用 / `--stage` 必須（逐語訂正）/ 未知 / initialization / scope 外の
  5 拒否、`single: true` / `gate: false` / `next_stage` 不在を `directive_drawing` の single 経路で強制、state は読まない）。
  `report --single` は result → FORWARD → `--stage`（trim なし）→ 未知 → initialization → 記録 → `catch_up` → `done`
  `Single-stage run of "<slug>" committed under synthetic workflow "single-stage:<slug>". The main workflow's Current Stage is untouched.`。
  `report --skeleton-stance` は値検証 → state 必須 → 記録 → `catch_up` → `print`
  `Recorded walking-skeleton stance "<v>" for "<slug>". Re-run \`next\` to continue — the gate is now determined.`、拒否
  `Current stage "<slug>" is not the skeleton-gate stage for scope "<scope>" — ...`、失敗 `Failed to record ... for "<slug>": <detail>` 2 形。
  b46 の「not wired」2 本と短い旧 `SINGLE_REQUIRES_STAGE` を撤去。
- **Quint v2.4**: `stanceRecorded` 変数、`actRecordSkeletonStance`（guard `cursor == skeletonGateStage` — 静的計画由来）と `actSingleRun`
  （全変数不変）、不変条件 `single_run_frame` / `stance_frame`（12 本へ）、witness `w_single_run` / `w_stance_recorded`。
  mutation 検査: `actSingleRun` に `cursor' = cursor + 1` → `single_run_frame` が検出、`actRecordSkeletonStance` に
  `checkbox' = checkbox.set(cursor, CompletedBox)` → `stance_frame`（と `no_gate_bypass`）が検出。ITF フィクスチャは状態変数が
  増えたため既存 9 本を同じ seed で再採取（採取条件は v2.2 のモデルで既存ファイルをバイト再現できるものを総当たりで確定）+ 新規
  `trace-0x404`（`not(w_single_run)`）/ `trace-0x505`（`not(w_stance_recorded)`）。準拠テストは `stanceRecorded` を `skeleton_stance().is_some()` と
  突き合わせ、網羅リストに `single_run` / `record_skeleton_stance` を追加、合成計画のフェーズ割当を「索引 1 以降 = Construction」へ。
- **テスト**: 新規 54 本（`#[test]` / `#[tokio::test]` の増分）。フレーム空の固定 `an_isolated_run_records_the_stage_without_moving_the_workflow`、
  監査 2 行の逐語 `an_isolated_run_appends_the_two_audit_rows_verbatim_and_touches_nothing_else`、round-trip
  `the_skeleton_gate_round_trip_turns_unresolved_into_a_determined_gate`、`next --single` の 5 拒否と成功、`report --single` / `--skeleton-stance` の逐語。
- `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `cargo test --workspace` — **全緑（49 スイート、1,859 本）**。`tools/lint` 自己テスト 69 本も緑
- `scripts/quint-gate.sh` — **全 PASS（39 ステップ。不変条件 12 本、witness に `w_single_run` / `w_stance_recorded`）**
- `scripts/coverage.sh --base origin/main` — 絶対 **99.12% ≥ 90.0%**、相対 **99.12% ≥ 99.12%（base 99.117 − 0.01）PASS**

### ITF フィクスチャの採取コマンド（`M=formal/orchestration/engine_loop.qnt`、`D=tests/conformance/fixtures/engine_loop`）

```
for s in 0xa1 0xb2 0xc3 0xd4 0xe5 0xf6 0x202; do quint run $M --seed $s --max-samples 1 --max-steps 40 --out-itf $D/trace-$s.itf.json; done
quint run $M --seed 0x101 --max-samples 2000 --max-steps 40 --invariant 'not(lastAction == "report_revised")' --out-itf $D/trace-0x101.itf.json
quint run $M --seed 0x303 --max-samples 2000 --max-steps 40 --invariant 'not(w_repark)'          --out-itf $D/trace-0x303.itf.json
quint run $M --seed 0x404 --max-samples 2000 --max-steps 40 --invariant 'not(w_single_run)'      --out-itf $D/trace-0x404.itf.json
quint run $M --seed 0x505 --max-samples 2000 --max-steps 40 --invariant 'not(w_stance_recorded)' --out-itf $D/trace-0x505.itf.json
```

`#meta` は既存ファイルと同じく quint の生出力のまま（1 行 JSON・末尾改行なし）。状態数: 素の 7 本 = 41、0x101 = 14、0x303 = 13、0x404 / 0x505 = 5。

### 設計との差分（実装レビューで受け入れたもの）

1. ドメイン層は一部が実装先行になり、TDD の red は mutation 2 件（`SingleStageRunCommitted` の適用に autonomy 変更を注入 → フレーム空テストが赤、`gate_decision` から `Unresolved` 分岐を削除 → 3 値テストが赤）で検出力を証明した。
2. `next_decision(&self, intent: &Intent, request)` へ署名変更 — skeleton-gate stage が静的計画（`Intent::stages`）を要るため。RMU は既に intent を持っている。
3. ゲート判定は upstream `computeGate(node)` と同じく **RunStage が名指すステージ**に対して計算する（カーソルではない）。
4. `SkeletonStanceError::Command.stage` は `Option<StageSlug>`（`unwrap` 回避 — 型が「必ず在る」を知らないため）。
5. `read_run_stage.in_scope` 列を新設（定義 × scope で決まる静的グリッドの値 — `next --single` の scope 外ガードの材料）。
6. `report --single` の拒否順は「実行カーソル不在 → 未知ステージ / initialization」。ピンは未知 → initialization → 記録（記録先の不在は spawn の失敗として最後に出る）。差が出るのは「記録が無く、かつ stage も不正」の組合せだけ。
7. `read_next_answer.gate` の `ungated` 綴りは現状の経路（誕生 = 初期化完了済み）では到達しない — 3 値の完全性のために残す。
8. 準拠テストの網羅コメントを実測に合わせた（`report_revised` は 0x101、`report_skipped` は 0xe5 が持つ）。
