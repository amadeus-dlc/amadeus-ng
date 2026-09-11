# U2への引継ぎ: 旧ゴールデン参照と比較粒度

## 調査範囲

2026-09-08、現作業ツリーの `modules/` を `rg -n 'upstream-3c3146cf|supplemental-3c3146cf|v2\\.6\\.40'` と `rg -n '3c3146cf|v2\\.6\\.40'` で調査した。Rustソースは変更していない。旧資料の内容を復元する作業ではなく、FR1の受入移行で変更が必要な現コードの一覧である。

## 実際に旧コーパスを読む境界

| ファイルと行 | 現在の比較 | U2で確認すること |
| --- | --- | --- |
| `modules/app/aidlc/tests/cli_golden_test.rs:49` | 旧CLIを読む。`continue/invalid-token` はstdoutのバイト比較。load-steering/run-stageはキー集合。parkは3値とキー集合。reportはステージslugを置換して比較 | 同じ2.7.1配布条件で各面の比較を行う。キー集合の一致を内容一致へ拡大解釈しない |
| 同 `:235` / `:280` | run-stageのconductor_persona/narration、parkのnarration欠落を期待するテスト | 欠落を固定する期待を移行完了の成功条件にしない。2.7.1の実出力と対応実装を揃える |
| 同 `:19` / `:351` | stage-jump-print、completed-ungated、approved-across-phases、after-approvalを比較対象として駆動できていない旨を記載 | 本家2.7.1で必要な経路の前提を再現する。既存設計による到達不能と上流の仕様不一致は裁定対象として残す |
| `modules/core/command/interface-adapter/tests/golden_parity_test.rs:90` | 配布JSONのパース、ノード/スコープ件数、レビュー設定、格納・再読込、JSONバイトの再出力を検証 | データの参照と、旧実測定数・比較内容を合わせて移行。版名だけ更新しない |
| `modules/core/infrastructure/tests/support/mod.rs:228` | コーパスのrootと正規化設定の読込み | 新コーパスの前提・正規化・ID対応の契約に合わせる |
| `modules/core/infrastructure/tests/golden_corpus_read.rs:155` | 採取ファイルの有無、入力形、フック結果分類、来歴、必須範囲、正規化の固定点、欠落一覧 | 検査できることと実装適合を区別する。追加ケース/観測面/正規化規則に合わせる |
| 同 `:493` | `../supplemental-3c3146cf/cases.json` と旧SHA・ファイル数を直接検証 | 複数部配送/停止制御等の新規採取へ移行。既存のset-autonomy合成前提を実地成功と扱わない |
| `modules/core/infrastructure/tests/golden_hash_canonical.rs:22` / `:216` | 旧rootと完全SHAを固定。canonical/compact/pretty文字列とdigestを全行比較 | 新しい関数採取と来歴へ移行し、入力クラスの網羅性も維持 |
| `modules/core/read-model-updater/tests/audit_block_golden_test.rs:35` / `:123` | 旧audit.mdの非空ブロックを再描画し、時刻のみ正規化してバイト比較。70ブロック等の件数も固定 | 2.7.1の語彙・フィールド・順序・固定文言・実測件数へ合わせる。件数を下げて成功にしない |
| `modules/core/read-model-updater/tests/projection_golden_test.rs:84` / `:239` | 旧グラフ/グリッド、state.diffの前後断片、監査を読む。必要な断片外の行を補い、投影結果と比較 | 完全な初期状態と新データで同じ前提を再現。状態・監査・no-opの契約を確認 |
| 同 `:268` / `:518` | genesisは監査のみ比較し、scaffold missingも許容する。completed-ungatedは参照テストなし（`:352`） | この検査が証明しない状態全文やCLI経路を別の検証で満たす。U2のReportedイベント変更も既存投影の契約を保持 |

`golden_corpus_read.rs:431–432` の旧ピンは、固定文字列を誤って正規化しないことを検査する入力リテラルである。現行コーパスの参照パスとは意味が違うため、文字列の全置換で処理しない。

## 旧ピンを根拠とする本体コメント

以下は旧コーパスを実行時に読むことを意味しない。ただし、その操作を変更するときは2.7.1の該当根拠へ更新し、未検証のまま「現行互換」と表示しない。

- CLI: `modules/app/aidlc/src/{scaffold,presenter,wording,runtime}.rs`、`cli/request.rs`。
- コマンドのドメイン: `orchestration/{mod,intent_execution,gate_decision,review_verdict}.rs`、`orchestration/intent_execution_event/review_requested.rs`、`workspace/{mod,audit_events}.rs`、`workflow_definition/review_policy.rs`。
- コマンドのユースケース: `orchestration/review_log_kind.rs`。
- コマンドのアダプタ: `orchestration/compiled_definition_repository_impl.rs` とそのテストのharness.json根拠。
- RMU: `workspace/{wording,projection}.rs`、`orchestration/read_model_updater.rs`。

全体の説明を書き直す対象ではない。対象経路のソース・比較条件を再実測し、その根拠として必要な注釈だけを更新する。

## U1とU2の責任

U1は新コーパスと比較設定、旧新の出力差分を渡す。U2は本一覧の実装/検査を変更してFR1の適合を検証する。U1の採取成功や今回のRust基準2,354件成功は、旧参照の解消や新仕様適合の達成を表さない。

