# U2のテスト手順

## 実行基盤と最初のコマンド

既存Rust workspaceのCargo/tokio/tempfileと、既存のSQLiteイベントストアを使う。最初に既存の報告ユースケースのテストを実行してランナーを確認する。

```bash
cargo test -p core-command-use-case --lib orchestration::commit_verdict_use_case::tests::
```

U1開始時の全Rust検査は2,354件成功。これはU2実装後の成功を意味しない。新規ファイルは承認後にテストを先に追加し、そのテストが意図した振る舞いで失敗したことを確認してから実装する。未検出・依存不足・構文不正をRedの代わりにしない。

## 単位限定のコマンド

次の新規integration test targetを追加する。通常のテストは実SQLite・公開APIを使い、実行用バイナリのテストはCargoがビルドしたaidlcを子プロセスで呼ぶ。既存multi-call面と新しいhook入口の引数を、その公開パーサへ渡す。

| コマンド | 対象 |
| --- | --- |
| `cargo test -p core-command-interface-adapter --test report_result_contract` | 報告イベントの保存と再構成 |
| `cargo test -p core-read-model-updater --test report_result_projection_contract` | 報告結果の投影・チェックポイント・復旧 |
| `cargo test -p core-query-interface-adapter --test report_result_dao_contract` | ID指定の結果取得・不在・ドメイン非依存 |
| `cargo test -p aidlc --test stage1_authority_contract` | 質問/回答/引継ぎ/レビューと真正な許可の境界 |
| `cargo test -p aidlc --test upstream_271_contract` | 本家2.7.1の同条件CLI/フック・状態/監査比較 |
| `cargo test -p harness-claude --test hook_contract` | 主要4フックと必要な追加イベントの入力/出力 |
| `cargo test -p aidlc --release --test upstream_271_contract` | releaseバイナリによる同じ契約の確認 |

これらは計画段階では未作成である。必要なdev-dependenciesは各所有crateのCargo.tomlへ追加し、既存ランナーを使う。テストを引数フィルターで増やす場合も、実行件数が0になっていないことを確認する。

## 既存契約の回帰コマンド

U2が変更する境界の既存検査を対象ファイルで実行する。

```bash
cargo test -p core-command-interface-adapter --test commit_verdict_use_case_wiring_test
cargo test -p core-command-interface-adapter --test intent_execution_repository_contract
cargo test -p core-read-model-updater --test publication_recovery_contract
cargo test -p core-read-model-updater --test projection_golden_test
cargo test -p core-read-model-updater --test audit_block_golden_test
cargo test -p core-query-interface-adapter --test read_model_dao_contract
cargo test -p aidlc --test intent_lifecycle
cargo test -p aidlc --test next_branches
cargo test -p aidlc --test steering_across_processes
cargo test -p aidlc --test cli_golden_test
cargo test -p core-command-interface-adapter --test golden_parity_test
cargo test -p core-infrastructure --test golden_hash_canonical
cargo test -p core-infrastructure --test golden_corpus_read
```

workspace全体・カバレッジ・Quint/ITF・CI・依存監査は、承認済み実行計画の共通検査としてB1統合前に行う。本手順の単位限定コマンドと区別する。

## 必須ケース

| 対象 | 正常と拒否/失敗 |
| --- | --- |
| report | 遷移する報告、3種類のno-op、構文/状態拒否、1回の競合再試行と2回目の伝播、報告ID/対象の保持 |
| 保存と結果 | SQLite追記→RMU→ID指定クエリ、別報告の取り違え、永続化失敗、投影失敗/再開、投影済み結果の不在、反復投影 |
| CLI進行 | Brownfield/bugfix 9段開始、必要なnext観測、複数部continue、期限/対象が合わないトークン、ゲート待ちと承認後の進行 |
| 受領 | decision/answer/link/review、内容確認、実装計画承認、回答前/別session/古い質問/内容変更/対象変更/受領再利用の拒否 |
| フック | 正常JSON、不正JSON/不足値、無関係入力、停止再入と正当な待機、実際のファイル保存、正規入口以外の遷移拒否 |
| 補助更新 | セッション帰属、規則保存の空/追加/重複/不正、同期とruntime graph、初回に不要な正本を作らないこと、診断実施監査の成功/失敗 |

Standardの各コンポーネント5–8件を基準に、契約の拒否条件を優先する。単に実装と同じ計算を書いたアサートや、コマンドの戻り値だけを観察するテストに偏らない。

## データ・代替処理・品質基準

U1の `tests/golden/upstream-a277af21/` と来歴・生観測・比較規則を入力にする。期待出力の手修正や固定文字列を消す正規化は行わない。反復コマンドの出力だけでなく、状態・監査の変化/無変更を同じ条件で比較する。新しい上流採取が必要なら固定コミットから実行して追加し、既存の比較基準の変更を隠さない。

テストの人間応答・会話は一時ワークスペース内の明示した合成入力とする。実際のユーザー承認や本リポジトリの受領を書き換えない。時計・乱数・プロセス/ファイル障害等の外部境界だけを必要に応じて制御し、Repository/RMU/Queryは実物の連携を検証する。

行カバレッジ床90.0%、相対ゲート0.01ポイント、seed20260823、既存Quint3モデルとITFを維持する。Red/Greenの実行コマンド・失敗理由・終了状態をU2記録へ残す。実地Claudeスモークとnative doctor成功は後続で実施し、このテストの成功で置き換えない。

