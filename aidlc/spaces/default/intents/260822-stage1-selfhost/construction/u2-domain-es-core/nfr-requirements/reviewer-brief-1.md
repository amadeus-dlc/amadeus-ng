# reviewer-brief-1 — U2 NFR Requirements の独立レビュー（2026-09-07、advisory、iteration 1）

Conversation language: 日本語（コード識別子・固定トークン・YAML キーは英語のまま）

## A. 依頼の条件

- **役割**: `aidlc-architecture-reviewer-agent`（独立レビュー担当）。ステージ `nfr-requirements`、Unit `u2-domain-es-core`
  （ドメインのイベントソーシング中核 — `core-command-domain` の Intent / IntentExecution 集約）。レビュー種別 **advisory**、
  iteration **1**（この 1 回のみ。再レビューはしない）。
- **レビュー対象（`review_artifact`）**: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/nfr-requirements/security-requirements.md`。
  この末尾に **`## Review` 節を厳密に 1 つだけ追記**する。節は次を各 1 回だけ含む: `**Verdict:** READY` または
  `**Verdict:** NOT-READY`、`**Reviewer:** aidlc-architecture-reviewer-agent`、`**Iteration:** 1`。節内の小見出しは H3 以下（`###`）のみ。
  `## Review` より前のバイトは 1 文字も変えない。他のファイルは一切書き換えない・新規作成しない。
- **読む範囲（read scope）**: 下記の必読ファイルと、リポジトリの実コード・形式モデル・コーディング規則・CI 設定のみ。
  **他 Unit の `construction/<other-unit>/` 配下は読まない**（glob / grep / ls でまたがるのも禁止）。intent 記録直下の `ls` / glob もしない。
- **必読ファイル**（順に読む）:
  1. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/nfr-requirements/nfr-requirements-questions.md`
     （2026-08-23 の前提・まとめは履歴。**2026-09-07 再走の P7〜P12 と `## Consolidated Summary Confirmation` が今回の確認事項**）
  2. `.../u2-domain-es-core/nfr-requirements/security-requirements.md`（レビュー対象。NFR1.1〜NFR4.5、新規 NFR2.5）
  3. `.../u2-domain-es-core/nfr-requirements/tech-stack-decisions.md`
  4. `.../u2-domain-es-core/nfr-requirements/traceability.json`
  5. `.../u2-domain-es-core/nfr-requirements/security-requirements-review-history-2026-08-23.md`（旧世代のレビュー、履歴。今回の判定に流用しない）
  6. `.../u2-domain-es-core/nfr-requirements/validation-20260907.md`（本再走の実測記録: ドメインクレート単独カバレッジ、センサー結果）
- **上流（consumes）**: `.../u2-domain-es-core/functional-design/functional-spec.md`（§2 API、§9 引継ぎ、末尾の 2026-09-06 レビュー節
  R-01〜R-10 — advisory NOT-READY、成果物は凍結中）、`.../u2-domain-es-core/functional-design/rules.md`（BR1.0〜BR5.5）、
  `.../u2-domain-es-core/functional-design/entities.md`、`aidlc/spaces/default/intents/260822-stage1-selfhost/inception/requirements-analysis/requirements.md`
  （NFR1〜NFR5）、`aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md`（C3 / C5 / C6）。
- **照合する実コード・設定**: `modules/core/command/domain/Cargo.toml`（依存ベースライン）、`modules/core/command/domain/src/orchestration/`
  （`intent.rs` / `intent_execution.rs` / `intent_execution_event.rs` / `*_event_id.rs`）、`modules/core/command/domain/tests/`
  （`engine_loop_conformance.rs` / `collection_contract_test.rs`）、`modules/core/infrastructure/src/collections/`、
  `formal/orchestration/engine_loop.qnt`（ヘッダの版）、`Cargo.toml`（`[workspace.lints]` の件数）、`rust-toolchain.toml`、
  `scripts/coverage.sh`（床・許容差・シード・除外式）、`.github/workflows/ci.yml`、
  `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README.md、first-class-collections.md、aggregate-commands.md、
  aggregate-references.md、domain-persistence-neutrality.md、error-handling.md、tell-dont-ask.md）、
  `aidlc/spaces/default/memory/team.md`（Testing Posture / Code Style）、`aidlc/spaces/default/codekb/docs/technology-stack.md`。
- **今回の再走の趣旨**: 2026-08-23 の旧世代 NFR（`WorkflowExecution`・12 変種・snapshot 値オブジェクト・「panic しない」・
  `DefinitionMismatch`・`core-domain` 配置・lints 48）を、後続裁定（Intent / IntentExecution 分離、16 変種、最新スナップショット + 差分再生、
  壊れた歴史は panic、`IntentMismatch`、BR5.5 FCC 化）と現行コード・現行 CI（U10 実測）へ同期し、新規 NFR2.5（FCC の契約試験・
  Monoid 則）を追加した。
- **見るべき観点**（advisory。判定閾値はあなたのペルソナに従う）:
  1. 各 NFR の合格基準が**検証可能**か（コマンド・テスト名・数値・レビュー観点が具体か）。曖昧語（「十分」「適切」）が無いか。
  2. 実測との一致: 依存ベースライン（Cargo.toml）、lints 件数（rust / rustdoc / clippy）、toolchain 版、coverage.sh の床・許容差・シード・除外式、
     ITF テストと契約試験ハーネスの実在、Quint モデル版、`std::time` / 乱数の利用箇所（`*EventId::generate` 以外に無いか）、
     `next_decision` の現行署名（Result ではない — code-generation で同期、と書かれているか）。
  3. 機能設計との整合: NFR の記述が rules.md / entities.md / functional-spec.md（BR1.1 事後条件、BR2.1 / BR2.3 / BR2.6 / BR5.2 / BR5.5、
     §9 引継ぎ、R-01〜R-03）と矛盾しないか。凍結中の機能設計に対する所見（R-01〜R-03）を NFR がどう引き継いでいるか。
  4. コーディング規則との適合: domain-persistence-neutrality、aggregate-commands（再生の形、panic の射程）、first-class-collections
     （Monoid 則・差集合則・機械的追加の禁止）、error-handling。
  5. 上流との整合: requirements.md の NFR1〜NFR5 が traceability.json で過不足なく被覆され、各 OK target の枝番が要求表に存在するか。
     contract-summary の古い記述（全再生）を黙って読み替えていないか。
  6. STRIDE・データ分類がライブラリ規模として妥当か（過剰・過少）。
  7. センサー実測: `bun .claude/tools/aidlc-sensor-required-sections.ts --stage nfr-requirements --output-path <各 md>`（2 本）、
     `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-requirements --output-path <traceability.json>`。
     必要なら読み取り専用の確認として `cargo test --locked -p core-command-domain --test collection_contract_test` 等。
- **書き方**: `## Review` 節は日本語。所見表は `| ID | Severity | Location | Finding | Required action |`（Severity は
  Critical / Major / Minor / Info）。`### Findings` / `### Validation Tool Results` / `### Summary` の 3 小節。
  所見は現行本文に対して出す。根拠はファイルパスと該当箇所で示す。
  レビュー本文以外の出力（最終報告）は簡潔に: Verdict、所見の件数（重大度別）、主要所見 3 件以内。

## B. 規則束（org → team → project → phases/construction、逐語）

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

