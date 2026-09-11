# Step 6 の適用範囲と検証（必須経路の検証完了）

2026-09-09。対象は通常Claudeで行う本リポジトリのBrownfield bugfix（9段、Unitなし）。承認済みTesting Contract `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55` を維持する。U2全体の完了を意味しない。以前のstop-checkpoint.mdの未完一覧はこの記録で更新する。

## 適用表

| 境界 | 今回の扱いと証拠 |
| --- | --- |
| Stopの差止め・上限・再入・進捗変更・reset | 実CLI、domain、固定本家の11署名/文言観測で検査。SQLite→RMU→要求ID Queryを使う。 |
| 開いた承認ゲート・改訂待ち | IntentExecutionの最新状態が判断。停止照会で計画受領・active directive・指示発行の事実を変更しない。 |
| 現段階の未回答文書 | active stage直下の質問だけを観測。回答後/別段階を区別し、待機中はcounterを変更しない。 |
| 現段階のログ質問 | 別段階のログ質問→差止め、現段階の質問→待機、真正なUserPromptSubmit→answer→反復再開を実CLIで検査。 |
| 会話 | StopTranscriptの固定87入力一致と公開Stop結合。履歴中のengine実行有無を区別。履歴なしの人間/engine marker時刻を結合。 |
| shared resume待ち | StopResumeWaitの固定127入力一致。対象stateのhash/所有者、ロック付き読取り、busy時の停止許可、stale markerの通常処理復帰を実CLIで検査。 |
| payload sessionのintent選択・fresh handoff | shared cursorを動かさずpayload bindingを選ぶ。新規intentへの正確なhandoffを一度だけ消費。producerのsession hook接続はStep 7。 |
| nextの実タイムアウト | memory/org.mdのFIFOで実native nextを停止させ、10秒期限でsilent allow、counter非生成を確認。stage-graph FIFOの初回試験は停止地点にならず、失敗したfixture試行として保存。 |
| SQLite・投影・公開確定失敗 | 要求IDで選択と公開成否を分離。未確定操作がある間は新規判断を拒否。復旧は公開IO観測の確定を先に行い、その後に新しいcounter読取りを行う。 |
| counterの書込不能 | 本家の実フックは2回ともblock。nativeも選択結果を保持し、公開失敗は別イベントで確定。DB障害時はQueryを迂回せずsilent allow。 |
| 同時Stop | 公開前観測→選択→公開→確定を同じ排他内に置き、同時2要求でblock1/release1/counter2を実プロセスで検査。 |
| 人間応答記録 | 元入力（番号、数値型、順序、不正UTF-8、匿名、unattended）と実セッションを保持し、受領の再利用/別対象/保存後再開を公開契約で検査。 |
| 状態遷移保護 | U1固定4境界のstdout/stderr/exit全文、追加shell/不正封筒/UTF-8、非適用時の不要な初期化なしを検査。 |
| ファイル保存監査 | created/updated/無関係/欠落/drop/heartbeatを通常のArtifactSavedイベント・共通RMU経路で検査。EISDIR診断は人間裁定済みの1ケースだけBun固有詳細を別扱い。 |
| usage flush | Step 7が有効経路の必要性と副作用を分類するという承認済み責任分担。現在のStop検査を有効usage経路の完了へ拡大しない。U4の配布接続も未実施。 |
| compose・background ledger・autonomous park・unit-major例外 | 選定された通常Claude bugfix経路ではproducer/実行指示がない。任意入力全般に対応済みとはしない。Codex自律swarmのbackground実行や他ハーネスを新規移植する要求へ読み替えない。 |

## 公開IO観測の責任

停止の更新UseCaseはunitだけを返す。集約は要求ID、進捗、今回読み取ったcounterの観測、未確定操作を所有して選択する。RMUは選択を再判定せず、counterを書き出した実際の成否を要求ID行へ投影する。Controllerがその観測をID付きの値オブジェクトとして確定UseCaseへ渡し、集約が対象・順序・再利用を照合して単一の公開成功/失敗イベントを保存する。その後のRMUと元の要求ID Queryだけが表示結果を返す。古い失敗結果で新しいcounterを巻き戻さない。

今回のcounter読取り対象は本家/nativeが生成する正準counterと、欠落・破損JSON・読取不能である。第三者が正準形外の数値（負数、小数、無限大相当）を手書きしたcounterの全挙動は検証していない。

## 比較の範囲

stop-publication.jsonは固定本家の実フック13観測を保持する。Greenfield/Brownfieldで正常公開後directory化→2回書込失敗→障害解除後のcount1まで含む。sourceはload-steeringを返し、nativeの既存fixtureはrun-stageを返すため、その2つのstdoutをそのまま全文一致と呼ばない。書込障害時のblock/exit/stderrを実フック原観測と比べ、run-stageの文言全文は同じ材料で採ったstop-values.jsonの原関数出力と比較する。計画承認の比較範囲・C1の不足はU2 Step 4/8の受入作業に残る。

## 検査記録

| コマンド | 実測 |
| --- | --- |
| `cargo test -p aidlc --test upstream_271_contract --test claude_hook_contract` | 74件＋10件成功。後から追加したcounter置換検査は次のStop最終実行に含む。 |
| `cargo test -p aidlc --test upstream_271_contract stop_` | 最新22件成功、53件はフィルター対象外。公開成否/復旧/同時実行/ログ質問/counter置換を含む。 |
| `cargo test -p core-command-domain --test workflow_continuation_contract` | 最新14件成功。初回snapshotの偽公開済み状態・reset/probe拒否も含む。 |
| `cargo test -p core-command-domain --test hook_health_contract --test artifact_audit_contract` | 各7件成功。 |
| `cargo test -p core-command-interface-adapter --test workflow_continuation_repository_contract` | 共通公開契約2件成功。最後の追検査で差分履歴/foreign snapshotの共通2件も成功。 |
| `cargo test -p harness-claude` | unit5件、integration各1件×3成功。integrationの固定観測はhuman object orderとStopTranscript87/StopResumeWait127。 |
| `cargo clippy -p aidlc -p harness-claude -p core-command-domain -p core-command-interface-adapter -p core-command-use-case -p core-read-model-updater -p core-query-interface-adapter --all-targets -- -D warnings` | 成功。 |
| `cargo lint` | 各実装区切りで実行。最後の実装区切りも成功。one-public-type所見を是正したRed/Greenを保持。 |
| `cargo fmt --all --check` / `git diff --check` | 両方成功。 |

親の独立検証は公開失敗時の判断維持、確定保存失敗後の回収、同時Stopの3件が各1件・終了0。上記全target成功とは別の検証であり、test-evidence/stop-publication-independent.logへ保存する。生ログはtest-evidence/step6-*.log、stop-*.log。前提不足のfixture実験や静的検査失敗は成功ログと別名で残した。

HookHealth/ArtifactAuditの公式memory共通契約も完了。公開契約6件、両backendの履歴4件とmemoryエラー2件を含むRepository実装回帰47件、app plan_runtime5件、対象Clippy/fmt/diff/最終cargo lintが成功（hook-repository-backends-verification.md）。今回の必須Step 6境界の未完は0件となり、計画のStep 6チェックだけを完了へ更新する。U2のworkspace全体、90%床/相対ゲート、Quint/ITF、CI、独立レビューは後続であり、ここでは成功したとは記録しない。

親担当のStopTranscript、StopResumeWait、ArtifactAudit再構成の証跡は、それぞれstop-transcript-verification.md、stop-resume-wait-verification.md、artifact-replay-verification.md。HookHealth/ArtifactAuditのSQLite/公式memory共通契約はhook-repository-backends-verification.mdに記録済み。


## 完了条件と次工程

Step 6で必要な通常Claudeの停止・真正応答記録・遷移保護・保存監査の未完了境界は0件。対象コードと原観測・検査ログの断面はstep6-source-checkpoint.jsonへSHA-256付きで記録した。共有ファイルには先行U2作業と他担当の変更も含む。このファイルはU2全体の最終source-manifestではない。

承認済み計画のStep 7へ進み、session-start/end、subagent、規則受渡し、freeze/scope/TaskUpdate/runtime-graph、PreCompact、learnings persist、診断記録、usageの有効経路を実装・分類する。Step 4/5/8/9、C1未一致の投影、センサー監査3種類の未裁定、全品質ゲートとU4配布接続は残る。現工程の計画本文・Testing Contractは変更していない。


## Repository境界の最終追検査

他担当のHookHealth/ArtifactAudit検査で見つかった2つの問題をContinuationでも検証し、両backendで動作Red→Greenを確認した。競合のactualは固定0から実際のsnapshot versionへ是正し、復号したsnapshotの集約IDが要求IDと異なるときはCorruptで拒否する。診断のための版再読取りを更新判断へ使わない。

- `cargo test -p core-command-interface-adapter --test workflow_continuation_repository_contract --lib continuation`: 共通公開契約2件＋差分履歴/foreign snapshotの共通契約2件が成功。
- `cargo test -p aidlc --test upstream_271_contract stop_recovers_failed_publication_before_observing_the_next_counter`: 最後のRepository修正後も成功。
- `cargo clippy -p core-command-interface-adapter --all-targets -- -D warnings`、定期`cargo lint`、workspace fmtとdiff check: すべて成功。

この追検査中はStep 6チェックを未完へ戻し、是正・検証終了後に再び完了へ更新した。現時点の必須未完は0件。memory API不在のコンパイルログと、この2件の動作Redは区別する。
