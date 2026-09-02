# クエリ側 / RMU 全量監査 — 「クエリ側は作成・更新済みのリードモデルを読むだけ」に照らして（2026-09-02）

発端: オーナー指摘（2026-09-02、PR #88 マージ後）「RMU でクエリ側が複雑なことをしなくても済むように
**計算結果としてのリードモデル**を構築する必要がある。クエリ側はこんな複雑なロジックは実行しない。
文字通り作成・更新されたリードモデルを読むだけ。クエリ側の実装は大幅に間違っているし、RMU の実装も
間違っている」。本監査はその指摘を実コードで検証した記録である。裁定はオーナー。

基準（オーナー原則 + 既存正本）:

- 集約は FSM — データ・遷移・**判断**を同じ集約に閉じ込め、ユースケースは進行管理のみ
  （project.md Corrections 2026-08-22）。
- クエリ側は RMU が構築したリードモデルだけに依存して読む（cqrs-boundaries 規則 6）。
  **読む = 判断しない・導出しない**。導出（計算結果）は RMU が投影して持たせる（本指摘で明確化）。
- 仕様 10 §2.3: `next_decision` / `jump_resolve` は**集約 `WorkflowExecution`（= `IntentExecution`）の
  クエリメソッド**であり、「状態の所有者の外で判断する Ask 型を避ける」（ADR-002 ④）。

## 結論（先に）

指摘は **実コードで裏づけられる**。

1. **RMU が投影しているのは upstream 互換の 2 面（`aidlc-state.md`・監査シャード）だけ**で、
   クエリ側が答えに使う計算結果（次の指示・スコープ解決・配信計画・トークン束縛の材料・
   定義の述語面）は**一切投影していない**。定義のイベント列（`WorkflowDefinitionEvent`）は
   読み飛ばしている。
2. **クエリ側は、その不足分をすべて自前で計算している**（非テスト行で約 2,300 行）。
   `next` の 21 分岐ラダー・スコープ推定・コスト計算・jump 解決・steering 配信計画の分割と
   パック・continue_token の束縛検証・定義 3 入力のパースと述語導出。これは「読むだけ」ではなく
   **判断と導出**であり、しかも `next_decision` は仕様 10 §2.3 が集約のクエリメソッドと
   定めたものを、b26 で集約から**削除してクエリ側へ移した**（`ExecutionStateView::next_decision`
   の doc に「分岐と優先順は旧 `IntentExecution::next_decision` と同一」と自ら書いている）。
3. 経緯: 2026-08-30 裁定「`next` / `continue` は書かないのでクエリ側」を、b26 / b27 の実装者
   （AI）が「**判断もクエリ側へ移してよい**」と読み替えた。動詞の置き場と判断の置き場は別で
   あり、この読み替えが誤り。仕様 10 §2.3 は更新されておらず、実装だけが仕様から離れた。

## A. RMU の実測

| 項目 | 実測 | 出典 |
| --- | --- | --- |
| 投影核の入口 | `project(entries, plan, read_model)` — `IntentExecutionEvent` 列 + 解決済み計画 → `ReadModel` | `read-model-updater/src/workspace/projection.rs:306` |
| リードモデルの面 | **2 面のみ**: 状態ファイル（置換）・監査シャード（追記） | `workspace/read_model.rs`（「投影が読み書きする 2 つの面」）、`orchestration/projection_targets.rs` |
| 投影が見るもの | イベント・計画・現状態ファイル本文。**ワークフロー定義は引かない**（冪等のため） | `projection.rs` モジュール doc「ワークフロー定義も引かない」 |
| 定義イベント | `WorkflowDefinition` のストリームは**読み飛ばし** | `journal_reader_impl.rs:1520` `the_definition_stream_is_skipped_instead_of_being_read_as_ours` |
| 投影**していない**計算結果 | 次の指示（`NextDecision` / directive）、スコープ解決の材料、in-scope 部分グラフ・経路、gate/skeleton/自律モードの実効値、steering 配信計画、continue_token の束縛材料（state / graph / route / bundle の各ダイジェスト）、定義の述語面（`valid_scopes` / `stages_in_scope` …） | RMU ツリーに該当モジュール無し |

RMU の設計方針（純粋投影核・定義を引かない・upstream 互換バイトの再現）は **upstream 互換の
2 面に限れば正しい**が、「クエリ側が読むだけで済む計算結果」を作る責務を**持っていない**。

## B. クエリ側の実測 — 何を計算しているか

| 場所 | 行数（非テスト） | 中身 | 本来の所在（基準に照らして） |
| --- | --- | --- | --- |
| `query/use-case/.../next_use_case.rs` | ≈1,190 | `next` の 21 分岐ラダー全体: 前置ガード、park / resume、scope 解決ラダー、compose 提案（例文・**コスト計算** `scope_cost`）、`--new-intent` の命令組立、`--single`、設定変更検出、jump（`branch_7` の目的地探索）、ハッピーパス、run-stage の組立と steering 連鎖 | 判断は集約（`IntentExecution::next_decision` / `jump_resolve` — 仕様 10 §2.3）。計算結果は RMU が投影。クエリ側に残るのは「要求フラグに応じて投影済みの答えを選び、逐語文言で出す」だけ |
| `port/execution_view/execution_state_view.rs` | ≈347 | `next_decision` / `parked_active` / `accepts_commands` / `is_gated` / `effective_plan` / `state_binding` の材料 — **旧 `IntentExecution` のクエリメソッドの写し** | 集約（仕様 10 §2.3 の所在どおり）。ビューは投影済みの答えを持つだけ |
| `port/workflow_view/definition_view.rs` + `scope_resolution.rs` | ≈200 + 162 | `valid_scopes` / `subgraph_for_scope` / `stages_in_scope` / `first_in_scope_stage_of_phase` / `stage_route` / `resolve_scope`（state > --scope > positional > env > default、キーワード推論・5 語超抑止）— **ドメイン `WorkflowDefinition` の述語面の複製 + スコープ解決の判断** | 述語は集約 `WorkflowDefinition` / `ScopeMetadata`。解決結果は RMU が投影 |
| `steering_plan.rs` | ≈178 | 規則束の Markdown 見出し分割・過大セクション分割・輸送目標へのパック（`SteeringPlan::pack`） | 配信計画は投影で作れる計算結果（規則ファイルの内容が変わる契機で RMU が再投影）。少なくともクエリ側ユースケースの仕事ではない |
| `continue_use_case.rs` + `continue_token.rs` + `bindings.rs` + `*digest.rs` | ≈214 + 175 + 100 | トークン検証、ディスク状態からの run-stage **再構築**、ピン再適用、4 種ダイジェスト束縛の照合（fail-closed） | 束縛材料（ダイジェスト）は投影済みリードモデルが持つ。クエリ側は「読んだ値と一致するか」の等値比較と提示のみ |
| `query/interface-adapter/.../workflow_definition_parse.rs` / `execution_state_parse.rs` | — | 配布 3 ファイルの serde パース + frontmatter 手書きパース。**RMU が書いた Markdown（`aidlc-state.md`）を逆パース**して `ExecutionStateView` を組む | upstream 互換の Markdown は人間・upstream ツール向けの面。クエリ側の答えに必要な値は、RMU が**構造化リードモデル**（JSON / SQLite）として別に投影すべきで、逆パースは不要になる。定義 3 入力のパースも、定義イベントを RMU が投影すれば不要 |
| `query/use-case/tests/engine_loop_ladder_conformance.rs` | — | Quint `engine_loop` の**観測面（lastDirective）**をクエリ側で照合 | 判断が集約に戻れば、観測面の ITF も domain 側（`engine_loop_conformance.rs`）へ戻る |

`NextDecision` / `ScopeResolution` / `SteeringPlan` / `Bindings` はリードモデルではなく**判断の型**、
`Directive` / `RunStageDirective` / `LoadSteeringDirective` / `AskDirective` / `EngineCommand` /
`EngineSignal` / `ContinueToken` / `StateBinding` は**出力（ワイヤ）モデル**。「クエリ側のモデルは
リードモデルだけ」という原則に照らすと、前者はクエリ側に存在してはならず、後者は「投影済みの
答えを出力の形に写す」用途に限って残りうる。

## C. 仕様との乖離

- 仕様 10 §2.3 は `next_decision` / `jump_resolve` を集約のクエリメソッドと規定したまま。実装は
  b26 で集約から削除。**実装が仕様から離れており、仕様側は正しい**。
- 仕様 10 §3 は「CQRS は採用しない」「`Next` に Repository を注入しないことで型強制」と旧記述の
  まま（ADR-001/003/004・b26 以降と不整合）。別件として更新が要る。
- cqrs-boundaries 規則 6 の「読むだけ」に「判断・導出をしない」が明文化されていない — 本指摘で
  明確化された点であり、正本への追記対象。

## D. 目標の形（オーナー確認済み 2026-09-02 — 詳細は read-model-spec.md）

- **クエリ側ユースケースは `dao.find(key) → View` を返すだけ**。判断も導出も**選択も文言組立も
  しない。要求フラグによる分岐はコントローラの構文的ルーティング（どの DAO をどのキーで引くか）、
  逐語文言と directive / token の綴りはプレゼンタ。
- **リードモデルは 2 系統**: (1) 人・upstream ツールがそのまま見るファイル面（`aidlc-state.md` /
  監査シャード — クエリ側ユースケース不要）、(2) CLI 読取コマンド（`next` / `continue` / 将来の
  `--status` / `doctor`）向けの構造化リードモデル。(2) は **RMU が計算結果まで作る**（媒体は
  SQLite でも JSON でもよい）。クエリ側で作ることは禁止。
- **判断は集約へ戻す**（`IntentExecution::next_decision` / `jump_resolve` ほか — 仕様 10 §2.3 の
  所在どおり）。RMU はイベントから `replay` で集約を起こし、クエリメソッドを呼んで答えを行に書く。

## E. 規模と進め方の所見

- 移動対象はクエリ側の非テスト約 2,300 行と RMU の新規投影。集約側の `next_decision` は b26 以前の
  実装（git 履歴）と Quint `engine_loop` の観測面が復元の正本になる。
- 分割案: (1) 判断の集約復帰 + 観測面 ITF の domain 復帰、(2) RMU の計算結果投影（構造化
  リードモデルの形の裁定を含む）、(3) クエリ側の縮小（DAO は構造化リードモデルを読む形へ、
  Markdown 逆パースと定義 3 入力パースの撤去）。(1)→(2)→(3) の順が安全。
- 先に正本へ「クエリ側は判断・導出をしない。計算結果は RMU が投影する」を明文化する
  （cqrs-boundaries 規則 6 追記、仕様 10 §2.3 は現行どおりで実装を戻す旨を注記）。
