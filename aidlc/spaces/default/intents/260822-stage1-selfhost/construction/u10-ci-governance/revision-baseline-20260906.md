# U10 CI・品質管理の改訂基準

## 位置付け

2026-09-06のmain `53b5667e52ed7d28a395458afb3fe254911b1b45`とGitHub rulesetの実測を基準とする引継ぎメモ。
要件・設計・実装記録の3つの`pending-revision.md`に残る2026-08-23の案より、現状の記述には本メモを優先する。
これは未回答の要約確認や未実施の独立レビューを置き換えるものではない。

## 改訂時に維持する事実

| 対象 | 現行設定と改訂方針 |
| --- | --- |
| 必須チェック | `check`・`quint`・`coverage`・`CI Success`の4つ。strict有効。 |
| 集約条件 | `aidlc-distribution`・`check`・`quint`・`coverage`の成功を必須とする。`pull_request`ではreview-threadの成功も必須。`merge_group`と`workflow_dispatch`ではreview-threadのskippedを必須とする。 |
| audit | workspaceと`tools/lint`の2つのCargo.lockを検査する。必須チェックおよびCI Success集約の対象外。失敗や未実施を成功と記載しない。 |
| 権限 | workflow既定は`contents: read`。review-threadジョブに限り`checks: write`・`statuses: write`・`issues: read`・`pull-requests: read`も付与する。 |
| 信頼境界 | review-threadの外部再利用ワークフローはSHA固定。配布物検証のcheckoutとBun導入もSHA固定。全ActionがSHA固定とは記載しない。トークンは秘密情報として扱う。 |
| カバレッジ | 絶対床90%、相対許容差0.01ポイント、固定シード20260823、除外は`modules/app/aidlc/src/main.rs`のみ。過去の暫定0.05へ戻さない。 |
| 再現性 | 設定・過去の結果と、同一リビジョンの2回実測を区別する。追加測定を行っていない時点で差0.00ポイント達成とは記載しない。 |
| ツールチェーン | Rust 1.95.0。`toolchain-inputs.sh`で正本ファイルからchannel/componentsを抽出してCI Actionの入力へ渡す。 |
| 品質検査 | workspaceと独立した`tools/lint`の双方にfmt・Clippy・テストを実行し、workspaceには`cargo lint`も実行する。 |

正本は`.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`scripts/governance/`。
`bash scripts/governance/verify-ci-governance.sh --with-ruleset`は今回20項目成功・失敗0。
この静的検査・ruleset検査だけで全受入条件を達成したとはみなさない。

## 後続で更新する文書

- 要件: `nfr-requirements/security-requirements.md`、`tech-stack-decisions.md`、`traceability.json`。NFR2.6のレビュー条件、正常系のマージキュー完走、権限と秘密情報の扱いを整合させる。
- 設計: `nfr-design/security-design.md`、`traceability.json`。配布物検証を含む集約条件、4コンテキスト、相対許容差0.01、信頼境界を整合させる。
- 実装記録: `code-generation/code-summary.md`、`traceability.json`。実ファイルと現在の検査範囲へ合わせ、targetに説明を混在させない。

以前の改訂案にあるレビュー所見は、古い要求値と切り分けて再評価する。今回のメモ追加で所見を解決済みにしない。
