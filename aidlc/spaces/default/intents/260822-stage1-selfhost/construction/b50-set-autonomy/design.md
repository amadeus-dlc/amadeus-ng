# b50 設計 — `aidlc-bolt set-autonomy` 面の一括着地: human presence ガード（I11）の CQRS 裁定 + `SwitchAutonomyUseCase` + `--mode` 逐語ピン（2026-09-05）

対象: GitHub #72（#7 キュー 6）。前段: [`b49-practices-receipt/design.md`](../b49-practices-receipt/design.md)。
ピン: upstream `3c3146cf`（`core/tools/aidlc-bolt.ts` `handleSetAutonomy` `:799-859`、`parseFlags` `:60-77`、dispatcher `:889-910`；`aidlc-lib.ts` `humanActedSinceGate` `:3726-3860`、`humanPresenceGuardDisabled` `:6542`、`setFieldStrict`；`aidlc-orchestrate.ts` `readAutonomyMode` `:1263`）。

## 0. 裁定（オーナー、着手前に質問 2026-09-05）

| 問い | 裁定 |
| --- | --- |
| human presence ガードの材料（「人がタイプしたか」の証拠）をどこから得て、誰が判断するか（Issue #72 の CQRS 境界の裁定） | **A′**: 材料は 2 種類あり性質が違う。(1) **直近のゲート解決の時刻**は集約 `IntentExecution` 自身のイベント（`GateApproved` / `GateRejected` / `AutonomyModeSet(autonomous)`）なので**集約が自分の歴史から状態として持つ**（規則 4「コマンド側の最新状態は常に集約から」。監査シャードは投影＝遅延するリードモデルであり、そこから読むのは禁止パターン）。(2) **`HUMAN_TURN`（人がタイプした事実）**はフックがシャードに直接書く一次の事実で、我々のイベントの投影ではない（シャードが唯一の記録）— b49 のドラフトやメモリ層と同じ**外部の入力**として、合成ルートが読んで値オブジェクト `HumanTurns` にし、集約コマンドへ**引数で渡す**（aggregate-references「材料は引数」）。判断（昇格の可否）は集約のガード（I11）。同一秒のタイはシャードを問わず fail-closed（upstream より少し厳しいだけで、緩くはならない）。`QUESTION_ANSWERED` / `SUMMARY_CONFIRMATION_RECORDED`（`aidlc-log answer` 由来の解決）は、その動詞がイベントとして配線されるまで解決集合に入れない（逸脱台帳） |
| 受理集合 | **A: upstream に揃える**。`switch_autonomy` の状態ガード（`NotRunning`）を外し、park 中・完了後でも切替を受理する（`--single` / review / promote と同じ受理集合）。Quint `actSetAutonomy` の `status == Running` も外し、ITF フィクスチャを再採取する |

用語（初見向け）: **autonomy（自律モード）** = Construction の残り Bolt を人間のゲート承認なしで進めるモード。**昇格** = gated → autonomous（以後のゲートを全部自動で通す強い権限の付与）、**降格** = その逆。**human presence ガード** = 昇格のときだけ「直近のゲート解決（承認・差し戻し・自律付与）より後に本物の人間がタイプしたか」を確かめる仕組み（仕様 I11）。**HUMAN_TURN** = ハーネスのフック（`aidlc-record-human-turn.ts`）がプロンプト送信のたびに監査シャードへ追記する「人が居た」という行（本文は持たない）。**ladder prompt** = walking skeleton の Bolt 1 承認直後に 1 回だけ出る「残りの Bolt をどう進めるか」の質問で、その回答を `set-autonomy` が記録する。

## 1. 原則からの導出

- **コマンド側 = 集約と判断**。「直近のゲート解決」は集約自身の遷移なので状態 `last_gate_resolution_at` として持つ（ジャーナルが真実源、投影の遅延に影響されない）。昇格の可否は `switch_autonomy` の FSM ガードに閉じる。
- **外部の事実は合成ルートが読み、値オブジェクトで渡す**。`HUMAN_TURN` 行の読取はフックが書いたファイルの読取であり、`PracticesPromotion::plan` にドラフトを渡した b49 と同型。合成ルートは判断しない（読んで `HumanTurns::find_in` に渡すだけ）。
- **同一秒のタイは fail-closed**。upstream は同一シャード内の追記位置で順序を決めるが、我々の解決時刻はジャーナルの発生時刻（位置を持たない）なので、秒が等しければ「後」と証明できない → 拒否側に倒す。緩くなる方向の差は無い。
- **RMU は変更なし**。`AutonomyModeSet` の投影（監査行 `AUTONOMY_MODE_SET { Mode }` + 状態ファイル `Construction Autonomy Mode`）は既にある。
- **`aidlc-bolt` は新しい面 `Face::Bolt`**。`set-autonomy` だけを配線し、他の 7 動詞（`start` / `complete` / `fail` / `abort` / `dispatch-event` / `hold-merge` / `release-merge` — slice 2）は not-wired 拒否。
- **繰延（記録して進む）**: `QUESTION_ANSWERED` / `SUMMARY_CONFIRMATION_RECORDED` の解決集合入り（`aidlc-log answer` / `decision` の配線と同時）、同一シャード内の位置による同秒判定、Bolt の他 7 動詞、`--project-dir` 以外の upstream 共通処理。

## 2. ドメイン（`modules/core/command/domain`）

### 2.1 `workspace::HumanTurns`（値オブジェクト、新規 `human_turns.rs`）

監査台帳から読み取った「人が居た」証拠。`{ latest: Option<DateTime<Utc>>, tracked: bool }`。

- `HumanTurns::find_in(buffer: &str) -> HumanTurns`（唯一の構築経路 — `OrderedAuditEvents::find_in` と同型）: 連結バッファ（`read_all` の出力）を `OrderedAuditEvents::find_in` で読み、
  - `tracked` = `DOCUMENT_INDEXED` / `DOCUMENT_UPDATED` / `DOCUMENT_REMOVED` **以外**のイベントが 1 つでも在るか（upstream `sawPresenceTrackingEvent` — DocumentKB の来歴行は presence 追跡を有効にしない）。
  - `latest` = `HUMAN_TURN` 行のうち最新の秒精度タイムスタンプ（`YYYY-MM-DDTHH:MM:SSZ` として `DateTime<Utc>` に読めた行だけ。読めない行は無視）。
- クエリ: `latest() -> Option<DateTime<Utc>>`、`is_tracked() -> bool`。
- `Default`（`latest: None, tracked: false` = 「台帳が無い」）は ITF 準拠テストと既存テストの駆動用。合成ルートは必ず `find_in` で組む（doc に明記）。

### 2.2 `IntentExecution` の状態・クエリ・コマンド・適用

- 状態に `last_gate_resolution_at: Option<DateTime<Utc>>`（genesis は `None`）。`apply_event` で `GateApproved` / `GateRejected` / `AutonomyModeSet(Autonomous)` を適用したとき `Some(occurred_at)` に更新する（`mutate` に `occurred_at` を渡すか、`apply_event` 側で変種を見て更新する — 実装者の判断）。upstream の解決集合のうち我々のイベントに在る 3 つ（`AUTONOMY_MODE_SET` は `Mode == autonomous` のときだけ — 降格は解決ではない）。スナップショット DTO・完全コンストラクタ（引数追加、`Option<DateTime<Utc>>`、欄不在は `None`）・再構成に載せる。
- クエリ `human_acted_since_gate(&self, turns: &HumanTurns) -> bool`（B9 の述語 — orchestration の所有、集約のクエリメソッド。upstream `humanActedSinceGate` の写し、位置比較を秒比較に置き換えたもの）:
  1. `!turns.is_tracked()` → `true`（presence 追跡が無い台帳は fail-open — upstream `events.length == 0 → !sawPresenceTrackingEvent`）。
  2. `turns.latest()` が `None` → `false`（HUMAN_TURN が 1 つも無い）。
  3. `self.last_gate_resolution_at` が `None` → `true`（解決が無ければ最新の人間の行が後）。
  4. 秒精度で比較: `turn > resolution` → `true`、`turn < resolution` → `false`、**等しい → `false`**（fail-closed。upstream は同一シャードの位置順で決めるが、我々は位置を持たない）。`resolution` は `last_gate_resolution_at` を秒に切り捨てて比べる。
- コマンド `switch_autonomy(&mut self, intent: &Intent, mode: AutonomyMode, turns: &HumanTurns, human_presence_guard: bool, occurred_at) -> Result<IntentExecutionEvent, CommandError>`（署名変更）:
  1. 取り違え → `IntentMismatch`。
  2. **状態ガードは無い**（裁定 A: Completed / park 中 / autonomous でも受理）。
  3. `mode == Autonomous && human_presence_guard && !self.human_acted_since_gate(turns)` → `CommandError::HumanPresenceRequired`。降格は無条件。
  4. 受理 → `AutonomyModeSet { mode }`（イベントは変更なし）。適用: `autonomy = mode`、autonomous なら `last_gate_resolution_at = Some(occurred_at)`（付与がその人間の turn を消費する — upstream の "AUTONOMY_MODE_SET only counts when its Mode is autonomous"）。
- `CommandError` 新変種 1: `HumanPresenceRequired`（材料なし — 逐語は app）。
- `report_request.rs` の `human_presence_guard` は変更なし（report の段の `--user-input` 要求は別の仕組み）。

## 3. Quint（`formal/orchestration/engine_loop.qnt`、v2.7）

- `actSetAutonomy` から `status == Running` を外す（裁定 A）。他は不変。ヘッダに v2.7 の説明（受理集合の変更、presence ガードは E2+E3 の射程でモデル外 — I11 の表のとおり）。
- 不変条件・witness の追加は無し（新しい不変条件が無いので mutation の追加も無い）。既存の不変条件 19 本が green のままであること、`actPark` の `not(autonomous)` ガードとの相互作用（autonomous に切り替えた後 park できない、park 中に autonomous へ切り替えられる）が既存の不変条件で壊れないことを `scripts/quint-gate.sh` で確認する。
- ITF フィクスチャ 14 本を b49 §9 と同じコマンドで再採取（`actSetAutonomy` の受理集合が広がるので経路が変わりうる）。`engine_loop_conformance.rs` の `set_autonomy` 駆動は `switch_autonomy(&intent, mode, &HumanTurns::default(), false, at())`（モデルは presence を持たない）。

## 4. ユースケース（`modules/core/command/use-case`）

- 新規 `SwitchAutonomyUseCase<E, I>`（`aidlc-bolt set-autonomy`）: 入力 `AutonomySwitchRequest { mode: AutonomyMode, turns: HumanTurns, human_presence_guard: bool }`。定型 3 手（find execution → find intent → `switch_autonomy` → store）+ 楽観競合 1 回再試行（`PromotePracticesUseCase` と同型）。定義は引かない。CQS: 成功は `Ok(())`。`SwitchAutonomyError { Repository, IntentRepository, Command(CommandError) }`。
- `park_use_case.rs` 等の既存テストで `switch_autonomy` を呼ぶ箇所は新署名（`&HumanTurns::default(), false`）へ。

## 5. RMU（`modules/core/read-model-updater`）

- 変更なし（`autonomy_mode_set` は監査行 `AUTONOMY_MODE_SET { Mode }` と `Construction Autonomy Mode` の `set_field` を既に描く。`read_execution.autonomy` 列も既存）。
- command 側スナップショット DTO に `last_gate_resolution_at: Option<DateTime<Utc>>`（欄不在は `None`）。RMU 側の DTO はイベントの形が変わらないので変更なし。

## 6. app（`modules/app/aidlc`）

- `cli/face.rs`: `Face::Bolt`（`aidlc-bolt`）。`cli/request.rs`: `(Face::Bolt, Some("set-autonomy"))` → `Request::BoltSetAutonomy(SetAutonomyArgs)`；`start` / `complete` / `fail` / `abort` / `dispatch-event` / `hold-merge` / `release-merge` → `Request::BoltNotWired { verb }`（own wording、b48/b49 と同型: `Cannot run aidlc-bolt <verb>: the <verb> subcommand is not wired in this build. Only \`set-autonomy\` is available.`）；未知 → stderr `Unknown subcommand: <sub>. Valid: start, complete, fail, abort, set-autonomy, dispatch-event, hold-merge, release-merge`（`:908` 逐語。sub が無ければ `undefined`）。
- `cli/set_autonomy_args.rs`: upstream `parseFlags`（`:60-77`）の写し — `--x` はすべて値必須（真偽フラグ無し）。値欠落 2 形は `--stage` と同じ逐語（`<flag> expects a value, got end of arguments.` / `<flag> expects a value, got another flag: "<val>". Did you forget the value?` — `wording::flag_expects_a_value*` を再利用）。`--mode` の生値を運ぶ。`--project-dir` は `Invocation` が剥がす。
- `runtime::set_autonomy`（順序は `handleSetAutonomy` どおり、拒否はすべて stderr + exit 1）:
  1. フラグ文法 → `Missing --mode <autonomous|gated>`（`:806` 逐語）→ `Invalid --mode: <m>. Must be 'autonomous' or 'gated'.`（`:808` 逐語、`AutonomyMode::parse` → `InvalidModeArg` — b26 以来消費者の無かった境界型の着地）。
  2. 実行カーソル不在 → own wording `Cannot resolve the active intent for the autonomy switch.`；読めない・壊れているは `unreadable_execution_cursor`。
  3. `catch_up_before_reading`（HUMAN_TURN はフックが書くので投影とは無関係だが、状態ファイルの欄検査を最新で行う）。
  4. 監査シャードを読む: `core_read_model_updater::workspace::read_all(audit_dir)`（RMU の読取ヘルパ — 合成ルートは RMU に依存してよい）→ `HumanTurns::find_in(&buffer)`。
  5. 状態ファイルの欄検査（upstream `setFieldStrict` を書込前に通す形の写し — 構文段）: `- **Construction Autonomy Mode**:` 行が無ければ `State update failed: Field not found in state file: "Construction Autonomy Mode". Cannot update — refusing to silently no-op.`（M12 修正後は scaffold が欄を書くので、手編集で消したときだけ到達）。
  6. `SwitchAutonomyUseCase`（`human_presence_guard = human_presence_guard()` — 既存の env 判定 `AIDLC_SKIP_HUMAN_PRESENCE_GUARD != "1"`）。拒否の逐語: `HumanPresenceRequired` → `Refusing to switch Construction to autonomous: a real human has not acted since the last gate resolution, and autonomous mode is granted only by the human's ladder-prompt answer (it waives every later gate, so the grant itself needs a fresh human turn). Ask the human to confirm autonomous mode in a typed message, then retry. Do not log the ladder choice via aidlc-log answer; the choice is recorded by set-autonomy itself.`（`:824-830` 逐語）；その他（リポジトリ失敗等）→ own wording `Failed to switch autonomy: <材料>`（upstream は `error(errorMessage(e))` の素通し）。
  7. `catch_up` → stdout JSON 1 行（canon-json `ContractCompact`）: `{"emitted":"AUTONOMY_MODE_SET","mode":"<mode>","state_updated":true}`（`:852-857` の鍵順）。
- `runtime.rs` の面の表と `cli/mod.rs` の doc 表に `aidlc-bolt` を足す。

## 7. テスト（TDD、層ごとに red → green）

- ドメイン: `HumanTurns::find_in`（HUMAN_TURN 無し / 最新の選択 / 読めないタイムスタンプの無視 / DOCUMENT_* だけの台帳は untracked / 空バッファ）；`human_acted_since_gate` の表（untracked → true、turn 無し → false、解決無し → true、後 / 前 / 同秒）；`switch_autonomy` の拒否（取り違え、昇格 + ガード + 不在）と受理（降格は不在でも通る、guard=false は通る、park 中・完了後・autonomous 中でも通る）；適用で `last_gate_resolution_at` が動く事実 4 種（`GateApproved` / `GateRejected` / autonomous 付与で更新、gated への降格・他のイベントでは不変）；再構成で復元、欄不在の読み。
- Quint: ゲート全緑、ITF 14 本の準拠（アクション網羅 23 のまま）。
- ユースケース: 3 手・競合再試行・拒否の伝播。
- interface-adapter: スナップショット `last_gate_resolution_at` の往復と欄不在。
- app: パーサ、逐語カタログ、`Workspace` ハーネスで end-to-end — 昇格の拒否（HUMAN_TURN 無し → 逐語）、シャードに HUMAN_TURN 行を追記してから昇格 → JSON 1 行 + 監査行 `AUTONOMY_MODE_SET` + 状態ファイルの `Construction Autonomy Mode: autonomous`、承認（解決）の後に古い HUMAN_TURN しか無ければ拒否、降格は常に通る、`AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1` で通る、`--mode` 2 形の逐語、欄欠落の逐語、park 中でも切替が通る、未知 / not-wired 動詞。
- ゴールデン（`cli/*`）に set-autonomy は無い（b46 注記: upstream の実行出力として採れていない）— 回帰で確認。
- カバレッジ相対ゲート（base ≧ 99.13%）を割らない。

## 8. 仕様・記録

- `docs/specs/10-orchestration.md`: §2.3 の `human_acted_since_gate` 行を「集約 `IntentExecution` のクエリメソッド（材料 `HumanTurns` は引数、解決時刻は集約の状態）」へ改訂；§6 I11 の E 欄を更新（E2+E3 — `switch_autonomy` のガード + 単体 / end-to-end。同秒 fail-closed はシャードを問わない）；§3 ユースケースに `SwitchAutonomy`；§9 に v2.7；§10 に b50 の実装ノート（裁定 A′ / A、材料の 2 分類）。B9 の記述に「HUMAN_TURN は外部入力として合成ルートが読む」を追記。
- `docs/specs/11-workspace.md`: B9 の供給面（shard 列挙と位置付き読取）に「b50 では `read_all` の連結バッファ → `HumanTurns::find_in`」を追記。
- `docs/specs/deviations.md` #7: `QUESTION_ANSWERED` / `SUMMARY_CONFIRMATION_RECORDED` の解決集合入り繰延、同秒タイの fail-closed（同一シャードの位置順は使わない）、Bolt の他 7 動詞の not-wired、`Cannot resolve the active intent for the autonomy switch.` / `Failed to switch autonomy: …` の own wording。
- `handoff-b50.md`、Issue #7 キュー 6 の本文と Issue #72 の close（`Closes #72`）は依頼者が行う。

## 9. 検証記録（2026-09-05 実測、実装は Opus サブエージェント、統合レビューは Fable 5）

### 変更ファイル

**新規 5 本**

| 層 | ファイル |
| --- | --- |
| domain | `modules/core/command/domain/src/workspace/human_turns.rs`（`HumanTurns`） |
| use-case | `modules/core/command/use-case/src/orchestration/autonomy_switch_request.rs`（入力 VO） |
| use-case | `modules/core/command/use-case/src/orchestration/switch_autonomy_use_case.rs` |
| use-case | `modules/core/command/use-case/src/orchestration/switch_autonomy_error.rs`（封筒） |
| app | `modules/app/aidlc/src/cli/set_autonomy_args.rs`（upstream `parseFlags` の写し） |

**変更 25 本**（ITF フィクスチャ 14 本を除く）

| 層 | ファイル | 変更 |
| --- | --- | --- |
| domain | `orchestration/intent_execution.rs` | 状態 `last_gate_resolution_at` / アクセサ / 完全コンストラクタの引数 / genesis / `mutate` の 3 変種 / クエリ `human_acted_since_gate` / `switch_autonomy` の署名 + 状態ガード撤去 |
| domain | `orchestration/command_error.rs` | `HumanPresenceRequired`（材料なし） |
| domain | `workspace/audit_event_record.rs` | `instant()`（秒精度 ISO の解釈を行の持ち主へ） |
| domain | `workspace/mod.rs` | `HumanTurns` の `pub use` |
| domain (test) | `tests/engine_loop_conformance.rs` | `set_autonomy` 駆動を新署名へ |
| use-case | `orchestration/mod.rs` | 3 型の `pub use` |
| use-case | `orchestration/park_use_case.rs` | テストの呼出を新署名へ |
| ia | `orchestration/dto/intent_execution_dto.rs` | `last_gate_resolution_at`（`#[serde(default)]`、欄不在は `None`） |
| ia (test) | `orchestration/dto/tests.rs`、`tests/support/contract.rs` | ゴールデン更新 + 往復 2 本 |
| RMU | `read_tables/spelling.rs` | `jump_refusal` に `human-presence-required`（17 変種） |
| RMU (test) | `tests/read_tables_test.rs`、`tests/support/mod.rs` | 新署名へ |
| app | `cli/face.rs` / `cli/mod.rs` / `cli/request.rs` | `Face::Bolt`、面の表、`Request` 3 変種 + 認識 7 動詞 |
| app | `wording.rs` | Bolt 面の逐語 8 本 |
| app | `runtime.rs` | `set_autonomy`（7 段）/ `audit_ledger` / `autonomy_field_guard` / `switch_autonomy_refusal` / `set_autonomy_line` |
| app (test) | `tests/intent_lifecycle.rs`、`tests/crash_reconstruction_test.rs` | e2e 13 本 + 新署名へ |
| formal | `formal/orchestration/engine_loop.qnt` | v2.7（`actSetAutonomy` の `status == Running` 撤去） |
| 仕様 | `docs/specs/10-orchestration.md` / `11-workspace.md` / `deviations.md` | §2.3・§3・§6 I11・§9・§10・B9、B9 供給面、逸脱 #7 |

### ゲートの実測値

- `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `cargo test --workspace` — **全緑（49 スイート、2,118 本）**。`tools/lint` 自己テスト 69 本も緑。`cargo doc --workspace --no-deps` warning 0
- 新規テスト **51 本**（`#[test]` / `#[tokio::test]` の増分。起点は b49 の 2,067 本）。内訳: 新規 5 ファイルに 21 本（`HumanTurns` 6 / `SetAutonomyArgs` 6 / ユースケース 6 / 封筒 2 / 入力 VO 1）、既存ファイルに 30 本（集約の述語・受理集合・解決時刻の適用 10、`AuditEventRecord::instant` 1、DTO 往復と欄不在 2、`Request` のルーティング 2、逐語カタログ 1、app の end-to-end 14）
- `scripts/quint-gate.sh` — **全 PASS（25 ステップ。engine_loop の不変条件 19 本、witness 9 本のまま）**
- ITF フィクスチャ 14 本を再採取（採取コマンドは下記）。状態数: 0x101 = 35、0x202 = 41、0x303 = 3、0x404 = 2、0x505 = 3、0x606 = 34、0x707 = 39、0x808 = 34、0xa1 / 0xb2 / 0xc3 / 0xd4 / 0xe5 / 0xf6 = 41。アクション網羅 23 のまま
- `scripts/coverage.sh --base origin/main` — 絶対 **99.1304% ≥ 90.0% PASS**、相対 **99.1304% ≥ 99.1309 − 0.01 PASS**

### ITF フィクスチャの採取コマンド（`M=formal/orchestration/engine_loop.qnt`、`D=tests/conformance/fixtures/engine_loop`、いずれもリポジトリルートから）

b49 §9 と**同じコマンド**である（14 本すべて再採取した）。

```
for s in 0xa1 0xb2 0xc3 0xd4 0xe5 0xf6 0x202; do quint run $M --seed $s --max-samples 1 --max-steps 40 --out-itf $D/trace-$s.itf.json; done
quint run $M --seed 0x101 --max-samples 2000 --max-steps 40 --invariant 'not(lastAction == "report_revised")' --out-itf $D/trace-0x101.itf.json
quint run $M --seed 0x303 --max-samples 2000 --max-steps 40 --invariant 'not(w_repark)'             --out-itf $D/trace-0x303.itf.json
quint run $M --seed 0x404 --max-samples 2000 --max-steps 40 --invariant 'not(w_single_run)'         --out-itf $D/trace-0x404.itf.json
quint run $M --seed 0x505 --max-samples 2000 --max-steps 40 --invariant 'not(w_stance_recorded)'    --out-itf $D/trace-0x505.itf.json
quint run $M --seed 0x606 --max-samples 2000 --max-steps 40 --invariant 'not(w_approved_reviewed)'  --out-itf $D/trace-0x606.itf.json
quint run $M --seed 0x707 --max-samples 2000 --max-steps 40 --invariant 'not(w_retry_review)'       --out-itf $D/trace-0x707.itf.json
quint run $M --seed 0x808 --max-samples 2000 --max-steps 40 --invariant 'not(w_approved_practices)' --out-itf $D/trace-0x808.itf.json
```

### 設計との差分（実装で受け入れたもの）

1. **秒精度 ISO の解釈は `AuditEventRecord::instant()` が持つ** — 設計 §2.1 は `HumanTurns::find_in` が「読めた行だけ」を選ぶとだけ書いていたが、逐語のタイムスタンプ文字列を時刻に解釈するのは**行の持ち主**の仕事である（`coding-rules/domain-services.md`「導出はまず所有する型の関連メソッドへ」）。読み手ごとに書式（`%Y-%m-%dT%H:%M:%SZ`）を書き写さないよう、変換を行の型に 1 か所だけ置いた。
2. **`mutate` が `occurred_at` を受け取る** — 設計 §2.2 が実装者の判断に委ねた 2 択のうち前者を採った。`last_gate_resolution_at` は「変種ごとに立つかどうかが決まる状態」であり、`last_updated_at`（適用の共通後始末）とは性質が違うので、写す場所は適用の中（`mutate`）である。
3. **`human_acted_since_gate` は `const fn`** — workspace lints の `missing_const_for_fn` が deny なので、宣言をそれに合わせた（意味は変わらない）。
4. **RMU の読取ヘルパの綴りは `read_all_audit_shards`** — 設計 §6 は `core_read_model_updater::workspace::read_all` と書いていたが、ファサードの `pub use` は `read_all as read_all_audit_shards` である（媒体名が消えないよう改名して公開されている）。合成ルートはその綴りで呼ぶ。
5. **`CommandError` の新変種は `jump_refusal` にも綴りが要る** — `read_tables/spelling.rs` の閉集合 match は「起きないはずの値をどれかに寄せない」ため全変種を綴るので、`human-presence-required` を足した（17 変種、綴りの一意性テストも更新）。設計には書かれていないが、変種を足せばビルドが要求する。
6. **面の表は `cli/mod.rs` にしかない** — 設計 §6 は「`runtime.rs` の面の表と `cli/mod.rs` の doc 表」と書いていたが、実測では `runtime.rs` のモジュール doc に面の表は無い（あるのは `cli/mod.rs` の 1 枚だけ）。そちらに `aidlc-bolt` 行を足した。
7. **`AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1` の end-to-end は組めない** — 設計 §7 の app テスト一覧にあるが、プロセス内で env を差し替えるには `unsafe` が要り、workspace lints が `unsafe_code` を forbid している（既存の段 13 テストが同じ理由で集約の単体テストに寄せているのと同じ制約）。ガードが外れた昇格は集約の単体テスト `a_disabled_guard_lets_the_escalation_through` が固定する。
8. **「ストアが開けない」ではなく「居ない実行」で中継形を踏む** — 設計 §7 の失敗経路のうち、`journal` 表を落とす形は投影（`catch_up_before_reading` より後の `catch_up`）が先に倒れるので `Failed to switch autonomy:` へ到達しない。b49 と同型に、実行カーソルを居ない実行へ向けて `repository:` の材料を運ばせる形にした。
9. **失敗経路の app テストを 4 本足した（カバレッジ相対ゲートの回復）** — 最初の計測で head 99.1098% と base − 0.01 を 0.021pp 下回った。未カバーだったのは `set_autonomy` の失敗経路（実行カーソル破損・状態ファイルの読取失敗・投影失敗・空間名不正）だったので、b49 の同型テストに倣って 4 本足した。
10. **park 中の受理を既存テストで固定し直した** — `every_command_but_park_and_unpark_is_refused_while_parked` から `switch_autonomy` を外し、「park 中でも切替は通り、park マーカーは残る」を同じテストの中で表明した（受理集合の変更を既存テストが黙って通さないようにするため）。

### 統合レビュー（Fable 5、2026-09-05）

- 差分 39 ファイル + 新規 5 ファイルを全読。裁定 A′（解決時刻は集約の状態、`HUMAN_TURN` は外部入力、判断は集約のガード）と裁定 A（受理集合は upstream）が設計どおり実装されていることを確認し、上の差分 10 点をすべて受け入れた（いずれも規律の帰結か実測合わせ）。
- ゲートを再計測: fmt / clippy / `cargo lint` / `cargo test --workspace`（49 スイート 2,118 本）/ `tools/lint` 69 本 / `cargo doc` warning 0 / Quint ゲート 25 ステップ PASS。
- ITF フィクスチャ 14 本を上の採取コマンドで再採取し、`#meta` を除いてバイト一致を確認（新しい不変条件は無いので mutation の追加は無し）。

