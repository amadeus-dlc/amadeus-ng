# デプロイ戦略 — Issue #134 の修正

## 前提と上流の成果物

bugfix scope では CI Pipeline と Infrastructure Design を実行しないため、上流の `ci-config`・`quality-gates`・`infrastructure-specification`・`cicd-pipeline` は scope の設計どおり存在しない。経路と品質ゲートは、実在する `.github/workflows/ci.yml` とリポジトリ設定を根拠に `cd-config.md` にまとめた。

## 戦略

- **一括のマージ**を選ぶ。blue/green、canary、rolling は当てはまらない。デプロイ先の実行環境が無く、配布物が CLI だからである。
- **フィーチャーフラグは使わない。** 変更はトランザクションの開始 1 行で、切り替えて段階的に広げる対象が無い。フラグを足すと、DEFERRED と IMMEDIATE を選ぶ口が残ることになり、`no-backward-compatibility.md` に反する。

## 環境の昇格

| 段 | 何で確かめるか | 通過の条件 |
| --- | --- | --- |
| ローカル | Build and Test の作業用チェックアウト | 済み。ビルド・静的検査・修正の単体テストは緑。全体テストとカバレッジは macOS の SIGKILL で未確定のまま、人の判断で先へ進めた |
| PR の CI | `pull_request` イベントの全ジョブ | `ci-success` が緑。とくに `check` と `coverage` が再実行なしで緑であること（NFR2）。1 回目のコミット `1ec94b6f` では全ジョブ緑。改訂 1 を追加のコミットとして push した後の CI で改めて判定する |
| merge queue | `merge_group` イベントの必須チェック | `CI Success` が緑 |
| main | squash のマージ | merge queue が自動で行う |

## 本番（main）への承認

- 利用者は「CI greenならマージいいよ。タイミング任せる」と事前に承認した（質問 Q2 = X）。これが `org.md` Deployment にある「本番への手動承認」に当たる。進行役は、改訂 1 を載せた PR の必須チェックがすべて緑になったことを確かめてから merge queue へ入れる。緑でなければ入れない。
- PR 本文には次を書く。
  - Issue #134 を閉じること
  - 射程外の所見の Issue（#151、#152）への参照（要件 FR5.1 の合否基準）
  - ローカルで全体テストとカバレッジが確かめられなかった事情

## デプロイ後の確認（スモーク）

main に入った後、次の 2 点を確かめる。

1. main の先端で `cargo test -p core-read-model-updater --lib orchestration::journal_reader_impl::tests::replace_pipeline_waits_for_a_write_lock_held_by_another_connection -- --exact` が緑であること
2. main への push で走る CI（merge queue の `merge_group` の結果）が緑であること

NFR2（CI の安定性）は、PR の CI が再実行なしで緑になったかどうかで判定する。フレークの頻度は低いので、1 回の緑は十分な証拠にならない。その後の main の CI で #134 の症状（`projection: read: io: WouldBlock`）が再発しないかは、運用の中で見続ける。

## 中止の条件

- PR の CI で `check` または `coverage` が #134 の症状（`WouldBlock` と終了コード 1）で落ちた場合は、修正が効いていないとみなしてマージしない。Code Generation へ戻る。
- SIGKILL（`unix_wait_status(9)`）など別の症状で落ちた場合は、失敗の形を記録したうえで、再実行するかをオーナーが決める。
