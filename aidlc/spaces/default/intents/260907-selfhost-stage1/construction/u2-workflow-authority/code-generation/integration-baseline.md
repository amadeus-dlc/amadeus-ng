# B1統合前の基準確認

2026-09-08にGitHub APIと現在の作業ツリーから確認した。実装中の変更に対するCI結果ではない。

- リモートmain、ローカルmain、HEADは `1dc727e00a26c27258d916cb3a5e0592c6c48c1c`。
- [基準コミットのCI](https://github.com/amadeus-dlc/amadeus-ng/actions/runs/34112228945) はmerge_group実行で成功。check・aidlc-distribution・quint・coverage・audit・CI Successの6ジョブが成功し、CI Review Thread Gateはイベント条件により非適用。
- coverageの相対比較は同実行では非適用で、絶対床の検査が成功。B1の変更では相対比較も別途確認する。
- [最新の定期実行](https://github.com/amadeus-dlc/amadeus-ng/actions/runs/34173639064)も同じコミットで成功しているが、これを変更後のCIや全検査の代わりにしない。
- [stage1ブランチの開いているPull Request一覧](https://github.com/amadeus-dlc/amadeus-ng/pulls?q=is%3Apr+is%3Aopen+head%3Astage1)は確認時点で空。

現在のCI定義はpull_request・merge_group・workflow_dispatchで実行される。checkとcoverageはRustだけを導入し、Bunとsubmodule固定祖先の取得はaidlc-distributionに限られる。Rust契約テストを、Bunや手元にだけあるvendor祖先へ暗黙依存させない。

mainの適用規則をGitHub APIでも確認した。マージキューは `SQUASH` / `ALLGREEN`、同時ビルド・同時統合とも1件。必須チェックはcheck・quint・coverage・CI Successで、ベースの最新化も必須。B1はこのキューを通し、キュー用のmerge_group検証も確認する。auditは必須チェックの列挙外だが、今回の完了条件では引き続き成功を要求する。

U1の追加テスト・採取物とU2実装はまだ基準コミットへ統合されていない。B1を具体的な差分へまとめ、変更したコミットに対するCI全結果と収束を確認してから統合する。

後続の実地確認に向け、2026-09-08に `claude auth status` を実行し、終了コード0、`loggedIn: true`、`apiProvider: firstParty` を確認した。認証情報そのものは採取していない。これは既存ログインの確認であり、Rustバイナリを使った実地スモークの成功を意味しない。

同日の実装途中に依存脆弱性検査を実行し、`cargo audit`（125依存）と `cargo audit --file tools/lint/Cargo.lock`（5依存）はいずれも終了コード0だった。RustSecの助言1,242件を読み込んだ結果であり、CIの成功を代替しない。検査対象のSHA-256は本体 `Cargo.lock` が `3f89019f70c68d40d0fd01291a4c073f37e123af7e93a5400b36628b677ecf78`、lint側が `216e95f5af45ada9756c044b789c6f780183ce3aa45b87eeaaff0429eedc0f47`。以後ロックファイルが変わった場合は再検査する。

`bash scripts/quint-gate.sh` も終了コード0で成功した。3モデルの型検査、不変条件、到達性、停止制御の決定的シナリオを既存シード・試行数で実行した。検査時点の3モデルとスクリプトは上記HEADと同じバイトで、ログは `/tmp/amadeus-stage1-quint-gate.log`。これはモデルの検証結果であり、変更中のRust実装によるITF再生やフック契約の検証は別途必要である。

独立した `tools/lint` は、`cargo fmt --manifest-path tools/lint/Cargo.toml --all --check`、`cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings`、`cargo test --manifest-path tools/lint/Cargo.toml` がいずれも終了コード0。テスト93件が成功し、ログは `/tmp/amadeus-stage1-lint-tool-test.log`。検査時点の `tools/lint` はHEADと同一である。これは検査ツール自体の確認であり、変更後の全ソースに対する `cargo lint` も別途通す。

U2の承認記録の接続後、実装担当とは別に `cargo test -p aidlc --test plan_runtime_contract plan_answer_result_is_read_by_its_id_after_projection_and_recovery` を実行し、1件成功・失敗0・終了コード0を確認した。SQLiteへの保存、RMUによる投影、指定操作IDのQuery、未投影時の旧結果、再接続後の回復を対象とする。この時点では承認回答のCLIと実装開始の接続は未完了であり、この結果をU2全体の完了やCI成功へ拡大しない。

続いて `cargo test -p aidlc --test plan_runtime_contract generation_query_observes_publication_then_certification_by_the_same_id` も独立実行し、1件成功・失敗0・終了コード0を確認した。開始候補と確定を同じ操作IDで追跡し、確定保存後も投影前には旧結果、再接続・投影後には開始済みの結果を取得する境界を検証した。この検査時点では `begin` のCLI接続は未完了である。

状態遷移ガードの接続後に `cargo test -p aidlc --test claude_hook_contract` を独立実行し、3件成功・失敗0・終了コード0を確認した。通常実行53ケース、委譲実行53ケース、およびU1の基本4ケースを照合する検査である。引用、ラッパー、関数定義、ヒアドキュメント内の実行置換などを含む。この時点では不正なフック入力や追加のラッパー境界、最終の全体検査は未完了である。

利用者の途中検査の指示を受けて `cargo lint` を実行し、802ファイルの走査で `use-case-domain-getter` 違反4件、終了コード1を確認した。`record_plan_answer_use_case.rs` の集約参照IDとソース指紋の取得、`record_plan_decision_use_case.rs` の選択肢取得2箇所であり、是正対象へ渡した。Clippy成功をこの検査の代替にしない。

Artifact監査をイベント経路へ是正後、`cargo test -p aidlc --test claude_hook_contract write_audit_hook_does_not_bypass_the_artifact_event_pipeline` を独立実行し、1件成功・失敗0・終了コード0を確認した。Write/Editおよび監査ディレクトリ障害がArtifact監査を直接書かず、障害時だけHookHealthのdropへ流れる境界である。

HookHealthのRMU接続後、`cargo test -p core-read-model-updater --test hook_health_projection_contract rmu_projects_only_hook_health_events_after_interleaved_journal_rows` を独立実行し、1件成功・失敗0・終了コード0を確認した。HookHealthの3事実を `read_hook_health` へ投影する範囲である。テスト名に反して別streamの行を混在させた選別と承認側履歴の保全は、この時点では未検証であり、後続テストへ渡した。

`setter-method` 追加後に `cargo build --manifest-path tools/lint/Cargo.toml` を実行し、生成された実バイナリを7つの独立した一時ワークスペースへ適用した。privateメソッド、raw識別子のtrait宣言、レシーバなし関連関数、`cfg(test)` 内メソッド、allowコメント付きの5ケースは `error[setter-method]` と終了コード1。消費型 `with_*` ファクトリ、自由関数と文字列の2ケースは終了コード0だった。この確認は検出ルールのCLI接続の証拠であり、実リポジトリのsetter・getter違反の是正完了とは区別する。

setter3件・getter4件の是正後に、親が `cargo lint` を再実行し、所見なし・終了コード0を確認した。コードの読取りでも、回答検証が `IntentExecution::verify_plan_answer`、提示の生成が `PlanChallenge::from_prompt`、参照IDによる取得がRepositoryへ移ったことを確認した。この時点では完全コンストラクタ経路の棚卸しと関連する回帰・整形・静的解析の最終確認は継続中である。
