# CLI 読取コマンド用リードモデル仕様 — RMU が計算結果を作り、クエリ側は読んで返すだけ（2026-09-02、§10 はオーナー裁定済み）

**位置づけ**: オーナー指示「CLI の読取コマンドがあるなら、何かしらのリードモデルを RMU が作って
おかないといけない。クエリ側で作るのはダメ。SQLite でも JSON でもよい。最適なリードモデル仕様を
考えて」への回答。採否と各裁定点（§10）はオーナー。既存正本との整合は §11。

---

## 1. 責務の線引き（4 者）

| 主体 | 持つもの | 持たないもの |
| --- | --- | --- |
| **集約**（コマンド側 domain） | データ・遷移・**判断**（`next_decision` / `jump_resolve` / `scope_cost` / 述語面 …） | 永続化知識・出力形式 |
| **RMU**（中間） | イベントから集約を `replay` で起こし、判断を**呼んで**答えを計算し、**リードモデルとして書く**。参照入力（memory 規則ファイル）の畳み込みも RMU | 判断の再実装（集約のクエリメソッドを呼ぶだけ）、要求時の入力（フラグ・本文） |
| **クエリ側ユースケース + DAO** | `execute(key) = dao.find(key) → View` を返すだけ。DAO は**キーによる引当**（`WHERE` 相当）だけを行う | 判断・導出・選択・文言組立。**新しい事実を計算しない** |
| **コントローラ / プレゼンタ**（アダプタ・合成ルート） | 要求の**構文的**分類（フラグの有無・本文の語数）→ どの DAO をどのキーで引くかの**ルーティング**。行の `kind` に従って逐語文言 / directive JSON / continue_token に**描く** | 状態を見て決める判断（それは行に書いてある） |

線引きの規則（裁定点 §10-2）: DAO は「要求パラメータで行を引く」ことは許され（GraphQL のリゾルバ
が引数でフィルタするのと同じ）、「行に無い事実を作る」ことは禁じられる。コントローラの分岐は
**要求の形**（フラグの組合せ）だけで決まり、**状態の値**で決まる分岐は一切持たない — 状態依存の
答えは RMU が行に書いてある。

## 2. リードモデルの 2 系統

| 系統 | 読み手 | 形 | 現状 |
| --- | --- | --- | --- |
| (1) 人・upstream ツール向けファイル面 | 人が開く、upstream の hook / ステージが読む | `aidlc-state.md`（Markdown）、監査シャード（Markdown 追記）、配布束の面 `stage-graph.json` / `scope-grid.json` | **現状維持**（RMU が投影中。バイト互換の golden で固定） |
| (2) CLI 読取コマンド向け構造化リードモデル | `next` / `continue` /（将来）`--status` / `doctor` の DAO | **本仕様の新設対象** | 無い（クエリ側が (1) を逆パースして自分で計算している — 監査 audit-1.md §B） |

(2) を新設すると、クエリ側の Markdown 逆パース（`execution_state_parse.rs`）と配布 3 ファイルの
パース（`workflow_definition_parse.rs`）は不要になる。(1) は upstream 互換のためだけに残る。

## 3. 媒体 — SQLite の `read_*` テーブル（オーナー裁定 2026-09-02: 「SQLite のほうが自由度が高い。非正規データとしてのリードモデルを読むときに楽」）

**確定**: イベントストアと同じ SQLite ファイル（`aidlc/spaces/<space>/intents/.aidlc-store.sqlite`）
に、接頭辞 `read_` のリード表を置く。RMU が `catch_up` ごとに**ジャーナル差分の読取・行の
差し替え・チェックポイント前進を 1 トランザクション**で行う。

| 観点 | SQLite（推奨） | JSON ファイル（代替） |
| --- | --- | --- |
| 整合性 | チェックポイントと同一 Tx で更新でき、半端な状態を読まない | ファイルごとの原子的置換はできるが**複数ファイル間**の整合は保証できない（配布束 `store` と同じ問題） |
| 要求パラメータでの引当 | `WHERE` で自然（`--stage X` の行、スコープ別の行、キーワード引当） | DAO のコードで走査・選別することになり「読むだけ」の境界が曖昧になりやすい |
| 多重起動 | SQLite のロックで安全（hook は同時に走る） | ロック設計が要る |
| 可読性・デバッグ | `sqlite3` が要る | `cat` で見える |
| 既存資産 | RMU は既に同ファイルに `amadeus_projection_checkpoint` を持ち、rusqlite 依存も持つ | 追加依存なし |

クエリ側の DAO 実装が rusqlite で `read_*` を読むことは規則上許されている（cqrs-boundaries 規則 6
追補「DAO はファイルや SQLite のテーブルを読んで DTO で返してよい。媒体は実装詳細」）。
ドメインもコマンド側クレートも依存しない。JSON を選ぶ場合は「1 リードモデル 1 ファイル、
ファイル単位の原子的置換、複数面の整合は `as_of` 通番で検出」とする。

## 4. カタログ（構造化リードモデル）

キーの語彙: `definition_id`（系譜名）、`intent_id`、`execution_id`（実行）、`stage_index`
（計画上の索引）、`scope`（スコープ名）、`phase`、`request_kind`（要求種別）。すべての行に
`as_of_global_seq`（投影に使った最後のジャーナル通番）を持たせる。

### 4.1 定義由来（出所: `WorkflowDefinitionEvent::Defined` / `Redefined` — RMU が定義ストリームを購読する。現状は読み飛ばしており、購読の追加が要る）

| 表 | キー | 列 | 使う読取 |
| --- | --- | --- | --- |
| `read_definition` | definition_id | revision, default_scope（`classic`）, stock_scopes（compose 提案の 3 スコープ） | compose 提案、state なし群 |
| `read_definition_stage` | definition_id × stage_slug | 文書順 index, number, name, phase, execution, mode, lead_agent, support_agents, for_each, workspace_requires, reviewer, reviewer_max_iterations, review_class, plugin, enabled, **gated**（phase ≠ initialization）, stage_file（ハーネス相対）, consumes / produces / sensors_applicable / rules_in_context / protocol_modules（各 JSON 配列） | `--single`（孤立 run-stage の材料）、state なしの jump、run-stage の材料 |
| `read_definition_scope` | definition_id × scope | depth, keywords（JSON 配列）, skeleton, review_cap, freeform_default, has_grid_column, **cost**（EXECUTE 数・ゲート数・per-unit 反復数 — `scope_cost` を集約に持たせ RMU が呼ぶ）, is_stock | scope 検証（有効スコープの引当）、compose の費用節、`--new-intent` の費用節 |
| `read_definition_scope_keyword` | definition_id × keyword → scope | — | 自由記述のキーワード引当（scope 名アルファベット順の最初） |
| `read_definition_scope_stage` | definition_id × scope × stage_slug | action（EXECUTE / SKIP / 未収載）, in_scope_order | scope 別の部分グラフ・経路 |
| `read_definition_scope_phase_entry` | definition_id × scope × phase | first_stage_slug | walking skeleton のアンカー、`--phase` の解決 |

### 4.2 intent 由来（出所: `IntentEvent::Created`）

| 表 | キー | 列 |
| --- | --- | --- |
| `read_intent` | intent_id | definition_id, definition_revision, scope, request（依頼文）, depth / test_strategy（あれば）, created_at, scan（project type 等） |
| `read_intent_stage` | intent_id × stage_index | slug, phase, plan_action, conditional, number, name, lead_agent, gated |

### 4.3 実行由来（出所: `IntentExecutionEvent` 全変種 — RMU が `IntentExecution::replay` で集約を起こし、クエリメソッドで答えを計算する）

| 表 | キー | 列 | 計算元 |
| --- | --- | --- | --- |
| `read_execution` | execution_id | intent_id, status, cursor_index, cursor_slug, parked_at, **parked_active**, autonomy, revision_count, seq_nr, last_updated_at, **accepts_commands**, **state_binding**（`a` + `h` の畳み込み） | 集約のクエリ（`parked_active` / `accepts_commands` / `state_binding` の材料） |
| `read_execution_stage` | execution_id × stage_index | checkbox, effective_plan（overlay 反映後）, approved | 集約のクエリ（`checkbox` / `effective_plan` / `approved`） |
| `read_next_answer` | execution_id × request_kind ∈ {bare, resume, free_text, reentry} | decision_kind（run-stage / done / parked / unpark-then-resume / resume-menu / new-work-routing / inconsistent-skip / recover-skip-inconsistency）, stage_index, gated, checkbox, reason | **`IntentExecution::next_decision(request)`**（集約へ復元） |
| `read_next_jump` | execution_id × target_stage_slug | outcome（resolve-forward / resolve-backward / refused-init / refused-current / unknown）, direction, target_index | **`IntentExecution::jump_resolve`**（集約へ復元） |
| `read_next_jump_phase` | execution_id × phase | target_stage_slug（そのフェーズの最初の in-scope ステージ） | 計画 + 定義述語 |
| `read_run_stage` | **definition_id × scope × stage_slug**（訂正 2026-09-03: 材料は定義 × scope で決まり実行状態に依存しない。`gate` も phase 由来） | run-stage 指示の材料一式: stage_slug, phase, lead_agent, support_agents, mode, gate（gate field）, stage_file, memory_path（ハーネス相対）, inline_context_paths, consumes, produces, sensors_applicable, next_stage（次の in-scope）, reviewer, review_class, reviewer_max_iterations, protocol_modules, narration, unit（unit-major のときの unit kind / name）, rules_in_context, **directive_digest**, **route_digest** | 計画 + 定義 + 集約のクエリ。パスは**ハーネス相対**で持ち、絶対パスはプレゼンタが Layout から補う |
| `read_scope_change` | execution_id × scope | kind（scope-change / same-as-state）, command 材料（scope、修飾子の受け皿） | 「state scope と異なる有効 scope か」を RMU が事前計算。同じなら `same-as-state`（コントローラは bare の行へ流すだけ） |
| ~~`read_config_current`~~ | — | — | **不要（訂正 2026-09-03、b41 調査）**: upstream / 現行クエリ側の config-change は現在値を見ず「`--depth` / `--test-strategy` / `--review` のいずれかが来たら出す」構文分岐。現在値との差分判定は存在しないので表を作らない |

### 4.4 steering 由来（出所: **参照入力** = memory 規則ファイル `org.md` / `team.md` / `project.md` / `phases/<phase>.md` — 束の選択と `source_digest` は**この 4 ファイルと phase だけ**で決まる。ステージの `rules_in_context` は入力ではなく、run-stage が返す**配信済みパス台帳**（`delivered_paths`）である（訂正 2026-09-03、b41 調査）。イベントではないので、RMU が `catch_up` のたびに内容ダイジェストを見て変化時だけ再投影する）

| 表 | キー | 列 |
| --- | --- | --- |
| `read_steering_plan` | **phase**（訂正 2026-09-03: 束は `org.md` → `team.md` → `project.md` → `phases/<phase>.md` で決まり、ステージの `rules_in_context` は選択に関与しない） | bundle_digest, part_count, delivered_paths, source_digest（規則ファイル群の内容ダイジェスト — 再投影の要否判定に使う） |
| `read_steering_part` | **phase** × part_index | rules_content（path + text の JSON 配列 — 02 §10 の分割・パック済み） |

分割・パック（`SteeringPlan::pack`）は**RMU の投影ヘルパ**へ移す（判断ではなく導出。集約の関心事
でもない）。

### 4.5 `continue` の引当

`continue` は新しい表を要しない。トークンが運ぶ束縛（bundle / directive / route / state）を
`read_steering_plan` / `read_run_stage` / `read_execution` の同名列と **`WHERE` で突き合わせて
引く**。行が返れば次部（`read_steering_part` の `next_part_index`）か終端 run-stage（ピン
`gate` / `next_stage` / `unit` / `single` はトークンの値をプレゼンタがそのまま載せる）、返らなければ
「ドリフト — fresh `next` からやり直せ」の固定文言（fail-closed）。**再構築も照合ロジックも
クエリ側に無い** — 等値の引当だけである。

### 4.6 将来の読取コマンド

`--status` は `read_execution` + `read_execution_stage` をそのまま描く。`doctor` の読取面は
`read_definition*` と workspace の構造化面（別途）を描く。いずれも新しい計算はしない。

## 5. `next` の各分岐がどう「読むだけ」になるか

| 分岐（契約マップ §1） | コントローラの構文分類 | DAO の引当 | プレゼンタ |
| --- | --- | --- | --- |
| 前置ガード（パース失敗・`--review` 併用・`--stage` と `--phase` 併用） | フラグの形だけで決まる | **引かない** | 固定の逐語文言 |
| 0 Kiro ラッチ、1 読み取り専用ユーティリティ、1b 名詞トークン | 同上 | 引かない | 固定文言 / 素通し |
| state の有無 | active intent カーソル（機構）の有無 | `read_execution` を execution_id で引く。無ければ state なし群へ | — |
| 2.5 / 2.6 park、6 resume、9c 自由記述、10 ハッピーパス、skip 不整合 | request_kind を bare / resume / free_text に分類 | `read_next_answer(execution_id, request_kind)` **1 行** | `decision_kind` に従い run-stage / parked / done / ask を描く。run-stage は `read_run_stage(execution_id, stage_index)` を引き、規則束があれば `read_steering_part(…, 1)` と `continue_token` を描く |
| 3b / 4 scope 解決 | 明示 `--scope` / positional / env の**有無** | `read_definition_scope(definition_id, scope)` で有効性を引く（無ければ拒否文言）。自由記述は本文を語に分け（≤5 語のときだけ）`read_definition_scope_keyword` を引く | 拒否文言 / 解決結果 |
| 4c compose | フラグ | `read_definition_scope` の stock 行（費用節つき） | compose 提案 |
| 4a `--new-intent` / state なし 9a | フラグ + 本文 | 解決 scope の `read_definition_scope`（費用節） | `intent-create --scope <scope> --arguments=… --label …`（費用節つき） |
| 4b `--single` | フラグ + stage | `read_definition_stage(definition_id, slug)` | 孤立 run-stage（gated は行の値） |
| 5 scope-change / config-change | フラグ | `read_scope_change(execution_id, scope)` / `read_config_current` を要求値と突き合わせ | 命令 1 本に修飾子をまとめて描く |
| 7 jump | `--stage` / `--phase` | `read_next_jump(execution_id, slug)` / `read_next_jump_phase` | resolve 命令 / 拒否文言 |
| 7b / 8 / 9b state なし群 | state なし | `read_definition*` | NO_STATE 文言 / compose 提案 |

スコープ解決の**優先順**（state > `--scope` > positional > env > default）は「どの引当を先に試すか」
というコントローラのルーティング順であり、状態の値を見る判断は含まない（state scope は
`read_execution.scope` を読むだけ）。裁定点 §10-3。

## 6. RMU の投影手順（`catch_up`）

1. ジャーナル差分を読む（intent / execution / **definition** の全ストリーム — 定義の購読を追加）。
2. ストリームごとに集約を **`replay`** で起こす（`Intent::replay` / `IntentExecution::replay` /
   `WorkflowDefinition::replay` — domain が提供する再構成経路。Repository は使わない。
   投影核の入口は**イベントのまま**であり、規則 3 と整合する）。
3. 集約の**クエリメソッド**を呼んで答えを計算し（`next_decision` × 4 request_kind、`jump_resolve` ×
   全ステージ、`scope_cost`、述語面 …）、§4 の行を**キーごとに差し替える**。
4. (1) 系統の upstream 互換面（Markdown）も従来どおり投影する。
5. 参照入力（memory 規則ファイル）の内容ダイジェストを取り、`read_steering_plan.source_digest` と
   違うステージだけ分割・パックし直す。
6. 1〜5 と `amadeus_projection_checkpoint` の前進を **同一トランザクション**で確定する。

冪等・決定性: 壁時計を読まない（`as_of` はジャーナル通番）、同じ差分からは同じ行。再投影は
キー差し替えなので何度でも走らせられる（NFR3）。

## 7. 判断の所在の復元（クエリ側から消えるもの）

| いまクエリ側 | 戻す先 |
| --- | --- |
| `ExecutionStateView::next_decision` / `parked_active` / `accepts_commands` / `is_gated` / `effective_plan` / `state_binding` の材料 | `IntentExecution`（仕様 10 §2.3 の所在どおり — b26 以前の実装が正本） |
| jump の目的地探索（branch 7） | `IntentExecution::jump_resolve` |
| `DefinitionView::subgraph_for_scope` / `stages_in_scope` / `first_in_scope_stage_of_phase` / `stage_route` / `valid_scopes` | 既に domain の `WorkflowDefinition` にある — 複製を削除 |
| `scope_cost`（compose / new-intent の費用節） | `WorkflowDefinition::scope_cost(scope)`（新設） |
| `SteeringPlan::pack` | RMU の投影ヘルパ |
| continue_token の再構築・束縛照合 | 消える（§4.5 — 等値の引当） |
| `NextDecision` / `ScopeResolution` / `Bindings` 等の判断型 | 消える（`NextDecision` は domain の集約 API 型として復元） |
| 逐語文言 `wording` | プレゼンタ（出す側） |
| Markdown 逆パース / 配布 3 ファイルのパース | 消える（構造化リードモデルを読む） |

クエリ側に残るのは: DAO ポート（`find` のみ）、`*View`（基本データ型の値。判断メソッド無し）、
ユースケース（`dao.find(key)` を返すだけ）。

## 8. 整合性・鮮度

- 読取コマンドは常に合成ルートの `catch_up` の**後**に走る（現状どおり）。行の `as_of_global_seq`
  がジャーナル末尾と一致することが「最新」の定義。
- `state_binding` は行に含まれ、continue_token に載る。トークンと行の不一致は §4.5 の引当で
  自然に検出される。
- 参照入力（規則ファイル）は `catch_up` ごとに再投影されるので、`next` と `continue` の間で
  規則ファイルが変われば `bundle_digest` が変わり、fail-closed が保たれる。

## 9. 移行段階（提案）

1. **判断の集約復帰**: `IntentExecution::next_decision` / `jump_resolve` / `state_binding` 材料、
   `WorkflowDefinition::scope_cost`。Quint `engine_loop` の観測面 ITF（`lastDirective`）を domain
   側 `engine_loop_conformance.rs` へ戻す。
2. **RMU の構造化投影**: 定義ストリームの購読、`replay` による集約導出、§4 の表と Tx、
   steering の参照入力リフレッシュ。契約テスト: 各表が集約のクエリと一致すること（RMU 単体）。
3. **クエリ側の縮小**: DAO を `read_*` の引当へ差し替え、ユースケースを `find` → View に、
   コントローラ / プレゼンタへ分類と文言を移し、判断型と 2 つのパーサを削除。golden
   （directive / token の逐語）で外部観測が変わらないことを固定。

## 10. オーナー裁定（2026-09-02、確定）

1. **媒体**: SQLite の `read_*` 表。「SQLite のほうが自由度が高い — 非正規データとしての
   リードモデルを読むときに楽」（オーナー）。リードモデルは正規化しない（読取コマンドが 1 回の
   引当で答えを得られる形に非正規化して置く）。
2. **DAO の線引き**: 要求パラメータによる引当（`WHERE`）は可、行に無い事実の導出は不可 —
   「作ったら CQRS 違反」（オーナー）。
3. **スコープ解決の優先順**: コントローラのルーティング順。自由記述のキーワード引当（語分割・
   ≤5 語）は構文分類として扱う。
4. **steering の参照入力**: `catch_up` ごとのダイジェスト比較で変化時だけ再投影。
5. **advisory マーカー**（MAC 鍵など）の書込は合成ルートの機構モジュールが担う。

## 11. 既存正本との整合（要更新）

- cqrs-boundaries 規則 6 に「クエリ側ユースケースは DAO で View を読んで返すだけ。判断・導出・
  選択・文言はしない。計算結果は RMU が投影する」を追記。規則 3 に「投影核は `replay` で集約を
  導出してクエリメソッドを呼んでよい（入口はイベントのまま）」を追記。
- 仕様 10 §2.3 は現行どおり（`next_decision` / `jump_resolve` は集約のクエリ）。§3 の「CQRS は
  採用しない」「`Next` に Repository を注入しない型強制」は失効注記が要る。
- 仕様 11（workspace）に構造化リードモデルの節を追加（媒体・表・更新契機）。
