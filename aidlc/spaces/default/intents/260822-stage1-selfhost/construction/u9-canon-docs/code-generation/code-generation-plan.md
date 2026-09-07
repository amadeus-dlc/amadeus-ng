# code-generation-plan — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Code Generation（Construction 3.5）の計画（Unit: U9、kind: spec、規模 M）。**改訂履歴**: 初版 2026-08-23（Bolt B4、PR #28 でマージ済み — coding-rules 4 /
> 仕様 5 / components 1 の改訂）→ **再走 2026-09-07（本版、Modify）**: functional-design 差し戻し後の U9 再走。現行コードを基準に実測した記録
> `../functional-design/gap-measurement-20260907.md`（§1〜§5、追加実測 1〜4）と再走版 NFR 設計 `../nfr-design/security-design.md`（§2 作法 / §3 委譲 /
> §4 受入 10 項目）に合わせ、改訂対象・委譲・受入検査を書き直した。
>
> 出典: `../functional-design/rules.md`（open の BR1.6 / BR3.3 / BR3.7 / BR4.2 / BR5.1 / BR5.3、done の BR3.2 / BR3.6 / BR5.2）、`../functional-design/entities.md`、
> `../functional-design/gap-measurement-20260907.md`、`../nfr-requirements/security-requirements.md`（NFR1.1〜1.5 / NFR2.1〜2.6）、`../nfr-design/security-design.md`、
> `../nfr-design/nfr-design-questions.md`（P4〜P8）、`../../../inception/requirements-analysis/requirements.md`（FR8.1 / FR8.2 / FR9.6）、
> `../../../inception/units-generation/unit-of-work.md`（U9）、`aidlc/spaces/default/memory/project.md`（Mandated「実装は委譲」/ Corrections 収束ルール・`git add -A`）。
>
> **本 Unit はコードを書かない。** 「生成」の対象は正本・仕様・共有契約の文書であり、TDD の赤→緑は「受入検査（grep / diff / 行数 / バイト比較）を改訂前に
> 走らせて赤を記録し、改訂で緑にする」と読み替える。既存テストスイートは触らない（コード diff ゼロ）。

## 1. 前提と範囲

- **基準はコード**: 「正しい姿」の正本は `gap-measurement-20260907.md` §4 の台帳（O1〜O15 / P1〜P9 / R1〜R7 / W1〜W5 / S1〜S4 / K1〜K6、1 件ずつコードで
  実否確認済み）と §4.8 の裁き。記録（Bolt / Unit の設計記録・完了報告）を転記しない（BR5.3、NFR2.6）。台帳の基準は `main` `02cacea2` であり、
  その後 `origin/main` は `e8ca4a5f`（#117）まで進んでいる — **Step 0 で主要件数を現行 HEAD で再実測**し、差があれば code-summary に記録してブリーフ補遺に載せる。
- **ブランチ / PR**: 作業ブランチは現行の `stage1-selfhost`（U9 再走の記録コミット `6d5b99b6` / `0ce57e3f` / `410d75b4` を含む、`origin/main` 起点）。
  PR は 1 本直列、`main` へ squash-merge、コミット名 = Bolt slug `u9-canon-docs`。記録コミット → 文書コミットの順（開発エージェントはコミットしない）。
  コミットは `git add -A` で作業ツリー全体を回収する（監査シャードを落とさない — project.md Corrections 2026-09-03）。
- **コード変更ゼロ**: `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空（NFR2.1）。`docs/specs/research/**`・`formal/`・
  `docs/specs/deviations.md` も変更ゼロ（NFR1.1 / NFR1.2）。
- **改訂対象（nfr-design P4 の確定版）**:
  - coding-rules **20 行 9 ファイル** = 改訂 12 行 6 ファイル（BR1.6）+ 履歴マーカー付与 8 行 3 ファイル（本文は変えずマーカー語のみ）。
  - 仕様 4 号（`docs/specs/01-domain-model.md` / `10-orchestration.md` / `11-workspace.md` / `12-workflow-definition.md` — BR3.3、行単位は gap-measurement §2.1〜§2.4）。
  - 共有契約 3 本（`inception/domain-design/components.md` 全面 / `inception/contract-design/contract-summary.md` 節単位 / `inception/units-generation/unit-of-work.md` 注記のみ — BR3.7）。
  - `inception/domain-design/decisions.md` ADR-010 :476-478 への B13 失効注記 1 段落（追加実測 2 — BR3.7 (d)「変更しない」の訂正）。
- **前版（B4）で済んだもの**（再実施しない）: BR1.1〜BR1.5 / BR2.1 / BR2.4 / BR2.5 / BR3.1 / BR3.4 / BR4.1 / BR5.2（rules.md の状態 done）。`error-handling.md` は
  現行本文が正本（BR4.1）で、今回触るのは :4 の型名と :12 のマーカーだけ。
- **訂正 2 件（追加実測 1・2）を全ブリーフに明示**: ① RMU 呼出は「ポート・ユースケース・RMU は `async fn`、駆動ループ・`tokio::spawn` なし、合成ルートが
  await で直列に呼ぶ」と書き「同期」と書かない（ADR-006 は有効）。② ADR-010 :476-478 は削除せず `~~…~~ — 失効（2026-08-30 / B13、version は集約内
  `version()` / `with_version()`、`Rehydrated*` 撤去、`expected_version` 引数なし）` を追記。
- **レビュー所見の扱い**: nfr-requirements R-01 は却下（実測で反証 — 追加実測 3。ブリーフには gap-measurement §2 の表を 3 列丸ごと渡す）、R-02 は採用
  （CONSISTENCY-AUDIT を grep 範囲外 + 履歴マーカー 8 行 — オーナー裁定 2026-09-07 会話回答 A）。

## 2. 改訂対象 × 規則 × 派遣の写像

行番号は 2026-09-07 実測（探索の目安。派遣は同じ節の現在の内容を読む）。「処置」の詳細は gap-measurement §2 の該当行。

| 派遣 | ファイル | 箇所（行） | 処置 | BR |
|---|---|---|---|---|
| A | `coding-rules/README.md` | :50 | error-handling 行の「message-catalog」→ 出す側の `wording`（本文 :12 と同期） | BR1.6 |
| A | 同 | :10 / :12 | first-class-collections の追加告知 1 行を「規則が衝突したら」節から外し一覧表の同行へ畳む（表の規則行は 22 のまま）、「規則が 13 本」→ 現行件数 22。各行の一言・機械強制を本文と突合 | BR4.2 |
| A | `coding-rules/error-handling.md` | :4 | 適用例の型名 `StartError` / `SnapshotError`（不存在）→ 現行 `CommandError` / `ApplyError` / `IntentError` / `IntentExecutionError` / `PlanError` … | BR1.6 |
| A | 同 | :12 | 履歴マーカー付与のみ | BR1.6 |
| A | `coding-rules/factory-naming.md` | :47 / :84 | :47 反例「次 Bolt で是正」→ 履歴化（型ごと消滅）、:84 `WorkflowExecution::with_version` → `IntentExecution::with_version` / `WorkflowDefinition::with_version` | BR1.6 |
| A | 同 | :5 / :99 | 履歴マーカー付与のみ | BR1.6 |
| A | `coding-rules/gateway-taxonomy.md` | :223 / :299 | :223 集約名 → `IntentExecution`、:299 `core-domain` → `core-command-domain` | BR1.6 |
| A | 同 | :20 / :290 | 履歴マーカー付与のみ | BR1.6 |
| A | `coding-rules/module-visibility.md` | :11 / :21 / :12 | `core_domain::…` → `core_command_domain::…`（2 行）、:12 の `message_catalog::{state, lock, bolt}` 例 → 現行の共有語彙（`core_infrastructure::canon_json` 等） | BR1.6 |
| A | `coding-rules/use-case-rules.md` | :11 / :36 | :11 クレート名 → `core-command-use-case` / `core-command-interface-adapter`、:36 テストダブル記述 → コマンド側 `XxxRepositoryImpl<S>::in_memory()`、クエリ側 `InMemoryXxxDao` | BR1.6 |
| A | `coding-rules/command-query-separation.md` / `interior-mutability.md` / `field-visibility.md` | :5 / :5 / :46 | 履歴マーカー付与のみ | BR1.6 |
| A | `docs/specs/10-orchestration.md` | :3-15、:43-46、:48、:50-52、:55、:66-76、:70、:82-87、:93、:99-101、:105-106（+ ポート 2 行追加）、:110、:137 / :205-206 / :236 | 冒頭注記の畳み込み、§2.1 を `Intent` + `IntentExecution` の 2 集約へ分割（12 属性・15 コマンド + genesis・16 変種・memento なし・`replay(snapshot, events)`）、`next_decision(&Intent, &NextRequest) -> Result`、§2.2 に「所在 = クエリ側」列、§3 ユースケース列を実装名へ（未実装は予定）、テストダブル記述、ポート表 4 行 | BR3.3 |
| A | `docs/specs/01-domain-model.md` | :3-15、:96、:102、:106、:116、:121、:184 / :231、:278 | 冒頭注記の畳み込み、§3.2 集約段落の書き直し、§3.3 workspace 集約は予定、:121 脚注の履歴化、§7.1 原則 5 の参照例を 2 段に | BR3.3 / BR3.6 |
| B | `docs/specs/11-workspace.md` | :3-15、:41-49、:47 / :49、:101、:111-116、:126 / :195 | 冒頭注記の畳み込み、§2.1 集約 3 つは予定（orchestration の `Intent` と同名・別物を注記）、§3 ポート表 `IntentExecutionRepository`、供給面 4 つは予定、名称 | BR3.3 / BR3.2 |
| B | `docs/specs/12-workflow-definition.md` | :3-15、:77-93、:164-165 / :215 / :221、:178、:183 | 冒頭注記の畳み込み、§2.3 名称改訂 + クエリ 3 件（`scope_cost` / `review_policy` / `stage_route`）追記、§5 ユースケース表を実装名へ、`find_for_intent` 追記 | BR3.3 |
| B | `inception/domain-design/components.md` | 全面（`## Review` :430 以降は不変） | クレート 10 / 集約 4 / イベント 16・1・2・4 / クエリ側（DAO 14・`Find*` 13）/ RMU と `read_*` 17 表 / 再構成 = 差分再生（R-02 注記の訂正） | BR3.7 (a) |
| B | `inception/contract-design/contract-summary.md` | C1 / C3 / C4 / C5 / C6 / §4（`## Review` :486 以降は不変） | trait 全文を現行へ、`Rehydrated…` の履歴化、`WorkflowExecutionEvent` 11 → `IntentExecutionEvent` 16、message-catalog 3 件、`read_*` / `amadeus_read_model_head` / publication 表の注記。「同一シャード内の直接行と投影行の順序」は未決のまま残す | BR3.7 (b) |
| B | `inception/units-generation/unit-of-work.md` | :34 / :83 / :91 / :144 + :64（`## Review` :207 以降は不変） | U3 の独自スキーマ・`InMemoryWorkflowExecutionRepository` 記述に失効注記（本文は書き換えない）。:64 の ADR-010 注記「集約の外へ」は B13 で再失効しているため同形の追記 1 行（計画時実測で追加、設計 P4 の 4 行 + 1） | BR3.7 (c) |
| B | `inception/domain-design/decisions.md` | :476-478 | ADR-010 の当該段落を打消し線 + 失効注記（追記のみ。他の ADR は触らない） | BR3.7 (d) 訂正 |

## 3. 作法（security-design §2 の要約 — ブリーフに §2 全文を転記する）

最小変更（gap-measurement §2 の行に限り『維持』行は触らない）・改訂箇所の末尾に出典注記（形式 `（ADR-010）` / `（B13 2026-08-30）` /
`（実測 intent_execution.rs:40 / IntentExecution::replay）`）・逐語契約（監査イベント名 / CLI 語彙 / `AIDLC_*` / 逐語文言 / ファイル形式、`Directive` の kind 列挙と
JSON 形）は改変しない・失効は `~~旧文~~ — 失効（日付 / 出典）` で残し履歴行には必ずマーカー語（`~~` / 旧 / 失効 / 是正済み / 改名 / 履歴）を含める・
未実装は `予定（未実装、クリティカルパス n）`・RMU 呼出に「同期」を使わない・日本語正本（固定トークンは英語）・表は見出しと同じ列数（`\|` エスケープ）・
見出し重複なし・設計に無い判断が要ったら推測せず `developer-report-<n>.md` の「保留」に書く。

## 4. 委譲の形（security-design §3）

- **派遣 2 本、所有ファイルは非重複、並行**。派遣 A（`developer-brief-3.md` → `developer-report-3.md`）= coding-rules 9 ファイル + 10 号 + 01 号。
  派遣 B（`developer-brief-4.md` → `developer-report-4.md`）= 11 号 + 12 号 + components.md + contract-summary.md + unit-of-work.md（注記のみ）+ decisions.md
  （ADR-010 注記のみ）。番号 1 / 2 は B4（2026-08-23）の履歴として残す。
- **モデル**: 両派遣とも Opus（security-design §3 (g) — 10 号 §2.1 の 2 集約分割と 01 号 §3.2 の書き直し、components.md 全面は §4 台帳の読解を要する）。
- **ブリーフの必須事項**（§3 (a)〜(g)）: 1 行目 `AIDLC-UNIT: u9-canon-docs`、2 行目 `AIDLC-TESTING-CONTRACT: <contract_sha256>`、承認済みの本計画と
  `unit-test-instructions.md` の全文、security-design §2 全文、gap-measurement §2 の該当表を **3 列（所在 / 現行文言 / コード）丸ごと**、§4 台帳の該当節と §4.8、
  訂正 2 件、報告の形（改訂 1 件ごとに根拠列「コードの所在 / テストの有無 / 仕様の該当節」、添えられないものは保留）、禁止事項（push / PR / GitHub 書込、
  `AIDLC_*` でのフック回避、書込スコープ外、コードと `formal/`、`git commit`）、成果物の保存は Write / Edit 経由、受入 (2)(4) の自己実行と結果の貼付。
- **メインの責任**: Step 0 の基線、ブリーフ作成、派遣結果の diff 全件レビュー、受入検査 (1)〜(10) の実測、gap-measurement §2 との突合、統合の受入判断、
  `code-summary.md` / `source-manifest.json` / `traceability.json`、PR 本文への実測転記、収束ルールの運用。新しい方針や規範衝突はオーナー裁定へ上げる。

## 5. 実装ステップ（受入検査を赤→緑に）

Testing Contract の層（Data model / Repository / Business logic / API / Frontend）は文書だけの Unit には該当しない。runner_step の「Unit 限定コマンド」は
`unit-test-instructions.md` §1 の受入検査 (1)〜(9)（`cargo test` は実行しない — コード diff ゼロ）。各派遣は「Red = 受入検査を先に走らせて現状の赤を
`developer-report-<n>.md` に記録 → Green = 改訂 → Refactor = 出典注記・表整形・再検査」で進める。

### 5.0 メイン（承認後・派遣前）

- [ ] Step 0. **基線の実測**（`origin/main` = `e8ca4a5f` の作業ツリー）: (i) gap-measurement §1 の主要件数を再確認 — クレート 10（`Cargo.toml` members）、
      `IntentExecutionEvent` 16 変種、コマンド側ポート 4、FCC 16 型、`Rehydrated*` 0 件、`async fn` の RMU / ポート — 差があれば code-summary に記録しブリーフ補遺へ。
      (ii) 受入検査 (2) sentinel grep = 44 件、(3) 規則ファイル 22 = 表 22 + 索引ずれ 2 点、(7) 予定表記 0 件、(9) 用語 0 件、(6) `## Review` 節 3 本の sha256、
      (5) deviations diff 空 — を赤の基線として記録。(iii) `developer-brief-3.md` / `developer-brief-4.md` を Write。

### 5.1 派遣 A — coding-rules 9 ファイル + 10 号 + 01 号（開発エージェント、Opus）

- [ ] Step 1. Red: 所有ファイルに対する sentinel grep（履歴マーカー除外）の件数・行、README の規則行数と索引ずれ 2 点、10 号 / 01 号の `予定（未実装` 0 件を
      `developer-report-3.md` に記録。
- [ ] Step 2. Green: BR1.6 の 12 行置換 + 履歴マーカー 8 行、BR4.2 の README 同期、10 号 §2.1 / §2.2 / §2.3 / §3 / 冒頭注記 / 名称行、01 号 §3.2 / §3.3 / §7.1 /
      :121 / 冒頭注記（gap-measurement §2.1 / §2.4 / §2.5 の『処置』列どおり、§4 台帳 O1〜O15 / P1〜P9 / S4 / K1 を出典に）。
- [ ] Step 3. Refactor: 出典注記・表の列数・見出し重複を整え、Step 1 の検査を再実行 — 所有ファイルの sentinel = 0、README 22 = 22 かつ索引ずれ 0、
      10 号 / 01 号の未実装項目に予定表記、「同期」0 件。根拠列つきで報告。

### 5.2 派遣 B — 11 号 + 12 号 + 共有契約 3 本 + decisions.md（開発エージェント、Opus）

- [ ] Step 4. Red: 所有ファイルの sentinel grep 件数・行、`## Review` 3 節の sha256、11 号 / 12 号の `予定（未実装` 0 件、decisions.md :476-478 の現文を
      `developer-report-4.md` に記録。
- [ ] Step 5. Green: 11 号 §2.1 / §3 / 供給面 / 名称、12 号 §2.3 / §5 / :183 / 名称、components.md 全面（R1〜R7 / P6 / K1〜K6 を出典に。`## Review` 以降は不変）、
      contract-summary C1 / C3 / C4 / C5 / C6 / §4（`## Review` 以降は不変）、unit-of-work U3 注記 5 行、ADR-010 失効注記 1 段落。
- [ ] Step 6. Refactor: 出典注記・表整形・見出し重複を整え、Step 4 の検査を再実行 — 所有ファイルの sentinel = 0、`## Review` 3 節の sha256 が Step 4 と同一、
      予定表記あり、「同期」0 件、decisions.md の diff が ADR-010 の追記だけ。根拠列つきで報告。

### 5.3 メイン（統合）

- [ ] Step 7. 派遣 2 本の diff を全件レビュー（gap-measurement §2 の『処置』列と突合、『維持』行の不変を確認、根拠列の無い改訂は差し戻すか保留に）。
      受入検査 (1)〜(9) を全体で実測（`unit-test-instructions.md`）。`code-summary.md`（実測結果・保留・設計質問）/ `source-manifest.json` / `traceability.json` を書く。
- [ ] Step 8. advisory レビュー（アーキテクチャレビュアー、review_artifact = 本計画）→ READY で Unit 完了 → `git add -A` で記録 + 文書を 1 コミット
      （コミット名 = `u9-canon-docs`）→ push → PR（本文に受入 (1)〜(9) の実測を貼る）。
- [ ] Step 9. 受入検査 (10): CodeRabbit のスレッドを全件返信 + resolve、CI 7 ジョブ緑、収束条件（必須 CI green ∧ unresolved = 0 ∧ 全コメント返信済み）を
      最新 head で再実測してから merge queue へ（オーナー包括承認 2026-08-29 により AI 裁定でよい）。

## 6. トレーサビリティ（要求 → 規則 → ステップ）

| 要求 | BR | ステップ |
|---|---|---|
| FR8.1 | BR1.6, BR4.2 | 1〜3 |
| FR8.2 | BR3.3, BR3.7, BR3.2, BR3.6 | 2〜6 |
| FR9.6 | BR4.2（error-handling 行の同期） | 2〜3 |
| NFR1.1 / NFR1.3 / NFR1.5 | BR5.2, BR3.7 | 2, 3, 5, 6, 7 |
| NFR1.2 | BR5.1 (5) | 0, 7 |
| NFR1.4 | BR3.2, BR3.3（予定表記） | 2, 5, 7 |
| NFR2.1〜NFR2.4 | BR5.1 (1)〜(4), (10) | 0, 3, 6, 7, 9 |
| NFR2.5 | BR5.2 | 2, 5 |
| NFR2.6 | BR5.3（3 列丸ごと・根拠列・実測突合・用語） | 0, 3, 6, 7 |

## 7. 前版（B4、2026-08-23）からの変更

- 改訂対象を entities.md の 10 ファイル（B4 で完了）から、再走の確定版 — coding-rules 20 行 9 ファイル / 仕様 4 号（deviations は触らない）/ 共有契約 3 本 /
  ADR-010 注記 — に置き換えた。sentinel は 7 語 → 10 語（`WorkflowExecution` / `RehydratedWorkflowExecution` / `message-catalog` を追加）、範囲から
  `CONSISTENCY-AUDIT-*.md` を除外、履歴マーカー除外条件を明文化。
- 受入検査を 6 → 10 項目（`## Review` 節のバイト同一、予定表記、実測表との突合、用語、CI 7 ジョブ）。
- 委譲の境界を security-design §3 に従い A（coding-rules + 10 号 + 01 号）/ B（11 号 + 12 号 + 共有契約 + decisions）に組み替え、ブリーフ必須事項 (a)〜(g)
  （3 列丸ごと・台帳が正本・訂正 2 件・根拠列・禁止事項・自己検査・Opus）を計画に固定。
- ブランチを `bolt/b4-u9-canon-docs` から現行の `stage1-selfhost` へ。Step 0 に「現行 HEAD での基線再実測」を追加（台帳の基準 `02cacea2` と `origin/main` の差の吸収）。

> 注: 本 Unit はプロダクションコードを持たないため、層ごとの Red / Green / Refactor は受入検査の赤→緑として運用し、既存スイートは diff ゼロで緑のまま（§5 冒頭）。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "新規プロダクションコードはレイヤーごとに red-green-refactor",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 新規プロダクションコードはレイヤーごとに red-green-refactor\n  （失敗するテストを先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・\n  ゴールデンパリティは TDD サイクルの外側の受け入れゲートとして維持し、\n  TDD の red を代替しない。（インタビュー Q2、選択肢 A で確定——品質レビュー\n  の自己完結化置換案どおり）\n\nテストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識した配分とする\n（オーナー明言）。比率は**定性のみ**とし、数値目標は定めない（インタビュー\nQ3、選択肢 A）: 単体テスト優位・統合テストは境界ごと・E2E は最小、という\n配置規則で充足する。\n\nこのプロジェクトは TDD の上に **3層の品質保証** を重ねている点が特徴的で、\nそれぞれ役割が異なる（`code-quality-assessment.md` §品質保証の全体像より）:\n\n1. **Quint 形式検証**（毎 PR）— 決定論コアの状態機械契約そのものを検証。\n   不変条件 run 27本・到達性 witness 12本の反転判定・決定的シナリオ。\n   モデルの検査力自体も mutation テストで証明済み（engine_loop 3/3、\n   audit_lock 10/10 + witness 7/7、stop_hook 7/7）。\n2. **ITF 準拠テスト**（`modules/core/domain/tests/`、engine_loop / audit_lock\n   の2モデル・2ファイル）— Quint モデルのトレースを集約に再生し状態射影を\n   突き合わせることで、モデルと実装の乖離を検出。TDD の「テストを先に書く」\n   対象は実装コードだが、契約の正本は Quint 側にあるため、ITF 準拠テストは\n   実装後に契約適合を機械確認する位置づけ（TDD サイクルの red-green-refactor\n   そのものではなく、その外側のゲート）。なお stop_hook は ITF 準拠テストが\n   未整備（既知の穴、`evidence.md` インタビュー未確定事項 (e) 参照）。\n3. **PBT（proptest）+ ゴールデンパリティ**— upstream 配布実バイト33ノードの\n   全数 load パリティを固定し、upstream 互換の逸脱を検出。\n\nしたがって TDD サイクルは主にユニットテスト層（インライン `#[cfg(test)]`、\n実測**40ファイル**——集計方法: `modules/` 配下・`tests/` ディレクトリを除いた\nインライン `#[cfg(test)]` 数。`tests/` 配下6本（ITF準拠2 + 統合4）を含めると\n46、`tools/lint/src/check.rs` を含めても47であり、いずれの集計でも48には\nならない。開発者レビュー指摘どおり40へ訂正した）に適用し、ITF 準拠テスト・\nゴールデンパリティはレイヤー横断の受け入れ確認として TDD サイクルの外側に\n位置づける。\n\n- **カバレッジ**: 絶対ゲート90%床 + PR 相対ゲート（head が base を下回ったら\n  fail、許容誤差 0.5pp。PBT のシード非固定に起因するノイズ較正値であり、\n  stage-1 スコープで**シード固定により 0.01 へ引き締める**——インタビュー\n  Q7、選択肢 A/B。除外設定は現状無いが、**composition root（`main.rs` の\n  配線部分）のみカバレッジ除外を許可**し、それ以外は床を維持する\n  （インタビュー Q5、選択肢 B。除外設定は `scripts/coverage.sh` への確定\n  アクション、`evidence.md` 参照）。実測 94.87〜95.29%（`scripts/coverage.sh`）。\n- **ツーリング**: `cargo test --workspace`（234テスト全緑、実測）、\n  `cargo-llvm-cov`、Quint 0.32.0（Node 22 経由）。\n- **テスト種別**: ユニット（インライン `#[cfg(test)]`）、PBT（proptest、集約\n  本体同居）、ITF 準拠（`modules/core/domain/tests/` 2本）、統合（\n  `modules/core/interface-adapter/tests/` 4本 — ゴールデンパリティ・FS ロック・\n  Repository 実装・シンボリックリンク防御）。\n- **CI ゲート**（`main` へのマージ条件、実測）: `check` ジョブ（`cargo fmt\n  --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →\n  `cargo lint` → `cargo test --workspace`）、`quint` ジョブ\n  （`scripts/quint-gate.sh`）、`coverage` ジョブ（`scripts/coverage.sh`、\n  絶対90%床 + PR 相対ゲート）の3ジョブすべてを緑にする。この3ジョブは\n  **stage-1 スコープで branch protection の required status checks として\n  機械強制する**（インタビュー Q4、選択肢 A——現状は運用規律のみで機械強制が\n  無いという品質レビューの重大指摘を受けての裁定。設定作業は\n  `evidence.md` の確定アクションに記載）。\n- **スコープ注記**: `tools/lint`（`cargo lint` の実装クレート）は workspace\n  非メンバーの detached クレートであり、CI の fmt/clippy/test がまだ届いて\n  いない（設計監査 C27）。**stage-1 スコープに含める**: `tools/lint` への\n  CI 3ステップ（fmt/clippy/自己テスト）追加（インタビュー Q7、選択肢 A）。\n  macOS CI ジョブ追加・`main` への push トリガー追加は本 intent には\n  含めず、後続 intent へ繰り延べる（インタビュー Q7、選択肢 E 相当の一部\n  不採択）。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:e4f36aa113753d3604df570f5ec3a0cb465d4b29d82a17a16efbb2ea8b993111",
  "contract_sha256": "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
}
```

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T03:27:03Z
**Iteration:** 1

レビュー種別は advisory（1 回のみ、所見は承認ゲートで人間が重み付けする。修正→再レビューの往復は前提にしない）。
すべての所見は改訂後の実ファイルと現行コードを自分で実測して裏取りした。裏取りできなかった疑いは Minor / Info に留め、理由を書いた。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/traceability.json` の coverage 配列（`target` フィールド） | components.md の全面改訂でコンポーネントが改名・統合・解消されたのに、同じ `inception/domain-design/` にある traceability.json の `target` が旧名を指したまま残っている。実測: `EngineUseCases`（FR2 / FR2.1 / FR2.2 / FR3 / FR3.2 / FR5 / FR5.1〜FR5.4 / FR6 / FR6.1）、`PersistenceGateways`（FR1 / FR1.1 / FR1.2 / FR1.3 / NFR3）、`CanonJson`（FR3.2 / FR7 / FR7.1 / FR7.3 / NFR1）、`PublishedLanguage`（FR3.2 / FR4.2 / NFR1）。改訂後の components.md が定義するのは OrchestrationEngine / WorkflowDefinitionModel / WorkspaceModel / CommandUseCases / CommandGateways / QueryUseCases / QueryGateways / ReadModelUpdater / CliDispatcher / CoreInfrastructure / HarnessClaude / HarnessInfrastructure の 12 個であり、上の 4 名はどれも現行のコンポーネント名ではない。さらに保全された components.md の `## Review` 節が Validation Tool Results に「target 名解決（traceability.json → components.md）— 一致」と記録しているため、その記録が改訂後は成立しない。旧名は `~~EngineUseCases~~` のように打消し線の履歴として本文に残るので、素朴な部分一致の突合では通ってしまい、要求が退役済みコンポーネントを静かに指し続ける | traceability.json の `target` を現行名（`EngineUseCases`→`CommandUseCases`、`PersistenceGateways`→`CommandGateways`、`CanonJson`→`CoreInfrastructure`、`PublishedLanguage`→解消先の記述）へ更新するか、更新を後続へ回すなら旧名→新名の対応表を traceability.json 側に注記して繰延を明示する。計画 §2 の改訂対象 17 ファイルに本ファイルが入っていないことが原因なので、スコープの 1 ファイル追加として扱う | New |
| R-02 | Minor | `unit-test-instructions.md` §1 (1) / (5) / (6) の後半コマンド | 受入コマンドが `git diff … origin/main..HEAD`（コミット間の比較）で書かれているが、改訂は Step 8 のコミット前に検査される運用（計画 §5.3 Step 7 → Step 8）。実測すると `git diff --stat origin/main..HEAD -- docs/specs …` は空、`git diff --stat origin/main -- …` が 17 files / +817 / −425 を返す。つまり指示どおり `..HEAD` で走らせると、コード変更ゼロ (1)・deviations 不変 (5)・decisions.md の diff (6 後半) はコミット前に常に空になり、未コミットのコード混入を検出できない。実際にメインが使ったのは作業ツリー比較の `origin/main` 形（code-summary §2 の 1 行目のコマンド）で、結果自体は正しい | `..HEAD` を外して作業ツリー比較にする、または「コミット後に実行する検査」と明記する。指紋済みのため pending-revision へ 5 番目の項目として追記する | New |
| R-03 | Minor | `docs/specs/11-workspace.md` :63（`StateVersion` の行）と :183（W7 の行） | gap-measurement §4.7 は `doctor` を未実装として挙げるが、この 2 行は「runtime と doctor が同一関数を使う」と現在形で書かれ、`予定` 表記も履歴マーカーも無い。受入 (7) の第 2 コマンドの残存行としてメインは把握しており、code-summary §2 の脚注で「11 号の値オブジェクト・Presenter・未決事項（:63 / :183 …）」に分類して許容している。W7 は装置 E1 の設計不変条件（こう作れ）という読みは成り立つので、実装状態の主張と断定はできない | 判断は保留でよいが、`doctor` 側だけ `（doctor は予定（未実装、クリティカルパス 6））` のような 1 語の注記を足せば読み違いが消える。次の改訂候補として申し送りに残す | New |
| R-04 | Minor | `../functional-design/gap-measurement-20260907.md` §1 / §4（P4 / R1 / R4 / P9 / O15） | 「正しい姿の正本」と宣言された台帳に、code-summary §4 の T1 / T3 / T6 / T7 / T8 の 5 件の誤りが残ったままである。私の実測でも台帳側が誤り: use-case クレートに `#[cfg(test)]` の `pub(crate)` フェイク 4 つが実在（`test_support.rs:241 / 388 / 622 / 770`、`mod.rs:38-39` で gate）、`JournalReader` は `async fn` 8 + 同期 `fn prepare_read_model` 1、publication 系は 6 表、`Face` は 5 変種、`StateBinding` はコマンド側とクエリ側の両方に同名型が実在する。改訂後の 17 文書は訂正後の実測に合わせてあるため、いま台帳を読む人は文書と正本が食い違う状態を見る | code-summary §7.1 の申し送りどおり、functional-design ゲートで台帳と rules.md へ 5 件を折り戻す。折り戻しが済むまでは gap-measurement の該当行に「T1 / T3 / T6 / T7 / T8 で訂正済み（code-summary §4 参照）」の 1 行を置くと誤読を防げる | New |
| R-05 | Info | `code-generation-plan.md` §1（前提と範囲）の 3 行目 | 「`origin/main` は `e8ca4a5f`（#117）まで進んでいる」は誤り。実測 `git rev-parse origin/main` = `02cacea2…`（`e8ca4a5f` はその親）で、台帳の基準そのものである。計画は指紋済みのため本文は据え置き、code-summary §1 と §4 T10、pending-revision 項目 4 に訂正が記録されている。Step 0 の「現行 HEAD での基線再実測」は差分ゼロの確認として実行されており、結論に影響は無い | 追加作業は不要。ゲートで人間が「承認済み計画の本文に誤記が残る」ことを認識すればよい | New |
| R-06 | Info | `code-generation-plan.md` §1 の「coding-rules 20 行 9 ファイル」 vs `code-summary.md` §3 の棚卸し | 計画は coding-rules を 20 行に固定していたが、実際の改訂は 24 件（README 4 / error-handling 2+2 / factory-naming 4 / gateway-taxonomy 4 / module-visibility 3 / use-case-rules 2 / 残り 3 ファイル各 1）。超過分は code-summary §5「台帳に無い行の改訂」でメインが名称のみ・注記のみと判定して採用したと開示されている。ファイル数 9 は計画どおりで、私の実測でも coding-rules の変更は 9 ファイル | 開示済みなので追加作業は不要。件数が計画の固定値を超えたことをゲートで確認する | New |
| R-07 | Info | `source-manifest.json` | 改訂 17 ファイルのうち `docs/specs` の 4 本しか載っていない。理由は code-summary §3 に記載（`aidlc/` 配下はフレームワークが記録ツリーとして manifest から除外し `aidlc-log review` が拒否する）で、17 ファイルの全数列挙は code-summary §3 の表が担う。設計として妥当だが、manifest 単独では改訂範囲が読めない | 追加作業は不要。code-summary §3 が全数の正本であることは既に本文に明記されている | New |

### 独立検証

すべてリポジトリルートで実行。`origin/main` = `02cacea2`（実測）。改訂は未コミットのため、コミット間比較ではなく作業ツリー比較（`git diff origin/main -- <path>`）で測った。

| # | 検査 | 実行したもの | 実測結果 | 計画・報告との一致 |
|---|---|---|---|---|
| 1 | コード変更ゼロ | `git diff --stat origin/main -- modules tools scripts .github Cargo.toml Cargo.lock formal` と `-- docs/specs/research` | どちらも出力なし | 一致 |
| — | 改訂規模 | `git diff --stat origin/main -- docs/specs aidlc/…/knowledge aidlc/…/inception` | 17 files changed, 817 insertions(+), 425 deletions(-) | code-summary 冒頭と完全一致 |
| 2 | sentinel 10 語 grep | `unit-test-instructions.md` §1 (2) のコマンドをそのまま | 改訂後 **0 件**。基線は `git show origin/main:<file>` で復元して同じ grep をかけ **40 件**（coding-rules 12 = README 2 / command-query-separation 1 / error-handling 1 / factory-naming 4 / field-visibility 1 / gateway-taxonomy 2 / interior-mutability 1、10 号 9、01 号 6、11 号 5、12 号 8） | code-summary の基線 40 と内訳が完全一致。developer-report-3 の派遣 A 分 27 件（12 + 9 + 6）とも整合 |
| 3 | README の無矛盾 | 規則ファイル数・表の行数・件数記載の 3 コマンド | 22 / 23（good-examples 1 行を含むので規則行 22）/「規則が 22 本」の 1 行のみ。`2026-…追加:` の告知行は出ない | 一致（索引ずれ 2 点は解消） |
| 4 | 表の列数・見出し重複 | §1 (4) の `python3` スクリプトを 17 ファイルに対して実行 | `tables ok` | 一致 |
| 5 | 逸脱登録の維持 | `git diff --stat origin/main -- docs/specs/deviations.md` | 出力なし | 一致 |
| 6 | `## Review` 節の履歴保全 | 3 本について `git show origin/main:<f>` と現ファイルの `## Review` 以降を shasum -a 256 で比較 | components `b954159a…` / contract-summary `7374b976…` / unit-of-work `b8d0e4a1…` の 3 値が基線と同一（見出し行は :430→:663 / :486→:593 / :207→:207 へ移動）。`decisions.md` の diff は ADR-010 の 1 段落のみで、`-` 2 行の内容は `~~` 付きで `+` 側に残る | 基線 3 値・移動先行番号とも一致 |
| 7 | 実装状態の表記 | `予定（未実装` の件数と、未実装項目の言及で予定表記も履歴マーカーも無い行の列挙 | 01 号 1 / 10 号 8 / 11 号 10 / 12 号 1。形は `予定（未実装、クリティカルパス 4）` 12 件、`… 4 = マルチコール CLI + 文言カタログ配線` 7 件、`… swarm はスコープ外` 3 件。第 2 コマンドの残存行は code-summary の分類どおり（10 号 :183 は直上に「本節全体が予定（未実装、swarm はスコープ外）」がある） | 一致。`doctor` の 2 行だけ R-03 として残す |
| 8 | 実測表との突合 | gap-measurement §2 の『維持』行を base と head で diff | 11 号 §2.2 / §2.3、12 号 §2.1 は完全に同一。10 号のトランザクション境界行（同一 Tx / 楽観 version）も文言不変 | 一致 |
| 9 | 用語 | RMU 文脈（rmu / catch_up / 投影）の「同期」 | 0 件 | 一致 |
| 10 | レビュー | PR 未作成のため対象外 | — | — |

コードで実否を確認した主張（`modules/` を直接 Read / grep）:

- クレート 10（`Cargo.toml` members の実測列挙）。
- `IntentExecution` は 12 属性（id / intent_id / slots / cursor / status / parked_at / autonomy / skeleton_stance / last_gate_resolution_at / seq_nr / version / last_updated_at）。`&mut self` の公開関数は 17 で、内訳は 15 コマンド + 再生用の `apply_event` / `apply_report`。genesis は関連関数 `start`（`intent_execution.rs:225`）。台帳 §1 の「15 + genesis（＋ `apply_event` / `apply_report`）」と一致。
- `IntentExecutionEvent` は 16 変種（`Started` から `PracticesAffirmed` まで実際に列挙して確認）。contract-summary C5 の列挙と一致。
- `next_decision` の署名は `(&self, intent: &Intent, request: &NextRequest) -> Result<NextDecision, CommandError>`（`intent_execution.rs:1864`）で、`matches` 不成立時に `CommandError::IntentMismatch`。10 号 :60 / :89 の記述と実測所在が一致。
- コマンド側ポートは 4（`intent_execution_repository` / `intent_repository` / `workflow_definition_repository` / `compiled_definition_repository`）。
- `JournalReader` は `async fn` 8 + 同期 `fn prepare_read_model(&mut self)` 1（`journal_reader.rs:38`）。T3 のとおり。
- `CREATE TABLE IF NOT EXISTS` の実測は distinct 26 = `read_*` 17 + `amadeus_*` 8 + `journal` 1。publication 系は 6 表（`amadeus_publication` / `_file` / `_history` / `_history_file` / `_snapshot` / `_snapshot_file`）で、contract-summary :465-467 に 6 表とも全数列挙されている。T6 のとおり。
- `Face` は 5 変種（Orchestrate / Utility / Log / State / Bolt。Orchestrate が `aidlc-orchestrate` と素の `aidlc` を兼ねる）。T7 のとおり。
- `test_support.rs` に `pub(crate)` のフェイク 4 つ（:241 WorkflowDefinition / :388 CompiledDefinition / :622 IntentExecution / :770 Intent）、`mod.rs:38-39` の `#[cfg(test)]` で gate。公開のインメモリは `in_memory()` を持つアダプタ 3 実装。T1 の 3 層構造のとおり。冒頭 doc の「ここに置くのは 1 つだけ」が実態 4 つとずれる点も再現（code-summary §7.3 の別 Bolt 候補）。
- `CompiledDefinitionRepositoryImpl` は非ジェネリックで `in_memory()` を持たない（:408）。T9 とメインの追記のとおり。
- `StateBinding` は `command/domain/src/orchestration/state_binding.rs` と `query/use-case/src/orchestration/state_binding.rs` の両方に実在。T8 のとおりで、台帳 O15 の「クエリ側に住む」は誤り。
- DAO trait 14 / `InMemory*Dao` 13 / `Find*` 13。components.md の記述と一致。

作法の遵守（逐語契約の不変を base と head の比較で確認）:

- `AIDLC_*` 環境変数は 11 種・出現回数まで base と head で完全一致（改変ゼロ）。
- `DirectiveKind` の「10 種の閉集合」とその綴り、placeholder 2 件の注記は逐語のまま。10 号 §2.2 の表は「所在」列を 1 列足しただけで、kind の列挙・JSON 形・28KiB は 1 バイトも変わっていない。
- 失効は削除ではなく `~~旧文~~ + 失効注記` で残っている（decisions.md ADR-010、unit-of-work の 5 箇所、components.md の旧コンポーネント名）。履歴行にマーカー語がある証拠は受入 (2) が 0 件で緑になったこと（除外条件が `~~` / 旧 / 失効 / 是正済み / 改名 / 履歴 のいずれかを要求するため）。
- unit-of-work.md は本文を書き換えず 5 行とも追記のみ（diff の `-`/`+` 対はいずれも既存文が `+` 側にそのまま残る形）。
- 日本語正本で固定トークン（型名・関数名・ファイルパス・CLI 動詞）は英語のまま。

委譲の規律:

- `developer-brief-3.md` / `-4.md` とも 1 行目 `AIDLC-UNIT: u9-canon-docs`、2 行目 `AIDLC-TESTING-CONTRACT: sha256:303d9bb7…`（計画の Testing Contract `contract_sha256` と一致）、`Conversation language` 行、gap-measurement §2 の 3 列表、訂正 2 件、根拠列の指示、禁止事項を備える。
- `developer-report-3.md` は改訂 A1〜A34 を「出典注記 / 根拠（コードの所在・テスト・仕様節）」の列つきで、`developer-report-4.md` は 45 行の表で報告している。根拠を添えられなかった改訂は両者とも 0 件という code-summary §5 の記述と整合する。
- メインの追加訂正 3 件はいずれも妥当。(a) `ApplyError` が `pub(crate) enum`、`IntentExecutionError` が `pub struct` である点はコードで確認でき、「公開 enum 33 本の代表」に混ぜるのは誤りだった。(b) 重複マーカーの除去は無害。(c) `CompiledDefinitionRepositoryImpl` の例外追記は R-04 の T9 として実測どおり。

### 判定理由

advisory クラスで Critical 0・Major 1 のため **READY**。判定規則（Critical が 1 件でもあるか Major が 3 件以上なら NOT-READY）に照らして READY だが、advisory の本パスは修正ループを持たないので、Major の R-01 は承認ゲートで人間が明示的に扱う所見として上げる。

READY と判断した積極的な根拠は次の 3 点である。

1. **受入検査 (1)〜(9) を自分で実行し、9 項目すべてが緑であることを再現した**。基線（40 件の sentinel、README の索引ずれ 2 点、予定表記 0 件、`## Review` 節の sha256 3 値）は `git show origin/main:<file>` から復元して独立に測り直しており、報告された赤→緑の遷移は実在する。派遣の自己申告をそのまま受けた項目は無い。
2. **「正しい姿」の主張がコードと一致する**。10 号 §2.1 の 2 集約分割（12 属性 / 15 コマンド + genesis / 16 変種 / memento なし / 差分再生）、`next_decision` の完全署名、ポート 4、publication 6 表、`read_*` 17 表、`Face` 5、DAO 14 / `Find*` 13 を `modules/` で直接確認し、すべて改訂後の記述どおりだった。記録の転記ではなくコード実測が正本になっているという BR5.3 / NFR2.6 の要求は満たされている。
3. **保存すべきものが保存されている**。逐語契約（`AIDLC_*` 11 種・`DirectiveKind` の 10 種閉集合・28KiB 上限・JSON 形）は base と head でバイト同一、`## Review` 節 3 本は sha256 が同一、gap-measurement §2 が『維持』と指定した節（11 号 §2.2 / §2.3、12 号 §2.1、10 号のトランザクション境界）は diff がゼロ、コード・`formal/`・`docs/specs/research`・`deviations.md` の変更もゼロである。

READY を弱める事情も明記する。R-01 は本 Unit の改訂が原因で生じた参照切れであり、`inception/domain-design/` の中で components.md と traceability.json が食い違う状態のまま Unit が閉じる。旧名が打消し線の履歴として components.md 本文に残るため機械的な部分一致は通ってしまい、壊れていることが検知されにくい。改訂対象を 17 ファイルに固定した計画 §2 の副作用であり、1 ファイルの追加で解消できる。R-04 は「正本」と宣言された台帳の側に既知の誤りが 5 件残る問題で、code-summary §7.1 が折り戻しを申し送っているものの、それが済むまでは台帳を読む後続の作業者が誤った基準を使う危険が残る。いずれも実装を止める性質のものではないため Critical には上げていない。
