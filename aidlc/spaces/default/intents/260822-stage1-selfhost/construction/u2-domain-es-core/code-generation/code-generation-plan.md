# code-generation-plan — U2 ドメイン ES コア（FCC 化と `next_decision` の ID 照合、Bolt b51）

> Code Generation（Construction 3.5）の計画（Unit: U2 `u2-domain-es-core`、kind: library、規模 L）。**2026-09-07 再走（Modify）** —
> 2026-08-23 に承認した旧計画（Bolt B3、`WorkflowExecution` の ES 化）は実装済みで、`code-generation-plan-history-2026-08-23.md` /
> `unit-test-instructions-history-2026-08-23.md` / `code-summary-history-2026-08-23.md` / `traceability-history-2026-08-23.json` /
> `code-generation-questions-history-2026-08-23.md` に全文保存した。本計画は 2026-09-05 是正・2026-09-07 再走後の機能設計と NFR 設計が
> 現行コードに対して命じる差分（functional-spec §9 #1〜#4）を実装する。
>
> 出典: `../functional-design/functional-spec.md`（§2 API、§9 引継ぎ、末尾レビュー R-01〜R-10）、`../functional-design/rules.md`
> （BR1.1 / BR2.1〜BR2.6 / BR3.1 / BR5.1〜BR5.5）、`../functional-design/entities.md`（FCC 型の不変条件・操作）、
> `../nfr-requirements/security-requirements.md`（NFR1.1〜NFR4.5、NFR2.5、末尾レビュー R-01〜R-08）、`../nfr-design/security-design.md`
> （§2 検査点の二層、§6）、`../nfr-design/logical-components.md`（§1 置き場と追随表、§4 受入手順、末尾レビュー R-01〜R-07）、
> `../../../inception/units-generation/unit-of-work.md`（U2）、`../../../inception/contract-design/contract-summary.md`（C3 / C5 / C6）、
> `../../../inception/requirements-analysis/requirements.md`（FR8.3 / FR8.4、NFR1〜NFR4）、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（first-class-collections / module-visibility / field-visibility /
> aggregate-commands / error-handling / tell-dont-ask / command-query-separation / cqrs-boundaries / no-backward-compatibility）、
> `code-generation-questions.md`（P1〜P5、Q1〜Q2 の裁定）。
>
> 用語: **FCC** = ファーストクラスコレクション（配列を不変条件と操作を持つ専用型で包んだもの）。**DTO** = アダプタ層が保存・復元に
> 使う写し。**RMU** = read-model-updater（イベントからリードモデルを投影するクレート）。**ITF 準拠テスト** = Quint モデルのトレースを
> 集約に再生して突き合わせるテスト。

## 1. 目的と変更範囲

**作るもの**（functional-spec §9 #1〜#4 の実装）:

1. **FCC の新設（BR5.5）** — `orchestration/` に `StageEntries` / `StageSlot` / `StageSlots` / `StageIndexSet` / `ArtifactPaths` /
   `StageSlugSet` / `TransitionSteps` / `ReviewClosures` / `PendingIterations`（クレート内型）、`workspace/` に `PromotedSections` /
   `RuleLines`。各型は §2 の不変条件・操作・エラー型を持ち、共通契約 `core_infrastructure::collections::FirstClassCollection`
   （`len` / `is_empty` / `at` / `fold_left` / `filter`）を実装して既存ハーネス `tests/collection_contract_test.rs` に登録する。
   `combine`（和集合）/ `divide`（差集合）は **集合型 2 つ（`StageIndexSet` / `StageSlugSet`）だけ**に実装し Monoid 則・差集合則を
   性質試験で固定する（Q1 = A）。
2. **集約・イベント・`PracticesPromotion` の切替** — `IntentExecution` の 7 並列列（`stage_keys` / `overlay` / `checkbox` /
   `review_attempts` / `practices_affirmed` / `approved` / `revision_count`）を `slots: StageSlots` に統合し、`stage_keys()` を廃止して
   `slots()` / `stage_key(StageIndex)` を公開する。`Intent::stages()` / `Created.stages` / `Started.stages` → `&StageEntries`、
   `GateOpened.artifacts` / `open_gate` → `ArtifactPaths`、`Recomposed.skipped` / `added` → `StageSlugSet`、`recompose` → `StageIndexSet`、
   `apply_report` / `ReportDecision::Commit.steps` → `TransitionSteps`、`ReviewAttempt` の `closed` → `ReviewClosures`・`pending` →
   `PendingIterations`（Q2 = A）、`PracticesAffirmed` / `PracticesPromotion` の `sections` / `mandated` / `forbidden` →
   `PromotedSections` / `RuleLines`。`StageEntry::check_plan(&[StageEntry])` と `Intent::check_plan` は `StageEntries::new` の構築検査へ移す。
   生の `Vec` / `&[..]` の公開は DTO 境界の理由付き例外を除いて 0 件にする。
3. **`next_decision` の Result 化（BR2.6 / BR3.1、Q5 = A）** — `Result<NextDecision, CommandError>` にし、`matches(intent)` 不一致を
   `CommandError::IntentMismatch` で拒否する。
4. **冒頭 doc の是正（§9 #4、NFR 設計レビュー R-03）** — `intent_execution.rs` 冒頭の「12 の decide コマンド」→ 16、「楽観 version は
   持たない … `seq_nr` だけ」→「集約は不透明な版トークン `version` を持ち回る（`with_version` / `version()`）が採番はストアの責務
   （ADR-010）」、「`# Panics` を持つ公開 API は無い」→ `replay` / `apply_event` / 誕生変換 `From<(Started, DateTime<Utc>)>` の 3 か所、
   「memento」の旧説明を削除。`orchestration/mod.rs` 冒頭の「ジャーナル全再生」→ 最新スナップショット + 差分（BR2.3）、
   「`next_decision` はクエリ側が所有」→ `IntentExecution::next_decision`、「`recompose(&[stage])`」→ `StageIndexSet`。
5. **兄弟クレートへの追随（越境の裁定 — P8、NFR 設計レビュー R-01）** — §3 の実測一覧のとおり、`core-command-interface-adapter` /
   `core-read-model-updater` / `core-command-use-case` / `aidlc`（app）と各クレートの `tests/` を同じ Bolt で追随させ、
   ワークスペース全体を緑に保つ。リードモデル側（RMU / クエリ側）は FCC 型を**定義・保持しない**（読取専用の `fold_left` / `at` の
   呼出は可）。DTO の列表現（正準 JSON のバイト）は変えない。
6. **契約試験・性質試験・ITF の追随** — `tests/collection_contract_test.rs` へ新設型を登録、集合型の Monoid 則、列型の構築検査、
   `next_decision` の `IntentMismatch` テスト、`tests/engine_loop_conformance.rs` の改修後 API への追随（Quint モデル v2.7 は不変）。

**作らないもの**: Quint モデルの改訂、DTO の列構造・JSON バイトの変更、`combine` / `divide` / `map` の共通 trait への一律化
（オーナーの最終方針、着手時期は別途裁定 — 積み残し）、上流 `components.md` / `contract-summary.md` C3 の「ジャーナル全再生」注記の
同期（積み残し）、`workflow_definition` 文脈の改修（`WorkflowDefinition::replay` の `# Panics` は射程外）、依存クレートの追加
（NFR4.1: runtime = `chrono` / `uuid` / `core-infrastructure`、dev = `proptest` / `serde_json` から増やさない）、`scripts/**` /
`.github/**` の変更、GitHub への書込（PR 作成・コメントは親セッションが行う）。

**ブランチと PR（P3）**: 本ワークツリーのブランチ `stage1-selfhost`（`origin/main` `e8ca4a5f` から intent 記録 4 コミット先行、
未 push、上流追跡なし）で作業する。開発エージェントは意味単位でコミットし、push / PR は行わない。Bolt 完了後に親セッションが push し
PR 1 本（直列、タイトル = Bolt slug `b51: …`、squash-merge）を開き、収束ルール（必須 CI green ∧ unresolved = 0 ∧ 全コメント返信済み）で
畳む。

**コーディング規則の要点**（正本 `coding-rules/`）: フィールド既定 private + アクセサ（field-visibility）、型ファイル mod は private で公開は
ファサード `mod.rs` の `pub use` のみ（module-visibility、利便再エクスポート禁止）、FCC は要素型を所有する文脈に置く、`unwrap` / `expect`
はプロダクトコード禁止、`missing_docs` / `missing_panics_doc` deny、手実装 enum + `Display` + `std::error::Error`（error-handling、
thiserror / anyhow 不使用）、CQS（コマンド = `&mut self` で戻り値なし or `Result<(), E>`、クエリ = `&self`）、内部可変性禁止、
Tell-Don't-Ask（ユースケースは getter で組み立て直さず操作を依頼）、後方互換の旧 API は残さない（no-backward-compatibility）、
ドメインの名前はユビキタス言語（`set_*` / `data` / `helper` 等を使わない）。

## 2. 設計の確定事項（本計画で確定し、functional-design ゲートの Request Changes で本文へ折り戻す）

機能設計レビュー R-01 / R-03 / R-04 / R-07 と NFR 要求レビュー R-05 / R-06 が未決とした点を、Q1 / Q2 の裁定に従って確定する。

| 型 | 置き場（`modules/core/command/domain/src/`）| 要素 / 表現 | 不変条件（構築検査で Err） | 操作（共通契約 + 業務操作） | `Filtered` | エラー型 | 用途（実測） |
|---|---|---|---|---|---|---|---|
| `StageEntries` | `orchestration/stage_entries.rs` | `StageEntry` の列（文書順） | 非空・slug 一意・initialization は EXECUTE かつ無条件（現行 `StageEntry::check_plan` の `PlanError` 4 変種をそのまま吸収） | `new(Vec<StageEntry>) -> Result<_, PlanError>`、`at(StageIndex)`、`position_of(&StageSlug) -> Option<StageIndex>`、`first_of(PhaseId, PlanAction)`、`fold_left`、`filter` | `Collection<StageEntry>` | `PlanError`（既存） | `Intent.stages` / `Created` / `Started`、skeleton ゲート判定、RMU の行生成、app の scaffold |
| `StageSlot` | `orchestration/stage_slot.rs` | 位置 1 つの記録: `key: StageKey` / `plan_action: PlanAction`（overlay）/ `checkbox: CheckboxState` / `approved: bool` / `revision_count: u32` / `review_attempt: ReviewAttempt` / `practices_affirmed: bool` | なし（値の組） | `genesis(key, plan_action)`（Pending・未承認・0・空会計）、`new(全属性)`（DTO 境界）、アクセサ、コマンド `mark(CheckboxState)` / `record_approval` / `invalidate_approval` / `bump_revision` / `override_plan(PlanAction)` / `reset_attempt` / `record_review_request(u32)` / `record_review_verdict(u32, ReviewVerdict)` / `affirm_practices` | — | — | `StageSlots` の要素 |
| `StageSlots` | `orchestration/stage_slots.rs` | `StageSlot` の列、添字 = `StageIndex` | 非空・長さ = stage_count・slug 一意 | `new(Vec<StageSlot>) -> Result<_, StageSlotsError>`（DTO 境界）、`genesis(&StageEntries)`（誕生時の全 Pending）、`at(StageIndex) -> Option<&StageSlot>`、`stage_key(StageIndex)`、`position_of(&StageSlug)`、`fold_left`、`filter`、位置指定コマンド（上記 `StageSlot` のコマンドを `StageIndex` 付きで）、一括コマンド `mark_all(&StageIndexSet, CheckboxState)` / `invalidate_approvals(&StageIndexSet)` / `reset_attempts_all()`（jump のフロア）| `Collection<StageSlot>` | `StageSlotsError`（新設: `Empty` / `DuplicateSlug`）| `IntentExecution.slots`、DTO の 7 列との相互変換（`fold_left` で展開、`new` で畳む）、ITF の射影 |
| `StageIndexSet` | `orchestration/stage_index_set.rs` | `BTreeSet<StageIndex>`（昇順） | なし（空を許す） | `empty()`、`singleton`、`range(from, to)`、`contains`、`at`（昇順の添字）、`fold_left`、`filter`、**`combine`（和集合）/ `divide`（差集合）** — 空集合を単位元とする Monoid（結合・左右単位元・冪等・交換）と `A \ A = ∅` / `A \ ∅ = A` を性質試験 | `Self` | なし（全域） | `recompose` の入力（複数位置）、jump の読み飛ばし・巻き戻し・承認無効化の対象集合（現行 `Vec<StageIndex>` と range ループを置換） |
| `ArtifactPaths` | `orchestration/artifact_paths.rs` | `String` の列（素通し、順序・重複を保持） | なし | `empty()`、`new(Vec<String>)`、`at`、`fold_left`、`filter` | `Self` | なし | `open_gate` の入力、`GateOpened.artifacts`、DTO |
| `StageSlugSet` | `orchestration/stage_slug_set.rs` | `BTreeSet<StageSlug>`（辞書順） | なし（空を許す） | `empty()`、`new(impl IntoIterator<Item = StageSlug>)`、`contains`、`at`（辞書順）、`fold_left`、`filter`、**`combine` / `divide`**（Monoid 則・差集合則を性質試験） | `Self` | なし | `Recomposed.skipped` / `added`（`StageEntries::slugs_at(&StageIndexSet)` で位置集合から写す）、DTO |
| `TransitionSteps` | `orchestration/transition_steps.rs` | `TransitionStep` の列（`report_dispatch` が決めた遷移順） | 重複なし | `new(Vec<TransitionStep>) -> Result<_, TransitionStepsError>`、`single(step)`、`contains(TransitionStep)`、`at`、`fold_left`、`filter`。`apply_report` の段分岐は名前付きクエリ（例 `is_single(step)` / `is_pair(a, b)`）か、理由を doc に書いた `pub(crate)` のスライス公開のどちらかで書く（開発者判断、`code-summary.md` に理由を記す） | `Self` | `TransitionStepsError`（`Duplicate`）| `ReportDecision::Commit.steps`、`apply_report` の入力、use-case の `contains(Approve)` |
| `ReviewClosures` | `orchestration/review_closures.rs` | `ReviewClosure` の列（記録順） | なし | `empty()`、`new(Vec<ReviewClosure>)`（DTO 境界）、`record(ReviewClosure)`（コマンド）、`at`、`fold_left`、`filter`、`has_terminal(&ReviewPolicy)` | `Self` | なし | `ReviewAttempt.closed`、DTO（`intent_execution_dto.rs:98`）|
| `PendingIterations` | `orchestration/pending_iterations.rs`（`pub(crate)`、ファサード非公開） | `BTreeSet<u32>` | なし | `empty()`、`with(u32)` / `without(u32)`（コマンド）、`contains`、`at`、`fold_left`、`filter` | `Self` | なし | `ReviewAttempt.pending`（外部に出ない）|
| `PromotedSections` | `workspace/promoted_sections.rs` | `PromotedSection` の列（順序保持） | 見出し一意 | `new(Vec<PromotedSection>) -> Result<_, PromotedSectionsError>`、`at`、`fold_left`、`filter`、`headings()` は `fold_left` で書く | `Self` | `PromotedSectionsError`（`DuplicateHeading`）| `PracticesPromotion.sections`、`PracticesAffirmed.sections`、RMU の投影、DTO |
| `RuleLines` | `workspace/rule_lines.rs` | `String` の列（素通し、順序・重複を保持） | なし | `empty()`、`new(Vec<String>)`、`at`、`fold_left`、`filter` | `Self` | なし | `PracticesPromotion` / `PracticesAffirmed` の `mandated` / `forbidden`、RMU の投影、DTO |

補足の確定事項:

- `IntentExecution::new` の引数は `(id, intent_id, slots: StageSlots, cursor: usize, status, parked_at, autonomy, skeleton_stance,
  last_gate_resolution_at, seq_nr, last_updated_at)`。DTO（`IntentExecutionDto` の 7 列）は列ごとに `StageSlot::new` を組み、
  `StageSlots::new` を通す。列の長さ不一致は DTO 側の `DtoDecodeError::InvariantViolation`（→ `RepositoryError::Corrupt`、C3）で、
  現行と同じ失敗境界（層 (1)）に留まる。
- `Recomposed` の投影順序: 現行は位置昇順（文書順）で `skipped` / `added` を描く。`StageSlugSet` は辞書順なので、RMU の投影
  （`projection.rs:1097-1098` の `stage_list`）は `plan.stages()` の位置で並べ直してから描く。ゴールデン（`projection_golden_test.rs`）と
  監査行の逐語一致は U4 / U7 の NFR1 要求であり、赤になったら文書順へ写す側を直し、`StageSlugSet` の順序を変えない。
- 誕生変換 `From<(Started, DateTime<Utc>)>` は現行どおり panic（層 (2)）。`IntentExecution::new` の Err（層 (1)）との振り分けは
  「DTO からの復元は `new`、イベントからの再生は誕生変換」— doc に 1 行書く（NFR 設計レビュー R-02）。
- `version` は集約が持ち回る不透明トークン（`with_version` / `version()`）で、採番と比較はストアの責務（ADR-010）。
  `unit-of-work.md` U2 の「version は失効」は「集約が採番しない」の意味であり、フィールドの不在ではない — doc に 1 行書く。
- `next_decision` の Err を受ける RMU（`read_tables/next_answer_row.rs:58`）は既存のエラー型に写す（新変種が要れば追加）。RMU は
  intent と execution を対で持つため実運用では起きず、テストで Err 経路を 1 本固定する。

## 3. 追随対象（実測 — 2026-09-07、`rg` による全ワークスペース走査）

| クレート | 生産コード | テスト |
|---|---|---|
| `core-command-domain` | `orchestration/{intent, intent_execution, intent_execution_event, stage_entry, review_attempt, report_decision}.rs`、`intent_event/created.rs`、`intent_execution_event/{started, gate_opened, recomposed, practices_affirmed}.rs`、`workspace/practices_promotion.rs`、`orchestration/mod.rs` / `workspace/mod.rs`（`pub use`）| インライン `#[cfg(test)]`（`intent_execution.rs` 約 60 箇所の `stages()` / `stage_keys()` / `closed()` / `skipped()`）、`tests/engine_loop_conformance.rs:356,449,488`、`tests/collection_contract_test.rs`（登録）|
| `core-command-interface-adapter` | `src/orchestration/dto/{intent_dto.rs:85, created_dto.rs:47,68, intent_execution_dto.rs:98,142, intent_execution_event_dto.rs:113,121,175,176,227,231,232,254}` | `src/orchestration/dto/tests.rs:534,595`、`tests/{commit_verdict_use_case_wiring_test.rs:81,102, intent_execution_repository_impl_test.rs, upstream_event_store_conformance.rs, support/contract.rs, support/mod.rs}`（`open_gate(.., vec![..], ..)` 12 箇所）|
| `core-read-model-updater` | `src/read_tables.rs:239,284`、`src/read_tables/{stage_lookup.rs:23, next_answer_row.rs:58}`、`src/workspace/resolved_plan.rs:49`、`src/workspace/projection.rs:466,477,542,581,853,875,908,1059,1080,1083,1087,1097,1098,1146,1157,1403,1416,1420,1432,1443,1444,1730`、`src/orchestration/dto/{intent_dto.rs:93,116, started_dto.rs:31,49, gate_opened_dto.rs:26, recomposed_dto.rs:25,26, practices_affirmed_dto.rs:42,49,50}` | `src/workspace/projection.rs:2022`、`src/workspace/resolved_plan.rs:246`、`tests/{projection_golden_test.rs:175,576, read_model_updater_test.rs:144,985, read_tables_test.rs（`stages()` / `stage_keys()` / `next_decision` / `open_gate` 15 箇所）, support/mod.rs:280, journal_reader_impl_test.rs:1378,1416}` |
| `core-command-use-case` | `src/orchestration/commit_verdict_use_case.rs:196-218`（`report_dispatch` → `steps.contains` → `apply_report`）| `src/orchestration/commit_verdict_use_case.rs:496,533,694,778`、`src/orchestration/promote_practices_use_case.rs:190,194`、`src/orchestration/test_support.rs:114,856,889` |
| `aidlc`（app、越境 4 つめのクレート）| `src/scaffold.rs:46,161,182`（`intent.stages().iter().filter(..)`）| `tests/{journal_protocol_conformance.rs:306,565,815, crash_reconstruction_test.rs:71, support/mod.rs:208}` |
| `core-query-interface-adapter` | なし | `tests/support/mod.rs:238`（`open_gate`）|

RMU の `projection.rs` の `plan.stages()` は多くが `ResolvedPlan`（RMU 自前の平坦な計画表現、`resolved_plan.rs`）の呼出で、
ドメインの `Intent::stages` とは別物である。着手時（Step 1）に `rg` で再走査し、ドメイン型の呼出だけを追随対象として確定する。

## 4. 実行ステップ（Testing Contract の TDD 順序に沿う）

チェックボックスは親セッション（コンダクタ）が検証後に付ける。開発エージェントは計画ファイルを編集せず、進捗と各 Red の失敗出力を
`developer-report-<n>.md` に書く（P2）。

- [ ] **Step 0. 基線とランナーの確認（委任 1 の冒頭）** — `git status` がクリーンで `origin/main..HEAD` が記録コミットのみであることを
  確認。`unit-test-instructions.md` §2 の Unit 限定コマンドがそのまま走ることを実測し、テスト件数（`core-command-domain --lib` / ITF /
  契約試験 / 兄弟クレートの対象テスト）と `PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only`
  の行カバレッジ（基準値 98.66%、`--ignore-filename-regex 'modules/core/command/domain/src/(workflow_definition|workspace)/'` の
  orchestration 単独値も）を記録する。`# Panics` の所在（`rg -n '# Panics' modules/core/command/domain/src`）と生の `Vec` / `&[..]` 公開
  （`rg -n 'pub fn .*-> &\[' modules/core/command/domain/src/orchestration modules/core/command/domain/src/workspace`）を実測して報告に残す。
- [ ] **Step 1. データモデル層 — FCC 11 型の新設（委任 1、Opus、追加のみで既存 API は触らない）** — 型ごとに Red（`#[cfg(test)]` の
  失敗テスト: 構築検査の Err、`at` の範囲外 `None`、`fold_left` の順序、`filter` の結果型、集合型は Monoid 則・差集合則の proptest、
  `TransitionSteps` の `Duplicate`、`PromotedSections` の `DuplicateHeading`）→ Green（最小実装）→ Refactor（緑のまま整理）。
  `FirstClassCollection` を実装し `tests/collection_contract_test.rs` の `check(..)` に **空と非空の 2 例ずつ**登録する（非空型は非空例のみ）。
  ファサード `orchestration/mod.rs` / `workspace/mod.rs` へ `pub use` を追加（`PendingIterations` は `pub(crate)` で非公開）。
  受入: `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain` 全緑、`cargo fmt --all --check` / `cargo clippy --workspace
  --all-targets -- -D warnings` / `cargo lint` 緑、`git diff --stat` が `modules/core/command/domain/` に閉じている。コミット 1 つ
  （`feat(domain): FCC 11 型を新設し契約試験へ登録`）。
- [ ] **Step 2. ビジネスロジック層 — 集約・イベント・`PracticesPromotion` の切替と `next_decision` の Result 化（委任 2、Opus）** —
  Red: (a) `next_decision` が `intent_id` 不一致で `Err(CommandError::IntentMismatch)`、一致で `Ok` を返す新規テスト（コンパイル失敗を
  Red として記録）、(b) `Intent::stages()` / `slots()` / `stage_key()` / `open_gate(ArtifactPaths)` / `recompose(StageIndexSet)` /
  `apply_report(&TransitionSteps)` / `ReviewAttempt::closed() -> &ReviewClosures` / `PracticesPromotion::sections() -> &PromotedSections`
  を使う既存テストの書換え（コンパイル失敗の出力を記録）。Green: 7 並列列 → `slots`、`stage_keys()` 廃止、`StageEntry::check_plan` /
  `Intent::check_plan` → `StageEntries::new`、jump の読み飛ばし・巻き戻し・承認無効化を `StageIndexSet` + `StageSlots` の一括コマンドで
  書き直す、`Recomposed` を `StageEntries::slugs_at(&StageIndexSet)` で組む、`ReviewAttempt` の内部列を FCC へ、
  `PracticesPromotion` / `PracticesAffirmed` の列を FCC へ。Refactor: 冒頭 doc の是正（§1 #4）、`resolve` / `mark_stage` /
  `invalidate_approval` 等の内部ヘルパの整理。ITF 準拠テスト（`engine_loop_conformance.rs`）を改修後 API へ追随（モデル不変、
  8 fixture 全緑）。受入: `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain` 全緑（PBT 既存性質 + ITF + 契約試験）、
  `rg -n 'pub fn .*-> &\[' modules/core/command/domain/src/orchestration modules/core/command/domain/src/workspace` が 0 件、
  `rg -n 'stage_keys\(' modules/core/command/domain/src` が 0 件。この時点でワークスペースは赤（Step 3 で回復）。
- [ ] **Step 3. データアクセス層 — DTO 境界と兄弟クレートの追随（委任 2、続き）** — Red: `core-command-interface-adapter` /
  `core-read-model-updater` / `core-command-use-case` / `aidlc` の既存テスト（往復・ゴールデン・配線・クラッシュ再構成）がコンパイル
  失敗する出力を記録。Green: §3 の追随（DTO の要素列挙は `fold_left`、7 列 ↔ `StageSlots` の相互変換、`ResolvedPlan::of` /
  `read_tables` / `stage_lookup` の列挙を `fold_left` / `at` へ、`next_answer_row.rs` の Err 処理、`commit_verdict_use_case.rs` の
  `contains` を `TransitionSteps` の操作へ、`scaffold.rs` の `filter` を `StageEntries::filter` / `fold_left` へ、`Recomposed` の投影順序を
  文書順へ写す）。Refactor: 重複した列挙ヘルパの整理。受入: `PROPTEST_RNG_SEED=20260823 cargo test --workspace` 全緑、
  `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` 緑、`bash scripts/quint-gate.sh` 緑、
  `rg -n 'StageEntries|StageSlots|StageIndexSet|StageSlugSet|ArtifactPaths|TransitionSteps|ReviewClosures|PromotedSections|RuleLines'
  modules/core/read-model-updater/src modules/core/query` が**型の定義・保持**（struct フィールド・`let` 束縛の保持）を含まないことを
  目視確認し報告に書く（読取専用の呼出は可）。コミットは意味単位（例: ドメイン切替 + ITF / DTO 境界 / RMU / use-case + app）で、各コミットで
  `cargo check --workspace` が通らない場合は 1 コミットにまとめ、その旨を報告に書く。
- [ ] **Step 4. 受入の実測（委任 2 の末尾で実施、コンダクタが再実測）** — (a) CI 4 ステップ（fmt / clippy / `cargo lint` /
  `cargo test --workspace`）+ `scripts/quint-gate.sh` + `cargo audit`（ワークスペースと `tools/lint/Cargo.lock`）緑、(b)
  `bash scripts/coverage.sh` を同一条件で 2 回実行し差 0.00 と絶対床 90% の PASS を記録、(c) `PROPTEST_RNG_SEED=20260823 cargo llvm-cov
  --package core-command-domain --summary-only` の行カバレッジが基準値 98.66% を下回らない（orchestration 単独値も記録）、(d) BR4.1 の
  判定式（`unit-test-instructions.md` §2）が 0 件で、検出力の裏取り（`workflow_definition` へ流すと 1 件以上）を記録、(e) `# Panics` の
  所在が `intent_execution.rs` の 3 か所（`replay` / `apply_event` / 誕生変換）と `workflow_definition.rs:213` のままで増えていない、
  (f) `modules/core/command/domain/Cargo.toml` と `Cargo.lock` が不変（`git diff --stat -- Cargo.lock modules/core/command/domain/Cargo.toml`
  が空）、(g) 生の `Vec` / `&[..]` 公開 0 件（DTO 境界の `pub(super)` / `pub(crate)` は理由付きで許容し一覧を報告）。
- [ ] **Step 5. 記録（コンダクタ）** — `code-summary.md`（作成・変更ファイル、設計判断、Step 4 の実測、計画からの逸脱、§2 の確定事項を
  functional-design ゲートへ折り戻す一覧）、`source-manifest.json`（strict schema、`writes` に作成・変更・削除した全アプリケーション側パス）、
  `traceability.json`（BR1.1〜BR5.5 / NFR1.1〜NFR4.5 の各 ID → 実在の実装・テストファイル 1 つ）、センサー（required-sections /
  traceability）実行、独立レビュー（advisory、1 回）、Unit 完了、`git add -A` で作業ツリー全体を回収してコミット、push、PR 作成。

Testing Contract の層のうち API / エンドポイント層とフロントエンド層は本 Unit（library）に存在しないため省く。環境 / ビルド設定の
変更はない（依存追加なし）。ドキュメント層は Step 2 の doc 是正と Step 5 の記録で満たす。

## 5. 要求からステップへの対応

| 要求 / 規則 | Step | 確認対象 |
|---|---|---|
| BR5.5、Q4 / Q4a / Q1 / Q2、NFR2.5 | 1・2・3 | FCC 11 型、契約試験の登録、Monoid 則・差集合則の性質試験、生の `Vec` / `&[..]` 公開 0 件、RMU が FCC 型を定義・保持しない |
| BR2.6 / BR3.1、Q5、NFR3.4 | 2・3 | `next_decision` の Result 化と `IntentMismatch` テスト、`next_answer_row.rs` の Err 処理 |
| BR1.1 / BR2.1 / BR2.3、NFR2.2 / NFR3.1 | 2 | PBT 既存性質が改修後も緑、`apply_event` の純関数性、時計利用は `*EventId::generate` のみ |
| BR2.5、NFR1.1 | 2 | ITF 準拠テストの追随（8 fixture、`EngineSignal` 照合）、Quint モデル v2.7 不変 |
| BR1.3、NFR1.2 | 2 | 誕生が initialization 全段を Completed、実グラフ索引のテストが緑 |
| BR2.4、NFR1.3 / NFR3.3 | 3 | 16 変種の網羅 match、DTO の列表現不変（往復・ゴールデン緑） |
| BR5.1 / BR5.2、NFR3.2 / NFR4.3 / NFR4.5 | 2・4 | `StageIndex` の型保証、`new` の Err と誕生変換の panic の振り分け doc、`# Panics` 3 か所、`unwrap` / `expect` 0 件 |
| BR4.1、FR8.3、NFR2.4 | 4 | 判定式 0 件 + 検出力の裏取り、CI 4 ステップ緑 |
| NFR2.1 | 1〜3 | 各 Red の失敗出力を報告に記録、テスト先行のコミット順 |
| NFR2.3 | 0・4 | クレート全体 98.66% 床、orchestration 単独値、`scripts/coverage.sh` 2 回同値 |
| NFR4.1 / NFR4.2 | 4 | `Cargo.toml` / `Cargo.lock` 不変、`cargo audit` 緑、`unsafe_code = "forbid"` |
| NFR4.4 | 1 | `ArtifactPaths` / `RuleLines` が素通し（順序・重複保持、加工なし） |
| §9 #4、NFR 設計レビュー R-02 / R-03 | 2 | 冒頭 doc の是正 3 点 + memento、誕生変換の doc、`version` の doc |

## 6. 委任と作業の進め方

| 委任 | 担当モデル | 範囲 | 所有ファイル（書込） | 受入 |
|---|---|---|---|---|
| 委任 1（`developer-brief-3.md` → `developer-report-3.md`） | Opus | Step 0〜1 | `modules/core/command/domain/src/orchestration/{stage_entries, stage_slot, stage_slots, stage_slots_error, stage_index_set, artifact_paths, stage_slug_set, transition_steps, transition_steps_error, review_closures, pending_iterations}.rs`、`modules/core/command/domain/src/workspace/{promoted_sections, promoted_sections_error, rule_lines}.rs`、両 `mod.rs` の `pub use` 追加、`modules/core/command/domain/tests/collection_contract_test.rs` | Step 1 の受入 |
| 委任 2（`developer-brief-4.md` → `developer-report-4.md`） | Opus | Step 2〜4 | 上記以外の `modules/core/command/domain/**`、`modules/core/command/interface-adapter/**`、`modules/core/read-model-updater/**`、`modules/core/command/use-case/**`、`modules/app/aidlc/**`、`modules/core/query/interface-adapter/tests/**`（テストのみ） | Step 2〜4 の受入 |
| コンダクタ | Fable 5 | Step 5、各委任の diff 全件レビュー、受入の再実測、レビュー派遣、Unit 完了、コミット・push・PR | `aidlc/**` の記録 | センサー緑、独立レビューの受領 |

委任 2 の作業が委任 1 の型定義の変更を要する場合（操作の不足・結果型の誤り）は、委任 2 が同じ Bolt 内で変更してよい（所有は時系列で
引き継ぐ）。変更点は報告に列挙し、コンダクタが §2 の確定事項へ反映する。開発エージェントは push / PR / GitHub への書込を行わず、
他者の変更を戻さず、`scripts/**` / `.github/**` / `formal/**` / `aidlc/**` を触らない。計画にない設計判断が要る場合は、実測ありの問題と
案を報告に書いてコンダクタの裁定を待つ（ドメインサービスの新設・ドメインオブジェクト 4 種以外の追加は人間の裁定が必須）。

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
**Date:** 2026-09-06T19:00:09Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action |
|---|---|---|---|---|
| R-01 | Minor | `code-summary.md` §7 申し送り 4・6、`functional-spec.md` §9 #5(a)、`nfr-design/security-design.md` / `nfr-design/logical-components.md` の `## Review`（2026-09-06、Verdict: NOT-READY） | 本計画 §2 は FCC 11 型の不変条件・操作・`Filtered`・エラー型を確定し、functional-design の NOT-READY 所見（R-01: TransitionSteps / ReviewAttempt の pending・closed / PromotedSections / RuleLines の型定義欠落、R-02: `core-command-use-case` と `engine_loop_conformance.rs` の追随漏れ、R-03: `StageSlugSet` の「文書順」不変条件が型の表現力を超える）を裁定 Q1/Q2 で実質的に解消し、実装（`stage_slug_set.rs` の doc、`projection.rs` の `in_document_order`、§3 追随表への `core-command-use-case` と `aidlc` app 追加）で裏付けている。ただし `entities.md` / `rules.md` / `functional-spec.md`（functional-design ゲート）と `security-design.md` / `logical-components.md`（nfr-design ゲート）本文はまだ更新されておらず、双方の `## Review` は 2026-09-06 時点の NOT-READY のまま残っている。fold-back は code-summary.md 自身が §7 で申し送り済みだが、この Bolt の完了時点で intent 記録内に 2 つの未解消 NOT-READY ゲートが残ることは、承認者が明示的に把握すべき事実である | functional-design と nfr-design の fold-back Bolt を近い将来に実施し、両ゲートの `## Review` を現状（このコード生成で確定した設計）に基づいて再判定する。それまでは `code-summary.md` §7 の申し送りが唯一の参照点であることを承認記録に明記する |
| R-02 | Info | `code-summary.md` §8「コンダクタの diff レビュー」の留意点 2 件（`IntentExecutionDto::to_domain` の列長検査重複、`scaffold.rs::first_post_initialization` の `Option<StageEntry>` clone） | コンダクタ自身が機能・契約に影響しないと判定済みの軽微な非効率。独立検証でも同じ結論（`filter` が所有コレクションを返す設計上の制約に起因し、ホットパスではない） | 対応不要。次回この付近を触る Bolt で ついでに整理する程度でよい |

### Validation Tool Results

| ツール / コマンド | 結果 | 解釈 |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（plan / unit-test-instructions / code-summary） | 3 件とも `pass: true`、findings 0 | H2 構成は計画どおり（7 / 6 / 8 見出し） |
| `bun .claude/tools/aidlc-sensor-traceability.ts`（traceability.json） | `pass: false`だが `gaps` / `orphans` / `missing_from_table` / `invalid_entries` / `invalid_targets` すべて 0、`missing_from_upstream_ids` 37 件 | `code-summary.md` §6 の申告と完全一致。37 件は他 Unit 所管 ID の既知ノイズで、U2 の対応関係自体に欠落・孤児・不正 target は無い |
| `git diff --name-only origin/main..HEAD -- modules` vs `source-manifest.json` | ソート正規化後にバイト一致（78 パス） | 記録の整合は完全 |
| `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain` | 699 passed（lib）+ 2（契約）+ 1（ITF）+ 3（doc-test）、失敗 0 | `code-summary.md` の 591→699 と一致 |
| `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` | いずれも exit 0 | 受入 (a) と一致 |
| `bash scripts/quint-gate.sh` | `[PASS] quint gate: all steps green`（typecheck 3・invariants 3・witness 16・quint test 2） | 受入 (a) と一致。モデル不変 |
| `git diff --stat -- Cargo.lock Cargo.toml modules/core/command/domain/Cargo.toml` / `git diff --stat origin/main..HEAD -- tests formal scripts .github` | いずれも空 | 依存不変・fixture/モデル/スクリプト不変（受入 (f) と一致） |
| `rg` による実測（`# Panics`、`stage_keys`/`check_plan`/`#[deprecated]`、`pub fn .*-> &\[`、`to_vec()`） | `# Panics` は `intent_execution.rs:347,1506,2364` + `workflow_definition.rs:213` の 4 か所のみ。他は 0 件 | 受入 (e)(g) および no-backward-compatibility 適合を実測で確認 |
| `rg` による RMU/query の FCC 型保持検査（フィールド・戻り値型） | 0 件（テストフィクスチャの `-> StageEntries` 2 件のみ） | cqrs-boundaries 適合（RMU が FCC を定義・保持しない）を実測で確認 |
| `tests/collection_contract_test.rs` の `check(..)` 登録 | 新設 9 型（StageEntries/StageSlots/StageIndexSet/StageSlugSet/ArtifactPaths/TransitionSteps/ReviewClosures/PromotedSections/RuleLines）を含む全登録を確認 | NFR2.5・受入と一致。`PendingIterations` はインライン契約検査（開発者報告の逸脱1）で妥当に代替 |
| `stage_slug_set.rs` / `projection.rs::in_document_order` の実装確認 | `StageSlugSet` は `BTreeSet`（辞書順）、投影側が `plan.stages()` の順で並べ直す | functional-spec レビュー R-03 の懸念を「不変条件から文書順を外す」方向で実装により解消 |

### Summary

計画・設計判断・実装・記録のすべてで整合が取れており、承認済み計画からの逸脱 11 件はいずれもコンパイラ・lint・型安全性上の必然か、契約に影響しない範囲に収まっている。`next_decision` の `IntentMismatch` 化、FCC 11 型の新設と契約試験登録、`StageSlugSet` の辞書順と投影側の文書順並べ直し、兄弟クレート（`core-command-use-case` と `modules/app/aidlc` を含む）への追随、`# Panics` 3 箇所への収束など、直近の functional-design（NOT-READY）と nfr-design（NOT-READY）のレビュー所見のうち実装に関わる部分は、質問票 Q1/Q2 の裁定を経てこの Bolt で具体的に解消されている。テスト・カバレッジ・Quint ゲート・記録（`source-manifest.json` / `traceability.json`）の実測はすべて申告と一致した。唯一の留意点は、functional-design と nfr-design 双方の設計文書自体（entities.md / rules.md / functional-spec.md、security-design.md / logical-components.md）がまだ fold-back されておらず、それぞれの `## Review` が 2026-09-06 時点の NOT-READY のまま残っていること（R-01、Minor）。これは code-summary.md 自身が申し送り済みで、今回のコード生成の妥当性を損なうものではないため、承認をブロックしない。
