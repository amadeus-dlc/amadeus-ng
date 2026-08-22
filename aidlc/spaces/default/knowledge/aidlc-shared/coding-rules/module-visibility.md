# モジュールはデフォルト private — 公開はファサードの `pub use` 経由

**裁定日**: 2026-08-22（オーナー、共通ルール。同日追補: 利便再エクスポート禁止）
**適用例**: モジュールファサード化スイープ PR
**機械強制**: 既存 `unreachable_pub`（deny 済 — 私有 mod 化により初めて実効化する。下記）+ `cargo lint` ルール化予定（コンテキスト直下より深い `pub mod` の検出）

## ルール

- **`mod` はデフォルト private**。`pub mod` の連鎖はファイル構成（型ごとのファイル分割）をそのまま公開 API に漏らし、内部整理が破壊的変更になる — モジュール性喪失の温床。
- **`pub mod` を許すのは名前空間として意味を持つ階層だけ**:
  - 境界づけられたコンテキスト（`core_domain::{workspace, orchestration, workflow_definition}` 等） — ユビキタス言語の所属を示す情報であり隠さない
  - 共有クレートの語彙名前空間（`message_catalog::{state, lock, bolt}`、`infra_io::{atomic, append_only, fs_meta, process_probe}`）
- コンテキストの**内側**の型ファイル mod は private にし、mod.rs（ファサード）で **`pub use` を意図的に列挙**する。mod.rs は「キュレーションされた公開 API 宣言」になる。

```rust
// workflow_definition/mod.rs
mod stage_slug;                       // private — ファイル構成は内部事情
pub use stage_slug::StageSlug;        // 公開 API はここで列挙
```

- **昇格の運用**: 利用者からの妥当な利用が発生したときに `pub use` へ追加する（先回りで全公開しない）。消費側のパスはコンテキスト直下（`core_domain::workspace::CheckboxState`）で安定し、ファイル分割の変更が非破壊になる。
- クレート内の兄弟コンテキストからの参照も**ファサード経由**（private mod は親サブツリー外から見えないため、自然に強制される）。
- **利便性のための再エクスポートはどこでも禁止**（オーナー裁定 2026-08-22）。`pub use` を書いてよいのは**所有コンテキストのファサード（コンテキスト直下の mod.rs）だけ**。別コンテキストが他所有の型を「便利だから」「後方互換のため」と再輸出すること（`orchestration::PlanAction` のようなエイリアス再輸出を含む）、クレート root や任意モジュールでの寄せ集め再輸出（prelude 的な `pub use` 束）は禁止 — 型の所有元が消費側のパスから読めなくなり、構造（どのコンテキストが何を所有するか）が読めなくなるため。所有を移すときは**完全移動**（呼出側のパスを一斉に直す）で行い、エイリアス再輸出で先送りしない。

## `unreachable_pub` との運用ループ（この方針の要）

deny 済みの `unreachable_pub` は、全 `pub mod` 構成では**一度も発火しない**（すべて到達可能のため）。私有 mod + 選択的 `pub use` に変えると、**`pub` を付けたが再輸出していないアイテムが全部ビルドエラー**になる — 「公開は宣言的・意図的に」という本ルールの運用を、追加ツールなしで既存 lint が強制する。再輸出しない内部共有は `pub(crate)` / `pub(super)` へ明示的に降格する。

## 根拠

[field-visibility.md](field-visibility.md)（フィールドの private 既定）のモジュール版。公開面が「型と関数の集合」になり「ファイルツリー」でなくなることで、内部リファクタリングの自由と公開 API のレビュー可能性（mod.rs の diff = API 変更）を同時に得る。
