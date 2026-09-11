# 本家2.7.1の採取証跡

## 固定元と再採取

本家コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` のGitオブジェクトから `git -C vendor/aidlc-workflows archive a277af218f0df7f325d3b8be7b6d90fce2c5bd40 dist/claude` で取得した。forkの作業ツリーやローカル修正フックは使っていない。

配布277ファイルについて、相対パスのバイト順で並べたSHA-256一覧のハッシュは `282b17c53cb82c24755b149f28e01508ce9eaebbdbe3020034a4dd4c8bda4459`。一覧を `scripts/goldens/upstream-manifest.sha256` とコーパスの `source-manifest.sha256` に保存した。最初の測定でPathオブジェクトの順序とバイト順が異なることを検出し、規定のバイト順で独立に再測定して固定した。

最終採取は次の2コマンドで独立したワークスペースと出力先へ実行した。どちらも終了0。

```bash
bash scripts/goldens/recapture-cli.sh /tmp/amadeus-u1-release-a /tmp/amadeus-u1-upstream/dist/claude
bash scripts/goldens/recapture-cli.sh /tmp/amadeus-u1-release-b /tmp/amadeus-u1-upstream/dist/claude
bun scripts/goldens/verify-corpus.ts /tmp/amadeus-u1-release-a /tmp/amadeus-u1-release-b
```

比較は終了0、出力は `採取コーパスの検証成功`。Aの内容を変更せず `tests/golden/upstream-a277af21/` へ保存し、保存後の検証も終了0。取得方法を指定しない場合は、固定SHAへのGit fetch、失敗時に固定SHAのcodeloadへ進む。取得方法によらず配布277ファイル全体の検証が必須である。既存の出力先へは上書きしない。

## 採取範囲

| 保存面 | 結果 |
| --- | --- |
| 配布JSON | 固定元の `data/` を実バイトで保存。旧ルートのstage-graph/scope-grid/harnessは新しい `data/` 内を参照する |
| 従来のCLI表示面 | 28ケース、欠落0。補助初期化を含めたrawは主要フックと合わせて46観測 |
| 主要4フック表示面 | 14ケース、欠落0 |
| hash-canonical | 32ケース、欠落0。正準化・非正準化の出力とハッシュを実行して採取 |
| 補完 | 複数部分の継続配送、set-autonomyの合成状態、会話中Stopの3ケース・13観測 |
| stage1 | 47観測。計画/内容確認/レビューの受領、保護有効の許可と拒否、ゲート承認、link、追加フック、表示補助 |
| doctor | 20診断ケース＋8初期化観測。本家全文を保存し、Native独自診断は作らない |
| learnings | 8観測。surface、空選択、1件追加、重複抑止、不正選択等 |

プロセスの生観測は合計142件。コーパスは345ファイル・10,259,357バイト。hash-canonicalの純粋関数32ケースはプロセス観測件数とは別である。

`aidlc-testing-posture.ts` 全体のSHA-256は `c06fb41743d9c10c5d50f88530096db8a1a5e5c8f61468855cfbdab39ee2c4f0`。関数抽出範囲は160–179行で、抽出本文のSHA-256は `c8894a433d620538e1701f178b8542528603f012b98680b6b79233f70704418f`。旧版の104–123行から移動しているが、3関数の本文は同一バイトだった。import/exportの外枠以外を変更せず実行した。

## 生データと比較規則

各観測にargv、stdin、採用環境、初期ファイル、変更ファイル、stdout/stderr、終了コード、signal、起動失敗、観測時刻を保存する。stdout/stderrとファイル内容はbase64でも保存し、不正UTF-8を別のバイトと同一視しない。固定配布と同じファイルは参照マニフェストで再現し、変更・追加・削除を保存する。Bunの生成キャッシュは作業成果や監査ではないため含めない。

親の環境は継承しない。PATHは実Bun配置とOS標準パス、HOMEは採取ワークスペース内の `aidlc/.capture-home`、localeはC.UTF-8、TZはUTC。doctorのロック競合だけは、親が保持する本家のロック領域と一致するTMPDIRを明示する。全設定は各input.environmentに残る。

正規化は `comparison.json` の実測値へのliteral置換で、同じ役割を両辺へ適用する。未知の規則、片辺だけの役割、複数IDを同じIDへ潰す規則は拒否する。

- 作業パス、実行ファイル/実行環境、採取先は実際の引数・環境から取得する。
- intentのUUIDは生成されたintents.jsonの `uuid` から取得する。固定セッションUUID・固定コミット・要求IDは保持する。
- 継続トークンは返された `continue_token` と、Stop理由に埋め込まれた同じ封筒構造から取得する。各トークンの対応を保持し、後続入力の取り違えを検出する。
- 監査のclone名、Fire id、Duration msは該当する実測フィールドから取得する。発火IDは対応を保持する。これは既存の本家出力の採取であり、センサー製品機能を追加するものではない。
- 生成時刻は採取時間内の値、heartbeatは明示した合成入力、学習日は生成されたlearned句、受領のmtimeはArtifact Mtime Msフィールドに限定する。
- 計画承認の環境依存ハッシュはDirective Epoch、Approval Fingerprint、Questions SHA-256、Prompt SHA-256の実測フィールドだけを対応付ける。任意のSHAを一括削除しない。ハッシュ算法は別の固定32ケースで検査する。

署名鍵、lock、pid等の `.aidlc-*` 機械内ファイルは生の再現入力に残すが、C1の公開state/audit比較面とは区別する。時刻正規化後に初期値と同一になるchanged_files項目だけを除く。これはsync-workflow-stateが同じ秒には無変更、秒を跨ぐとLast Updatedだけを更新する実測差に対応する。状態値が変わる場合は除外しない回帰テストを置いた。

## 旧版との照合と限界

[全数台帳](legacy-differences.json)と[全バイト差分](legacy-differences.patch)へ、旧版320ファイルと新規追加26ファイルを対応付けた。220ファイルは完全バイト一致。状態・監査・標準入出力、来歴、正規化、配布JSONを別々に分類し、U2の[参照移行一覧](legacy-consumer-inventory.md)へ接続する。

旧CLIの正規化済み表示ファイルだけを新しい受入比較の正本にしない。旧版はUUID等の対応情報を失っていたため、新版の生観測と対応表を使う。U1は本家の採取基盤であり、Rustの2.7.1適合、ネイティブdoctor、実地スモーク、CI全ジョブの完了を宣言するものではない。

合成会話・初期ファイル・段階の設定を使った契約テストである。実際の人間の承認を採取用に転用していない。従来CLIの保護無効化はprovenanceへ明記し、stage1の保護有効ケースと区別した。fold-usageは明示的な利用量集計無効時だけを採った。外側の実地スモークの成功証拠には数えない。
