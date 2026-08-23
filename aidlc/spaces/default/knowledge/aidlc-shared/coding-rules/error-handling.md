# エラーハンドリング様式 — モジュールごとの手実装エラー enum

**裁定日**: 2026-08-23（オーナー、FD Q1 = A）
**適用例**: Bolt B1 / B3 のエラー型（core-domain `CommandError` / `ApplyError` / `StartError` / `SnapshotError`、core-use-case `GraphReadError`）
**機械強制**: `missing_errors_doc` / `missing_panics_doc` / `unwrap_used` / `expect_used` deny（`Cargo.toml` workspace lints）。`thiserror` / `anyhow` 禁止は `cargo lint` ルール候補（赤例テスト必須）

## ルール

- ドメイン層・ユースケース層の失敗はモジュールごとの**手実装エラー enum** で表現する。
- `thiserror` / `anyhow` 等のエラーハンドリング外部クレートには依存しない。
- 各エラー enum は `std::fmt::Display` と `std::error::Error` を手実装する。
- `Display` は**材料**（ID・索引・状態・原因）だけを描く開発者向けの診断表示であり、利用者向けの逐語文言（upstream 互換面）はアダプタ層（message-catalog）が組み立てる — ドメイン層に文言を持ち込まない。
- 変種フィールドは材料のみ（`stage`, `actual`, `expected`, `path`, `cause` など）で、`String` の文言を運ぶ変種を作らない。
- fallible な公開関数には `# Errors` セクションを付ける（`missing_errors_doc` deny）。
- `# Panics` を要する公開関数は作らない（範囲は型で保証 — `StageIndex` 等）。

## 根拠

依存最小化と、エラー型をドメイン語彙に閉じ込める方針（Always Valid、設計監査 R4）。文言をドメインのエラー型に載せると、upstream 互換の逐語文言がドメイン層の変更理由になり、逐語契約の所在が二重化する。材料だけを運べば、文言の組み立て（および将来の多言語化・書式変更）はアダプタ層に閉じる。

## 対象外

- **アダプタ層の message-catalog**: 利用者向けの逐語文言そのものを保持するのはこの層の責務であり、本ルールの「文言を持ち込まない」対象外。
- **テストコードの `unwrap` / `expect`**: `clippy.toml`（`allow-unwrap-in-tests` / `allow-expect-in-tests`）で許容する。
