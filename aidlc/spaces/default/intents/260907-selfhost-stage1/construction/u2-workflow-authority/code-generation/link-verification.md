# log linkの接続と検証

## 今回閉じた範囲

承認済みU2計画Step 5の `aidlc-log link` を、通常Claude・未登録の単一root repositoryで接続した。固定本家2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `aidlc-log.ts:820–1001` を基準に、入力、宣言されたlinkの順序、重複、引継ぎファイル、試行境界、保存、監査、公開結果を検査した。

**linkコマンドの接続完了であり、Step 5・U2全体の完了ではない。** 次の指示へ完了linkを表示する処理、pipeline完了ガード、レビュー内容のfingerprint結合等の残件は後半に列挙した。

対象は[承認済み計画](code-generation-plan.md)Step 5、[テスト手順](unit-test-instructions.md)、[契約C2/C4](../../../inception/contract-design/contract-summary.md)。DDDの値・集約・Repository配置のスキル正典は同セッションで読了済みで、coding-rulesの完全コンストラクタ、setter禁止、UseCaseからdomain getter禁止、1公開型1ファイル、単一イベント、成功unitを適用した。

## 保存と判断の責任

- `LinkArgs` は本家parseFlagsの値・空値・省略を保持する。未知の値付きflagは本家同様に拒否しない。`--single` / `--retry-pending` / `--stage-level` は値無しflag。
- ファイル入力境界が期待パスと指定パス、通常ファイル性、symlink、内容、mtimeを観測する。UnixではO_NOFOLLOW/O_NONBLOCKで開き、読込前後のmetadata変化も検査する。観測結果を渡すだけで順序や重複を決めない。
- `IntentExecution::record_pipeline_link` が定義のmode・lead/support、受領履歴、試行、ファイル鮮度を判断する。`PipelineHistory` は境界と受領を時系列で保持する一級コレクション。履歴の要素は `PipelineRecord` というドメイン型。
- `PipelineLinkCompleted` の単一イベントを、既存IntentExecutionRepositoryのSQLiteへ保存する。新しいRepository・監査writer・storeは作らない。成功UseCaseは `Result<(), PipelineLinkCommandError>` で、表示材料を返さない。
- 通常RMUが `PIPELINE_LINK_COMPLETED` のStage / Link / Positionと条件付きartifact・repo・Workflow欄を描く。成功JSONに要るstage/linkは呼出入力の値であり、現在状態をQueryや出力側で再判定しない。
- 失敗は親担当が同時に実装した共通ERROR_LOGGED経路へraw診断を渡す。JSON error化と失敗監査は共通処理が行い、この機能専用の監査writerは作らない。[共通処理の検証](error-log-verification.md)と合わせ、今回の拒否監査全文も本家へ一致した。

## 引継ぎと試行の契約

reverse-engineeringのdeveloper linkは、`<record>/inception/reverse-engineering/developer-scan.md` を要求する。別パス・不存在・directory・symlinkは拒否する。内容のSHA-256とmtimeを受領へ保存し、後段linkの受理でも現在のファイルと比較する。mtime差の許容は本家の0.01ms。

同じ試行の既存受領は重複として拒否する。引継ぎの内容やmtimeが変わると、そのdeveloper以後の連鎖を有効な受領として使わない。再記録は過去の同じlinkのmtimeより新しく、現在試行の開始時刻以後であることを要する。

通常実行の開始・jump・当該stageのreject・前段完了によるstage開始、単独実行の開始境界を履歴へ保持する。単独実行と通常実行の受領は混ぜない。既存の単独実行テストは、新しい独立試行履歴を**明示的な期待値として加えた集約全体**で比較し、本流のcursor・status・checkbox・承認等の不変を維持した。新しい履歴フィールドを比較から除外して通していない。

## 本家の採取と実行結果

新規 `scripts/goldens/capture-pipeline-link.ts` が配布束を `verifySource` で固定commitへ照合し、一時workspaceでのみ実行する。採取先は `tests/golden/selfhost-stage1/pipeline-link.json`。既存U1コーパス・採取スクリプトは変更していない。

27プロセス観測は、必須flag・不正構文・selector・cold状態、非pipeline・未知stage/link・repo指定、順序、artifact要求・場所・不存在・古さ・singleの試行不在、developer/architectの成功と重複、内容変更・書直し、directory・symlink、空値・無視するflagを含む。

stdout・stderr・監査追加全文・state変更の有無を比較する。正規化は選択recordとworkspaceのパス、監査のISO時刻だけ。handoffのmtimeは明示した固定入力であり比較から除外しない。成功・失敗とも状態ファイルの内容が動かないことも本家に一致した。

| 検査 | 結果・証跡 |
| --- | --- |
| 接続前の公開CLI反例 | [Red](link-logs/cli-red.log): 本家のMissing stage JSONに対してnativeはnot-wired本文。コンパイル失敗はRedに数えていない |
| 固定本家27観測 | [Green](link-logs/cli-golden-green.log)。単一テスト内で全観測を比較するため、テスト件数は1件 |
| 試行の再開 | jump/reject後に以前のarchitectを拒否し、同mtimeのdeveloper再記録も拒否。書直し後は受理 |
| 単独実行の分離 | 保存した独立試行境界の前後で通常/単独の連鎖を混同しない。合成境界は一時workspace内だけ |
| 同時重複 | 2プロセスから同じdeveloper完了を記録し、成功1・受領1になる |
| 公開失敗と復旧 | 監査公開をdirectory衝突で失敗させ、修復後に保存済みdeveloperからarchitectへ続行。受領の重複なし |
| 上記追加契約の最終結果 | [5件成功](link-logs/final-contract.log)。そのうち1件が本家27観測、残り4件が再開・並行・公開の契約 |
| Domain回帰 | [718件成功](link-logs/domain-regression.log)。完全コンストラクタの新しい履歴引数も復元元から渡して同値を保つ |
| event列挙検査 | 新しいlinkと、以前fixtureから漏れていたPlanAnswerLoggedを加え、この時点の全24変種を検査 |
| build | [cargo check -p aidlc成功](link-logs/production-check.log) |
| 静的検査 | 親担当の共通全target Clippy成功。自分のmanual-let-elseを修正済み。fmt・git diff --check成功 |
| tools/lint | 実装・完全コンストラクタ修正・追加検査の区切りでcargo lintを実行し、最終回も終了0 |

構築中に共通enum/constructorの追加によるコンパイル不足が生じたが、実行Redとは区別した。大規模な共通変更をコンパイル可能な区切りへ戻して親・別担当へ通知した。Domain回帰で見つかった復元テストの履歴引数不足等は、期待を減らさず新しい状態を明示して修正した。

## 変更ファイル

- `modules/app/aidlc/src/cli/link_args.rs`、`cli/request.rs`、`cli/mod.rs`
- `modules/app/aidlc/src/runtime/pipeline_link.rs`、`runtime.rs` のLogLink armとmodule宣言
- `modules/app/aidlc/src/wording.rs` の不要になったnot-wired link文言の削除
- `modules/app/aidlc/Cargo.toml` と `Cargo.lock`（no-follow openに、既存workspaceでも使われるlibcをappへ追加。新しいlibc版の導入ではない）
- `modules/core/command/domain/src/orchestration/pipeline_{handoff,handoff_input,history,link_error,link_request,receipt,record}.rs`
- 同 `intent_execution_event/pipeline_link_completed.rs`、`intent_execution_event.rs`、`intent_execution.rs`、`mod.rs` の該当部分
- `modules/core/command/use-case/src/orchestration/record_pipeline_link_use_case.rs`、`pipeline_link_command_error.rs`、`mod.rs`
- `modules/core/command/interface-adapter/src/orchestration/dto/pipeline_link_completed_dto.rs`、`pipeline_record_dto.rs`、`intent_execution_dto.rs`、`intent_execution_event_dto.rs`、`tests.rs` の完全コンストラクタ引数、DTOとorchestrationの `mod.rs`
- `modules/core/read-model-updater/src/orchestration/dto/pipeline_link_completed_dto.rs`、`intent_execution_event_dto.rs`、DTOとorchestrationの `mod.rs`
- `modules/core/read-model-updater/src/workspace/projection.rs` のPipelineLinkCompleted投影arm
- `modules/app/aidlc/tests/pipeline_link_contract.rs`、上記専用採取・コーパス・本記録

共有match/mod.rsと親のCommandFailed・他担当のsession hooksを保持した。Stop/health/continuationの判断は変更していない。engine next/report/pause、実作業のagent/link完了記録、commit/push、外部投稿は行っていない。

## Step 5の残る不足の読み取り棚卸し

以下はこのタスクで一括実装していない。確認済みのコード上の不足と、今回再検証していない面を区別する。

| 優先 | 境界 | 読み取りで確認した現在地・次の作業 |
| --- | --- | --- |
| 高 | pipelineを含むnextの再開表示 | `modules/app/aidlc/src/directive_drawing.rs` はPipelineDirectiveのcompletedへ `Vec::new()` を渡す。本家orchestrateのrun-stage組立は `pipelineLinkEvidence.completed` を使う。今回保存したPipelineHistoryを通常RMUの構造化面へ反映し、Queryで読む接続が必要 |
| 高 | pipeline段階の完了/承認ガード | 本家state `verifyPipelineLinkPrecondition` とorchestrate `checkPipelineLinkEvidence` は現試行の全linkを要求する。nativeのreport/approve側は今回のPipelineHistoryをまだ消費していない。引継ぎが未記録・変更された状態の完了拒否をTDDで接続する必要がある |
| 高 | review受領の内容・ソース結合 | `ReviewRequested` はstage/reviewer/iteration/retry、`ReviewCompleted` はstage/reviewer/iteration/verdictだけ。後者のdocもArtifact/Source Fingerprint繰延を明記している。本家2.7.1のartifact fingerprint、request fingerprint、review appendixの境界/digest/challenge、workspace source v2結合、変更後の失効を接続する必要がある |
| 高 | review完了ガード | `IntentExecution::require_review_receipt` はReviewAttemptのterminalだけを見る。上の内容・ソースへの結合が無いため、ファイル変更後にも古いterminalだけで通さない検査が必要 |
| 高 | 承認待ちstageへの再報告 | 親担当が固定本家との追加比較で、既に承認待ちでも根拠を再検証する2.7.1契約との差を確認した。nativeの既存AlreadyAwaiting no-opを文言だけ変えて済ませない。親担当の所有事項 |
| 中・scope確認 | 通常decision/answerのunit/single | `runtime::log_decision` / `log_answer` は通常・summary経路でunit/singleをnot-wired拒否する。plan-approval専用分岐はこの拒否より前に処理するため、計画承認のunit対応まで未実装と断定しない |
| 中・scope確認 | reviewのunit/single | `runtime::log_review` はunit/singleを明示的にnot-wired拒否する。必要な実行範囲を確定して専用の受領・失効・再開を接続する |
| 中 | 通常interactionのflag解析 | `InteractionArgs` の値欠落診断とboolean集合は専用LinkArgs/本家parseFlagsと一致していない箇所がある。今回linkは固定本家に揃えたが、既存decision/answerの全flag組合せは再採取・再検証していない |
| scope外を未達と分離 | repo識別付きlink | 現Intent/開始要求に登録repo情報がないため、通常rootの `--repo` 拒否を実装した。登録repoを持つintentの必須repo/既知repo照合・独立連鎖は未接続。未登録rootの成功で代用しない |
| 実装あり・維持 | 計画承認 | 専用分岐はPlanTarget(unit/stage-level)、session、active directive、source floor、文書、challenge/真正な回答、操作IDによる投影/取得を実装している。今回そこを変更・弱化せず、未実装と誤分類しない。残る全2.7.1差の検証完了は本taskでは主張しない |
| 親担当で閉鎖 | ERROR_LOGGED共通経路 | 今回のlink拒否audit全文も一致し、以前の未投影状態は解消済み |

単独実行のlink受領の分離は検証したが、`next --single`→実agent→handoff→reportの全体を本taskだけで完成としない。複数repoやリンク列の再利用受領、他ハーネスの一般化も今回の完了範囲へ含めていない。

workspaceカバレッジ90%床・相対0.01・全CI・実Claude一周は親担当の統合検証へ残る。既存の承認やsource-v2の条件を下げていない。

## 独立実行コマンド

```sh
cargo test -p aidlc --test pipeline_link_contract
cargo test -p core-command-domain --lib
cargo lint
```

推奨はこの限定検証を独立実行してlinkコマンドを受領し、現計画の上記残件へ進むこと。Validation Basisを先に扱う場合も、pipeline/レビュー受領の消費側が未接続である点を残量管理へ保持する。
