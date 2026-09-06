# unit-test-instructions — U1 正準JSON

> 対象: u1-canon-json-goldens。現行code-generation-planとTesting Contract、NFR2.1〜NFR2.3、
> nfr-design/logical-components.md §4に従う。以下はすべて本Unitに限定する。

## 1. ランナーと設定

Rust標準のcargo testを使い、追加のランナーや設定は導入しない。Rustのバージョンはrust-toolchain.toml、依存はCargo.lockを正本とする。proptestは既存の開発依存で、シード20260823を明示する。serde_jsonのpreserve_orderとfloat_roundtripを維持する。

単体・PBTはmodules/core/infrastructure/src/canon_json/、結合試験は同クレートのtests/golden_hash_canonical.rsとgolden_corpus_read.rs、比較器はtests/support/mod.rsにある。

## 2. Unit限定コマンド

ワークスペースルートで実行する。単体試験のcanon_jsonフィルタは同クレートの別機能の試験を除く。ゴールデンは対象ファイルを明示し、rustdocはcanon_jsonを指定する。

```sh
PROPTEST_RNG_SEED=20260823 cargo test --locked -p core-infrastructure --lib canon_json
PROPTEST_RNG_SEED=20260823 cargo test --locked -p core-infrastructure --test golden_hash_canonical --test golden_corpus_read
PROPTEST_RNG_SEED=20260823 cargo test --locked -p core-infrastructure --doc canon_json
```

今回の計画準備時の結果は/tmp/u1-plan-unit-baseline.log・/tmp/u1-plan-golden-baseline.log・/tmp/u1-plan-doc-baseline.logに記録し、実行担当が結果を確認してcode-summaryへ日付・件数・対象を残す。ログが失われた場合は上記コマンドで再確認する。ソースコメント更新後はrustdocを含め、同じコマンドで確認する。

## 3. 合格基準と検証範囲

各コマンドが成功し、対象試験が0件に減っていないことを確認する。現行単体・PBTは87件、ゴールデンは7+9件で、受入表の32行を3プロファイル・2族で比較する。rustdocの件数は実際の出力から記録する。期待値を書き換えて成功させない。

同一入力の決定性と対象値域での出力安定性を検証し、大整数や非有限数を含む任意値の完全往復は要求しない。127/128段の境界・不正UTF-8・孤立サロゲート・型付き変換失敗を既存試験で確認する。

ワークスペースのカバレッジ床は90%、相対差の許容は0.01ポイント。閾値・除外を緩和しない。Unit試験の成功は全体カバレッジ、全CLI経路、最新依存検査、性能測定の代替ではない。全体検証をUnitごとに繰り返すコマンドはここへ置かない。

## 4. データとテスト支援

採取済みデータはtests/golden/upstream-3c3146cf/を読取専用で使用する。パス解決は既存tests/support/mod.rsの実装を利用し、旧クレート配置の相対パスをコピーしない。正規化は期待値と実測値の双方へ同じ規則を適用する。

変換処理の試験に外部ネットワークやモックは不要。CLI/フックはコーパス読取・正規化・欠落理由の確認であり、実際の全経路実行とは区別する。既存の一時ファイル利用やPBT生成器を再実装しない。

## 5. 失敗時

失敗名・コマンド・出力を記録する。コメント修正以外の機能欠陥なら、対象レイヤーの再現試験を先に用意する変更案を親セッションへ返し、計画を更新する。今回の記録是正のために実装を壊して人工的なRedを作らない。
