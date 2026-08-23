# nfr-design-questions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Design（Construction 3.3）の質問票（Unit: U9、kind: spec、Bolt: B4）。出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜1.3 /
> NFR2.1〜2.5）、`../nfr-requirements/tech-stack-decisions.md`、`../functional-design/rules.md`（BR1.1〜BR5.2）と `../functional-design/pending-revision.md`
> （回復レビュー所見 — BR2.5 の範囲、§1b の WorkspaceLock 模範例、BR5.1 の grep 範囲、BR5.1 (d) の diff スコープ）、`../../../inception/contract-design/
> contract-summary.md`（U9 は契約面を持たない）。spec kind のため成果物は `security-design.md` / `traceability.json` の 2 つ（logical-components は作らない）。
>
> **質問なし。** 文書だけの Unit の NFR 設計は「改訂の作法と受入検査の具体化」に尽き、要求（NFR1.x / NFR2.x）から一意に決まる。次の前提を確認して
> 成果物へ進む。

## 前提（確認事項）

- P1. 改訂の作法（NFR1.1 / 1.3 / 2.5 の設計）: 各改訂は (a) 対象節を最小限に書き換え、(b) 行末または段落末に出典を括弧書き `（ADR-008 / C4 改訂 /
  Bolt B3 / オーナー裁定 2026-08-23）` で残し、(c) 逐語契約（D6）の文言・`docs/specs/research/` には触れず、(d) 日本語正本・固定トークンは英語。
  旧記述を残す場合は「旧」と明記した比較表（履歴注記）にだけ置く。
- P2. 受入検査の設計（NFR2.1 / 2.2 / 2.3 / 2.4）: PR の受入チェックリストを code-generation の計画に置き、(a) `git diff --stat origin/main..HEAD -- modules tools
  scripts .github Cargo.toml Cargo.lock` が空、(b) sentinel grep `grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start' aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md docs/specs/*.md`
  の結果が履歴注記（「旧」明記の比較表・禁止名テーブル）のみ、(c) README の行数 = ルールファイル数、(d) 改訂した表の列数一致（`\|` エスケープ）と見出し重複なし、
  を PR 本文に実測で貼る。(e) CodeRabbit の指摘はすべて返信・解消してから merge queue。
- P3. 逸脱登録の設計（NFR1.2）: deviations.md に 1 行 — 分類『設計変更』、upstream『状態ファイル・監査シャードのテキストファイル群を真実源とし、mkdir ロックで
  read-modify-write を直列化』、amadeus-ng『SQLite ジャーナル（journal / snapshot / checkpoint）を真実源、楽観 version で直列化、ロック dir は生成しない、
  `aidlc-state.md` / 監査シャードはリードモデルとしてバイト互換で再生成』、理由『ADR-001 / 003 / 004 / 007（NFR1 の逸脱登録）』、記録『2026-08-23 / ADR-003, ADR-007』。
  予約行（決定済み・記録待ち）の該当項目は本行へ統合。

## Consolidated Summary Confirmation

- U9 に固有の NFR 設計質問はなし。設計 = 改訂の作法（最小変更・出典注記・逐語契約不変・日本語正本・履歴注記は「旧」明記）、受入検査（diff 空・sentinel grep・
  README 行数・表整形・レビューボット全件対応）、逸脱登録の行（ADR-001 / 003 / 004 / 007）
- 成果物は security-design.md / traceability.json（logical-components は spec kind のため作らない）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
