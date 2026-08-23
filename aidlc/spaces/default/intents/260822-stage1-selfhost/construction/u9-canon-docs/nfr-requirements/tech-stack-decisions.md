# tech-stack-decisions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Requirements（Construction 3.2）成果物（Unit: U9、kind: spec、Bolt: B4）。出典: `security-requirements.md`（NFR1.1〜NFR2.5）、
> `../functional-design/rules.md`、`../../../inception/requirements-analysis/requirements.md`（制約 C4）、`aidlc/spaces/default/codekb/docs/
> technology-stack.md`（文書ツールチェーン: Markdown、markdownlint は CI 外、レビューボット CodeRabbit）、確認事項 `nfr-requirements-questions.md`（P3）。

## 1. 選定

| 領域 | 選定 | 理由 | 代替案（不採用の理由） |
|---|---|---|---|
| 文書形式 | Markdown（既存どおり）。表は見出しと同じ列数、regex 内の `\|` はエスケープ、同一見出しの重複なし | リポジトリの正本形式。レビューボットの markdownlint（MD056 / MD024）を予防 | — |
| 言語 | 日本語正本、固定トークン（型名・API 名・ファイル名・ID・YAML キー）は英語 | 制約 C4 / org.md 会話言語規則 | — |
| 出典注記 | 改訂箇所の末尾に括弧書きで出典（ADR-NNN / C-n / Bolt Bn / オーナー裁定 YYYY-MM-DD） | NFR1.3 の追跡可能性。後続 Bolt の実装者が「なぜそう書いてあるか」を辿れる | 脚注のみ: 行の近くに無いと見落とす |
| 自己整合の検査 | `grep -rnE` による sentinel 検査（NFR2.2 の語彙、対象 `coding-rules/*.md` + `docs/specs/*.md`、`research/` 除外、履歴注記・禁止名テーブルは除外） | 機械的に残骸を検出。PR の受入手順に載せる | 専用 lint の新設: 文書 1 Bolt のために過剰 |
| 差分の受入 | `git diff --stat` でコード領域（modules / tools / scripts / .github / Cargo.*）が空であることを確認 | NFR2.1 | — |
| レビュー | アーキテクチャレビュアー（ステージ）+ PR のレビューボット（CodeRabbit）— 指摘はすべて返信・解消してから merge queue（review-thread gate） | オーナー規律（PR コメントを無視しない）、U10 の gate | — |

## 2. 依存の差分（予定）

| 種別 | 追加・変更 | 備考 |
|---|---|---|
| ツール / 依存 | なし | 文書のみ |
| ファイル | `coding-rules/{use-case-rules,gateway-taxonomy,README}.md`（改訂）、`coding-rules/error-handling.md`（新規）、`docs/specs/{01-domain-model,10-orchestration,11-workspace,12-workflow-definition,deviations}.md`（改訂）、`inception/domain-design/components.md`（改訂） | BR1.x〜BR4.x |
| CI | なし | — |

## 3. 未決（後続で確定）

- なし（B4 の計画で各 BR の改訂文面を起草し、Plan Approval で確認する）。
