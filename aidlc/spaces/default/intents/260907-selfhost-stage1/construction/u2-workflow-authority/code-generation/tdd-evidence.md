# U2のTDD実行証跡

## 実行基準

承認済みTesting Contractは `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55`。最初に `cargo test -p core-command-use-case --lib orchestration::commit_verdict_use_case::tests::` を実行し、既存30件成功でランナーを確認した。U1のRust基準および統合条件は [integration-baseline.md](integration-baseline.md) を参照する。

## 失敗を先に確認した振る舞い

| 順 | テストコマンドの対象 | Redの観測 | Green |
| --- | --- | --- | --- |
| 1 | `cargo test -p core-command-use-case --lib a_successful_no_op_persists_one_report_fact` | exit101。保存件数が0で期待1に不一致 | 報告事実を1件保存して成功 |
| 2 | 同 `a_transition_persists_its_result_as_one_report_fact` | exit101。遷移が結果付きReportedではない | 遷移と結果を同じReportedに保持して成功 |
| 3 | `cargo test -p core-read-model-updater --test report_result_projection_contract` | exit101。`no such table: read_report_result`。報告を実ストアへ保存して投影した後の結果取得で失敗 | 結果表を同じ投影トランザクションに追加して成功 |
| 4 | `cargo test -p core-query-interface-adapter --test report_result_dao_contract` | 新規DAOの未接続状態では結果がNoneでexit101 | report_idによる引当を接続して成功。別報告と不在も検査 |
| 5 | `cargo test -p core-command-interface-adapter --test report_result_contract` | exit101。GateOpenedの事実なのにSkipを名乗る保存結果の復号が成功してしまう | Reportedの構築時に操作列と遷移の整合を検査し、両側DTOがその構築口を通って成功 |
| 6 | `cargo test -p core-command-domain --lib a_no_op_report_refuses_a_foreign_intent_without_recording_it` | exit101。no-opで他の依頼の結果を記録してしまう | 集約が依頼の取り違えを先に拒否し、状態不変で成功 |
| 7 | `cargo test -p aidlc --test upstream_271_contract` | exit101。Cargo/srcのある実ワークスペースがGreenfield/Unknownとして開始される | 実走査でBrownfield/Rust/cargoとbugfix9段の開始を確認して成功 |
| 8 | 同 `source_languages_are_counted_and_distribution_directories_are_excluded` | exit101。main.tsを置いてもGreenfield/Unknown | 根と既知ソースディレクトリの言語集計を接続しTypeScriptを確認して成功 |

上記のエラーはテスト関数のアサート失敗として確認した。変更途中の署名移行、未import、テスト用tokioの機能不足などのコンパイル失敗はRed成功に数えていない。新しいDAOの未接続状態を使った項目4は既存機能の不具合再現ではなく、新設した境界の接続確認である。

## CQS経路と回帰

更新ユースケースは呼出側のReportIdを受け取り、成功は `Result<(), CommitError>` のみ。旧CommitOutcomeと旧署名は削除した。集約のapply_reportが単一Reportedを返し、RepositoryのDTOで保存、RMU専用DTOで復号して結果表を投影する。QueryのDTO/DAOと取得ユースケースはドメインに依存しない。CLIは更新成功→RMU→ReportId指定Query→表示の順で呼び、結果不在や投影失敗を成功へ丸めない。

- 更新ユースケース32件成功。既存の拒否、1回だけの競合再試行と2回目の伝播を含む。テスト補助は受理されたイベントから結果を観測し、公開コマンドの戻り値へ戻していない。
- 結果投影5件成功。呼出側ID、後続報告からの独立、3種類のno-op、チェックポイント失敗時のロールバックと再開、反復投影を確認。
- `intent_lifecycle` 118件成功を確認。追加のno-op検証は1.1秒待ってから再報告し、状態全文（Last Updated含む）と監査シャードの同一バイト、内部ジャーナル1件増加を確認。
- 実Repository結線テスト成功。書く側と読む側のDTO横断適合テストにもReportedを追加。
- 構造化リードモデルは結果表の追加で18表（ジャーナル由来16＋参照入力2）、schemaは2。旧schemaからの再生成経路と、全表の削除・内容ダイジェストの一覧へ追加した。固定件数の回帰期待は新しい実表を含むよう更新した。

## 継続中の項目

この記録はU2完了報告ではない。実走査の境界条件、2.7.1のCLI全文・状態/監査比較、受領・主要フック・補助更新・旧参照移行・全品質ゲートは継続中。全workspace再検証と最終source-manifest/traceability/code-summaryは完了時に確定する。実地Claudeスモークとnative doctorは後続工程であり、上記テストでは達成扱いしない。

## 開始契約の追加確認と命名のやり直し

本家stage1/intent-create/bugfixの初期ファイルとargvを保存コーパスから読み、実バイナリを起動した。state全文比較のRedで番号列、SKIP行、Project Description Source、既定のDepth/Test Strategy、Project Root、Next Action、余分なAutonomy行を検出した。初期stateとproject-description.jsonをRMUへ移し、Createdからの生成でGreenを確認した。日時はStart DateとLast Updatedの実測値が同じ2箇所であることを検査し、その値だけを両辺で同じ規則に正規化した。

次にstdoutを同じ採取結果と比較したところ、nativeの旧JSON応答と本家の開始サマリーが不一致となりexit101。read_intentへ開始時の件数・最初の段階/フェーズを投影し、QueryのInitializationView/DAOから取得して描画する実装でGreenになった。appによるIntentRepository再読込とstate骨格の直接保存は廃止した。

**命名の最初の試行には順序不備があった。** id8禁止アサートを挿入する文字列置換が空白差により適用されず、確認前に命名の実装を変更していた。この試行をTDD達成として扱わない。親からの是正指示を受け、命名の本体と予約処理だけを変更前に戻し、他のCQS・初期状態の変更と正しい公開CLIテストを保持してやり直した。

1. 変更前の命名を復元後、`cargo test -p aidlc --test upstream_271_contract bugfix_initial_state_matches_the_captured_271_bytes`を実行。exit101、`初回の記録名にid8は付かない`のアサートで `stage1-01a07eb7 != stage1` を確認。
2. 本家のYYMMDD-label（slugify上限24）と、作成時の原子的な予約・番号付き衝突回避を実装し直した。
3. 同じコマンドがexit0となり、state全文・stdout・初回記録名の3面が成功。`/tmp/amadeus-u2-record-name-redo-red.log`と`/tmp/amadeus-u2-record-name-redo-green.log`に実行出力を保存した。

新しい命名は新規作成にだけ適用し、既存記録は移動していない。この時点では複数intent投影がMixedIntentsで拒否され、人間裁定待ちだった。後続の裁定と検証は以下に追記する。命名の衝突回避だけで複数intent実行が完了したとは扱わない。

Autonomy初期行の削除で、既存set-autonomy検査の前提が変わった。本家supplemental採取と同じく、自律切替用の合成テストでは当該欄を明示した状態から検証する。製品の初期stateにその行を戻していない。変更後のintent_lifecycle全118件が成功した。

## 複数intentと初回の解析指示

人間裁定 `multi-intent-questions.md` の回答1により、2件目以降の実行もU2に含めた。実バイナリで2件目を作成するテストが `projection: mixed intents` で失敗するRedを確認し、RMUの対象実行とチェックポイントを実行ID別に指定した。公開状態/監査は対象の履歴だけから、共有read表は全履歴から投影する。対象の指定がないMixedIntents拒否は保持し、既存RMU契約31件が成功した。2件目作成と1件目へのカーソル切替後も、1件目の状態/監査の実バイトが維持された。この時点では異なるscope、報告結果、失敗後の追いつきは未検証だった。後段「追加の保全とCQS是正」で実施して成功した。既存記録の移動は行っていない。

初回Reverse Engineeringの指示を、固定元の実行から `tests/golden/selfhost-stage1/bugfix-first-next.json` に追加採取した。U1の採取物は変更していない。両実行の根を一時親の下の `workspace` に揃え、同じTypeScriptファイルと配布規則を使った。Rust検査は保存JSONとnativeバイナリだけを使う。

- 配置/拡張子/規則台帳の差を公開フィールド比較で確認し、CodeKBの実際の配置、規則の空テンプレート判定、相対パスを合わせてGreen。CodeKBパスや任意IDの広い正規化は使わない。
- pipeline欠落をRedで確認し、配布定義のリンク順と初回の空の完了列を出力してGreen。後続link受領の反映はStep5で継続する。
- conductor_persona/narration欠落をRedで確認し、最初の実作業である事実を集約からRMUへ投影して描画しGreen。
- 値比較からJSONキー順を含むstdout実バイト比較へ強めると、protocol_modules/next_stageの順序でRed（exit101）。出力順を本家へ合わせ、同じ全文比較がGreen。ログは `/tmp/amadeus-u2-first-next-byte-red.log` と `/tmp/amadeus-u2-first-next-byte-green.log`。正規化は記録名の日付6桁だけで、ラベルが一致することを先に検査する。
- 全件実行時、固定basenameへfixtureを変更した際の読み取り先1箇所に旧パスが残っていた。これは製品の振る舞いのRedには数えず、fixtureを是正して再実行した。

## 追加の保全とCQS是正

異なるscopeの2件（bugfixはreverse-engineering、featureはintent-capture）で、2件目の報告保存後にSQLiteのpublication挿入をトリガで失敗させた。1件目へ切り替えてQueryを実行し、次に2件目を別プロセスで再開して追いつく検証が成功した。状態/監査は対象別に保持され、1件目のreport_idで取得する結果も不変。繰返しnextで監査を増やしていない。最初の追加アサートは状態の表示を誤って `Awaiting Approval`、続いてfeatureの初期段階をreverse-engineeringと書いていたため、これは製品のRedに数えず、実際のfeature配布計画（intent-capture）のcheckboxで是正した。

旧共有チェックポイント `orchestration` を持つ実ストアでは、新しい名前で再生を進めてしまうRed（run-stage、期待error）を確認した。RMUのrequire_unpublishedで旧公開位置を検査して明示拒否し、状態/監査/チェックポイントを変えないGreenを確認。旧ストアや公開記録の移動はしない。ログは `/tmp/amadeus-u2-legacy-checkpoint-{red,green}.log`。

TS由来の既存登録/recordを置きnative開始する保全検証は最初から成功した。その後、nativeの登録追加を要求するアサートがRed（行数1、期待2）。予約済み公開名をStartRequest/Createdの事実に保持し、RMUが登録JSONをpublicationへ含める最小差分でGreenになった。内部UUIDと公開dirName/slugは別々に照合する。既存TS行の全フィールドとstate/監査は保持し、appは登録JSONへ書かない。後続の初回解析指示の実バイト比較も維持した。ログは `/tmp/amadeus-u2-registry-{red,green}.log`。

既存ReviewLogOutcomeもコマンドの表示用戻り値だったため是正した。更新成功の実際の型名を検査するテストがReviewLogOutcomeでRed、executeをResult<(), ReviewLogError>へ変更し、依頼/判定/再試行の16件がGreen。表示は呼出側が保持しているReviewLogKindを使い、別名の成功戻り値を設けていない。再試行のアサートは戻り値から保存されたReviewRequestedのretry事実へ移した。ログは `/tmp/amadeus-u2-review-cqs-{red,green}.log`。

直近の開始/進行回帰は `cargo test -p aidlc --test upstream_271_contract --test intent_lifecycle` で15件＋118件が成功した。workspace全体は旧CLI goldenの「narration欠落を期待する」検査で停止しており、これはStep8で実際の2.7.1比較へ移行する。欠落期待を1件へ減らす緩和はしていない。cargo lintは成功。clippyは新しい型検査とJSON直列化の規則違反を検出し、unitの検査を汎用関数へ、登録の出力をcanon_jsonへ是正した。

## 通常の質問提示

`an_ordinary_decision_is_persisted_and_projected_without_changing_workflow_state` が、未配線のaidlc-log decision（exit1）でRedとなった。本家2.7.1 `stage1/cases.json` のquestion/decisionと同じargvを使い、集約のDecisionRecorded→Repositoryの書込みDTO→SQLite→RMUの読込みDTO→DECISION_RECORDED監査を接続してGreenを確認した。stdout実バイト、stderr空、状態全文不変、監査1件、保存イベント1件を検査している。入力と表示に使うステージは呼出側が既に保持し、RecordDecisionUseCaseの成功はunitだけ。

この時点で接続したのは通常質問のみで、checkpoint/unit/singleの質問、回答・人間応答の結合はまだ成功扱いしない。ログは `/tmp/amadeus-u2-decision-{red,green}.log`。追加イベントの網羅matchは落ちた箇所を明示的に追加し、ワイルドカードで塞いでいない。

## 人間応答フックの最初の接続

`a_human_prompt_hook_records_presence_and_preserves_a_numeric_response` が未配線のhook動詞（exit1）でRedになった。harness-claudeでClaude封筒のsession/responseを抽出し、集約のPromptObserved→SQLite→RMUでHUMAN_TURN監査と会話マーカーを投影してGreenを確認した。番号「1」をJSONの数値として捨てず、その元文字列を保持する。状態全文は不変、stdout/stderrは空。ログは `/tmp/amadeus-u2-human-hook-{red,green}.log`。

このフックは本家ソースの明示契約に従い、記録失敗時にも人間の入力を止めない。記録失敗を承認受領の発行と扱う経路はない。無人入力・不正封筒・対象なし等の追加境界と、回答側の受理/拒否との結合は継続中。

## 回答の受理・再利用拒否・復旧

`an_answer_consumes_one_recorded_human_response_across_processes` がanswer未配線exit1でRedとなった。質問・人間応答・消費済み応答を集約のInteractionStateに保持し、スナップショットへ保存する。AnswerIdは呼出側が発行し、RecordAnswerUseCaseはunitを返す。集約のAnswerRecordedをRMUがread_answer_resultへ投影し、AnswerResultUseCaseがIDで行を引く接続でGreen。回答を別プロセスから再利用すると本家の逐語で拒否し、拒否では結果行を増やしていない。

追加の境界検証として、回答の保存後にpublicationを失敗させ、再起動後も同じ応答が消費済みであること、未投影のQueryがNoneを返すこと、nextで追いついた後に元のAnswerIdで結果を取得できることを確認した。後続の別回答によって元の結果は変わらない。監査へHUMAN_TURNを手書きしても、保存されたPromptObservedがないanswerは拒否する。いずれも既存の保存・復旧機構を通す追加確認として成功し、人工的なRedを作っていない。

承認待ちのanswerがQUESTION_ANSWEREDを出して人間応答を消費するRedを確認した。ゲート開放より前の質問を未回答扱いで持ち越さず、承認はreportが担う場合にAnswerRecordedの結果をapproval-gate-report-ownedとして保存し、公開監査を増やさない変更でGreen。本家のskipped JSON、state不変、応答未消費を検査した。ログは `/tmp/amadeus-u2-answer-{red,green}.log`、`/tmp/amadeus-u2-answer-recovery.log`、`/tmp/amadeus-u2-answer-gate-{red,green}.log`。

親の読取確認により、封筒のJSON数値1まで文字列候補へ昇格していた過剰適用を検出した。数値型1は空、文字列"1"は保持という2テストを先に実行し、前者のRedを確認後に是正して双方Green。番号の是正対象は文字列に限り、応答候補の型を広げない。ログは `/tmp/amadeus-u2-numeric-envelope-{red,green}.log`。

read_answer_resultの追加により、現在の構造化表はジャーナル由来17＋参照由来2の計19表である。DDL、削除、内容ダイジェスト、投影の列挙へ追加している。表数/旧goldenなどの全回帰は仕上げの検証対象として継続する。

## 内容確認と可視性の追加採取

summary/decisionとsummary/answerの固定2.7.1入力を使うCLI結合が、未接続checkpointでRedとなった。質問内容の意味上のハッシュ、提示前の人間応答の識別、通常質問と内容確認の未回答状態を集約へ追加し、AnswerRecordedの内容確認結果をRMUから投影してGreen。確認対象のQuestions SHA-256は `486d1287b34b0854af34ee6dc12a067f86d56cfc4b75aa2598c01eb5b803338a` で採取値と一致した。提示前の発言を拒否し、提示後の新しい発言なら受理、同じ確認の再利用を拒否する追加CLI検証も成功。

文書可視性の最初の単純パーサは、フェンス/コメント中の偽確認節を受理し得たため完了扱いにしていない。`capture-summary-visibility.ts` が固定配布物のマニフェストを検証して本家関数を呼び、入力原文・base64・回答値・ハッシュ/エラーを `tests/golden/selfhost-stage1/summary-visibility.json` に保存する。原文の正規化は行わず、Rust検査は保存JSONだけを使う。入力境界の根拠は本家 `tests/integration/t185-stage-artifact-guard.test.ts`（同じ完全SHA）の850行以降、938/975、1230行以降、1399行以降を含む。

フェンス、HTMLコメント、raw HTML、HTML属性内回答、Setext/HTML見出し、コンテナの終端、インラインコード、閉じる#、BOM、未完了タグの境界について、誤受理/誤ったハッシュのRedを個別に確認して是正した。現時点で49観測の回答受理/拒否、構造エラー逐語、内容ハッシュが一致。テストは空コーパスとID重複も拒否する。解析本体を確認内容・見出し・可視性・コンテナに分け、整理後も同じ比較が成功した。

ログは `/tmp/amadeus-u2-summary-{red,green}.log`、`summary-freshness.log`、`summary-fence-{red,green}.log`、`summary-comment-{red,green}.log`、`summary-html-{red,green}.log`、`summary-attribute-{red,green}.log`、`summary-setext-{red,green}.log`、`summary-html-heading-{red,green}.log`、`summary-container-{red,green}.log`、`summary-nested-heading-{red,green}.log`、`summary-inline-{red,green}.log`、`summary-heading-suffix-{red,green}.log`、`summary-bom-{red,green}.log`、`summary-tag-boundary-{red,green}.log`。いずれも接頭辞は `/tmp/amadeus-u2-`。追加行列表記の回帰は `summary-heading-matrix.log`、分割整理は `summary-refactor.log`。

SHA-256のバイト計算は、既存canon_json内部の計算を汎用のcore-infrastructure/hashへ切り出して再利用した。新しい依存は加えていない。既存infrastructureの125テストは移動前後とも成功（`hash-extraction-{before,after}.log`）。

## 継続トークンの独立した2つの束縛

本家のnext→人間応答→continueを実行して追加採取したところ、公開state本文が不変なら継続できた。nativeは全イベントのseq_nrを状態束縛にしていたため拒否するRedとなった。集約に本流の進行状態を最後に変えた通番を保持し、実行IDと合わせてRMU/Queryへ運ぶ。これとは別に、入力/出力境界が公開stateの実測SHAを封緘トークン内に載せ、continueで実ファイルと照合する。ドメインへファイル形式/I/Oを追加せず、Queryは計算や判断をしない。外側のdirective JSONに新しい結果フィールドは加えていない。

同じ介入を固定本家とnativeで実行し、応答だけ/no-opは継続、本文だけの変更/正規のpark遷移/別intentへの切替は拒否、という5条件が一致した。さらに、本文と封緘鍵を両方コピーした別intentにnativeが進んでしまうRedを検出し、現在のexecution_idと引いた行の対応を検査して、本家と同じstaleの逐語で拒否するGreenを確認した。トークンは置換せず、その実行が発行した値をそのまま引き渡す。

採取はU2所有の `capture-selfhost-first-next.ts` と `tests/golden/selfhost-stage1/steering-*.json`。ログは `/tmp/amadeus-u2-steering-human-{red,green}.log`、`steering-state-{red,green}.log`、`steering-contract.log`（5件）、`steering-identity-{red,green}.log`。この変更後の全回帰は継続中で、Plan Approvalと残りのステップを完了したとは扱わない。

## 指示の発行記録とStop確認

指示を表示する前にDirectiveIssuedを集約で生成し、SQLite保存からRMUによるactive-directive公開へ接続した。初回nextとcontinue後の発行記録は、現在の入力から決まるプロジェクト実パス・依頼ID・state本文SHA・その派生所有者以外を置換せず、固定本家の記録と一致した。

Stop自身の確認入力 `AIDLC_STOP_HOOK_PROBE=1` について、本家orchestrate.tsのisStopHookProbe/emit（544–579行）に従う境界テストを追加した。確認でもrevisionが1から2へ増えるRedを実測後、発行の入力境界で同環境値を識別してGreen。通常nextとstdoutは同じで、active-directive全文・state全文・SQLiteイベント件数は不変。ログは `/tmp/amadeus-u2-stop-probe-{red,green}.log`。Plan Approvalの受領記録そのものの保持は、今後の承認結合検証にも含める。

## ソース指紋の入力境界

`capture-source-fingerprint.ts`で固定本家のworkspaceSourceFingerprintを呼び、通常ファイルと配布資産/生成物除外の観測を保存した。新しい入力境界が未接続エラーを返す最初のRedを確認後、ファイルの内容・実行権限と論理パスを安定読取りし、本家と同じハッシュを返すGreenを確認した。これは新しい接続のテストであり、従来の既存APIの不具合再現とは区別する。

空ワークスペースを追加採取すると、ファイル列が空のときの末尾改行差でRedとなった。本家の配列joinに合わせ、2観測ともGreen。ログは `/tmp/amadeus-u2-source-fingerprint-{red,green}.log`、`/tmp/amadeus-u2-source-empty-{red,green}.log`。ソース登録台帳やsymlinkなどの未対応入力と、Plan Approvalへの接続は継続中であり、この比較だけで完了とは扱わない。

コード生成の発行記録は、開始→REゲート承認→要求分析のレビュー依頼/READY→ゲート承認から進めたCLI結合で確認した。最初の試験準備では`next --stage`を段階移動と取り違え、次には要求分析のレビュー受領不足で目的地に達しなかった。これらを製品のRedには数えていない。正しい準備でauthority revision欠落を確認したRedは `codegen-marker-red-4.log`、Greenは`codegen-marker-green.log`。ソース基準と発行番号をDirectiveIssuedからRMUへ運び、未承認の再発行でソース基準まで変わる追加Redを、集約による基準保持でGreenにした（`codegen-source-retention-{red,green}.log`）。全て接頭辞は`/tmp/amadeus-u2-`。`cargo clippy -p aidlc --all-targets -- -D warnings`もこの変更で成功した（`directive-clippy.log`）。承認済み受領後の基準更新・challenge・beginは引続き未完了。

## 計画承認の文書とテスト契約

PlanQuestionsの新しい解析口について、未接続状態のRedから通常の承認済み/未回答をGreenにした。その後、フェンス/コメント中の偽承認、番号・強調・分割見出し、選択肢表記と最終回答/最終確認節、空の指紋上書きを追加採取して是正した。固定本家20観測のapproved/pending/fingerprintが一致し、既存SummaryQuestionsの49観測も成功した。文書内の承認欄だけでは、人間応答の受領や生成許可を発行しない。ログは`plan-questions-{red,green}.log`、`plan-visibility-{red,green}.log`、`plan-heading-{red,green}.log`、`plan-answer-{red,green}.log`、`plan-parser-regression.log`。

本家codeGenerationApprovalArtifactsは、埋込Testing Contractの自己ハッシュと、その時点の規則から再解決した契約ハッシュの一致を要求する。既存Rustに解決処理がなかったため、Step5の依存としてTestingPostureを追加した。空規則の既定、新規/既存開発、明示TDD、チームとプロジェクトの矛盾拒否、スコープと戦略の義務、構造化フィールド、コメント/コード例、各方法論と非構造化の宣言、規則文書からの節抽出を、Red→Greenで順に接続した。本家orgの配布原文と本リポジトリのteam原文も入力として記録し、契約全値・input_sha256・contract_sha256を比較している。

語句規則はworkspaceの既存regex 1.13.1をdomainの純粋照合依存にも追加した。版の追加はないがCargo.lockのdomain依存欄は変わったため、依存監査は最終検証で再実行する。JavaScriptの非Unicode正規表現との差も検証した。greedyな検索結果を後から除外する否定先読み代替は`first-class`より前の`first`を見落とすRedとなり、照合式内へ条件を組み込んでGreen。Unicode case foldingで長いsを`test`へ昇格するRedを、componentsは小文字化済み入力のフラグなし照合、orderingはASCIIのみ小文字化する照合で是正した。UTF-16量指定は🚀39/40個で80単位の両側を、19/20個で40単位の両側を比較した。原文・注記・ハッシュは保持し、量指定の一時照合表現だけを1 UTF-16単位=1照合文字としている。現在36観測が一致し、domainの全target clippyも成功した。

ログは`testing-posture-{red,green}.log`、`testing-brownfield-{red,green}.log`、`testing-tdd-{red,green}.log`、`testing-conflict-{red,green}.log`、`testing-obligations-{red,green}.log`、`testing-actual-rules.log`、`testing-field-{red,green}.log`、`testing-visibility-{red,green}.log`、`testing-methodologies-{red,green}.log`、`testing-prose-{red,green}.log`、`testing-mixed-{red,green}.log`、`testing-mixed-regression.log`、`testing-section-{red,green}.log`、`testing-casefold-{red,green}.log`、`testing-utf16-{red,green}.log`、`testing-utf16-40-boundary.log`、`testing-posture-clippy.log`。この節の全ログ接頭辞は`/tmp/amadeus-u2-`。

`aidlc-testing-posture render`の未配線Redを確認し、RMUが規則原文と依頼条件をドメインへ渡してread_testing_contractへ投影し、Queryがintent_idで1行読む接続でGreenにした。表示は固定本家renderTestingContractの全文と一致する。純粋な表示のために新たな承認イベントは作らない。規則変更時にQuery単独では値が変わらず、RMU更新後に新しい契約が見えること、挿入失敗時に旧行が保持されること、再実行で回復することも追加検証として成功した。状態・監査・イベント件数は不変である。ログは`testing-render-{red,green}.log`と`testing-render-recovery.log`。fingerprint/承認challenge/受領/beginおよび最終の全体検証は引続き未完了。

## 実行境界と承認要求の識別

コード生成のrun floorを固定本家6観測で比較し、未接続のRedから種類・時刻・同種境界の通し番号が一致するGreenにした。集約の開始→コード生成への進行→差戻し→別段階への移動→再入でも期待する境界になる。監査ファイルを権限の正本として読み戻さず、イベント適用で持つ状態である。スナップショットでこの状態が失われるRedを検出し、専用DTOから検査付き再構成へ接続してGreenにした。ログは`run-floor-{red,green}.log`、`run-floor-events-{red,green}.log`、`run-floor-snapshot-{red,green}.log`。

U1の保存されたplan/decisionのmarker・challengeと、同じ固定元のmemory実バイトから、発行エポック、計画指紋、提示内容のpromptSha256、challengeIdを比較した。各新しい接続のRed後に順にGreenを確認している。Evidenceの最初の試験準備ではU1のinitial_filesに配布memoryが含まれないため失敗した。この準備失敗はRedに数えず、同一source/manifestの保存済みreference_filesを使って正しいRedを取り直した（`plan-evidence-red-2.log`）。ログは`plan-authority-{red,green}.log`、`plan-fingerprint-{red,green}.log`、`plan-exact-answer-{red,green}.log`、`plan-evidence-green.log`、`plan-challenge-{red,green}.log`。

埋込Testing Contractは本家10観測（改変・version・欠落・fence・コメント・1.0等）で受理/拒否と全値が一致した。自己ハッシュが正しくても現行のTesting Postureが変われば拒否する追加Red→Greenも確認した。ログは`embedded-contract-{red,green}.log`、`embedded-current-{red,green}.log`。

`fingerprint --stage-level`も未配線Redから、ドメインの計算→対象別read_plan_fingerprintへのRMU投影→execution_id＋target_idのQuery→表示を通してGreenになった。保存済み行とCLIの値が一致する。本家の同一intent/marker/stateを別パスへ複写する実測では、指紋は変わらず成功した。当初の「実パス変更で拒否するはず」という仮定は採用せず、結果を`plan-project-binding.json`へ保存した。Claude経路のreadActiveDirectiveMarkerは状態本文SHAを照合する。本家が行わない実パス拒否は追加していない。nativeの移設前後も同じ結果を確認した。ログは`plan-fingerprint-cli-{red,green}.log`、`plan-fingerprint-relocation.log`。

保護された選択肢の照合は本家9観測で、通常の番号・正規ラベル・完全一致時の番号拒否・BOM/NEL・無関係な応答を比較し、Red→Greenで一致した。これは純粋な照合であり、challenge/receipt/beginの耐久保存は未接続である。共用Runtimeの配置は親の`approval-runtime-questions.md`による裁定を待っており、共有ストアはまだ作成していない。

## ECMAScript空白と不正stdin

HumanTurnEnvelopeのsession/responseでBOMが残る、NELが消える2つのRedを実測し、言語拡張のECMAScript空白集合を使ってGreenにした。PlanQuestionsでは回答欄のJS trimと、指紋欄のASCII空白のみを区別して比較した。Testing Postureのcustom ordering、構造化Methodology/Ordering、notes、節見出し末尾にも同じ規範を適用した。現在、PlanQuestions24観測、Testing Posture44観測が一致する。SummaryQuestionsの回答前BOM/NELと末尾BOMも追加し、52観測が一致した。ログは`human-whitespace-{red,green}.log`、`testing-whitespace-{red,green}.log`、`testing-trim-{red,green}.log`、`testing-structured-whitespace.log`、`plan-whitespace-{red,green}.log`、`summary-whitespace-red.log`、`protected-input-regression.log`。

固定本家へ実際にbyte FFのstdinを渡す採取を追加した。本家は匿名HUMAN_TURNを残すが、nativeはread_to_stringのInvalidDataで欠落するRedとなった。Bun.stdin.textと同じ置換復号へ接続してGreen。保護された応答のsession/responseの照合は別であり、匿名入力が計画承認候補になる変更ではない。stdout/stderr/exitと監査追記全文（Timestampのみ対応付け）、状態本文不変を検証した。ログは`human-invalid-utf8-{red,green}.log`。

## 保存・参照投影の回帰整理

書込側DTOからRMU側DTOへの保存境界を、現在の全21イベント変種へ拡大し、変種の重複も拒否するようにした。全件の一致を確認した（`all-event-wire-contract.log`）。スナップショットの記録バイトには新しいactive_directiveとrun floorを明示し、adapterの100テストが成功した（`adapter-regression-{before,after}.log`）。

参照モデルの再構築時にread_testing_contract/read_plan_fingerprintが残るRedを検出し、両者も再構築で破棄されるよう是正した（`reference-rebuild-{red,green}.log`）。現在space DBの構造化表はジャーナル由来17＋参照由来4の計21表である。steeringの順序検査は、見出しだけの空fixtureを、本家が配送する本文付きの規則に置き換えて本来の順序/初期化除外を検査する。RMUの306テストが成功した（`rmu-lib-regression-after.log`）。

この節の全ログ接頭辞は`/tmp/amadeus-u2-`。各整理の区切りでcargo fmt --allを実行し、今回の回帰整理後はcargo fmt --all --checkも成功した。これらは途中の範囲検証であり、全workspace・90%床・CI・残りステップの完了を意味しない。

### 共有承認集約の発行回・応答・失効準備（Step 5、継続中）

- 親所有の `approval-runtime-questions.md` にある追加承認を適用し、全intent・spaceで共有する `PlanApprovalRuntime` をドメインへ追加した。保存先追加は承認済みだが、この項の時点では共有SQLite/Repository/RMU/CLIの配線は未完了である。
- 既存 `plan_authority_contract` の固定本家challenge構築を戻り値付きfixtureへ整理し、既存1件の成功を確認した（`/tmp/amadeus-u2-plan-runtime-fixture-refactor.log`）。元の本家バイト・ハッシュのアサートは保持した。
- 新しい集約の提示発行口に未実装を返す骨格とテストを先に置き、Redを確認した。`/tmp/amadeus-u2-plan-runtime-issue-red.log` は実行1件が発行未実装で失敗、同 `issue-green.log` は既存を含む2件成功。コンパイル不備をRedとは扱っていない。新しい発行回IDは、内容から作る本家公開challengeIdと分けた。
- 応答記録口も同じ順で `response-red.log` → `response-green.log`（3件）を確認した。観測時の発行回ID、元のsession、提示した選択肢で照合し、意味上の選択と元応答のSHAを事実として保存するドメインイベントを追加した。
- 同一質問の再発行後に旧発行回の応答が到着するケース、session公開キーが同じでも元の名前が違うケース、有効応答後の無関係発言、イベント再生の同値を追加回帰で確認した（`observation-boundaries.log`、5件）。これらは直前の発行回照合の実装で既に成功した境界検査であり、別のRedを作ったとは記録しない。
- 通常指示の失効準備を先に記録して、未完了中は新しい提示・応答の受領を拒否する境界を `barrier-red.log` → `barrier-green.log`（6件）で実装した。元の発行が確定していなければ以前の提示を保ち、確定した場合は共有提示全体を消す境界を `resolve-red.log` → `resolve-green.log`（7件）で実装した。
- 再試行による同一操作IDの再発行を拒否するテストは、実際に旧提示を作り直すためRedとなった（`retry-red.log`）。発行・応答・失効の確定済み操作集合を所有させてGreen（`retry-green.log`、8件）とし、失効後にも旧操作の応答が復活しないことを確認した。
- `cargo fmt --all` を各区切りで実行した。clippyは大型イベントの値サイズと、規則が許す破損履歴のpanic/expectへの理由注記を指摘した（`clippy.log`）。提示イベントのペイロードをBoxへ移し、再生の破損箇所だけ規則への参照付き注記を付けた。通常の拒否は引き続きErrであり、履歴破損を成功へ読み替えていない。
- ログの共通接頭辞は `/tmp/amadeus-u2-plan-runtime-`。この項のGreenはドメイン内の境界である。実SQLiteの再起動、2ストア間の途中失敗、共有RMUとID指定Query、監査の単一書込所有者の接続は次に検証する。

### ワークスペース全体の承認状態を管理する集約の保存・投影・Query（Step 5、継続中）

- `PlanApprovalRuntime` の正式なRepositoryポート/SQLite実装を追加した。最新スナップショットとそれ以後のイベントだけを使い、毎回のイベントとスナップショットは本家 `event-store-adapter-rs` の同一トランザクションで保存する。ドメインにはserde/SQL/ストアtraitを追加していない。`StorePath::for_runtime` の導出先は追加承認どおり `.aidlc-runtime.sqlite` である。
- 記録済みの承認値を、現在の文書から再解釈せずに復元する口を `CodeGenerationAuthority` / `PlanApprovalEvidence` へ追加した。`/tmp/amadeus-u2-plan-value-reconstruction-red.log` → `green.log`、続いて不正な指紋・実行境界・質問パスの拒否を `plan-value-validation-red.log` → `green.log` で確認した。`plan-runtime-snapshot-red.log` → `green.log` は提示と応答、未完了失効をスナップショットから再構成する境界である。
- `cargo test -p core-command-interface-adapter --test plan_approval_runtime_repository_contract` は、保存口が未実装のため1件失敗するRed（`plan-runtime-store-red.log`）から、誕生→提示→応答→失効準備の4イベントを追記して再オープンできるGreen（`plan-runtime-store-green.log`）へ進めた。
- 保存境界の追加回帰は4件成功（`plan-runtime-store-recovery-final.log`）。追記失敗とスナップショット更新失敗の双方で、イベント/基底に部分的な更新が残らず、再試行後も未完了失効を保持する。古い版の別接続による上書きはConflictで拒否する。古い基底と後続3イベントからの再生も全状態で一致した。
- 障害注入の最初のスナップショット用triggerはINSERTに置いたが、固定ライブラリの更新はUPDATEだったため発火しなかった（`plan-runtime-store-recovery.log`）。これは製品不具合のRedではない。ライブラリ実装のUPDATEを確認してtriggerを修正し、実際の保存失敗とロールバックを再検証した（`recovery-after.log`）。
- RMUは書き手と独立した復号DTOで同じDBのjournalを読み、集約を再生して `read_plan_operation` を構築する。行集合と `amadeus_plan_projection_checkpoint` は同一トランザクションで確定する。RMUの新入口が未実装のRedは `plan-runtime-projection-red.log`。途中でSQLite整数変換とDTO可視性のコンパイル不備があり、これらはRedとは扱わない。Query検査をいったん外してRMU単体の結合成功を確実に確認した（`plan-runtime-projection-green-verified.log`）後、Query検査へ戻した。
- Query側は `PlanApprovalOperationView` / DAO / UseCaseだけを追加した。`plan-runtime-query-red-verified.log` は、保存した操作IDの行を未実装DAOが返さないためのRed。`plan-runtime-query-green-after.log` は、指定IDの1行をSELECTする実装で成功した。Queryのプロダクション依存にドメインやRMUはなく、状態・ハッシュ・結果を再計算しない。
- `PreparePlanInvalidationUseCase` は `Result<(), PlanApprovalCommandError>` のみを返す。集約へ準備を命じ、返った1イベントをRepositoryへ保存し、その後のRMU/Queryで既知の操作IDを読む。未実装のRed（`plan-runtime-command-red.log`）→結合のGreen（`plan-runtime-command-green.log`）を確認した。関連clippyも成功（`plan-runtime-command-clippy.log`）。
- `cargo test -p aidlc --test plan_runtime_contract` の2件が成功（`plan-runtime-projection-recovery-final.log`）。保存済み未投影の間はQueryが前の行を返し、投影INSERTの障害でも前の行とcheckpointがそろって残る。障害解除→RMU再オープン→追いつきで同じ操作IDが更新され、繰り返してもjournal件数は増えない。
- 記録済み人間応答のSHA-256不正を `plan-response-value-red.log` → `green.log` で拒否した。さらに適用済み操作IDを別イベントで再生しても無言で再実行しないよう、破損履歴のテストを `plan-runtime-corrupt-replay-red.log` → `green.log` で追加した。通常のCommandの重複はErr、破損した保存履歴の再生は規則どおり停止する。
- 上記ログは共通して `/tmp/amadeus-u2-` 配下である。`cargo fmt --all` は各区切りで実施済み。この時点で共有ストアは一時テスト先でのみ使っており、本リポジトリのホスト保存先は切り替えていない。
- **未完了**: 本家互換のchallenge/response/receiptファイル投影、receipt/begin、通常指示発行と共有失効の2ストア連携、元の発行/監査操作IDによる回復、CLI/フックからの接続、共有ストアのGit除外/利用済み欠落の診断引継ぎ。保存/投影の個別境界が成功したことを、2ストア途中失敗の検証完了とは扱わない。全9ステップの残作業も継続する。

### 通常nextの共有失効と初回/欠落の区別（Step 5/6、継続中）

- `DirectivePublication` の操作IDを `DirectiveIssued` に保持し、実行集約の発行済みID集合をスナップショット/差分再生の双方へ追加した。最新markerだけで「保存済み/未保存」を判断しない。`/tmp/amadeus-u2-directive-operation-red.log` → `green.log`、`directive-operation-store-red.log` → `green.log` が対応する。
- 回復UseCaseは各集約を各Repositoryで読み、共有側が元のspace/実行と発行履歴を照合して失効を確定する。呼出側が任意のboolを渡す公開口はprivateへ閉じた。`invalidation-source-red.log` → `green.log`、`invalidation-recovery-command-red.log` → `green.log` で、space保存済み→共有保存失敗→再接続→元の発行確認→失効完了を検証した。テスト補助の可視性をpub(super)へ是正した後、関連回帰も成功した（`invalidation-owner-refactor-*-after.log`）。
- 通常 `next` はOSファイルロックを保持し、先行操作の回復、失効準備、実行側の発行、共有側の失効、RMUを順に行う。`approval-file-lock-red.log` → `green.log`、`shared-publication-cli-red.log` → `green-after.log` で確認した。Stop probeはこの更新へ入らず、共有ストアを作らず/イベントを増やさない。
- 初期化の機械ローカル記録 `.aidlc-runtime.state.json` はinitializing/readyを区別する。readyの共有DB欠落は新規作成にしない。Aで使用→欠落→新しいB（同一/別space）でも拒否した（`shared-store-scope-regression.log`）。初回作成途中の誕生未保存/保存済みは回復でき、利用後の履歴を古いinitializingで初期化し直すことは拒否した（`shared-initialization-recovery.log`、`shared-initialization-guard-green.log`）。
- 初期の欠落テストはexit 1を仮定していたが、既存nextの拒否契約はstdoutのkind:error/exit 0である。誤った仮定を直し、初期化接続だけを変更前へ戻して正しいRedを実行後、実装を戻してGreenを取り直した（`shared-store-missing-red-corrected.log` → `green-corrected.log`）。この経緯を省略しない。
- Git除外は本体/WAL/SHM/lock/初期化記録に限定して追加した。`git check-ignore --no-index` の変更前exit 1と変更後exit 0は `shared-runtime-ignore-{red,green}.log`。intent成果物のcode-summaryは引き続き除外されない。U3はこれらのファイルの欠落・不読・初期化不整合を診断し、診断で空DBを作らない。

### 計画承認のdecisionとchallengeの実CLI・本家全文比較（Step 5）

- 実際にbugfixの進行でCode Generationまで通したfixtureで、本家と同じdecision argvを検証した。最初のRedは `--stage-level` を値付きとして扱う誤解析（`plan-decision-cli-red.log`）。フラグ解析、実行集約による根拠検証、space側の計画監査、共有側のChallengeIssued、RMUの本家形式JSONを接続してGreen（`plan-decision-cli-green.log`）になった。
- `PlanDecisionEvidence` は文書の根拠とsessionを対で保持する。Sourceの監査を共有ストアが書く構成にはしない。Root/SourceのDTOは各側で保持し、Evidence DTOの共通化も同じ側の内部に限定した。大型enumの指摘はPlanDecisionEvidenceのBox化で是正し、lintを抑止していない。
- キー集合だけでは十分でないため、同じworkspaceの状態/発行/文書を固定本家へ渡し、導出ハッシュを含むchallengeの全バイトを比較した。最初はHOMEをproject直下へ置いたためBunが `Library/Caches/bun/` を追加し、比較前にソース指紋が動いた。これは検査装置の不備であり製品Redではない。`aidlc/.capture-home` に揃えて再実行し、stdoutとJSON全バイト一致、正規化0を確認した（`plan-decision-full-parity-corrected.log`）。nativeの指紋計算や期待hashを置換していない。
- 再実行は `scripts/goldens/capture-plan-decision-parity.ts`。固定SHA/原入力/両出力/nativeバイナリSHAは `tests/golden/selfhost-stage1/plan-decision-parity.json`。通常Rust CIにBun依存を足さず、外部比較は採取コマンドが明示的に実行する。
- この時点で `upstream_271_contract` の43件を全実行して成功した（`upstream-271-shared-runtime-regression.log`）。これは旧2.6.40参照を含むworkspace全体の成功ではない。

### 人間応答の先行保存と、元の記録への配送（Step 5/6）

- `PlanResponsePrepared` に観測先・発行回・原文・当時の選択を固定し、Source側PromptObservedに同じ観測操作IDを残す。共有側ResponseObservedはSourceの記録を確認してから反映する。準備だけでは受領済みにならない。`plan-response-preparation-red.log` → `green.log`、`plan-response-delivery-red.log` → `green.log`、`plan-response-store-red.log` → `green.log` が対応する。
- 元の観測操作IDはInteractionStateに保持し、後の無関係な人間ターンで失わない。Sourceの記録が既にある場合は、集約の配送判断がRecordedとなり、UseCaseは二重にイベントを作らない。
- 実CLIの文字列"1"をSource/共有の両ストアへ保存し、RMUがresponse JSONを描く境界がGreen（`plan-human-cli-red.log` → `green.log`）。session/challengeId/意味上の選択/元応答SHA/state不変/別sessionによる非上書きを確認した。
- 当初は同じ提示とraw番号応答による全responseバイト一致と記録したが、その主張は撤回した。比較前にnativeのresponseを残していたため、本家の不変更を新規生成と誤認した。後段「人間応答比較の持越し是正」を正とする。
- 共有側ResponseObservedだけを失敗させ、Source保存済み→別プロセス入力で回復→Source観測を二重記録しないことを確認した（`plan-human-recovery-cli.log`）。その失敗中の再提示で、同一内容/変更内容のどちらも旧応答を残さない（`plan-human-reissue-cli.log`）。同一公開challengeIdでも発行回は別である。
- さらにAのSource保存を失敗させBへ切り替えて回復したところ、Aのイベントだけ保存されAの監査が未投影になる差を検出した（`plan-response-origin-audit-red.log`）。配送をSource保存→元recordのRMU→共有受領確定へ分け、元recordは既存execution Viewと登録簿のQueryで解決する。active-intentを変更せず、Aの監査まで反映してGreenになった（`plan-response-origin-audit-green.log`）。
- **残り**: receiptの保存/監査配送、begin、各拒否の逐語/ERROR_LOGGED、残る3フック・補助更新・旧受入参照の移行・最終全検査。decisionのchallenge保存失敗は本家と同じ監査先行の部分成功となる。親は固定本家の保存順序と承認済み契約を確認し、明示的decision再試行で扱う方針との整合を確認した。旧challenge/応答の維持と再試行時の不流用の実測比較は残る。任意のhuman-turn開始時に未提示offerを自動再発行する処理は入れていない。

### 受領モデルの着手（未完了）

- PlanApprovalReceipt/PlanAnswer/配送状態のモデルを追加中。最初の実行が公開型の文書不足でコンパイル失敗したため、これをRedとは扱わない。記録メソッドだけを未実装へ戻し、文書を是正した後に `plan-answer-receipt-red-corrected.log` で真正な応答なし/ありの境界を実行した。実装を戻し `plan-answer-receipt-green-corrected.log` でGreenを確認した。CLI受領完了とは扱わない。

### 計画回答の監査配送・認証取消し・保存（Step 5、継続中）

- 集約の回答受領→元IntentExecutionのPlanAnswerLogged→共有AnswerCompletedを接続した。`plan-answer-audit-red.log` の未実装配送判断を確認後、`plan-answer-audit-green.log` で成功した。未保存の元実行では完了せず、同じ操作の監査を二重作成しない。
- 認証直後のソースが異なる場合はAnswerAbortedにより受領を除去し、元のchallenge/応答を残す。取消しも集約が現在のソースと元実行を照合し、呼出側が任意に取り消す口を作らない。引数をその判断材料へ揃えて未実装のまま再実行した `plan-answer-abort-red-corrected.log` → `plan-answer-abort-green.log` が対応する。
- 監査待ち回答がある間は新提示・通常指示の失効・新応答を保留する。`plan-answer-pending-red.log` で新提示を通してしまう差を検出し、`plan-answer-domain-regression.log` で16件成功した。
- Rootの受領/回答履歴、Sourceの監査操作IDはそれぞれ所有集約で保持し、独立DTOで保存する。SQLite再オープンの初回はDTO公開変種の文書不足でコンパイル失敗したためRedではない。文書修正後の `plan-answer-store-red-corrected.log` では復元回答がNoneになる真正のRedを確認し、スナップショットの完全状態へ追加して `plan-answer-store-green.log` の6件が成功した。
- この時点ではreceiptのRMU/Query/CLI、beginの接続は未完了。最終バイナリでchallenge/response/receipt/beginの固定本家比較を再実行する。人間入力中のJSON整数キーの列挙順も残るStep6境界として保持する。

### 受領の投影・ID Query・実CLI（Step 5）

- PlanAnswerLoggedの監査投影は公開stateを書き換えず、固定2.7.1のPLAN_APPROVAL_RECORDED/QUESTION_ANSWEREDとplanApprovalFieldsの順序へ合わせた。最初の検査は借用/公開語彙の参照不備、次は合成run floor不備であり製品Redではない。入力修正後 `plan-answer-audit-projection-red-valid.log` で未実装の投影拒否を確認し、`plan-answer-audit-projection-green.log` が成功した。新語は固定a277のaidlc-audit.ts:81/230に実在する。
- 共有RMUは現在のreceipt JSONと、監査配送待ちの操作ID/元space/元executionを投影する。末尾改行を期待値に含め忘れた検査装置の誤りは、固定aidlc-lib.ts:2862の書込を確認して是正した。該当投影接続を戻し `plan-receipt-projection-red-corrected.log` → `plan-receipt-projection-green-corrected.log` を再実行した。ハッシュや本文を正規化していない。
- `read_plan_answer` はRoot内に置き、指定操作IDでpending/recorded/abortedを読む。Queryは独立のView/DAOを読み、ドメインやファイルを読み直さない。`plan-answer-query-red.log` → `plan-answer-query-green.log` で保存済み未投影→再起動RMU→同じIDの結果/未知ID/冪等を検証した。親も同じ対象テスト1件を独立実行して成功した（integration-baseline.md）。
- `aidlc-log answer --checkpoint plan-approval` をRecordPlanAnswerUseCase→Root保存→RMU→ソース再照合→Source監査保存/投影→Root完了→ID Queryへ接続した。最初のテスト補助APIの参照誤りはコンパイル失敗でありRedではない。修正後 `plan-answer-cli-red-corrected.log` の未配線拒否から `plan-answer-cli-green.log` へ進めた。
- 全receiptバイト/stdout/stderrは同じ一時workspace・同じ回答文書・同じ提示/応答で固定本家answerとも一致した（`plan-receipt-full-parity.log`）。本家へ渡す直前に回答前のchallenge/responseだけを復元し、nativeストアを本家入力にはしない。来歴・元バイトは `tests/golden/selfhost-stage1/plan-receipt-parity.json`、手順は `scripts/goldens/capture-plan-receipt-parity.ts`。比較時のbinary SHAを持つので、最終版では再実行する。
- 実SQLiteのSource監査保存を失敗させた後、再プロセスでソースが同じなら監査を1回だけ配送し、ソースが違えば取消しイベントでreceiptを除去してchallenge/responseを維持した。`plan-receipt-recovery-cli.log` の2条件が成功（テストのPath借用不備を直してから実行）。これは保存済み/未保存を識別して回復する検査であり、本家の一般的な全操作原子性を主張しない。
- 保存状態の受領に対応する回答がない場合と、監査待ち承認の受領候補欠落は復元で拒否する。`plan-receipt-invariant-red.log` → `plan-receipt-invariant-green.log`（16件）で確認した。
- **残り**: begin、受領公開直後の同一呼出内編集を含む本家比較、拒否の逐語/ERROR_LOGGED、他3フック、補助更新、旧受入参照の移行と全体品質検査。CLIの正常系や上記障害回復を、これらの完了と混同しない。

### 実装開始の判定・保存・投影・CLI（Step 5、継続中）

- `CodeGenerationApproval` は本家 evaluateCodeGenerationApproval の全12公開フィールドを判断する。固定本家の14観測（`plan-readiness.json`）に対し `plan-readiness-red.log` → `plan-readiness-green.log` が成功した。開始済みでソースだけ変更した場合は可、開始済みでも計画・質問・現在の規則を変更した場合は拒否する。準備時のTesting Posture節の末尾改行欠落は、本家の実ファイル解決との自己比較で検出して原文を保持した。製品Redには数えない。
- 開始要求・公開後の確定・失効をそれぞれイベント化し、照合待ちの間は他の承認更新を保留する。`generation-request-red.log` → `generation-request-green.log`、`generation-certify-red.log` → `generation-certify-green.log` で公開前後のソース不一致、確定の二重実行、開始済み要求の冪等な意味を検証した。
- 開始操作のスナップショット欠落は実SQLiteの再オープンでRedとなり、保存へ追加して共有Repository全7件がGreen（`generation-store-red.log` → `generation-store-green.log`）。RMUは開始結果と回復待ち操作を投影し、Queryは同じIDのpending/generationを読む。DTO引数の機械変換不足のコンパイル失敗を修正後、`generation-query-red-corrected.log` → `generation-query-green.log`（4件）が成功した。親の独立実行も成功している。
- begin CLI の最初のRedを実行中に、終了結果の確認より先にSource照合メソッドと2つのUseCaseファイルを書いた。順序の不備として、その新規差分だけを戻し、`begin-cli-red-reconfirmed.log` で未配線拒否を再確認してから実装を戻し、CLI接続を行った。他の既存差分を巻き戻していない。`begin-cli-green.log` が成功した。
- beginは現在文書の検査に使う観測と、公開前/後のソース観測を分ける。各UseCaseはunit成功を返し、RootのGenerationRequested/Certified/RevokedとRMUを経た操作ID Queryから公開結果を組む。正常開始、開始後のソース編集と再実行、質問編集後の拒否をCLIで検証した。
- 固定本家beginとのstdout/stderr/受領全文の比較も成功（`begin-full-parity.log`）。同じ文書を保持し、回答済みapproved受領を復元して本家を実行する。`capture-plan-begin-parity.ts` と `plan-begin-parity.json` に元バイトと来歴を保持する。最終バイナリの比較は全変更後に直列で再実行する。
- 確定保存失敗からの再開検査では、ソース変更による受領除去後に空ディレクトリをRMUが削除する差を検出した（`begin-recovery-cli.log`）。固定本家clearPlanApprovalReceiptはunlinkだけで空ディレクトリを残し、resetPlanApprovalRuntimeは全削除する。`capture-plan-runtime-directory.ts` / `plan-runtime-directory.json` に実測を追加した。RMUで保存された失効種別に従う配置へ是正し、回復検査を継続している。
- receipt完了後のRefactorではClippyが採取コードの直接serde JSON直列化・不用なclone・添字操作等を検出した。テストの採取出力もcanon_json経路へ揃え、`cargo clippy -p aidlc --all-targets -- -D warnings` が成功した（`receipt-clippy.log`）。新たな抑止は追加していない。この結果は開始イベント追加前の節目であり、最終検査は別途必要である。

### 人間応答比較の持越し是正（検査装置の不備）

- 固定本家のextractResponseTextを実関数のまま実行し、文字列"1"→空、標準ラベルApprove Plan→同じラベル、整数キーを持つオブジェクト→整数昇順の最初の値を確認した。番号文字列保持は承認済み是正であり、本家未修正と同じという意味ではない。
- 以前のhuman比較はnativeのresponseファイルを削除せず本家を実行したため、本家が何も生成しなくても一致した。`plan-human-full-parity.log` と旧 `plan-human-parity.json` に基づくraw番号のフック全文一致という主張を撤回する。旧採取物は `test-evidence/invalid-raw-number-human-parity.json` に無効証拠として保存した。
- 比較前のresponse不存在をassertし、本家による新規生成を確認する検査へ変更中。初回はOption化した比較の型修正が不足してコンパイル失敗したため、その実行は反証やRedに数えない。修正後にraw番号の比較を実行して持越しを検出し、標準ラベルの一致と承認済み番号修正を別々に記録する。
- decision/challenge と receipt の出力も本家実行前に除去して新規生成を確認する。beginは入力approved受領の復元が元からあり、同じ不備と断定しない。入力approved→出力generationとバイト非一致を明示して確認を強める。


## 2026-09-08: setter・getter・完全構築の明示是正

利用者の指示を優先し、write-audit-logの実装を一時中断して是正した。`with_*` は消費型ファクトリでありsetterとはしない。集約ID5型は集約名+Idに適合しており、操作相関IDを集約IDと誤認して改名していない。全体の構築棚卸しは `construction-inventory.md` を参照する。

1. **新lintのRed→Green**: setter-methodを未接続の状態で4件失敗・1件成功を確認（`/tmp/amadeus-u2-setter-lint-red.log`）。impl/traitのset_*（private、関連関数、cfg(test)、raw識別子を含む）を構文木で検出し、allowコメントでは抑制しない。with_*、自由関数、コメント/文字列は非検出。実装後98件成功（`/tmp/amadeus-u2-setter-lint-green.log`）。マクロ展開は対象外であり、未展開内容まで保証しない。親の独立CLI7ケースも成功した。
2. **getter是正**: RecordPlanAnswerUseCaseの対象取得をRepositoryへ、選択・ソース対応の判断をIntentExecutionへ移動。RecordPlanDecisionUseCaseの選択肢解釈をPlanChallenge::from_promptへ移した。4件の機械所見に加え、PlanChoice::as_strのUseCaseでの変換も解消。getter改名やQueryへの判断移動では回避していない。decision/answerのCLI2件成功（`/tmp/amadeus-u2-getter-behavior-regression.log`）。
3. **set_stateのRed→Green**: `recorded_generation_events_require_the_matching_pending_operation`。他操作、終端済み状態、同イベント再適用を拒否し、対応するPendingだけを認証/失効する。`/tmp/amadeus-u2-setter-domain-red.log` → `...-green.log`。ReadModelの任意文字列setterもDirectiveIssuedの適用に変更した。
4. **構築経路のRefactor**: Defaultと通常構築、派生コレクション、Repositoryの再オープンを同じ完全構築へ集約。初回domain回帰は701成功・5失敗。そのうち2件は往復テストが追加済みrun-floorを復元材料へ渡していなかったため、現在の全材料を使う形へ是正。残る3件は旧監査語彙期待だった。planフィルタ30件、完全構築往復2件、adapter100件、lintツール98件成功。途中clippyでconst不足7件とRepositoryのstrategy材料漏れ2件を検出し是正した。これは成功証拠ではなく検出→修正の経緯である。
5. **RecomposedのRed→Green**: `a_recorded_recomposition_changes_only_its_named_slot` はExecute/Skip不一致でRed。任意PlanActionを受けるoverride_plan系を除去し、記録された対象slugだけに適用。既存再構成を含む9件成功（`/tmp/amadeus-u2-recomposition-event-{red,green}.log`）。
6. **ゲート・前進のRed→Green**: `a_gate_event_changes_only_the_named_stage_progress` はPending/AwaitingApproval不一致、`a_recorded_completion_starts_only_the_next_effective_stage` はPending/InProgress不一致でRed。その後、Gate系とReportedの事実から対象と次の実効ステージを導出した。質問clearはIntentExecutionのゲート/改訂・報告の元の分岐に保持。`/tmp/amadeus-u2-gate-event-{red,green}.log`、`...-progression-event-{red,green}.log`。
7. **Started完全構築のRed→Green**: initializationを完了、最初の実ステージを進行中、他を未着手、承認は全falseとして完全構築する。最初のテストには末尾の否定assertの誤記があり、実装後にその誤記で失敗した。誤記を直し、実装をgenesisへの委譲に戻して正しいRed（Pending/Completed）を再確認してから実装を戻した。正式証拠は `/tmp/amadeus-u2-started-construction-red-corrected.log` と `...-green-corrected.log`。元の `...-green.log` は失敗ログであり成功扱いしない。
8. **JumpedのRed→Green**: `jump_events_preserve_forward_skips_and_backward_resets` はInProgress/Skipped不一致でRed。保存済みJumpedと適用前カーソル/計画/進捗から従来の読み飛ばし・巻戻しを導出し、mark/mark_allを除去。対象外SKIP・redo・承認無効化を含む既存13件成功（`/tmp/amadeus-u2-jump-event-{red,green}.log`）。
9. **監査語彙のRed→Green**: 固定本家2.7.1のSetと見出し宣言を無変更で実行し、`tests/golden/selfhost-stage1/audit-registry.json`へ91語全値を保存。Rustの全値照合は87/91でRed。4語と見出しだけを追加してGreen。swarm等の実行機能は追加していない。初回のテスト実行はimport配置/重複によるコンパイル失敗だったためRedには数えず、修正後の87/91不一致を正式Redとした。`/tmp/amadeus-u2-audit-registry-red.log`、`/tmp/amadeus-u2-domain-corrections-green.log`（domain全711件成功）。

最終のCLI/投影・ITF・fmt/clippy/cargo lintは、この記録時点では実行中。これらの完了前に是正全体完了とはしない。FCC要素のプリミティブ禁止とPendingIteration移行・新lintは利用者の明示承認により後続 [Issue #123](https://github.com/amadeus-dlc/amadeus-ng/issues/123) へ分離しており、この是正の完了条件へ混ぜない。完了後は追加許可を求めずwrite-audit-logへ復帰する。


### 是正完了の追記

2026-09-08: ReadModel/MemoryFacesの任意本文setterも除去し、RMU内の完成値を返すファクトリへ集約した。ReadModelは既存の監査追記/指示/メモリ面を完全コンストラクタへ渡し、MemoryFacesは描画済み本文をdirty=trueで構築する。旧値を不要とするMemoryFacesファクトリにselfを付けた初回形はclippy unused_selfで検出され、関連関数へ直した。抑制は追加していない。

最終検査はdomain711、adapter100、CLI契約53、計画結果の縦結合4、RMU307、engine ITF、lintツール98が成功。cargo lint、両clippy --all-targets/-D warnings、両fmt --check、git diff --checkも成功した。構築AST再走査の残候補は0件（検出限界は棚卸しに明記）。生ログをtest-evidenceへ保存した。今回の明示是正は完了し、承認済みU2のwrite-audit-logへ復帰する。U2全9ステップ全体やCIを完了扱いにはしない。


### 是正後の追加RMUファイル結合

`read_model_updater_test`は初回29成功・2失敗。mtime保全自体は成功した。失敗の正確な原因は、Fixtureがorg.mdを `# Org\n`、phaseを `# Inception\n` の空見出しだけにしていたことである。固定本家2.7.1のisSubstantiveRuleTextを直接実行し、これらが配信対象外になることを確認した。件数期待を減らさず、規則の投影/削除を検査するfixtureを本文付きへ修正した。

同じSource観測10件でBOM/NELの差も比較した。Rust trimではbom-onlyを実体ありとしてしまう真のRedを確認後、既存ECMAScript trimへ変更し10件Green。最初の実行はテストを内側attributeの前へ挿入したコンパイル失敗であり、Redには含めない。修正後の真のRedを保存した。結果は `corrections-rmu-files-green.log` の全31件成功（本文/mtime保全を含む）、`steering-content-green.log`。全体失敗を旧期待と推測したまま放置せず、固定本家の入力/出力を確認して閉じた。追加静的解析も `steering-content-clippy.log` で成功。

### write-audit-log / HookHealth の復帰

- 固定本家の実フック8呼出し（cold/active、不正JSON、監査directory障害など）を採取し、heartbeatが入力検査に先行し、dropが `timestamp<TAB>reason\n` で追記される事実を保存した。heartbeat書込先directoryのBun stderrは処理系固有であり、親の裁定待ちとして受入比較だけ保留する。
- HookHealthTarget、HookName、HookHealthIdの各Red→Greenを実施。`HookHealthId`は対象領域＋フック名から安定導出し、集約IDとHookHealthEventIdを別型で保持する。
- Started、HeartbeatObserved、AuditDroppedのイベント族と完全コンストラクタを追加。初回発火、後続heartbeat、drop理由のCRLF/LF一行化をRed→Greenで確認した。既存 heartbeat を更新せずdrop件数だけ増やす。
- HookHealthRepository（Commandポート）とSQLite実装を追加。`heartbeat_and_drop_are_persisted_as_separate_events_and_replayed` が1件Greenで、再オープン後のseq3/drop1を確認した。
- HookHealth専用RMUを追加し、`read_hook_health` と専用checkpointを同じ共有DBへ投影。`rmu_projects_only_hook_health_events_after_interleaved_journal_rows` は、HookHealth3行＋plan-approval別stream1行の計4行を実際に投入し、HookHealthだけを投影して1件Green。
- PlanApprovalJournalReaderもRed→Greenでown aid/連番へ修正。health streamのrowidを承認seqと誤認しない。`plan_reader_ignores_an_interleaved_hook_health_stream` 1件Green。
- CLI `aidlc hook write-audit-log`をHookHealth保存→専用RMUへ接続。`write_audit_hook_records_a_heartbeat_in_the_shared_runtime_store` はcoldの不正JSONと2回目のnull入力でexit0、共有journalを1→2件、heartbeat公開ファイルを確認してGreen。
- 残り: write-audit-logのartifact監査本文（対象ファイルのCreated/Updated投影）、dropファイルの完全履歴投影、heartbeat-directoryのBun固有stderr裁定、全体回帰。既存TS監査・承認ファイルを上書きする経路を成功扱いしない。


### HookHealthの4事実とCLI/RMU/Query

HookHealthTarget/HookName/HookHealthIdの境界、Started→HeartbeatObserved→AuditDropped、SQLite Repository、RMU read_hook_health、ID Query、CLI heartbeat/dropを順にRed→Greenで閉じた。保存/再生 `heartbeat_and_drop_are_persisted_as_separate_events_and_replayed`、RMU混在 `rmu_projects_only_hook_health_events_after_interleaved_journal_rows`、承認reader混在 `plan_reader_ignores_an_interleaved_hook_health_stream`、Query `query_reads_only_the_requested_projected_health_row`、CLI `write_audit_hook_records_a_heartbeat_in_the_shared_runtime_store` を保存した。

runtimeのartifact直接描画は一時的な出力確認後に除去し、`write_audit_hook_does_not_bypass_the_artifact_event_pipeline`でArtifact監査が未接続のまま直接追記されないことを固定した。Created/Updatedの旧出力Greenログは実装準拠の成功証拠として使わない。HookHealthの通常heartbeat/drop経路だけを成功とし、ArtifactSavedは未完了である。

### ArtifactAudit（専用RMU段階）

ArtifactAudit/ArtifactAuditId/ArtifactWriteObservationを追加し、保存観測を集約の `ArtifactSaved` 事実へ移した。`saved_artifacts_are_one_event_each_and_replay_by_the_same_id`、Repositoryの `saved_artifacts_survive_reopen_and_keep_one_event_per_command`、native intent-createからの `native_intent_create_then_artifact_hook_uses_the_event_pipeline` がGreen。runtimeの直接 `render_audit_block`/`append_audit_shard` は除去し、ArtifactAuditRepository→ArtifactAuditReadModelUpdater→Query（`query_reads_only_the_requested_projected_health_row`）へ接続した。TS由来recordのCreated/Updated/dropテストも保持している。

**未完了の統合差**: ArtifactAuditReadModelUpdaterは現在専用checkpoint・専用RMUとして存在する。承認済み要件は既存spaceの通常ReadModelUpdater/JournalReader/PublicationBatchへArtifactSaved DTO・投影・checkpointを統合し、監査shardの単一書込所有者にすることである。通常JournalReaderはartifact manifestを実行イベントとして誤読しないよう暫定で選別するが、同一PublicationBatchへの取り込みはまだGreenではない。専用RMU段階のGreenを最終完了へ拡大しない。

### ArtifactAudit縦結合（専用RMU段階の記録）

ArtifactSavedのドメイン集約・Repository・専用RMU・Query・CLIは順に接続した。native intent-createケース `native_intent_create_then_artifact_hook_uses_the_event_pipeline` と、TS由来recordを明示的に残した `write_audit_hook_projects_created_updated_and_drop_via_events` がCreated/Updated/dropの実測を通った。保存とQueryは `saved_artifacts_survive_reopen_and_keep_one_event_per_command` / `query_reads_only_the_requested_projected_health_row`。runtimeから監査shardへの直接書込は除去した。

これは専用RMU段階の証跡であり、完了証跡ではない。ArtifactAuditReadModelUpdaterが独自のcheckpointとPublicationFile書込を持つため、通常ReadModelUpdaterのJournalReader/PublicationBatchへArtifactSavedを取り込む作業を残している。通常JournalReaderはartifact manifestを拒否せず選別しているが、同一PublicationBatchでのatomicな状態/監査/Artifact投影は未検証である。専用RMUのGreenを設計準拠の完了と扱わない。

## 2026-09-09再開時の現在地とnext入力の是正

**上のArtifactAudit専用RMUの未完了記述は過去時点の記録であり、現在地ではない。** 2026-09-08T18:36:59Zの監査記録と再開時の現コードを照合した。ArtifactSavedは通常のJournalReaderImpl→JournalBatch.artifacts→ReadModelUpdater→PublicationBatchへ統合済みで、専用ArtifactAuditReadModelUpdaterは存在しない。`read_artifact_audit`は通常read表の生成・削除・ダイジェスト・置換に含まれる。再開時にこの実装を戻したり、人工的なRedを作ったりしていない。

19:09以降に加わったnextの名詞引数・複数recordの観測まで保持して再開した。変更前の `cargo test -p aidlc --test upstream_271_contract` は53件成功（`test-evidence/resume-baseline.log`）。これは変更前の基準であり、U2全体の完了を意味しない。

### 今回修正した観測差

固定本家2.7.1 `aidlc-orchestrate.ts:1434–1460,3903–3948` と `aidlc-lib.ts:732–823` は、先頭workspace名詞が後続フラグを含む全argvを所有し、list/create/switchをユーティリティのargvへ写す。既存差分は文中の名詞も取り込み、`intent list --status` の末尾を読取フラグへ昇格し、出力のコマンド綴り・終端文言も異なっていた。

1. `scripts/goldens/capture-next-input.ts` が固定配布物277ファイルのマニフェストを検証し、空の独立workspaceで21入力を実行。stdout/stderr/exitを無変換で `tests/golden/selfhost-stage1/next-input.json` へ保存した。U1の保存コーパスは変更していない。
2. 公開バイナリを呼ぶ `cargo test -p aidlc --test next_input_contract` がexit101。最初の `next intent` のstdoutで `aidlc-utility intent`＋terminal文言と、本家の `bun .claude/tools/aidlc-utility.ts intent`＋短い終端文言が不一致（`test-evidence/next-workspace-red.log`）。コンパイル失敗ではない。
3. CLIパーサは先頭名詞の全argvを保持して返す。workspaceの構文変換はControllerの `turn/workspace_command.rs` へ置いた。状態判断・Repository・ドメインの更新経路は追加していない。表示は既存EngineCommandのshell quotingを使い、空白と引用符を保持する。
4. 同じ全文比較がexit0（`test-evidence/next-workspace-green.log`）。list/json、明示switch、create、help、予約語、名前欠落、後続フラグ、空白と引用符を持つ引数を含む21入力すべてが一致し、nativeはaidlcディレクトリを作らない。文中/`--`後の名詞を自由記述として扱う公開パーサの回帰も成功した。この追加回帰は既に修正済みだったため、別のRed達成とは数えない。
5. 採取側にworkspace無変更のassertを追加した初回再採取は、HOMEをworkspaceへ重ねていたためBunのLibraryキャッシュで失敗した。HOMEを独立した兄弟ディレクトリへ置き直すと全21件でworkspace無変更を確認。再採取JSONは元の保存JSONと全バイト一致した。これは検査環境の是正であり製品Redではない。
6. Clippyは新規補助関数のconst不足と持越しテストの `== false` を検出した。抑止を足さず是正し、再実行が成功。構文解析の旧単体テストにあった「名詞後の--statusが読取フラグになる」という誤期待も本家の全argv所有へ直した。

### この区切りの検証結果

- `cargo test -p aidlc --test next_input_contract --test next_branches`: 2件＋44件成功。前者は21プロセス観測と自由記述3入力を含む。親も独立実行して成功。
- `cargo test -p aidlc --lib`: 287件成功。
- `cargo test -p aidlc --test claude_hook_contract`: 8件成功。Artifactのnative/TS由来record、Created/Updated/dropとheartbeatの接続を再確認（`test-evidence/resume-artifact-regression.log`）。
- `cargo test -p core-query-use-case --lib orchestration::engine_command`: 9件成功、42件は対象外フィルター。
- `cargo clippy -p aidlc --all-targets -- -D warnings`、`cargo lint`、`cargo fmt --all --check`、`git diff --check`: 終了0。
- 生ログは `test-evidence/next-input-*` と `test-evidence/next-command-regression.log`。この再開分の変更一覧は `next-input-checkpoint.json`。

### 次回の開始点と未完了

workspace名詞の上記21入力は是正済み。plugin/knowledge名詞の個別文法と宛先、doctor追加引数等の未接続next観測は、今回の成功範囲へ含めない。停止フック、補助更新、必要な残りの受領/拒否経路、旧受入参照移行、release比較と全体品質ゲートを続ける。U2最終のsource-manifest/traceability/code-summary、独立レビュー、全CI・90%床と相対条件は未完了である。

`hook-runtime-diagnostic-questions.md` のheartbeat-directory診断出力Q1は未回答。この1ケースの出力・比較規則は未確定のまま保持した。再開はこの最新節から行い、過去のArtifactAudit専用RMU記述へ戻らない。


## 2026-09-09: 診断裁定の実装とStopの縦結合（継続中）

heartbeat-directoryのQ1は人間がAを承認した。固定原観測の初期ファイルを復元し、`heartbeat_directory_failure_preserves_public_files_and_reports_eisdir`を実行。Redは原因がInvariantViolationへ潰されたstderr、Greenは承認済みのEISDIR/open/対象パスの1行だった。Bunの原stderrも種別・操作・実測パスを検査し、原観測は変更していない。全公開ファイル集合と既存バイトを前後比較し、別監査shardを含め無変更。Rust内部の共有SQLite/WAL/SHM/初期化markerのみ明示除外する。親も独立実行して成功。

Stopの作業なし入口は未接続のUnknown hookでRed。接続後にaidlc未作成の初回heartbeat欠落を検出し、共有初期化境界で親ディレクトリを作ることで標準JSON/不正JSON/nullが成功した。`stop-cold-green.log`は途中失敗で、正式Greenは`stop-cold-green-corrected.log`である。

active Stopは次の順に進めた。

1. `stop_blocks_pending_work_once_then_releases_without_workflow_progress`: Unknown hookでRed。WorkflowContinuation集約、既存共有SQLiteへのRepository、要求IDごとの結果投影とQuery、10秒上限付きnative nextのprobeを接続した。コマンド成功はunitのみで、呼出側のContinuationAttemptIdでQueryを引く。初回block、同じ進捗の2回目allow、block-count.jsonのcount2とワークフローstate不変を確認。初期段階をrequirements-analysisと書いたテスト期待はfixtureのreverse-engineeringと不一致だったため訂正し、`stop-active-green-corrected.log`が正式Green。この期待誤記は製品のRedではない。
2. `stop_limit_honors_the_numeric_prefix_and_reentrant_initial_count`: cap3でも初回再入を解除するRed。実行の最新集約から自律モードを参照して既定2/8を決め、環境の正の数値prefixだけを優先する経路を接続。cap3/3trailingで初回count2はblock、次のcount3はallowがGreen。無効値・自律モードの広い検証は継続する。
3. `stop_allows_an_open_human_gate_without_starting_a_no_progress_streak`: reportで開いたゲートにblockを出すRed。IntentExecutionが待機理由を判断し、RMUのread_execution.continuation_waitをDAOがIDで読む経路にしてGreen。QueryやControllerへcheckboxの再判定を置いていない。

現在のStopは未完了である。進捗署名の固定本家全値比較、再入/上限/回復、正当な未回答文書・compose・背景agent・会話・resume待ち、done/park等のreset、セッション選定と各公開副作用を残す。WorkflowContinuationは初回/後続の停止判定を単一イベントとして保存し、RMUはDB排他中にmarker公開を行った後で結果と位置を確定する段階であり、障害注入・並行実行の契約検証は未実施。HookHealthの既存replay/applyのResult戻り値等の規則違反も最終レビュー前に是正する。

タイムアウト付きprocessにtokioのprocess/time機能を追加しCargo.lockへ4依存が増えたため、依存監査は最終検査で再実行する。ここまでの成功をU2やStop全体の完了と扱わない。


### Stop resetまでの検証済み区切り（2026-09-09）

park後のcounterが1のまま残るRedから、署名なしのreset要求を単一イベントとして保存・投影し、空署名/count0へ戻すGreenを確認した。Stop/health/dropが使用済み共有DBの欠落を空DB作成で埋める不具合もRed→Greenで解消。`prepare_shared_store`で承認初期化と同じ検査を通す。

HookHealthの壊れた通番を再生してもpanicしないRedを確認し、replay/applyを正式な歴史再生へ是正して7件成功。HookDropSummaryの空/追加の構築も検査付きnewへ集約した。Continuationの不正な初回count9を受理するRedはconstructorで拒否し、関連7件がGreen。固定本家の署名比較を加えて現在8件である。

固定本家計測は11入力へ広げ、progressSignature/continuationReason/blockStopの本文は無変更、export追加だけで採取。`run-reverse-engineering`のstdout全文も公開CLIと一致させた。新しいコード生成計画承認の保全検査は最初から成功し、偽のRedを作っていない。実際のStopフックを通っても計画承認ファイル・active directive・Sourceイベント数は変わらなかった。

cargo lintは集約内のCheckboxState変種列挙を検出したため、分類をCheckboxState::is_waiting_for_humanへ移し、抑止せず解消。最終Clippy/lint/fmt/diff成功。Stop CLI7件、health等10件、domain7+8件、app単体287件、既存DAO34件が成功。全量ログと現在の残作業はstop-checkpoint.md/jsonを参照する。これはU2完了やStop全ケース完了の記録ではない。


## 2026-09-09 Step 6 再開: 人間待ち・セッション・IO復旧

承認済み計画とTesting Contractを維持し、通常ClaudeのBrownfield bugfixに必要なStop境界を実装した。前回のArtifactSaved通常RMU統合とnext入力Greenはそのまま保持する。工程next/report/pause/reissueは実行していない。

- 未回答の現段階文書、待機後の再入counter、payload session binding、会話判定、stateに束縛したshared resumeはそれぞれRed→Greenを保存した。関数部品の原観測87/127は親担当のstop-transcript-verification.md/stop-resume-wait-verification.mdを参照。
- resume lock競合は旧5秒待ちをRedで検出し、本家相当の1秒待ち・固定理由・silent allowへ修正。handoffは正確な新intent/session組を一度だけ消費してresetする。
- 実next timeoutは最初のstage-graph FIFOが照会の停止地点にならなかったため、失敗したfixture実験として残した。memory/org.md FIFOで10秒の実IO待ち→silent allowを確認した。これを未接続機能のRedとして数えていない。
- SQLite保存失敗と結果投影失敗を、counter・要求ID結果・再開の公開/実DB境界で検査。最初の保存イベントがcount9を偽称できる不具合はRed→Green。ArtifactAuditのreplay是正は親担当artifact-replay-verification.mdに記録。
- 本家counter-directory実測は2回ともblockだったがnativeは2回ともsilent。この差を受け、停止判断と公開成否を分けた。未確定の選択を次counterに使えない契約のRed、公開失敗時の選択結果欠落のRedを保存。後者の最初の検査は異なるdirective全文も比べていたため、Green検査では実フック原観測のblock/exit/stderrと、同じrun-stage材料のsource関数描画全文を明示して別々に照合する。
- RMUは公開IOの実際の成否を要求ID行へ投影し、Controller→確定UseCase→集約がそのIDと自身の未確定操作を照合する。成功/失敗の確定はそれぞれ単一イベント。公開確定保存失敗の回収を新しいcounter読取りより先に行い、過去の失敗を再試行してcountを進めないことを実CLIで確認。古い要求の失敗で後続guardを巻き戻すことも拒否する。
- 正常公開→counter directory置換→失敗2回→解除後count1は、追加された固定本家13観測のBrownfield系列を使って確認。状態/rulesが異なるsource load-steeringとnative run-stageの全stdout一致とは扱っていない。
- 同時Stop、現段階/別段階のログ質問と回答後の復帰は追加検証。回答の初回fixtureはUserPromptSubmitを欠き、既存の真正応答保護で拒否された。正規の人間応答を追加してGreenとし、保護を弱めていない。
- 完全なsnapshotで初回から公開確定済みを偽称する不具合はRed→Green。resetとwait-probeを同時に作れないResult契約も追加。後者の初回コンパイル失敗は新しい拒否API不在によるもので、ビルド環境障害ではない。
- Repositoryの公式memory入口不在を契約テストで検出し、SQLite/公式Memoryが同じgeneric Repository本体を使う形へ是正。共通関数でNotFound、保存/再読込み、公開失敗確定、楽観競合を両方に課して成功。
- 定期cargo lintで公開KeyDtoとRepository同居のone-public-type違反を実際に検出し、KeyDtoを専用ファイルへ分離してから後続検査へ進んだ。Clippyの余分なclone・const指摘も是正。検査ログには失敗と修正後の実行を両方保持する。

最新の適用表・正確な件数・残るStep 7責任はstep6-verification.mdへ集約する。個別の実装完了をU2の全品質ゲート、C1全文互換、U4配布接続、Claude実地完了の代替にはしない。


### Step 6返却直前のRepository追検査

HookHealth/ArtifactAudit担当の実測を受け、Continuationの同型の不足も追検査した。SQLite/Memory双方で競合actualが0固定の失敗、別IDの有効snapshotを返す失敗を実行確認し、実在版の診断と要求ID照合でGreenへした。停止の公開確定保存失敗からの復旧も修正後に再検査して成功した。Memory API不在のコンパイル確認は動作Redに数えず、これらの実行Redと区別する。step6-verification.mdに最終コマンドを記録し、定期lint、対象all-targets Clippy、workspace fmt/diffはすべて成功。Step6の必須未完0件としてチェックのみ更新した。U2の未完工程を完了へ変更していない。
