# 技術スタック — amadeus-ng

## 言語・ツールチェーン

| 要素 | 版 | 根拠 | 用途 |
| --- | --- | --- | --- |
| Rust | toolchain `1.95.0` 固定、edition 2024 | `rust-toolchain.toml`、`Cargo.toml` の `[workspace.package]` | プロダクト全体 |
| Cargo ワークスペース | resolver 3、メンバー 10 クレート | `Cargo.toml` | ビルド |
| TypeScript（Bun） | — | `.claude/tools/*.ts`、`scripts/goldens/*.test.ts` | upstream 配布物の実行、ゴールデン採取 |
| Quint | 0.32.0 | CI の `quint` ジョブ | 形式モデル（`formal/`） |
| Python | — | `scripts/tests/` | 補助スクリプトのテスト（unittest） |

## ランタイムの主要ライブラリ

版は `Cargo.lock` の解決値。指定の幅は `Cargo.toml` の `[workspace.dependencies]` と各クレートの指定による。どのクレートがどれを使うかは `dependencies.md` を参照。

| ライブラリ | 指定 | 解決版 | 用途 |
| --- | --- | --- | --- |
| event-store-adapter-rs | `=3.0.0`（完全固定、`sqlite` feature） | 3.0.0 | 集約のイベントソーシング永続化（`EventStoreForSqlite` / `EventStoreForMemory`） |
| rusqlite | ワークスペース共通（`bundled`） | 0.40.2（libsqlite3-sys 0.38.2） | SQLite 接続。接続時に既定の busy timeout 5000ms を設定する（`inner_connection.rs:118`） |
| tokio | `1`（`rt`・`macros`。合成ルートは `process`・`time` も） | 1.53.1 | current_thread ランタイム |
| serde / serde_json | `1`（derive）/ `1`（`preserve_order`・`float_roundtrip`） | 1.0.229 / 1.0.151 | DTO と契約 JSON。契約 JSON の直列化は `canon_json` 経由に限る（`clippy.toml` で強制） |
| chrono | `0.4` | 0.4.45 | 発生時刻（`DateTime<Utc>`） |
| uuid | `1.26` | 1.26.0 | 識別子（合成ルートとドメインで v7 採番） |
| hmac / base64 / sha2 / getrandom | `0.12` / `0.22` / `0.10` / `0.3` | — | 封緘・ダイジェスト・乱数 |
| hostname | `0.4` | — | 監査シャード名のホスト部 |
| regex | `1` | 1.13.1 | Testing Posture の語句照合、テストの正規化 |
| libc / nix | `0.2` / `0.30` | 0.2.189 / 0.30.1 | `O_NOFOLLOW` などの OS 呼出、シグナル・ファイル操作 |

## テスト専用

| ライブラリ | 版 | 用途 |
| --- | --- | --- |
| proptest | `1` | 性質ベーステスト（`PROPTEST_RNG_SEED=20260823` 固定でカバレッジ計測） |
| tempfile | 3.27.0 | 一時ディレクトリ・ストア |
| Rust 標準テスト + `tokio::test` | — | 単体・契約テスト |
| Quint の ITF トレース準拠テスト | — | 形式モデルと実装の突き合わせ |

## ストレージ

- **SQLite**（rusqlite の `bundled` で同梱）。1 スペース 1 ファイル `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`。
- `journal_mode` の設定はリポジトリ内に無いので、既定のロールバックジャーナルで動く。WAL は使っていない。
- 並行制御は SQLite のトランザクションと集約の楽観 version（ADR-007）。

## 品質ツール

rustfmt（style_edition 2024、幅 100）、clippy（`-D warnings`）、`cargo lint`（`tools/lint`）、cargo-llvm-cov（`scripts/coverage.sh`）、`cargo audit`、markdownlint-cli2、CodeRabbit。運用の評価は `code-quality-assessment.md` を参照。
