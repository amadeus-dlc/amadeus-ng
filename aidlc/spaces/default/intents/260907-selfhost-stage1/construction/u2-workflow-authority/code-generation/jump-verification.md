# jumpの保存・投影・公開入口の検証

## 実装した通常経路

U2計画Step 4/8のうち、Brownfield・bugfix・単一rootの`aidlc-jump resolve`と、解決された方向に沿う`execute`を接続した。CLIの別名に`aidlc-jump`を追加し、orchestrateへ独自動詞は追加していない。

- `JumpUseCase`は集約をRepositoryから取得し、単一の`Jumped`を保存する。成功戻り値は`Result<(), JumpError>`。
- `Jumped`へ`JumpObservation`を追加した。ソース一覧と`JumpArtifact`の宣言パス・存在・Review節の観測を保存する。集約全体や状態ファイルのコピーをイベントへ入れていない。
- コマンド側とRMU側のDTOは独立して定義した。`SourceBaselineDto`は側ごとの専用ファイルに分離し、フィールドはprivate。未採取と採取不能を区別する。
- 呼出側が内部のイベントIDを発行し、RMUが`read_jump_result`へ結果を投影する。QueryはそのIDで保存済み結果を返す。外部にIDフラグは追加していない。
- `read_next_jump`へresolveの結果列を追加した。影響する工程の判断をQueryで再実装せず、投影済みの結果を描画する。
- Source Baselineは移動監査と直後の工程開始監査で同一値を使い、既存の原子的な公開処理からcode-generation配下の内容アドレス付き一覧を公開する。
- backwardの3配列は移動前の実効プラン・checkboxと保存済み外部観測から導出する。対象の宣言成果物と、実際にresetされる後続工程の既存成果物・Review節を区別し、本家と同じ順序で重複を除く。
- 失敗監査は既存の共通処理へtool名を渡して再利用する。既存log面の処理を別経路へ置き換えていない。

## TDDと追加検証

最初のRed3件とredo結果差のRedは会話内の実行出力で確認した。下表はその転記であり、後から作った生ログとして扱わない。

| 検査 | 最初の結果 | 修正後 |
|---|---|---|
| 公開redo | `Unknown subcommand: execute. Valid: next, continue, report, park`で失敗 | 公開入口、保存、投影、結果取得を接続して成功 |
| backward監査 | `jumpの無効化監査が必要`で失敗 | 実際の宣言とファイル観測を保存して成功 |
| resolve | `Unknown subcommand: resolve. Valid: resolve, execute`で失敗 | 読取り用の投影列と取得を接続して成功 |
| 固定本家の通常5操作 | redoの`stages_reset`がnative `[]`、本家`[target]`で失敗 | 結果行を本家のredo契約へ合わせて成功 |
| ファイル変更後の再投影 | 追加した初回検査で成功 | 保存された監査と状態が元のバイトに一致 |
| 未公開の複数jumpが同じ一覧を共有 | 追加した初回検査で成功 | 同一の基準ファイルを公開し、2件の監査を保持 |
| SQL表の既存規約 | `read_jump_result`に`as_of`がなく失敗 | 列と書込みを追加し、SQL契約13件成功 |
| v4読取りスキーマ | 直前の正常ビルド済みCLIでresolution列欠落を再現、exit 1 | スキーマ版を5へ更新し、journal件数不変を含む検査に成功 |

v4の[Red観測](jump-logs/schema-red.log)は、レビュー実装の並行変更で新しいテストのビルドが一時停止したため、直前の正常ビルド済みCLIで同じ一時SQLite条件を実行して採取した。ビルド失敗をRedに数えていない。

## 固定本家との比較

`scripts/goldens/capture-jump-normal.ts`は`verifySource`で配布束全体を固定本家`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`へ照合し、一時workspaceだけで5操作を採取する。

採取先は`tests/golden/selfhost-stage1/jump-normal.json`。resolve、redo、forward、backward、target不足について、stdout・stderr・追加監査全文を比較する。正規化はworkspace/recordの場所とISO時刻だけ。基準fingerprint、成果物パスの後半、配列順、固定文言を消していない。

既存`projection_golden_test`のjump3件は、採取スクリプトが作った3つのpractices文書の観測と空のソース一覧をイベントへ与え、監査・状態の期待バイトを変更せず一致した。初期化へのbackwardも投影比較から外していない。ただし、保存済みイベントの投影成功と、同じ操作を公開CLIが受理することは別の検査である。

## 検査ログ

- [公開契約](jump-logs/contracts-current.log): 移行テストを含む7件成功。
- [jump投影golden](jump-logs/projection-golden.log): 3件成功。
- [Clippy](jump-logs/clippy.log): jump実装の対象testと依存コードが成功した時点のログ。並行変更を含む[最終実行](jump-logs/clippy-current.log)は、別担当の`review_binding_dto.rs`・`review_completion_dto.rs`で`&e.to_string()`の不要借用2件を検出して停止した。最終workspace全体のClippy成功とは扱わず、親担当へ修正を引き継いだ。
- [SQL契約](jump-logs/read-tables-current.log): `as_of`追加後の13件成功。元の失敗は[変更前ログ](jump-logs/read-tables.log)に保持。
- [独自lint](jump-logs/lint.log): 終了0。正常時は出力なし。

新規専用ファイルはrustfmtで整形した。共有ファイル全体の最終整形・全workspace検査・90%床と相対条件は統合時に確認する。

## 保留している契約差

[jump-contract-questions.md](jump-contract-questions.md)に、方向不一致・別scope・初期化への直接executeの本家参照、既存ドメイン規則、実入力と両出力、未回答欄をまとめた。新しい拒否を独自に確定していない。現在の方向不一致入力はnativeが導出した方向で動いてしまう既知差であり、正常対応として受領してはならない。

現在の成果物観測は、通常bugfixのstage単位の保存場所とreverse-engineeringのCodeKB保存場所を対象にする。登録済み複数repo、unit/DAG別の成果物展開、別scopeの実行を本検証で対応済みとはしない。

推奨の次の作業は、上の契約差を裁定してから全体の受入を判断すること。通常経路の検査を統合する場合も、保留条件を未達として保持する。
