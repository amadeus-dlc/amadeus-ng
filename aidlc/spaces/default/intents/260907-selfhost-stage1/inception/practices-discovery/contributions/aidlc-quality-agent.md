**Collaborator:** aidlc-quality-agent

## Contribution

品質面の独立レビュー。対象は lead の4草案、依頼原文、現行の規則、CI設定および代表的なテストである。基準コミットは `HEAD` / `main` ともに `1dc727e00a26c27258d916cb3a5e0592c6c48c1c`。ほかの支援者の成果物は参照していない。今回、実装、テスト実行、カバレッジ再計測、GitHub上の最新結果確認は行っていない。

### 確認できた品質規則

- `team-practices.md` の `Methodology: tdd` と明示した `Ordering` は依頼に合致する。TDD（失敗するテストから実装を進める方法）の red → green → refactor は確定済みであり、既定の test-after へ戻さず、同じ内容を再質問しない。既存コードが多いことや工程の深さが Minimal であることは、この順序や外側の受入検査を省く根拠にならない。
- `scripts/coverage.sh:37` 以降の実値は絶対床90.0%、相対許容0.01パーセントポイント、`PROPTEST_RNG_SEED=20260823`、明示除外は `modules/app/aidlc/src/main.rs` のみ。草案の記述は正確である。これは workspace 行カバレッジの床であり、各クレートの床や分岐カバレッジ90%を意味しない。相対ゲートは `head >= base - 0.01`、絶対ゲートにはこの許容を適用しない。
- `.github/workflows/ci.yml` の `check` は workspace の fmt・clippy・独自lint・テストを実行し、workspace外の `tools/lint` にも fmt・clippy・テストを実行する。`aidlc-distribution` は同期検証と配布関連の Bun テストを別に持つ。Rustテストだけの成功では「CI全ジョブ成功」を満たさない。
- 同ファイルの `ci-success.needs` に `audit` は含まれない。草案が、集約チェックの成功と利用者の求める全ジョブ成功を区別している点に同意する。依存監査を必須チェックへ追加する変更は、この規則確認だけでは決めない。
- `scripts/quint-gate.sh` は3モデルの型検査・不変条件・到達性検査に加えて `stop_hook` の決定的シナリオを実行する。到達性検査は非ゼロ終了だけで成功とせず、出力の `[violation]` を確認し、解析等の失敗を区別する。こうした検査の強さを維持する。

### 継承する実テストの形

| 境界 | 現物 | 継承する規則と限界 |
|---|---|---|
| コマンドのユースケース | `modules/core/command/use-case/src/orchestration/test_support.rs:1` と `coding-rules/use-case-rules.md` §1・§2 | 依存方向を守るためクレート内 `#[cfg(test)]` のポート実装を使用する。この限定例外を根拠に他層へ自作ストアを増やさない。 |
| 永続化の契約 | `modules/core/command/interface-adapter/tests/intent_execution_repository_contract.rs` | 同じ契約群を本家memoryバックエンドと実SQLiteへ適用する。各ケースの一時ディレクトリを分離し、再オープン・古い版からの競合・異なる実行IDの拒否を検査する。 |
| 書込み・投影・読取り | `modules/app/aidlc/tests/intent_lifecycle.rs:492`、`crash_reconstruction_test.rs` | `runtime::run` を用いる統合検査と、永続化後の再構成検査を継承する。ドメイン単体の成功だけでCLIからの到達性を推定しない。 |
| モデルと実装の一致 | `modules/core/command/domain/tests/engine_loop_conformance.rs:392`・`:611`、`modules/app/aidlc/tests/journal_protocol_conformance.rs` | ITF（モデルの実行列を格納する形式）の各状態・遷移を実装と比較する。engine_loop はフィクスチャ最低件数と到達アクションも確認する。3モデルの存在が、3モデルすべてのRust実装との照合完了を意味するとは書かない。 |
| 公開出力の比較 | `modules/app/aidlc/tests/cli_golden_test.rs:1`、`modules/core/read-model-updater/tests/projection_golden_test.rs` | ゴールデン（比較用に採取した出力）のバイト一致と、キー集合だけの比較を区別する。可変値の扱いと既知の差分を明示した既存テストを、完全互換の証明へ拡大解釈しない。 |

上記は実コードから確認した検査パターンであり、今回すべてのケースが成功したという報告ではない。新規ケースの具体的な一覧は、要求分析で確定する呼出し・受領・拒否条件から作る。

### 草案へ補うべき検証上の限界

1. **ゴールデンの現行テストには意図的な差の扱いがある。** `cli_golden_test.rs:20` 以降は駆動できないケースを明記し、`:35` 以降は `conductor_persona` / `narration` の欠落を明示する。`load-steering` / `run-stage` の一部はキー集合を比べる。したがって既存ゴールデン検査が成功しても、切替条件2の完全な根拠にはならない。要求分析へ、比較の粒度・未検証ケース・明示された差を持ち越す。どの版を採用するか、どの差を今回実装するかは選ばない。
2. **既存の「縦切り」「プロセスをまたぐ」テストと実地スモークを区別する。** `steering_across_processes.rs:7` 以降は `runtime::run` の反復がプロセス内状態について同値であるとの検査前提を説明している。これは実OSプロセスを起動する検査ではない。また `intent_lifecycle.rs:191` の `append_human_turn` はフックの代わりに監査へ事実を置く試験装置である。これらを本リポジトリ上のreleaseバイナリ・Claude Code・実フック・人間の承認で通した証拠とは扱わない。main.rs のカバレッジ除外も、実地スモークの省略を認めるものではない。
3. **CIのイベントと対象コミットを証跡に残す。** 設定は push 起動を持たず、`pull_request` は相対カバレッジとレビュー検査、`merge_group` / `workflow_dispatch` は絶対床とレビュー検査の意図的スキップを使う。切替時には対象コミット・起動イベント・各ジョブの結果を確認し、集約成功だけで全条件を満たしたとしない。イベントにより元から適用されない検査を成功に書き換えず、適用理由とともに区別する。適用除外の扱いが切替判断に影響すると分かった場合は人間へ提示する。

### 質問と後続工程への引継ぎ

品質規則について、この時点で追加の必須質問はない。TDD、90%床、Quint・ITF・ゴールデン維持、CI全ジョブ成功、実地スモークは既に明示されている。

最小の通し経路を先行するかは未決の作業順であり、どちらを選んでも完了時の実地スモークは必要である。ゴールデンの版差、既知の出力差、必要な動詞・フック・受領の集合、バイナリと配布補助の担当範囲は要求分析で裁定する。対象外とされた課題の修正を、検証の一般論だけで追加しない。

## Positions

- AGREE: TDDの方法・順序を明示し、90%床・相対ゲート・既存品質検査を維持する草案は、利用者の指示と現行設定に一致する。
- AGREE: CI集約に入らない依存監査も今回の全ジョブ成功条件に含め、集約成功だけで完了としない扱いは妥当である。
- AGREE: 採用するゴールデンの版と実装範囲を要求分析へ残し、この工程で決めない。
- OBJECT: evidence.md は既存ゴールデンの比較粒度・既知の欠落と、runtime呼出しによる統合テストの限界をまだ記録していない。上記1・2を補い、既存検査の成功が実配布資産の完全互換や実地スモーク成功を証明するとの誤読を防ぐ。
