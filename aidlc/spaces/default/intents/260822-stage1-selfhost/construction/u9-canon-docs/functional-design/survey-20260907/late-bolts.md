# 後期 Bolt（b39〜b50）記録調査 — 裁定台帳

`C` = `aidlc/spaces/default/intents/260822-stage1-selfhost/construction` と略記する。
記録の記述は「主張」として写しており、正しさの判定はしていない（判定は依頼者がコードで行う）。

## 裁定台帳

| # | 日付 | 裁定（1 文） | 出典 path:line | 影響する仕様・共有契約の節 | コードで確認すべき点 |
| --- | --- | --- | --- | --- | --- |
| 1 | 2026-09-02 | `Started` が集約 ID・intent_id・計画（各ステージの slug/phase/plan_action）を運び、`From<(Started, at)>` が genesis 状態を導出する（ストリームの自己完結） | `C/b39-rmu-read-tables/design.md:15-21,34-38` | 10 号 §2.1 イベント表、12 号、contract-summary C5 | `command/domain/src/orchestration/intent_execution.rs` の `impl From<(Started, DateTime<Utc>)>` と `start` |
| 2 | 2026-09-02 | RMU は replay した集約のクエリの答えを行へ写すだけで判断を持たず、全再計算＋全差し替え・壁時計を読まない・`as_of` は走査済み最終通番 | `C/b39-rmu-read-tables/design.md:23-30` | 11 号 §4.1、components.md（RMU の責務） | `read-model-updater` の `ReadTables::project` |
| 3 | 2026-09-02 | 行の全差し替えとチェックポイント前進は同一トランザクション（`BEGIN IMMEDIATE`） | `C/b39-rmu-read-tables/design.md:27,115-117` | 11 号 §4.1 | `JournalReader::advance_checkpoint` の署名 |
| 4 | 2026-09-02 | `--phase` ジャンプの目的地判断を集約のクエリ `first_in_scope_of_phase(PhaseId) -> Option<StageIndex>` に置く | `C/b39-rmu-read-tables/design.md:39-41` | 10 号 §2.3 | `intent_execution.rs` の同名クエリ |
| 5 | 2026-09-02 | `read_*` 13 表のカタログとキー（definition / definition_stage / scope / scope_keyword / scope_stage / scope_phase_entry / intent / intent_stage / execution / execution_stage / next_answer / next_jump / next_jump_phase） | `C/b39-rmu-read-tables/design.md:76-92` | 11 号 §4.1 | `read_tables/sql.rs` の DDL |
| 6 | 2026-09-02 | 行は基本データ型のみ（配列・構造は canon_json `ContractCompact` の JSON 文字列）、`RequestKind` は 4 値で列値は kebab-case、読取面の綴りは `spelling.rs` 1 箇所 | `C/b39-rmu-read-tables/design.md:71-74,108-111` | 11 号 §4.1 | `read_tables/json_column.rs`、`read_tables/spelling.rs` |
| 7 | 2026-09-02 | ADR-011: 構造化リードモデルの媒体は SQLite の `read_*` 表 | `C/b39-rmu-read-tables/design.md:136-139` | 11 号、decisions.md ADR-011 | — |
| 8 | 2026-09-02 | ドメインイベントはエンティティ。`XxxEvent { id: XxxEventId, aggregate_id: XxxId, .. }` とし、採番は集約のコマンド内で UUIDv7 | `C/b40-domain-event-id/design.md:3-16,38` | 01 号 §3、10 号 §2.1、12 号、C5 | 各 `intent_execution_event/*.rs` の `new(id, aggregate_id, ..)` |
| 9 | 2026-09-02 | イベント ID は payload に閉じ、封筒は `aggregate_id` / `seq_nr` / `occurred_at` のまま（ADR-010 維持） | `C/b40-domain-event-id/design.md:17-19,71-74` | 10 号、decisions.md ADR-002 / ADR-010 | `EventEnvelope` のキー |
| 10 | 2026-09-02 | 全 19 変種が id と aggregate_id を持ち（`Unparked` は unit → struct へ昇格）、`apply_event` は aggregate_id を見ず、照合は復号境界で全変種・不一致は `Corrupt` | `C/b40-domain-event-id/design.md:28-35,40-41,48-50` | C5、11 号、components.md | `decode_*` と `find_by_id` の再生経路 |
| 11 | 2026-09-03 | steering の束は phase の関数であり、ステージの `rules_in_context` は束の選択に使わない | `C/b41-rmu-run-stage-steering/design.md:19-21,32,76-78` | 11 号 §4.1、10 号 | `read_steering_plan` のキー |
| 12 | 2026-09-03 | `read_run_stage` は「定義 × scope」で決まり実行に依存しない。パスは相対で、絶対化はプレゼンタの仕事 | `C/b41-rmu-run-stage-steering/design.md:31,38` | 11 号 §4.1、10 号 §2.3 | `read_run_stage` の `*_rel` 列 |
| 13 | 2026-09-03 | `directive_digest` の素材は環境由来 4 キーのみで pins を含めない | `C/b41-rmu-run-stage-steering/design.md:31,51-52` | 10 号（continue の照合） | `hash_compact` の呼出素材 |
| 14 | 2026-09-03 | `read_config_current` は作らない（config-change は現在値を見ない構文分岐）。代わりに `read_scope_change` と `read_execution.scope` を持つ | `C/b41-rmu-run-stage-steering/design.md:24-25,34-35,77-78` | 11 号 §4.1 | `read_scope_change` の kind 列 |
| 15 | 2026-09-03 | steering は別投影単位＋別トランザクションで、`source_digest` が変わったときだけ再パックし、比較は `catch_up` の早期 return より前に行う | `C/b41-rmu-run-stage-steering/design.md:53-62` | 11 号 §4.1 | `replace_steering`、`SteeringSource` |
| 16 | 2026-09-03 | #85 = A。非ゲート完了パイプラインを撤去し（`complete_stage` / `StageCompleted` 削除）イベントは 11 変種。ただし監査行の型 `EventType::StageCompleted` は残す | `C/b42-remove-ungated-completion/design.md:1-13,34-35,43` | 10 号 :50-51、C5、11 号（監査面） | `intent_execution_event.rs` の変種列挙 |
| 17 | 2026-09-03 | クエリ側ユースケースは `dao.find(key) → View` だけで、判断・導出・選択・文言組立を持たない | `C/b43-query-dao-lookup/design.md:3-8` | 10 号 §3、components.md | `query/use-case` の各 `execute` |
| 18 | 2026-09-03 | JOIN 解体。DAO は 1 表 1 引当で JOIN も副問合せも非正規化の焼き込みもせず、関連行は FK 列で指し、ユースケースが FK をたどる | `C/b43-query-dao-lookup/design.md:9-10,25-28,48-50`、`C/b43-query-dao-lookup/inventory.md:4-7` | 11 号 §4.1、components.md | `*_dao_impl.rs` の `FROM` が各 1 つか |
| 19 | 2026-09-03 | 1 表 = 1 ポート = 1 View（行の写し）。組み立て View はユースケース側 `orchestration/` に置き `port/` の住人にしない | `C/b43-query-dao-lookup/design.md:13-16,25-28` | components.md、contract-design | `NextTurnView` / `ContinuationView` の所在 |
| 20 | 2026-09-03 | ポートは 12 本で動詞は `find`、読取エラーは 1 本 `ReadModelReadError { kind, path }` に収束 | `C/b43-query-dao-lookup/design.md:30-44` | components.md、10 号 §4 | `query/use-case/src/port/` |
| 21 | 2026-09-03 | 要求の形（フラグ・語数・token の有無）で決まる分岐はコントローラ、状態の値で決まる分岐は行の `kind` に従いプレゼンタが描く | `C/b43-query-dao-lookup/design.md:11-12,58-62` | 10 号 §3 | `app/src/turn.rs`、`directive_drawing.rs` |
| 22 | 2026-09-03 | 実行カーソル `<record>/.aidlc-execution` を `mint_intent` が書き、next/continue/report/park のキーにする。ジャーナル先頭決め打ちは撤去、`definition_id` は `"claude"` 固定 | `C/b43-query-dao-lookup/design.md:19-22`、`C/b43-query-dao-lookup/inventory.md:31-39` | 11 号（ワークスペース語彙） | `execution_cursor.rs`、`Layout::execution_cursor` |
| 23 | 2026-09-03 | `read_*` 17 表は単一主キー `id` + 自然キー UNIQUE + FK 列。同一 Tx の FK が引けなければ `broken_projection`、steering 2 表は別 Tx なので不在は `None` | `C/handoff-b43.md:6-7`、`C/b43-query-dao-lookup/design.md:55` | 11 号 §4.1 | `read_tables/row_id.rs` |
| 24 | 2026-09-03 | クエリ側の SQL はコンパイル時リテラルで、1 表であることを `cargo lint` の `dao-single-table` が機械強制する | `C/b43-query-dao-lookup/design.md:48-50`、`C/handoff-b43.md:11` | 11 号、coding-rules | `tools/lint/src/check.rs` R5 |
| 25 | 2026-09-04 | 1 要求 = 1 読取専用接続を 12 DAO 実装が `Rc` で共有する | `C/handoff-b44.md:14-16` | components.md | `ReadModelDaos` |
| 26 | 2026-09-04 | lint 2 本を追加。use-case 層の公開 trait はコマンド側 `XxxRepository` / クエリ側 `XxxDao` のみ、コマンド側は `*_repository_impl.rs` 以外で I/O 禁止 | `C/handoff-b44.md:34-38` | coding-rules gateway-taxonomy §1d | `tools/lint` R6 / R7 |
| 27 | 2026-09-04 | park の受理述語は park 専用（取り違え → autonomy → `Status::is_running`）で `parked_active` を見ない＝再スタンプ許容 | `C/b45-park-complete/design.md:30-47,146-148` | 10 号 §2.3 / §10、formal | `IntentExecution::park` |
| 28 | 2026-09-04 | park の失敗はすべて `Cannot park the workflow: <detail>` の error directive（exit 0）、成功後の stage は投影された行から読み、ユースケースは CQS で何も返さない | `C/b45-park-complete/design.md:16-18,99-109` | 10 号 §10、11 号 §4.1 | `ParkUseCase`、`FindExecutionUseCase` |
| 29 | 2026-09-04 | Quint `actPark` を Running ∨ WorkflowParked へ緩和し、witness `w_repark` と不変条件 `parked_marker_status` を追加 | `C/b45-park-complete/design.md:49-68,149-160` | 10 号 §9、`formal/orchestration/engine_loop.qnt` | モデルの不変条件本数 |
| 30 | 2026-09-04 | upstream の「Current Stage 不在」拒否は Always Valid の帰結として構造的に発生不能であり逸脱ではない | `C/b45-park-complete/design.md:27,137-142` | 10 号 §10、deviations.md | 不変条件 `cursor_in_scope` |
| 31 | 2026-09-04 | `report_dispatch` は独立ドメインサービスではなく集約のクエリ（`&self`） | `C/b46-report-guards/design.md:20-23,162` | 10 号 §2.3 | `IntentExecution::report_dispatch` の署名 |
| 32 | 2026-09-04 | 入力 `ReportRequest`（VO、domain 置き）、出力 `ReportDecision`、拒否 `ReportRefusal`（13 変種、`CommandError` とは別型） | `C/b46-report-guards/design.md:67-83,99-100,189-190` | 10 号 §2.3 | `report_request.rs` ほかの型 |
| 33 | 2026-09-04 | `TransitionStep` は 8 段で upstream の綴りを返す。`GateStartRecovered` は独立イベントではなく `[-]` からの `Approve` 1 イベント | `C/b46-report-guards/design.md:74-75,108-111` | 10 号 §2.3、11 号（監査面） | `TransitionStep::subcommand` |
| 34 | 2026-09-04 | report の判定順はピン準拠（対象解決 → skipped 5 条件 → gate 系 → 段 13 human presence → forward 表） | `C/b46-report-guards/design.md:85-97` | 10 号 §2.3 / §10 | `report_dispatch` 本体の分岐順 |
| 35 | 2026-09-04 | app は構文段だけを持ち状態の値で決まる分岐を 1 つも持たない。拒否・no-op の材料は決定が運び、app は文言のためにリードモデルを読まない | `C/b46-report-guards/design.md:26-30` | 10 号 §3 | `runtime.rs` の report 経路 |
| 36 | 2026-09-04 | `StateFileDao`（状態ファイルの生テキストを返す、動詞は `find`）と `StateVersionClassification`（綴り一致、`CURRENT_STATE_VERSION = "8"`） | `C/b46-report-guards/design.md:38-43,188,192` | 11 号、10 号 §10 | `state_file_dao`、`classify` の比較 |
| 37 | 2026-09-04 | ワークフロー完了の投影は状態 7 欄＋監査 3 行（`PHASE_COMPLETED` → `PHASE_VERIFIED` → `WORKFLOW_COMPLETED`）で、`STAGE_COMPLETED` は再 emit しない。checkbox の綴りは domain 1 箇所を正本に RMU も使う | `C/b46-report-guards/design.md:132,136-146,198` | 11 号（状態ファイル・監査面）、01 号 | `projection.rs` の `complete_workflow`、`CheckboxState::spelling` |
| 38 | 2026-09-04 | `CommitOutcome` は 3 形、`CommitError` に `Refused` と `UnwiredTransition` を追加し、`UnknownStage` は `ReportRefusal` へ移す（後方互換を残さない） | `C/b46-report-guards/design.md:104-113,180-181` | 10 号 §3 | `commit_verdict_use_case.rs` |
| 39 | 2026-09-04 | 裁定 B。`report --single` の疑似 ID 対は新集約を作らず `IntentExecution` のイベントにし、I10 の強制手段を E1（ポート非注入）から E4（`single_run_frame`）＋単体へ改訂。適用はフレーム空 | `C/b47-single-skeleton/design.md:12,33-36,138,146-148` | 10 号 §6 I10、§9 | `record_single_stage_run` の適用が状態を動かさないか |
| 40 | 2026-09-04 | 裁定 A。skeleton stance も `IntentExecution` のコマンドとイベントにする | `C/b47-single-skeleton/design.md:13,37-41` | 10 号 §6、01 号 | `record_skeleton_stance` |
| 41 | 2026-09-04 | イベントを 13 変種へ拡張し、状態に `skeleton_stance: Option<SkeletonStance>` を追加 | `C/b47-single-skeleton/design.md:28-32` | C5、01 号 §3 | `intent_execution.rs` の状態欄 |
| 42 | 2026-09-04 | skeleton-gate stage は静的計画（`Intent::stages` のグリッド）由来で recompose overlay を見ない。再記録は上書き | `C/b47-single-skeleton/design.md:37-41,51-55,196` | 01 号 B11、10 号 | `skeleton_gate_stage` の実装 |
| 43 | 2026-09-04 | `next_decision` の gate を 3 値 `GateDecision`（Gated / Ungated / Unresolved）へ。署名に `&Intent` を追加し、ゲート判定は RunStage が名指すステージに対して行う | `C/b47-single-skeleton/design.md:43-46,196-197` | 10 号 §2.3 | `next_decision` の署名と戻り値 |
| 44 | 2026-09-04 | `SingleStageRunCommitted` は監査 2 行だけ（`Workflow: single-stage:<slug>`）で状態ファイルも `read_*` も動かさない。stance は Runtime State 欄と `read_execution` 列 | `C/b47-single-skeleton/design.md:88-92,154-156` | 11 号 | `projection.rs` の両腕 |
| 45 | 2026-09-04 | `read_next_answer.gated INTEGER` を `gate TEXT`（3 綴り、正本は `GateDecision::spelling`）へ置換し、`read_run_stage.in_scope` を新設 | `C/b47-single-skeleton/design.md:93-95,157,199` | 11 号 §4.1 | `sql.rs` の列定義 |
| 46 | 2026-09-04 | 読み面スキーマの版管理を `PRAGMA user_version` で行い、不一致なら `read_*` 17 表を作り直す。投影チェックポイントは戻さない | `C/b47-single-skeleton/design.md:96-98,208` | 11 号 §4.1 | `JournalReaderImpl::open` の版照合 |
| 47 | 2026-09-04 | `next --single` は state を読まず何も記録せず、directive は `single: true` / `gate: false` / `next_stage` 無しを強制する | `C/b47-single-skeleton/design.md:102-109,159-160` | 10 号 §10 | `directive_drawing` の single 経路 |
| 48 | 2026-09-04 | 裁定。レビュー受領証は `IntentExecution` のイベント、鮮度は順序だけ、形は依頼と判定の対（`ReviewRequested` / `ReviewCompleted`） | `C/b48-review-receipts/design.md:10-12,56-63,137` | 10 号 B10、§6 I18 | `request_review` / `record_review_verdict` |
| 49 | 2026-09-04 | レビュー方針は `WorkflowDefinition::review_policy(slug, scope, override)` の判断。cap と override は下げるだけ、reviewer 宣言ありでクラス無しは adversarial、budget は advisory 1 / adversarial `max_iterations` / none 0 | `C/b48-review-receipts/design.md:28-38,139` | 12 号、10 号 §2.3 | `review_policy.rs`、`ReviewCapValue::weaker` |
| 50 | 2026-09-04 | `ReviewVerdict`（Ready / NotReady、`READY` / `NOT-READY`）は report の `Verdict` と別型 | `C/b48-review-receipts/design.md:40-42` | 10 号 | `review_verdict.rs` |
| 51 | 2026-09-04 | 状態に `review_attempts: Vec<ReviewAttempt>`（計画と同じ長さ）を持ち、`ReviewAttempt` は requests / pending / closed | `C/b48-review-receipts/design.md:44-54` | 01 号 §3 | 完全コンストラクタの長さ検査 |
| 52 | 2026-09-04 | 試行のフロアは 4 か所（前進・読み飛ばしで立った次ステージ、`GateRejected` のステージ、`Jumped` は全ステージ）。`StageRevised` は区切らない | `C/b48-review-receipts/design.md:55,138` | 10 号 §6 I18 | `reset_attempt` の呼出箇所 |
| 53 | 2026-09-04 | `approve_gate` に `policy` 引数を足し、段 11 は checkbox 前提の後・変更の前。受領証系の `CommandError` 変種を 7 本追加 | `C/b48-review-receipts/design.md:64,67,137,139` | 10 号 §2.3、§6 I18 | `approve_gate` の署名とガード順 |
| 54 | 2026-09-04 | RMU は監査 2 行だけを描き、状態ファイル・`Last Updated`・checkbox・`read_*` を触らない | `C/b48-review-receipts/design.md:103-105,142` | 11 号 | `projection.rs` の Review 2 腕 |
| 55 | 2026-09-04 | `CommitVerdictUseCase` に定義ポートを足し Approve 段だけが定義を読む。`RecordReviewUseCase` を新設 | `C/b48-review-receipts/design.md:97-98,141` | 10 号 §3 | ポートの型引数と lookup 回数 |
| 56 | 2026-09-04 | 新しい面 `Face::Log`（`aidlc-log`）。`review` だけ配線し、失敗はすべて stderr + exit 1、`ERROR_LOGGED` は描かない | `C/b48-review-receipts/design.md:109-111,143` | 10 号 §10、deviations #5 | `cli/face.rs`、`runtime::log_review` |
| 57 | 2026-09-05 | 裁定 A。`team.md` / `project.md` への書込は RMU の投影面とし、合成ルートは材料を渡すだけで集約がイベントに載せる | `C/b49-practices-receipt/design.md:10,20,96-102` | 11 号（メモリ層） | `ReadModel::with_memory`、`MemoryFaces` |
| 58 | 2026-09-05 | `PRACTICES_DISCOVERY_SLUG` はリテラル定数にする（定義集約のクエリにしない） | `C/b49-practices-receipt/design.md:27-29` | 12 号 | `workflow_definition` の定数 |
| 59 | 2026-09-05 | Markdown の 3 関数（extract / replace / append_under_heading）を domain の `workspace` に置き、RMU の `with_field_or_insert` をそこへ寄せる（挿入位置は次の見出しの直前） | `C/b49-practices-receipt/design.md:31-39,188` | 11 号 | `markdown_sections.rs`、`state_writers` |
| 60 | 2026-09-05 | `PracticesPromotion::plan` の規則（5 節の選択、印付け、正本との重複除去、見出し不在は Err、節も規則も空でも受理） | `C/b49-practices-receipt/design.md:41-49` | 11 号 | `practices_promotion.rs` |
| 61 | 2026-09-05 | 状態に `practices_affirmed: Vec<bool>`、`affirm_practices` のガードは 2 つだけ、段 12 は checkbox の後・段 11 の前 | `C/b49-practices-receipt/design.md:51-61,143-146` | 10 号 §6 I19、§2.3 | `require_practices_receipt` の位置 |
| 62 | 2026-09-05 | `PracticesAffirmed` が昇格内容そのもの（sections / mandated / forbidden / affirming_user）を運び、イベントは 16 変種 | `C/b49-practices-receipt/design.md:62,142` | C5、01 号 §3 | `practices_affirmed.rs` |
| 63 | 2026-09-05 | 新しい面 `Face::State`（`aidlc-state`）で `practices-promote` だけ配線し、他 24 動詞は not-wired。失敗時の `PRACTICES_OVERRIDE` は描かず `practices-event` は射程外 | `C/b49-practices-receipt/design.md:11-12,108,186` | 10 号 §10、deviations #6 | `cli/request.rs` の動詞表 |
| 64 | 2026-09-05 | 段 12 の拒否は orchestrate 自身の error directive（段 11 は `aidlc-state approve` の包み文）で、腕の順序は段 12 が先 | `C/b49-practices-receipt/design.md:119,193` | 10 号 §10 | `commit_refusal` の match 腕順 |
| 65 | 2026-09-05 | クエリ側に `DefinitionStageDao` / `DefinitionStageView`（`stage_slug` と `support_agents` の 2 列のみ）を新設し、行が引けないこと自体を「グラフに無い」の答えにする | `C/b49-practices-receipt/design.md:153,191` | components.md | View の列数 |
| 66 | 2026-09-05 | `ProjectionTargets::new` は memory ディレクトリ 1 本を受け取る（別 space の 2 本を取り合わせられなくする） | `C/b49-practices-receipt/design.md:189` | 11 号 | `ProjectionTargets` の構築口 |
| 67 | 2026-09-05 | 裁定 A′。直近のゲート解決時刻は集約の状態として持ち（監査シャードから読むのは禁止パターン）、`HUMAN_TURN` は外部入力として合成ルートが読んで値オブジェクトで渡し、判断は集約のガード。刻むのは `GateApproved` / `GateRejected` / autonomous 付与 | `C/b50-set-autonomy/design.md:10,17-19,38,48` | 10 号 §2.3 B9、§6 I11、11 号 | `last_gate_resolution_at` の更新箇所 |
| 68 | 2026-09-05 | 裁定 A。`switch_autonomy` の状態ガードを外し park 中・完了後でも受理する。Quint の `actSetAutonomy` も同様 | `C/b50-set-autonomy/design.md:11,46,54` | 10 号 §9、formal | `switch_autonomy` のガード |
| 69 | 2026-09-05 | `HumanTurns`（`find_in` が唯一の構築経路、`DOCUMENT_*` だけの台帳は追跡なし）と `AuditEventRecord::instant()`（秒精度 ISO の解釈は行の持ち主） | `C/b50-set-autonomy/design.md:26-34,119,160` | 11 号 B9 | `human_turns.rs` |
| 70 | 2026-09-05 | `human_acted_since_gate` は 4 段判定で、同一秒のタイは fail-closed（拒否側） | `C/b50-set-autonomy/design.md:39-43` | 10 号 §6 I11、deviations #7 | 同名クエリの比較 |
| 71 | 2026-09-05 | 新しい面 `Face::Bolt`（`aidlc-bolt`）で `set-autonomy` だけ配線し、他 7 動詞は not-wired | `C/b50-set-autonomy/design.md:21,70-79` | 10 号 §10 | `cli/face.rs`、`runtime::set_autonomy` |
| 72 | 2026-08-24 | CQRS の依存境界 4 点（両側は相互独立 / RMU が要るのはドメインイベントだけ / コマンド側の最新状態は常に集約から / 境界はクレート分離で物理強制）＝ ADR-009 | `C/code-generation/memory.md:84` | 01 号、10 号、11 号、components.md | 各クレートの `Cargo.toml` 依存 |
| 73 | 2026-08-24 | ユビキタス言語 3 件（`set` は不採用 / Published Language と Ubiquitous Language の区別 / 完全コンストラクタがあれば setter は不要） | `C/code-generation/memory.md:85` | 01 号、coding-rules ubiquitous-language | `switch_autonomy`・`mark_stage` 等の名前 |
| 74 | 2026-08-27 | 楽観 version はストアが採番する不透明トークンで `seq_nr` と混ぜない（BR5.3 改訂）。内部可変性は既定で禁止 | `C/code-generation/memory.md:76,87` | 01 号 rules、11 号、coding-rules interior-mutability | Repository の `store` 署名 |
| 75 | 2026-09-05 | Tell, Don't Ask 是正でユースケースからドメイン getter を排し、判断をドメインへ移した（`ReportRequest::for_retry_at` / `Intent::resolve_review_policy` / `IntentExecution::apply_report` ほか） | `C/tell-dont-ask-remediation.md:6-21` | 10 号 §3、components.md | 各ユースケースの `execute` |
| 76 | 2026-09-05 | 関連取得は Repository の `find_for_execution` / `find_for_intent` に限定し、ID 読取は interface-adapter 内で既存 `find_by_id` へ委譲する | `C/tell-dont-ask-remediation.md:16-21` | coding-rules gateway-taxonomy / aggregate-references、components.md | Repository trait の動詞 |
| 77 | 2026-09-05 | `record_single_stage_run(&Intent, &StageSlug, t)` を唯一の公開コマンドに一本化し、拒否型は `SingleStageRunRefusal`。添字を取る旧署名・別名は残さない | `C/tell-dont-ask-remediation.md:45-55` | 10 号 §3 | 同コマンドの署名 |
| 78 | 2026-09-05 | 保存前にイベントの集約 ID と保存対象 ID を I/O 前に照合する。再構成は「最新スナップショット＋その通番より後の差分イベント」 | `C/consistency-verification-20260905.md:8,11` | 11 号、01 号、components.md | 両 Repository の store 入口 |
| 79 | 2026-09-06 | b51（u2 再走）でファーストクラスコレクション 11 型を新設し、7 並列列を `StageSlots` へ、`next_decision` を Result 化。`StageIndexSet` / `StageSlugSet` に combine / divide（Monoid 則）、`ReviewAttempt` の pending / closed も FCC 化。付随して `NextAnswerRow::of` の可謬化（`IntentUnavailable`）、`Recomposed` の文書順投影、`OutOfRange` → `ApplyError` | `C/code-generation/memory.md:89,91,92` | 01 号 §3、10 号 §2.3、11 号 §4.1、C5 | `StageSlots` / 各 FCC 型、`in_document_order` |

## 仕様への反映が未了と明記された箇所

| 出典 path:line | 何が未了か | 対象の文書 |
| --- | --- | --- |
| `C/code-generation/memory.md:95` | 機能設計・NFR 設計の本文への折り戻しが未実施で、両ゲートの `## Review` が 2026-09-06 の NOT-READY のまま残る（レビュー R-01） | functional-design / nfr-design の各成果物 |
| `C/code-generation/memory.md:94` | u2 の確定事項 4 件（`StageSlugSet` 辞書順と投影側の文書順並べ直し、`PendingIterations` の境界例外、11 型の確定事項と判断 1〜9、components.md / C3 の「ジャーナル全再生」注記の同期）を functional-design ゲートへ折り戻すこと | entities / rules / functional-spec、components.md、contract-summary C3 |
| `C/code-generation/memory.md:54` | 機能設計（entities / rules）への反映待ちとして列挙された 2026-08-23 時点の項目 | u2 の entities.md / rules.md |
| `C/b43-query-dao-lookup/inventory.md:5-7` | JOIN 前提で書かれた本文が 2026-09-03 の裁定で置き換わり、「正は `design.md`」と注記されたまま旧記述が残る | `read-model-spec.md`、同 inventory.md |
| `C/b41-rmu-run-stage-steering/design.md:76-78` | `read-model-spec.md` §4.4 のキーを「stage_slug」から「phase」へ、§4.3 の `read_config_current` を「不要」へ訂正する作業（予定として記載、完了記録なし） | `query-side-audit/read-model-spec.md` |
| `C/b48-review-receipts/design.md:175` | 仕様 B10 の旧記述（`record_review_receipt` → `ReviewReceiptRecorded`）は対の裁定で置き換わったが、経緯として旧記述を残すと明記 | `docs/specs/10-orchestration.md` B10 行 |
| `C/b46-report-guards/design.md:16,44` | 段 1 の turn-shape marker（`markEngineTouch`）はクリティカルパス 5 へ繰延、置き場と形式は未決で記録のみ | 10 号 §10、deviations |
| `C/b42-remove-ungated-completion/design.md:42` | Quint モデルの注記（`:41-44`）を「撤去済み」に更新するのは任意とし、本 Bolt では行わない | `formal/orchestration/engine_loop.qnt` の注記 |
| `C/handoff-b41.md:19-21` | `next_stage_name` に対応する集約の静的クエリが無く RMU が列を畳んでいる（本来形は集約側、Bolt 3 で判断とされたまま） | 12 号、components.md |
| `C/handoff-b44.md:61-62` | `read_next_jump_phase.next_jump_id` の FK 化が未実施（現状は自然キー引当） | 11 号 §4.1 |
| `C/handoff-b44.md:63-67` | CLI ゴールデンの駆動不能 3 ケースの扱い | `cli_golden_test.rs` の記載、deviations #1 |
| `C/handoff-b44.md:68-85` | 到達不能コードの所見 7 群（プロダクションは未変更、削除の判断を先送り） | 10 号 §10、記録のみ |
| `C/handoff-b44.md:101-102` | `cargo doc` を CI に載せるかが未裁定 | CI 設定、u10 の NFR 記録 |
| `C/handoff-b43.md:12`（→ `C/handoff-b44.md:49-52` で解消） | `docs/CLAUDE.md` の委譲ポリシー二重記載が b43 時点で未削除、b44 で memory 層への参照へ置換したと記載 | `docs/CLAUDE.md` |
| `C/b48-review-receipts/design.md:24,131`、`C/b49-practices-receipt/design.md:23,137`、`C/b50-set-autonomy/design.md:22,96` | 各 Bolt の繰延一覧（fingerprint 2 欄・stale-receipt recovery・`--unit` / `--single`・swarm 例外・`--target-dir`・`practices-event`・revision backstop・`QUESTION_ANSWERED` 等）を逸脱台帳へ記録して進む方針 | `docs/specs/deviations.md` #5 / #6 / #7 |

補足として、同じ論点で記録同士が食い違い、後の記録が前を上書きしていると明記されている箇所が 3 件ある。

- 受領証の形。`C/b46-report-guards/design.md:15` と `C/handoff-b46.md:47` は `record_review_receipt` → `ReviewReceiptRecorded` の単発と書くが、`C/b48-review-receipts/design.md:12,175` が依頼と判定の対へ置き換えたと明記（矛盾、b48 が上書き）。
- `Started` の自己完結。`C/handoff-b38.md:6-7` の「計画は `Started` で自己完結」を `C/b39-rmu-read-tables/design.md:17-18` が「誤りだった」と訂正（矛盾、b39 が上書き）。
- `directive_digest` の素材。`C/b41-rmu-run-stage-steering/design.md:15` の現行クエリ側 7 キーと `:31,51-52` の環境由来 4 キーが別物であり、`C/handoff-b41.md:17-18` が Bolt 3 での統一を申し送り（併存、統一先は 4 キー版）。

## 読めなかった・判断がつかなかったもの

- 指定された 11 ディレクトリはいずれも `design.md` 1 本のみで、`b43-query-dao-lookup/` だけが `inventory.md` を併せ持つ。指定外だが `b44` に相当するディレクトリは存在せず、記録は `handoff-b44.md` だけである。
- `code-generation/memory.md` は 96 行中、2026-08-29 以降の行が 8 行（`:9-11,17-18,23,89-96`）で、残りは 2026-08-22〜27 の記録だった。構造の規範に当たるものとして 2026-08-24 と 2026-08-27 の横断裁定（台帳 72〜74）は日付が早いものの拾っている。
- 各設計書の「§9 検証記録」に並ぶ実測値・ゲート結果・テスト件数は、指示どおりプロセス情報として除外した。
