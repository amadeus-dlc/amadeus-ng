# U4 公開中断からの復旧 — 実装・検証記録

初回検証日: 2026-09-05。実装同期日: 2026-09-06 JST。作業ツリー上の実装・検証記録であり、ステージ承認やintent全体の完了を表すものではない。直近のCLIと取得処理の変更を合わせた全workspaceの検証件数と、最新main上の相対カバレッジは未測定。

## 解消した問題

従来の取得処理は、ファイルを書いた後にチェックポイントを進めていた。その間で停止すると、再開時に同じ監査イベントを追記した。先行の再現では監査イベント数が2件から4件へ増え、状態ファイルのバイト列だけは変わらなかった。

書込前後のバイト列、要求ID、世代、対象パスの束縛、変換規約の版、履歴の終点をSQLiteへ先に保存するよう変更した。ファイル反映後に、構造化リードモデル・チェックポイント・完了記録を同じトランザクションで確定する。再開時は保存済み計画の反映済み部分を照合し、未反映部分だけを完了する。

## 実装した動作

- 監査の全反映・部分追記・UTF-8文字途中の中断から、同じバイト列へ復旧する。
- 未完了計画の保存後にイベントが増えた場合、元の終点までを先に確定し、同じ`catch_up`呼出しで追加分を別計画・別Txとして処理する。最大2計画で戻り、runtimeに駆動ループを置かない。後続計画が失敗しても、先行計画のcommitは保持する。
- 出力先ごとに最後の確定済み計画を保持する。Aの公開後にBを公開しても、Aの欠落ファイルを復元でき、Bと進んだチェックポイントを保持する。
- 存在する利用者編集を保持する。削除された利用者所有のmemory原本は復活させない。
- 未完了計画との競合は明示的な解決要求で新世代へ置換する。対応を証明できない本文編集は競合として残す。
- 共有リードモデルの公開位置・世代・内容ダイジェストを保存する。古い候補で新しい面を巻き戻さず、同位置の内容不一致や既存面の破損を拒否する。
- 保存計画の破損、異なる変換規約の未完了計画、置換済み要求の再送を拒否する。
- CLIの初回構造化投影とファイル投影のチェックポイントを分け、起動時に欠落ファイルの復元を呼び出す。
- `catch_up_before_reading`は復旧失敗を握りつぶさない。`next` / `resume`はerror directive・exit 0で通常の指示を止める。`report` / `practices_promote` / `set_autonomy`はrefused・exit 1で、後続の集約操作・イベント追加の前に拒否する。

集約の再構成は、利用者が指定した「最新スナップショットと、それより後の差分イベント」で行う。ここで扱う構造化リードモデルの再計算は別の処理であり、指定した終点までのジャーナルを入力とする。

## 2026-09-05時点の検証履歴

以下の件数・数値・レビュー結果は、直近の同呼出し内の追加処理とCLIの失敗伝播を加える前の記録である。最新結果としては扱わない。

| 検証 | 結果 | 根拠 |
|---|---|---|
| 実SQLiteでファイル反映後のチェックポイント確定を失敗させる | 再接続後も監査バイト列が不変 | publication_recovers_without_duplicate_bytes_after_checkpoint_failure |
| 子プロセスをファイル反映後・DB確定前に強制終了 | 未完了計画から復旧、二重追記なし | publication_survives_process_termination |
| A→B→再起動→A復元 | Aを復元、Bと位置6を維持 | changing_targets_does_not_restore_files_from_the_previous_intent。修正前の失敗も記録 |
| 保存済み終点と追加イベントの分離 | 当時は2回の取得で分離を検証。現在は同じ呼出しで別計画として処理する（下記追補） | recovery_finishes_the_saved_cut_before_consuming_new_events |
| 旧規約の正常な計画ダイジェスト | 破損とは独立に再開を拒否 | a_valid_plan_from_an_old_transform_is_not_resumed |
| CLIで状態・監査出力を削除してnextを2回実行 | 元のバイト列を復元、二重追記なし | next_restores_missing_projection_files_without_repeating_audit |
| cargo test --locked --workspace | 2,135件成功、失敗・無視0 | 49スイート。上記CLI追加試験1件はこの後に別途成功 |
| cargo fmt / workspace Clippy / cargo lint | 成功 | Clippyは全target、警告をエラー扱い |
| scripts/quint-gate.sh | 全項目成功 | モデルの型検査・不変条件・到達性・決定的シナリオ |
| scripts/coverage.sh | 行カバレッジ98.6467%、90%床を通過 | 相対ゲートは未実行。CLI追加試験前の同一プロダクトコードを計測 |
| 独立した実装再レビュー | 最後の対象別復元の指摘を解消、新たな重大指摘なし | 読み取り専用レビュー。正式な設計承認とは区別 |

全体試験で、クエリ側のDAOフィクスチャが「空ジャーナル・位置0・非空リードモデル」を作るため30件失敗した。整合性検査を緩めず、同じイベントをジャーナルへ保存して実際の終点で確定するフィクスチャへ修正した。保存履歴を読み直した投影との一致も検査し、DAO契約34件は成功した。

実行ログは `/tmp/verify-this/` の `u4-workspace-tests-complete.log`、`u4-clippy-complete.log`、`u4-lint-complete.log`、`u4-quint.log`、`u4-coverage.log`、`u4-cli-restoration.log` に保存した。一時ファイルなので、本記録には試験名と結果も残す。

## 2026-09-06 JSTの実装同期と契約検証

公開処理を、計画を保存する`prepare`と、次のTxで再照合して確定するprivateな`publish_prepared`へ分割した。別の書き手による同一要求の完了、新世代への置換、チェックポイントの前進、古いpredecessorに基づく解決候補を実SQLiteで検証した。古い計画はファイル操作前に拒否する。

SQLiteのエラー変換は`SqliteResultExt::at_store`へ、ファイルI/Oは`PublicationIoResultExt::at_output`へ集約し、対象パスと分類を維持した。prefix／suffixの余剰部分は`strip_prefix`／`strip_suffix`で直接取得する。条件・SQL・トランザクション境界を変更せず、利用者本文の保持とI/O拒否の契約を確認した。

`JournalReader`のFakeReader自己検証8件は実SQLite契約との包含を確認して退役した。不足していたsteeringの出典とチェックポイント独立性の2件は、公開JournalReader経路へ先に追加した。

| 追加・更新した検収 | 結果と根拠 |
|---|---|
| Conflict／Ioの診断 | 実ファイルで失敗させ、診断文字列のパス・分類と`Error::source`を確認。`publication_file_contract.rs` |
| REAL／BLOBへの共有行の破損 | 古い候補・同位置候補とも公開せず、再生成後に保存済み計画から再開。`typed_shared_row_corruption_blocks_old_and_same_position_publications_until_rebuilt` |
| 同位置のSAVEPOINT比較中のINSERT失敗 | 共有行・head・CPを保持し、trigger除去後に同じ候補を再試行。`a_same_position_comparison_failure_preserves_rows_head_and_checkpoint` |
| 古い解決候補 | P1から作った候補をP2の準備後に拒否。`a_resolution_based_on_an_old_predecessor_cannot_replace_the_new_generation` |
| CP書込中のshared head喪失 | DB全体をrollbackし、保存済み計画と既反映ファイルを保持して再開。`losing_the_shared_head_during_checkpoint_write_rolls_back_and_keeps_the_plan` |
| 同呼出しでの旧計画復旧と後続処理 | 最初の履歴targetは元の終点、最終CPは追加分まで進む。後続だけを失敗させた場合も旧cut・共有as_of・監査を保持し、再試行で最新へ進む。`recovery_finishes_the_saved_cut_before_consuming_new_events` |
| RMU全試験 | 最新の後続処理を含めて成功。`/tmp/pr3-rmu-drain-final.log`。全workspaceの結果とは区別する |
| CLI契約 | 118件成功。破損した保存計画の拒否を含む。`/tmp/pr3-intent-lifecycle-recovery.log` |
| 全workspace・最新main上の相対カバレッジ | 未測定。統合後に測定する |

エラー境界・世代・原子性の追加時点ではRMU465件とClippyが成功している（`/tmp/pr3-coverage-final-contract-all-tests.log`、`/tmp/pr3-coverage-final-contract-clippy.log`）。これは上表の最新CLI変更を含む全workspace検証ではない。

### 相対カバレッジの測定履歴

以下はbase `001b989bcfef10b3a5d2efc38237a9dd8a5d6199`を固定した過去の作業断面であり、直近の診断・型破損・比較失敗の追加契約と、同呼出し内の後続処理・CLI変更をすべて含む測定ではない。閾値・除外・許容誤差は変更していない。

| 作業断面 | head | 固定base | 相対ゲート（許容0.01pp） |
|---|---:|---:|---|
| 初回 | 98.646714% | 99.130435% | FAIL |
| 公開契約31件の追加後 | 98.830135% | 99.130435% | FAIL |
| SQLiteエラー変換集約・自己検証整理後 | 98.981363% | 99.130435% | FAIL |
| prefix／suffix整理・2相の競合検証後 | 99.036280% | 99.130435% | FAIL |
| 直近のU4／U7変更を統合した最新版 | 未測定 | 最新mainで測定予定 | 未判定 |

対応するログは`/tmp/pr3-coverage-before.log`、`/tmp/pr3-coverage-after.log`、`/tmp/pr3-coverage-refactor-final.log`、`/tmp/pr3-coverage-two-phase-final.log`。各断面のhead JSONと比較JSONも同じ接頭辞で保存している。

## 検証の限界

強制終了はプロセス停止の試験であり、電源断や記憶装置の故障を再現したものではない。競合解決APIは保守的に照合し、曖昧な変更を自動上書きしない。一般的な複数intentのイベント振分けの拡張は今回の変更に含まれない。過去の相対カバレッジは上記のとおり未達。直近変更を統合した相対カバレッジとリモートCIの結果は未取得である。


## 最新main統合後の検証（2026-09-06 JST）

Tell Don’t Askの変更を含むmain `a1ddb37d3bc4d9f91c3d5c84ad96f2264bf043a3` へ統合した。復旧失敗を先に返すため旧診断を期待していたCLI試験を更新し、障害の分類・完全パス・イベント非追記を検査した。

- `cargo test --locked -p aidlc --no-fail-fast`: 9スイート470件成功、失敗・無視0。
- `cargo test --locked --workspace --exclude aidlc`: 42スイート1,730件成功、失敗・無視0。
- 上記を合わせてワークスペース51スイート2,200件成功。CLIには破損計画からの指示・更新の拒否、実イベント確定後の公開失敗と一度だけの回復を含む。
- workspaceの整形、Clippy（全target、警告をエラー扱い）、`cargo lint`、差分検査は成功。

ログは `/tmp/pr3-aidlc-all-tests.log`、`/tmp/pr3-integrated-noncli-tests.log`。相対カバレッジとリモートCIはこの追記時点で未完了であり、上記の成功件数には含めない。


## レビュー修正時点の記録

日付はAsia/Tokyo（JST）で記録する。`9b4a6d55`のCIはUTC 2026-09-05 15時台、JSTでは2026-09-06 0時台の実行であり、将来の実行を記載したものではない。このheadの2,200件成功は確定しているが、相対カバレッジはローカル・CIとも99.01854354520681%対99.13906683022125%で未達だった。

後段読取との競合を実ファイル・別SQLite接続で検証する試験を追加した。所有パス一覧を一元化し、到達不能なメモリの再取出しと、使用されなくなった`CatchUpError::AuditShardWrite`を撤去した。旧変換headの再生成はopenから公開入口へ移し、投影不能を`ReadTables`のまま伝える。チェックポイントを巻き戻すフェイク独自試験を削除し、実SQLiteの中断復旧試験を正本とする。これらはこの追記時点で再検証中である。

共有面の完全な内容照合は行数に比例する。現在は全表再投影を行うため、表ごとのダイジェストを保存しても再計算対象は減らない。位置・行数だけでは同位置・同件数の改変を検出できず、今回の破損拒否を代替できない。性能最適化は完全照合の保証を維持する設計と規模別計測が必要であり、ここで計測済み・性能保証済みとは主張しない。


## 収束ループ完了（2026-09-06 JST）

[復旧修正のPR #111](https://github.com/amadeus-dlc/amadeus-ng/pull/111)は、最終head `e1691a5325d3f0f492efa52d671ca5c3a0af7cc9` の必須CI成功・未解決指摘0を確認し、マージキュー内のCIも成功してmainへ統合された。merge commitは `a37acecc16041238697c72f5a6e6bc07ef48a886`。

- カバレッジ計測の通常試験: 41スイート2,216件成功（doctestを除く）、失敗・無視0。
- 行カバレッジ: 99.13072110635495%。基準main `a1ddb37d` は99.13906683022125%。規定の許容差0.01ポイント以内で相対ゲート成功。閾値・除外の変更はない。
- `check`、Quint、coverage、配布整合性、監査、レビュー検査、集約CIが成功。記録は[CI run 33997547304](https://github.com/amadeus-dlc/amadeus-ng/actions/runs/33997547304)。
- CodeRabbitの指摘13件は現行コードで検証し、修正または根拠を記して解決した。別のレビューサービスが利用制限でスキップした実行を、コードレビュー実施済みとは数えない。

追加の是正は次のとおり。

- 共有headの欠落と、再openで作られた未検証headを公開入口で修復する。未公開の初期状態は先行再生成せず、未完計画の終点→追加分という順序を保つ。
- 空・部分的な計画も全出力先の束縛で照合する。所有や規約が合わない保存済みスナップショットを復旧不要として黙認しない。
- 差分を観測した後の全履歴が空なら`HistoryDisappeared`で拒否する。正常な読み面と破損した集約スナップショットが併存しても、報告処理は新規イベントを書かない。
- ファイル反映済みでも親ディレクトリの同期が失敗すれば公開成功としない。更新動詞の公開確認を共通化し、reportの保存先も入口で確定した`StorePath`を各経路へ渡す。
- 使われなくなった`AuditShardWrite`変種と`ReadModelUpdater::steering` getterは、互換用に残さず撤去した。

計測ログは `/tmp/pr111-store-context-coverage.log`、集計は `/tmp/pr111-final-coverage-comparison.json`。一時ファイルに依存せず再現できるよう、試験は各crateのテストとしてコミットした。正式な設計ゲートやintent完了の承認とは区別する。
