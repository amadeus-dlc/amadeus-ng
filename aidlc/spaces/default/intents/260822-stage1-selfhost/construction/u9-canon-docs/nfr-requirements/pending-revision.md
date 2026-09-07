# pending-revision — U9 nfr-requirements（ステージゲートの Request Changes で適用する改訂案）

> レビュー（iteration 1、READY: Major 1 / Minor 1）の所見。受領は終端のため本文は据え置き、ゲートで Request Changes を選んだ直後に適用。
> B4 の code-generation 計画には「コード変更ゼロの diff スコープ = modules tools scripts .github Cargo.toml Cargo.lock」を採用する（安全側）。

1. NFR2.1 の出典欄に「BR5.1 (d)（`modules tools`）を `scripts .github Cargo.toml Cargo.lock` まで広げて強化 — 依存操作の見落とし防止。rules.md 側は
   functional-design pending-revision 項目 4 で同期」を注記。
2. NFR2.2 の出典欄を「BR5.1 (c)、`../functional-design/pending-revision.md` 項目 1・3（rules.md 本文は未適用）」と明示。

> **解消（2026-09-07、U9 再走）**: 項目 1・2 は rules.md BR5.1 が diff 範囲（`modules tools scripts .github Cargo.toml Cargo.lock`）と sentinel
> （`StageGraphReader` を外す）を同期したことで根拠が揃い、security-requirements.md NFR2.1 / NFR2.2 の出典欄を BR5.1 (c)(d) に一本化して閉じた。本ファイルは履歴。
