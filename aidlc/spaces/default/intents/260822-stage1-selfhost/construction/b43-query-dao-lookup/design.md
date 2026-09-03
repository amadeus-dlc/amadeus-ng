# b43 設計 — Bolt 3 前半: クエリ側を「DAO 引当 → View」に縮め、`next` / `continue` を `read_*` 上で成立させる（2026-09-03）

**裁定**: オーナー 2026-09-02（クエリ側ユースケースは DAO で View を読んで返すだけ / DAO はキー引当のみ / スコープ解決順はコントローラのルーティング /
文言はプレゼンタ）、2026-09-03 = B（Bolt 3 は b43 → b44 の 2 本、lint 込み）。**正本**: cqrs-boundaries 規則 6（2026-09-02 追記）、gateway-taxonomy §3（`XxxDao` / `find`）、
use-case-rules §4（クエリ側はポート注入）、仕様 11 §4.1（17 表）。**調査**: `bolt3-design-draft.md`（21 分岐ラダーの分類、continue の 4 照合、token DTO、lint 流儀）。

## 0. 原則からの導出
1. クエリ側ユースケース = `execute(key) = dao.find(key) → Option<View>`。判断・導出・選択・文言組立を持たない。
2. DAO はキーによる引当だけ（`WHERE` = 要求パラメータ）。**1 表 1 引当 — JOIN も副問合せも非正規化の焼き込みもしない**。行に無い事実を作らない。
   **改訂 2026-09-03（裁定 — JOIN 解体）**: 初稿の「キー結合の JOIN は可」はオーナー裁定 2026-09-03（`project.md` Corrections 最終行 / `cqrs-boundaries.md` 規則 6「表の形と読み方」）の逐語「JOIN しない」と衝突するため撤回した。関連行は FK 列で指し、**ユースケースが FK をたどって表ごとに引き、組み立て View を返す**（判断は無い — null の FK は「無し」）。
3. 要求の**形**（フラグ・本文の語数・token の有無）で決まる分岐はコントローラ（app）。状態の**値**で決まる分岐は RMU が行に書いた `kind` に従ってプレゼンタが描く。
4. 逐語文言・directive JSON・token の封緘 / 開封・パスの絶対化（Layout の dir 前置）はプレゼンタ（app）。
5. **1 表 = 1 DAO = 1 View（行の写し）**。View は基本データ型の値で、判断メソッドを持たない。
   **改訂 2026-09-03（裁定 — JOIN 解体）**: 初稿の「1 要求種別 = 1 引当 = 1 View（非正規化）」を撤回。1 要求は複数の引当になりうる
   （`next` は最大 5、`continue` は最大 3、`--phase` ジャンプは 2）。まとめるのは組み立て View（`NextTurnView` / `ContinuationView`）で、
   これはユースケース側（`orchestration/`）に置く — DAO が返す型ではないので `port/` の住人ではない。

## 1. アクティブな実行の解決（合成ルートの機構）
- 実行カーソル `<record>/.aidlc-execution`（1 行目 `execution_id`、2 行目 `intent_id`。gitignore 済み `aidlc/spaces/*/intents/*/.aidlc-*`）を
  `mint_intent` が書く（機構モジュール `execution_cursor.rs`、`clone_identity.rs` と同じ流儀）。`Layout::execution_cursor()` で読む。
- `next` / `continue` / `report` / `park` はこれをキーにする。カーソル無し = state なし群。`report` の `active_execution`（ジャーナル先頭決め打ち）は撤去。
- `definition_id` は `harness_name()` の固定値 `"claude"`（`read_intent.definition_id` と一致）。

## 2. ポート（query use-case `port/`、trait は `XxxDao` + `find`。DTO/View と読取エラーは同居）
**改訂 2026-09-03（裁定 — JOIN 解体）**: **1 表 = 1 ポート = 1 View（行の写し）**。View は `id` と FK 列を運び、
複数の表にまたがる答えはユースケースが FK をたどって組む（組み立て View は DAO の戻り値ではないので `port/` ではなく
`orchestration/` に置く）。ポートは 10 → **12**（`SteeringPlanDao` / `SteeringPartDao` / `JumpPhaseDao` を新設、
`ContinuationDao` を撤去）。

| ポート | キー | 返す View（1 表の行の写し） | 使う分岐 |
| --- | --- | --- | --- |
| `NextAnswerDao::find(execution_id, request_kind)` | `read_next_answer` | `NextAnswerView` = decision_kind / stage_index / stage_slug / gated / checkbox + FK 2 本（execution_id, run_stage_id） | 2.5 / 2.6 / 6 / 9c / 10 / 10-deliver |
| `ExecutionDao::find(execution_id)` / `find_by_state_binding(state_binding)` | `read_execution` | `ExecutionView`（intent_id, scope, cursor_slug, status, parked, state_binding。**definition_id は載せない** — intent の FK をたどる） | state の有無・5-a の前提・continue の state 照合 |
| `SteeringPlanDao::find(id)` / `find_bound(id, bundle_digest)` | `read_steering_plan` | `SteeringPlanView`（id, phase, bundle_digest, part_count, delivered_paths） | 10-deliver / continue |
| `SteeringPartDao::find(steering_plan_id, part_index)`（`FIRST_PART = 1` はポート定数） | `read_steering_part` | `SteeringPartView`（steering_plan_id, phase, part_index, rules_content） | 10-deliver / continue |
| `JumpPhaseDao::find(execution_id, phase)` | `read_next_jump_phase` | `JumpPhaseView`（target_index, target_slug） | 7（`--phase`） |
| `ScopeDao::find(definition_id, scope)` / `find_stock(definition_id)`（stock 3 scope の行） | `read_definition_scope` | `ScopeView`（depth, keywords, cost_*） | 3b / 4 / 4a / 4c / 8' / 9a |
| `ScopeKeywordDao::find(definition_id, keyword)` | `read_definition_scope_keyword` | scope 名 | 8 |
| `RunStageDao::find(definition_id, scope, stage_slug)` / `find_by_id(id)` / `find_bound(definition_id, scope, stage_slug, route_digest, directive_digest)` | `read_run_stage` | `RunStageView`（23 列 — `id` と `steering_plan_id` を含む） | 4b（`--single`）、state なし jump、`next` の FK 追跡、continue |
| `JumpDao::find(execution_id, target_slug)` / `find_by_target(execution_id, target_index)` | `read_next_jump` | `JumpView`（outcome, refusal, target） | 7 |
| `PhaseEntryDao::find(definition_id, scope, phase)` | `read_definition_scope_phase_entry` | first_stage_slug | 7（state なし） |
| `ScopeChangeDao::find(execution_id, scope)` | `read_scope_change` | kind | 5-a |
| `DefinitionDao::find(definition_id)` | `read_definition` | revision / stage_count | 定義未取込の検出（無ければ NO_STATE 系） |
読取エラーは 1 本 `ReadModelReadError { kind, path }`（SQLite 失敗）。upstream 逐語の定義読取失敗文言（12 §4）は b44 で golden を配線し直す際に「定義未取込」の扱いとして整理（プレゼンタ）。

## 3. クエリ側の実装（interface-adapter）
- `rusqlite` を追加。各 `XxxDaoImpl::open(&StorePath)`（read-only、`busy_timeout`）。`InMemoryXxxDao` は行を保持するフェイク。
- SQL は表と列名だけを知る。**改訂 2026-09-03（裁定 — JOIN 解体）**: 1 実装 = 1 表（`FROM read_*` は各ファイル 1 つ、JOIN も副問合せも無い）。
  SQL はすべてコンパイル時の文字列リテラル（`concat!` / `macro_rules!`）で、実行時に `format!` で組まない — 引く表が 1 つであることを
  リテラルの検査（レビューと `cargo lint` の `dao-single-table`）で確かめられるようにするため。
- 組み立てはユースケース側:
  - `next` = `read_next_answer` →(execution_id) `read_execution` →(run_stage_id) `read_run_stage` →(steering_plan_id) `read_steering_plan` →(id, `FIRST_PART`) `read_steering_part` = `NextTurnView`（最大 5 引当）。
  - `continue` = `read_run_stage`（自然キー + route/directive 束縛）→(steering_plan_id + bundle 束縛) `read_steering_plan` →(id, part_index) `read_steering_part` = `ContinuationView`（最大 3 引当）。state 束縛はコントローラが `ExecutionDao::find_by_state_binding` で先に引く（要求の形の分岐 = 構文的ルーティング）。
  - `--phase` ジャンプ = `read_next_jump_phase` →(target_index) `read_next_jump`（2 引当）。
  - 同一トランザクション（ジャーナル由来 15 表）の FK が宙に浮いたら `ReadModelReadError::broken_projection()`（`InvalidData`）。steering の 2 表は別 Tx なので不在は `None`（正常）。
- Markdown 逆パース・配布 3 ファイルのパースは**使わない**（削除は b44）。

## 4. app（コントローラ / プレゼンタ）
- `cli/request.rs` → `NextTurnInput` の分類（前置ガード 2・2・4c・5b・9b・≤5 語のキーワード分割・scope 解決順 state > `--scope` > positional > env > default）。
- `runtime.rs::next` = 分類 → 実行カーソル → 該当 DAO 引当（ユースケース）→ プレゼンタ。`runtime.rs::resume` = token 開封 → `ContinuationDao` → プレゼンタ。
- `presenter.rs` に `next_use_case::wording` / `continue_use_case::wording` / `EngineCommand::cli_spelling` / run-stage の相対パス絶対化 / token 封緘（pins は token の値）を移す。
- 未配線の分岐（Kiro ラッチ / read-only / 名詞トークン / records_without_cursor / env_default_scope）は構文分岐としてコントローラに残す（文言はプレゼンタ）。

## 5. テスト
- DAO 契約テスト（SQLite フィクスチャ: RMU の `advance_checkpoint` / `replace_steering` で行を作り、各 `find` がキーで引けること・無ければ `None`）。
- app e2e（`intent_lifecycle.rs` / `steering_across_processes.rs`）は実 CLI 経路（鋳造 → RMU 投影 → `next` / `continue`）でそのまま。定義読取失敗の逐語 2 本は媒体変更で書き換え。
- golden（directive JSON / continue token の外部観測）が不変であることを固定。

## 6. b44 に残すもの
旧ユースケース・判断型・両パーサ・steering 複製・クエリ側 ITF・`EngineSignal` 複製の削除、`golden_parity_test.rs` の配線し直し、`cargo lint` 2 本（#47）、正本の履歴注記整理。
