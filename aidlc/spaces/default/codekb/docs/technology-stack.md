# technology-stack — 言語・フレームワーク・ライブラリ

> リバースエンジニアリング成果物（2026-08-22 実施、`c4d8d95` 時点）。バージョンは `Cargo.lock`（コミット済み）からの実測値。

## 言語とツールチェーン

- **Rust** — edition 2024 一律、Cargo workspace resolver = "3"。全クレート version 0.1.0、ライセンス `MIT OR Apache-2.0` デュアル（workspace 既定）。
- `rust-toolchain.toml` は**存在しない**（実地確認）— ツールチェーンのバージョン固定は現状なし。edition 2024 のため実質 Rust 1.85 以降が前提（推定）。
- フォーマット: rustfmt（`rustfmt.toml` — max_width 100 / style_edition 2024）。
- lint: clippy（workspace lints で約 50 ルール deny。`clippy.toml` でテストのみ unwrap/expect 許可）+ カスタムリンタ `amadeus-lint`（`.cargo/config.toml` の `lint` エイリアス）。

## ランタイム依存（プロダクトコード）

| ライブラリ | バージョン | 用途 | 利用箇所 |
| --- | --- | --- | --- |
| serde（derive） | 1.0.229 | ワイヤ構造体のデシリアライズ | core-interface-adapter のみ（ドメインは serde 非依存が規約） |
| serde_json | 1.0.151 | PL 3 入力の JSON 読取 | core-interface-adapter |
| md5 | 0.8.1 | ロック dir 名 `md5(identity)[..8]` の upstream 互換導出 | core-interface-adapter |
| nix（signal, fs） | 0.30.1 | シグナル・fs 系 syscall の safe wrapper | infra-io |
| libc | 0.2.189 | 低水準定数・型 | infra-io |

`#![forbid(unsafe_code)]` は infra-io を含め維持されている（unsafe は nix / libc の safe wrapper 経由に封じ込め）。

## 開発・テスト依存

| ライブラリ | バージョン | 用途 |
| --- | --- | --- |
| proptest | 1.11.0 | Property-Based Testing（集約の PBT） |
| tempfile | 3.27.0 | FS 系統合テストの一時領域 |
| syn | 2.0.119 | amadeus-lint の構文解析 |
| proc-macro2 | 1.0.107 | amadeus-lint |

## 検証・CI ツール

| ツール | バージョン | 用途 |
| --- | --- | --- |
| Quint（`@informalsystems/quint`） | 0.32.0 | 形式モデル検査（CI quint ジョブ、Node 22 上で実行） |
| cargo-llvm-cov | （CI 導入） | カバレッジ計測（絶対 90% 床 + PR 相対ゲート 0.5pp） |
| GitHub Actions | — | `.github/workflows/ci.yml` 1 本（check / quint / coverage の 3 ジョブ） |

## 不在（計画のみ・意図的）

以下は**意図的に未導入**であり、欠落ではなく計画済みの空白である:

- **async ランタイム**（tokio 等）— ワンショット CLI のため現状不要。プロセス実行基盤は A4 で確定予定。
- **CLI パーサ** — `aidlc` バイナリ自体がスタブ（フェーズ A）。
- **`tracing` / OpenTelemetry** — ADR 0004 で確定済み・未導入。
- **正準 JSON シリアライザ** — ADR 0001 で確定済み、`canon-json` はスタブ。
- **bun / Node ランタイム依存** — プロダクトからは排除済み（D1 の核心）。Node 22 は CI の Quint 実行にのみ登場する。

## バージョン管理上の観察

- メインの `Cargo.lock` に syn の 2 系と 3 系が併存する（推定: zerocopy-derive 由来の間接依存）。実害は未確認。
- `tools/lint` は独自 `Cargo.lock` を持ち、依存解決がメイン workspace から独立している。
