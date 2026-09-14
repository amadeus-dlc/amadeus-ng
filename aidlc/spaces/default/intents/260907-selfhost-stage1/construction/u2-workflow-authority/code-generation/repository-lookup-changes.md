# RepositoryのID検索への統一

2026-09-14の利用者指示に基づき、調査で提示した案Aを実装した。変更前の事実は `repository-lookup-investigation.md` に残す。

## 変更内容

- `find_for_intent`、`find_for_execution`、`find_for_approval_origin` の契約・実装を撤去し、21か所のユースケース呼出しを参照先IDによる既存の `find_by_id` へ変更した。関連索引は追加していない。
- `tell-dont-ask.md`、`aggregate-references.md`、`gateway-taxonomy.md`、`use-case-rules.md` と規則一覧、lintのREADMEを整合させた。
- lintは、コマンド側のRepository traitの検索引数型と、getterが返すドメインのID型が一致する直接受け渡しを許可する。借用・括弧・IDのclone・UFCSも対象とする。IDの比較や分岐、内部表現への変換、ID以外のgetter、補助関数やブロックへの移動は許可しない。
- `InMemoryWorkflowDefinitionRepository` は `HashMap<WorkflowDefinitionId, WorkflowDefinition>` のみを保存状態とする。検索回数と成功書込みの観測はSpy、誤応答・破損・競合はそれぞれ専用のテスト用型へ分離した。通常のRepositoryに障害注入の設定は残していない。
- lintが `impl` の型制約も追跡するようになり、既存の業務側getter使用2件が新たに検出された。生成承認では共有集約をドメインの判断メソッドへ渡し、指示の文脈照合は `IntentExecution::matches_directive_context` に委譲した。

## 失敗を先に確認した検査

| 検査対象 | 修正前の結果 |
|---|---|
| RepositoryにIDを直接渡すlint許可例 | 所見が返るため失敗 |
| 借用・clone・UFCSのID受け渡し | 所見が返るため失敗 |
| `impl` にだけRepositoryの型制約がある場合 | 所見が返るため失敗 |
| 異なる定義IDの保存・改訂の独立性 | 2個目の新規保存が `Conflict { expected: 0, actual: 1 }` となり失敗 |
| 集約による文脈一致の判断 | 追加予定メソッドが存在しないためコンパイルで失敗 |

## 検証

以下は切り出し元の作業ツリーでの検証結果である。実行ログは同作業ツリーの `repository-lookup-verification/`（Git管理対象外）に保存した。

- ユースケース単体188件、Repository関連105件、ドメインの入口ガード10件、独立lint103件が成功。
- `cargo lint`、ワークスペースと独立lintの整形検査、両方のClippy（全ターゲット、警告をエラー扱い）が成功。
- 変更したコード・規則文書の差分空白検査は成功。既存監査ログには末尾空白の指摘があり、監査ログは手編集していない。
- 通常の `cargo test --workspace --quiet` は、アプリ単体の414件成功・1件失敗で停止した。失敗箇所は今回未変更の `session_processes::tests::ancestry_cleanup_removes_stale_entries_and_keeps_live_ones`。実プロセスを50ms以内に観測するテストであり、`None` と `Some("s-live")` の不一致が発生した。時間制限や期待値は変更していない。

再確認では、失敗したテストの単独実行が成功した（1件、0.06秒）。`--test-threads=1` による全体の逐次再実行でも、アプリ単体415件すべてが成功した（9.47秒）。通常実行時の失敗がこの変更に起因するという証拠は得られていない。

全体の逐次再実行は、アプリ単体415件とmainの0件の成功後に中断した。実行ファイルの起動が長時間待機し、macOSのサンプリングで `_dyld_start` に留まっていることを確認したためである。タイムアウト条件・期待値・テストの無効化は変更していない。**ワークスペース全体のテスト完走は未確認**であり、関連検査の成功を全体成功へ拡大解釈しない。

静的解析はrustcの型検査ではなく、マクロ・外部契約・複雑な型推論の限界を持つ。lint成功だけで、全業務判断の適切な配置まで保証したとは扱わない。

公開用ブランチは最新の `origin/main`（`33b61cdb`）から作成し、今回の48ファイルだけを切り出した。共有の `test_support.rs` では、今回以前からの自己診断・CodeKB向け追記を含めていない。切り出し元の未コミット変更は保持した。ワークフローの工程遷移と監査記録の手編集は行っていない。

切り出し後にも、独立lint103件、`cargo lint`、ワークスペースと独立lintの整形検査が成功した。
