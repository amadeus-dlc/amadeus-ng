# b45 設計 — park の完全実装: 集約ガード再設計（再スタンプ許容）+ `ParkUseCase` + 逐語 3 形（2026-09-04）

対象: GitHub #74（#7 キュー 4）。前提 Bolt: b44（#100 — app が `read_*` 経路で `next` / `continue` を描く）。
b29（U7 配線）で park は「認識するが未配線」（`Cannot park the workflow: park is not wired in this build.`）
のまま保留されていた。本 Bolt でその暫定ハンドラを本物に差し替える。呼出側の契約
（`aidlc-orchestrate park` → stdout に directive 1 行、失敗はビジネス拒否 = `error` directive / exit 0）は不変。

## 0. 原則からの導出

- **コマンド側 = 集約と判断**: park を受理するか、どの順で拒否するかは `IntentExecution::park` だけが決める。
  ユースケースは find → コマンド → store の進行管理だけ（`use-case-rules.md`）。
- **RMU = 計算結果をリードモデルへ投影**: `Parked` イベントから `Parked` / `Parked At Stage` の状態ファイル欄、
  `WORKFLOW_PARKED` の監査ブロック、`read_execution.parked_at_slug` / `parked_active`、`read_next_answer` の
  `decision_kind = parked` を描く。これは b39〜b41 で実装済みで、本 Bolt では触らない。
- **クエリ側 = DAO で View を読んで返すだけ**: park 成功後に app が stage を知る経路は、上流と同じく
  **投影された行を読む**（upstream `handlePark` も mutation 後に状態ファイルの `Parked At Stage` を読み直す）。
  既存の `FindExecutionUseCase::execute(execution_id)` → `ExecutionView::parked_at_slug()` を使う。
  ユースケース（コマンド側）は CQS を守り何も返さない（`CommitVerdictUseCase` と同じ）。
- **文言は出す側が組む**: 逐語 3 形と中継形 `Cannot park the workflow: <detail>` は app の `wording.rs`。

## 1. 集約 — `IntentExecution::park` の受理述語（park 専用）

upstream `aidlc-state.ts handlePark` の順序と意味論（採取 `3c3146cf`、`.claude/tools/aidlc-state.ts:1685-1760`）:

1. `Construction Autonomy Mode: autonomous` → 拒否 `Refusing to park: Construction Autonomy Mode is autonomous. An unattended autonomous run has no human to resume it and must keep moving - do not park it.`
2. `Status: Completed` → 拒否 `Workflow is already Completed - nothing to park.`
3. `Current Stage` 不在 → 拒否 `State file has no Current Stage - cannot park.`（我々では **Always Valid の帰結として構造的に発生不能** — カーソルは不変条件 `cursor_in_scope` により常に計画上の位置を指す。M12 と同型の upstream 側限定経路であり逸脱ではない — §7 の仕様ノート）
4. それ以外は**成功**。park 済みでも成功し、`Parked`（時刻）を上書き・`Parked At Stage` を書き直し・`WORKFLOW_PARKED` を再 emit する（**再スタンプ**）。

新しい `park`:

```text
park(intent, occurred_at):
  if !matches(intent)            -> IntentMismatch      (取り違えガード — 従来どおり入口 1 か所)
  if autonomy.is_autonomous()    -> RefusedUnderAutonomy (順序 1)
  if !status.is_running()        -> NotRunning           (順序 2 — Status は Running | Completed の 2 値なので Completed と同義)
  // parked_active() は見ない — 再スタンプ許容（順序 4）
  commit Parked { stage = slug_of(cursor) }
```

- `accepts_commands()`（BR1.0: Running ∧ ¬parked_active）は他コマンドが共有するので**触らない**。park だけが
  `guard_running_for` を使わず、自分の受理述語を持つ。
- 適用（`apply` の `Parked` 腕 — `parked_at = Some(stage)`）と不変条件 `parked_position`（park の位置 = カーソル）は
  そのまま。再スタンプは同じ位置に同じ値を書くだけなので、`applying_a_park_away_from_the_cursor_crashes` も不変。
- 既存テストの改訂: `every_command_but_unpark_is_refused_while_parked` の `park` 行は `Ok(Parked)` に変わる
  （再スタンプ）。追加: 「autonomous ∧ Completed は autonomy の拒否が勝つ」「Completed は `NotRunning`」
  「park 済みへの park は `Parked` を再 emit し `parked_active` のまま」。

## 2. Quint モデル（`formal/orchestration/engine_loop.qnt`）— #74 スコープ 2

現状 `actPark` は `status == Running` を要求し、`WorkflowParked` からの再 park を**許さない**。upstream の再スタンプ
意味論と食い違うので改訂する:

- `actPark` のガードを `or(status == Running, status == WorkflowParked)`（∧ `not(autonomous)`）へ。
  `status' = WorkflowParked`、`parkedAt' = cursor`、`lastDirective' = DParked`、`lastAction' = "park"` は不変。
  （3 値 status は「マーカー有 ∧ 位置一致」の合成状態の簡約 — ヘッダ注記どおり。stale-by-progress は slice 2）
- 到達性 witness `w_repark = (lastAction == "park") and (prevParkedAt != -1)` を追加し、`scripts/quint-gate.sh` の
  負形式 witness（`--invariant "not(w_repark)"` が違反を見つける = 経路実在）へ登録する。
- **mutation 検査**（ADR 0003 DoD）: 再 park 腕で `parkedAt' = -1`（マーカー脱落）にした変異が `parked_position` で
  検出されること、`status' = Running` にした変異が `parked_position` か `unpark_restores_position` で検出されることを
  実測し、結果を design.md §8 と handoff に記録する（モデル本体には残さない）。
- **ITF フィクスチャ再採取**: `quint run --seed 0x303 --max-samples 2000 --max-steps 40 --invariant 'not(w_repark)' --out-itf tests/conformance/fixtures/engine_loop/trace-0x303.itf.json`
  で再 park を含むトレースを採り、既存フィクスチャと同じく `#meta` を正規化してコミットする。既存 8 本は
  ガード緩和で無効にならない（許容集合が広がるだけで、既存トレースの各遷移は引き続き有効）ことを
  `engine_loop_conformance` の全再生で確認する。
- 準拠テスト（`modules/core/command/domain/tests/engine_loop_conformance.rs`）: `park` の再生で集約が既に
  `parked_active()` なら合成アクション名 `repark` を `seen` に積み、網羅アサートに `"repark"` を追加する
  （再スタンプ経路を含むフィクスチャの消失退行を防ぐ）。

## 3. ユースケース — `ParkUseCase`（`modules/core/command/use-case/src/orchestration/`）

`CommitVerdictUseCase` と同じ形（1 ファイル 1 公開型、ポートは保持して `execute` 内で使う、引数は集約 ID と値だけ）:

```text
pub struct ParkUseCase<E: IntentExecutionRepository, I: IntentRepository> { .. }
impl ParkUseCase { pub const fn new(execution_repo, intent_repo) -> Self;
  pub async fn execute(&mut self, execution_id: &IntentExecutionId, occurred_at: DateTime<Utc>) -> Result<(), ParkError> }
```

- 流れ: ① `find_by_id(execution_id)` → ② `intent_repository.find_by_id(aggregate.intent_id())` → ③ `aggregate.park(&intent, occurred_at)` → ④ `store(&event, &aggregate)`。
- 楽観 version の `Conflict` は **1 回だけ**再構成からやり直す（`CommitVerdictUseCase` と同じ規律。park は
  対象ステージの名指しが無いので `AttemptOutcome` の target は不要 — 単純な 2 回試行）。
- `ParkError { Repository(RepositoryError<IntentExecutionId>), IntentRepository(RepositoryError<IntentId>), Command(CommandError) }`
  — 封筒であり言い換えない。`Display` / `Error::source` は `CommitError` と同じ規律（連鎖を切らない）。
- `mod.rs` のファサードに `pub use park_use_case::ParkUseCase; pub use park_error::ParkError;` を追加。
- テスト（`test_support.rs` の in-memory リポジトリを使う）: 成功で `Parked` が store されること、autonomous の
  `Command(RefusedUnderAutonomy)`、Completed の `Command(NotRunning)`、park 済みでもう一度 park が通ること、
  `Conflict` 1 回は再試行で通り 2 回連続は `Repository(Conflict)` で返ること。

## 4. app（合成ルート `modules/app/aidlc/src/runtime.rs` / `wording.rs`）

`Request::Park` → `park(layout)`:

1. `store_path(layout)` — 失敗は `emit_error(message)`（report と同じ。`invalid_active_space` は完結した利用者向け文で、`next` も同じ状況を `error` directive で返す — 初稿の「`Completion::refused`」は誤記で、実装レビュー時に訂正）。
2. `active_execution(layout)` — `Ok(None)`（未鋳造）は `emit_error(wording::park_refused("No workflow execution to park. Run `next` first."))`
   （upstream に対応する逐語は無い — upstream は状態ファイル不在時に readStateFile の失敗文を中継する）。
   `Err` は `unreadable_execution_cursor` の中継 `park_refused(..)`。
3. リポジトリ 2 つを open（失敗は `orchestrate_failure("cannot open the event store")`）→ `ParkUseCase::execute(&execution_id, Utc::now())`。
4. 失敗の描き方（すべて `error` directive、exit 0 — upstream の `handlePark` 自身が非ゼロ exit を stdout の error directive に中継する層に合わせる）:
   - `ParkError::Command(CommandError::RefusedUnderAutonomy)` → `wording::park_refused(PARK_REFUSED_AUTONOMOUS)`（逐語 1）
   - `ParkError::Command(CommandError::NotRunning)` → `wording::park_refused(PARK_NOTHING_TO_PARK)`（逐語 2）
   - それ以外 → `wording::park_refused(&error.to_string())`（中継形 — 材料はエラーの `Display`）
5. `catch_up(layout)` — 失敗は `Completion::refused(orchestrate_failure(..))`（report と同じ。握り潰さない）。
6. `ReadModelDaos::open(store)` → `FindExecutionUseCase::new(daos.execution()).execute(execution_id)` →
   `parked_at_slug` → `Directive::Parked { stage: StageSlugView::parse(slug), message: wording::parked(slug) }`。
   行が無い / slug が無い / 読めないは壊れた投影として `Completion::refused(orchestrate_failure(..))`
   （投影直後に無いのは実装の穴であり、利用者の操作で起きる形ではない）。
7. `wording.rs` に追加: `pub fn park_refused(detail: &str) -> String`（`Cannot park the workflow: {detail}`）、
   `pub const PARK_REFUSED_AUTONOMOUS`（逐語 1、`aidlc-state.ts:1712-1714`）、`pub const PARK_NOTHING_TO_PARK`（逐語 2、`:1742`）。
   `parked(stage)` は既存（分岐 2.5 と同じ文言 `Workflow parked at "<slug>". Resume with /aidlc --resume.`）。
   `narration`（`Pausing here with everything saved. ...`）は directive 全般の既知の欠落（cli_golden_test の記載）で本 Bolt の対象外。

## 5. テスト（app 結合 — `modules/app/aidlc/tests/`）

- `intent_lifecycle.rs`: 未配線を固定していた `park_is_refused_as_a_business_error_on_stdout` を本物に差し替える —
  鋳造 → `park` → `kind = parked`・`reason` 逐語・`stage = カーソルの slug`、状態ファイルに `- **Parked**: <ts>` /
  `- **Parked At Stage**: <slug>` が `## Runtime State` 末尾に入り、監査シャードに `WORKFLOW_PARKED` ブロック
  （`**Stage**: <slug>`）が追記される。続けて `park` → 成功（再スタンプ: `WORKFLOW_PARKED` が 2 ブロック、`Parked` は 1 行のまま）。
- 拒否 2 形: Completed（全ステージを report で畳んで `Completed` にしてから `park`）と autonomous（`set-autonomy` は
  未配線（#72）なので、テストは `IntentExecutionRepositoryImpl` を直接開いて `switch_autonomy` を store してから
  `park` — リポジトリ経由の実駆動）。
- `cli_golden_test.rs`: `park/park` を追加 — 採取済み `stdout.json` とキー集合を突き合わせ（`narration` は既知の欠落）、
  `reason` / `stage` / `kind` の値はバイト一致。
- `next_branches.rs`: b44 が `rewrite_decision_kind("bare", "parked")` で作っていた parked 系（`a_parked_execution_stops_the_bare_next_at_its_stage` /
  `unpark-then-resume`）を **`park` の実駆動**に置き換える（handoff-b44 の約束）。壊れた投影の注入（slug でない
  parked 行など）はそのまま直接書きでよい。
- ドメイン単体・ユースケース単体は §1 / §3。

## 6. TDD の順序（team.md「レイヤーごとに red-green-refactor」）

1. domain: 赤（再スタンプ受理・順序）→ 緑（`park` 書き換え）→ 既存テスト改訂。
2. Quint: モデル改訂 → `scripts/quint-gate.sh` 緑 → witness / mutation / フィクスチャ再採取 → 準拠テスト緑。
3. use-case: 赤（`ParkUseCase` テスト）→ 緑。
4. app: 赤（結合テスト差し替え）→ 緑（配線 + 文言）→ golden → `next_branches` 実駆動化。
5. 全ゲート（fmt / clippy / lint / test / quint / coverage 相対）。

## 7. 仕様への反映（本 Bolt でオーナー裁定は不要 — #74 のスコープを実装するだけ）

- `docs/specs/10-orchestration.md` §10 実装ノートに「park」項を追加: 受理順序（autonomy → Completed）、再スタンプ
  意味論、`State file has no Current Stage - cannot park.` が構造的に発生不能で逸脱ではないこと（M12 同型）、
  失敗はすべて `Cannot park the workflow: <detail>` の error directive。
- `docs/specs/deviations.md` は追記しない（逸脱ではない）。

## 8. 検証記録（2026-09-04 実測、実装は Opus サブエージェント、統合レビューは Fable 5）

- **受理述語（実装）**: 取り違え → autonomy → `Status::is_running` の 3 段。`parked_active()` は見ない。upstream 順序 3
  （`Current Stage` 不在）は構造的に発生不能。既存テスト `every_command_but_unpark_is_refused_while_parked` は
  `every_command_but_park_and_unpark_is_refused_while_parked` へ改訂（park 行を削除）。
- **Quint v2.3**: `actPark` のガードを `or { status == Running, status == WorkflowParked }` へ。到達性 witness
  `w_repark = (lastAction == "park") and (prevParkedAt != -1)`（gate に seed `0x303` で登録、PASS）。
  **mutation 検査**（`quint run --seed 0x1a2b3c --max-samples 2000 --max-steps 40 --invariants <10 本>`、
  変異は一時適用し `diff -q` で復元確認）:

  | 変異 | 検出した不変条件 |
  | --- | --- |
  | `actPark` の `parkedAt' = -1`（マーカー脱落） | `parked_position` / `unpark_restores_position` |
  | `actPark` の `status' = Running` | 初回実測では **9 不変条件のいずれも検出せず**（`WorkflowParked` が到達不能になり `parked_position` / `unpark_restores_position` が空虚に成立 — b45 以前の `Running` 限定ガードでも同じで、b45 の退行ではない既存の検出穴） → `parked_marker_status = (parkedAt != -1) implies (status == WorkflowParked)` を追加（統合判断 — モデルヘッダの「3 値 status は合成状態の簡約」をそのまま不変条件にしたもの。オーナー確認待ち）→ **`parked_marker_status` のみが検出**。2 本は相補（マーカー脱落は前件が偽になるので `parked_position` 側が捕まえる） |

  設計 §2 の想定「`status' = Running` は `parked_position` か `unpark_restores_position` で検出」は**外れていた**
  （空虚成立の見落とし）。不変条件は 10 本になり gate へ登録済み。
- **ITF フィクスチャ再採取**: `quint run formal/orchestration/engine_loop.qnt --seed 0x303 --max-samples 2000 --max-steps 40 --invariant 'not(w_repark)' --out-itf tests/conformance/fixtures/engine_loop/trace-0x303.itf.json`
  → 4 状態、`repark` は step 3（`lastAction = park`、`prevParkedAt = 1`、`status = WorkflowParked`）。既存 8 本は
  全再生で有効のまま。準拠テストは `park` 再生時に `parked_active()` なら合成アクション `repark` を積み、網羅
  アサートに `"repark"` を追加（フィクスチャを退けると赤になることを確認）。
- **ゴールデン**: `cli/park/park` は `kind` / `reason` / `stage` の 3 値がバイト一致。欠落は `narration` 1 キー
  （`Directive::Parked` に欄が無い — directive 全般の既知の欠落、b45 対象外）。
- **テスト**: park 関連の新規 24 本（domain 3・use-case 13・app 統合 7・app 単体 4）。各レイヤーで赤を先に確認
  （domain: `parking_an_already_parked_execution_restamps_the_marker` が `Err(NotRunning)` で赤 / use-case: 未実装の
  `mod` 登録で `E0432` / app: `parking_stamps_the_marker_and_projects_both_faces` ほか 4 本が未配線ハンドラで FAILED）。
  b44 で行を直接置いていた parked 系 3 本（`turn.rs` 2・`next_branches.rs` 1）を `park` の実駆動へ置換し、
  `rewrite_decision_kind` を削除。
- **残る未カバー 4 行**: `runtime.rs:296`（park 直後の壊れた投影の分岐 — 分岐先 `parked_directive` は単体で全経路）、
  `park_use_case.rs:137`（in-memory ダブルが `Conflict` 以外の store 失敗を返せない）、テスト内 let-else の `panic!` 2 行。
- **ゲート実測**: 本文の PR 記載を参照（fmt / clippy / lint / test 49 スイート 1,730 本 / Quint 17 ステップ / coverage 相対）。
