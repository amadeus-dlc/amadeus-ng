# amadeus-ng ポリシー

amadeus-ng（AI-DLC Workflows の Rust 再実装）で、計画・テスト・実装・レビューのすべてに適用する。

## 原則

| 原則 | 基準 |
|------|------|
| コーディング規則に従う | 正本は `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（1 規則 1 ファイル）。まず `README.md` の一覧を読み、変更する層に関係する規則の本文を読んでから書く。規則どうしが衝突したときの優先順も `README.md` にある |
| 観測互換を守る | upstream 互換の観測可能な契約（ゴールデン `tests/golden/upstream-a277af21/` と `vendor/aidlc-workflows/`）は設計規則より優先する。逐語の文言・出力は変えない |
| 後方互換を残さない | 改名や署名の変更では呼び出し側をすべて一度に直す。`#[deprecated]`・旧名の別名・互換のための口を残さない（`no-backward-compatibility.md`） |
| CI と同じ検査を通す | 完了と報告する前に、下の「必須の検査」をすべて実行し、通ったことを結果の出力で示す |
| 範囲を広げない | 指示された範囲だけを変える。範囲外で見つけた問題は直さず、報告に書く |
| AI-DLC の記録を作らない | このリポジトリの開発では AI-DLC のワークフロー（`/aidlc`、`aidlc/spaces/*/intents/` の intent・ステージ・ゲート）を使わない。コーディング規則の README に「intent 記録へ残す」とある箇所は、PR 本文へ書くことで代える |

## 必須の検査

変更を完了とする前に、リポジトリのルートで次を実行する（CI の `check` ジョブと同じ）。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo lint
cargo test --workspace
```

`tools/lint/` を変更したときは、次も実行する。

```sh
cargo fmt --manifest-path tools/lint/Cargo.toml --all --check
cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/lint/Cargo.toml
```

`formal/`（Quint）を変更したときは `bash scripts/quint-gate.sh` も実行する。

- `cargo lint` はコーディング規則を機械的に強制する独自のリンター（`tools/lint`）。省略しない（省略して CI で落ちた前例がある）。
- macOS でワークスペース全体のテストを回すと、子プロセスが signal 9 で落ちることがある（毎回別のテスト・単独では通る）。落ちたテストは単独で流し直し、通れば既知の揺れとして報告に書く。複数のワークスペーステストを並行して走らせない。

## 禁止事項

- 検査を通すために、テスト・lint の規則・カバレッジの基準を弱めること。
- `aidlc/spaces/*/intents/` 配下のファイルを作る・変えること。
- `.claude/settings.json` に aidlc 以外のフックを書くこと（doctor の Native hook bindings が失敗する）。
