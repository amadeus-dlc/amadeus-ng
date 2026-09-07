パス短縮: `$I` = `aidlc/spaces/default/intents/260822-stage1-selfhost`（以下すべてリポジトリ相対、行番号は `grep -n` / `awk` 実測）。

## 裁定台帳

| # | 日付 | 裁定（1 文、平易に） | 出典 path:line | 影響する仕様・共有契約の節 | コードで確認すべき点 |
|---|---|---|---|---|---|
| 1 | 08-22 | イベントソーシングを採る。機構は event-store-adapter-rs の型と API に従い、スナップショットは毎コミット更新する | `$I/inception/domain-design/decisions.md:9-27` | 01 §3、10 §2、components.md | 毎コミットで snapshot 行が進むか（`persist_event_and_snapshot`） |
| 2 | 08-22 | 集約は FSM。コマンドは単一のドメインイベントを返し、`apply_event` が状態を進める。ユースケースは進行管理のみ | 同上 `:29-49` | 10 §2.2、01 §3.3 | 集約コマンドの戻り値が単一イベントか、`Vec` 返しが無いか |
| 3 | 09-02 | 非ゲート完了の経路（`complete_stage` / `StageCompleted` / 両側 DTO / RMU 投影 / `STAGE_COMPLETED` 非ゲート文言 / commit_verdict の非ゲート腕）を撤去する。イベントは 12 → 11 変種 | 同上 `:51-56` | 10 §2.1 のイベント変種表、C5 | `IntentExecutionEvent` の変種数と `complete_stage` の有無。→ ただし #4 と突き合わせること |
| 4 | 09-06 | イベント変種は 16。旧世代 NFR の「12 変種」は後続裁定で置き換わったと記録されている | `$I/construction/nfr-requirements/memory.md:5` | 10 §2.1 | 実コードの変種数を数え、#3 の「11 変種」との**矛盾**を裁く（記録同士が食い違う） |
| 5 | 09-02 | ドメインイベントはエンティティ。各変種が自前の `id: XxxEventId`（UUIDv7）と `aggregate_id: XxxId` を別フィールドで持つ。集約 ID をイベント id に流用した形は誤り | `$I/inception/domain-design/decisions.md:58-64` | 01 §3、10 §2.1、C5 | 全変種に `id` と `aggregate_id` の 2 フィールドがあるか。採番は集約コマンド内 `generate()` |
| 6 | 09-02 | イベント ID は payload に閉じ、`seq_nr` / `occurred_at` は本家封筒が運ぶ。復号境界は行の `aid` と payload の `aggregate_id` を全変種で照合し、不一致は `Corrupt` | 同上 `:495-499` | C5、10 §2.1 | Repository 再生と RMU `decode_*` に照合があるか |
| 7 | 08-22 | ジャーナル・スナップショット・チェックポイントは SQLite。`aidlc-state.md`・監査シャード等の upstream 互換ファイルはすべてリードモデルで、RMU がプロセス内同期の差分関数として生成する | 同上 `:66-85` | 11 §2、01 §3、10 §3 | RMU が常駐でなくコマンド末尾で呼ばれるか |
| 8 | 08-22 | 集約ルートは実行集約。`aidlc-state.md` は集約でも媒体スナップショットでもなくリードモデル。01 §3 の集約候補表から `StateFile` を落とす | 同上 `:87-99` | 01 §3 集約表、11 §2 | 集約表に `StateFile` が残っていないか |
| 9 | 08-22 | `PlanAction` の所有は `workflow_definition`。`orchestration` 側の定義を削除する**完全移動**とし、利便性の再エクスポートはどこでも禁止 → 初版 ADR-005 の re-export 併存は失効（`decisions.md:105-116`） | 同上 `:101-116` | 12 §2、01 §3 | `orchestration` に `pub use workflow_definition::PlanAction` が無いか |
| 10 | 08-22 | tokio（current_thread）で async main。コントローラ・ユースケースは async、ドメインは純粋・同期のまま（`.await` を集約に出さない） | 同上 `:118-137` | 10 §3、11 §3 | 集約に `async fn` / `.await` が無いか |
| 11 | 08-22 | 永続化の動詞 `store` は ES 拡張語彙として gateway-taxonomy §2b の許容動詞表の**例外**として採用する | 同上 `:129-133` | contract-summary C3、coding-rules/gateway-taxonomy | ポート動詞が `store` / `find_by_id` に揃っているか |
| 12 | 08-22 | mkdir ロック機構を退役。書込は SQLite Tx + 楽観 version、リードモデル書込は tmp+rename の冪等生成。`FsWorkspaceLock` / `WorkspaceLock` / `LockProtocol` / `reap_eligible` / `OwnerStamp` を削除 | 同上 `:139-157` | 11 §2/§3、01 §3.3、deviations | ロック dir を生成しないこと、退役 5 型が残っていないこと |
| 13 | 08-23 | `WorkflowDefinition` はエンティティ。不変の系譜 ID `WorkflowDefinitionId` と値属性 `DefinitionRevision` を持ち、`find()` は廃止して `find_by_id` に一本化。実行集約は `definition_id` / `definition_revision` を `Started` に記録 | 同上 `:159-178` | 12 §2.1、01 §3、C4 | 引数なし `find()` が残っていないか、`Started` に 2 値が載るか |
| 14 | 08-23 | `start` は定義の id / revision を無条件に記録するだけ（検査しない）。`Started` 適用後に `&WorkflowDefinition` を受けるクエリは id 不一致を `Err` | 同上 `:173-176` | rules.md BR2.6、10 §2.3 | `start` に定義検査が無いこと、`next_decision` に id ガードがあること |
| 15 | 09-02 | 内容版はドメインが導出する（`DefinitionRevision::of_content(graph, grid, scopes)`）。生バイト・未知フィールド・キー順は入力に含めない → ADR-008 の「Repository が計算」は失効 | 同上 `:190-203` | 12 §2.1、C4 | `of_content` が集約側にあるか。**注**: `$I/construction/command-domain-audit/audit-1.md:146-147` は「Repository / 投影側が計算」と書いており矛盾。日付は本行が後 |
| 16 | 09-02 | 配布束は集約 `CompiledDefinition`（`CompiledDefinitionId`、`recompile` / `register_scope` / `apply_plugin_selection` の 3 遷移）。ジャーナルの定義は `define(id, &CompiledDefinition, at)` / `redefine(...)` で内容と内容版を受け取り、系譜違いは `LineageMismatch` | 同上 `:190-203` | 12 §2.1 | `CompiledDefinition` の存在と 3 遷移、`LineageMismatch` |
| 17 | 08-29 | 解決済み計画の表示属性（`StageDisplay` 3 値）と走査結果（`WorkspaceScan`）はイベントへ焼き込む例外。定義全体の複製は引き続き禁止 | 同上 `:204-213` | C5、NFR3 | `StageDisplay` / `WorkspaceScan` の焼き込み先 |
| 18 | 09-02 | → **失効**: 走査結果と表示属性は `IntentEvent::Created` が運ぶ。`Started` は `id` / `intent_id` / `stages`（解決済み計画の写し）を運ぶ形へ是正。自ストリームだけで実行集約を再生するため | 同上 `:214-222` | C5、10 §2.1、NFR3 | `Started` の payload 3 要素、`Created` が scan と display を運ぶこと |
| 19 | 08-24 | CQRS の依存境界をクレートで物理強制する。コマンド側とクエリ側は相互に依存せず、RMU だけが両側に依存してよい。判定は `Cargo.toml` を見るだけでよい | 同上 `:231-333` | 10 §3、11 §3、cqrs-boundaries | 各 `Cargo.toml` の依存に側越えが無いか |
| 20 | 08-28 | RMU コンポーネントが取得ループを持つ（`events_after` で差分を引き、純粋投影核へ渡し、`advance_checkpoint` を自分で進める）。`JournalReader` / `ProjectionName` / `GlobalSeqNr` / `JournalReadError` の所有は use-case → RMU クレートへ移す → ADR-009 初稿の「中立クレート不要の理由 / U4 責務の縮小」は失効 | 同上 `:266-285` | 10 §3、11 §3、unit-of-work U4 | `JournalReader` trait の所有クレート、取得ループの所在 |
| 21 | 08-29 | U7（合成ルート）は RMU の**起動のみ**を持ち、駆動ループを持たない（カバレッジ除外領域に実ロジックを置かないため） | 同上 `:279-281` | 11 §3 | 合成ルートにバッチ・チェックポイント制御が無いこと |
| 22 | 08-29 | interface-adapter / use-case をコマンド側とクエリ側に分割し、`core-{command,query}-` 接頭辞で統一。`JournalReaderImpl` は RMU クレートに置く。両側を知ってよいのは合成ルートだけ | 同上 `:286-296` | 10 §3、11 §3 | クレート名と `JournalReaderImpl` の所在 |
| 23 | 08-29 | ドメインはコマンド側の持ち物（`modules/core/command/domain`）。クエリ側クレートはドメインに絶対依存しない。RMU は中間なので `core-read-model-updater` とする | 同上 `:297-307` | 10 §3、11 §3、components.md | クエリ側 `Cargo.toml` に domain が無いこと |
| 24 | 08-29 | `modules/shared` を解体。監査イベント語彙 86 語と `DirectiveKind` はドメイン知識として `core-command-domain` へ、`canon-json` は `core_infrastructure` へ、文言は出す側（RMU の `wording` / アダプタの `cli_wording`）へ。ドメインの pub 型がそのまま公開言語 | 同上 `:308-318` | 01 §3、02、11 §2 | `modules/shared` の不在、`InvalidModeArg` が材料だけ運ぶこと |
| 25 | 08-29 | 状態ファイル骨格の生成は投影の責務外。骨格は intent-create 時の環境成果物で、書くのは合成ルート。RMU は差分適用に徹し、骨格が無ければ `ProjectionError::ScaffoldMissing` で止める | 同上 `:319-328` | 11 §2、NFR3 | `ScaffoldMissing` の存在、骨格生成の所在 |
| 26 | 08-26 | event-store-adapter-rs へ乗り換える。Conformist を採り腐敗防止層は置かない。ドメイン型が本家 trait を直接実装し、`chrono` と `usize` を受け入れる → ADR-006 の「crate 直接依存の見送り」は撤回 | 同上 `:351-380` | C3、10 §3、NFR4.1 | 本家 crate への直接依存とピン |
| 27 | 08-26 | 全集約横断の順序読取はライブラリのサポート外。本家 `journal` 表の rowid を別接続で読み、チェックポイントは自前表に持つ。本家スキーマへの結合はバージョン完全固定とガードテストで守る | 同上 `:381-391` | C6、11 §3 | 別接続・rowid カーソル・スキーマガードの実在 |
| 28 | 08-28 | rowid は削除ゼロの純追記ゆえ VACUUM 後も同値。多層防御としてチェックポイント表に `(aid, seq_nr)` アンカーを併記し、不一致は `Corrupt(CheckpointAnchorMismatch)` で拒否 | 同上 `:393-406` | C6 | `anchor_aid` / `anchor_seq_nr` 列と照合分岐 |
| 29 | 08-27 | `EventStoreImpl` を削除。コマンド側は `IntentExecutionRepositoryImpl<S>`、読取側は別接続の `JournalReaderImpl`。我々が定義する表は `amadeus_projection_checkpoint` 1 表のみ → ADR-003 / ADR-007 / ADR-009 の当該記述を supersede | 同上 `:408-416` | C6、10 §3、11 §3 | `EventStoreImpl` の不在、SQLite の表構成（journal / snapshot は本家、我々は 1 表） |
| 30 | 08-26 | 登録簿 `intents.json` の read-modify-write は本家経由では守れない。案 (b)（登録簿を SQLite の表へ移す）が筋に見えるが、U7 と併せて判断する — **未決** | 同上 `:417-436` | 11 §2.2/§3/§10、BR2.4、NFR3.5 | `within_write_transaction` 相当の口が存在しないこと |
| 31 | 08-27 | genesis の初期 version は Gateway が写しに 1 を載せる。`Conflict` の `actual` は競合時のみ読み直しで得る。`busy_timeout` は本家接続に設定不可で、単一プロセス前提を受容する | 同上 `:448-454` | C3 ⑥、BR2.1、NFR3.5 | ただし #32 で `set_version(1)` は消滅済み |
| 32 | 08-29 | v3.0.0 へ乗り換える。`Event` / `Aggregate` trait は廃止され `EventEnvelope` / `SnapshotEnvelope` に置換。ドメインイベントは輸送メタデータを持たない素の payload になり、旧封筒 struct と `WorkflowExecutionEventId` を削除 → `=2.0.0` は失効 | 同上 `:462-474`、`$I/construction/esa-v3-migration/developer-report-1.md:18-34` | C3、C5、10 §2.1、11 §3 | 封筒を組むのが Repository であること、payload に輸送メタが出ないこと |
| 33 | 08-29 | 楽観 version を集約と memento から削除し集約の外を持ち回る。`find_by_id` は再構成レコード（集約 + ストア採番 version）を返し、`store` は `expected_version: usize` を取る（初稿の `persist_event(envelope, snapshot.version())` は TOCTOU で撤回） | `$I/inception/domain-design/decisions.md:476-483`、`$I/construction/esa-v3-migration/brief-1.md:47-71` | C3、C6、BR5.3 | 集約と memento に `version` が無いこと、`store` の引数 |
| 34 | 08-29 | 更新も `persist_event_and_snapshot` を使う（v3 の `persist_event` は snapshot の seq_nr を進めず Quint 不変条件 `snapshot_tracks_journal` を破るため）。genesis / 更新の分岐は `event.seq_nr == 1` から導出 | `$I/inception/domain-design/decisions.md:485-490` | C3、journal_protocol.qnt | 分岐の導出元 |
| 35 | 08-29 | manifest 列の値は `intent-execution-event/1`（旧 `workflow-execution-event/1` は B12 改名で追従）。Repository が書き、JournalReaderImpl が不一致・欠落を `Corrupt(UndecodablePayload)` で拒否 | 同上 `:472-474`、`$I/construction/intent-aggregate-rename/developer-report-1.md:206-239` | C5 | 定数値と拒否分岐。仕様側に旧綴りが残る（後述） |
| 36 | 08-29 | `JournalReader` は本家 `EventEnvelope` をポートから出さず、自前の `JournalEntry`（global_seq / 集約 id / seq_nr / occurred_at / event）を返す | `$I/construction/esa-v3-migration/brief-1.md:75-81` | C3 | ポート戻り値の型 |
| 37 | 09-02 | CLI 読取コマンド向けリードモデルはイベントストアと同じ SQLite の `read_*` 表。RMU が 3 ストリームから `replay` で集約を起こし、クエリメソッドの答えを非正規化して書く。行の差し替えとチェックポイント前進は RMU 側接続の同一 Tx | `$I/inception/domain-design/decisions.md:501-523` | 11 §4.1（新設）、10 §3 | `read_*` 表の実在と Tx 範囲 |
| 38 | 09-02 | クエリ側の DAO はキーで引くだけ（`WHERE` は可、行に無い事実の導出は不可）。判断・導出・選択・文言組立をしない | 同上 `:510-513`、`$I/construction/query-side-audit/read-model-spec.md:15-21,186-196` | cqrs-boundaries 規則 6、10 §2.3 | DAO とクエリユースケースに分岐・計算が無いこと |
| 39 | 09-02 | 判断は集約へ戻す。`next_decision` / `jump_resolve` は集約のクエリメソッド → b26 でクエリ側へ移した実装は誤りで、仕様 10 §2.3 の側が正しい | `$I/construction/query-side-audit/audit-1.md:14-15,27-33,66-69,84-85` | 10 §2.3 | `IntentExecution::next_decision` / `jump_resolve` の実在 |
| 40 | 09-02 | RMU が定義ストリームも購読する（従来の読み飛ばしを撤去）。`scope_cost` は `WorkflowDefinition` に新設し RMU が呼ぶ | `$I/inception/domain-design/decisions.md:520-521`、`$I/construction/query-side-audit/read-model-spec.md:58,64,156` | 11 §4.1、12 §2 | 定義ストリームの購読、`scope_cost` の所在 |
| 41 | 09-02 | 表カタログ: `read_definition` / `read_definition_stage` / `read_definition_scope` / `read_definition_scope_keyword` / `read_definition_scope_stage` / `read_definition_scope_phase_entry` / `read_intent` / `read_intent_stage` / `read_execution` / `read_execution_stage` / `read_next_answer` / `read_next_jump` / `read_next_jump_phase` / `read_run_stage` / `read_scope_change` / `read_steering_plan` / `read_steering_part`。全行に `as_of_global_seq` | `$I/construction/query-side-audit/read-model-spec.md:52-94` | 11 §4.1 | 表名・キー・列の一致 |
| 42 | 09-03 | `read_run_stage` のキーは `definition_id × scope × stage_slug`（材料は実行状態に依存せず `gate` も phase 由来）。`read_config_current` は作らない（config-change は現在値を見ない構文分岐） | 同上 `:85,87` | 11 §4.1 | キー構成、config-change の分岐条件 |
| 43 | 09-03 | steering の束は `org.md` → `team.md` → `project.md` → `phases/<phase>.md` と phase だけで決まる。ステージの `rules_in_context` は入力ではなく run-stage が返す配信済みパス台帳。`read_steering_plan` のキーは phase | 同上 `:89-94` | 02 §10、11 §4.1 | 束選択の入力と `read_steering_plan` のキー |
| 44 | 09-02 | `SteeringPlan::pack`（分割・パック）は RMU の投影ヘルパへ移す。steering の参照入力は `catch_up` ごとのダイジェスト比較で変化時だけ再投影 | 同上 `:96-97,142-143,195` | 11 §4.1 | `pack` の所在 |
| 45 | 09-02 | `continue` は新しい表を要さない。トークンの 4 束縛を既存表の同名列と `WHERE` で突き合わせるだけで、再構築も照合ロジックもクエリ側に無い | 同上 `:99-106` | 02 §4、11 §4.1 | `continue` 経路に再構築が無いこと |
| 46 | 09-02 | advisory マーカー（MAC 鍵など）の書込は合成ルートの機構モジュールが担う | 同上 `:196` | 11 §3 | マーカー Gateway の所在 |
| 47 | 08-29 | 「読取モデル集約」「読取専用集約」の呼称を廃止し**集約**に統一。`WorkflowDefinition` は集約であり、変異がスコープに入った時点で状態遷移はイベントを吐く | `$I/construction/command-domain-audit/audit-1.md:96-102` | 12 §2.1、01 §3 | 仕様 12 から「読取モデル集約」の語が消えているか |
| 48 | 08-29 | 実ファイル（`stage-graph.json` / `scope-grid.json` / scope カタログ）が `WorkflowDefinition` のリードモデル。配置は現状維持で `WorkflowDefinitionRepository` の名も維持 → 監査 B 群の「要再配置候補」は上書き | 同上 `:103-114` | 12 §2.1、C4 | ポート名と B 群の所在 |
| 49 | 08-29 | `WorkflowDefinition` の将来形（`compose_scope` → `ScopeComposed`、自前ジャーナル、実ファイルの投影化）はオーナー承認済みの設計方針。ただし実施は stage-1 スコープ外 | 同上 `:135-151` | 12（申し送り） | 現状は find のみであること |
| 50 | 08-29 | 集約 `WorkflowExecution` を `Intent`（静的）と `IntentExecution`（実行時）へ分割する。1 intent : n 実行が現在の意味論 | `$I/construction/intent-aggregate-rename/brief-1.md:101-130` | 01 §3、10 §2.1、12 | 2 集約の実在と `IntentExecutionId` |
| 51 | 08-29 | 集約は `Intent` を埋め込まず ID で参照する。実行集約が保持するのは `id` / `intent_id` / cursor / 実行時ベクトル / autonomy / parked_at / seq_nr のみで、`stages` / `scope` / `request` / `scan` / `definition_id` / `revision` は持たない → 改訂 2 の埋め込み形は誤り | 同上 `:151-182`、`$I/construction/intent-aggregate-rename/developer-report-1.md:78-85` | 01 §3、10 §2.1 | 集約フィールドと snapshot の属性一覧（12 属性） |
| 52 | 08-29 | 計画が要るコマンド・クエリは `&Intent` をパラメータで受け、`intent.id() == self.intent_id` と長さ一致をガードして不一致は `Err`。実効プランは `intent.stages[i].plan_action ⊕ overlay[i]` で導出 | `$I/construction/intent-aggregate-rename/brief-1.md:167-170`、`developer-report-1.md:285-296` | 10 §2.2/§2.3 | ガードの位置（入口 1 か所）と `IntentMismatch` |
| 53 | 08-29 | Repository の署名は自集約の ID だけを取り、他の集約・エンティティを引数にも戻り値にも出さない。再生材料は自ストリームの誕生イベントから内部復元する → 改訂 4（A+）・改訂 5（`&Intent` 引数）は失効 | 同上 `:247-263` | C3、gateway-taxonomy | `find_by_id(&IntentExecutionId)` の署名、再構成レコードが Intent を載せないこと |
| 54 | 08-30 | `Intent` も集約。`IntentEvent::Created`（材料 = intent 全属性）を新設し `Intent::create(...) -> (Intent, IntentEvent)` の対を返す。再構成経路は無イベント | 同上 `:302-325` | 01 §3、10 §2.1 | 対返しファクトリと再構成コンストラクタの分離 |
| 55 | 08-29 | `WorkflowDefinitionEvent::Defined { id, revision }` を新設し `define(...)` が対を返す。実ファイル読取は genesis ではなく再構成（`from_artifacts`）でイベントを生成しない。内容フルは焼かない | 同上 `:277-298` | 12 §2.1 | `define` / `from_artifacts` の分離 |
| 56 | 08-30 | ドメインから永続化知識を全撤去する。serde・`event_manifest`・本家 crate 依存・`AggregateId` 実装を domain から外し、永続化 DTO はアダプタが所有する。RMU は自前の復号 DTO を持つ → ADR-010 の「serde 受容」は失効 | 同上 `:329-367`、`$I/inception/domain-design/decisions.md:441` | 01、10 §3、BR5.2、NFR4.1 | `core-command-domain/Cargo.toml` に serde と本家 crate が無いこと |
| 57 | 08-30 | `chrono` は残る（時刻の値は永続化知識ではない）。監査語彙 86 語と `StorePath` は AI-DLC の概念なので domain 残留が正 | `$I/construction/intent-aggregate-rename/brief-1.md:361`、`developer-report-1.md:564-568` | 01 §3 | domain に残る依存の線引き |
| 58 | 08-30 | 書込ユースケースはリポジトリを保持し `execute` 内部で使う。`execute` の引数に集約を渡さない（集約 ID と値オブジェクトのみ）— 一般則。読取専用の型保証は find 系動詞しか持たない読取専用ポートの注入へ置換 → I8（Controller が集約を `&` で渡す）は失効 | 同上 `:371-421` | use-case-rules §2b/§4、10 §3 | `CommitVerdictUseCase<E, I>` の 2 ポート保持、`Next` の読取専用ポート注入 |
| 59 | 08-29 | ユースケース名は `CommitVerdictUseCase`、`execute` は `Result<(), CommitError>`。`ReportOutcome` は削除し CQS 例外は却下。`Resumed` は入力型から外し U7 が手前で分岐 | `$I/construction/u5-report-use-case/decisions-1.md:210-243` | 10 §3、C5 | 戻り値の形と `Resumed` の不在 |
| 60 | 08-29 | フェーズ境界は集約が内部導出する（承認ステージの phase と次の実効 EXECUTE ステージの phase を比較、同一または次が無ければ `None`）。外形は 1 バイトも変えない | 同上 `:32-46`、`$I/construction/u5-report-use-case/developer-report-1.md:114-150` | C5、FR1.1 | `crossed_phase_boundary` の実在と `GateApproved` payload の不変 |
| 61 | 08-29 | 投影キャッチアップの起動は合成ルート（U7）。コマンド側ユースケースは RMU に依存しない | 同上 `:9-30` | unit-of-work U5/U7、cqrs-boundaries | `core-command-use-case/Cargo.toml` に RMU が無いこと |
| 62 | 08-29 | `StoreVersion` newtype 化は却下（Conformist 維持）。ポート面の `expected_version` は `usize` のまま → ただし B16 で自前 VO `StatePosition` 内だけ `StoreVersion` を使う形で両立 | 同上 `:48-61`、`$I/construction/u6-next-continue/brief-1.md:114-117` | C3、BR5.3 | ポート署名の `usize` と `StatePosition` の内部型 |
| 63 | 08-29 | `RepositoryError::Corrupt` は `{ aggregate_id, seq_nr, source }` とし、原因分類はアダプタ私有へ。use-case 層の `corrupt_cause.rs` は削除 → 「一旦許容」は上書き | `$I/construction/u5-report-use-case/decisions-1.md:178-208` | C3、error-handling | `corrupt_cause.rs` の不在と `source` 連鎖 |
| 64 | 08-29 | 楽観 version 競合は 1 回だけ再試行し、再試行は再構成からやり直す。2 回目も競合なら伝播 | `$I/construction/u5-report-use-case/brief-1.md:96-104` | C3 ③、Q6 | 再試行回数と再構成のやり直し |
| 65 | 08-29 | CLI の出力データはコマンドユースケース → RMU → クエリユースケースの経路で得る。クエリ側はドメインに絶対依存せず、文言は出す側が組む | `$I/construction/u5-report-use-case/decisions-1.md:250-282` | 10 §3、11 §4 | クエリ側の依存と文言の所在 |
| 66 | 08-29 | RMU の投影は非同期タスクで走る。合流点は 2 つ（upstream 互換の md / 監査シャードはプロセス終了前に join、クエリ向き表はクエリ直前に join） | 同上 `:296-312` | 11 §3、NFR3 | spawn と join の位置 |
| 67 | 08-29 | 集約の再構成は ES 本則へ整列: 構築 API は genesis / replay / apply_event のみ、イベントは値を運ぶ、再構成は失敗を返さない（壊れた歴史はクラッシュ）、memento 双子型禁止、エラーは `RepositoryError<Id>` 1 本 | `$I/construction/functional-design/memory.md:20` | 01 §3、C3、10 §2 | 構築 API の 3 本、`RepositoryError<Id>` のジェネリック化 |
| 68 | 09-05 | Repository の読取は最新スナップショット + 差分イベントの再生が正 → #67 の「ジャーナル全再生」は失効 | 同上 `:12`、`:6` | components.md、C3 | `find_by_id` が全再生でないこと |
| 69 | 09-06 | コマンド側のドメインモデルの配列はファーストクラスコレクション化する（`StageEntries` / `StageSlots` / `StageIndexSet` / `ArtifactPaths` / `StageSlugSet` 等、BR5.5）。リードモデル側では FCC を使わず境界で列挙する | 同上 `:6`、`:36` | 01 §3、10 §2.1 | FCC 型の実在、位置ごとの 7 並列列が `StageSlots` に統合されているか |
| 70 | 09-06 | `next_decision` は `Result<NextDecision, CommandError>`（`IntentMismatch`）へ揃える | 同上 `:6` | 10 §2.3 | 戻り値の型と呼出元（RMU の `next_answer_row.rs`）の追随 |
| 71 | 08-30 | `Directive` は 10 kind の判別共用体で構築不能な placeholder を持つ。逐語文言は公開契約なので use-case の `wording` に置く | `$I/construction/u6-next-continue/brief-1.md:19-21,25-26` | 02 §4.4、10 §3 | `Directive` の kind 数と `wording` の所在 |
| 72 | 08-30 | `ContinueToken` は 18 キー厳密型表・HMAC-SHA256 封筒 `{p,m}`・base64url・timing-safe 検証。暗号はアダプタのみで domain / use-case は封筒に依存しない | 同上 `:40-44,65-76` | 02 §4、11 §3 | codec の所在と依存 |
| 73 | 08-30 | B16 是正 3 原則: すべてドメインプリミティブ化、ユースケース層はフロー制御、貧血補償コード禁止。`scope_resolution` はドメインへ、steering の分割・パックはアダプタへ、`command_spelling` は `EngineCommand`（ドメイン）+ ポート + アダプタ実装へ分離 | 同上 `:82-113` | 10 §3、02 §10 | 各モジュールの所在 |
| 74 | 08-30 | ポート契約 trait と契約依存型は `use-case/src/orchestration/port/` に集約する（公開ファサードは据え置き） | 同上 `:118-121` | 10 §3 | `port/` ディレクトリの構成 |
| 75 | 08-27 | `IntentId` の `value()` は外部 trait 実装として tell-dont-ask の例外。doc に理由を書く | `$I/construction/esa-v2-migration/brief-1.md:39-42` | ubiquitous-language | `value()` と `as_str()` の並立（申し送り扱い） |
| 76 | 08-27 | `last_updated_at` は適用したイベントの `occurred_at` を置く（集約は時計を読まない）。イベント ID は決定的採番 → v3 で封筒へ移行し #5 / #6 が現行 | `$I/construction/esa-v2-migration/developer-report-1.md:188-211` | NFR3.1、BR2.3 | 集約に時計・乱数が無いこと |

## 仕様への反映が未了と明記された箇所

| 出典 path:line | 何が未了か | 対象の文書 |
|---|---|---|
| `$I/construction/functional-design/memory.md:6` | components.md / contract-summary C3 の「ジャーナル全再生」が 2026-09-05 裁定（最新スナップショット + 差分再生）で上書き済みだが同期は積み残し | `components.md`、`contract-summary.md` C3 |
| `$I/construction/functional-design/memory.md:21` | specs 01 / 10 / 11 / 12 の全文追従（ES 再構成の意味論を含む）は「引き続き候補」のまま | `docs/specs/01,10,11,12` |
| `$I/construction/query-side-audit/audit-1.md:70-73` | 仕様 10 §3 の「CQRS は採用しない」「`Next` に Repository を注入しないことで型強制」が旧記述のまま。cqrs-boundaries 規則 6 に「読む = 判断・導出をしない」が未明文化 | `docs/specs/10` §3、`coding-rules/cqrs-boundaries.md` |
| `$I/construction/query-side-audit/read-model-spec.md:198-205` | 規則 6 と規則 3 への追記、仕様 10 §3 の失効注記、仕様 11 への構造化リードモデル節の追加がいずれも「要更新」 | `cqrs-boundaries.md`、`docs/specs/10` §3、`docs/specs/11` |
| `$I/inception/domain-design/decisions.md:522-523` | クエリ側の Markdown 逆パース・配布 3 ファイルのパース・判断型の削除は Bolt 3 で行う。表カタログの正本は仕様 11 §4.1 と b39 design.md §4.1 | `docs/specs/11` §4.1 |
| `$I/inception/domain-design/decisions.md:56` | 非ゲート完了パイプラインの撤去（12 → 11 変種）は「是正 Bolt 2 の後」の独立 Bolt で行う | 実装 + 10 §2.1 |
| `$I/construction/command-domain-audit/audit-1.md:86-88,109-110` | specs/12 の「読取モデル集約」除去とスライス 2（`StageDefinition` / `AgentPersona`）からの集約の語の剥がしが論点 3。呼称の残存 3 箇所は是正済みと記載 | `docs/specs/12` §2.1、L37 |
| `$I/construction/command-domain-audit/audit-1.md:129-133,149-151` | スコープ合成をコマンド側へ取り込む時点で `ScopeComposed` 等のイベント設計と実ファイルの投影化が必須。実施は stage-1 スコープ外の後続 intent | `docs/specs/12`（申し送り） |
| `$I/construction/intent-aggregate-rename/developer-report-1.md:515-519` | `security-design §2`「検査点は `from_snapshot` の 1 か所」と `entities.md` の `WorkflowExecutionState` 節（型名・属性数 17→12・Builder 名・エラー名）が失効したまま | U3 nfr-design `security-design.md` §2、U3 `entities.md` |
| `$I/construction/intent-aggregate-rename/developer-report-1.md:520-523` | `docs/specs/10-orchestration.md:44` と `decisions.md:435` / `contract-summary.md` 271・291・374 行に旧 manifest 綴り `workflow-execution-event/1` が残存 | `docs/specs/10`、`decisions.md`、`contract-summary.md` |
| `$I/construction/intent-aggregate-rename/developer-report-1.md:534-537` | `WorkflowDefinitionEvent` / `IntentEvent` の永続化先が未接続。intent / 定義のジャーナルには別の manifest 値が要る | 実装 + C5 |
| `$I/construction/intent-aggregate-rename/developer-report-1.md:542-545` | `formal/orchestration/journal_protocol.qnt` の対応表コメント 5 行が古い。`docs/**` と `coding-rules/**` の旧名はメインセッション担当 | `formal/`、`docs/`、`coding-rules/` |
| `$I/construction/intent-aggregate-rename/developer-report-1.md:558-560` | `docs/specs` と inception 期成果物に「ドメイン層に serde が入る」記述が残っている可能性（未確認） | `docs/specs`、inception 成果物 |
| `$I/construction/intent-aggregate-rename/developer-report-1.md:528-533` | 「この intent の現在の実行」の解決と「同一 intent の生きた実行は同時に 1 つ」の不変条件が未決（U6 / U7 の設計点） | `docs/specs/11`、U7 設計 |
| `$I/construction/esa-v2-migration/doc-sync-report.md:109-119` | NFR3.2 の `PRAGMA user_version` 記述、NFR3.5「登録簿の直列化 — `within_write_transaction`」（実現不能）、DoS 緩和の `busy_timeout` 依存、traceability.json の NFR3.5 target、NFR4.1 の再検討が未編集 | U3 `nfr-requirements/security-requirements.md:32,35,51`、`nfr-design/traceability.json:17` |
| `$I/construction/esa-v2-migration/doc-sync-report.md:123-133` | coding-rules 正本への追記 3 件（BR1.7 の射程、`thiserror` の推移依存、`value()` と `as_str()` の並立）と NFR4.1 の再検討そのもの | `coding-rules/`、NFR4.1 |
| `$I/construction/esa-v2-migration/doc-sync-report.md:134-149` | `security-design.md` §2 検査点 2 の縮退（実地確認と要否の裁定が未了）、`entities.md` / `functional-spec.md` に残る `WorkflowExecutionSnapshot` 旧名 | U3 `security-design.md` §2、U2 `entities.md` / `functional-spec.md` |
| `$I/construction/esa-v2-migration/doc-sync-report.md:151-159` | BR2.4 / `intents.json` の直列化と `busy_timeout` は「未決（U7 で裁定）」と明記。11 号 §10 の「確定」は打ち消して未決へ差し戻し | `docs/specs/11` §10、C3 / C6 未解決表、U3 rules BR2.4 |
| `$I/construction/esa-v3-migration/developer-report-1.md:400-403` | `docs/specs` の C5 / C6 記述をどの Bolt で追随させるかが確認事項（doc-sync-report.md:44-45 では解消済みと記載され、両者が食い違う） | `contract-summary.md` C5 / C6 |
| `$I/construction/u5-report-use-case/decisions-1.md:94-97` | FR2.2 の「B10 述語」が何を指すか文書内に定義が無く、レシート新鮮判定という読みは推定。着手前にオーナー確認が要る | Issue #7 3-B、FR2.2 |
| `$I/construction/u5-report-use-case/decisions-1.md:274-282,311-312` | 複数投影の駆動（現行 RMU は単一投影）とテーブルの具体設計は U7 の課題。常駐プロセスでの連続投影を要件に含むかは回答待ち | `docs/specs/11`、U7 設計 |
| `$I/construction/u5-report-use-case/developer-report-1.md:435-442` | slug → `StageIndex` の公開クエリが集約に無く同ロジックが 2 か所にある。`skip_stage` のフェーズ境界（最終ステージ skip の `PHASE_COMPLETED`）は payload にも RMU にも無い既知の穴 | 10 §2.3、FR1.1 |
| `$I/construction/u6-next-continue/brief-1.md:46-51` | ディスパッチャ語彙の完全 ROUTES 写し（30 経路 + `SLASH_FLAG_ALIASES`）、run-stage の conductor_persona 焼き込み、active-directive マーカーは U7 へ | 02、11 §3 |
| `$I/construction/nfr-requirements/memory.md:11` | U2 の advisory レビュー R-01〜R-03（NFR2.5 の合格基準が成立しない、NFR1.1 の追随対象漏れ、`next_decision` の Result 化で壊れる呼出元の未列挙）は成果物凍結のため未反映。ステージゲートの Request Changes で折り込む | U2 `nfr-requirements` 成果物 |

## 読めなかった・判断がつかなかったもの

なし。指定された全ファイルを読了しました。ただし次の 3 点は記録同士が食い違っており、判定は依頼者側でお願いします。

1. **イベント変種の数**。`decisions.md:55` は「12 → 11 変種」（非ゲート完了の撤去、2026-09-02）、`nfr-requirements/memory.md:5` は「16 変種」（2026-09-06）。両者の間に変種を増やす裁定を記録上は見つけられませんでした。
2. **`DefinitionRevision` の計算主体**。`decisions.md:190-203`（b36、2026-09-02）は「集約が導出する」、`command-domain-audit/audit-1.md:146-147`（2026-08-29）は「Repository / 投影側が計算（ADR-008 維持）」。日付では前者が後です。
3. **C5 / C6 の仕様同期の状態**。`esa-v3-migration/developer-report-1.md:402` は「どの Bolt で追随させるか」を確認事項として残し、同じ Bolt の `doc-sync-report.md:44-45` は「解消済み」と書いています。

なお `decisions.md:1` の見出しは「ADR-005 は 2026-08-22 再入で完全移動へ改訂」とあり、ADR-001〜011 の有効・失効の扱いは `decisions.md:224-229` の「ADR ステータス注記」に集約されています。初版 ADR-001〜006 は現行版が全面的に置き換え、ADR-006 の crate 見送りは ADR-010 が撤回、ADR-003 / 007 / 009 の `EventStoreImpl` 前提は ADR-010 の 2026-08-27 追記が supersede、ADR-008 の revision 計算主体は b36 が改訂、ADR-009 の初稿 2 項目は 2026-08-28 改訂で失効、という関係です。
