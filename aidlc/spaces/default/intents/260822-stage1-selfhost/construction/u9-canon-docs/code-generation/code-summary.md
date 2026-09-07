# code-summary — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Code Generation（Construction 3.5）の作成報告（Unit: U9、kind: spec）。**改訂履歴**: 初版 2026-08-23（Bolt B4、PR #28）→ **再走 2026-09-07（本版、Modify）**。
> 出典: `code-generation-plan.md`（承認指紋 sha256:e4d9ca10…、Testing Contract sha256:303d9bb7…）、`unit-test-instructions.md`（受入 10 項目）、
> `developer-report-3.md`（派遣 A）/ `developer-report-4.md`（派遣 B）、`../functional-design/gap-measurement-20260907.md`、`../nfr-design/security-design.md`。
>
> **本 Unit はコードを書いていない。** 変更は文書 17 ファイル（coding-rules 9 / 仕様 4 / 共有契約 3 / ADR-010 注記 1）、`git diff --stat origin/main` で
> 17 files changed, 817 insertions(+), 425 deletions(-)。`modules` / `tools` / `scripts` / `.github` / `Cargo.*` / `formal` / `docs/specs/research` / `docs/specs/deviations.md`
> の diff は空。

## 1. 何をしたか

- **基準 = 現行コード**: `origin/main` = `02cacea2`（#118 b51）で、gap-measurement の基準そのもの。作業ツリーの `modules/` はこれと diff ゼロなので、
  §4 の台帳（1 件ずつコードで実否確認済み）をそのまま「正しい姿」の正本として使った。計画 §1 の「`origin/main` は `e8ca4a5f`（#117）まで進んでいる」は
  **誤り**（`e8ca4a5f` は `02cacea2` の親）— 計画は指紋済みのため本書で訂正する。
- **Step 0（メイン、派遣前）**: 主要件数を現行 HEAD で再実測（クレート 10 / `IntentExecutionEvent` 16 / ポート 4・全 `async fn` / FCC ドメイン 16 + infrastructure 3 /
  `Rehydrated*` 型 0 / tokio current_thread・`spawn` 0 / DAO 14 / `Find*` 13 / `InMemoryXxxDao` 13）、受入基線を記録（sentinel 40、README 22 = 22 + ずれ 2、
  予定 0、用語 0、`## Review` sha256 3 値）。この過程で gap-measurement の訂正 5 件（§4 T1〜T5）を見つけ、ブリーフに渡した。
- **派遣 A**（`developer-brief-3.md` → `developer-report-3.md`、Opus）: coding-rules 9 ファイル（改訂 12 行 + 履歴マーカー 8 行 + README 索引 2 点 + ロードマップ 2 行目）、
  10 号 23 箇所、01 号 10 箇所（§7.1 原則 7〜12 追記を含む）— **改訂 34 件**、全件に根拠列（コードの所在 / テスト / 仕様節）。
- **派遣 B**（`developer-brief-4.md` → `developer-report-4.md`、Opus）: 11 号 11 件、12 号 10 件、components.md 全面（12 コンポーネント）、contract-summary 12 件、
  unit-of-work 注記 5 行、ADR-010 失効注記 1 段落 — **改訂 51 件**、全件に根拠列。
- **メインの diff 全件レビューと追加訂正 3 件**: (a) `error-handling.md:4` の適用例 — `ApplyError` は `pub(crate) enum`、`IntentExecutionError` は `pub struct` で、
  「公開 enum 33 本の代表」に混ぜられていたので実測どおり書き分けた、(b) 同 :12 の重複マーカー「。履歴」を除去（「旧」が既にマーカー）、
  (c) `components.md` `CommandGateways` に `CompiledDefinitionRepositoryImpl` の例外（型引数 `S` も `in_memory()` も無い — 媒体が配布ファイル）を 1 行追記（B 保留 6）。

## 2. 受入検査（`unit-test-instructions.md` §1、2026-09-07 実測）

| # | 検査 | 基線（赤） | 結果（緑） | 判定 |
|---|---|---|---|---|
| 1 | コード変更ゼロ（`git diff --stat origin/main -- modules tools scripts .github Cargo.toml Cargo.lock formal docs/specs/research docs/specs/deviations.md`） | — | 出力なし | ✓ |
| 2 | sentinel 10 語 grep（`coding-rules/*.md` − CONSISTENCY-AUDIT + `docs/specs/*.md`、履歴マーカー除外） | **40**（coding-rules 12 + 10 号 9 + 11 号 5 + 01 号 6 + 12 号 8） | **0** | ✓ |
| 3 | README 無矛盾 | 22 = 22（表 23 − good-examples 1）、索引ずれ 2（:10 告知行 / :12「13 本」） | 22 / 23 / 「規則が 22 本」の 1 行のみ | ✓ |
| 4 | 表の列数・見出し重複（17 ファイル、`table_check.py`） | — | `tables ok` | ✓ |
| 5 | 逸脱登録の維持（`deviations.md` diff） | 空 | 空 | ✓ |
| 6 | `## Review` 節の履歴保全（sha256） | components `b954159a…` / contract-summary `7374b976…` / unit-of-work `b8d0e4a1…` | **3 値とも同一**（見出し行は :430→:663 / :486→:593 / :207→:207） | ✓ |
| 6' | `decisions.md` の diff | — | ADR-010 :476-478 の段落のみ（`-` 2 行の内容は `~~` 付きで `+` 側に残り、削除された文字は無い） | ✓（下記 §5 の読み） |
| 7 | 実装状態の表記（`予定（未実装`） | 4 号すべて 0 | 01 号 1 / 10 号 8 / 11 号 10 / 12 号 1 | ✓ |
| 8 | 実測表との突合（gap-measurement §2.1〜§2.6 の『処置』列 vs diff） | — | 全行反映、『維持』行（10 号 :56 Tx 境界 / 11 号 §2.2・§2.3 / 12 号 §2.1）は不変 | ✓ |
| 9 | 用語（RMU 文脈の「同期」） | 0 | 0（B が components.md の同一行擬陽性 1 件を改行で解消） | ✓ |
| 10 | レビュー（CodeRabbit 全件 + ステージレビュー READY + CI 7 ジョブ） | — | PR 作成後に実測 — 結果は PR 本文に転記 | 未 |

検査 (2) のコマンドは `CONSISTENCY-AUDIT-*.md` を除外済みなので基線は 40（gap-measurement の 44 = 40 + CONSISTENCY-AUDIT 4）。
検査 (7) の第 2 コマンド（予定表記も履歴マーカーも無い未実装項目の言及）で残る行は、いずれも実装状態を主張しない行 — コンテキストの責務・所有の記述
（10 号 :15 / :19 / :59、01 号 :84 / :88 / :179 / :189、12 号 :34 / :37）、契約コーパス・語彙表（10 号 :7、01 号 :68 / :210 / :217、12 号 :87）、Quint の不変条件名
（10 号 :151 / :239）、§7 の見出し（10 号 :183 — 直上で本節全体が予定と明記）、実装済みの実装ノート（10 号 :176 / :267）、11 号の値オブジェクト・Presenter・
未決事項（:42 / :63 / :67 / :127 / :168 / :183 / :221 / :222）。

## 3. 改訂の棚卸し（ファイル別）

`source-manifest.json` はアプリケーション側の文書 4 本（`docs/specs/01 / 10 / 11 / 12`）だけを載せる — `aidlc/` 配下（coding-rules 9・共有契約 3・ADR-010 注記）は
フレームワークが記録ツリーとして manifest から除外するため（`aidlc-log review` が拒否）、本表が 17 ファイルの全数を列挙する。

| ファイル | 派遣 | 件数 | 主な内容 |
|---|---|---|---|
| `coding-rules/README.md` | A | 4 | error-handling 行の `wording` 化、:10 告知行の一覧表への畳み込み、「13 本」→ 22、ロードマップ 1〜2 行目の是正済み化 |
| `coding-rules/error-handling.md` | A + メイン | 2 + 2 | 適用例の型名（公開 enum 33 + `pub(crate) ApplyError` + `pub struct IntentExecutionError`）、履歴マーカー |
| `coding-rules/factory-naming.md` | A | 4 | 反例の履歴化、`with_version` の例 3 件、マーカー 2 |
| `coding-rules/gateway-taxonomy.md` | A | 4 | 集約名、クレート名、マーカー 2 |
| `coding-rules/module-visibility.md` | A | 3 | クレート名 2、共有語彙の例を `core_infrastructure::{7 モジュール}` へ |
| `coding-rules/use-case-rules.md` | A | 2 | クレート名 + dev-dependency 不可の明記、ポート実装の 3 層（T1） |
| `command-query-separation.md` / `interior-mutability.md` / `field-visibility.md` | A | 各 1 | マーカーのみ |
| `docs/specs/10-orchestration.md` | A | 23 | 冒頭注記の畳み込み、§2.1 の 2 集約分割（12 属性 / 15 + genesis / 16 変種 / memento なし / 差分再生 / `next_decision` の署名）、§2.2 所在列、§2.3、§3 ユースケース実装名・3 層・ポート表 4 行、I8 失効、§8、S1、予定表記 |
| `docs/specs/01-domain-model.md` | A | 10 | 冒頭注記、§3.2 集約段落の書き直し、§3.3 予定 + 脚注の是正済み化、§7.1 原則 5 の 2 段化と原則 7〜12（BR3.3 (h) / (j) — 未参照 coding-rules 12 本すべてに相互参照） |
| `docs/specs/11-workspace.md` | B | 11 | 冒頭注記、§2.1 実装状態 + 集約 3 の予定 + 同名別物注記、ポート表 `IntentExecutionRepository`、供給面 4 の予定、名称 5 箇所 |
| `docs/specs/12-workflow-definition.md` | B | 10 | 冒頭注記、§2.3 クエリ 3 件の表と旧述語の不在、§4 #8 の `Intent::create`、§5 の 3 層（コマンド 1 / クエリ 5 / `read_definition*` 6）、`find_for_intent`、名称 |
| `inception/domain-design/components.md` | B + メイン | 12 + 1 | 全面改訂 — コンポーネント 12 ↔ クレート 10（`QueryUseCases` / `QueryGateways` / `HarnessInfrastructure` 新設、`PublishedLanguage` 解消、`CanonJson` + `InfraIo` 統合）、図・要約表・Entity Ownership・Rationale。`## Review` 以降不変 |
| `inception/contract-design/contract-summary.md` | B | 12 | C1 / C3 / C4 / C5 / C6 / §4 の節単位現行化（v2 trait 全文は履歴として残置 — Review 節所見 3 が参照）。`## Review` 以降不変 |
| `inception/units-generation/unit-of-work.md` | B | 5 | U3 の :34 / :64 / :83 / :91 / :144 に改名・失効注記（本文不変）。`## Review` 以降不変 |
| `inception/domain-design/decisions.md` | B | 1 | ADR-010 :476-478 の段落を `~~…~~` + 失効注記（追記のみ） |

## 4. 実測記録・計画からの訂正（functional-design ゲートで gap-measurement / rules.md へ折り戻す）

| # | 対象 | 記録の主張 | 実測 | 反映先 |
|---|---|---|---|---|
| T1 | gap-measurement §1「コマンド側ユースケース」/ §4.2 P4 | テストダブル型 `InMemoryXxxRepository` は無い | `core-command-use-case/src/orchestration/test_support.rs` に `#[cfg(test)]`（`mod.rs:38-39`）の `pub(crate)` フェイク 4 つ（IntentExecution / Intent / WorkflowDefinition / CompiledDefinition）。DIP のクレート分離でアダプタを dev-dependency にも書けないための単体テスト専用の例外（オーナー裁定 2026-08-31）。公開のインメモリ実装はアダプタ層 `XxxRepositoryImpl<S>::in_memory()` が正 | use-case-rules §2 / 10 号 §3 / 11 号 §3 / components / contract-summary C3 / unit-of-work :144 を **3 層構造**で記述。gap-measurement §1・P4 の訂正が要る。`test_support.rs:1-17` の doc「ここに置くのは 1 つだけ」も実態（4 つ）とずれる — コードの doc 修正は別 Bolt 候補 |
| T2 | §1 / O8 | `Rehydrated*` は 0 件 | 型は 0 件。`port/mod.rs:15-18` / `port/intent_execution_repository.rs:36-38` の doc に「廃止済み」言及 2 | 一致（表現の精密化のみ） |
| T3 | R1 / 追加実測 1 | `JournalReader` は `async fn` 9 | `async fn` 8 + 同期 `fn prepare_read_model(&mut self)` 1（`journal_reader.rs:38`） | components / contract-summary C3 に「9 = async 8 + 同期 1」。gap-measurement の訂正 |
| T4 | O13 / K4 | FCC 16 型 | ドメイン 16（orchestration 8 / workspace 6 / workflow-definition 2）+ `core-infrastructure` の `Collection<T>` / `NonEmptyCollection<T>`（#114）/ `canon_json::ObjectMembers` | 01 号 §7.1 原則 7 / components `CoreInfrastructure` に「ドメイン 16 + infrastructure 3」 |
| T5 | P8 | tokio current_thread、spawn なし | `main.rs:12` `flavor = "current_thread"`、`tokio::spawn` / `spawn_blocking` 0 件 | 一致（実測所在を追記） |
| T6 | R4 / §1 | publication 系 4 表 / 5 表 | `CREATE TABLE IF NOT EXISTS` 実測 **6 表**（`amadeus_publication` / `_file` / `_history` / `_history_file` / `_snapshot` / `_snapshot_file`） | components `ReadModelUpdater` / contract-summary C6 に 6 表を全数列挙。gap-measurement R4・§1 の訂正 |
| T7 | P9 | 面 4（`aidlc-bolt` / `aidlc-log` / `aidlc-state` / `aidlc-utility`） | `Face` enum は 5 変種（素の `aidlc` / `aidlc-orchestrate` を含む — `cli/face.rs:5`） | components `CliDispatcher`「面は 5 つ」。数え方の差として両方が読める形 |
| T8 | O15 | `StateBinding` はクエリ側に住む | 同名型が両側にある — `command/domain/src/orchestration/state_binding.rs:10`（`IntentExecution::state_binding()` の戻り値）と `query/use-case/src/orchestration/state_binding.rs:11`（継続トークン封筒側） | 10 号 §2.2 の表に `StateBinding` 行は無く改訂に影響なし。gap-measurement O15 の文言訂正が要る |
| T9 | P1 の一般形 | `XxxRepositoryImpl<S>::in_memory()` | `CompiledDefinitionRepositoryImpl` は非ジェネリックで `in_memory()` を持たない（媒体が配布ファイル）。`in_memory()` は 3 実装 | components `CommandGateways` に例外 1 行（メイン追記） |
| T10 | 計画 §1 | `origin/main` は `e8ca4a5f`（#117） | `origin/main` = `02cacea2`（#118）= 台帳の基準そのもの。ブランチは記録コミット 3 本先行、後方なし | 本書で訂正（計画は指紋済み） |

## 5. 裁定（メイン）と保留の処理

- **予定表記の番号**: workspace 集約 3 / 供給面 4 / `intents.json` は、ブリーフの「項目 2」ではなく派遣 A の読み **「クリティカルパス 4 = マルチコール CLI + 文言カタログ配線」**
  で統一した（CLI 動詞の配線で着地する未実装。B にも指示し 9 箇所を統一）。unpark / jump / recompose のユースケース = 4、フック 4 本 = 5、doctor = 6、Bolt / SwarmBatch = swarm はスコープ外。
- **受入 (6) の読み**（B 保留 1）: 「削除行（`-`）が無い」は「**内容の削除が無い**」の意味で読む — 既存行に `~~` を付けるには行の書き換えが要り、`git diff` は必ず `-`/`+` の対を出す。
  実測の `-` 2 行はどちらも `~~` 付きで `+` 側に残っている。`unit-test-instructions.md` §1 (6) の文言はこの読みへ次の改訂で寄せる（本書 §7）。
- **台帳に無い行の改訂**（A 保留 3、B 保留 3・4）: 10 号 §1 B1 / 11 号 :35 / :95 / :165 / :196、README ロードマップ 2 行目、Bolt / SwarmBatch / OpaqueFlagStore の予定表記は、
  受入 (2)・BR4.2・作法「実装状態の表記」を満たすための名称のみ・注記のみの改訂で、**採用**。
- **BR3.3 (j) の相互参照**（A 保留 4）: 未参照 coding-rules 12 本は 01 号 §7.1 の原則 7〜12 で**すべて参照済み**（A は 14 本に張った）。11 号 / 12 号側の追加参照は不要。
- **C3 の v2 trait 全文**（B 保留 7）: `## Review` 節の所見 3 が参照するため削除せず履歴として残し、節冒頭に現行 4 ポートの表を置く形を**採用**（NFR1.5）。
- **根拠を添えられなかった改訂**: A / B とも 0 件。予定表記は不在を `grep` / `ls` で実測。

## 6. 検証の記録

- 派遣 A / B の報告にある引用（型名・関数名・行番号・件数）のうち次をメインが再実測して一致を確認: `intent_execution.rs:225/290/352/544/1788/1864/2330`、
  `directive_schema.rs:11` / `directive.rs:26` / `continue_token.rs:27`、`presenter.rs:62` `DIRECTIVE_MAX_BYTES`、`next_decision.rs` 8 変種、ポート 4 の `async fn` 署名、
  `# Panics` 3 + 1、`pub enum *Error` 33、`intent.rs:69/114`、`infrastructure/src/lib.rs:20-26`、use-case `Cargo.toml` の依存、`workflow_definition.rs:433/483/522`、
  `Face` 5 変種、`request.rs:127` `unpark`、`CREATE TABLE` 25 表（read_* 17 + amadeus 8）、`*RepositoryImpl` の宣言行、イベント enum 4 族。
- 不一致として直したもの: `ApplyError` / `IntentExecutionError` の種別（§1 (a)）。

## 7. 申し送り（次の改訂・ゲートへ）

1. functional-design ゲートで gap-measurement へ T1 / T3 / T6 / T7 / T8 を折り戻し、rules.md に BR1.6 20 行 / BR3.7 (d) 訂正 / BR5.1 (c) CONSISTENCY-AUDIT 除外 /
   BR3.3 (g) 「同期」の是正を同期する（凍結中のため本 Bolt では触っていない）。
2. `unit-test-instructions.md` §1 (6) の期待「削除行が無い」→「内容の削除が無い（`-` 行は `~~` 付きで `+` 側に残る）」（指紋済みのため次の改訂で）。
3. コードの doc ずれ 1 件（`test_support.rs:1-17`「ここに置くのは 1 つだけ」— 実態はフェイク 4 つ）は 1 行修正の別 Bolt 候補。`formal/orchestration/journal_protocol.qnt` の
   コメント 5 行の旧名も同様（gap-measurement §5）。
4. 受入 (10) は PR 作成後に実測し、PR 本文へ転記する。
