# セルフホスト切替の契約差分

## 適用範囲と正本

[要求書](../requirements-analysis/requirements.md)、[4単位の定義](../units-generation/unit-of-work.md)、[依存関係](../units-generation/unit-of-work-dependency.md)、[契約確認](contract-design-questions.md)を入力とする。既存CLI・SQLite・RMU（イベントから読取りモデルを構築する処理）の構造を保ち、必要な差分を定める。

外部の観測契約は本家2.7.1の固定コミット`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`を正本とする。配布はvendorのforkを継続使用する。新しい採取先`tests/golden/upstream-a277af21/`はU1が作成する予定であり、現時点で採取済みとは扱わない。

CQS（更新と読取りを分ける規則）について、**reportの結果材料を返す例外は採用しない**。集約のイベント生成、RepositoryによるSQLiteへの追記、RMUによる投影、クエリAPIによる取得を分ける。コマンドの成功結果を戻り値で呼出側へ運ぶ現経路と、その正当化コメントはU2で是正する。[Q1]

以下のYAMLは、CLI・共有データ・内部APIの契約である。新しいHTTPサービス、別データベース、配布一般化を導入する仕様ではない。イベントや読取りモデルの論理フィールドを、ドメイン型へSQLや直列化属性を持ち込む理由にしない。

## 契約一覧

| # | Provider Unit | Consumer | Mechanism | Owner |
| --- | --- | --- | --- | --- |
| C1 | U1 | U2・U3・U4 | 採取データ・比較規則 | U1 |
| C2 | U2 | External: Claude Codeの作業手順 | CLIの引数・標準入出力・終了状態 | U2 |
| C3 | U2 | External: Claude Codeのフック実行 | JSON入力とフック別の応答 | U2 |
| C4 | U2 | U2の出力側・クエリ利用者 | イベント追記・RMU投影・報告結果クエリ | U2 |
| C5 | U2 | U3 | 状態・監査・配布構成の診断材料 | U2（正本）、U3（診断契約） |
| C6 | U2 | U4・再利用する補助処理 | CLI接続と投影済みデータの読取り | U2（更新）、U4（接続） |
| C7 | U3 | U4・External: 開発者 | 自己診断CLI | U3 |
| C8 | U4 | External: 開発者とClaude実行環境 | 接続設定・バイナリ識別・切替準備 | U4 |

## C1: 本家2.7.1の採取・比較契約

```yaml
contract: C1
format: acceptance-corpus
source:
  repository: https://github.com/awslabs/aidlc-workflows
  commit: a277af218f0df7f325d3b8be7b6d90fce2c5bd40
  version: 2.7.1
  distribution: dist/claude
corpus_root: tests/golden/upstream-a277af21
case_input: [argv, stdin, initial_files, environment]
case_expected: [exit_code, stdout_bytes, stderr_bytes, state_delta, audit_delta]
provenance: [source_commit, tree_manifest_sha256, captured_at, tool_versions, capture_command, environment]
comparison:
  default: bytes
  doctor: selected_checks_defined_by_C7
  nondeterministic_values: explicit_symmetric_normalization
  fixed_words_and_ids: preserve
  unknown_difference: fail
```

配布JSONと採取した期待出力は本家から得たバイトを保存する。正規化は実測した環境依存値に限定し、期待値と実測値の両方へ同じ規則を適用する。固定コミット、要求・段階の識別子、状態値、監査語彙を「似た形だから」と消さない。入力間のID対応は保持して検証する。

正常系に加え、不正入力、古い受領、変更後の再利用、投影前後の失敗等、今回実装する経路の拒否条件を採取・比較する。対話を含まない採取用設定と、本番相当の実地スモークを区別する。採取時に無効化した検査があれば来歴へ記録し、そのケースだけで当該保護が働くとは主張しない。

U1は採取と比較基盤、U2以降は対応実装とテストの移行を担当する。旧2.6.40の参照・比較項目を現行受入の根拠として残さない。差分を全数確認し、未対応や未裁定の差を成功扱いしない。

## C2: CLIの公開契約

```yaml
contract: C2
format: cli
transport: argv-and-standard-streams
operations:
  progression: [intent-create, next, continue, report]
  receipts: [decision, answer, link, review]
  conditional: [park, resume, reuse-artifact]
inputs:
  continuation_token: opaque
  stage: upstream_stage_slug
  human_choice: exact_submitted_semantic_choice
  checkpoint: [summary-confirmation, plan-approval]
  session: required_when_upstream_binds_authority_to_session
observable_contract: C1
command_success: persisted_before_success_response
query_behavior: read_projected_data_without_domain_logic
```

外部の引数・フラグ、空値と省略の違い、終了コード、出力の種類と逐語はC1で固定する。配布手順にある`next / continue / report`、質問・回答・引継ぎ・レビューの入口を使う。追加の公開`report-id`フラグや、比較対象へ独自の結果フィールドを差し込まない。

CLIの構文解析は入力をユースケースへ渡す。状態依存の受理・拒否・適用対象の判断はコマンド側の集約に置く。クエリ側や出力側が現在の状態を見て同じ判断を実装し直すことはしない。必要な環境・カーソル観測だけを接続し、他ハーネス用の経路を今回の必須機能へ広げない。

実装計画承認は、質問、真正な人の回答、内容・対象・セッションに結び付いた許可の組を要求する。監査行の存在だけで実行を許可しない。CLIの失敗後にTypeScript側へ切り替えて同じ更新を行うフォールバックも設けない。

## C3: Claudeフックの契約

```yaml
contract: C3
format: hook-json
input:
  encoding: utf-8
  envelope_fields: [session_id, cwd, transcript_path, hook_event_name, tool_name, tool_input, tool_response, stop_hook_active]
  required_fields: per_upstream_hook_and_event
hooks:
  stop: aidlc-continue-workflow
  human_reply: aidlc-record-human-turn
  transition_guard: aidlc-state-transition-guard
  file_audit: aidlc-write-audit-log
outputs:
  stop: upstream_stop_decision_and_reason
  pre_tool: upstream_allow_or_deny_channel
  observational: upstream_exit_and_stream_contract
authority_updates: U2
observable_contract: C1
```

入力フィールドの一覧は共通封筒であり、すべてのイベントに全フィールドを必須化しない。必須項目、不正JSONや不足値の扱い、無関係なツールの無視は、本家のフック別契約と同じにする。

停止制御は正当な質問待ち・承認待ちを妨げず、再入と進捗のない繰返しを本家の条件で制御する。人間応答は実際のユーザー入力とセッションから記録する。保存監査は実際に対象資料を保存した事実に基づく。状態遷移の保護は正規の入口を経ない更新を拒否する。

配布設定にある実装計画承認保護、レビュー範囲・確定後の変更保護、開始・終了・状態同期等も、今回発火する経路について入力・副作用・出力を確認する。状態・監査・承認を更新する経路はRustへ統一し、主要4本以外だからという理由で未検証の更新処理を残さない。センサー実装の追加は本製品の今回の対象外である。

## C4: 報告イベントから結果を取得する契約

### 責任と順序

呼出側は報告の識別子を持ち、その値をコマンドへ渡す。集約がドメインロジックを起動し、報告された事実と適用結果をイベントにする。Repositoryは既存のイベントストア契約に従って追記・保存する。RMUがイベントを読み、リードデータベースに報告結果を投影する。結果が必要な呼出側は、その識別子を使ってクエリAPIから取得する。

```yaml
contract: C4
format: internal-command-event-query
identity:
  report_id: caller_known_report_identity
  aggregate_id: intent_execution_identity
command:
  inputs: [report_id, aggregate_id, reported_request]
  success_return: unit
  failure_return: typed_error
  responsibility: invoke_aggregate_and_persist_its_event
event:
  concept: Reported
  identity: distinct_domain_event_identity
  facts: [report_id, aggregate_id, effective_stage, result]
  result:
    changed: committed_transition_facts
    unchanged: no_op_reason_and_necessary_context
  persistence: existing_sqlite_event_store
projection:
  owner: RMU
  natural_key: report_id
  source: persisted_report_event_facts
  commit: result_rows_and_checkpoint_together
query:
  key: report_id
  returns: optional_report_result_view
  depends_on: read_model_only
  missing: explicit_not_found
```

`Reported`はこの契約での報告事実の名称であり、既存の`IntentExecutionEvent`に実装済みではない。報告による段階の変更と報告結果は同じ永続化単位で確定し、片方だけを成功としない。集約の1コマンド1イベントを維持する。イベントが集約全体のコピーを運ぶ形にはしない。既存の遷移イベントの事実をどう組み込むかは、既存の再生・投影テストと照合してU2の実装計画へ落とす。

成功する`no-op`も「報告を受理したが段階を変えなかった」という結果を取得できるように記録する。新しい段階の遷移を捏造しない。内部の報告事実の追記により、本家が外へ出さない監査イベントや不要な状態変更を公開することもない。状態・監査ファイルの出力は引き続きC1で検証する。

更新ユースケースの公開署名は`Result<(), E>`とする。`CommitOutcome`等の表示用成功結果を返す経路、別名、別の戻り値経路を並立させない。集約がイベントを返すことはコマンド側内部の正規経路であり、ユースケースの成功結果を呼出側へ返す例外とはしない。

結果モデルは当該報告の事実だけを持つ。呼出側が既知の入力を返すために余分な結果を作らず、適用された段、解決した対象、no-opの理由など表示に必要な実際の結果をイベントから投影する。クエリ用のViewはクエリ側が所有し、コマンド側のドメイン型・戻り値型へ依存しない。

### 報告結果の論理スキーマ

```yaml
schema: report-result-view
key:
  report_id: {type: string, format: uuid}
fields:
  aggregate_id: {type: string, format: uuid}
  effective_stage: {type: string, format: upstream-stage-slug}
variants:
  committed:
    applied_steps: {type: array, items: upstream-transition-name, ordered: true}
  no_op:
    reason: {enum: [already_awaiting, already_completed_moved_on, workflow_already_completed]}
    observed_current_stage: {type: string, required_when: already_completed_moved_on}
constraints:
  - one_result_per_report_id
  - missing_is_not_found_not_an_empty_success
  - query_types_have_no_command_domain_dependency
  - no_current_state_business_decision_at_read_time
```

このスキーマはクエリAPIが返す論理形であり、SQLの列配置やドメイン型の直列化形を指定するものではない。`report_id`は同一呼出し内で維持し、イベント自身のIDや集約IDと混同しない。ここでの結果種別は現コードのコミットと3種類のno-opに対応する。2.7.1の差分確認で追加の意味が必要と分かった場合は、無理に既存種別へ押し込まず契約差分として扱う。

### 失敗・再試行・取得の保証

| 場面 | 保証 |
| --- | --- |
| 構文不正・集約の拒否 | 成功結果を作らず型付きエラーを返し、本家の拒否出力へ変換する |
| 永続化の失敗 | 成功を表示しない。部分的な結果だけを公開しない |
| 同一呼出し内の楽観競合再試行 | 報告識別子と最初に解決した対象を保持する。別段階への意図しない前進を起こさない |
| 永続化後にRMUが失敗 | コマンドの成功結果を直接返す代替を使わない。RMUの再開で投影できるようイベントを正本にする |
| 投影後に結果が見つからない | 現在状態から結果を捏造せず、明示的な取得失敗として扱う |
| 別の呼出しが後続で報告 | 「最新の結果」で代用せず、指定したreport_idの結果を返す |

再試行は現在の有限な競合処理と本家の対象契約を確認して維持する。すべてのエラーへ無条件な再実行や無限待ちを追加しない。クエリのSQLや索引等は自由に最適化できるが、ドメイン依存、業務判断、状態遷移をクエリ側へ移さない。

## C5: 自己診断へ渡す根拠

```yaml
contract: C5
format: diagnostic-read-model
provider: U2
consumer: U3
facts:
  - selected_binary_and_entrypoints
  - claude_hook_bindings
  - required_scope_assets
  - workflow_state_version_and_identity
  - audit_and_projection_consistency
provenance: [observed_configuration, projection_position]
business_decisions: produced_upstream_not_recomputed_by_query
missing_or_stale: diagnostic_failure_or_explicit_unavailable
```

状態や報告に関する判断結果は、コマンド側の事実からRMUが構築した読取りモデルを使う。U3がドメインを再構成したり、状態ファイルを逆パースして業務判断を作り直したりしない。

ファイルや実行環境からの参照入力は、既存のRMUの参照入力処理と同様に観測元を識別して扱う。RMUによる通常の読取りモデル更新と、コマンド側の状態の修復・再初期化は区別する。クエリAPI自身は正本を更新しない。

## C6: 配布接続と補助処理の読取り

```yaml
contract: C6
format: local-cli-and-projected-files
writer: U2
integrator: U4
reader_candidates:
  review_brief: [summary, review, context]
  testing_posture: [render, fingerprint]
reuse_conditions:
  - reads_rust_projected_contract_correctly
  - does_not_mutate_state_audit_or_approval_authority
  - preserves_upstream_output_contract
excluded_from_read_only:
  - receipt_creation
  - code_generation_authority_begin
  - learnings_persist
  - audited_catalog_updates
```

再利用は操作単位で判定する。同じファイルに表示と更新の両方があっても、ファイル名だけで読取り専用と扱わない。依存先まで含む副作用と、前後の正本の不変を統合テストで確認する。

配布手順が指示する入口と実際のRustプロセスへの接続はU4が管理する。呼び出す作業ディレクトリ、選択した作業記録、セッション、入力文字列を維持する。入力をシェルの文字列展開で再解釈しない。接続の失敗を別の更新実装へ切り替えて隠さない。

## C7: 自己診断CLI

```yaml
contract: C7
format: cli
invocation: [target/release/aidlc, --doctor]
checks: [D1, D2, D3, D4, D5]
success_exit: 0
failed_check_exit: 1
evaluation:
  required_false: increment_failed
  evaluated_advisory: increment_passed_without_failing
  not_applicable: omit_row_and_do_not_count
output: selected_upstream_text_plus_explicit_native_checks
effects:
  cold_record: no_files_or_events_created
  initialized_record: HEALTH_CHECKED_via_U2_command_and_RMU
  automatic_repair: forbidden
  query: read_only
  derived_model_refresh: RMU_only
```

### 採用項目と本家への対応

本節の出典行はすべて、本家固定コミット`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`の`dist/claude/.claude/tools/aidlc-utility.ts`を指す。インストール済みforkの行番号と混同しない。下表のラベルは固定文言で、変数部分・修復案は指定した本家分岐の値を使う。

| ID | 本家の検査・参照 | 今回の適用条件と判定 | 出力ラベル・終了への影響 |
| --- | --- | --- | --- |
| D1.a | Bunの存在、2266–2278 | 表示等のTypeScript再利用があるため常時実施。PATHまたは本家が認めるBun配置を確認 | `bun installed (required for CLI tools and hooks)`。不在は必須失敗 |
| D1.b | 対応なし：Rustバイナリ固有 | 実行中バイナリの実体とU2が提供する必要操作の対応を確認。診断のために更新操作を実行しない | 独自の必須診断`Native engine entry points`。実体不明・必要入口不在・評価不能は失敗 |
| D2.a | Claude設定からのhook存在確認、2288–2349 | `.claude/settings.json`を読み、参照されたhookを一覧化して存在確認する。ディレクトリの自己列挙で欠落を見逃さない | 不読は`Hook contract: settings.json unreadable — cannot verify wired hooks`、参照なしは`Hook contract: settings.json wires no aidlc-*.ts hooks`、各存在確認は`<hook>.ts present`。いずれも本家どおり必須判定 |
| D2.b | hook全無効化、2379–2431 | 管理設定・プロジェクトlocal・project・userの本家の優先順で、明示されたbooleanを解決する | 有効は`Hooks enabled (resolved disableAllHooks is not true)`、無効時は本家の`Hooks DISABLED via ...`ラベル。無効は必須失敗 |
| D2.c | managed-only制限、2433–2447 | 読取り可能な管理設定で`allowManagedHooksOnly=true`なら該当 | `Claude managed hook policy: allowManagedHooksOnly=true`。必須失敗。管理上の設定を診断から変更しない |
| D2.d | 設定ファイルの存在、2713–2720 | Claude設定ファイルの存在を常時確認 | `settings.json present`。不在は必須失敗 |
| D2.e | 対応なし：Rust接続固有 | 設定のJSONを解析し、U4の接続定義と、必要イベント・matcher・呼出し先の対応を照合する。パース不能、未接続、更新先の混在を正常としない | 独自の必須診断`Native hook bindings`。不正・不一致・評価不能は失敗。存在だけで実発火成功を証明しない |
| D2.f | heartbeat、3073–3218。許容差は1832の300000ms | 作業進行前で未発火なら本家どおり初回状態として扱う。進行後の未発火、不読、最終進行より5分を超えて古いheartbeatは本家分岐どおり失敗 | 正常は`Hooks last fired: ...`または`Hook heartbeats: not yet fired (first workflow stage will populate)`。不正時の`Hook heartbeat data`等のラベル・fixは本家分岐を保持 |
| D3.a | workspace配置、2979–2998 | `.claude/`とdefault spaceのmemory配置を常時確認 | `workspace shell ready (.claude/ + aidlc/spaces/default/memory/)`。不足は必須失敗 |
| D3.b | スコープ検証、4259–4289 | 対象スコープをbugfix/featureの2つに限定する。各定義・必要入力の検査を本家のエラー/助言分類で行う | 正常は`Scope validation: 2 scopes valid (<n> advisories)`。エラー数が1以上なら失敗、advisoryだけなら失敗にしない。評価不能は`Scope validation: check failed` |
| D3.c | 循環・ステージ欠落、4177–4257 | 選定した2スコープで利用するグラフについて、循環と必要ステージの実ファイルを検査 | `Cycle detection: ...`、`Orphan stage files: ...`。循環・必要ファイル欠落・評価不能は本家同様に必須失敗 |
| D3.d | ステージschemaと参照、4290–4377 | 同じ対象集合のfrontmatter、役割、成果物・段階の参照を検査。初期化段階の役割例外も本家に従う | `Schema validation: ...`、`Graph references: ...`。不正・参照不能・評価不能は必須失敗 |
| D4.a | 状態版分類、3360–3409 | 状態ファイルがある場合だけ、本家と同じ分類器の契約で版を判定する。対応版は8 | 正常は`State Version: 8`。解析不能・過去版・未来版はそれぞれ`state version readable/current/compatible`の失敗行と、本家分類のmessageを表示 |
| D4.b | 対応なし：不読を明示する追加要件 | 状態が存在するのに読めない場合、本家3403–3408のcatchによる行省略を採らず明示的に検出する | 独自の必須診断`Native workflow state readable`。不読・評価不能は失敗。存在しない初回状態とは区別 |
| D4.c | 対応なし：選択中の実行識別 | 作業記録がある場合、選択されたintent・実行・状態・保存先が対応し、対象を一意に決められるか確認する | 独自の必須診断`Native workflow identity`。曖昧・取り違え・評価不能は失敗。本家の全space registry助言を必須へ昇格したものではない |
| D5.a | 対応なし：SQLite固有 | 作業記録がある場合、既存ストアを新規作成せず、読取り可能性と対応schemaを確認する | 独自の必須診断`Native event store readable`。不在・不読・schema不正・評価不能は失敗 |
| D5.b | 対応なし：RMU固有 | C5の投影位置と整合の材料から、必要な状態・監査・報告結果が保存済み事実に対応するか確認する。Queryにドメイン判断を再実装しない | 独自の必須診断`Native projection consistency`。必要投影の欠落・未反映・不整合・評価不能は失敗。本家の`runtime-graph-stale`警告とは別の検査 |

本家項目と対応する行の修復案は、その固定コミットから採取する。独自診断の失敗行は、上記の固定ラベルに` — <原因>`を付ける。原因には対象の相対パスと、`missing`・`unreadable`・`incompatible schema`・`binding mismatch`・`projection unavailable`等の判定を示す。独自項目は本家に同名の検査があるように表示しない。

管理設定は本家が読めるファイル面だけを対象とし、MDM・レジストリ・コマンドラインの一時設定まで検証済みと主張しない。本家のfix文言が保護無効化を案内する場合でも、その案内を実地スモークで採用する許可にはしない。実地確認は人間承認の保護を有効にしたまま行う。

### 初回状態・評価不能・警告の区別

- **初回状態**: 作業記録・状態・監査・イベントストアがまだないとき、D1–D3と初回heartbeat判定を実施する。D4–D5の記録依存項目は非適用として行を出さず、成功数にも失敗数にも加えない。空の状態やSQLiteを作って通さない。この成功だけでセルフホスト切替の実地完了とはしない。
- **既存記録**: 記録があるのに状態やストアが欠落している場合は初回扱いせず、D4.b/D4.c/D5.aの該当失敗にする。破損・権限不足・読取り例外も正常へ丸めない。
- **投影前後**: 診断用の参照入力と派生データはRMUだけが更新する。既存事実の通常の反映と、元データの修復・再初期化を区別する。反映に失敗した場合はD5.bを失敗にし、Query側で補完しない。
- **助言**: D3.bで本家の検証が返すadvisoryは、そのまま助言として表示する。必須エラーとは区別し、助言しかないことを理由に終了コード1へ変えない。

### 出力と終了コード

本家4692–4736・4794–4798の描画と終了判定に対応させる。stdoutはUTF-8で次の形とし、通常の検査成功・検査失敗ではstderrは空とする。区切りは`─`を37個、各行末はLFとする。

```text
AI-DLC Health Check
─────────────────────────────────────
✓  <成功または評価済み助言のlabel>
✗  <必須失敗のlabel> — <fixまたは原因>
─────────────────────────────────────
<passed> passed, <failed> failed
```

表示順はD1.aから表の順とし、hook別の行は本家どおり名前順にする。成功行のfixは本家同様に通常表示しない。実際に出した成功/助言行だけを`passed`、必須失敗行だけを`failed`へ数える。`failed=0`なら終了0、1以上なら終了1とする。

本家の`runDoctorAnalysis`が返す作業履歴診断は4692–4733で**終了値へ影響しない別枠**に描画される。今回、その汎用診断全体は採用しないため`Workflow diagnosis (advisory)`欄を生成しない。`runtime-graph-stale`等をD5の必須失敗へ読み替えない。doctor全文とのバイト一致は主張せず、採用した本家項目と明示した独自項目をこの契約で比較する。

### 実行記録の副作用

本家4669–4682・4737–4746は、起動時に監査が存在する場合だけ診断実施を記録する。**doctor全体を無書込みとした旧C7記述は訂正する。** Query APIは読取り専用のまま、外側の呼出し処理がU2の別コマンドへ診断事実を渡し、生成イベントをSQLiteへ追記し、RMUが本家の`HEALTH_CHECKED`監査行へ投影する。Queryユースケースからコマンドユースケースを呼ばず、コマンドから値を返す例外も設けない。

- 起動時に監査なし: 状態・監査・ストアを新規作成せず、診断出力だけを返す。
- 起動時に監査あり: 成功/失敗の診断結果に対し、`Request: /aidlc --doctor`、`Details: <passed> passed, <failed> failed`を持つ`HEALTH_CHECKED`を1件記録する。記録のために既存の状態や監査を修復しない。
- 診断実施の記録が失敗: 成功したふりをせず、C2の共通エラー経路で終了1とする。本家と同様、既に表示した診断stdoutは残り得る。検査結果が成功でも記録の失敗を終了0にしない。
- `GUARDRAIL_LOADED`は本家ではpaired-rule集計ができた場合の別記録であり、今回除外するセンサー対応率の検査に従属するため採用しない。これを暗黙に削るのではなく、doctorサブセットの監査範囲として明示する。

### 対象外と理由

| 本家の項目 | 今回の扱いと理由 |
| --- | --- |
| 他ハーネス、プラグイン選択/診断、センサー対応率 | 明示された対象外。成功行を置かない |
| 全spaceのIntent registry照合（4589–4640）、命名ずれの助言（3000–3028） | 広域管理を追加しない。選択中記録の利用可否は独自D4.cで検査し、助言の必須化と混同しない |
| 未コンパイルの不活性ステージ、重複producerやキーワードの広域検査 | 使わない定義の管理は対象外。必要な2スコープの参照・構造はD3で検査する |
| 孤児worktree/branch、swarm/claim、compose marker、merge dispatch | 今回のスモーク経路で必要としない管理機能のため除外。必要性が実測された場合は差分を明示する |
| 実践の古さ、規則見出しの重なり、汎用の作業履歴診断、workspace管理の助言 | 実施中でも発生する助言を自己診断の必須失敗にしない。今回の5条件の診断へ限定する |
| export/bundle等の診断資料出力 | 配布・診断一般化を増やさない。C7の公開入力は引数を追加しない`--doctor`のみ |

### 採取・比較ケース

| ケース | 条件 | 期待する結果 |
| --- | --- | --- |
| DC1 | 配布・入口は正常、記録は未作成 | 終了0。D4–D5記録依存行を省略し、ファイル・監査・SQLiteを作らない |
| DC2 | 初期化済みで全採用項目が正常 | 終了0、stderr空、失敗0。`HEALTH_CHECKED`1件 |
| DC3 | Bun不在／必要hookファイル欠落／hook全無効化 | 該当する本家行が失敗、終了1、stderr空。ほかの行も評価する |
| DC4 | 設定が読めない／参照なし／JSON不正／Rust接続不一致 | D2の該当行が失敗、終了1。どの失敗が本家対応か独自かを区別 |
| DC5 | 初回未発火／進行後未発火／heartbeat不読／300000ms超の遅延 | 初回だけは本家の初回成功行。残りは該当失敗行と終了1。境界値300000msは遅延失敗にしない |
| DC6 | 必要ステージ欠落／循環／不正schema／参照不正 | D3の本家対応行が失敗、終了1。advisoryだけの場合は終了0 |
| DC7 | 状態なしの初回／既存状態の不読／版欠損・過去版・未来版 | 初回は非適用。不読は独自D4.b、各版不正は本家D4.aの失敗と終了1 |
| DC8 | 既存記録の識別不整合／ストア欠落・不読／投影欠落・不整合 | 該当する独自D4.c/D5の必須失敗、終了1。元データを修復しない |
| DC9 | 本家で作業履歴警告だけが出る入力 | この汎用診断は対象外なので欄を出さず、これだけで終了1にしない。別の独自必須失敗があればその理由で終了1 |
| DC10 | 診断結果の監査記録に失敗 | 終了1。記録成功を捏造しない。診断出力と記録失敗を区別する |

DC1–DC10で採取する契約はここで固定する。実バイトの採取実行はU1、独自診断の実装・異常注入検証はU2/U3、実配線での確認はU4が担当する。

## C8: 接続・ホスト識別と切替準備

```yaml
contract: C8
format: local-runtime-binding
owner: U4
identity:
  host: verified_stable_tag_and_binary_identity
  target: development_commit_and_binary_identity
binding: explicit_selected_host
switch_preconditions: [live_bugfix_smoke_pass, doctor_pass, all_ci_jobs_pass]
rollback: restore_verified_host_binding
not_proof_of_completion: preparation_or_fixture_test_only
```

ホストと開発版を取り違えないよう、タグ・コミット・バイナリ実体を対応付ける。切替方法は本リポジトリ固有の接続として実行計画へ落とし、一般向けインストーラへ拡張しない。

実地スモークはこのリポジトリで実際のClaude・フック・人間承認を使う。模擬の人間応答や保護の無効化を成功証拠にしない。スモークに必要な解析資料だけを生成する。U4の準備完了と、検証済みホストへ実際に切り替えたことは別々に記録する。

## 所有・変更・検証の規則

- 各契約の所有者は上の表の単位とする。外部契約の所有者は本家であり、こちらの実装に合わせて変更しない。
- 本家の追加フィールド・不正入力の受理挙動は本家と照合する。独断で未知値を捨てたり、未知値を一律拒否したりしない。内部の新しい結果モデルの変更は、提供側・消費側を同じ検証対象にする。
- C4は今回の明示裁定による差分である。CQS例外の許可を再要求せず、現在の成功戻り値への依存と説明を是正する。
- 検証は、C1の再採取と比較、CLI/フックの正常・拒否ケース、報告識別子による結果の分離、永続化後のRMU再開、読取りの副作用、D1–D5の異常検出、実地スモークで構成する。必要なテストと実装はTDDで進める。
- 出力側は既知の入力と投影された結果から文言を作る。報告の適用・拒否を判定し直さない。クエリ側はドメインに依存せず、読取りの最適化だけを担う。

## 未解決事項と実装時の確認

| Contract | Question | Blocks |
| --- | --- | --- |
| C1–C3 | 新しい採取データの実バイトと、全対象ケースの差分確認はU1で実施する。未採取のケースを一致済みとはしない | U2以降の受入完了 |
| C4 | 新しい報告事実を既存の1イベント1コマンド・再生・投影へ組み込む具体的な型と移行はU2の実装計画で固定する。責任と外部契約は本書のとおり | U2の当該実装 |
| C5–C7 | C7で固定した採用項目・判定・出力・副作用とDC1–DC10を、具体的な取得/投影方法と実装へ対応させる。外部の判定・出力の選択は後続へ先送りしない | U3の実装・受入完了 |
| C6・C8 | 再利用候補の副作用検査、実際のホスト参照先・タグ・復帰先は実行計画と切替工程で特定する | U4の統合検証・切替 |

未回答の方針選択はない。上表は未実装・未実行の検証対象であり、未検証のまま契約を満たしたとする許可ではない。新たな仕様不一致が判明した場合は、具体的な根拠を提示する。

## Sources

- [requirements] [要求書](../requirements-analysis/requirements.md): FR1–FR8、NFR1–NFR4。
- [units] [単位定義](../units-generation/unit-of-work.md)、[依存関係](../units-generation/unit-of-work-dependency.md)。
- [Q1] [契約確認](contract-design-questions.md): CQS例外の否定、イベント追記・RMU・クエリの明示裁定と全体確認。
- [source] `modules/core/command/use-case/src/orchestration/commit_verdict_use_case.rs`、`commit_outcome.rs`、`modules/app/aidlc/src/runtime.rs:313`付近: 現在の成功結果の直接返却。
- [source] `modules/core/command/domain/src/orchestration/intent_execution.rs:1998`、`intent_execution_event.rs`、`report_no_op.rs`、`report_request.rs`: 既存の報告適用とイベント・入力。
- [source] `modules/core/read-model-updater/src/lib.rs`: イベント取得と投影、派生データ、チェックポイントの既存境界。
- [upstream] 本家固定コミットの`dist/claude/.claude/skills/aidlc/SKILL.md`、共通プロトコルとbugfixのステージ定義。呼出し一覧は[要求確認](../requirements-analysis/requirements-analysis-questions.md)を参照。
- [rules] `coding-rules/aggregate-commands.md`、`command-query-separation.md`、`cqrs-boundaries.md`、`gateway-taxonomy.md`、`README.md`。クエリの最適化に関する今回の人間の指示は、その射程で優先する。

本書の作成時点で、本体のCQS是正、2.7.1の再採取、実地スモーク、切替はまだ実行していない。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T14:42:48Z
**Iteration:** 1
**Request Challenge:** review:8ecdd34ee0a96adfb7c860625e1bf5ee

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/260907-selfhost-stage1/inception/contract-design/contract-summary.md > C7: 自己診断CLI（D1–D5）、未解決事項と実装時の確認（C5–C7） | 要求書FR6と単位定義U3は、本家2.7.1の各診断項目との対応・対象外理由・正常/異常の出力契約を本工程で固定すると定めている。現状はD1–D5の分類だけで、本家の検査名・参照箇所、適用条件、終了コードと出力の対応がなく、期待出力を後続へ先送りしている。本家固定コミットのaidlc-utility.ts:4692–4733は環境検査の失敗と終了値へ影響しないworkflow diagnosisを区別するため、「必要検査の失敗」と「本家の終了状態」の対応を実装者が独自に決める余地が残る。 | D1–D5それぞれを本家固定コミットの具体的な検査へ対応付け、対応する本家検査がないものは独自の必須診断と明記する。必要な対象外理由、状態未作成・評価不能時の適用条件、正常/異常の終了コードと標準出力/標準エラーの形を固定する。採取実行はU1へ残してよいが、採取・比較する契約を先に識別可能にする。 | Resolved |

### 解消確認

R-01の表は初回指摘の内容とIDを保持し、状態を更新した。改訂後のC7はD1.a–D5.bの対応表に本家固定コミットの参照箇所とラベルを記し、Rust入口・接続・識別・SQLite・RMUの独自診断を区別している。初回と既存記録の欠落、必須失敗と助言、stdout/stderr、終了コード、表示順・集計、対象外理由、DC1–DC10が確定したため、指摘した契約選択の先送りは解消した。

本家固定コミットのhook存在・無効化・managed-only判定（2266–2447）、heartbeat（3073–3218）、グラフ・スコープ・schema検査（4177–4377）、監査存在条件（4669–4682）と終了判定（4794–4798）をGitから直接確認した。前回確認した状態版分類とadvisory診断の描画も含め、採用する分岐と独自条件の区別に新たな重大な矛盾はない。汎用workflow diagnosisを必須失敗へ読み替えていない。

診断の実行記録は、読取り専用Queryの外からU2の別コマンドへ渡し、イベント追記とRMU投影を経てHEALTH_CHECKEDを記録する。起動時の監査不在では生成せず、既存監査がある場合の記録失敗も成功へ丸めない。元データの修復と診断事実の追記は区別されており、C4の成功戻り値なし・報告IDによる取得と併せて、CQS例外は導入されていない。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| bun .codex/tools/aidlc-sensor-required-sections.ts --output-path aidlc/spaces/default/intents/260907-selfhost-stage1/inception/contract-design/contract-summary.md | PASS: h2_count=13、findings_count=0（Review追記前） | 文書構造を確認した。 |
| bun .codex/tools/aidlc-sensor-upstream-coverage.ts --stage contract-design --output-path aidlc/spaces/default/intents/260907-selfhost-stage1/inception/contract-design/contract-summary.md --consumes unit-of-work,unit-of-work-dependency,requirements --deliverables contract-summary | PASS: consumes 3件、unreferenced=[]、findings_count=0 | 指定された上流3成果物への参照を確認した。 |
| Bun.YAML.parseによる全yamlフェンス検査 | PASS: 9ブロック | YAML構文を確認した。API完全性のスキーマ検査ではない。 |
| git diff --check（contract-summary.md） | PASS | 空白差分の問題なし。 |
| git show a277af218f0df7f325d3b8be7b6d90fce2c5bd40:dist/claude/.claude/tools/aidlc-utility.ts | 対象分岐の参照照合を実施 | 本家の実行結果を新たに採取したという意味ではない。 |

### Summary

R-01は解消済みで、新たな指摘はない。契約は実装計画へ引き継げるが、本体実装・ゴールデン再採取・実地スモークは引き続き未実施であり、本レビューはそれらの成功を示さない。
