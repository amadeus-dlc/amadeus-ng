# U9 再構成方式の実測検証

## 判定

**VERIFIED** — 現在の IntentExecutionRepositoryImpl は、保存されたスナップショットを基底として、それより後のイベントを再生する。全イベントの読取りを必須としていない。

実測後の 2026-09-05 オーナー裁定により、最新スナップショットとそれ以降の差分イベントによるリプレイが正しい方式と確定した。再生方式については現実装を維持し、古い全再生の規則を訂正する。レビュー開始時の保存判定の不整合は、この検証では扱わない。

## 再現条件

- 実施日: 2026-09-05。
- コード基準: `537c4e56a838a4cb28f6564d4c0add1d4adfe915`。本検証でコードとテストを変更していない。
- SQLite の一時 DB に同じ seed（genesis と 2 コマンド、計 3 イベント）を作り、テストごとに独立した DB で条件を変える。
- 既存の契約テストと実装固有テストを `--locked` で実行した。テストの成功は下表の assert が実行時に成立したことを示す。

```sh
cargo test --locked -p core-command-interface-adapter \
  --test intent_execution_repository_contract \
  --test intent_execution_repository_impl_test
```

実測ログはローカルの `repository-tests.log` に保存した（リポジトリには含めない）。再現コマンドと各条件の試験名は本書に記録する。契約テスト 20 件（memory / SQLite）、実装固有テスト 23 件、合計 **43 件成功・0 件失敗・0 件無視**。全ワークスペースのテストは実施していない。

## 対照と条件変更

| 条件 | 実行時に検証された結果 | 判別できること |
| --- | --- | --- |
| 対照: genesis のスナップショットと 3 イベントが存在 | `seq_nr = 3`、期待する最新状態と一致 | 基底と後続イベントで最新状態になる |
| 変更: 同じ seed 後に journal の全行を削除 | 読取成功、`seq_nr = 1`、`version = 3` | イベント列を全再生することは必須ではなく、保存状態から復元する。イベント欠落時には状態が古くても読める |
| 変更: genesis 行の manifest を未知の型へ変更 | 読取成功、期待する最新状態と一致 | 基底以前のイベントは読取・型検査の対象外 |
| 変更: 差分にあたる `seq_nr = 2` の行を削除 | `RepositoryError::Corrupt`、`seq_nr = Some(3)` | 基底以後のイベント欠落は拒否する |
| 変更: snapshot 行を全削除し journal を残す | `RepositoryError::Corrupt`、原因は `missing snapshot` | この実装は journal だけから状態を再構成しない |

対応するテストは `modules/core/command/interface-adapter/tests/intent_execution_repository_impl_test.rs` 内の次の関数である。

- 対照: `a_stale_snapshot_plus_delta_matches_the_freshest_state`
- journal 全削除: `a_snapshot_alone_is_a_sufficient_rehydration_base`
- 基底以前の破損: `a_foreign_manifest_before_the_snapshot_base_is_not_read`
- 差分欠落: `a_gap_in_the_delta_rows_is_corrupt_not_a_crash`
- snapshot 欠落: `a_journal_without_a_snapshot_is_corrupt_not_missing`

## 解釈と限界

全再生か差分再生かの判別は、コメントや API の形ではなく、保存データを変更した条件で成立する assert に基づく。正常系が通るだけでは両方式を区別できないため、全イベント削除・基底以前の破損・差分欠落を合わせて確認した。

検証時には `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/aggregate-commands.md` に全再生指定が残っていた。実測は動作の証拠、2026-09-05 のオーナー裁定は採用方針の根拠として区別する。同裁定に従い、aggregate-commands と ubiquitous-language の旧指定を訂正した。journal 全削除時の古い状態への復元は観測結果であり、欠落を許容すべきという新たな承認ではない。
