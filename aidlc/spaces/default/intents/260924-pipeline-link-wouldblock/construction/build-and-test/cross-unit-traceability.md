# 最終カバレッジゲート（ステージ単位）— Issue #134

## 判定

**FAIL（未カバー 4 件）**。4 件のどれにも理由が記録されており、承認ゲートで人間が判断する。

## 突き合わせた範囲

- 要件: `aidlc/spaces/default/intents/260924-pipeline-link-wouldblock/inception/requirements-analysis/requirements.md` の FR1.1〜FR5.1（11 件）と NFR1〜NFR5（5 件）
- 受け入れ基準（AC）: User Stories はこの scope で実行されないので、対象外
- トレーサビリティ: ステージ単位の `construction/code-generation/traceability.json` だけ（Unit の分割は無い）

## ID ごとのカバー状況

| ID | 状態 | 担当（ステージ / Unit） | 対象ファイル | ファイルの存在 |
| --- | --- | --- | --- | --- |
| FR1.1 | OK | code-generation / ステージ単位 | `modules/app/aidlc/tests/pipeline_link_contract.rs` | あり |
| FR1.2 | OK | code-generation / ステージ単位 | `modules/app/aidlc/tests/pipeline_link_contract.rs` | あり |
| FR1.3 | OK | code-generation / ステージ単位 | `modules/app/aidlc/tests/pipeline_link_contract.rs` | あり |
| FR2.1 | OK | code-generation / ステージ単位 | `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` | あり |
| FR2.2 | OK | code-generation / ステージ単位 | 同上 | あり |
| FR2.3 | OK | code-generation / ステージ単位 | 同上 | あり |
| FR3.1 | OK | code-generation / ステージ単位 | 同上 | あり |
| FR3.2 | OK | code-generation / ステージ単位 | 同上 | あり |
| FR4.1 | OK | code-generation / ステージ単位 | `modules/app/aidlc/tests/pipeline_link_contract.rs` | あり |
| FR4.2 | **Deferred（未カバー）** | code-generation / ステージ単位 | 理由のみ（ローカルの全体テストが無関係の要因で緑にならない） | — |
| FR5.1 | **N/A（未カバー）** | code-generation / ステージ単位 | 理由のみ（コードではなく GitHub Issue で満たす） | — |
| NFR1 | OK | code-generation / ステージ単位 | `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` | あり |
| NFR2 | **Deferred（未カバー）** | code-generation / ステージ単位 | 理由のみ（CI でしか判定できない） | — |
| NFR3 | **Deferred（未カバー）** | code-generation / ステージ単位 | 理由のみ（カバレッジをローカルで測っていない） | — |
| NFR4 | OK | code-generation / ステージ単位 | `modules/app/aidlc/tests/pipeline_link_contract.rs` | あり |
| NFR5 | OK | code-generation / ステージ単位 | `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` | あり |

## 未カバーの要素（承認ゲートで示す）

- **FR4.2**（既存スイートの緑）: この段で、コミット対象の状態を作業用のチェックアウトで検証した。スモーク用のローカル設定に由来する 1 件は解消した。一方、macOS ローカルで契約テストの子プロセスが断続的に SIGKILL で終わる件は、修正前でも同じ頻度で起きる。結果は `test-results.md` にある。
- **FR5.1**（射程外の所見を Issue に記録）: 起票済み。[#151](https://github.com/amadeus-dlc/amadeus-ng/issues/151)（同じ形の DEFERRED 2 か所）と [#152](https://github.com/amadeus-dlc/amadeus-ng/issues/152)（失敗文言に段を載せる）。要件の合否基準のうち「PR 本文から参照されている」は、PR を作る Deployment Execution で満たす。
- **NFR2**（CI の安定性）: 修正 PR の CI でしか判定できない。担当は Deployment Execution。
- **NFR3**（カバレッジ 90% の床）: この段でローカルの計測を試した。結果は `test-results.md` にある。
