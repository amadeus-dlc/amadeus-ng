# 状態・監査の2.7.1受入移行

## 変更した検査と成功範囲

`modules/core/read-model-updater/tests/audit_block_golden_test.rs` を2.7.1へ移した。42ファイルに含まれる監査は106ブロック・27イベント語である。旧版を読んだまま106件を要求すると70対106で失敗し、参照移行後は全ブロックの再描画がバイト一致した。イベント発生処理を全91語について実装したという主張ではない。

`modules/core/read-model-updater/tests/projection_golden_test.rs` も2.7.1へ移した。採取元SHAを検証し、旧版を読む状態で旧SHAと期待SHAの不一致を検出した。移行後は18件中9件成功・9件失敗となった。原本や比較対象の監査行は削除していない。

そのうち `Skip Kind` の欠落を、保存イベントの種類に従って `modules/core/read-model-updater/src/workspace/projection.rs` で是正した。通常の読み飛ばしは `conditional-runtime`、ジャンプによる読み飛ばしは `jump` を本家の位置へ描く。通常の読み飛ばしケースは状態・監査の両方が一致し、投影の既存41単体テストも成功した。

## 未完了の差

Skip Kind是正後も、新コーパスの投影テストは10件成功・8件失敗である。

| 対象 | 未実装・未裁定の観測差 |
| --- | --- |
| gate開始・改訂後の再提示 | センサー実行の監査。対象外指定との整合は `sensor-audit-comparison-questions.md` で人間へ確認中 |
| 工程承認・完了 | `Validation Basis`。グラフ、入力、出力の指紋等を保存事実から描く経路が必要 |
| 作業開始 | `Source Baseline`。固定値を投影へ埋めず、開始時の観測を保存する必要がある |
| 前方・後方ジャンプ | `Changed Upstream Artifacts` / `Invalidated Downstream Artifacts` 等。元の変更・無効化事実を運ぶ必要がある |

これらを正規化で消さず、実装や裁定が済むまで移行完了としない。現在の投影テストは従来どおり差分ハンクと補完文脈を使うため、CLI全体の同一初期条件・全ファイル比較の代わりにはしない。

## 実行証跡

- 移行前: 監査描画1件・状態投影18件成功。
- `audit_block_golden_test` の106件要求: 70対106でRed。参照変更後はGreen。
- `the_golden_corpus_is_where_the_test_thinks_it_is`: 旧採取元SHAでRed。参照変更後はGreen。
- `projection_golden_test`: 9成功・9失敗から、Skip Kind是正後は10成功・8失敗。
- `skipping_a_stage_moves_on_without_touching_the_completed_count`: 是正後に1件成功。
- `cargo test -p core-read-model-updater --lib workspace::projection`: 41件成功、ほか267件はフィルタ除外。

ログは `test-evidence/rmu-golden-*.txt` に保存した。未完了のテストを無視・削除していない。
