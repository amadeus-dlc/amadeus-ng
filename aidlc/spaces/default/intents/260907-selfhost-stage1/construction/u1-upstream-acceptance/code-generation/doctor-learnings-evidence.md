# 自己診断・学びの追加採取の検証

## 対象と経路

固定本家 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の配布277ファイルを `verifySource` で検証し、使い捨てディレクトリへ展開して本家のCLIを実行した。本リポジトリの状態・承認や規則へ合成入力を流していない。

`capture-doctor.ts` は `captureDoctor(dist)`、`capture-learnings.ts` は `captureLearnings(dist)` を公開する。生の標準入出力、終了状態、初期ファイル、変更ファイル、引数と環境は共通の `captureObservation` を使って保存する。出力を本実装の期待に合わせて修正していない。

## TDDの記録

各項目を追加するたびに、採取観測が存在せず `expect(row).toBeDefined()` が失敗するRedを確認した。その後、実際の本家CLIを呼ぶ採取を実装してGreenを確認した。実行コマンドは以下の単位限定コマンド。

- `bun test ./scripts/goldens/capture-doctor.test.ts`
- `bun test ./scripts/goldens/capture-learnings.test.ts`

テスト未検出やimport失敗をRedとして数えていない。初期の入口は固定元を検証して空の観測配列を返すところまで用意し、テストが実行されて欠けた採取動作を検出することを確かめた。

## doctor

最終結果: **10件成功・0件失敗・80 assertions**（終了0）。20診断シナリオと、記録を要するケースの8初期化観測を生成する。通常実行は約15秒で、監査ロックの競合待ちを含む。

| 検証 | 採取ケース |
| --- | --- |
| 初回にファイルを生成しない | cold |
| 既存記録にHEALTH_CHECKEDを1件記録 | initialized |
| Bun不足 | missing-bun |
| hook/設定の欠落、無効化、参照なし、JSON不正 | missing-hook / disabled-hooks / missing-settings / no-hooks / invalid-settings |
| 管理設定による制限 | managed-only |
| 状態版の欠損・7・9 | version-missing / version-past / version-future |
| 300000ms境界の許容、300001msの失敗 | heartbeat-boundary / heartbeat-stale |
| 読取り不能を初回未発火と区別 | heartbeat-unreadable |
| 必要ステージ欠落・schema不正・参照不正・循環 | missing-stage / invalid-stage / invalid-reference / cyclic-graph |
| 診断監査の保存失敗 | audit-locked |

`heartbeat-boundary` は全診断の終了0も確認した。汎用履歴診断のadvisoryが表示されても、この警告だけで終了値は失敗にならない。本家のinitializedは実フック未発火の構成なので、正常構成と同一視しない。

`fixture_changes` は固定配布からの合成変更をbase64または削除のnullで記録する。`fixture_directories` は不読heartbeatを再現するための空ディレクトリを明示する。権限000は採取側自身も読み取れなくなるため、ファイルを期待する場所へディレクトリを置き、本家の実ファイル読取り失敗を起こした。

管理設定は本家が提供する `AIDLC_MANAGED_SETTINGS_PATH` を一時ディレクトリ内へ向け、実マシンの管理設定を変更しない。Bun不足は子プロセスのPATHだけを隔離し、本家CLIは絶対パスのBunで起動する。

監査失敗は本家の公開 `acquireAuditLock` を親が保持し、正規CLIのロック取得を拒否させた。`fixture_lock` がその前提を示す。子のTMPDIRも親の実測値に合わせる。実測上、doctorと同じ省略引数で取得したロックが競合する。必ずfinallyで解放し、ロック失敗を模擬の監査イベントで代替していない。

最終ログ: `/tmp/doctor-green.log`。個別のRedログは `/tmp/doctor-{cycle,managed,unreadable,audit,invalid-settings}-red.log` に保存した。先行ケースのRed/Green出力はこの作業のツール実行記録にある。

## learnings

最終結果: **6件成功・0件失敗・27 assertions**（終了0、約0.7秒）。8観測を生成する。

1. 本家intent-createでbugfixの合成作業を初期化する。
2. 本家runtime compileでsurfaceに必要なruntime-graphを構築する。
3. surfaceが選択元space/intentと空候補を返し、ファイルを変えない。
4. 空選択のpersistは規則・監査を変えない。
5. 合成した学び1件をpersistし、規則とRULE_LEARNEDの対応を確認する。
6. 同じ選択の再実行は追加なし・ファイル変更なし。
7. 異なるslugへのpersistは終了1・変更なし。
8. spaceを欠いた不正な選択入力は終了1・変更なし。

selectionファイルのspace/intentは実際のsurface出力から引き継ぐ。規則は採取用の日本語1行を `user_addition` の合成入力として渡す。センサー生成や本リポジトリへの規則追加は行っていない。

## 引継ぎ上の限界

本家doctorの全文を採っており、C7のネイティブサブセットへ加工していない。Native workflow identity・SQLite・RMUの独自診断とその異常注入はU2/U3の責任。本データは実地スモークの証拠ではない。

承認済み計画Step 5のC6/C7採取を、担当ファイルの競合を避けるため独立した2つの採取ファイルと2つのテストへ分けた。追加コマンドはCIと全体検証へ接続する。Testing Contractと承認済み計画本文は変更しない。

