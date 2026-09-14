# U4 のテスト手順

## 実行基盤と最初のコマンド

既存 Rust workspace の Cargo / tokio / tempfile、`tests/support/tool_link.rs`（バイナリ配置）と `tests/support/coverage_profile_env.rs`（計測下の `.profraw`）を使う。CLI・フックの契約は Cargo がビルドした `aidlc` を子プロセスで呼ぶ。bun 側の検査は既存の `scripts/aidlc-sync.test.ts` を使う。

最初に既存の本家契約テストを実行してランナーを確認する。

```bash
cargo test -p aidlc --test upstream_271_contract
```

U3 マージ前の workspace 計測は 3,666 件成功・行カバレッジ 98.0666%。これは U4 実装後の成功を意味しない。新規テスト target は承認後に先に追加し、意図した振る舞いで失敗したことを確認してから実装する。未検出・依存不足・構文不正を Red の代わりにしない。

## 単位限定のコマンド

| コマンド | 対象 |
| --- | --- |
| `cargo test -p aidlc --test harness_binding_contract` | フック接続: 配布の登録形から `aidlc hook <name>` が起動し、stdin 受渡し・終了コード・stdout/stderr が配布 `.ts` と同契約であること。未知フック名の拒否。native に無い 2 本を native 扱いしないこと |
| `cargo test -p aidlc --test required_surface_contract` | 必要集合: bugfix 9 ステージの実バイトから導いた動詞・フック・受領記録の列挙が、配線済み / 未配線 / 配布 TS のまま、を取り違えないこと |
| `cargo test -p aidlc --test read_only_reuse_contract` | 読取り専用の再利用: `review-brief` の `summary` / `review` / `context`、`testing-posture` の `render` / `fingerprint` の前後で、状態ファイル・監査シャード・ストア・承認受領が不変であること。除外操作が混ざらないこと |
| `cargo test -p aidlc --test host_binding_contract` | ホスト識別: タグ・コミット・バイナリ実体の対応、復帰手順、準備完了を達成と取り違えないこと |
| `bun test ./scripts/aidlc-sync.test.ts` | 追加した同期パッチを含めて `--check` が成功すること |

これらは計画段階では未作成である。必要な dev-dependencies は各所有 crate の `Cargo.toml` へ追加し、既存ランナーを使う。テストを引数フィルターで絞る場合も、実行件数が 0 になっていないことを確認する。

## 既存契約の回帰コマンド

U4 が触れる境界の既存検査を対象ファイルで実行する。

```bash
cargo test -p aidlc --test upstream_271_contract
cargo test -p aidlc --test claude_hook_contract
cargo test -p aidlc --test doctor_contract
cargo test -p aidlc --test next_branches
cargo test -p aidlc --test cli_golden_test
bun scripts/aidlc-sync.ts --check
bun test ./scripts/aidlc-harness.test.ts
```

workspace 全体・カバレッジ・Quint / ITF・CI・依存監査は、承認済み実行計画の共通検査として B3 統合前に行う。本手順の単位限定コマンドと区別する。

## 必須ケース

| 対象 | 正常と拒否/失敗 |
| --- | --- |
| フック接続 | 14 本の native 名で起動・stdin 受渡し・終了コード一致、未知フック名の拒否、`plan-approval-guard` / `run-sensors` を native 名で呼べないこと |
| 入力の保持 | 作業ディレクトリ・選択した作業記録・セッション・入力文字列が維持されること。シェルの文字列展開で入力を再解釈しないこと（C6） |
| 接続の失敗 | 接続が失敗したとき、別の更新実装へ切り替えて隠さず、失敗として返すこと（C6） |
| 必要集合 | 未配線の動詞を配線済みと数えないこと。配布 TS のままの動詞を native と数えないこと |
| 読取り専用 | 5 動詞それぞれで正本が 1 バイトも変わらないこと。除外操作（受領作成・承認開始・学習永続化・目録更新）が混ざらないこと |
| ホスト識別 | ホストとターゲットを取り違えないこと。復帰が検証済みホストへ戻すこと。未検証の版を安定版と呼ばないこと |
| 準備と達成の区別 | 準備完了・fixture テストの成功を、実地スモーク・自己診断・切替の成功証拠にしないこと（C8 `not_proof_of_completion`） |
| 同期パッチ | 追加差分を含めて `--check` が成功すること |

Standard の各コンポーネント 5–8 件を基準に、契約の拒否条件を優先する。実装と同じ計算を写すだけのアサートや、戻り値だけを観察するテストに偏らない。

## データ・代替処理・品質基準

一時ワークスペースへ配布資産（`settings.json` / フック `.ts` / `stage-graph.json` / scope ファイル）を置いて接続を検証し、本リポジトリの実ファイルは変更しない。macOS 採取と Linux CI の差はテスト側の正規化で吸収し、期待値を手修正しない。

**模擬の人間応答、承認保護の無効化、`runtime::run` の繰返しを、実地スモークの成功証拠にしない。** これらは接続の検証には使えるが、FR7・FR8 の達成証拠にはならない（C8）。

行カバレッジ絶対床 90.0%、seed `20260823`、既存 Quint 3 モデルと ITF を維持する。Red / Green の実行コマンド・失敗理由・終了状態を U4 記録へ残す。本リポジトリでの実地スモーク・自己診断成功・切替は後続工程で実施し、このテストの成功で置き換えない。
