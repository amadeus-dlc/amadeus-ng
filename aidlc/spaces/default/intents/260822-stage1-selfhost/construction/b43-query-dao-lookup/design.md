# b43 設計 — Bolt 3 前半: クエリ側を「DAO 引当 → View」に縮め、`next` / `continue` を `read_*` 上で成立させる（2026-09-03）

**裁定**: オーナー 2026-09-02（クエリ側ユースケースは DAO で View を読んで返すだけ / DAO はキー引当のみ / スコープ解決順はコントローラのルーティング /
文言はプレゼンタ）、2026-09-03 = B（Bolt 3 は b43 → b44 の 2 本、lint 込み）。**正本**: cqrs-boundaries 規則 6（2026-09-02 追記）、gateway-taxonomy §3（`XxxDao` / `find`）、
use-case-rules §4（クエリ側はポート注入）、仕様 11 §4.1（17 表）。**調査**: `bolt3-design-draft.md`（21 分岐ラダーの分類、continue の 4 照合、token DTO、lint 流儀）。

## 0. 原則からの導出
1. クエリ側ユースケース = `execute(key) = dao.find(key) → Option<View>`。判断・導出・選択・文言組立を持たない。
2. DAO はキーによる引当だけ（`WHERE` = 要求パラメータ。キー結合の JOIN は可）。行に無い事実を作らない。
3. 要求の**形**（フラグ・本文の語数・token の有無）で決まる分岐はコントローラ（app）。状態の**値**で決まる分岐は RMU が行に書いた `kind` に従ってプレゼンタが描く。
4. 逐語文言・directive JSON・token の封緘 / 開封・パスの絶対化（Layout の dir 前置）はプレゼンタ（app）。
5. 1 要求種別 = 1 引当 = 1 View（非正規化）。View は基本データ型の値で、判断メソッドを持たない。

## 1. アクティブな実行の解決（合成ルートの機構）
- 実行カーソル `<record>/.aidlc-execution`（1 行目 `execution_id`、2 行目 `intent_id`。gitignore 済み `aidlc/spaces/*/intents/*/.aidlc-*`）を
  `mint_intent` が書く（機構モジュール `execution_cursor.rs`、`clone_identity.rs` と同じ流儀）。`Layout::execution_cursor()` で読む。
- `next` / `continue` / `report` / `park` はこれをキーにする。カーソル無し = state なし群。`report` の `active_execution`（ジャーナル先頭決め打ち）は撤去。
- `definition_id` は `harness_name()` の固定値 `"claude"`（`read_intent.definition_id` と一致）。

## 2. ポート（query use-case `port/`、trait は `XxxDao` + `find`。DTO/View と読取エラーは同居）
| ポート | キー | 返す View（`read_*` の JOIN） | 使う分岐 |
| --- | --- | --- | --- |
| `NextAnswerDao::find(execution_id, request_kind)` | `read_next_answer` | `NextAnswerView` = decision_kind / stage_index / stage_slug / gated / checkbox + `read_execution`（scope, cursor_slug, parked_at_slug, status, state_binding）+ `read_intent`（definition_id, scope）+（decision が run-stage のとき）`read_run_stage`（definition_id × intent.scope × stage_slug の全列）+ `read_steering_plan`（phase）+ `read_steering_part`（phase, 1） | 2.5 / 2.6 / 6 / 9c / 10 / 10-deliver |
| `ContinuationDao::find(state_binding?, route_digest, directive_digest, bundle_digest, part_index)` | 等値照合 | `ContinuationView` = 該当 `read_run_stage` 行 + `read_steering_plan`（part_count, delivered_paths）+ `read_steering_part`（part_index）or 終端 | continue |
| `ExecutionDao::find(execution_id)` | `read_execution` × `read_intent` | `ExecutionView`（scope, definition_id, cursor_slug, status, parked） | state の有無・5-a の前提 |
| `ScopeDao::find(definition_id, scope)` / `find_stock(definition_id)`（stock 3 scope の行） | `read_definition_scope` | `ScopeView`（depth, keywords, cost_*） | 3b / 4 / 4a / 4c / 8' / 9a |
| `ScopeKeywordDao::find(definition_id, keyword)` | `read_definition_scope_keyword` | scope 名 | 8 |
| `RunStageDao::find(definition_id, scope, stage_slug)` | `read_run_stage` | `RunStageView` | 4b（`--single`）、state なし jump |
| `JumpDao::find(execution_id, target_slug)` / `find_phase(execution_id, phase)` | `read_next_jump` / `read_next_jump_phase` | `JumpView`（outcome, refusal, target） | 7 |
| `PhaseEntryDao::find(definition_id, scope, phase)` | `read_definition_scope_phase_entry` | first_stage_slug | 7（state なし） |
| `ScopeChangeDao::find(execution_id, scope)` | `read_scope_change` | kind | 5-a |
| `DefinitionDao::find(definition_id)` | `read_definition` | revision / stage_count | 定義未取込の検出（無ければ NO_STATE 系） |
読取エラーは 1 本 `ReadModelReadError { kind, path }`（SQLite 失敗）。upstream 逐語の定義読取失敗文言（12 §4）は b44 で golden を配線し直す際に「定義未取込」の扱いとして整理（プレゼンタ）。

## 3. クエリ側の実装（interface-adapter）
- `rusqlite` を追加。各 `XxxDaoImpl::open(&StorePath)`（read-only、`busy_timeout`）。`InMemoryXxxDao` は行を保持するフェイク。
- SQL は表と列名だけを知る（JOIN はキー結合）。Markdown 逆パース・配布 3 ファイルのパースは**使わない**（削除は b44）。

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
