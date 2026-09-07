# tech-stack-decisions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> NFR Requirements（Construction 3.2）成果物（Unit: U9、kind: spec）。**改訂履歴**: 初版 2026-08-23（Bolt B4 向け）→ **再走 2026-09-07（本版、Modify）**。
> 出典: `security-requirements.md`（NFR1.1〜NFR2.6）、`../functional-design/rules.md`（BR1.6 / BR3.3 / BR3.7 / BR5.1 / BR5.3）、
> `../functional-design/gap-measurement-20260907.md`（実測の方法 §1、行単位の処置 §2）、`../../../inception/requirements-analysis/requirements.md`（制約 C4）、
> `aidlc/spaces/default/codekb/docs/technology-stack.md`（文書ツールチェーン: Markdown、markdownlint は CI 外、レビューボット CodeRabbit）、
> `.github/workflows/ci.yml`（実測 7 ジョブ）、確認事項 `nfr-requirements-questions.md`（P3 = 2026-08-23、P7 = 2026-09-07）。

## 1. 選定

| 領域 | 選定 | 理由 | 代替案（不採用の理由） |
|---|---|---|---|
| 文書形式 | Markdown（既存どおり）。表は見出しと同じ列数、regex 内の `\|` はエスケープ、同一見出しの重複なし | リポジトリの正本形式。レビューボットの markdownlint（MD056 / MD024）を予防 | — |
| 言語 | 日本語正本、固定トークン（型名・API 名・ファイル名・ID・YAML キー）は英語 | 制約 C4 / org.md 会話言語規則 | — |
| 出典注記 | 改訂箇所の末尾に括弧書きで出典（ADR-NNN / C-n / Bolt bn / オーナー裁定 YYYY-MM-DD / **実測の所在 path:line または型・関数名**） | NFR1.3 の追跡可能性。後続 Bolt の実装者が「なぜそう書いてあるか」を辿れる | 脚注のみ: 行の近くに無いと見落とす |
| 実測の手段 | `grep` / `awk` による現行コードの型・関数・変種の列挙（`pub struct` / `pub enum` / `pub fn` / `pub const fn` / 変種名）と、文書の該当行との突合。基準コミットを明記（`main` `02cacea2`） | NFR2.6（検証規律）。記録の主張を転記せず、コードで実否を確認する（gap-measurement §1 の方法） | LSP / rust-analyzer による列挙: 精度は高いが再現手順を文書に残しにくい。手動読解のみ: 取り落としが出る（b51 前の記録が 11 イベントのまま残っていた実例） |
| 自己整合の検査 | `grep -rnE` による sentinel 検査（NFR2.2 の 10 語、対象 `coding-rules/*.md` + `docs/specs/*.md`、`research/` 除外、履歴マーカー行は除外） | 機械的に残骸を検出。PR の受入手順に載せる | 専用 lint の新設: 文書 1 Bolt のために過剰 |
| 差分の受入 | `git diff --stat origin/main..HEAD` でコード領域（modules / tools / scripts / .github / Cargo.toml / Cargo.lock）が空であることを確認 | NFR2.1 | — |
| 履歴の保全 | 失効した記述は削除せず `~~…~~ — 失効（日付 / 出典）` で残す。既存 `## Review` 節は不変 | NFR1.5、no-backward-compatibility.md 対象外（履歴記述は消さず失効を追記） | 削除して書き直す: 経緯が消え、レビュー所見の追跡が切れる |
| レビュー | アーキテクチャレビュアー（ステージ）+ PR のレビューボット（CodeRabbit）— 指摘はすべて返信・解消してから merge queue（`review-thread-resolution` ジョブ） | オーナー規律（PR コメントを無視しない）、収束ルール | — |

## 2. 依存の差分（予定）

| 種別 | 追加・変更 | 備考 |
|---|---|---|
| ツール / 依存 | なし | 文書のみ |
| ファイル（coding-rules） | `README.md`（:50 / :115）、`error-handling.md`（:4）、`factory-naming.md`（:47 / :84）、`gateway-taxonomy.md`（:223 / :299）、`module-visibility.md`（:11 / :12 / :21）、`use-case-rules.md`（:11 / :36） | BR1.6 / BR4.2、12 行 |
| ファイル（仕様） | `docs/specs/{01-domain-model,10-orchestration,11-workspace,12-workflow-definition}.md` | BR3.2 / BR3.3。`deviations.md` は触らない |
| ファイル（共有契約） | `inception/domain-design/components.md`（全面）、`inception/contract-design/contract-summary.md`（C1 / C3 / C4 / C5 / C6 / §4）、`inception/units-generation/unit-of-work.md`（U3 の失効注記のみ） | BR3.7。`decisions.md` は変更しない（既に注記あり） |
| CI | なし | — |

## 3. 未決（後続で確定）

- なし（次ステージ NFR Design で委譲 2 派遣の書込スコープと受入手順を確定する — functional-spec §2 責任分担）。

## 4. 前版からの変更（2026-09-07 再走）

- §1 に「実測の手段」「履歴の保全」の 2 行を追加、出典注記に実測の所在を加えた。§2 を再走の対象一覧（6 + 4 + 3 ファイル）へ差し替え。
