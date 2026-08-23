AIDLC-UNIT: u2-domain-es-core
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3
Conversation language: 日本語（コミットメッセージ・報告・コード内コメント／rustdoc は日本語可。識別子・固定トークンは英語）

# developer-brief-2 — U2 ドメイン ES コア（Bolt B3 / 委任 2: orchestration 側の ES 化 = 計画 Step 9〜20）

あなたは aidlc-developer-agent。承認済み計画 `code-generation-plan.md`（下に全文）と `unit-test-instructions.md`（下に全文）の
**Step 9〜Step 20** を実装する。委任 1（Step 1〜8: PlanAction 移動、WorkflowDefinitionId / DefinitionRevision、find_by_id）は完了済みで
ワークスペースは緑 — その成果（`core_domain::workflow_definition::{PlanAction, WorkflowDefinitionId, DefinitionRevision}`、
`WorkflowDefinition::id()/revision()`）をそのまま使う。埋め込みの Testing Contract（tdd / standard / brownfield）が権威。TDD: 各 Red で失敗する
コマンド出力を報告ファイルに記録してから Green に進む。

## 作業場所・ブランチ・コミット
- ワークスペースルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、ブランチ `bolt/b3-u2-domain-es-core`。`git push` / PR 作成はしない。
  コミットは意味単位（例: `feat(core-domain): event-sourced WorkflowExecution — events, snapshot, StageIndex`、`feat(core-domain): decide/apply commands`、
  `test(itf): replay engine_loop traces through the event-sourced aggregate`）。`aidlc/` 配下はコミットしない。`git add -A` は使わない。
- 計画ファイル（`code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`）は**絶対に編集しない**。進捗・Red 記録・
  逸脱・棚卸し（I2 / I6）・カバレッジ・品質ゲート・コミット一覧は
  `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/code-generation/developer-report-2.md` に書く（新規、Write）。

## 所有するファイル（委任 2 の書込範囲）
- `modules/core/domain/src/orchestration/**`（`workflow_execution.rs` 全面改訂、`workflow_execution_event.rs` / `workflow_execution_snapshot.rs` /
  `stage_index.rs` / `stage_entry.rs` / `next_decision.rs` / `status.rs` / `start_error.rs` / `command_error.rs` / `apply_error.rs` / `snapshot_error.rs`
  / 必要なら `intent_id.rs` / `phase_boundary.rs` の新設、`mod.rs` の `pub use` 列挙）。`skeleton_stance.rs` / `verdict.rs` / `autonomy_mode.rs` /
  `jump_direction.rs` は変更なし（`jump_direction` に `derive` が既にある — 再利用）。
- `modules/core/domain/tests/engine_loop_conformance.rs`（新 API への書き換え）。`audit_lock_conformance.rs` は触らない。
- `modules/core/domain/src/lib.rs`（クレート rustdoc の追記のみ、必要なら）。
- 他クレート（use-case / interface-adapter）に `WorkflowExecution` の利用者が無いことは実測済み（doc コメントのみ）。`core-domain` の
  `Cargo.toml` は不変（依存追加なし）。

## 受入基準（委任 2 の Done）
1. `core_domain::orchestration` の公開面（`pub use` 列挙）が logical-components §1 の列挙どおり: `WorkflowExecution`、`WorkflowExecutionEvent`
   （+ 変種ごとのペイロード型、`PhaseBoundary`）、`WorkflowExecutionSnapshot`、`StageIndex`、`StageEntry`、`IntentId`（既存が無ければ新設）、
   `NextRequest` / `NextDecision` / `EngineSignal`、`Status`、`StartError` / `CommandError` / `ApplyError` / `SnapshotError`、既存の
   `AutonomyMode` / `JumpDirection` / `SkeletonStance` / `Verdict` 等。旧 API（`report_forward` / `gate_start` / `reject` / `revise` /
   `report_skipped` / `recompose_flip` / `next`）は**削除**。`PlanAction` は再輸出しない。
2. 12 コマンド（`complete_stage` / `open_gate(artifacts)` / `approve_gate(user_input, phase_boundary)` / `reject_gate(feedback)` / `revise_stage` /
   `skip_stage(reason)` / `jump(target)` / `park` / `unpark` / `recompose(flips)` / `set_autonomy(mode)` + `start`）が **1 コマンド 1 イベント**で
   `Result<WorkflowExecutionEvent, CommandError>` を返し、Err は状態不変。各 decide は `occurred_at: &str` を受け取り封筒に載せる（集約は時計を
   持たない）。`apply_event` が通常実行とリプレイの同一経路。
3. `gated(s) = stages[s].phase != PhaseId::Initialization`（索引 0 特別扱いなし）。`complete_stage` は非ゲートのみ（ゲートで呼ぶと `InvalidTarget`）、
   `approve_gate` はゲートのみ。birth の `complete_stage` ×n は呼出側（ユースケース）の責務 — 集約は 1 回 1 イベント。
4. `next_decision(&self, &WorkflowDefinition, &NextRequest) -> Result<NextDecision, CommandError>`: (0) `def.id() != self.definition_id` →
   `DefinitionMismatch{expected, actual}`（revision の差は Ok）、以下 BR3.1 の優先順 (1)〜(7)。`EngineSignal::from(&NextDecision)` で 4 値へ。
   `jump_resolve` / `stale_report`（`Ok(NextDecision::Done)`）はクエリ。
5. `snapshot()` は 16 属性（intent_id / definition_id / definition_revision / stages / plan / overlay / conditional / checkbox / cursor / status /
   parked_at / autonomy / approved / revision_count / seq_nr / version）、`from_snapshot` は不変条件（長さ一致 / cursor in-scope / active ≤ 1 /
   gated Completed ⇒ approved / parked_at = cursor / seq_nr ≥ 1）を検証し違反は `SnapshotError::InvariantViolation(reason)`。`with_version(v)`。
6. `apply_event`: `seq_nr != self.seq_nr + 1` → `SequenceGap{expected, actual}`、未知 slug → `UnknownStage`、適用後の不変条件違反 →
   `InvariantViolation`（一時コピーに適用して検証してから差し替え — Err で状態不変）。
7. `StageIndex` は集約だけが構築（`stage_index(usize) -> Option<StageIndex>`）、集約内部の添字はすべて `StageIndex` 経由、`# Panics` 0 件、
   `unwrap` / `expect` / `panic!` / 範囲外添字なし。
8. PBT（`workflow_execution.rs` 同居、`PROPTEST_RNG_SEED` 固定、既定 256 ケース、コマンド列 ≤ 60、合成定義 stage_count 2〜8 / initialization 1〜3）:
   (a) decide 後 == 旧 + apply、(b) replay(events) == execute(commands)、(c) seq_nr 単調 + SequenceGap、(d) Quint 不変条件（cursor_in_scope /
   no_gate_bypass / at_most_one_active / parked_position / unpark_restores_position）、(e) Err 無副作用、(f) `from_snapshot(snapshot()) == self`。
9. ITF 準拠 `engine_loop_conformance.rs`: 8 fixture 全緑 + 既存のアクション網羅アサート。合成 `WorkflowDefinitionId::parse("itf")`、
   `DefinitionRevision::parse("sha256:" + "0"*64)`、Quint の plan / conditional から `StageEntry` 列（索引 0 = initialization、他は任意の非 init
   PhaseId、slug は `stage-<i>` 等）を作り `WorkflowExecution::start_with_entries(intent_id, definition_id, definition_revision, scope, request,
   entries, occurred_at)` で集約を作る（`start(&def, …)` はこれに委譲）。対応表: `report_forward` → 索引 0（非ゲート）は `complete_stage`、
   gated は `approve_gate`；`report_awaiting_approval` → `open_gate`；`report_rejected` → `reject_gate`；`report_revised` → `revise_stage`；
   `report_skipped` → `skip_stage`；`jump_*` → `jump`；`park` / `unpark`；`recompose` → `recompose(&[s])`；`set_autonomy`（モデルはトグル →
   反転値を set）；`next*` / `done_stutter` → `next_decision` + `EngineSignal::from`；`report_stale` → `stale_report`。射影は現行 `assert_projection`
   と同じ（overlay ⇔ `effective_plan`、parkedAt -1 ⇔ None、Running ⇔ running ∧ !parked_active、WorkflowParked ⇔ parked_active、
   WorkflowCompleted ⇔ Completed）。
10. 実グラフ索引テスト（ユニット）: initialization 3 ステージ（phase = Initialization）+ 非 init 数ステージの合成 `StageEntry` 列で、索引 0〜2 が
    非ゲート（`complete_stage` 可 / `open_gate` は `InvalidTarget`）、索引 3 以降がゲート、`jump(1)` が `InvalidTarget`。
11. 品質ゲート緑: `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo lint`、`cargo test --workspace`、
    `bash scripts/coverage.sh`（絶対床 90%）、`cargo llvm-cov -p core-domain --summary-only`（着手前基準 lines 94.70% を下回らない）。
    合格 grep（BR4.1）= 0 件を再確認。
12. 報告ファイル `developer-report-2.md`: 変更ファイル一覧、各 Red の失敗出力、設計との差分（判断）、棚卸し I2 / I6、カバレッジ実測、品質ゲート結果、
    コミット一覧。最終応答は「## Subagent Summary」形式。

## 委任 1 からの申し送り（developer-report-1.md §7 — 必ず読む）
- 委任 1 は完了済み（コミット 6cda871〜9210685、ワークスペース緑: fmt / clippy / lint / test 368 passed）。`core_domain::workflow_definition::{PlanAction,
  WorkflowDefinitionId, DefinitionRevision}`、`WorkflowDefinition::{id(), revision(), stages_in_scope(), grid().action()}` が使える。
- 委任 1 が定義側から**削除した PBT 2 性質**（実効プランの合成 = サフィックスがグリッドに勝つ / 次の in-scope ステージの読み飛ばしの最小性 =
  `next_in_scope_stage_is_the_first_qualifying_node_in_document_order` と `suffixes_beat_the_grid_and_absence_is_none`）の等価物を、集約側の
  PBT（`effective_plan` と `next_decision` の RunStage 先）として**必ず復活**させる。
- `IntentId` は既存に無い（実測 0 件） — `orchestration/intent_id.rs` に新設（`<kebab-slug>-<id8>` を parse、Always Valid）。
- `engine_loop_conformance.rs` は import パスのみ修正済みで旧 API を使って緑 — 本体の書き換え（Step 16）はあなたの仕事。
- 委任 1 の報告全文: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/code-generation/developer-report-1.md`。

## 設計の所在（必要な箇所を読む。他 Unit の construction/ 配下は読まない）
- 機能設計（正本）: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/{entities,rules,functional-spec}.md`
  — entities（16 属性・12 ペイロード・StageEntry・NextDecision）、rules BR1.0〜BR1.9 / BR2.1〜BR2.6 / BR3.1〜BR3.3 / BR5.1〜BR5.4、
  functional-spec §2 / W1〜W6 / §4 状態遷移表 / §5 エラー。
- NFR: `.../nfr-requirements/{security-requirements,tech-stack-decisions}.md`、`.../nfr-design/{security-design,logical-components}.md`（検査点 3 か所、
  モジュール分割、テスト配置）。
- 契約: `.../inception/contract-design/contract-summary.md`（C5 イベント語彙 — ペイロードの形）、ADR: `.../inception/domain-design/decisions.md`（ADR-001〜008）。
- Quint モデル: `formal/orchestration/engine_loop.qnt`（不変条件・アクション — モデルは変更しない）。fixture: `tests/conformance/fixtures/engine_loop/*.itf.json`。
- 既存実装（書き換え元）: `modules/core/domain/src/orchestration/workflow_execution.rs`（現行 FSM のガード集合・PBT・`amadeus-lint: allow` の書式）、
  `modules/core/domain/src/workspace/checkbox.rs`（`CheckboxState` の述語 is_in_flight / is_active / is_finished）。
- コーディング規則（必読、正本）: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/*.md`。

## 規則の束（逐語 — org / team / project / construction）

<!-- org.md -->
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

<!-- team.md -->
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

<!-- project.md -->
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

<!-- phases/construction.md -->
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

## 承認済み計画（code-generation-plan.md 全文 — 読み取り専用）

# code-generation-plan — U2 ドメイン ES コア（`u2-domain-es-core`）

> Code Generation（Construction 3.5）の計画（Unit: U2、kind: library、Bolt: B3、規模 L）。出典:
> `../functional-design/functional-spec.md`（§2 インターフェイス、W1〜W7、§4 状態遷移、§5 エラー）、`../functional-design/rules.md`
> （BR1.0〜BR1.9 / BR2.1〜BR2.6 / BR3.1〜BR3.3 / BR4.1〜BR4.2 / BR5.1〜BR5.4）、`../functional-design/entities.md`（エンティティ正本）、
> `../nfr-requirements/security-requirements.md`（NFR1.1〜NFR4.5）、`../nfr-requirements/tech-stack-decisions.md`、
> `../nfr-design/security-design.md`、`../nfr-design/logical-components.md`（モジュール分割・テスト配置・B3 の範囲拡張）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C4（find_by_id）/ C5 / C6）、`../../../inception/domain-design/
> decisions.md`（ADR-001〜008）、`../../../inception/units-generation/unit-of-work.md`（U2）、`../../../inception/requirements-analysis/
> requirements.md`（FR1.3 / FR2.1 / FR3.1 / FR3.3 / FR8.3 / FR8.4、NFR1〜NFR4）、`../../../inception/delivery-planning/bolt-plan.md`（B3）、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（全規則）、`code-generation-questions.md`（Q1）。
>
> 実装はワークスペースルート（`modules/core/domain/`、`modules/core/use-case/`、`modules/core/interface-adapter/`、`tests/golden/`）に
> 書く。記録ディレクトリにはコードを置かない。brownfield: 既存ファイルはその場で変更し、複製ファイルを作らない。**後方互換の
> ための旧 API（`report_forward` / `gate_start` / `find()` 等）は残さない**（オーナー裁定 2026-08-23）。

## 1. 前提と範囲

- **作るもの**: (1) `core-domain` の `WorkflowExecution` をイベントソーシング形 FSM に全面改訂（decide / apply_event / snapshot /
  from_snapshot / with_version、12 イベント、`StageIndex` / `StageEntry`、`NextRequest` / `NextDecision` / `EngineSignal`、エラー 4 型）。
  (2) `PlanAction` の `workflow_definition` への完全移動（FR8.3）と `effective_plan_action` / `next_in_scope_stage` の削除（FR8.4）。
  (3) `WorkflowDefinition` のエンティティ化 — `WorkflowDefinitionId` / `DefinitionRevision` の新設と `id()` / `revision()`（ADR-008）。
  (4) C4 改訂の波及: `core-use-case` の `WorkflowDefinitionRepository::find_by_id(&WorkflowDefinitionId)`（`find()` 削除）と
  `GraphReadError::{NotFound, HarnessIdentity}` の追加、`core-interface-adapter` の `WorkflowDefinitionRepositoryImpl`（id は
  `<data_dir>/harness.json` の `name`、revision は 3 入力の正準 JSON の `sha256:`）と `InMemoryWorkflowDefinitionRepository`、
  既存テスト（repository impl test / golden parity test）、ゴールデン dir への `harness.json`（upstream ピンの実バイト）。
  (5) ITF 準拠テスト `engine_loop_conformance.rs` の新 API への書き換え。
- **作らないもの**: Repository（`WorkflowExecutionRepository`、SQLite、EventStore — U3）、投影（U4）、ユースケース（U5 / U6）、
  CLI（U7）、Quint モデルの改訂（不要 — BR2.5 の射影で 1:1）、仕様文書の改訂（12 号 §2.1 の識別子追記は U9）。
- **ブランチ**: `origin/main`（#26 のスカッシュ `0092761`）から `bolt/b3-u2-domain-es-core` を切る。最初のコミットは aidlc 記録
  （`aidlc/` 配下）、以降はコードのコミット（意味単位）。PR は Bolt ゲート承認後にコンダクタが 1 本だけ開く（直列運用、
  squash-merge、コミット名 = Bolt slug）。開発エージェントは push / PR を行わない。
- **定義の識別子（Q1）**: `WorkflowDefinitionId` の値は `<data_dir>/harness.json` の `name`（upstream ピンにも同ファイルあり —
  `claude`）。Q1 = B なら `aidlc:<name>`。`DefinitionRevision` = `canon_json::hash_canonical(JsonValue{ "stage_graph": <stage-graph.json
  の値>, "scope_grid": <scope-grid.json の値（欠損時は導出グリッドを直列化）>, "scopes": [<identity frontmatter を name 昇順>] })`
  （`sha256:<hex64>`）。revision は値属性であって ID ではない。
- **コーディング規則**（正本 `coding-rules/`）: フィールド既定 private（アクセサ公開）、型ファイル mod 既定 private（公開はコンテキスト
  直下 mod.rs の `pub use` 列挙のみ、利便再エクスポート禁止 — `orchestration` は `PlanAction` を再輸出しない）、ドメイン同値は
  `PartialEq` / `Eq`、`unwrap` / `expect` はプロダクトコード禁止、`missing_docs` deny、手実装エラー enum + `Display` + `Error`、
  Tell-Don't-Ask（checkbox の分類は `CheckboxState` の述語、ゲート前提集合は `// amadeus-lint: allow(checkbox-vocabulary)` + 不変条件番号）、
  集約は Repository を呼ばない、ユースケース層は trait のみに依存。

## 2. 公開 API（設計の写し — 実装の契約）

```text
// core_domain::orchestration（ファサード pub use のみ）
WorkflowExecution::start(id: IntentId, def: &WorkflowDefinition, scope: &str, request: String)
    -> Result<(WorkflowExecution, WorkflowExecutionEvent), StartError>     // Started を返す。def.id()/revision() を記録（検査しない）
complete_stage(&mut self) / open_gate(&mut self, artifacts: Vec<String>) / approve_gate(&mut self, user_input: Option<String>, phase_boundary: Option<PhaseBoundary>)
reject_gate(&mut self, feedback: Option<String>) / revise_stage(&mut self) / skip_stage(&mut self, reason: String)
jump(&mut self, target: StageIndex) / park(&mut self) / unpark(&mut self) / recompose(&mut self, flips: &[StageIndex]) / set_autonomy(&mut self, AutonomyMode)
    -> Result<WorkflowExecutionEvent, CommandError>                       // 1 コマンド 1 イベント、Err は状態不変
apply_event(&mut self, &WorkflowExecutionEvent) -> Result<(), ApplyError>  // seq_nr 連続性 / UnknownStage / 不変条件
next_decision(&self, &WorkflowDefinition, &NextRequest) -> Result<NextDecision, CommandError>   // DefinitionMismatch を検査
jump_resolve(&self, StageIndex) -> Result<JumpDirection, CommandError>     stale_report(&self, StageIndex) -> Result<NextDecision, CommandError>
snapshot(&self) -> WorkflowExecutionSnapshot    from_snapshot(WorkflowExecutionSnapshot) -> Result<Self, SnapshotError>   with_version(self, u64) -> Self
stage_index(&self, usize) -> Option<StageIndex>  accepts_commands(&self) -> bool  definition_id() / definition_revision() / intent_id() / stages() / cursor() / checkbox(StageIndex) / approved(StageIndex) / effective_plan(StageIndex) / gated(StageIndex) / status() / parked_at() / autonomy() / revision_count(StageIndex) / seq_nr() / version() / stage_count()
EngineSignal::from(&NextDecision)   // RunStage / Done / Parked / EngineError の導出（BR3.1）
WorkflowExecutionEvent { intent_id, seq_nr, schema_version = 1, occurred_at, payload: 12 変種 }   // 封筒 + ペイロード、アクセサ公開
StageIndex（集約だけが構築）、StageEntry { slug, phase, plan_action, conditional }、NextRequest { resume, reentry, free_text }、NextDecision（8 値）
StartError { UnknownScope, Empty, InitializationMustExecute, InitializationMustBeUnconditional }
CommandError { NotRunning, CheckboxPrecondition { stage, actual }, NotSkippable(StageIndex), NotStale(StageIndex), InvalidTarget(StageIndex), RefusedUnderAutonomy, DefinitionMismatch { expected, actual } }
ApplyError { SequenceGap { expected, actual }, UnknownStage(StageSlug), InvariantViolation(String) }   SnapshotError { InvariantViolation(String) }

// core_domain::workflow_definition
PlanAction（移動）、WorkflowDefinitionId（parse、非空）、DefinitionRevision（parse、`sha256:<hex64>`）
WorkflowDefinition::new(id, revision, graph, grid, scopes)、id()、revision()   // effective_plan_action / next_in_scope_stage は削除

// core_use_case::orchestration
trait WorkflowDefinitionRepository { fn find_by_id(&self, id: &WorkflowDefinitionId) -> Result<WorkflowDefinition, GraphReadError>; }
GraphReadError += NotFound { expected: WorkflowDefinitionId, actual: WorkflowDefinitionId } / HarnessIdentity { path, cause }   // harness.json 欠落・不正
```

設計からの差分（記録）: `occurred_at` はコマンド引数ではなく `WorkflowExecution::start` / 各 decide が受け取る `occurred_at: &str`（ISO 8601
UTC の文字列、呼出側が時計から渡す）— 集約は時計を持たない（NFR3.1）。`IntentId` は既存 `workspace` コンテキストに無ければ
`orchestration` に Domain Primitive として新設（`<kebab-slug>-<id8>` を parse）。`PhaseBoundary` は C5 の `phase_boundary` 投影材料の
値レコード（`from_phase` / `to_phase`、呼出側供給 — 集約は検証しない）。`Status` は `workflow_execution.rs` のインライン定義から private mod
`status.rs` に切り出す（module-visibility）。`skeleton_stance` / `verdict` は触らない。

## 3. 規則の実装方針（BR → コード）

| 規則 | 実装 |
|---|---|
| BR1.0 accepts_commands | `fn accepts_commands(&self) -> bool { self.status == Running && self.parked_at != Some(self.cursor) }`。unpark 以外の decide は先頭でこれを検査し `NotRunning` |
| BR1.1 1 コマンド 1 イベント | decide = ガード → イベント構築（`self.next_event(payload, occurred_at)`）→ `self.apply_event(&ev)`（Ok 前提）→ `Ok(ev)`。ガード不成立で `self` に触れない。PBT (a) decide 後 == 旧 + apply |
| BR1.2 / BR1.3 / BR1.4 / BR1.5 | `gated(s) = stages[s].phase != PhaseId::Initialization`。`complete_stage` は非ゲートのみ（ゲートで呼ぶと `InvalidTarget`）、`approve_gate` は gated のみ、前提 checkbox は現行 FSM と同じ集合。`skip_stage` は InProgress / Revising ∧（conditional ∨ 実効 SKIP） |
| BR1.6 jump | `jump_resolve` で検証（target < stage_count、非 initialization、in-scope、redo は cursor 非 initialization）→ `Jumped{direction, source, target, stages_reset, stages_skipped}`（slug 列）を構築、apply 側が direction / target から approved 消去を導出（backward: target 以降、redo: source） |
| BR1.7 / BR1.8 | park は gated のみ → `Parked{stage}`、unpark は park 中のみ → `Unparked{}`、recompose は全件検査してから `Recomposed{skipped, added, stages_in_scope}`、set_autonomy → `AutonomyModeSet{mode}` |
| BR1.9 | `stale_report(&self, s)`: accepts_commands ∧ s < cursor ∧ Completed ⇒ `Ok(NextDecision::Done)`、それ以外 `NotStale` / `NotRunning` |
| BR2.1 / BR2.3 | 封筒 seq_nr = 現在値 + 1 でなければ `SequenceGap`。apply は一時状態に適用して不変条件を検証してから差し替え（Err で状態不変）。PBT (b) replay == execute |
| BR2.2 Started | `start` は `is_valid_scope` → `stages_in_scope(scope)`（文書順・全ステージ・PhaseId）+ `graph().nodes()[i].execution()` の索引 zip で `StageEntry` 列。None → SKIP。initialization が SKIP / conditional なら Err。Started = {definition_id, definition_revision, scope, request, stages} |
| BR2.4 / C5 | 変種 12（Started / StageCompleted / GateOpened / GateApproved / GateRejected / StageRevised / StageSkipped / Jumped / Parked / Unparked / Recomposed / AutonomyModeSet）、ペイロードは C5 + `c5_revision_proposal`。`revision_count` は集約フィールド（reject で +1）。網羅 match（`#[non_exhaustive]` 無し） |
| BR2.5 ITF | `engine_loop_conformance.rs`: 合成 `WorkflowDefinitionId("itf")` / `DefinitionRevision("sha256:0…0")`、Quint の plan / conditional からステージ列を合成（索引 0 = initialization、他 = 任意の非 init フェーズ）。`report_forward` → 索引 0 は `complete_stage`、gated は `approve_gate`；`report_awaiting_approval` → `open_gate`；`report_rejected` → `reject_gate`；`report_revised` → `revise_stage`；`report_skipped` → `skip_stage`；`jump_*` → `jump`；`park` / `unpark`；`recompose`（1 要素）；`set_autonomy`（反転）；`next*` → `next_decision` + `EngineSignal::from`。合成定義の作り方は `WorkflowDefinition` を組み立てずに `WorkflowExecution::from_snapshot` 相当の合成 Started で集約を作る（ITF 用コンストラクタ `start_from_entries(...)` を `#[cfg(test)]`…ではなく、テスト側が `StageEntry` 列を直接与える公開関数 `WorkflowExecution::start_with_entries(id, definition_id, definition_revision, scope, request, entries)` を使う — `start` はこれに委譲） |
| BR2.6 / ADR-008 | `start` は def.id()/revision() を記録、`next_decision` は id 不一致で `DefinitionMismatch`。Repository: `find_by_id(id)` は harness.json の name と一致しなければ `NotFound{expected: harness 側, actual: 要求}`… 注: `expected` = Repository が提供できる id、`actual` = 要求された id |
| BR3.1 / BR3.3 | `next_decision` の優先順 (0) 定義 id → (1) park → (2) resume → (3) free_text → (4) completed → (5) in-flight ∧ SKIP → (6) in-flight → (7) next in-scope / Done。`EngineSignal::from` で 4 値へ導出。`jump_resolve` と `jump` の分離 |
| BR4.1 / BR4.2 | `plan_action.rs` を `workflow_definition/` へ移動、`orchestration/mod.rs` から `mod plan_action` / `pub use` を削除、呼出側 10 ファイル（現行の `use crate::orchestration::PlanAction` → `crate::workflow_definition::PlanAction`）を一斉修正。`WorkflowDefinition::effective_plan_action` / `next_in_scope_stage` と対応テストを削除（テストは集約側 / `grid().action()` 照会に書き換え）。合格 grep: `grep -rnE 'enum PlanAction\|pub use .*PlanAction' modules/core/domain/src/orchestration` = 0 |
| BR5.1 / BR5.2 / BR5.3 / BR5.4 | `StageIndex`（`usize` newtype、`Copy`、`Ord`、集約だけが構築 — `pub(crate)` コンストラクタ + `WorkflowExecution::stage_index`）。snapshot は全 16 属性、serde なし。`with_version` は値を置くだけ。エラーは手実装 + `std::error::Error` |
| NFR2.2 PBT | 既存 PBT（`quint_invariants_hold_under_random_command_sequences` / `stale_report_never_mutates`）を新 API に移植し、(a) decide = 旧 + apply、(b) replay == execute、(c) seq_nr 単調 / SequenceGap、(d) Quint 不変条件、(e) Err 無副作用、(f) `from_snapshot(snapshot()) == self` を追加。既定 256 ケース・コマンド列 ≤ 60・合成定義（stage_count 2〜8、initialization 1〜3） |
| NFR2.3 | Bolt 着手時に `cargo llvm-cov -p core-domain --summary-only` を 1 回取り、code-summary に基準値を記録（以後の下限） |

## 4. 棚卸し（code-generation で確定し code-summary に記録する事項）

- [ ] I1. ドメインクレート単独カバレッジの着手前基準値（`cargo llvm-cov -p core-domain --summary-only`）。
- [ ] I2. `WorkflowExecution` / `EngineSignal` / `Status` / `PlanAction` の外部利用箇所（実測: ドメイン外の利用は doc コメントのみ、
      `PlanAction` は 10 ファイル）— 実装後に再 grep して差分ゼロを確認。
- [ ] I3. upstream ピン `3c3146cf` の `dist/claude/.claude/tools/data/harness.json` の実バイト（`{ "name": "claude", "harnessDir": ".claude",
      "rulesSubdir": "rules" }`、HTTP 200 実測）をゴールデン dir に追加し README 表に行を足す（バイト不変規則は既存行に適用、追加は可）。
- [ ] I4. `DefinitionRevision` の入力順序と JSON 形（§1）— 同一入力で 2 回 load して一致、`scope-grid.json` を 1 文字変えて不一致、のテスト。
- [ ] I5. `IntentId` の既存有無（`workspace` コンテキストに `IntentSlug` 等があれば再利用し、無ければ新設）。
- [ ] I6. `orchestration/mod.rs` の公開面の最終形（`pub use` 列挙 = entities.md / logical-components の公開面と一致）。

## 5. 実装ステップ（TDD、レイヤーごとに Red → Green → Refactor）

Testing Contract の `plan_profile.steps` を基線とし、ライブラリに存在しない層（Frontend）は省く。「Data model」= Domain Primitive と
値オブジェクト・イベント・スナップショットの型、「Repository」= `WorkflowDefinitionRepository`（C4 改訂）、「Business logic」= 集約の
decide / apply / クエリ、「API」= ファサードと ITF 準拠・実グラフテスト。各 Red では失敗するコマンド出力（失敗テスト名と要約行）を
`code-summary.md` に記録してから Green に進む。

### 5.0 コンダクタ（承認後・委任前）

- [ ] Step 0. Bolt 開始とブランチ: `bun .claude/tools/aidlc-bolt.ts start --name B3 --batch 1` → `git switch -c bolt/b3-u2-domain-es-core origin/main`
      → aidlc 記録を 1 コミット（`chore(aidlc): record U2 design, ADR-008 and the B3 plan`）。基準値 I1 を取得。

### 5.1 workflow_definition 側（開発エージェント — 委任 1）

- [ ] Step 1. 骨格: `plan_action.rs` を `workflow_definition/` へ移動し `workflow_definition/mod.rs` の `pub use` に追加、`orchestration/mod.rs`
      から `mod plan_action` / `pub use plan_action::PlanAction` を削除、呼出側 10 ファイルの `use` を一斉修正。`cargo build --workspace`
      緑、合格 grep = 0。`WorkflowDefinition::effective_plan_action` / `next_in_scope_stage` と依存テストを削除（`grid().action()` /
      `stages_in_scope` への書き換え）。
- [ ] Step 2. テストランナー確認: `cargo test -p core-domain`（実測 126 + ITF 2）、`cargo test -p core-use-case`、
      `cargo test -p core-interface-adapter --test workflow_definition_repository_impl_test --test golden_parity_test` が走ることを確認し
      `unit-test-instructions.md` のコマンドを確定。
- [ ] Step 3. Data model — Red: `WorkflowDefinitionId`（非空・trim・`parse` 往復）、`DefinitionRevision`（`sha256:` + hex64 の形式検証、
      `Display`）、`WorkflowDefinition::new(id, revision, …)` + `id()` / `revision()` のテスト（各 5〜8 本）。失敗出力を記録。
- [ ] Step 4. Data model — Green: 最小実装（private フィールド + アクセサ、`PartialEq` / `Eq` / `Hash` / `Ord`、手実装 Display / Error）。
- [ ] Step 5. Data model — Refactor: rustdoc、`must_use`、ファサード列挙。
- [ ] Step 6. Repository — Red: `core-use-case` の trait を `find_by_id(&WorkflowDefinitionId)` に改訂（`find()` 削除）、
      `GraphReadError::NotFound { expected, actual }` / `HarnessIdentity { path, cause }` を追加。`core-interface-adapter` のテスト:
      (a) `find_by_id(id)` が harness.json の name と一致すれば id / revision 付きの定義を返す、(b) 不一致なら `NotFound`、
      (c) harness.json 欠落 → `HarnessIdentity`、(d) revision は同一入力で安定・入力変更で変わる（I4）、(e) `InMemory…` も同じ契約、
      (f) golden parity test が `find_by_id(WorkflowDefinitionId::parse("claude"))` で実グラフを読む。失敗出力を記録。
- [ ] Step 7. Repository — Green: impl に `load_harness_identity()`（`<data_dir>/harness.json` → `name`）と revision 計算（`canon_json::to_value`
      / `hash_canonical`、依存は既存）、`InMemory…` に id / revision の保持、ゴールデン dir に `harness.json`（I3）。
- [ ] Step 8. Repository — Refactor: 逐語文言の材料（`HarnessIdentity` / `NotFound` の Display は材料のみ）、rustdoc、既存テストの
      `find()` 呼出 13 箇所を `find_by_id` へ。品質ゲート（§5.4 Step 20 と同じ）を一度通してコミット。

### 5.2 orchestration 側 — Data model（開発エージェント — 委任 2）

- [ ] Step 9. Data model — Red: `StageIndex`（範囲保証、`Ord`）、`StageEntry`、`IntentId`（I5）、`WorkflowExecutionEvent`（封筒 + 12 変種の
      構築・アクセサ・`PartialEq`）、`WorkflowExecutionSnapshot`（16 属性）、`NextRequest` / `NextDecision` / `EngineSignal::from`、
      エラー 4 型の `Display` / `Error`（各 5〜8 本）。失敗出力を記録。
- [ ] Step 10. Data model — Green: 最小実装（private + アクセサ、手実装エラー）。`Status` を `status.rs` に切り出し。
- [ ] Step 11. Data model — Refactor: rustdoc、ファサード `pub use` 列挙の更新（旧 API 名は残さない）。

### 5.3 orchestration 側 — Business logic（委任 2 続き）

- [ ] Step 12. Business logic — Red: `start`（W1: 正常 / UnknownScope / InitializationMustExecute / Unconditional、Started の内容、
      definition_id / revision の記録）、12 コマンドのガードと遷移（現行ユニットテスト 9 本を新 API へ移植 + 新規: complete_stage の
      initialization 限定、approve_gate の open 省略経路、reject の revision_count、jump の stages_reset / stages_skipped、recompose 複数件、
      unpark）、`apply_event`（SequenceGap / UnknownStage / 不変条件）、`from_snapshot` の各不変条件、`next_decision`（W4 の優先順 8 分岐 +
      DefinitionMismatch + revision 差で Ok）、`jump_resolve` / `stale_report`。実グラフ索引テスト（initialization 3 ステージの合成 StageEntry
      列で索引 0〜2 非ゲート / 3 ゲート / jump(1) = InvalidTarget）。失敗出力を記録。
- [ ] Step 13. Business logic — Green: 集約本体（decide / apply / クエリ）。現行 FSM のガード集合は維持（`// amadeus-lint: allow(checkbox-vocabulary)`
      + 不変条件番号）。
- [ ] Step 14. Business logic — Refactor: apply の一時コピー方式の整理、重複ガードの関数化、rustdoc。テスト緑のまま。
- [ ] Step 15. PBT（`workflow_execution.rs` 同居、`PROPTEST_RNG_SEED` 固定）: 性質 (a)〜(f)（§3 NFR2.2）。

### 5.4 orchestration 側 — API（ファサード・ITF・品質ゲート）

- [ ] Step 16. API — Red: `engine_loop_conformance.rs` を新 API に書き換え（BR2.5 の対応表）— 8 fixture 全緑になるまで Red。
      `orchestration/mod.rs` の公開面がロジカル設計の列挙と一致することのテスト（`pub use` 行の読取 — canon-json と同じ方式）。
- [ ] Step 17. API — Green: 不足のアクセサ / 変換（`EngineSignal::from`）、ITF 用 `start_with_entries`。
- [ ] Step 18. API — Refactor: クレート rustdoc（`//!`）に ES 形・イベント 12 変種・射影表・gated = phase の説明、BR2.5 の注記。
- [ ] Step 19. 棚卸し I2 / I6 と `cargo llvm-cov -p core-domain --summary-only`（I1 の基準値と比較）。
- [ ] Step 20. 品質ゲート: `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` →
      `cargo test --workspace` → `bash scripts/coverage.sh`（絶対床）→ 合格 grep（BR4.1）= 0。コミットは意味単位
      （`feat(core-domain): …` / `refactor(workflow-definition): …` / `test(itf): …`）。

## 6. トレーサビリティ（要求 → ステップ）

| 要求 / 規則 | ステップ | 主な成果物 |
|---|---|---|
| FR8.3 PlanAction 完全移動 | 1, 19, 20 | `workflow_definition/plan_action.rs`、両 mod.rs、呼出側 10 ファイル |
| FR8.4 畳み込み移設 | 1, 12〜14 | `workflow_definition.rs`（削除）、`workflow_execution.rs`（effective_plan） |
| FR2.1 / FR3.1 / FR3.3 の土台（decide / next_decision） | 9〜18 | `orchestration/*.rs` |
| FR1.3 の集約側（snapshot / replay） | 9, 12〜15 | `workflow_execution_snapshot.rs`、`apply_event` |
| BR1.0〜BR1.9 / BR3.1〜BR3.3 | 12〜14 | `workflow_execution.rs` |
| BR2.1〜BR2.4 | 9, 12〜15 | `workflow_execution_event.rs`、`apply_event`、PBT |
| BR2.5 | 16, 17 | `tests/engine_loop_conformance.rs` |
| BR2.6 / ADR-008 / C4 | 3〜8, 12 | `workflow_definition_id.rs`、`definition_revision.rs`、`workflow_definition_repository.rs`（trait）、`workflow_definition_repository_impl.rs`、`memory/workflow_definition_repository.rs`、`tests/golden/upstream-3c3146cf/harness.json` |
| BR4.1 / BR4.2 | 1 | 同上 |
| BR5.1〜BR5.4 | 9〜11 | `stage_index.rs`、`workflow_execution_snapshot.rs`、エラー 4 型 |
| NFR1.1 / NFR1.2 / NFR1.3 | 12, 16, 17 | ITF、実グラフ索引テスト、網羅 match |
| NFR2.1〜NFR2.4 | 全 Red/Green/Refactor、15, 19, 20 | Red 記録、PBT、カバレッジ基準値、品質ゲート |
| NFR3.1〜NFR3.4 | 12〜15 | apply / from_snapshot / next_decision / snapshot |
| NFR4.1〜NFR4.5 | 1, 4, 7, 20 | 依存不変（core-domain）、StageIndex、素通し、serde なし |

## 7. 委任の形

- 委任 1（Step 1〜8: workflow_definition / Repository 側）と委任 2（Step 9〜20: orchestration 側）を同じ承認済み計画・同じ指紋の下で
  **直列に** aidlc-developer-agent へ委任する。各委任の冒頭行は `AIDLC-UNIT: u2-domain-es-core` と `AIDLC-TESTING-CONTRACT: <contract_sha256>`。
  委任 1 の終わりでワークスペースが緑（ビルド・テスト・lint）であること。
- 開発エージェントは計画のチェックボックスを更新しない（計画バイトは承認後凍結 — 進捗はエージェントの報告ファイル
  `developer-report-<n>.md` に書き、コンダクタが `code-summary.md` に統合する）。
- 失敗時はコンダクタが halt-and-ask（retry / skip / abort）を出す。規模 L: 委任 1 が 1 日相当を超えそうならオーナーと分割を相談。

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

## 承認済みテスト指示（unit-test-instructions.md 全文 — 読み取り専用）

# unit-test-instructions — U2 ドメイン ES コア（`u2-domain-es-core`）

> Code Generation（Construction 3.5）のユニットテスト指示（Unit: U2、Bolt: B3）。Testing Contract: tdd / standard / classic / brownfield
> （`code-generation-plan.md` の `## Testing Contract`）。方針の正本は `aidlc/spaces/default/memory/team.md` Testing Posture。

## 1. テストフレームワークと設定

- Rust 標準テストハーネス（`cargo test`）+ proptest 1.11（PBT、`core-domain` の dev-dependency — 既存）+ serde_json（dev、ITF の JSON 読取）。
  新規依存なし。
- PBT のシードは固定: `PROPTEST_RNG_SEED=20260823`（`scripts/coverage.sh` / CI と同じ値。proptest 1.11 の `RngSeed::Fixed`）。
- lint: `cargo clippy --workspace --all-targets -- -D warnings`（テストコードは `clippy.toml` で `unwrap` / `expect` 許可）、`cargo lint`。
- テストコードでは `unwrap` / `expect` を使ってよい（統合テストは file-level `#![allow(clippy::unwrap_used)]` — 既存どおり）。

## 2. 本 Unit のテストの走らせ方（Unit スコープのコマンド）

| 対象 | コマンド |
|---|---|
| ドメイン（ユニット + PBT） | `PROPTEST_RNG_SEED=20260823 cargo test -p core-domain --lib` |
| ITF 準拠（engine_loop） | `cargo test -p core-domain --test engine_loop_conformance` |
| Repository ポート（use-case） | `cargo test -p core-use-case` |
| Repository 実装・ゴールデン（interface-adapter、本 Unit が触るテストのみ） | `cargo test -p core-interface-adapter --test workflow_definition_repository_impl_test --test golden_parity_test` + `cargo test -p core-interface-adapter --lib orchestration::` |
| 合格 grep（FR8.3） | `grep -rnE 'enum PlanAction\|pub use .*PlanAction' modules/core/domain/src/orchestration` → 0 件で合格 |
| カバレッジ（ドメイン単独、基準値の記録） | `cargo llvm-cov -p core-domain --summary-only` |

最初の TDD Red の前に、brownfield の実測で上のコマンドが走ること（2026-08-23 実測: `core-domain` 126 + ITF 2 テスト緑）を確認する
（Step 2）。Build and Test は各 Unit のコマンドを実行するため、ワークスペース全体の `cargo test --workspace` は品質ゲート（Step 20）
でのみ使う。

## 3. テスト範囲と量（standard: コンポーネントごと 5〜8 本）

| コンポーネント | テスト（代表） |
|---|---|
| `WorkflowDefinitionId` / `DefinitionRevision` | parse 往復、空・不正形の拒否、`sha256:` 形式検証、Display、Eq / Ord |
| `WorkflowDefinition`（id / revision） | `new` + アクセサ、`stages_in_scope` の PhaseId、`effective_plan_action` / `next_in_scope_stage` の不在（コンパイルで担保） |
| `WorkflowDefinitionRepository`（Impl / InMemory） | `find_by_id` 一致で Ok、不一致で `NotFound`、harness.json 欠落で `HarnessIdentity`、revision の安定性と変化、golden parity が `claude` で読める |
| `StageIndex` / `StageEntry` / `IntentId` | 範囲外 → None、Ord、parse |
| `WorkflowExecutionEvent` | 封筒（seq_nr / schema_version = 1 / occurred_at）、12 変種のペイロードアクセサ、Eq |
| `WorkflowExecutionSnapshot` | 16 属性のアクセサ、`from_snapshot(snapshot()) == self`、不変条件違反の各 Err |
| `WorkflowExecution`（decide） | 現行 9 本の移植 + complete_stage の initialization 限定 / approve_gate 省略経路 / reject の revision_count / jump の差分集合 / recompose 複数件 / unpark / Err 無副作用 |
| `WorkflowExecution`（apply / クエリ） | SequenceGap / UnknownStage / InvariantViolation、`next_decision` 8 分岐 + DefinitionMismatch + revision 差、`jump_resolve`、`stale_report` |
| 実グラフ索引 | initialization 3 ステージの合成 StageEntry 列で索引 0〜2 非ゲート / 3 ゲート / jump(1) = InvalidTarget |
| PBT（6 性質） | (a) decide = 旧 + apply、(b) replay == execute、(c) seq_nr 単調 / SequenceGap、(d) Quint 不変条件、(e) Err 無副作用、(f) snapshot 往復 |
| ITF 準拠 | 8 fixture 全緑 + アクション網羅アサート（既存）を新 API で維持 |

## 4. カバレッジ目標

- ワークスペース絶対床 90%（`scripts/coverage.sh`）。ドメインクレート単独は Step 0 の基準値（`cargo llvm-cov -p core-domain --summary-only`）
  を下回らない。除外は `main.rs` のみ（U2 のコードに除外を足さない）。

## 5. モック / スタブ

- ドメインは I/O を持たないためモック不要。Repository のテストは tempdir（既存フィクスチャ）に 3 入力 + `harness.json` を書いて実ファイルで
  検証、`InMemoryWorkflowDefinitionRepository` はテストダブル（`Impl` 接尾辞を付けない）。
- ITF 準拠テストは合成 `WorkflowDefinitionId("itf")` / `DefinitionRevision("sha256:" + "0"×64)` と Quint の plan / conditional から合成した
  `StageEntry` 列（索引 0 = initialization）で集約を作る（`start_with_entries`）。

## 6. テストデータ

- Quint トレース fixture: `tests/conformance/fixtures/engine_loop/*.itf.json`（8 本、不変）。
- 実グラフ: `tests/golden/upstream-3c3146cf/{stage-graph,scope-grid,harness}.json`（harness.json は本 Bolt で追加 — upstream 実バイト）。
- 各テストは自前でデータを組み立て、共有の可変状態を持たない。
