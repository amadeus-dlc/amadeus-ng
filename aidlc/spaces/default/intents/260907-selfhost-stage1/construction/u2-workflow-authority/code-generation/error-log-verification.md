# log拒否の監査・診断形式

2026-09-09。Step 5のlink調査から検出した、log面共通の `ERROR_LOGGED` 欠落を是正した。U2全体の完了を示す文書ではない。

## 変更

`aidlc-log` の拒否を `{"error":"..."}` のstderr、exit 1、空stdoutへ揃えた。実行対象の状態が存在すれば、失敗したTool、Command、Errorを `CommandFailed` イベントとして既存IntentExecutionRepositoryに保存し、通常のRMUで監査を描く。工程位置・承認・進捗は変えない。新しいストア、監査専用ライター、TS更新フォールバックは作っていない。

監査のCommand/Errorでは本家と同じプロジェクトパス秘匿を行う。末尾の区切り、Windows区切り、実パス、JSの空白集合を扱い、似た名前の隣接パスや引用符直前を無条件に置換しない。stderrは元の診断を保持する。

固定本家 `aidlc-lib.ts:22207` は失敗監査の保存失敗を飲み込み、元のJSONエラーを必ず返す。この契約に合わせ、監査の保存・投影不能を元の拒否へ上書きしない。保存済み・未公開のイベントは通常の次回投影で回収し、二重に描かない。状態のない拒否ではintent・ストア・監査を新設しない。

## 本家観測と検証

`scripts/goldens/capture-log-failure.ts` で、固定配布277ファイルのmanifest照合後に本家CLIを実行し、6観測を `tests/golden/selfhost-stage1/log-failure.json` へ保存した。既存コーパスは変更していない。cold拒否、開始、通常拒否、反復、reviewの構文拒否、未知動詞を採取した。

- 初回Redは、公開診断がJSONではなく裸の文字列だったことを検出した。途中のドキュメント不足や他担当の編集中コードによるコンパイル失敗はRedに数えていない。
- 新規CLI 3件が成功。通常拒否は本家の追加監査全文とstderrを比較し、監査のTimestampだけを正規化した。工程ファイルのバイト不変、反復は別の失敗2件、cold無作成、公開障害からの重複なし回復も検証した。
- path秘匿2件、集約の権限不変・差分再構成1件、採番枯渇時の無変更1件が成功。
- 既存の同じRepository実装を使うMemory・SQLite共通契約2件が成功。再開後の全体同値と工程・進捗不変を確認した。
- `intent_lifecycle` 全121件が成功。既存review拒否の検査はJSON封筒を必須としたうえで、従来の診断本文を逐語比較する形へ更新した。
- link担当の別採取27観測も、本共通処理を通したstdout/stderr/監査全文が一致したとの報告を受領した。独立したlinkの証跡で最終照合する。

ログは `test-evidence/log-failure-*.log`、`log-redaction.log`、`error-domain.log`、`error-exhaustion.log`、`error-backends.log`、`error-cli-regression.log` に保存した。対象5crateの全target Clippy、全体fmt、定期cargo lint（18回目まで）が成功。Clippyログは `test-evidence/error-clippy.log` に保存した。

## 変更の所在

- domain: `command_failure.rs`、`intent_execution_event/command_failed.rs`、既存実行のコマンド・適用・イベント公開。
- use-case: `record_command_failure_use_case.rs`。成功戻り値はunit、同じ対象で楽観競合を1回だけ再試行する。
- interface-adapter / RMU: 各側専用の `command_failed_dto.rs` と既存イベントDTOの変種。RMUの純粋投影へERROR_LOGGEDを追加。
- app: `runtime/log_failure.rs` とlog面の共通終了接続。個々のhandlerは生の診断を返す。
- tests: `intent_lifecycle.rs`、domainの実行・イベントテスト、既存Repository共通契約。

## 残る範囲

今回はlog面の共通終了処理が対象。state等の他のCLIで同じ監査が必要な経路まで接続済みとは扱わない。稼働中のCLIが壊れた公開ファイルを修復する今回のSQLite/RMU固有動作は、同一障害後に本家と全ファイルが等しいという証明ではない。
