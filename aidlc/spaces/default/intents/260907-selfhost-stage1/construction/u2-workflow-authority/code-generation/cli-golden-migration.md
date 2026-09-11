# CLI比較の2.7.1移行

2026-09-09。`modules/app/aidlc/tests/cli_golden_test.rs` の参照を固定本家 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` へ移した。原本の期待バイトは変更していない。

## 変更と結果

- 採取元SHAのテストは旧版 `3c3146cf` を検出してRedになり、参照変更後にGreenになった。
- run-stage比較には同じ配布persona本文を配置した。旧テストの `conductor_persona` 欠落はfixtureにファイルを置いていなかったことによる。既存実装が本文を読み、全文が一致する。実装不足としての修正は行っていない。
- narration / conductor_personaを既知欠落として許すキー集合の例外を削除した。ただし合成グラフを使うため、実行指示全体の値が本家の元fixtureと一致するという検査ではない。
- parkはnarrationの実装欠落をRedで確認した。`presenter.rs` のparked描画に本家の固定説明文を追加し、kind/reason/stage/narrationを含む公開JSON全文がバイト一致した。Presenterの既存12件も成功した。
- reportの5ケースを別テストへ分け、先の失敗で承認・差戻し・改訂の比較が未実行になる問題を解消した。

現在は **10件中9件成功・1件失敗**。残りは承認待ちの再報告である。報告系は従来のslug置換を使う限定検査であり、同一初期状態・全公開ファイルの比較へ移したという主張ではない。

## 残る差

本家 `aidlc-orchestrate.ts:8064` は再報告時に `gate evidence revalidated` と返す。その前に `spawnState` でgate-startを実行し、根拠を再検証する。nativeは現在、単純なAlreadyAwaitingのno-op結果を返す。

再検証を行っていないまま表示文言だけを本家へ揃えない。成果物・レビュー等の検証と、そのうち対象外のセンサー監査をどう扱うかを整理する必要がある。失敗テストを削除・無視していない。

## 証跡

`test-evidence/cli-pin-red.log`、`cli-persona-green.log`、`park-narration-red.log`、`cli-golden-migration.log`、`presenter-regression.log`。

定期 `cargo lint` は19回目まで成功。最新のコード断面では対象5crateの全target Clippyも成功した。今回の検査範囲でStep 4・8全体の完了とはしない。

## 旧参照の残存確認

modules配下のRustを再検索し、U1一覧で実際に旧コーパスを読んでいた参照の移行を確認した。残るupstream-3c3146cf文字列は、誤正規化を防ぐテスト入力と歴史的な自律実行の採取注記である。旧版を現行の正本と読めるモジュール冒頭の注記は用途を訂正した。原本の意味を確認できたharness.json参照は新しいdata配下へ直した。個別の旧ソース行番号を全面的に書き換えたという意味ではない。
