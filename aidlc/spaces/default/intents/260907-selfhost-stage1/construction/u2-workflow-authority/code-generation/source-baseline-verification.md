# U2の開始時Source Baseline検証

## 対象と結論

承認済み計画Step 4・8のうち、**開始時のSource Baseline**を実装した。固定本家2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` のソース一覧、一覧のSHA-256、公開TSV、WORKFLOW_STARTEDの監査欄を比較した。通常のSQLite保存とRMU公開・再開を通す。Step 4全体・U2全体の完了を意味しない。

対象契約は[contract-summary](../../../inception/contract-design/contract-summary.md)のC1・C4、要求FR1–FR4、[承認済み計画](code-generation-plan.md)と[テスト手順](unit-test-instructions.md)。次工程へのルーティング・承認・unit pause・コミット・pushは操作していない。

## 実装と責任

- `SourceBaseline` は採取済み一覧または採取不能の値。TSVの行形式、エスケープ、UTF-16順、重複を構築時に検査する。永続化属性を持たない。ハッシュは承認用ソース指紋ではなく、一覧TSVそのもののSHA-256。
- `StartRequest` が開始基準を運び、既存のCreated・Intentへ保持する。書込DTOとRMU読込DTOは各側が所有し、検査付きコンストラクタを通して再読込する。更新UseCaseの成功戻り値と既存SQLite保存方式は変えない。
- 開始CLIの入力境界で実ファイルを採取する。Git有りで採取不能なら `unbindable`、Git無しで採取不能なら空一覧とする。本家のこの区別を成功した採取と混同しない。
- WORKFLOW_STARTEDへSource Baselineを描き、TSVは同じRMUのPublicationBatchへ含める。保存先は本家開始CLIと同じ `.aidlc-source-review/code-generation/baseline-<SHA先頭12>.tsv`。同一内容は再利用し、既存の異内容は上書きしない。
- 通常の公開計画の対象束縛を維持し、可変個数のTSVだけを内容とアドレスで照合する。復旧時も、開始基準の内容を利用者が変更した状態へrebasingしない。状態・規則等の通常ファイルに対する既存の復旧契約33件は維持した。
- 独立した監査writer・別RMU・別storeは増やしていない。`source_fingerprint.rs` の共有補助関数3本の可視性だけを広げ、承認用指紋の出力を変更していない。

## 固定本家の追加採取

`scripts/goldens/capture-source-baseline.ts` は既存の `verifySource` で固定配布束を照合し、`tests/golden/selfhost-stage1/source-baseline.json` へ追加採取する。U1コーパスと既存採取スクリプトは変更していない。

12ケースは空一覧、通常ファイルとshell除外、UTF-16順・4種類のescape・実行属性、内部/欠損/循環リンク、外部リンクのソースとテキスト選別、登録されたconditionalディレクトリ、センサーキャッシュの正確なパス境界、manifest無しのharness登録拒否、不正registry、未作成の登録パス、正常なembedded Gitとunborn Git。各ケースはTSV全文を比較し、値の正規化は行わない。

同じコーパスの `start_observation` は配布束を一時ワークスペースへコピーして**本家の開始CLI自体**を実行した観測。Rustの `start_publishes_the_listing_and_its_exact_hash` は同条件のソース、引数、配布グラフで、終了コード・stderr・Source Baseline欄・スナップショット名・TSV全文を比較する。このテスト単独で開始stdout・全監査・全状態の完全互換まで証明したとはしない。初期監査16行は別の既存ゴールデンで全文比較する。

## Red・Greenと回帰検証

| 対象 | Redと是正 | Green・ログ |
| --- | --- | --- |
| 開始監査16行 | Source Baseline欄だけ欠落 | [Red](source-baseline-logs/red.log) → [Green](source-baseline-logs/green.log)、1件 |
| ソース採取 | 空一覧では通常ファイルを保持できない | [Red](source-baseline-logs/scan-red.log) → [Green](source-baseline-logs/scan-green.log) |
| CLI公開 | 作成成功後のTSV不在 | [Red](source-baseline-logs/cli-red.log) → [最終](source-baseline-logs/cli-final.log)、3件 |
| 改変基準の復旧拒否 | 他ファイル復旧と同時に改変TSVを保存していた | [Red](source-baseline-logs/publication-red.log) → [Green](source-baseline-logs/publication-green.log) |
| 未完公開計画の解決 | TSVの追記を通常ファイルの編集として受理していた | [Red](source-baseline-logs/rebase-red.log) → [Green](source-baseline-logs/rebase-green.log)、公開契約4件 |
| 除外境界 | センサー名だけで除外し、manifest無しharnessへの登録を受理 | [cache Red](source-baseline-logs/boundary-red.log)、[registry Red](source-baseline-logs/registry-red.log) → [Green](source-baseline-logs/boundary-green.log) |
| 既存公開復旧 | 通常ファイルの復旧・競合・再開 | [33件+開始基準4件](source-baseline-logs/recovery-regression.log) |
| 値モデル | コーパス復号・hash・不正一覧の拒否 | [2件](source-baseline-logs/domain.log) |
| 指紋回帰 | 既存承認用指紋の固定観測を維持 | [3件](source-baseline-logs/source-regression.log)、うち2件がソース採取・指紋 |
| 静的検査 | 自分の新規テストのunused-self・indexingと採取コードのcollapsible-ifを修正 | [app側clippy](source-baseline-logs/clippy-own.log)、[core側clippy](source-baseline-logs/clippy-core.log)、`-D warnings` |

`cargo lint` は各実装・是正の区切りで7回成功し、最後の直接実行も終了0を確認した。fmt・clippyでは代用していない。他担当の変更中に全target clippyが止まった指摘は当該担当へ連絡し、この返却では自分の対象targetの成功と区別する。workspace全体の90%床・相対0.01・全CIは親担当のStep 9で実施し、今回の限定検証だけで達成済みとはしない。

## 実リポジトリの照合

最終採取コードでも、この作業ツリーを変更せず本家 `workspaceSourceState` → `serializeSourceListing` とRust `source_baseline::read` で順に走査した。**8,954行、1,514,855バイトが全文一致**した。観測時点のTSV SHA-256は `cab437b9c417ae5d68f7d748bfd4863f0f41a225e6c7bd42dc3a909f9284f477`。

これは観測時点のファイル一覧への値であり、後続の編集後に同じハッシュを要求しない。先行観測の8,952行から行数が変わったこともソース更新による。ログは[最終比較](source-baseline-logs/live-final.log)と[メタデータ](source-baseline-logs/live-metadata.json)。一時診断テストは削除済みで、製品の診断入口や公開APIは追加していない。

対応する採取本体は `modules/app/aidlc/src/source_baseline.rs` の先頭から最初の `#[cfg(test)]` 直前まで。そのUTF-8バイトSHA-256は `681154bf9942c84d9112a6f68f1d81d86c875af0b07a0298cfa8e495cc7df7f4`。診断の追加前と削除後でこの値が等しいことを確認し、比較に用いた採取本体と最終コードを対応付けた。診断コードの除去自体でワークスペースTSVの値は変わる。

## 変更ファイル

- `modules/app/aidlc/src/source_baseline.rs`、`source_fingerprint.rs`、`lib.rs`、`runtime.rs` のmint_intentのみ
- `modules/core/command/domain/src/orchestration/source_baseline.rs`、`source_baseline_error.rs`、`start_request.rs`、`intent.rs`、`mod.rs`
- `modules/core/command/interface-adapter/src/orchestration/dto/intent_dto.rs`
- `modules/core/read-model-updater/src/orchestration/dto/intent_dto.rs`、`read_model_updater.rs`、`projection_targets.rs`、`publication_batch.rs`、`journal_reader_impl.rs` の基準ファイルの復旧検査のみ
- `modules/core/read-model-updater/src/workspace/resolved_plan.rs`、`projection.rs` の開始監査のみ
- `modules/app/aidlc/tests/source_baseline_contract.rs`
- `modules/core/read-model-updater/tests/source_baseline_publication_contract.rs`、`projection_golden_test.rs` の開始材料
- 上記専用採取スクリプト・追加コーパス・本記録

## 適用範囲と残る境界

開始時の単一リポジトリが対象。advance、jump、別段階の開始基準、複数リポジトリのroofは未接続。この実装だけで全境界のSource Baseline対応完了とはしない。

AI-DLC自身の `.aidlc/worktree-meta.json` がある作業領域は、worktree固有の除外文脈をまだ接続していないため採取不能として扱う。現在のOrca Git worktreeにはこのファイルがなく、Git worktreeそのものとこの制約は異なる。登録パスが外部リンクを経由して内部へ戻る経路、詳細なsymlink解決上限、Gitメタデータの異常全種の同等性は今回未検証。正常なembedded Gitは実GitコマンドでHEADを確認するため、Git実行ファイル無しのembedded Git正常採取は本家のライブラリ内実装との差として残る。

この差を現在の正常経路へ読み替えたり、未採取ケースを成功扱いしたりしない。親担当がStep 4・8全体の対象と照合する。

## 親担当の独立実行対象

```sh
cargo test -p core-command-domain --lib source_baseline
cargo test -p aidlc --lib source_
cargo test -p aidlc --test source_baseline_contract
cargo test -p core-read-model-updater --test source_baseline_publication_contract --test publication_recovery_contract
cargo test -p core-read-model-updater --test projection_golden_test the_genesis_draws_all_sixteen_initialization_rows
cargo lint
```

推奨はこの限定対象を独立確認して現計画の残作業へ統合すること。対象外の境界を次に接続する場合は、固定本家の追加観測を先に採る。
