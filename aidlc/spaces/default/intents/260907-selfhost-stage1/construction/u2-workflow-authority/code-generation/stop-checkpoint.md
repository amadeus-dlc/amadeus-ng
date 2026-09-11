# Stop実装の検証済みチェックポイント（U2は未完了）

2026-09-09。承認済みTesting Contract `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55` に沿った途中記録である。工程の完了・承認・一時停止を表すものではない。

## 実装と確認が済んだ範囲

- heartbeat-directoryの診断裁定Aを実装。固定原観測と同じ初期ファイルを置き、EISDIR/open/対象パスのRust固定1行を確認した。原Bun診断は保存したまま、承認されたこのケースだけ全文比較を分ける。公開ファイルの集合・内容・heartbeat directoryを前後比較し、内部SQLite/WAL/SHM/共有初期化markerだけ明示除外する。
- 作業のないStopはJSONの形によらず停止を許し、heartbeatを記録する。初回にaidlcディレクトリがない場合も確認した。
- `WorkflowContinuation` が実行ごとに進捗署名と連続回数を所有する。Repositoryが既存共有SQLiteへ単一イベントとsnapshotを保存し、RMUがcounterと要求ID結果を投影する。更新UseCaseの成功はunit、Queryは呼出側の `ContinuationAttemptId` で結果を引く。
- 初回差止め、同進捗2回目の解除、再入時の初回count2、環境指定3/3trailing、park後の空署名/count0 resetを公開CLIで確認した。既定2/8の選択は最新 `IntentExecution` が所有する自律モードから決める。
- 開いた承認ゲートは `IntentExecution::continuation_wait` の判断を `read_execution` へ投影して停止を許す。Controllerでcheckboxを再判定しない。ログ質問の待機判断も同じ経路へ載せたが、専用の追加境界検証は未完了。
- Stopのnext照会は別nativeプロセスで10秒上限を持ち、`AIDLC_STOP_HOOK_PROBE=1` を渡す。承認済みCode Generationの受領ファイル・active directive・Sourceイベント数が照会で変わらないことを確認した。
- 使用済み共有承認ストアを失った後にStop/health/dropが空DBを再作成する不具合を解消。承認初期化と同じmarker検証を使い、停止は許すが欠落DBを作り直さない。
- 固定本家の関数本文を変えず、計測用exportだけを追加して11入力の進捗署名・文言・blockStop出力を保存した。全署名と通常run-stageのstdout全文が一致。既存U1採取物は変更していない。
- HookHealthのreplay/applyを、壊れた履歴にResultを返す形から既存原則どおりpanicする形へ是正した。HookDropSummaryの複数構築口も検査付きnewへ集約した。

## この地点の検査

| コマンド | 結果 |
| --- | --- |
| `cargo test -p aidlc --test upstream_271_contract stop_` | 7件成功、52件はフィルター対象外 |
| `cargo test -p aidlc --test claude_hook_contract` | 10件成功 |
| `cargo test -p core-command-domain --test hook_health_contract --test workflow_continuation_contract` | 7件＋8件成功。後者に11個の本家署名観測を含む |
| `cargo test -p aidlc --lib` | 287件成功 |
| `cargo test -p core-query-interface-adapter --test read_model_dao_contract` | 34件成功 |
| `cargo clippy -p aidlc --all-targets -- -D warnings` | 成功 |
| `cargo lint` / `cargo fmt --all --check` / `git diff --check` | いずれも成功 |

親がStopの初回/再入上限/承認ゲート3境界とEISDIRを独立実行して成功。親の最新 `cargo audit` は129依存で成功し、対象Cargo.lockのSHA-256は `9e2ab79cf4533986823f7f86afaf1e365e175c5dd67d735f29a8c7a225fc458f`。ロック変更後は再検査する。これらはCI・release・workspace全体の成功の代用ではない。

生ログは `test-evidence/stop-*`、`test-evidence/heartbeat-directory-*`、`test-evidence/hook-health-replay-*`。この再開分のファイル一覧とSHAは `stop-checkpoint.json`。U2全体の最終source-manifestではない。

## 次回へ残す範囲

1. Stopの未回答文書・compose・背景subagent・会話・resume待ち、session/intent選定、autonomousでのpark、unit-majorの例外、実タイムアウト・保存/投影障害・再開・並行実行の確認。停止回数のRepositoryは現在SQLite実装であり、memory backendの同一契約検査も残る。
2. Stop結果の投影は専用RMUのDB排他中にmarkerを書き、その後に結果と位置を確定する段階。失敗時の公開物とread表の回復を、正常系だけで検証済みとは扱わない。
3. 必要補助フック・link等の未接続CLI・既存の受領/拒否境界。HookHealth/ArtifactAuditを含む新規実装全体の設計規則・構築経路・再構成契約の最終点検。
4. 2.7.1投影差のSource Baseline、Validation Basis、ジャンプ無効化情報等。親が進めた受入移行の結果は `infrastructure-golden-migration.md`、`distribution-field-migration.md`、`rmu-golden-migration.md` を参照。SENSOR_FIRED/PASSED/FAILEDだけの扱いは `sensor-audit-comparison-questions.md` の人間回答待ち。
5. 全workspace/Quint・ITF/release・90%床と相対ゲート、最終source-manifest/traceability/code-summary、独立レビューとB1統合。実地Claude・native doctorは後続Unitの達成条件。

既存のArtifactSaved通常RMU統合と前回のnext入力修正は保持する。古い「Artifact専用RMUが未統合」の記録へ戻らない。
