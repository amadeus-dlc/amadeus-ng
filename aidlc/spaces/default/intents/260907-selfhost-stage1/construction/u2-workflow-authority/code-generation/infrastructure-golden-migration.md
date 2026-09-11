# 基盤ライブラリの受入参照移行

## 変更範囲

承認済み計画Step 8に従い、次の3ファイルの受入参照を本家2.7.1の固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` へ移した。製品コードと採取コーパスは変更していない。

- `modules/core/infrastructure/tests/golden_hash_canonical.rs`
- `modules/core/infrastructure/tests/golden_corpus_read.rs`
- `modules/core/infrastructure/tests/support/mod.rs`

ハッシュの全32行に対する出力・ハッシュの比較を維持し、外側の来歴だけでなく各ケースの採取元も固定コミットと照合する。CLI/フックの来歴、必要なケース、補完の複数部配送・自律モード設定・停止フックの観測は2.7.1のコーパスで検査する。自律実行機能を新しく実装したという意味ではない。

表示用の `normalization.json` では、未対応のセッションID・継続トークンを一律に消す規則が除かれている。旧期待の `<SESSION>` 一致を、入力中の識別子をそのまま保持する検査へ変更した。パス・時刻・明示されたcloneの置換検査は維持する。異なるIDを同値に潰す正規化を追加していない。

旧コミット文字列を含む `normalization_preserves_stable_record_names_and_upstream_pins` は、固定文字列を誤って正規化しないための入力なので保持した。旧コーパスへの参照としては使っていない。

## RedとGreenの証拠

2026-09-09（日本時間）に以下を実行した。Redは受入読取りが旧版を選んでいることを検出するものであり、製品ハッシュ関数の不具合を検出したという意味ではない。

| 検査 | 結果 |
| --- | --- |
| 移行前の `golden_hash_canonical` | 7件成功 |
| `the_corpus_carries_its_provenance` の採取元期待を2.7.1へ変更、読取り先は旧版のまま | 終了101。旧コミットと要求した固定コミットの不一致で失敗 |
| ハッシュの読取り先を2.7.1へ変更 | 7件成功 |
| `both_families_carry_their_provenance` の採取元期待を2.7.1へ変更、共通読取り先は旧版のまま | 終了101。同じ版の不一致で失敗 |
| 共通読取り先を2.7.1へ変更した初回 | 13件成功、1件失敗。旧テストの「セッションIDを一律に消す」期待が新しい表示規則と不一致 |
| 補完来歴の期待を2.7.1へ変更、補完パスは旧版のまま | 終了101。旧補完採取元との不一致で失敗 |
| 補完読取り先と表示規則の検査を是正した最終実行 | `golden_corpus_read` 14件、`golden_hash_canonical` 7件成功、失敗0 |

```bash
cargo test -p core-infrastructure --test golden_hash_canonical --test golden_corpus_read
cargo clippy -p core-infrastructure --all-targets -- -D warnings
rustfmt --edition 2024 --check modules/core/infrastructure/tests/golden_hash_canonical.rs modules/core/infrastructure/tests/golden_corpus_read.rs modules/core/infrastructure/tests/support/mod.rs
git diff --check -- modules/core/infrastructure/tests/golden_hash_canonical.rs modules/core/infrastructure/tests/golden_corpus_read.rs modules/core/infrastructure/tests/support/mod.rs
```

上記4コマンドはすべて終了0。ログは `test-evidence/infrastructure-golden-*.txt` に保存した。U2の他の旧コーパス消費者、全体カバレッジ、CI、実地スモークの完了証拠にはしない。
