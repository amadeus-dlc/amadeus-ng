# developer-brief-4 — 委任 2: 集約・イベント・境界の一斉切替と受入（U2、Bolt b51、Step 2〜4）

AIDLC-UNIT: u2-domain-es-core
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

Conversation language: 日本語（コード識別子・固定トークン・コミットメッセージの prefix は英語のまま。doc コメント・報告は日本語）

## A. 依頼の条件

- **役割**: `aidlc-developer-agent`（実装担当）。Unit `u2-domain-es-core`、ステージ code-generation、承認済み計画
  `code-generation-plan.md`（指紋 `sha256:dd1170c1a75b16e30a351f34d9f4ff57164bcbe65482361e94e6909de7f0634d`）の **Step 2〜Step 4** を実施する。
  委任 1（`developer-report-3.md`、コミット `3edf320d`）が FCC 11 型を新設済み。本委任はそれらを集約・イベント・境界へ**一斉に切り替え**、
  ワークスペース全体を緑に戻し、受入を実測する。
- **所有ファイル（書込可）**: `modules/core/command/domain/**`（委任 1 の新設型も、操作の不足や結果型の誤りがあれば変更してよい —
  変更点は報告に列挙）、`modules/core/command/interface-adapter/**`、`modules/core/read-model-updater/**`、`modules/core/command/use-case/**`、
  `modules/app/aidlc/**`、`modules/core/query/interface-adapter/tests/**`（テストのみ）。報告
  `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/code-generation/developer-report-4.md`。
  **触らない**: `Cargo.toml` / `Cargo.lock`（依存追加なし — NFR4.1）、`scripts/**`、`.github/**`、`formal/**`（Quint モデル v2.7 不変）、
  `tests/golden/**`・`tests/conformance/fixtures/**`（ゴールデン・ITF fixture 不変）、計画ファイル、`aidlc/**` の記録（報告ファイル以外）。
- **TDD（Testing Contract、tdd / standard）**: 層ごとに Red → Green → Refactor。型の切替では「既存テストがコンパイル失敗する」ことが Red で
  あり、層ごとに Unit 限定コマンドを走らせた**実際の出力の先頭数行**を報告に記録する。新規の振る舞い（`next_decision` の `IntentMismatch`、
  `next_answer_row` の Err 経路、`Recomposed` の投影順序）は失敗するテストを先に書く。Refactor は緑のまま行う。
- **DTO のバイト表現は変えない（P5）**: `IntentExecutionDto` の 7 列（`stages` / `overlay` / `checkbox` / `review_attempts` /
  `practices_affirmed` / `approved` / `revision_count`）、各イベント DTO の JSON 形はそのまま。ゴールデン（`projection_golden_test.rs`、
  `tests/golden/**`）と往復テストが緑であることを DTO 不変の証跡にする。ゴールデンが赤になったら**投影側の順序や写像を直し、
  FCC 型の意味（辞書順など）は変えない**。
- **リードモデル側の制約**: `core-read-model-updater` / `core-query-*` の struct フィールド・戻り値型・独自の型定義に FCC 型
  （`StageEntries` / `StageSlots` / `StageIndexSet` / `StageSlugSet` / `ArtifactPaths` / `TransitionSteps` / `ReviewClosures` /
  `PromotedSections` / `RuleLines`）を置かない。ドメインから受け取った FCC を**呼出直後に** `fold_left` / `at` / `position_of` /
  `contains` で読んで自前の平坦な表現（`ResolvedPlan`、行、文字列）へ写すのは可。
- **後方互換の旧 API は残さない**（no-backward-compatibility）: `stage_keys()`、`&[..]` を返す旧アクセサ、`StageEntry::check_plan` /
  `Intent::check_plan`、`Vec` / `&[..]` を受ける旧署名は削除し、エイリアス・`#[deprecated]`・`pub use .. as` を置かない。

### A.1 Step 2 — ビジネスロジック層（`core-command-domain`）

1. **`IntentExecution`**: 7 並列列（`stage_keys` / `overlay` / `checkbox` / `review_attempts` / `practices_affirmed` / `approved` /
   `revision_count`）を `slots: StageSlots` に統合。`new(id, intent_id, slots: StageSlots, cursor: usize, status, parked_at, autonomy,
   skeleton_stance, last_gate_resolution_at, seq_nr, last_updated_at) -> Result<_, IntentExecutionError>` にし、残る検査（cursor 範囲・
   parked_at = cursor・seq_nr ≥ 1・状態不変条件）はそのまま。列長一致と slug 重複の検査は `StageSlots::new` が構造的に担う。
   `stage_keys()` を削除し `slots(&self) -> &StageSlots` と `stage_key(&self, StageIndex) -> Option<&StageKey>` を公開。位置ごとのクエリ
   （`checkbox` / `approved` / `revision_count` / `review_attempt` / `practices_affirmed` / `effective_plan`）は `slots.at(stage)` へ委譲。
   `mark_stage` / `record_approval` / `invalidate_approval` / `reset_attempt` 等の内部ヘルパは `StageSlots` の位置指定コマンドへ委譲する。
   `StageSlotsError::OutOfRange` は `apply_event` の適用経路では `resolve` 済みの位置なので起きない — 起きたら壊れた歴史として既存の
   `ApplyError`（`pub(crate)`）経由の panic 経路に流す（`?` で `ApplyError::UnknownStage` 相当へ写す。無言 no-op にしない）。
2. **誕生変換 `From<(Started, DateTime<Utc>)>`**: `StageSlots::genesis(started.stages())` で全 Pending の列を作り、既存どおり initialization
   全段を Completed、最初の実効対象を InProgress にする（b34）。panic 契約は現行どおり（`# Panics`）。doc に「DTO からの復元は `new`（Err）、
   イベントからの再生は誕生変換（panic）」の 1 行を書く（計画 §2 補足、NFR 設計レビュー R-02）。
3. **`Intent`**: フィールド `stages: StageEntries`、`stages() -> &StageEntries`。`Intent::create` は `StageEntries::new(..)?`（`IntentError:
   From<PlanError>` は既存）。`Intent::check_plan` と `StageEntry::check_plan` は削除し、検査本体は `StageEntries::new` へ移す
   （委任 1 は `check_plan` を呼んで再利用しているので、本委任で本体を `stage_entries.rs` へ移して `stage_entry.rs` から消す。
   `stage_entry.rs` のテストは `StageEntries::new` のテストへ移す）。`Created.stages` / `Started.stages` → `StageEntries`（`stages() ->
   &StageEntries`）。`Intent::replay` の `check_plan` 呼出（`intent.rs:299`）は `StageEntries` が構築済みなので不要になる — 誕生の記録が
   `StageEntries` を運ぶ限り検査は構築時に済んでいる。
4. **`open_gate(&Intent, ArtifactPaths, t)`**、`GateOpened` の `artifacts: ArtifactPaths` / `artifacts() -> &ArtifactPaths`。
5. **`recompose(&Intent, StageIndexSet, t)`**: 空集合 → `InvalidTarget(cursor)`、各位置の妥当性は `fold_left` / `filter` で検査（部分適用
   しない）。`Recomposed` の `skipped` / `added` は `intent.stages().slugs_at(&positions)` で `StageSlugSet` に写す（`skipped()` /
   `added() -> &StageSlugSet`）。適用の腕は `fold_left` + `position_of` + `override_plan`。
6. **jump**: `stages_skipped_by_forward_jump` / `stages_reset_by_backward_jump` は `StageIndexSet::range(..).filter(..)` を返し、適用は
   `slots.mark_all(&set, ..)`。backward の承認無効化は `slots.invalidate_approvals(&StageIndexSet::range(target, stage_count))`。
   Jumped 適用のフロアは `slots.reset_attempts_all()`。
7. **`apply_report(&Intent, &ReportRequest, &TransitionSteps, ..)`**、`ReportDecision::Commit { steps: TransitionSteps, .. }`、
   `report_dispatch` は `TransitionSteps::single` / `TransitionSteps::new(vec![..])` で組む（重複は構築時に拒否 — `Duplicate` は起き得ないが
   `unwrap` 禁止なので `ReportRefusal` か `ReportCommitError` へ写す）。段分岐は `is_single` / `is_pair`（スライス match をやめる）。
8. **`next_decision(&Intent, &NextRequest) -> Result<NextDecision, CommandError>`**: 先頭で `matches(intent)` を検査し `IntentMismatch`。
   Red: 不一致で Err・一致で Ok の新規テスト 2 本以上。`state_binding` 等、内部で `next_decision` を呼ぶ箇所があれば追随。
9. **`ReviewAttempt`**: `pending: PendingIterations`、`closed: ReviewClosures`、`closed() -> &ReviewClosures`、`has_terminal` は
   `self.closed.has_terminal(policy)` へ委譲 1 行（重複解消）。`record_request` / `record_verdict` / `reset` / `is_pending` は
   `with` / `without` / `contains` で書く。`pending_iterations.rs` の `#[cfg_attr(not(test), expect(dead_code, ..))]` を削除。
   DTO 境界からの再構成用に `ReviewAttempt::new(requests, pending: PendingIterations?, closed: ReviewClosures)` が要るなら、
   `pending` はクレート外から組めないので **`Vec<u32>` を受けて内部で `PendingIterations` に畳む理由付きの境界コンストラクタ**にする
   （DTO 境界の例外として doc に理由を書く）。
10. **`PracticesPromotion`**: `sections: PromotedSections`、`mandated: RuleLines`、`forbidden: RuleLines`（アクセサは `&PromotedSections` /
    `&RuleLines`）。`plan(..)` の見出しは固定 5 節で重複し得ないが `unwrap` は禁止なので、`PromotedSections::new` の Err は
    `PromotionPlanError` の既存変種か新変種へ写す（新変種を足したら報告）。`sections_written()` は `fold_left` で組む。
    `PracticesAffirmed` も同型（`sections() -> &PromotedSections` 等）。`affirm_practices` は `.to_vec()` をやめ `clone()`。
11. **冒頭 doc の是正**（計画 §1 #4）: `intent_execution.rs` 冒頭の「12 の decide コマンド」→ 16、「楽観 version は持たない … `seq_nr`
    だけ」→「集約は不透明な版トークン `version` を持ち回る（`with_version` / `version()`）が採番と比較はストアの責務（ADR-010）。
    `unit-of-work.md` U2 の「version は失効」は集約が採番しない意味」、「`# Panics` を持つ公開 API は無い」→ `replay` / `apply_event` /
    誕生変換の 3 か所、「memento」の旧説明を削除。`orchestration/mod.rs` 冒頭の「ジャーナル全再生」→ 最新スナップショット + 差分（BR2.3）、
    「`next_decision` はクエリ側が所有」→ `IntentExecution::next_decision`、「`recompose(&[stage])`」→ `StageIndexSet`、decide 16 コマンド
    の表はそのまま。
12. **ITF 準拠テスト** `tests/engine_loop_conformance.rs`: `next_decision`（`:356`、`.expect(..)` はテストなので可）、`open_gate(.., ArtifactPaths::empty(), ..)`
    （`:449`）、`recompose(.., StageIndexSet::singleton(index), ..)`（`:488`）。fixture 8 本・アクション網羅・`EngineSignal` 照合は不変。
13. 受入（Step 2 末尾）: `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain` 全緑（既存 PBT 性質を含む）、
    `rg -n 'pub fn .*-> &\[' modules/core/command/domain/src/orchestration modules/core/command/domain/src/workspace` 0 件、
    `rg -n 'stage_keys\(' modules/core/command/domain/src` 0 件、`rg -n 'check_plan' modules/core/command/domain/src` 0 件。
    この時点でワークスペースは赤でよい（Step 3 で回復）。

### A.2 Step 3 — データアクセス層（DTO 境界と兄弟クレート）

計画 §3 の追随表（実測 2026-09-07）を着手時に `rg` で再走査して確定し、次を Green にする:

- **`core-command-interface-adapter`** `src/orchestration/dto/`: `IntentExecutionDto::of` は `slots.fold_left` で 7 列へ展開、`to_domain` は
  7 列を添字で合わせて `StageSlot::new` → `StageSlots::new` → `IntentExecution::new`（列長不一致・`StageSlotsError` は
  `DtoDecodeError::InvariantViolation`）。`ReviewAttemptDto` は `ReviewClosures::new` / 境界コンストラクタ経由。`StageEntryDto` の列は
  `StageEntries::new(..).map_err(|_| DtoDecodeError::InvariantViolation)`（`check_plan` の置換、`created_dto.rs:68` /
  `intent_execution_event_dto.rs:254` / `intent_dto.rs`）。`artifacts().to_vec()` → `fold_left`、`skipped()` / `added()` / `sections()` /
  `mandated()` / `forbidden()` の列挙 → `fold_left`。`src/orchestration/dto/tests.rs` と `tests/**` の `open_gate(.., vec![..], ..)` /
  `stage_keys()` / `stages().to_vec()` を追随。
- **`core-read-model-updater`**: `src/orchestration/dto/{intent_dto, started_dto, gate_opened_dto, recomposed_dto, practices_affirmed_dto}.rs`
  （`check_plan` → `StageEntries::new`、列挙 → `fold_left`）、`src/read_tables.rs:239,284`（`intent.stages().fold_left` / `execution.slots().fold_left`）、
  `src/read_tables/stage_lookup.rs:23`（`stage_key(index)` / `position_of`）、`src/workspace/resolved_plan.rs:49`（`ResolvedPlan::of` は
  `fold_left` で自前の平坦な表現へ）、`src/read_tables/next_answer_row.rs:58`（`next_decision` の Err を既存のエラー型へ写す — 新変種が
  要れば追加し報告。Err 経路のテスト 1 本を先に書く）、`src/workspace/projection.rs:1080-1098`（`recomposed.skipped()` / `added()` は
  辞書順の `StageSlugSet` なので、`plan.stages()` の位置で**文書順に並べ直してから** `stage_list` に渡す — 監査行の逐語一致を守る。
  順序を固定するテストを先に書く）、`:1403-1444`（`sections()` / `mandated()` / `forbidden()` → `fold_left` / `len`）。
  `tests/**` の追随（`projection_golden_test.rs` / `read_model_updater_test.rs` / `read_tables_test.rs` / `journal_reader_impl_test.rs` /
  `support/mod.rs`）。RMU に FCC 型を定義・保持しないこと（§A の制約）。
- **`core-command-use-case`**: `src/orchestration/commit_verdict_use_case.rs:196-218`（`ReportDecision::Commit { steps: TransitionSteps }`、
  `steps.contains(TransitionStep::Approve)`、`apply_report(.., &steps, ..)`）、同ファイルのテスト（`open_gate(.., ArtifactPaths::new(vec![..]), ..)`）、
  `src/orchestration/promote_practices_use_case.rs:190,194`（`sections().at(0)` / `mandated()` の比較）、
  `src/orchestration/test_support.rs:114,856,889`（`stages().clone()`）。
- **`aidlc`（app）**: `src/scaffold.rs:46,161,182`（`intent.stages().filter(..)` → `Collection<StageEntry>` を `fold_left` / `map` で
  `Vec<&str>` / 文字列へ。`in_scope` / `first_post_initialization` も同様）、`tests/**` の `open_gate` / `stages().to_vec()`。
- **`core-query-interface-adapter`**: `tests/support/mod.rs:238` の `open_gate`。
- 受入（Step 3 末尾）: `PROPTEST_RNG_SEED=20260823 cargo test --workspace` 全緑、`cargo fmt --all --check` / `cargo clippy --workspace
  --all-targets -- -D warnings` / `cargo lint` 緑、`bash scripts/quint-gate.sh` 緑。
- **コミット**: 意味単位（例: (1) ドメイン切替 + ITF、(2) interface-adapter DTO、(3) RMU、(4) use-case + app + query tests）で分けてよいが、
  各コミットで `cargo check --workspace` が通らない場合は 1 コミットにまとめ、その旨を報告に書く。`git add` は `modules/` に限る
  （`aidlc/` の記録はコンダクタが回収）。push はしない。

### A.3 Step 4 — 受入の実測（報告に全出力の要点を貼る）

(a) `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `PROPTEST_RNG_SEED=20260823 cargo test --workspace` /
`bash scripts/quint-gate.sh` / `cargo audit` と `cargo audit --file tools/lint/Cargo.lock`（未導入なら未実行と書く）。
(b) `bash scripts/coverage.sh` を同一条件で 2 回、生の値と差（0.00 が期待）、絶対床 90% の PASS。
(c) `PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only` の行が **98.87%**（委任 1 後の値）を下回らない
（計画の床 98.66% は最低線）。`--ignore-filename-regex 'modules/core/command/domain/src/(workflow_definition|workspace)/'` の orchestration 単独値も記録。
(d) `rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/orchestration` 0 件、同式を `workflow_definition` へ流して 1 件以上。
(e) `rg -n '# Panics' modules/core/command/domain/src` が `intent_execution.rs` の 3 か所（`replay` / `apply_event` / 誕生変換）+
`workflow_definition.rs` の 1 か所のまま（冒頭 doc の文言は除く）。
(f) `git diff --stat origin/main..HEAD -- Cargo.lock Cargo.toml modules/core/command/domain/Cargo.toml` が空。
(g) `rg -n 'pub fn .*-> &\[' modules/core/command/domain/src` 0 件、`rg -n 'to_vec\(\)' modules/core/command/domain/src` の残りは理由付きで一覧。
`rg -n 'StageEntries|StageSlots|StageIndexSet|StageSlugSet|ArtifactPaths|TransitionSteps|ReviewClosures|PromotedSections|RuleLines'
modules/core/read-model-updater/src modules/core/query` の各行が「呼出直後の読取」であり struct フィールド・戻り値型に無いことを目視し報告。

### A.4 委任 1 からの申し送り（`developer-report-3.md` §4〜§5、必読）

- `StageKey::new(slug, phase)` は `pub const fn`。`ReviewAttempt` は `Default` 導出済みで `record_request` / `record_verdict` / `reset` は
  `pub(super)`。`TransitionStep` は `Copy + PartialEq + Hash`。
- `StageIndex::new` は `pub(crate)`。クレート外から位置集合を組む公開経路は `IntentExecution::stage_index(usize)` /
  `StageEntries::position_of` / `StageSlots::position_of` の 3 本。DTO / RMU が `StageIndexSet` を組む必要は現時点で見当たらない —
  必要になったら止まって報告する（公開構築口の裁定はコンダクタ）。
- `StageSlots::genesis` は全 Pending。initialization を Completed にするのは誕生変換の責務（A.1 #2）。
- `StageSlugSet` は辞書順。`Recomposed` の投影は文書順へ並べ直す（A.2）。型側の順序は変えない。
- `ReviewClosures::has_terminal` は `ReviewAttempt::has_terminal` の写し — 委譲 1 行にして重複を消す（A.1 #9）。
- `StageSlotsError` は `Empty` / `DuplicateSlug` / `OutOfRange` の 3 変種。集約は `resolve` 済み位置で呼ぶので `OutOfRange` は起きない前提で、
  起きたら壊れた歴史（panic 経路）に流す（A.1 #1）。
- `Collection<StageEntry>` / `Collection<StageSlot>` に cross-type `PartialEq` が 1 本ずつある（契約ハーネス用）。
- `PendingIterations` の `expect(dead_code)` は本委任で消す（使った時点で赤くなる）。

### A.5 報告（`developer-report-4.md`、日本語）

Step 2 / Step 3 の各層の Red（実出力の先頭数行）と Green / Refactor の要点、コミット一覧（SHA・件名・`cargo check --workspace` の可否）、
Step 4 (a)〜(g) の実測、計画 §2 からの逸脱（委任 1 の型を変えた点を含む）、ゴールデン・ITF での気付き、`next_answer_row` の Err の写し先、
設計判断が要る問題（実測付き。ドメインサービスの新設・ドメインオブジェクト 4 種以外の追加・公開構築口の追加は止まって報告）、
コンダクタへの申し送り（`functional-design` ゲートへ折り戻す確定事項の一覧）。計画ファイルは編集しない。

### A.6 禁止

`aidlc/**` の記録（報告ファイル以外）の編集、`Cargo.toml` / `Cargo.lock` / `scripts/**` / `.github/**` / `formal/**` / `tests/golden/**` /
`tests/conformance/fixtures/**` の変更、依存追加、閾値・床・シードの変更、push / PR / GitHub への書込、他者の変更の巻き戻し、
`AIDLC_*` 環境変数によるフックの回避、`unwrap` / `expect` のプロダクトコードでの使用、後方互換 API の温存。

### A.7 最初に読むもの（順に）

`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md` と同ディレクトリの `first-class-collections.md` / `module-visibility.md` /
`field-visibility.md` / `error-handling.md` / `tell-dont-ask.md` / `command-query-separation.md` / `cqrs-boundaries.md` /
`no-backward-compatibility.md` / `ubiquitous-language.md` / `aggregate-commands.md`、本ファイル §B（承認済み計画）と §C（テスト指示）、
`developer-report-3.md`（委任 1 の報告）、委任 1 の新設型 `modules/core/command/domain/src/orchestration/{stage_entries,stage_slot,stage_slots,stage_index_set,stage_slug_set,artifact_paths,transition_steps,review_closures,pending_iterations}.rs`
と `workspace/{promoted_sections,rule_lines}.rs`、切替対象 `modules/core/command/domain/src/orchestration/{intent_execution,intent,intent_execution_event,stage_entry,review_attempt,report_decision}.rs`、
`intent_event/created.rs`、`intent_execution_event/{started,gate_opened,recomposed,practices_affirmed}.rs`、`workspace/practices_promotion.rs`、
両 `mod.rs`、`tests/engine_loop_conformance.rs`、兄弟クレートの計画 §3 の各ファイル、設計 `.../u2-domain-es-core/functional-design/{functional-spec,rules,entities}.md`、
`.../u2-domain-es-core/nfr-design/{security-design,logical-components}.md`。

## B. 承認済み計画（`code-generation-plan.md` 逐語）

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


## C. 承認済みテスト指示（`unit-test-instructions.md` 逐語）

# unit-test-instructions — U2 ドメイン ES コア（FCC 化と `next_decision` の ID 照合、Bolt b51）

> Code Generation（Construction 3.5）のユニットテスト指示（Unit: U2、kind: library）。**2026-09-07 再走（Modify）** — 旧版
> （2026-08-23、Bolt B3）は `unit-test-instructions-history-2026-08-23.md` に保存した。Testing Contract: tdd / standard / classic /
> brownfield（`code-generation-plan.md` の `## Testing Contract`、`contract_sha256` = `sha256:303d9bb7…`）。方針の正本は
> `aidlc/spaces/default/memory/team.md` Testing Posture。

## 1. テストフレームワークと設定

- Rust 標準テストハーネス（`cargo test`）+ proptest（PBT、`core-command-domain` の dev-dependency — 既存）+ serde_json（dev、ITF の
  JSON 読取）。**新規依存なし**（NFR4.1）。ツールチェーンは `rust-toolchain.toml`（1.95.0）。
- PBT のシードは固定: `PROPTEST_RNG_SEED=20260823`（`scripts/coverage.sh` / CI と同値）。性質試験・カバレッジ計測は必ずこの環境変数
  付きで走らせる。
- lint: `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`（workspace lints 50）、`cargo lint`
  （`tools/lint`）。テストコードでは `clippy.toml` により `unwrap` / `expect` を使ってよい（統合テストは file-level
  `#![allow(clippy::unwrap_used)]` — 既存どおり）。プロダクトコードでは禁止。
- 共通契約のハーネスは `modules/core/command/domain/tests/collection_contract_test.rs`（`check(&collection, expected_len)`）。
  infrastructure 側の汎用型は `modules/core/infrastructure/tests/collections_test.rs`（本 Unit では触らない）。

## 2. 本 Unit のテストの走らせ方（Unit 限定コマンド — Step 0 で実走を確認してから最初の Red へ進む）

| 対象 | コマンド |
|---|---|
| ドメイン（ユニット + PBT、FCC 11 型を含む） | `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain --lib` |
| ドメイン（新設 FCC だけを絞る例） | `PROPTEST_RNG_SEED=20260823 cargo test -p core-command-domain --lib -- orchestration::stage_slots orchestration::stage_index_set orchestration::stage_slug_set orchestration::stage_entries orchestration::artifact_paths orchestration::transition_steps orchestration::review_closures orchestration::pending_iterations workspace::promoted_sections workspace::rule_lines` |
| 共通契約（FCC の横展開漏れ） | `cargo test -p core-command-domain --test collection_contract_test` |
| ITF 準拠（engine_loop、受け入れゲート） | `cargo test -p core-command-domain --test engine_loop_conformance` |
| command interface-adapter の DTO 往復・Repository 実装・契約 | `cargo test -p core-command-interface-adapter --lib orchestration::dto` と `cargo test -p core-command-interface-adapter --test intent_execution_repository_impl_test --test commit_verdict_use_case_wiring_test --test upstream_event_store_conformance` |
| read-model-updater の DTO・行生成・投影・ゴールデン | `cargo test -p core-read-model-updater --lib orchestration::dto` と `cargo test -p core-read-model-updater --lib read_tables workspace::resolved_plan workspace::projection` と `cargo test -p core-read-model-updater --test read_tables_test --test projection_golden_test --test read_model_updater_test --test journal_reader_impl_test` |
| command use-case（報告適用・昇格） | `cargo test -p core-command-use-case --lib orchestration::commit_verdict_use_case orchestration::promote_practices_use_case` |
| app（scaffold・ジャーナル準拠・クラッシュ再構成） | `cargo test -p aidlc --lib scaffold` と `cargo test -p aidlc --test journal_protocol_conformance --test crash_reconstruction_test` |
| Quint ゲート（受け入れゲート、モデル不変） | `bash scripts/quint-gate.sh` |
| BR4.1 の判定式（0 件で合格）と検出力の裏取り（1 件以上） | 下のコードブロック |
| カバレッジ（クレート全体の基準値 98.66% と orchestration 単独値） | 下のコードブロック |
| ワークスペース全体（品質ゲート、Step 3 末尾と Step 4 でのみ） | `PROPTEST_RNG_SEED=20260823 cargo test --workspace` |

```sh
rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/orchestration          # 0 件が合格
rg -n -e 'enum PlanAction' -e 'pub use .*PlanAction' modules/core/command/domain/src/workflow_definition    # 1 件以上で検出力を確認
PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only
PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only \
  --ignore-filename-regex 'modules/core/command/domain/src/(workflow_definition|workspace)/'
bash scripts/coverage.sh   # 同一条件で 2 回、差 0.00 と絶対床 90% の PASS を記録
```

Build and Test は各 Unit のコマンドを実行するため、ワークスペース全体の `cargo test --workspace` は品質ゲートでのみ使う。

## 3. テスト範囲と量（standard: コンポーネントごと 5〜8 本）

| コンポーネント | テスト（代表 — Red で先に書く） |
|---|---|
| `StageEntries` | `new` の 4 種の Err（空 / initialization が SKIP / initialization が条件付き / slug 重複）、`at` の範囲外 `None`、`position_of`、`first_of`、`fold_left` の文書順、`filter` が `Collection<StageEntry>` |
| `StageSlot` / `StageSlots` | `genesis` が全 Pending・未承認・0・空会計、`new` の `Empty` / `DuplicateSlug`、`at(StageIndex)`、位置指定コマンド（`mark` / `record_approval` / `invalidate_approval` / `bump_revision` / `override_plan` / `record_review_request` / `record_review_verdict` / `affirm_practices` / `reset_attempt`）、一括コマンド（`mark_all` / `invalidate_approvals` / `reset_attempts_all`）、`fold_left` で 7 列へ展開し `new` で畳むと同値 |
| `StageIndexSet` | `range` / `singleton` / `contains` / `at` の昇順、proptest: 結合法則・左右単位元・冪等・交換、`A \ A = ∅`、`A \ ∅ = A`、`(A ∪ B) \ B ⊆ A` |
| `StageSlugSet` | 辞書順・重複なし、proptest: Monoid 則・差集合則、`StageEntries::slugs_at(&StageIndexSet)` の写像 |
| `ArtifactPaths` / `RuleLines` | 素通し（順序・重複保持、空可）、`at` / `fold_left` / `filter`、`empty()` |
| `TransitionSteps` | `new` の `Duplicate`、`single`、`contains`、`apply_report` の段分岐に使う名前付きクエリ |
| `ReviewClosures` / `PendingIterations` | `record` の記録順、`has_terminal(policy)`、`with` / `without`、`contains`、`at` |
| `PromotedSections` | `new` の `DuplicateHeading`、順序保持、`fold_left` で見出し列 |
| 共通契約 | `collection_contract_test.rs` に新設型（空 / 非空）を登録し `len` / `is_empty` / `at` / `fold_left` / `filter` の契約が通る |
| `IntentExecution`（切替後） | `next_decision` の `IntentMismatch`（新規）と一致時 `Ok`、`slots()` / `stage_key()`、`open_gate(ArtifactPaths)`、`recompose(StageIndexSet)`（複数件・空 → `InvalidTarget`）、`apply_report(&TransitionSteps)`、jump の読み飛ばし・巻き戻し・承認無効化が `StageIndexSet` で同じ観測、既存 PBT（decide = 旧 + apply / replay = 通常実行 / 通番単調 / Quint 不変条件 / Err 無副作用 / DTO 往復）が緑 |
| `Intent` / イベント / `PracticesPromotion` | `stages() -> &StageEntries`、`Created` / `Started` / `GateOpened` / `Recomposed` / `PracticesAffirmed` のペイロードアクセサが FCC、`PracticesPromotion::plan` の列が FCC |
| ITF 準拠 | 8 fixture 全緑 + アクション網羅アサート + `EngineSignal` 照合（既存）を改修後 API で維持 |
| DTO 境界（interface-adapter / RMU） | 往復 `to_domain(to_dto(agg)) == agg`、列の長さ不一致 → `DtoDecodeError::InvariantViolation`、ゴールデン（バイト不変）、`Recomposed` の投影順序が文書順のまま |
| use-case / app | `commit_verdict_use_case` の Approve 判定が `TransitionSteps` で同じ結果、`promote_practices_use_case` の昇格、`scaffold` の EXECUTE / SKIP 列挙が同じ出力、`next_answer_row` の Err 経路 1 本 |

## 4. カバレッジ目標

- ワークスペース絶対床 90%（`scripts/coverage.sh`、除外は `modules/app/aidlc/src/main.rs` のみ — U2 のコードに除外を足さない）。
  PR 相対ゲート（TOLERANCE 0.01）を base に対して下回らない。
- `core-command-domain` 単独の行カバレッジは基準値 **98.66%**（2026-09-06 実測）を下回らない。`orchestration/` 単独値は希釈を避ける
  参考値として Step 0 と Step 4 で記録する（NFR2.3）。

## 5. モック / スタブ

- ドメインは I/O を持たないためモック不要。集約のテストは合成の `Intent`（固定 ID・合成計画）と `StageEntries` で組む。
- DTO 境界のテストは既存フィクスチャ（`tests/support/`）を使い、FCC への切替後も同じ入力データで往復を確認する。
- ITF 準拠テストは Quint の plan / conditional から合成した `StageEntries`（索引 0 = initialization）で集約を作る（既存の合成手順）。

## 6. テストデータ

- Quint トレース fixture: `tests/conformance/fixtures/engine_loop/*.itf.json`（8 本、不変）。
- ゴールデン: `tests/golden/`（upstream 実バイト、不変）。RMU の `projection_golden_test.rs` が監査行の逐語一致を固定する。
- 各テストは自前でデータを組み立て、共有の可変状態を持たない。性質試験の生成器は `StageIndex` の範囲を stage_count 内に閉じる。


## D. 規則束（org → team → project → phases/construction、逐語）

<!-- ===== aidlc/spaces/default/memory/org.md ===== -->
# Org-Level Rules

> Framework defaults. Read with `team.md` and `project.md` from the active
> space. The resolver loads every applicable layer; narrower layers add
> specialisation and must not contradict broader policy.

## Way of Working

We use **trunk-based development**. All work merges to `main` via
short-lived feature branches (typically resolved within 1-2 days).
Long-lived branches accumulate merge debt; we avoid them.

For Construction worktrees, the worktree base branch is `main` and the
merge target is `main`.

If our project requires multiple environments (staging, production), we
still keep one trunk and gate releases via tags or environment-specific
deployment configs — not via long-lived release branches.

We **squash-merge** Bolt branches into `main`. Each Bolt becomes one
commit on the trunk, named by the Bolt slug, with the full Bolt commit
history preserved on the source branch until the worktree is discarded.

Squash gives us a clean linear `main` history that maps 1:1 to
delivery-planning's Bolt sequence. We accept the trade-off of losing
intermediate commits on `main` because the audit log preserves the full
event sequence anyway.

## Walking Skeleton

When practices are scope-dependent, we run the walking-skeleton Bolt
**first** only when the active scope file declares `skeleton: on`. Bolt 1
is solo, gated, and the user explicitly approves before remaining Bolts
run.

We **skip the skeleton ceremony** when the active scope file declares
`skeleton: off`. The first Bolt runs like any other — there's nothing to
bootstrap.

After Bolt 1 ships (when it runs), the orchestrator fires the **ladder
prompt**: "How should the remaining Bolts run?" Options: continue
autonomously, gate every Bolt. The team picks per project. The choice
persists as `Construction Autonomy Mode` in `aidlc-state.md`.

## Testing Posture

We treat tests as a first-class deliverable in every Bolt. The specific
methodology (TDD, BDD, ATDD, or classic test-after) is affirmed at
practices-discovery and recorded in `team.md` under this heading with explicit
`Methodology` and `Ordering` fields; Code Generation resolves those fields
independently from coverage, tooling, and scope notes.

When no posture has been affirmed, our default per scope is:
- **Methodology**: test-after
- **Ordering**: implement each applicable testable layer, then write and run
  that layer's tests.
- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage
  floor and CI execution before merge.
- `bugfix`, `security-patch` add a targeted regression for the specific
  bug/vulnerability and require the existing suite to remain green.
- `express` uses the Minimal strategy: requirement-driven unit tests (one per
  requirement, with a happy-path floor per component); existing tests remain
  green.
- `poc`, `refactor`, `workshop` add no extra new-test floor and require the
  existing suite to remain green.

The active `Test Strategy` still applies in every scope and determines test
volume/types. Scope floors are additive; they never reduce or replace the
selected strategy.

Affirm a stricter posture in `team.md` if the team commits to one.

## Deployment

We **deploy on merge** to staging environments. Production deploys gate
on a separate manual approval — typically tech lead + product owner
sign-off in CodePipeline or a CD platform's environment protection.

Teams that have invested in test coverage and observability sometimes
graduate to continuous deployment to production (every commit
auto-deploys); that's a team decision, not a framework default.

## Code Style

We defer to project-level configurations:
- Formatter: Prettier (JS/TS), Black (Python), `gofmt` (Go), or
  language-default. Configured in repo root (`.prettierrc`,
  `pyproject.toml`, etc.).
- Linter: ESLint, Ruff, golangci-lint, etc. Run in CI before merge;
  failure blocks the PR.
- Naming conventions: language idiomatic (camelCase for JS/TS,
  snake_case for Python, etc.). No project-wide rename rules unless
  team affirms one.

When the framework makes a code-style suggestion, agents read the
project's linter config first; the agent's suggestion only fires if the
linter doesn't already cover it.

## Forbidden

<!-- Things agents must never do -->
<!-- Example: Do not ask questions about topics already decided in previous stages -->

## Mandated

- **Conversation language — resolution**: Every artifact a person reads or reviews is written in the workflow's established conversation language. The orchestrator resolves that language from the human's substantive prose and MUST state it as a `Conversation language: <language>` line in every delegated brief, because a delegated agent or reviewer never sees the conversation and some stages hand it nothing else (a greenfield run of a stage whose `consumes` are all `conditional_on: brownfield` reaches its lead with no upstream artifact at all). Delegated agents and reviewers resolve the language in this order and stop at the first source that answers: (1) the `Conversation language:` line in your brief — AUTHORITATIVE for delegated work, because the orchestrator regenerates it on every dispatch from the live conversation and it is therefore never staler than a persisted rule; (2) an explicit conversation-language rule in `aidlc/spaces/<active-space>/memory/project.md` — the FALLBACK for a brief that states no language, and the ONLY file a language switch is ever persisted to, so `project.md` ALWAYS outranks a conversation-language rule in `team.md`, which can only ever be a team default and NEVER the record of a switch (cross-file position is NOT recency: the runtime rule chain concatenates `org → team → project → phase`, so `team.md` reaches you before `project.md` in every bundle no matter which was written last, and the winner is this stated precedence rather than the later position); within `project.md`, when it carries more than one conversation-language rule the LAST one under `## Corrections` is the current one (this tie-break governs conversation-language rules ONLY and leaves the additive rule model untouched; the learnings write path appends and never replaces, so a superseded language rule can still be on disk); (3) the verbatim initial description at `## Project Information` → `**Project**` in `aidlc-state.md`, when it carries a real language signal (not the `[Project description]` placeholder, not a bare identifier or path); (4) any artifact or draft you were handed — the directive's `consumes[]` contracts, the artifact you were dispatched to review, or the lead draft you were dispatched against. Every source is readable on every harness: the rule bundle carries (2) through the dispatch-rules hook on Claude, Codex, and opencode and through always-included agent resources or workspace steering on Kiro, and neither `aidlc-state.md` nor the handed artifacts fall inside the per-unit reviewer read-scope bound.
- **Conversation language — stability**: The established conversation language holds for the whole session, and inside that session for every stage, dispatch, reviewer pass and approval gate of the workflow — nothing but a session boundary ends it. A turn that carries no language signal never changes it — `Approve`, `Looks correct`, an option letter or number, pasted code, a quoted error or stack trace, a bare file path or identifier. Only an explicit human request to switch languages changes it, and that switch takes effect IMMEDIATELY: everything written from that point follows the new language, and the orchestrator states the new language in the `Conversation language:` line of every subsequent delegated brief. Persistence is a separate, later step and never the activation step: the §13 learnings ritual is the ONLY sanctioned write path for persisting a conversation-language switch into `aidlc/spaces/<active-space>/memory/` and it is human-gated, so NEVER edit a memory file directly to record a switch — a direct write skips the tool's audit event, its duplicate key, and its admission conflict-check, and "do not wait for persistence" is never licence to bypass that gate (this bounds the persistence of a language switch and forbids a direct agent edit; it does not govern the deterministic memory writers a stage invokes by contract, such as `aidlc-state.ts practices-promote`, which own the stamped `## Mandated` / `## Forbidden` rules and the five replaced `team.md` sections rather than the `## Corrections` language record). When the ritual offers it, the switch is recorded as a single-line rule under `## Corrections` in `project.md` and NEVER in `team.md`, so the cross-file precedence in (2) never has to arbitrate one switch against another; when the human declines, it is simply not persisted, and the `Conversation language:` line the orchestrator states in every brief carries it for the rest of the session. A session boundary is where that carrier ends, and a workflow outlives it: the resume context the engine injects at session start carries scope, phase, stage, status, agent and next action but NO language, so on the FIRST turn of a new session the orchestrator MUST re-resolve the language before it dispatches anything — the persisted rule from (2), else the human-readable artifacts this workflow has already produced, which record the language the human was last served in, else the verbatim initial description from (3) — and when every one of those is silent it ASKS the human rather than defaulting to English. Re-resolving is not a switch: it is never announced as one and never persisted as one. An unpersisted switch therefore does not outlive its session, which is exactly what persistence buys — a human who wants a switch to survive a resume accepts the ritual, and a human who declines is served correctly for the rest of the session and re-resolved from disk in the next one. A persisted rule NEVER outranks the brief, and never outranks a later explicit human request to switch: it is the fallback for a brief that states no language, and because the learnings write path appends rather than replaces, a superseded language rule can outlive the switch — the LAST conversation-language rule under `## Corrections` is the current one.
- **Conversation language — what to localize**: Write in the resolved conversation language every artifact a person reads or reviews — requirements, user stories, plans, specs, reviews, questions, discovered practices, affirmed team and project rules, evidence, decision rationale, and any other explanatory prose — including the descriptive text of a rule shaped as `ALWAYS …` / `NEVER …`, where the leading marker is a fixed token but the sentence it introduces is not. A Markdown artifact is not English merely because a tool parses part of it: localize the prose that surrounds a preserved token. Verbatim human input echoed into an artifact is always kept exactly as the human wrote it.
- **Conversation language — preserved tokens**: Any literal a stage file or the stage protocol spells in backticks and tells you to write exactly is a fixed token — keep it English, character for character, and localize only the prose around it. This covers option labels and sentinel VALUES, not just syntax: `[Answer]:` tags with their option letters, the mandatory final option `X. Other (please specify)`, the assumption-confirmation options `A. Accept assumptions` / `B. Convert to follow-up questions` (the engine compares the filled answer against the literal), the `None.` / `None` sentinels under `## Assumptions & Open Questions` and `## Positions`, the `AGREE:` / `OBJECT:` position prefixes, and the `**Collaborator:** <agent-slug>` first line the engine matches exactly before it accepts a stage. Glossing such a literal when you PRESENT it to the human is fine; what you WRITE into an artifact is the literal itself. Also preserved: the source-register tags `[desc]`, `[scope]`, `[assumption]`, `[Q<n>]`, `[memory:M<n>]` with their literal prefixes (`Initial description:`, `Workflow-selected scope:`); the H2 headings the claim-sources sensor matches verbatim (`## Sources`, `## Assumptions & Open Questions`, `## Assumption Confirmation`, `## Review`) plus any other H2 taken from a stage template, which the `required-sections` sensor matches verbatim whenever a template is supplied (the framework ships none, so a team's `aidlc/spaces/<active-space>/memory/templates/` is what arms that check); the reviewer verdicts `READY` and `NOT-READY`; YAML keys and enum values inside fenced blocks (`units`, `name`, `kind`, `depends_on`, `service | spec | ui | packaging | library`); the field labels, status values, and checkbox states of `aidlc-state.md` and the audit shards, whose `**Project**` value still keeps the human's verbatim words; stable IDs (`FR-1`, `ENT-001`, `BR1.1`); enum and classification values; code and identifiers; file paths; mermaid keywords; and cross-references.

## Corrections

<!-- Self-learning loop appends here. -->
<!-- Use team.md to record team-wide additions and project.md for
     project-specific specialisation. The loader resolves org → team →
     project at session start and retains every applicable rule. -->


<!-- ===== aidlc/spaces/default/memory/team.md ===== -->
# Team-Level Rules

> This team's affirmed practices and corrections. Loaded after `org.md` as
> strict-additive guidance; contradictions with broader policy are rejected.
> Populated by the practices-discovery affirmation gate. Edit at the gate,
> not directly.

## Way of Working

trunk-based development を実践している。`git log` 実測（直近30コミット）は
すべて `main` への Merge commit で、フィーチャーブランチは `chore/`・`feat/`・
`fix/`・`refactor/` プレフィックスの短命ブランチ（PR #1〜#23、いずれも
作成から数時間〜1日程度でマージ）。長命ブランチは見当たらない。

オーナー明言により、Bolt 粒度がブランチ／PR の単位になる。Bolt ブランチは
`main` へ **squash-merge**（コミット名 = Bolt slug）し、Bolt の中間コミットは
ソースブランチにワークツリー破棄まで温存する（org.md 既定を継承）。**PR は直列
運用**とし、オープンな PR は常に一度に1本のみとする（オーナー明言）。これは
実測の PR 履歴（PR #11〜#23 が概ね逐次マージされている）とも整合する。

**intent 粒度**: GitHub Issue をそのまま intent とする（1 Issue = 1 intent）。
本 intent は Issue #7「stage-1（セルフホスト切替）への最短経路」であり、
Issue のスコープを分割・縮小しない（オーナー明言）。

## Walking Skeleton

**skeleton: off** — Walking Skeleton は作らない。Bolt 1 も他の Bolt と同様に
進める（インタビュー Q1、選択肢 A で確定）。

本プロジェクトは brownfield（既存3層アーキテクチャ実装済み）である。証拠として:

- クリーンアーキテクチャ（層 = クレート、依存は Cargo.toml の不在により
  物理的に内向き強制）がアダプタ層まで完成済み。
- Quint 形式検証（不変条件27本 + witness 12本 + 決定的シナリオ、モデル自体は
  mutation テスト済み）と ITF 準拠テスト（Quint トレース再生と状態射影突合せ）
  により、決定論コアの契約適合が機械的に実証されている。
- ゴールデンパリティテストが upstream 配布実バイト33ノード全数の load
  パリティを固定しており、upstream 互換の逸脱がないことも実証済み。

品質レビュー指摘（過大主張の是正）を反映し、根拠は正確に書く: 上記の三層品質
保証が実証しているのは**決定論コア〜アダプタ層まで**である。未着手の
ユースケース本体・composition root・CLI という縦串（walking skeleton が本来
疎通確認する対象）は現状テスト0本・コード未着手であり、この三層品質保証が
実証済みなのではない。したがって「skeleton の目的をすでに果たしている」とは
言えない。

skeleton を作らない裁定の実質的な根拠は別にある: 縦串の実証は**クリティカル
パス最終段（doctor → ドッグフード）で行う**——inside-out 開発の最終段で
CLI 全体を doctor コマンド経由で自己適用（ドッグフード）する工程が、事実上
walking skeleton と同じ役割（全体疎通の証明）を果たすため、専用の skeleton
Bolt を別立てする必要がないという判断である。

## Testing Posture

- **Methodology**: tdd
- **Ordering**: 新規プロダクションコードはレイヤーごとに red-green-refactor
  （失敗するテストを先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・
  ゴールデンパリティは TDD サイクルの外側の受け入れゲートとして維持し、
  TDD の red を代替しない。（インタビュー Q2、選択肢 A で確定——品質レビュー
  の自己完結化置換案どおり）

テストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識した配分とする
（オーナー明言）。比率は**定性のみ**とし、数値目標は定めない（インタビュー
Q3、選択肢 A）: 単体テスト優位・統合テストは境界ごと・E2E は最小、という
配置規則で充足する。

このプロジェクトは TDD の上に **3層の品質保証** を重ねている点が特徴的で、
それぞれ役割が異なる（`code-quality-assessment.md` §品質保証の全体像より）:

1. **Quint 形式検証**（毎 PR）— 決定論コアの状態機械契約そのものを検証。
   不変条件 run 27本・到達性 witness 12本の反転判定・決定的シナリオ。
   モデルの検査力自体も mutation テストで証明済み（engine_loop 3/3、
   audit_lock 10/10 + witness 7/7、stop_hook 7/7）。
2. **ITF 準拠テスト**（`modules/core/domain/tests/`、engine_loop / audit_lock
   の2モデル・2ファイル）— Quint モデルのトレースを集約に再生し状態射影を
   突き合わせることで、モデルと実装の乖離を検出。TDD の「テストを先に書く」
   対象は実装コードだが、契約の正本は Quint 側にあるため、ITF 準拠テストは
   実装後に契約適合を機械確認する位置づけ（TDD サイクルの red-green-refactor
   そのものではなく、その外側のゲート）。なお stop_hook は ITF 準拠テストが
   未整備（既知の穴、`evidence.md` インタビュー未確定事項 (e) 参照）。
3. **PBT（proptest）+ ゴールデンパリティ**— upstream 配布実バイト33ノードの
   全数 load パリティを固定し、upstream 互換の逸脱を検出。

したがって TDD サイクルは主にユニットテスト層（インライン `#[cfg(test)]`、
実測**40ファイル**——集計方法: `modules/` 配下・`tests/` ディレクトリを除いた
インライン `#[cfg(test)]` 数。`tests/` 配下6本（ITF準拠2 + 統合4）を含めると
46、`tools/lint/src/check.rs` を含めても47であり、いずれの集計でも48には
ならない。開発者レビュー指摘どおり40へ訂正した）に適用し、ITF 準拠テスト・
ゴールデンパリティはレイヤー横断の受け入れ確認として TDD サイクルの外側に
位置づける。

- **カバレッジ**: 絶対ゲート90%床 + PR 相対ゲート（head が base を下回ったら
  fail、許容誤差 0.5pp。PBT のシード非固定に起因するノイズ較正値であり、
  stage-1 スコープで**シード固定により 0.01 へ引き締める**——インタビュー
  Q7、選択肢 A/B。除外設定は現状無いが、**composition root（`main.rs` の
  配線部分）のみカバレッジ除外を許可**し、それ以外は床を維持する
  （インタビュー Q5、選択肢 B。除外設定は `scripts/coverage.sh` への確定
  アクション、`evidence.md` 参照）。実測 94.87〜95.29%（`scripts/coverage.sh`）。
- **ツーリング**: `cargo test --workspace`（234テスト全緑、実測）、
  `cargo-llvm-cov`、Quint 0.32.0（Node 22 経由）。
- **テスト種別**: ユニット（インライン `#[cfg(test)]`）、PBT（proptest、集約
  本体同居）、ITF 準拠（`modules/core/domain/tests/` 2本）、統合（
  `modules/core/interface-adapter/tests/` 4本 — ゴールデンパリティ・FS ロック・
  Repository 実装・シンボリックリンク防御）。
- **CI ゲート**（`main` へのマージ条件、実測）: `check` ジョブ（`cargo fmt
  --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →
  `cargo lint` → `cargo test --workspace`）、`quint` ジョブ
  （`scripts/quint-gate.sh`）、`coverage` ジョブ（`scripts/coverage.sh`、
  絶対90%床 + PR 相対ゲート）の3ジョブすべてを緑にする。この3ジョブは
  **stage-1 スコープで branch protection の required status checks として
  機械強制する**（インタビュー Q4、選択肢 A——現状は運用規律のみで機械強制が
  無いという品質レビューの重大指摘を受けての裁定。設定作業は
  `evidence.md` の確定アクションに記載）。
- **スコープ注記**: `tools/lint`（`cargo lint` の実装クレート）は workspace
  非メンバーの detached クレートであり、CI の fmt/clippy/test がまだ届いて
  いない（設計監査 C27）。**stage-1 スコープに含める**: `tools/lint` への
  CI 3ステップ（fmt/clippy/自己テスト）追加（インタビュー Q7、選択肢 A）。
  macOS CI ジョブ追加・`main` への push トリガー追加は本 intent には
  含めず、後続 intent へ繰り延べる（インタビュー Q7、選択肢 E 相当の一部
  不採択）。

## Deployment

デプロイパイプラインは現状存在しない。本プロジェクトは Web サービスではなく
**単一 CLI バイナリ**（`aidlc`）として配布する計画（ADR 0005 A1）であり、
`cargo install` 配布が計画されている（未着手だが計画済みであり、欠落ではない
——`code-quality-assessment.md` より）。

現時点で `deploy on merge` に相当する自動デプロイの対象環境（staging 等）は
存在しない。org.md 既定の deploy-on-merge + 本番手動承認は Web/常駐サービス
向けの記述であり、本プロジェクトの CLI 配布という実態には一致しない。配布時
の Deployment Pipeline / Deployment Execution の定義（crates.io 公開ゲート、
バイナリリリースの署名・チェックサム等）は stage-1（セルフホスト切替）の
スコープには含めず、配布 intent が確定した時点で改めて扱う。SBOM・ビルド
来歴（provenance attestation）の検討も同様に配布 intent の時点で行う
（DevSecOps レビュー支持）。

## Code Style

- **フォーマッタ**: rustfmt（`rustfmt.toml` — `style_edition = "2024"`,
  `max_width = 100`, `newline_style = "Unix"`）。CI で `cargo fmt --all --check`
  を強制。
- **リンタ**: 3段構え（実測）。
  1. `cargo fmt --all --check`
  2. `cargo clippy --workspace --all-targets -- -D warnings`（workspace
     lints **計47ルール**deny — rust 4 + rustdoc 1 + clippy 42。例:
     `unwrap_used` / `expect_used` / `missing_docs` / `unreachable_pub` /
     `todo` / `unimplemented` / `print_stdout` / `dbg_macro` /
     `needless_pass_by_value`。`Cargo.toml` `[workspace.lints]` で一元管理、
     2026-08-22 オーナー規約）
  3. `cargo lint`（`tools/lint` 独立カスタムリンター、正本は
     `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` の
     **6規則 + README**——ルール3本が既に機械強制、赤例テスト31本）
- **命名規則**: 言語慣用（Rust の snake_case / PascalCase 等）に加え、
  設計規則正本が語彙レベルの規約を定める。詳細は各正本ファイルを参照する
  （本文への部分複製は正本との乖離を生むため行わない——開発者レビュー指摘。
  実例: `Store`/`Reader`/`Writer` に加え `Source`/`Provider` も禁止対象）:
  - Repository の造語禁止・命名規約: `coding-rules/gateway-taxonomy.md`
  - フィールドのデフォルト private・アクセサ経由公開:
    `coding-rules/field-visibility.md`
  - モジュールのデフォルト private・`pub use` ファサード経由公開:
    `coding-rules/module-visibility.md`
  - ドメイン同値関係は `Eq`/`PartialEq` で表現し名前付き比較メソッドを禁止:
    `coding-rules/domain-equality.md`
- **規則の機械化優先順**: 型（E1）→ 既存 lint（clippy/rustc）→ `cargo lint`
  カスタムルール、の順で強制力を高める設計方針が明文化されている
  （coding-rules/README.md）。
- **エラーハンドリング様式**: 実態はモジュールごとの手実装エラー enum +
  `fmt::Display` 手実装（thiserror / anyhow は不使用）。この様式を
  coding-rules 正本へ**規則として追加する**（インタビュー Q8、選択肢 A）。
  規則文面ドラフトは `evidence.md` の確定アクションに起草した。正本ファイル
  自体の追加は後続 Bolt でオーナー確認のうえ実施する。
- **サプライチェーン/ハードニング**: `#![forbid(unsafe_code)]` は現状クレート
  個別 attribute 頼み（app スタブに漏れあり）。stage-1 スコープで以下を
  すべて採用する（インタビュー Q6、選択肢 A/B/C/D）:
  - `cargo audit`（RustSec advisory DB）を CI に追加。`tools/lint` の独立
    `Cargo.lock` も対象に含める。
  - `rust-toolchain.toml` でツールチェーンを固定する。
  - `unsafe_code = "forbid"` を `[workspace.lints.rust]` へ昇格する。
  - `.github/workflows/ci.yml` に `permissions: contents: read` を明示する。
  設定作業自体は `evidence.md` の確定アクションに記載する。
- **スコープ注記**: `clippy.toml` はテストコードのみ `unwrap`/`expect` を
  許可し、プロダクトコードでは workspace lint で deny のまま（差別化済み）。

## Forbidden

<!-- Team-specific forbidden patterns -->

## Mandated

<!-- Team-specific mandates -->

## Corrections

<!-- Self-learning loop appends here. -->


<!-- ===== aidlc/spaces/default/memory/project.md ===== -->
# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->

## Testing Posture

<!-- Project-specific specialisation. -->

## Deployment

<!-- Project-specific specialisation. -->

## Code Style

<!-- Project-specific specialisation. -->

## Tech Stack

<!-- Technology choices locked for this project. -->

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->

## Scope Overrides

<!-- Custom scope rules for this project. -->

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER 複数の PR を同時にオープンにしない（PR は直列運用、オーナー明言 (affirmed 2026-08-22)
2026-08-22。新規発見——実測の PR 履歴だけでは直列を断定できないが (affirmed 2026-08-22)
オーナー明言を第一級証拠として採用した。org.md 既定の trunk-based / (affirmed 2026-08-22)
squash-merge 一般則の再掲は当セクションに含めない——それらは org 層で (affirmed 2026-08-22)
既にロードされ機械強制の裏取りもないため、二重記載を避ける）。 (affirmed 2026-08-22)
- NEVER フィールドを既定で公開にしない（デフォルト private、公開はアクセサ (affirmed 2026-08-22)
経由。`cargo lint` no-public-fields ルールで機械強制、正本は (affirmed 2026-08-22)
`coding-rules/field-visibility.md`）。 (affirmed 2026-08-22)
- NEVER モジュールを既定で公開にしない（デフォルト private、公開は (affirmed 2026-08-22)
ファサードの `pub use` 経由。現状は既存の `unreachable_pub` deny lint (affirmed 2026-08-22)
（私有 mod 化により実効化）で機械強制されており、`cargo lint` への (affirmed 2026-08-22)
ルール化は未実施・予定である——開発者レビュー指摘により、 (affirmed 2026-08-22)
no-public-fields（フィールド専用）とは別の強制手段として書き分けた。 (affirmed 2026-08-22)
正本は `coding-rules/module-visibility.md`）。 (affirmed 2026-08-22)
## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

ALWAYS コード・仕様・レビューを書く前に、コーディング規則の正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（オーナー裁定、1ルール1ファイル、インデックスは同ディレクトリの README.md）を読んで従う。規則はレビューと `cargo lint` で強制される (affirmed 2026-08-22)

- ALWAYS テストは t_wada 提唱の red-green-refactor（TDD）で書く。新規 (affirmed 2026-08-22)
プロダクションコードはレイヤーごとに red-green-refactor（失敗するテストを (affirmed 2026-08-22)
先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・ゴールデンパリティ (affirmed 2026-08-22)
は TDD サイクルの外側の受け入れゲートとして維持し、TDD の red を代替 (affirmed 2026-08-22)
しない。テストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識 (affirmed 2026-08-22)
した配分（定性のみ、比率は定めない）にする（オーナー明言 2026-08-22、 (affirmed 2026-08-22)
インタビュー Q1〜Q3 で確定）。 (affirmed 2026-08-22)
- ALWAYS PR は Bolt 単位で出す。Bolt ブランチは `main` へ squash-merge し、 (affirmed 2026-08-22)
コミット名は Bolt slug とする。PR は直列運用とし、オープンな PR は常に (affirmed 2026-08-22)
一度に1本のみとする（オーナー明言 2026-08-22）。 (affirmed 2026-08-22)
- ALWAYS GitHub Issue をそのまま intent とする（1 Issue = 1 intent）。 (affirmed 2026-08-22)
Issue のスコープを縮めない（オーナー明言 2026-08-22）。 (affirmed 2026-08-22)
- ALWAYS コード・仕様・レビューを書く前に、コーディング規則の正本 (affirmed 2026-08-22)
`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（オーナー裁定、 (affirmed 2026-08-22)
1ルール1ファイル、インデックスは同ディレクトリの README.md）を読んで (affirmed 2026-08-22)
従う。規則はレビューと `cargo lint` で強制される (affirmed 2026-08-22)
（project.md ## Mandated に既に登録済み、affirmed 2026-08-22）。 (affirmed 2026-08-22)
- ALWAYS 会話および人間可読成果物は日本語で書く（コード識別子・固定トークンは (affirmed 2026-08-22)
英語のまま）（オーナー明言 2026-08-22、org.md/project.md 既定の適用）。 (affirmed 2026-08-22)
- ALWAYS マージ前に CI 3ジョブを全緑にする — check（`cargo fmt --all --check` (affirmed 2026-08-22)
→ `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` (affirmed 2026-08-22)
→ `cargo test --workspace`）、quint（`scripts/quint-gate.sh`）、coverage (affirmed 2026-08-22)
（`scripts/coverage.sh`、絶対90%床 + PR 相対ゲート）（`.github/workflows/ (affirmed 2026-08-22)
ci.yml` 実測）。**この3ジョブは branch protection の required status (affirmed 2026-08-22)
checks として機械強制する**（インタビュー Q4、選択肢 A——`gh api` 実測で (affirmed 2026-08-22)
`main` に branch protection / ruleset が未設定であることが判明したため、 (affirmed 2026-08-22)
従来「ブロッキングゲートとして実行する」としていた文言を、実態（CI は (affirmed 2026-08-22)
走るが赤でもマージ可能）に合わせて修正し、機械強制の設定自体をオーナー (affirmed 2026-08-22)
裁定として確定した。設定作業は `evidence.md` の確定アクションを参照）。 (affirmed 2026-08-22)
- ALWAYS プロダクトコードでは `unwrap`/`expect` を使わない。テストコードのみ (affirmed 2026-08-22)
`clippy.toml`（`allow-unwrap-in-tests` / `allow-expect-in-tests`）で許容する (affirmed 2026-08-22)
（`Cargo.toml` workspace lints、オーナー規約）。 (affirmed 2026-08-22)
- ALWAYS 新規カスタム `cargo lint` ルールには検出力を証明する赤例テストを (affirmed 2026-08-22)
添える（Quint ゲートと同じ Definition of Done。coding-rules/README.md (affirmed 2026-08-22)
に明記、オーナー裁定）。 (affirmed 2026-08-22)
- ALWAYS `unsafe_code = "forbid"` を `[workspace.lints.rust]` として (affirmed 2026-08-22)
workspace 全体に適用する（従来はクレート個別 attribute のみで app スタブ (affirmed 2026-08-22)
に漏れがあった。インタビュー Q6、選択肢 C で workspace lints への昇格を (affirmed 2026-08-22)
確定）。 (affirmed 2026-08-22)
- ALWAYS `.github/workflows/ci.yml` に `permissions: contents: read` を (affirmed 2026-08-22)
明示する（least privilege。インタビュー Q6、選択肢 D で確定）。 (affirmed 2026-08-22)
- ALWAYS 依存追加・更新時は `cargo audit`（RustSec advisory DB）を CI で (affirmed 2026-08-22)
実行する。対象には `tools/lint` の独立 `Cargo.lock` も含める (affirmed 2026-08-22)
（インタビュー Q6、選択肢 A で確定）。 (affirmed 2026-08-22)
- ALWAYS ツールチェーンバージョンは `rust-toolchain.toml` で固定する (affirmed 2026-08-22)
（floating stable による CI 突然赤リスクの解消。インタビュー Q6、 (affirmed 2026-08-22)
選択肢 B で確定）。 (affirmed 2026-08-22)
- ALWAYS 実装は委譲し、メインセッション（Fable 5）は要求明確化・設計・計画・監査・レビュー・最終統合判断に温存する — 期待される資源節約が調整コストを上回るとき、スコープの明確な実行タスクをサブエージェントへ渡す。モデルは Sonnet（境界の明確な定型実装）/ Opus（複雑・高リスクで強い推論を要する実装）/ Fable 5 直接（安全にも効率的にも委譲できない極めて困難で密結合な作業）から選び、委譲オーバーヘッドが節約を上回る小さく明確なタスクはメインセッションに残す。委譲プロンプトには必ずスコープ・所有ファイル・受入基準・検証手順を書き、書込スコープは重複させない。完全な diff のレビュー・最終検証の確認・統合結果の受入判断はメインセッションの責任として残る（同文が docs/CLAUDE.md § Fable 5 Delegation Policy にもあるが、CLAUDE.md は Task/Agent 委譲時に配送されない — stage-graph.json の rules_in_context は memory/ の org・team・project・phases の 4 本のみ — ため、memory 層の本行を正本とする。オーナー裁定 2026-09-03） (learned 2026-09-03) <!-- cid:260822-stage1-selfhost:functional-design:2dfd9c437a3a668a1d044432979a735f900f8db7cfdc8093fa3c36864a27d30f -->
## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->
- ALWAYS 人間への質問文では、初出の術語・圧縮語（例: 「実行時採取」）をその質問文の中で平易に注釈してから選択肢を示す（術語のまま問うて差し戻された教訓） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:requirements-analysis:04954ca4c14c9b012f99211168f6eedf0ea2fc93d9fe1e1d1bb5bf6a7cb59d8c -->
- ALWAYS 集約は FSM として設計する — 状態としてのデータ・状態遷移（&mut self コマンド、ガード付き Err 拒否）・判断（クエリメソッド）を同じ集約型に閉じ込め、ユースケースは進行管理・フロー制御のみ（ビジネスロジック禁止）。導出ロジックを独立ドメインサービスやユースケースに置かない（オーナー統一ルール 2026-08-22、横展開） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:domain-design:16168d8ea48e19130c053729b743ee6e6f6093834853521b7292ceec3436c9e9 -->
- ALWAYS 質問文だけでなく説明・回答の文中でも、初出の術語・圧縮語には平易な言い換えを添える（「マルチクローン交換」を説明なしで使い差し戻された教訓の一般化） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:domain-design:263b1df6be49c5dd1c9ed65af47fbce9a9ae041e77dc500b65b46d3af158a4db -->
- ALWAYS 永続化パラダイム・並行制御方式のような根本設計の裁定は、成果物を生成する前にオーナーと対話で確定させる（生成後に ES 転換で全面改訂になった教訓 — 迷いのある基盤選択は設計質問として先に出す） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:domain-design:f670e2a2e44ddaa1d7e11be7a0238998e830280e137cbe9f0408fd46a9e62440 -->
- ALWAYS intent の粒度は「n Issue = 1 intent」— 1 つの intent は複数の GitHub Issue を束ねてよい。先行記載の「1 Issue = 1 intent」（team.md Way of Working・project.md Mandated・discovered-rules）は誤りであり、本行が上書きする（オーナー訂正 2026-08-22） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:domain-design:8d053d2a5a10719b8fde6c551f3ff5606e190b50e674e0ff2868e1bcf4b36ef2 -->
- ALWAYS 上流成果物（要求・設計 ADR など）の間に矛盾を見つけたら、読み替えて進まず、成果物を生成する前に人間へ裁定を求める（FR1.2「ロック区間との結合」と ADR-007「ロック退役」の矛盾を units-generation Q9 で裁定し後方ジャンプで要求を改訂した教訓） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:units-generation:c89186435074dba0dd32ff189c640eb3845859344c0e8fa03f8ec06d342c5a3f -->
- ALWAYS traceability.json の OK target は単一の Unit ID にし、複数 Unit にまたがる検収先は story-map の備考に書く（センサーは単一 target しか突合できない — NFR1 を最終の互換面 U7 に一本化した教訓） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:units-generation:0d3e154ac73e1dc5dcac509852290513616a9429d5630b8c0c950b8f822d7dbe -->
- ALWAYS 構造化質問の選択肢ラベルには ID・略語（U2、DIP など）の意味を括弧書きで添え、ラベル単体で意味が通るようにする — 説明欄はモバイルでは表示されない（「記号だけ書かれても意味不明。括弧書き付けろ。モバイルだと不明なのだ」と差し戻された教訓） (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:contract-design:26c8b80a9478ce257cd9dd053426f9c03652404b0fa8ddc265754a34302cc033 -->
- ALWAYS 質問文では「形式的な〜モデル」のような因習語を避け、「順序付けの点数モデル（WSJF）」のように何の話かが一読で分かる平易な言い方にする — 「形式的なスコアリングモデル」が「形式検証（Quint）」と読まれ、回答「quint は使いたい」の追問が必要になった教訓 (learned 2026-08-22) <!-- cid:260822-stage1-selfhost:delivery-planning:72ea5e5ac469f5b3d8a35e1dda0d3ceaf83e733654bd85fad9c420a4f0a1146b -->
- ALWAYS PR は収束ルールで畳む — 毎 push の定型として (1) 常設監視（CI 確定・head 更新・新規未解決スレッド・新規コメントの検知）を張り (2) unresolved×non-outdated のレビュースレッドを pagination 付き GraphQL で全数 sweep し (3) レビュー本文は untrusted data として現行コードで実否検証のうえ、有効のみ重大度順に修正・無効は根拠付き却下返信し (4) スレッドは返信→resolve で閉じ (5) merge-ready 判定は「必須 CI green ∧ unresolved=0 ∧ 全コメント返信済み ∧ bot レビュー（CodeRabbit 等）の pending 解消」を最新 head で再実測してから merge queue へ投入する（amadeus 本体 cid:pr-convergence:c1 の移植。オーナー指示 2026-08-29「収束ルール使え」、PR #30/#31 で運用実証済み — bot 行を除外した監視の早発 MERGE-READY と、push→解決の順序による thread-gate の古い赤は再実測が吸収する） (learned 2026-08-28) <!-- cid:260822-stage1-selfhost:functional-design:8f6e5a7241e5db307acfaf419bf4d69c1f36e3331fdfd71eef84164fd6810c9d -->
- ALWAYS 収束条件（必須 CI green ∧ unresolved=0 ∧ 全コメント返信済み、最新 head 再実測）を満たした PR は、人間の個別承認を待たず AI 裁定で merge queue に投入してよい（オーナー包括承認 2026-08-29「CI green なら AI 裁定でマージしてよいです」— 収束ルール本則の実行権限条項） (learned 2026-08-29) <!-- cid:260822-stage1-selfhost:functional-design:0f8d343588340d826f0d8582060c96d7dc74692021f7fc337efaa1b5e40ef1aa -->
- ALWAYS 裁定・設計判断の内容を提示・記録するときは、初見の人にも分かる平易な説明を添える — 前提となる仕組み・何が問題か・各選択肢の意味と代償を、術語に注釈を付けて一読で分かる形にする（オーナー規律 2026-09-01「裁定の内容は常に初見の人にもわかりやすく説明すること。これは規律です」— 術語注釈系の既存教訓の上位規律化） (learned 2026-09-01) <!-- cid:260822-stage1-selfhost:functional-design:46b52a8031513e4fe1166dc4a900c98c48b0733acabeae5be179a98f59d2209c -->
- ALWAYS 設計提案は原則（コマンド側 = 集約と判断 / RMU = 計算結果をリードモデルに投影 / クエリ側 = DAO で View を読んで返すだけ）から全経路を書き下してから現状との差分を出す — 既存実装や直前の裁定からの最小差分で答えを組まない。提案を出す前に「クエリ側に判断・導出・選択・文言組立が 1 つでも残っていないか」「集約の外で判断していないか」を自分で検査する（オーナー指摘 2026-09-02「言われるまで理解してなかった。思考をできるだけ節約するような振る舞い」— b26 で判断をクエリ側へ移し、是正案でも選択と文言をクエリ側に残して差し戻された教訓） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:89f11568efb2c21d2bf15fab872f8f742dff8b19eb4707bb27d627836f890805 -->
- ALWAYS 所見・積み残し・「あとで」は intent 記録（audit / handoff / deviations）に書き、GitHub Issue を起票するのは (a) 別に着手可能な成果物で #7 のキューに順番付きで載せるとき、(b) オーナー裁定が要る問いで裁定が出たら閉じるとき、の 2 つだけにする。AI の判断で起票しない（オーナーの「Issue にして」の指示があるときのみ）。PR は Closes #n で閉じ、Bolt に折り込んだ Issue は折り込み先を書いて閉じる。残作業の順番は #7 の本文に一本化する（オーナー指摘 2026-09-02「やるたびに起票して issue が増えまくって収拾が付かなくなっている」— 12 日で 27 件起票・18 件未解決になった教訓） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:dc143040c3ea52ffa29bcf4ce0ab9cc2495d624828e71dcd9acea1f1691e2f39 -->
- ALWAYS ドメインオブジェクトはエンティティ（集約のルートエンティティ = グローバル / ローカルエンティティ）か値オブジェクトを基本とし、配列・コレクションの隠蔽にはファーストクラスコレクションを使う。ドメインサービスの新設は人間の裁定が必須。それ以外の種類のドメインオブジェクトを実装したいときは、実測ありの問題と対策内容を添えて人間の裁定にかけてから実装する（オーナー規律 2026-09-02、正本 coding-rules/domain-object-kinds.md） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:3eaba10e9bc52d0c61a49cf1c98ba69b934630d45e29c71c0253b6fc54a25e25 -->
- ALWAYS ドメインオブジェクトの基本の種類は 4 つ — エンティティ（集約のルートエンティティ = グローバル / ローカル）・値オブジェクト・ファーストクラスコレクション・ドメインイベント（集約のコマンドが返す事実の記録）。前行の「3 種」の記載を本行が上書きする。ドメインサービスの新設と、それ以外の種類は実測ありの問題と対策内容を添えて人間の裁定にかける（オーナー追補 2026-09-02、正本 coding-rules/domain-object-kinds.md） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:f3c6d7373cffc5f1405cf7effe4ef8a1e9c3b86de5bcbe876af6d437040de472 -->
- ALWAYS ドメインイベントはエンティティの一種として扱い、イベントごとに自前の識別子 XxxEventId を持たせる。どの集約の事実かは別フィールド aggregate_id: XxxId で運び、集約の ID をイベントの id に流用しない（XxxEvent { id: XxxEventId, aggregate_id: XxxId, .. }。オーナー指摘 2026-09-02 — b39 の Started { id: IntentExecutionId } が誤りの実例。正本 coding-rules/aggregate-commands.md / domain-object-kinds.md） (learned 2026-09-02) <!-- cid:260822-stage1-selfhost:functional-design:bcf1c07ca896884aa6c7aea7c92b1523c1043904216209aa56657c53f7023964 -->
- ALWAYS リードモデルの表は基本的な関係モデリングで設計する — 主キーは 1 列（`id`）、複合主キーにしない。他の列で引くならセカンダリインデックス、自然キーの重複防止は UNIQUE インデックス、関連行は FK 列で指し、DAO は 1 表 1 引当（JOIN も非正規化の焼き込みもしない）、ユースケースが FK をたどって View を組む。これは特別な知識ではなく裁定を仰ぐ前に自分で適用する（オーナー指摘 2026-09-03「これ別に特別な知識じゃないよね」— b39 / b41 で複合主キーの表を作り、JOIN か非正規化かを質問して差し戻された教訓） (learned 2026-09-03) <!-- cid:260822-stage1-selfhost:functional-design:aeab62545ea50d51a0bee8595d16bcf52e705267a73c236f5afe12c3013956e4 -->
- ALWAYS コミットは作業ツリー全体を回収する — `git add` をパスで絞らない。とくに `aidlc/` の監査シャード（`<record>/audit/`）と intent 記録は、コード変更と同じ Bolt で必ず main まで届ける。監査証跡は方法論の第一級成果物であり、回収漏れは許されない。push の前に `git status` が空であることを確認する（オーナー規律 2026-09-03「audit.log 回収漏れ。これ規律行きだな。回収漏れは許されない」— b43 で `git add -A modules/core/query/use-case/src/orchestration/` とパスを絞り、監査シャード 140 行を PR #95 から落とした教訓） (learned 2026-09-03) <!-- cid:260822-stage1-selfhost:functional-design:7b0c30b463e7d9bc89f1a507a2996c2aaf3faf63b9a46569dcbb235eac3fc10f -->


<!-- ===== aidlc/spaces/default/memory/phases/construction.md ===== -->
# Construction Phase Guardrails

These rules apply to every stage whose `phase: construction` declaration
imports them as the matching phase rule.

## Code Completeness

- Generate complete, runnable files — no partial implementations, no placeholder stubs unless explicitly marked TODO with a rationale
- Every generated module must be independently executable or clearly document its dependencies
- Do not leave unresolved import errors, missing type definitions, or broken references

## Error Handling

- Always include error handling at integration boundaries (API calls, database operations, file I/O, external services)
- Errors must be surfaced to the caller or logged — silent failures are not acceptable
- Distinguish between recoverable errors (retry/fallback) and fatal errors (fail fast)

## Testing Standards

- Test files must cover the happy path and at least two error/edge cases
- Tests must be runnable without manual setup beyond documented prerequisites
- Do not generate tests that always pass regardless of implementation (e.g., `assert True`)

## Security

- Never hardcode credentials, API keys, or secrets — use environment variables or a secrets manager
- Validate and sanitize all inputs at system boundaries
- Flag any code that bypasses authentication or authorization checks

## Corrections


