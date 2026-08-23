# pending-revision — U9 functional-design（ステージゲートの Request Changes で適用する改訂案）

> 回復レビュー（iteration 2、2026-08-23、READY: Major 2 / Minor 1）の所見。回復枠は消費済みのため本文は据え置き、functional-design ステージゲートで
> Request Changes を選んだ直後に適用してレビュアーを再実行する。**ただし B4 の文書改訂はこの 3 点を code-generation の計画に取り込んで実施する**
> （計画は rules の適用範囲を明示できる — 実作業が設計の穴で止まらないようにする）。

1. BR2.5 の適用範囲: 12 号の `next_in_scope_stage` 出現 5 箇所すべて（§2.3 の 2 箇所 + §4 未知スコープ表の行 + §8 不変条件表 F2 行 + §9 ユビキタス言語例）を
   改訂対象に含める（履歴注記ではなく現行規範として書かれているため）。BR5.1 の grep は「履歴注記（『旧』と明記された比較表）を除く全出現」と定義。
2. BR1.5（新設）: `coding-rules/gateway-taxonomy.md` §1b「非 Repository ポートの模範例 — WorkspaceLock」を改訂 — WorkspaceLock は退役（ADR-007）のため、
   模範例を「非 Repository ポートの一般形（契約の意味論を型に載せる — 予算・再入・二重解放不能を型で表現）」として再構成し、具体名 WorkspaceLock への
   依存を外す（退役の旨を 1 行注記）。
3. BR5.1 の grep 範囲を `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md` + `docs/specs/*.md`（`docs/specs/research/` を除く）と明記。
   `StageGraphReader` は gateway-taxonomy §2 の禁止名テーブル（意図的な記録）を除外し、他に除去対象が無ければ sentinel から外す。
4. （nfr-requirements レビュー所見 1）BR5.1 (d) のコード変更ゼロの diff スコープを `modules tools scripts .github Cargo.toml Cargo.lock`（`origin/main..HEAD`）に
   広げ、NFR2.1 / tech-stack-decisions と同期する（依存操作の見落とし防止 — 安全側へ統一）。
5. （PR #28 CodeRabbit）entities.md の `CodingRule(gateway-taxonomy.md)` インスタンスの `revisions_in_b4` に BR1.5 を追加し（§1b 再構成）、rules.md 本文に BR1.5 を新設（項目 2）して 1:1 対応を回復する。
