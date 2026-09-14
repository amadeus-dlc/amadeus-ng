# U3 のテスト手順

## 実行基盤と最初のコマンド

既存 Rust workspace の Cargo / tokio / tempfile と、既存の SQLite イベントストア、U2 が着地した `RecordHealthCheckUseCase` と `HEALTH_CHECKED` 投影を使う。最初に U2 の診断記録 API の結合テストを実行してランナーを確認する。

```bash
cargo test -p aidlc --test diagnostic_record_contract
```

B1 マージ時点の workspace 検査は 3,599 件成功。これは U3 実装後の成功を意味しない。新規テスト target は承認後に先に追加し、意図した振る舞いで失敗したことを確認してから実装する。未検出・依存不足・構文不正を Red の代わりにしない。

## 単位限定のコマンド

次の新規 integration test target を追加する。通常のテストは実ファイル・実 SQLite・公開 API を使い、CLI の契約は Cargo がビルドした `aidlc` を子プロセスで呼ぶ（バイナリの配置は `tests/support/tool_link.rs`、計測下の `.profraw` は `tests/support/coverage_profile_env.rs`）。

| コマンド | 対象 |
| --- | --- |
| `cargo test -p core-query-interface-adapter --test doctor_observation_contract` | 観測 DAO: Bun / settings / フック / workspace shell / scope・graph・schema / 状態版 / ストア / 投影の不在・不読・不正の区別 |
| `cargo test -p core-query-use-case --test doctor_report_contract` | 判定 use case: D1–D5 の行・ラベル・原因、初回の非適用、advisory、対象外行の不在、集計と終了コード |
| `cargo test -p aidlc --test doctor_contract` | `aidlc --doctor` の入口・書式・終了コード・副作用（DC1・DC2・DC10）と corpus 駆動の DC3–DC9 |
| `cargo test -p aidlc --release --test doctor_contract` | release バイナリによる同じ契約の確認 |

これらは計画段階では未作成である。必要な dev-dependencies は各所有 crate の `Cargo.toml` へ追加し、既存ランナーを使う。テストを引数フィルターで絞る場合も、実行件数が 0 になっていないことを確認する。

## 既存契約の回帰コマンド

U3 が触れる境界の既存検査を対象ファイルで実行する。

```bash
cargo test -p aidlc --test diagnostic_record_contract
cargo test -p aidlc --test upstream_271_contract
cargo test -p aidlc --test next_branches
cargo test -p aidlc --test cli_golden_test
cargo test -p core-read-model-updater --test hook_health_projection_contract
cargo test -p core-read-model-updater --test audit_block_golden_test
cargo test -p core-query-interface-adapter --test read_model_dao_contract
```

workspace 全体・カバレッジ・Quint / ITF・CI・依存監査は、承認済み実行計画の共通検査として B2 統合前に行う。本手順の単位限定コマンドと区別する。

## 必須ケース

| 対象 | 正常と拒否/失敗 |
| --- | --- |
| 入口 | `aidlc --doctor` の受理、追加引数の拒否、`next --doctor` の既存 print directive の維持 |
| D1 | Bun 所在の成否、実行中バイナリの入口対応（必要面の欠落・評価不能） |
| D2 | settings 不読 / 参照なし / JSON 不正 / managed-only / disableAllHooks の優先順 / フック欠落 / 接続不一致、heartbeat の初回・進行後未発火・不読・300000ms 境界（境界値は失敗にしない） |
| D3 | workspace shell 不足、scope の error と advisory の区別、循環、必要ステージ欠落、schema 不正、参照不正 |
| D4 | 状態なしの初回（非適用）、状態不読、版欠損・過去版・未来版、識別不整合 |
| D5 | ストア不在 / 不読 / schema 不正、投影欠落・不整合 |
| 書式・終了 | ヘッダ、`─`×37、`✓  ` / `✗  `、`<passed> passed, <failed> failed`、LF、stderr 空、`failed=0` → 0 / それ以外 1 |
| 副作用 | 監査なしで何も作らない（DC1）、監査ありで `HEALTH_CHECKED` 1 件（DC2）、記録失敗で終了 1（DC10） |
| 本家対応 | corpus 14 観測の採用行とのバイト一致、非採用行を出さないこと |

Standard の各コンポーネント 5–8 件を基準に、契約の拒否条件を優先する。実装と同じ計算を写すだけのアサートや、戻り値だけを観察するテストに偏らない。

## データ・代替処理・品質基準

`tests/golden/upstream-a277af21/doctor/cases.json` と来歴・正規化規則を入力にする。期待出力の手修正や固定文字列を消す正規化は行わない。macOS 採取と Linux CI の差（パス接頭辞、ロック診断文言）はテスト側の正規化で吸収する。一時ワークスペースへ配布資産（`stage-graph.json` / `scope-grid.json` / `harness.json` / scope ファイル / フック `.ts`）を置いて異常を注入し、本リポジトリの実ファイルは変更しない。

行カバレッジ絶対床 90.0%、seed 20260823、既存 Quint 3 モデルと ITF を維持する。Red / Green の実行コマンド・失敗理由・終了状態を U3 記録へ残す。本リポジトリでの doctor 成功と切替は後続で実施し、このテストの成功で置き換えない。
