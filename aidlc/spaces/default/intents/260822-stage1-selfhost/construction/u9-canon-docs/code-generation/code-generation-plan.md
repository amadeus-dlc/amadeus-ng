# code-generation-plan — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Code Generation（Construction 3.5）の計画（Unit: U9、kind: spec、Bolt: B4、規模 S）。出典: `../functional-design/rules.md`（BR1.1〜BR5.2）、
> `../functional-design/entities.md`（改訂対象 10 ファイル）、`../functional-design/pending-revision.md`（項目 1〜4 — 本計画に取り込む）、
> `../nfr-requirements/security-requirements.md`（NFR1.1〜1.3 / NFR2.1〜2.5）と `pending-revision.md`、`../nfr-design/security-design.md`
> （§2 作法 / §3 受入チェックリスト / §4 逸脱登録行）と `pending-revision.md`、`../../../inception/contract-design/contract-summary.md`（C4 / C5）、
> `../../../inception/units-generation/unit-of-work.md`（U9）、`../../../inception/requirements-analysis/requirements.md`（FR8.1 / FR8.2 / FR9.6）、
> `../../../inception/delivery-planning/bolt-plan.md`（B4）、`../../../inception/domain-design/decisions.md`（ADR-001〜008）。
>
> **本 Unit はコードを書かない。** 「生成」の対象は正本・仕様の文書であり、TDD の赤→緑は「受入検査（grep / diff / 行数）を先に走らせて赤を
> 記録し、改訂で緑にする」と読み替える。既存テストスイートは触らない（diff ゼロ）。

## 1. 前提と範囲

- **ブランチ / PR**: `bolt/b4-u9-canon-docs`（`origin/main` 起点、作成済み）。PR は 1 本直列、squash-merge、コミット名 = Bolt slug。
  aidlc 記録 → 文書コミットの順。レビューボット（CodeRabbit）の指摘は全件返信 + resolve（review-thread gate）。
- **コード変更ゼロ**: `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` が空（BR5.1 (d) を安全側に統一 —
  FD pending-revision 項目 4、NFR2.1）。`docs/specs/research/**` も変更ゼロ（NFR1.1）。
- **改訂対象（entities.md）**: coding-rules 4（`use-case-rules.md` / `gateway-taxonomy.md` / `error-handling.md`（新規）/ `README.md`）、
  仕様 5（`docs/specs/01-domain-model.md` / `10-orchestration.md` / `11-workspace.md` / `12-workflow-definition.md` / `deviations.md`）、
  設計目録 1（`inception/domain-design/components.md`）。
- **取り込む pending-revision**（設計の穴を実作業で踏まないため、計画で確定する）:
  1. BR2.5 の適用範囲 = 12 号の `next_in_scope_stage` 全 5 出現（§2.3 ×2 / §4 未知スコープ表 / §8 F2 行 / §9 ユビキタス言語例）。
  2. BR1.5（新設）: `gateway-taxonomy.md` §1b の模範例を「非 Repository ポートの一般形（契約の意味論を型に載せる — 予算・再入・二重解放不能を
     型で表現）」へ再構成し、`WorkspaceLock` への依存を外す（退役の旨を 1 行注記 — ADR-007）。
  3. BR5.1 (c) の grep 範囲 = `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md` + `docs/specs/*.md`（`research/` 除外）。sentinel は 7 語
     （`effective_plan_action` / `next_in_scope_stage` / `AuditLedgerRepository` / `AuditLedgerService` / `StateFileStore` / `report_forward` / `gate_start`）。
     `StageGraphReader` は除去対象の BR が無く、gateway-taxonomy『適用の帰結』の旧→新移行表（旧列 = 履歴注記）にしか現れないため sentinel から外す
     （nfr-design pending-revision 1 の是正文言）。
  4. BR5.1 (d) の diff スコープ = 上記 Cargo.* まで。
- **語の区別**: sentinel の `effective_plan_action`（削除済み Rust API、snake_case）と、仕様が使う upstream 語 `effectivePlanAction`（合成読みの
  ユビキタス言語）は別物。後者は残してよいが、所有者は集約 `WorkflowExecution`（`effective_plan` — BR3.3 (c)）と明記する。
- **U2 設計の出典鮮度**: BR3.3 が引く U2 機能設計（`../../u2-domain-es-core/functional-design/*.md`）は pending-revision 未適用。実装（`modules/core/
  domain/src/orchestration/`、Bolt B3 = `origin/main`）を一次出典にし、名称は U2 pending-revision の承認済み改名（`WorkflowExecutionSnapshot` →
  `WorkflowExecutionState`、B5 で改名）を仕様の規範名として書き、現行コード名を括弧で注記する。`IntentId` は UUIDv7（Q2 = A）、現行実装の
  dirName 受理は B5 で是正と脚注。

## 2. 改訂対象 × 規則の写像

| ファイル | 箇所 | BR | 出典の注記（括弧書き） |
|---|---|---|---|
| `coding-rules/use-case-rules.md` | §4 `repository.load()` → `repository.find_by_id()` | BR1.1 | （C4 改訂 2026-08-23） |
| `coding-rules/gateway-taxonomy.md` | §4 散文「load / save」→「find / save」、§5 末尾「load / save の指揮」→「find / save の指揮」 | BR1.2 | （設計監査 C2） |
| 同 | §2b に ES Repository の拡張語彙 `store(event, aggregate)` / `find_by_id` の 1 段落（event-store-adapter-rs 同形） | BR1.3 | （ADR-006） |
| 同 | §2 実例リストの `AuditLedger → AuditLedgerRepository` 行を削除し「`AuditLedger` はイベントログ（ADR-001 / 003）であって集約ではない」の 1 行注記 | BR1.4 | （ADR-001 / 003 / 006） |
| 同 | §1b を非 Repository ポートの一般形へ再構成（`WorkspaceLock` は退役の注記） | BR1.5 | （ADR-007） |
| `coding-rules/error-handling.md`（新規） | Q1 = A の文面を既存書式（裁定日 / 適用例 / 機械強制 / ルール / 根拠 / 対象外）で | BR4.1 | 裁定日 2026-08-23 |
| `coding-rules/README.md` | 一覧表に `error-handling.md` 行を追加、gateway-taxonomy 行の一言を BR1.x と同期（行数 = 7） | BR4.2 | — |
| `docs/specs/deviations.md` | 表に # 4 行（security-design §4 の文面）、予約節の整理 | BR3.4 | 2026-08-23 / ADR-003, ADR-007 |
| `inception/domain-design/components.md` | `WorkspaceModel` → workspace 語彙（値オブジェクト）へ縮退、`ReadModelUpdater` に描画責務を追加 | BR3.5 | （オーナー裁定 2026-08-23） |
| `docs/specs/01-domain-model.md` | §3.1（`WorkflowDefinitionId` / `DefinitionRevision`、集約ルート）、§3.2（ES 形 FSM、PlanAction は workflow_definition 所有）、§3.3（集約 = Intent / Space / Worktree、リードモデル = StateFile / AuditShard、WorkspaceLock 退役、`IntentId` UUIDv7 / `IntentDirName`）、§7 ドメインモデルの原則 (1)〜(6) | BR2.2 / BR2.4 / BR3.1 / BR3.2 / BR3.6 | （ADR-001〜008 / オーナー裁定 2026-08-23） |
| `docs/specs/10-orchestration.md` | §2.1 を ES 形に（16 属性・12 イベント・decide / apply_event / state / from_state・`next_decision` Result と DefinitionMismatch・gated = phase ≠ initialization・Started 自己完結・effective_plan 所有）、§2.2 PlanAction を所有元参照行に、§3 ポート表（同上廃止、1 trait 1 Impl、`find_by_id`、AuditLedgerRepository / WorkspaceLock 行削除、WorkflowExecutionRepository 実装欄の書き換え）、§8 の in-memory 一式の記述を現行ポートへ | BR2.3 / BR2.4 / BR3.1 / BR3.3 | （ADR-002 / 004 / 005 / 008、Bolt B3、C3 / C4 / C5） |
| `docs/specs/11-workspace.md` | §2.1 集約（StateFile / AuditShard はリードモデル、WorkspaceLock 退役）、§2.3 描画関数は投影（U4）へ、§3 ポート表・供給面表を gateway-taxonomy 語彙へ（AuditLedgerService 退役、FileStore は実装内部、Clock / ProcessProbe は機構、Git は外部システムクライアント）、§4 | BR2.1 / BR3.2 | （設計監査 R3 / C3 / C4 / C11、ADR-003 / 004 / 007） |
| `docs/specs/12-workflow-definition.md` | §2.1（`WorkflowDefinitionId` / `DefinitionRevision`）、§2.2 PlanAction 所有明記、§2.3（`next_in_scope_stage` 行削除、StageGraphQuery 等の個別名廃止 → 集約の述語面 6 つ + `grid().action()`）、§4 / §8 F2 / §9 の `next_in_scope_stage` 言及の改訂、§5 `find` → `find_by_id`、§10 集約昇格の第一理由（lockstep 一貫性単位）、B1 注記（畳み込みの呼び出し側 = 集約） | BR2.4 / BR2.5 / BR3.1 / BR3.3 | （設計監査 C8 / C9 / C10、ADR-008、C4 改訂） |

## 3. 作法（security-design §2 の要約 — 委任ブリーフに転記する）

最小変更・行末の出典注記・逐語契約（監査イベント名 / CLI 語彙 / `AIDLC_*` / 逐語文言 / ファイル形式）と `docs/specs/research/**` には触れない・
日本語正本（固定トークンは英語）・旧記述は「旧」明記の比較表にだけ・表は見出しと同じ列数（`\|` エスケープ）・見出し重複なし・
設計に無い判断が要ったら推測せず `developer-report-<n>.md` の「設計質問」に書く。

## 4. 棚卸し（code-summary に記録する事項）

- 受入チェック 1〜6（security-design §3）の実測結果（コマンドと出力）。
- sentinel grep の残存ヒット一覧とそれぞれが履歴注記である根拠（ファイル:行）。
- README 行数とルールファイル数。
- 設計質問と裁定（あれば）。

## 5. 実装ステップ（受入検査を赤→緑に）

Testing Contract の層（Data model / Repository / Business logic / API / Frontend）は文書だけの Unit には該当しない。各委任は「Red = 受入 grep・
diff・行数検査を先に走らせて現状の赤を `developer-report-<n>.md` に記録 → Green = 改訂 → Refactor = 出典注記・表整形・再検査」で進める。

### 5.0 コンダクタ（承認後・委任前）

- [ ] Step 0. Bolt 開始（`bun .claude/tools/aidlc-bolt.ts start --name B4 --batch 1`、未開始の場合）。aidlc 記録を 1 コミット。基線: 受入チェック 1〜3 を
      `origin/main` 時点で実測して記録（赤の基線）。

### 5.1 委任 1 — coding-rules / components.md / deviations.md（開発エージェント、所有ファイル: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md`、`aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md`、`docs/specs/deviations.md`）

- [ ] Step 1. Red: `grep -n 'repository.load()' coding-rules/use-case-rules.md`、`grep -n 'load / save' coding-rules/gateway-taxonomy.md`、
      `grep -n 'AuditLedgerRepository\|WorkspaceLock' coding-rules/*.md`、README 行数（6）vs ルール数（6 → 7 予定）、deviations の # 最大値（3）を記録。
- [ ] Step 2. Green: BR1.1 / BR1.2 / BR1.3 / BR1.4 / BR1.5 / BR4.1（`error-handling.md` 新設 — 文面は `../functional-design/functional-design-questions.md`
      Q1 の改訂ドラフトそのまま、書式は既存ファイルに合わせる）/ BR4.2（README）/ BR3.4（deviations # 4 — security-design §4 の文面、予約節の該当項目を
      統合）/ BR3.5（components.md の `WorkspaceModel` と `ReadModelUpdater`）。
- [ ] Step 3. Refactor: 出典注記・表の列数・見出し重複を整え、Step 1 の検査を再実行して緑（`repository.load()` 0、`load / save` 0、
      `AuditLedgerRepository` 0、`WorkspaceLock` は退役注記の 1 行のみ、README 7 行 = 7 ファイル、deviations # 4 あり）。`developer-report-1.md`。

### 5.2 委任 2 — 仕様 01 / 10 / 11 / 12 号（開発エージェント、所有ファイル: `docs/specs/01-domain-model.md`、`docs/specs/10-orchestration.md`、`docs/specs/11-workspace.md`、`docs/specs/12-workflow-definition.md`）

- [ ] Step 4. Red: sentinel 7 語 + `WorkspaceLock` / `StageGraphQuery` / `StageNodeView` / `SensorBindingView` / `find(` を `docs/specs/*.md`（research 除外）で
      grep し件数と行を記録（`next_in_scope_stage` 5、`AuditLedgerRepository` 2、`AuditLedgerService` 1、`WorkspaceLock` 3 ほか）。
- [ ] Step 5. Green: BR2.1（11 号）/ BR2.2（01 号 §3.1〜3.3）/ BR2.3（10 号 §3 + §8）/ BR2.4（10 §2.2 / 12 §2.2 / 01 §3）/ BR2.5（12 号 5 箇所 + §10 C10）/
      BR3.1（12 §2.1・§5、10 §3、01 §3.1）/ BR3.2（01 §3.3、11 §2）/ BR3.3（10 §2.1、12 B1）/ BR3.6（01 §7）。一次出典は `origin/main` の
      `modules/core/domain/src/{orchestration,workflow_definition}/` と `modules/core/use-case/src/orchestration/workflow_definition_repository.rs`、
      ADR-001〜008、C3 / C4 / C5 / C6、U2 機能設計（鮮度注記つき）。
- [ ] Step 6. Refactor: 出典注記・表整形・見出し重複を整え、Step 4 の grep を再実行して sentinel 7 語 = 0（履歴注記以外）、`WorkspaceLock` は退役注記と
      逸脱台帳参照のみ。`developer-report-2.md`。

### 5.3 コンダクタ（統合）

- [ ] Step 7. 受入チェック 1〜6 を全体で実測（`unit-test-instructions.md` のコマンド）、`code-summary.md` / `traceability.json` を書く。
- [ ] Step 8. advisory レビュー（アーキテクチャレビュアー）→ 文書コミット → PR（本文に受入の実測を貼る）→ CodeRabbit 全件対応 → CI 緑 → merge queue →
      `aidlc-bolt.ts complete --name B4 --batch 1`。

## 6. トレーサビリティ（要求 → ステップ）

| 要求 | BR | ステップ |
|---|---|---|
| FR8.1 | BR1.1〜BR1.5, BR4.2 | 1〜3 |
| FR8.2 | BR2.1〜BR2.5, BR3.1〜BR3.3, BR3.5 | 4〜6（BR3.5 は 2） |
| FR9.6 | BR4.1, BR4.2 | 2〜3 |
| NFR1.1〜1.3 | BR5.2, BR3.4 | 2, 3, 6, 7 |
| NFR2.1〜2.5 | BR5.1 | 0, 3, 6, 7 |

## 7. 委任の形

- 委任 1・委任 2 は所有ファイルが重ならないため**並行**に走らせる（同一ワークツリー、コミットはコンダクタが行う — 開発エージェントは `git commit` しない）。
- 開発エージェントは本計画・`unit-test-instructions.md`・`code-generation-questions.md` を書き換えない。進捗・設計質問・検査結果は
  `developer-report-<n>.md` に書く（B1 の承認ガード教訓）。
- モデル: 両委任とも Opus（ADR と実装を突き合わせて仕様文を書く判断が要る）。

> 注: 本 Unit はプロダクションコードを持たないため、層ごとの Red / Green / Refactor は受入検査（grep / diff / 行数）の赤→緑として運用し、既存スイートは diff ゼロで緑のまま（§5 冒頭）。

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
