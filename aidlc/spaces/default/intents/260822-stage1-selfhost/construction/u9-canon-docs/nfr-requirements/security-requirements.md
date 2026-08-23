# security-requirements — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Requirements（Construction 3.2）成果物（Unit: U9、kind: spec、Bolt: B4）。出典: `../functional-design/rules.md`（BR1.1〜BR5.2 — 改訂内容と
> 合格条件）、`../../../inception/requirements-analysis/requirements.md`（NFR1 upstream 互換（D6 範囲）、NFR2 品質ゲート維持、制約 C4 日本語正本）、
> `../../../inception/contract-design/contract-summary.md`（U9 は契約面を持たない — C1〜C7 に影響なし。C4 / C5 の改訂内容を仕様へ写すのは BR3.1 / BR3.3）、
> `aidlc/spaces/default/codekb/docs/technology-stack.md`（文書ツールチェーン）、確認事項 `nfr-requirements-questions.md`（前提 P1〜P3、Looks correct）。
>
> spec Unit のため「セキュリティ要求」= 正本文書の**改訂の安全性**（逐語契約を壊さない、出典の追跡可能性、自己整合）の要求であり、NFR2（品質ゲート）
> の文書版もここに置く。各要求は Inception の NFR ID を継承し枝番を付ける（NFR1.x / NFR2.x）。

## 1. 範囲と信頼境界

- 対象は**文書だけ**: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md`（4 ファイル、うち 1 新規）、`docs/specs/{01,10,11,12}-*.md` と
  `docs/specs/deviations.md`、`inception/domain-design/components.md`。コードは触らない（`modules/` / `tools/` / `scripts/` / `.github/` の diff ゼロ）。
- 信頼境界: (a) 正本の権威 = オーナー裁定（coding-rules は 1 ルール 1 ファイル・裁定日つき）、(b) 仕様の権威 = upstream の観測可能契約（D6）と ADR、
  (c) PR レビュー（アーキテクチャレビュアー + レビューボット）。改訂は (a)(b) に遡れる出典を持つものだけを通す。
- 秘密情報・個人情報を扱わない。外部ネットワーク不要（upstream ピンの参照は既存のローカル写し `tests/golden/upstream-3c3146cf/` と `docs/specs/research/`）。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | **逐語契約の不変** — 仕様改訂は「構造の規範と所有の記述」に限り、upstream 互換の逐語契約（監査イベント 86 語・CLI 語彙・`AIDLC_*`・LLM 分岐条件の文言・ファイル形式）と `docs/specs/research/*.md`（抽出文書）には触れない | `git diff -- docs/specs/research` が空。10 号 §1 の「逐語の完全列挙は抽出文書と upstream を正とする」の一文を維持。改訂行に逐語文言の変更が含まれないことをレビューで確認 | NFR1, BR5.2 |
| NFR1.2 | **逸脱の登録** — ES 化に伴う観測可能な逸脱（SQLite ファイルの追加・ロック dir 非生成・互換ファイルはリードモデル）は `docs/specs/deviations.md` の表に登録し、本文の改訂だけで済ませない | deviations.md に 1 行（# / 分類 / upstream / amadeus-ng / 理由 / 記録）が追加され、理由欄が ADR-003 / 007 を指す | NFR1, BR3.4 |
| NFR1.3 | **出典の追跡可能性** — 各改訂箇所に出典（ADR 番号 / 契約 ID / Bolt / オーナー裁定日）を括弧書きで残し、推測で仕様を変えない | 改訂行の出典注記をレビューで確認（出典の無い改訂は差し戻し） | NFR1, BR5.2, BR3.x の source |
| NFR2.1 | **コード変更ゼロ** — Bolt B4 は文書のみ。CI 4 ジョブ（check / quint / coverage / audit）と `CI Success` は変更なしで緑のまま | `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空。PR の CI 緑 | NFR2, BR5.1 (d) |
| NFR2.2 | **自己整合の機械検査** — 削除済み API 名・退役機構・旧称が規範として残らない: `effective_plan_action` / `next_in_scope_stage` / `AuditLedgerRepository` / `AuditLedgerService` / `StateFileStore` / `report_forward` / `gate_start` を `coding-rules/*.md` と `docs/specs/*.md`（`research/` を除く）で grep し、履歴注記（『旧』と明記された比較表・禁止名テーブル）以外で 0 件 | grep の結果を PR 本文（または code-summary）に貼る。`StageGraphReader` は gateway-taxonomy §2 の禁止名テーブル（意図的な記録）を除外 | NFR2, BR5.1 (c), FD 回復レビュー所見 3 |
| NFR2.3 | **索引の無矛盾** — `coding-rules/README.md` の一覧表の行数 = ルールファイル数、各行の一言・機械強制・裁定日が各ファイルと一致 | README と `ls coding-rules/*.md` の突合（レビュー） | NFR2, BR4.2 |
| NFR2.4 | **表・見出しの整形** — 改訂した Markdown 表は見出しと同じ列数（regex 内の `|` はエスケープ）、同一見出しの重複を作らない（レビューボットの markdownlint 指摘 MD056 / MD024 を予防） | CodeRabbit の該当指摘 0 件（出たら PR 内で直す — PR コメントは無視しない） | NFR2, PR #25 / #27 の教訓 |
| NFR2.5 | **日本語正本** — 人間可読の改訂は日本語、固定トークン（型名・ファイル名・API 名・ID）は英語のまま | レビューで確認 | 制約 C4 |

## 3. 脅威の検討（STRIDE、文書改訂の規模）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing / Elevation of Privilege | 該当なし（文書だけ、権限変更なし） | — |
| Tampering | 改訂のついでに逐語契約（D6）を書き換えてしまう / 出典の無い「勝手な設計変更」を仕様に混ぜる | NFR1.1（research/ 不変）、NFR1.3（出典注記）、レビュー |
| Repudiation | 誰の裁定で変えたかが追えない | NFR1.3 + PR の記録（Bolt B4）、coding-rules の裁定日 |
| Information Disclosure | 該当なし（秘密情報を含む文書ではない） | — |
| Denial of Service | 該当なし（CI への影響なし — NFR2.1） | — |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| coding-rules / docs/specs / components.md | Public（公開リポジトリ） | 秘密情報なし |

## 5. 適用外

- NFR3（監査完全性）・NFR4（サプライチェーン）・NFR5（性能）: 文書だけの Unit で固有の要求を持たない（依存・コードの変更なし）。
