# Bolt 3 設計ドラフト — クエリ側縮小（調査 2026-09-03 に基づく）

## 原則（裁定済み）
クエリ側ユースケース = `dao.find(key) → View` だけ。DAO はキーによる引当（**1 表 1 引当**）、行に無い事実を作らない。
**改訂 2026-09-03（裁定 — JOIN 解体）**: 「JOIN もキー結合なら可」は撤回。オーナー裁定 2026-09-03 の逐語は「JOIN しない」であり、
関連行は FK 列で指し、**ユースケースが FK をたどって表ごとに引き、組み立て View（`NextTurnView` / `ContinuationView`）を返す**。
本節以下の JOIN を前提とした記述（設計判断の 1・2 行目、`continue` の等値 JOIN）は同裁定で置き換わり、正は `design.md` §0-2 / §2 / §3 である。
要求の形による分岐（フラグ）はコントローラ（app `cli/request.rs` + `runtime.rs`）、逐語文言・directive JSON・token 封緘はプレゼンタ（app `presenter.rs` + `wording.rs`）。
状態の値で決まる分岐は RMU が行に書いた `kind` に従ってプレゼンタが描く。

## 設計判断（統合側）
- **1 要求種別 = 1 引当 = 1 View**。DAO は `read_next_answer` を起点にキーで JOIN（`read_run_stage`（definition_id × scope × stage_slug）、`read_steering_plan` / `read_steering_part`(part 1)、`read_execution`（state_binding / scope / cursor_slug）、`read_intent`（definition_id / scope））し、プレゼンタがそのまま描ける `NextAnswerView` を返す。分岐はプレゼンタが `decision_kind` で描き分ける。
- `continue` = token の bindings（bundle / directive / route / state）と part_index をキーに `read_run_stage` × `read_steering_plan` × `read_steering_part` × `read_execution` を等値 JOIN → `ContinuationView`（行が無ければ fail-closed 文言）。pins（gate / next_stage / unit / single）は token の値をプレゼンタが載せる。
- state 無し群（compose / new-intent / `--single` / jump / scope 検証 / キーワード引当）は `read_definition_scope` / `read_definition_scope_keyword` / `read_definition_stage` / `read_run_stage` / `read_definition_scope_phase_entry` をキーで引く小さな DAO 群。コントローラが「どの引当をどのキーで」を決める（scope 解決順 = state > --scope > positional > env > default はルーティング順）。
- 未配線の分岐（Kiro ラッチ / read-only / 名詞トークン / records_without_cursor / env_default_scope）は app から呼ばれていない実測 → コントローラ側に**構文分岐として**残す価値があるものだけ移し、残りは削除候補（オーナー確認は不要 — 到達不能コードの整理。ただし upstream 互換の分岐は文言だけプレゼンタに残す）。
- パス: `read_run_stage` の `*_rel` を Layout の各 dir で前置するのはプレゼンタ（`WorkspaceLayout` は app 側の値へ）。
- ポート命名: `XxxDao` + `find(key)`。エラーは SQLite 読取失敗語彙 1 本（`ReadModelReadError { kind, path }`）へ収束（12 §4 の upstream 逐語文言はプレゼンタが行の有無・エラーで描く — 定義ファイルの読取失敗文言は「定義未取込」の状態として `read_definition` の欠如で表す）。

## PR 分割
- **b43**: ポート再設計 + SQLite DAO 実装（rusqlite を query interface-adapter に追加、`store_path(layout)` を read-only で開く）+ View（行の写し）+ ユースケース（`find`）+ app のコントローラ / プレゼンタ（分類・文言・JSON・token）で `next` / `continue` を `read_*` 上で成立させる。app の e2e（`intent_lifecycle.rs` / `steering_across_processes.rs`）は実 CLI 経路で RMU が行を作るので原則そのまま（定義読取失敗の逐語テスト 2 本は媒体変更で書き換え）。
- **b44**: 削除（`next_use_case.rs` 2933 行・`continue_use_case.rs`・`scope_resolution*`・`steering_plan.rs` の pack・`steering_digest.rs`・`bindings.rs`・`engine_signal.rs`・両パーサ 900 + 499 行・`definition_paths.rs`・`raw_artifact.rs`・`test_fixtures.rs`・`engine_loop_ladder_conformance.rs`・`workflow_definition_reading_test.rs` の大半）、golden の配線し直し（`golden_parity_test.rs` は RMU 投影経由で `read_definition*` を読んで同じ数値を pin）、`cargo lint` ルール 2 本（① コマンド側 use-case 層の `pub trait` は `Repository` 終わり、② `modules/core/command/**` の `*_repository_impl.rs` 以外に fs / 乱数 API 禁止 — 赤例テスト付き、`.claude/sensors/` は報告のみ）、正本更新（cqrs-boundaries / gateway-taxonomy / use-case-rules の履歴注記整理、仕様 10 §3、11 §4.1）。

## 調査の要点（保存）
- `next_use_case.rs` 21 分岐: 前置ガード 2・0・1・1b・2・4c・5b・9b は構文（CTRL）。2.5 / 2.6 / 6 / 9c / 10 は `read_next_answer`（request_kind × decision_kind）。3b/4 は scope 解決（CTRL 順 + `read_definition_scope` / `read_execution.scope`）。4a / 9a は `read_definition_scope.cost_*`。4b は `read_run_stage`。5-a は `read_scope_change`。7 は `read_next_jump` / `read_next_jump_phase` / `read_definition_scope_phase_entry`。8 は `read_definition_scope_keyword`（≤5 語は CTRL）。10-deliver は `read_steering_plan` / `read_steering_part`。
- `continue`: 4 本の等値照合（state_binding / route_digest / bundle_digest / directive_digest）+ part_index。逐語 6 本はプレゼンタへ。
- token DTO（18 キー、順序が封緘バイト）と鍵配線（`steering.rs`）は app 側に既にある。
- query interface-adapter に rusqlite が無い（追加必須）。`read_*` は journal と同じ SQLite ファイル（`JournalReaderImpl::open` が DDL を作る）。
- ドメイン側 ITF（`assert_signal`）が観測面を既に担うので、クエリ側 ITF と `EngineSignal` 複製は削除可。
- lint ルールの流儀: `tools/lint/src/check.rs`（`RULE_*` 定数 + `Visitor` / ファイル単位純関数、`Finding`、`is_test_path` / `has_cfg_test` 除外、抑制 `// amadeus-lint: allow(<rule>) — 理由`、テストは `r<N>_detects_… / r<N>_allows_…`）。

## 追記（2026-09-03 調査）— アクティブな実行 id の解決
- 現状: `read_*` は `execution_id` / `intent_id` / `definition_id` でキー付け。しかし record dir（`active-intent` カーソル）→ 実行 id の写像がどこにも無い。
  `report` は `active_execution(store)` = ジャーナル先頭の実行行（単一 intent 前提の決め打ち）。状態ファイル header にも `intents.json` にも実行 id は無い。
  dirName の id8 接尾辞は出荷済み dir（`260822-stage1-selfhost`）に無く、Rust 側に `intents.json` リーダも無い。
- b43 の設計: 合成ルートの**機構**として実行カーソル `<record>/.aidlc-execution`（1 行目 `execution_id`、2 行目 `intent_id`）を `mint_intent` が書き
  （gitignore 済みパターン `aidlc/spaces/*/intents/*/.aidlc-*`。ストア自体も機械ローカルなので整合）、`Layout` が `execution_cursor()` で読む。
  `report` / `next` / `continue` / `park` はこれをキーに使い、ジャーナル先頭決め打ちを撤去。カーソル無し = 「state なし」群（`NO_STATE` 文言）。
  `definition_id` は `harness_name()` の固定値 `"claude"`（`read_intent.definition_id` と一致）。
- これは「advisory マーカーの書込は合成ルートの機構」（裁定 §10-5）と同じ扱い。ドメイン・クエリ側は関与しない。
