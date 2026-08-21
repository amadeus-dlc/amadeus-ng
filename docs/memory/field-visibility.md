# フィールドはデフォルト private — 公開はアクセサ経由

**裁定日**: 2026-08-22（オーナー、共通ルール）
**適用例**: フィールド可視性スイープ PR（`UnknownPhase(pub String)` 型の全面見直し）
**機械強制**: `cargo lint` ルール化予定（`no-public-fields`）

## ルール

- **構造体のフィールドはデフォルト private**。`pub struct X(pub String)` / `pub struct X { pub field }` のように内部構造をさらけ出さない。エラー型・値レコード・newtype も例外ではない。
- 読み取りは**アクセサメソッド**で公開する。命名は既存の house style に従う:
  - `String` フィールド → `pub fn as_str(&self) -> &str`（単一 String newtype）/ `pub fn message(&self) -> &str`（メッセージ担体）/ フィールド名そのまま（`pub fn scope(&self) -> &str` 等。`get_` 接頭辞は付けない）
  - `Vec<T>` → `&[T]`、`Copy` 型 → 値返し、その他 → `&T`
- 定義モジュール外から構築が必要な型（Gateway が組み立てる値レコード・テストが構築するエラー値）には `pub fn new(..)` を与える。不変条件を持つ型は従来どおり private ctor + `parse`（Always Valid）。
- **enum の変種フィールドは言語仕様上 private にできない**ため本ルールの対象外（変種フィールドの公開が問題になる型は struct への昇格を検討する）。
- `pub(crate)` は同一クレート内の実装詳細共有にのみ許す（既定はやはり private）。

## 根拠

内部構造の直接公開は表現の変更（フィールド追加・型変更・不変条件導入）を破壊的変更にし、Tell-Don't-Ask 違反（[tell-dont-ask.md](tell-dont-ask.md)）の入口になる。アクセサ経由なら表現を変えても契約面が保たれる。
