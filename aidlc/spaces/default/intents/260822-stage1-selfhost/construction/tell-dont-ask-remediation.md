# ユースケースのTell, Don't Ask是正

2026-09-05。ユースケースのドメインgetter禁止ルールが検出した24件を是正した。
検出ルールを緩めず、allowによる抑制も追加していない。

## 責務を移した先

| 従来のユースケース処理 | 是正後の依頼先 |
|---|---|
| 入力getterから再試行要求を再構築 | ReportRequest::for_retry_atが初回対象を固定し、他の観測を保持 |
| スコープ・レビュー指定を取り出して方針を組立て | Intent::resolve_review_policyが定義IDを照合して方針を判断 |
| 報告の段を分岐し、理由・人間入力を整形 | IntentExecution::apply_reportが判断に沿う操作を実行し、単一イベントを返す |
| 添字帳を取り出してステージ名を検索 | record_single_stage_runが対象を解決して隔離実行を記録 |
| 拒否前に現在地とスコープを取得 | コマンド拒否がstage・scope・原因の文脈を返す |
| 応答用のスコープを事前取得 | ReportDecisionが判断の文脈を返す |
| 実行・依頼のgetterで関連IDを取り出す | Repositoryのfind_for_execution / find_for_intentへ関連取得を依頼 |

関連取得のID読取はinterface-adapter内に限定し、既存find_by_idへ委譲する。
別の集約を復元・保存する権限、新たなキャッシュやI/O、業務判断は追加していない。
getter禁止と旧「Repositoryは自集約IDしか引数に取らない」の制限が矛盾しないよう、
gateway-taxonomy / aggregate-referencesの正本に、関連取得の限定範囲を記録した。

ユースケースは取得・ドメイン操作・保存と、楽観競合時の1回の再試行を進行管理する。
再試行の対象は初回に解決したステージへ固定し、状態と関連依頼を取得し直す。
復旧承認でも1コマンド1イベントを維持する。表示では内部文字列をgetterで抜き出さず、値自身のDisplayに任せる。

## 検証

以下は分割前の作業ツリー全体（復旧機構等の変更も含む）の実測記録である。
変更単位へ切り分けた後は各ブランチとCIで別途検証し、この件数を個別の変更単位の件数とは扱わない。

- cargo lint: 違反24件から0件、終了コード0。
- cargo test --locked --workspace: 49スイート、2,157件成功、失敗・無視0。
- cargo clippy --locked --workspace --all-targets -- -D warnings: 成功。
- cargo fmt --all -- --check / git diff --check: 成功。
- scripts/coverage.sh: 行カバレッジ98.5588222251785%、90%床を通過。
- 独立した読み取り専用レビュー: getterの改名や移動だけではなく、判断・操作がドメインへ移ったことを確認。重大な残件なし。

ドメインの新規操作と関連取得はテストを先に書き、契約不足の失敗から実装へ進めた。
既存のCLI応答・監査互換、競合時の対象固定、ガード拒否後の非変更、関連取得の不存在・破損・I/O情報の伝播も検証した。
実測ログは `/tmp/tell-workspace-tests.log`、`/tmp/tell-workspace-clippy.log`、`/tmp/tell-coverage.log`、`/tmp/tell-final-lint.log`。

これは作業ツリーの実装修正の記録であり、正式なライフサイクル承認やリモートCIの結果を代行しない。

## 隔離実行APIの一本化（2026-09-05）

`record_single_stage_run(&Intent, &StageSlug, t)` を唯一の公開コマンドに改めた。
対象の名指し解決とガードを同じ本体へ統合し、拒否型は `SingleStageRunRefusal` とする。
添字を取る旧署名、名指し専用の別名、旧拒否型のエイリアスは設けない。
単体テストの添字ラッパーも削除し、ユースケース・モデル準拠テスト・投影テストを
正規APIへの直接呼出しに揃えた。

旧公開口が利用できないことは3件のコンパイル失敗テストで固定する。変更前には3件とも
コンパイルが通ってテストが失敗することを確認した。名指し成功・未知名・初期化拒否・
他のintentの拒否、単一イベントと本流不変は正規APIのテストで検証する。

一本化後の検証結果:

- `cargo test --locked --workspace --quiet`: 49スイート、2,160件成功、失敗・無視0。
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: 成功。
- `cargo lint`: 所見0、終了コード0。
- `cargo fmt --all -- --check` / `git diff --check`: 成功。

実測ログは `/tmp/single-run-api-workspace-tests.log`、`/tmp/single-run-api-clippy.log`、
`/tmp/single-run-api-lint.log`。過去のレビュー・監査本文は変更していない。
