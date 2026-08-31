//! **クエリ側**のユースケース層 — リードモデルの**クエリモデル**と、読むだけの動詞。
//!
//! クエリ側は RMU が構築したリードモデル (`stage-graph.json` / `scope-grid.json` /
//! `scopes/aidlc-<name>.md` / `aidlc-state.md`) だけに依存し、それを**自前のビュー型**へ
//! 写して読む。ドメイン (`core-command-domain`) には絶対依存せず、集約の再構成もしない
//! (`coding-rules/cqrs-boundaries.md` 規則 6 / 2026-08-29 オーナー裁定)。`Cargo.toml` に
//! コマンド側クレートが**無いこと**がその機械強制であり、違反はビルドで落ちる。
//!
//! 型はコマンド側ドメインの写しではなく**側ごとの専用モデル**である (同規則「共有部品は
//! 側の独立を DRY に優先」)。両側が同じ Published Language を読むので形は似るが、
//! 変更理由は独立している — コマンド側の集約が変わってもこちらは動かない。
//!
//! # モジュールの分担
//!
//! コンテキストは [`orchestration`] 1 つで、その中が読み手の責務ごとに分かれる。
//!
//! - `orchestration/port/` — リードモデルを読む **DTO/DAO ポート**。DAO の契約 (trait) と
//!   ポート面のエラーに加え、**DAO が依存する DTO も同居する** — DTO/DAO ポートは一つの
//!   パッケージである (オーナー裁定 2026-08-31)。DTO は読む対象ごとに 2 族:
//!   `workflow_view` (ワークフロー定義リードモデル 3 入力のビュー型。「何を実行しうるか」) と
//!   `execution_view` (実行状態リードモデル `aidlc-state.md` のビュー型と、その上の判断 =
//!   BR3.1 の 8 分岐。「いま何が起きているか」)
//! - `orchestration` 直下 — 読むだけの動詞 (`next` / `continue`) と、それが放出する directive
//!   プロトコル (公開言語 B14)。「次に何をせよと言うか」
//!
//! `port` も 2 つの DTO 族も mod は private であり、公開 API は
//! [`orchestration`] のフラットなファサード (`pub use`) が唯一の宣言である。消費側のパスは
//! 読む対象によらず `core_query_use_case::orchestration::<型>` で安定する
//! (`coding-rules/module-visibility.md`)。

pub mod orchestration;
