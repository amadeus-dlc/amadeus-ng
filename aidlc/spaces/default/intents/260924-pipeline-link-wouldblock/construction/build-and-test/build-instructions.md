# ビルド手順 — Issue #134（`replace_pipeline` を IMMEDIATE に揃える）

## 前提

- Rust toolchain `1.95.0`（`rust-toolchain.toml` で固定。`rustup` が自動で入れる）
- 依存の取得にネットワークが要る（crates.io）。初回の `cargo build` が取得する
- 追加の環境変数・設定ファイル・ローカルサービスは不要。SQLite は `rusqlite` の `bundled` でビルドに含まれる

## 検証する状態

コミットの対象になる状態（`main` の先端 + 今回の変更）でビルドとテストを検証する。本体の作業ツリーには、コミットしないローカル変更がある（`.claude/settings.json` の run-sensors の起動形、`.codex/hooks.json` の削除）。ワークスペース全体のテストのうち 1 本（`engine_hook_wiring_contract`）は作業ツリーの `settings.json` を読むので、本体の作業ツリーでは結果がずれる。そこで次の手順で、ローカル変更を含まない作業用のチェックアウトを作ってから検証する。

```bash
# 本体のリポジトリルートで
git worktree add --detach <作業用ディレクトリ> HEAD
cp modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs \
   <作業用ディレクトリ>/modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs
cd <作業用ディレクトリ>
git status --short   # 差分が journal_reader_impl.rs の 1 件だけであることを確かめる
```

ビルドの出力先は、本体の `target/`（フックが使う `target/release/aidlc` を含む）と混ぜないように、`CARGO_TARGET_DIR` で別の場所を指す。

## ビルド

```bash
CARGO_TARGET_DIR=<別のディレクトリ> cargo build --workspace --all-targets
```

## ビルドの検証

```bash
CARGO_TARGET_DIR=<別のディレクトリ> cargo fmt --all -- --check
CARGO_TARGET_DIR=<別のディレクトリ> cargo clippy --workspace --all-targets -- -D warnings
CARGO_TARGET_DIR=<別のディレクトリ> cargo lint
```

## テスト

- この修正の単体テストは、`construction/code-generation/unit-test-instructions.md` のコマンドをそのまま使う（重複は 1 回だけ走らせる）。
- 既存スイートの緑（Testing Contract の scope floor）は `cargo test --workspace --no-fail-fast` で確かめる。
- カバレッジの床（90%）は `scripts/coverage.sh` で確かめる。閾値と除外の設定は変えない。

## よくある問題

| 症状 | 原因と対処 |
| --- | --- |
| `engine_hook_wiring_contract::the_binding_definition_counts_registrations_apart_from_hook_names` が `18` 対 `17` で落ちる | 作業ツリーの `.claude/settings.json` にコミットしないフック登録の変更がある。上の作業用チェックアウトで検証する |
| 契約テストの子プロセスが `unix_wait_status(9)`（SIGKILL）で終わり、出力が空 | macOS ローカルで断続的に起きる既知の現象（Issue #134 のコメントにある PR #138 の観測、修正前でも再現）。落ちたテストを単独で再実行して切り分ける |
| 初回ビルドが遅い | 別の `CARGO_TARGET_DIR` を使うので、依存を含めてフルビルドになる |
