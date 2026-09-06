# developer-brief-4 — U10 CI・品質管理の実装記録の是正（2026-09-06）

AIDLC-UNIT: u10-ci-governance
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3
Conversation language: 日本語

> 委任 4（Code Generation、Unit: u10-ci-governance、kind: packaging）。承認済み計画（承認指紋
> `sha256:73fb6047d771f21ad6fa75a7cb9179c25d20dd34e637e9e3e0a03a60a4defe45`、PLAN_APPROVAL_RECORDED 2026-09-06）の
> Step 1〜6 を実行する。本ファイルは §A（委任の条件）→ §B（規則束、逐語）→ §C（承認済み計画、逐語）→ §D（承認済みテスト手順、逐語）
> → §E（改訂済み要件・設計、逐語）の順に連結してある。**全文を読んでから着手すること。**

## A. 委任の条件

### A.1 スコープ

- ワークスペースの CI・品質設定は **変更しない**。読取と照合、Unit 限定コマンドの実行、受入の実測のみ。
- 更新するのは記録ディレクトリ `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/code-generation/` の
  次の 3 ファイルと、作業完了後の計画のチェックボックス、最終報告 `developer-report-4.md` だけ:
  - `code-summary.md`（現行の事実で全面的に書き直す。旧版は `code-summary-history-2026-08-23.md` に保存済み — 旧版を編集しない）
  - `traceability.json`（15 件、target はワークスペース相対パス単体）
  - `source-manifest.json`（新規、strict schema、`writes` は空配列）
- 変更しない: `.github/workflows/*`、`scripts/**`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`Cargo.lock`、
  `tools/lint/Cargo.lock`、`modules/**`、他 Unit の記録、`../nfr-requirements/`・`../nfr-design/`（凍結済み）、
  `superseding-decisions.md`・`pending-revision.md`・`developer-brief-3.md`・`developer-report-3.md`・`ruleset/`・履歴ファイル。
- GitHub への書込（ruleset 変更、PR、コメント、push）は行わない。`scripts/governance/ruleset-required-checks.sh` は `--dry-run` を含め実行しない。
- commit は行わない（親セッションが作業ツリー全体を回収する）。

### A.2 作業順序（承認ガードとの相互作用）

計画 `code-generation-plan.md` のバイト列は承認指紋の対象である。チェックボックスを途中で `[x]` にするとワークスペース側の編集
（Step 3(c) の一時変更）が承認不足として拒否される。したがって **Step 1〜3（ワークスペースに触る作業）を先に完了し、次に Step 4〜5
（記録の更新）を行い、最後に Step 6 で計画の Step 1〜6 のチェックボックスをまとめて `[x]` にする**。計画本文はチェックボックス以外
変更しない。

### A.3 受入基準

1. Unit 限定コマンド（§D §2）がすべて成功し、件数・結果・完了日時（UTC）が `code-summary.md` に記録されている。期待値の書き換えなし。
2. 受入の実測（§D §3）: `bash scripts/coverage.sh` 2 回の生の head 値と差、`cargo audit` 2 件の結果（未導入・取得失敗は成功と書かない）、
   unsafe 不適合例の拒否（workspace メンバー 1 クレートと `tools/lint`）、`rustc -V` が記録されている。差 0.00 ポイント未達なら未達のまま
   原因を記録し、`TOLERANCE`・除外・シードを変えない。
3. `code-summary.md` が現行の事実（7 ジョブとイベント別の CI Success 集約条件、review-thread-resolution のジョブ別権限 5 種と外部呼出先・
   `ci_ref` の SHA 一致、必須 4 コンテキスト・strict・bypass なし・SQUASH/ALLGREEN/同時 1 件、絶対床 90%・相対許容差 0.01・シード
   20260823・除外 `main.rs` のみ、Rust 1.95.0 と `toolchain-inputs.sh` の導出、`unsafe_code = "forbid"` の継承、検査 20 項目）を
   記述し、今回の実測・未検証範囲（全 CI 実行、キューの成功/失敗両経路の実働、レビュー再評価の反映、外部再利用ワークフロー内部）・
   過去の裁定（暫定 0.05、残差 0.0175pp、除外 regex の訂正、ruleset 適用、PR #25/#26）を区別している。過去の事実を今回の実施と書かない。
4. `traceability.json` の 15 件（FR9.1〜9.5、NFR2.1〜2.5、NFR4.1〜4.5）がすべて実在ファイルのパス単体を target とし、
   `bun .claude/tools/aidlc-sensor-traceability.ts --stage code-generation --output-path <traceability.json のパス>` で
   `invalid_targets` が 0（`missing_from_upstream_ids` は他 Unit の要求 ID で既知のノイズ。センサー全体の `pass` は false のままでよい）。
   `bun .claude/tools/aidlc-sensor-required-sections.ts --stage code-generation --output-path <code-summary.md のパス>` で `pass: true`。status は実装済みで検証できたものを `OK` とし、実測で未達が残る要求は `Partial` と
   その理由を code-summary へ書く（target は同じくパス単体）。
5. `source-manifest.json` が `{"stage":"code-generation","unit":"u10-ci-governance","version":1,"writes":[]}` の形。
6. 終了時 `git status --short` にワークスペース側（`aidlc/` 配下以外）の差分が **1 件もない**。記録側の差分は上記ファイルに限る。
7. `developer-report-4.md` に §A.5 の形式で最終報告を書く。

### A.4 検証手順（親セッションが行う。同じ手順で自己確認すること）

- `git status --short` / `git diff --stat` でワークスペース側の差分ゼロを確認。
- `bash scripts/governance/verify-ci-governance.sh --with-ruleset` を再実行して 20 項目成功。
- `bun .claude/tools/aidlc-sensor-traceability.ts` で `invalid_targets` 0、`bun .claude/tools/aidlc-sensor-required-sections.ts` で
  `code-summary.md` の H2 が 2 つ以上。
- `code-summary.md` の数値（件数・カバレッジ値・日時）が実行ログと一致。

### A.5 最終報告の形式（`developer-report-4.md`）

1. 実行した Step と結果（Step ごと）
2. Unit 限定コマンドの結果表（コマンド / 件数 / 終了コード / 完了日時 UTC / ログの保存先）
3. 受入の実測（coverage 2 回の生の値と差、audit 2 件、unsafe 拒否の出力要約、rustc の版）
4. 要件・設計との照合で見つかった不一致（なければ「なし」。あれば対象・再現手順・提案する検出項目 — 修正はしない）
5. 更新したファイルと、変更しなかったファイル
6. 未検証範囲と親セッションへの引き継ぎ

### A.6 実行上の注意

- 長時間コマンド（`scripts/coverage.sh` は数分〜10 分程度）は Bash の `timeout` を最大（600000 ms）にするか `run_in_background` で実行し、
  出力をスクラッチパッド `/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-stage1-selfhost/bdae4b2f-d1d9-470f-bf7c-df8853392e07/scratchpad/`
  のログファイルへ保存する（`/tmp` 直下は使わない）。ログのパスと更新時刻を code-summary に残す。
- `cargo audit` が未導入なら `cargo audit --version` の失敗出力を記録し、インストールはしない（未実行として記録）。
- unsafe 不適合例の確認は、対象クレートの `src/lib.rs` 末尾に `unsafe fn __aidlc_forbid_probe() {}` のような 1 行を追加して
  `cargo check -p <crate>` を実行し、`error: usage of an unsafe block/function` 相当の拒否（`forbid(unsafe_code)`）を記録したうえで、
  **必ず `git checkout -- <file>` で戻し、`git status --short` で差分ゼロを確認する**。`tools/lint` は `cargo check --manifest-path
  tools/lint/Cargo.toml`。
- 品質目標（90% 床、0.01、シード、除外、期待値）は入力であり提案ではない。緩和・無効化・書き換えをしない。未達は未達として報告する。
- `code-summary.md` の H2 見出しは「## 1. …」のように番号付きで 2 つ以上。末尾に `## Review` 節を **書かない**（レビュアーが追記する）。
- プロダクトコード・テストコードの新規作成はない。人工的な Red を作らない。
- 途中で判断に迷う事項（例: 差 0.00 未達、audit 未導入、設定の不一致）は止まらずに事実を記録して進め、最終報告で明示する。

### A.7 参照する上流成果物（必要なら Read）

- 要件: `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/requirements-analysis/requirements.md` — FR9.1〜9.5（CI ガバナンス:
  ruleset の必須チェック、サプライチェーン、tools/lint の CI、PBT シード固定と相対差、カバレッジ除外）、NFR2（品質ゲート）、NFR4（セキュリティ）。
- Unit 定義: `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/units-generation/unit-of-work.md` — U10 は packaging、
  責務 FR9.1〜9.5、合格「CI 緑・audit clean・required checks 確認」。
- 契約: `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md` — U10 は製品の外部契約を持たない。
- 改訂基準: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/revision-baseline-20260906.md`（§E に逐語）。
- ruleset 観測: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/ruleset-observed-20260906.json`。
- 過去の裁定（履歴）: `.../code-generation/superseding-decisions.md`、`.../code-generation/code-summary-history-2026-08-23.md`、
  `.../code-generation/ruleset/{before,after}.json`、`.../code-generation/ruleset/2026-08-23-ci-success/{before,after}.json`。
- コーディング規則の正本: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README がインデックス）。今回コードは書かないが、
  規則に反する記述を記録へ持ち込まない。
- 検査対象の実ファイル: `.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、`scripts/coverage.sh`、
  `rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/verify-ci-governance.sh`、
  `scripts/governance/ruleset-required-checks.sh`、`scripts/governance/toolchain-inputs.sh`、各 workspace メンバーの `Cargo.toml`。

### A.8 プロジェクトの前提

- Rust ワークスペース（Rust 1.95.0、`rust-toolchain.toml` 固定）。CI は GitHub Actions（`ci.yml` 7 ジョブ）。bash 3.2（macOS 標準）互換の
  スクリプト。`jq`・`gh`（読取のみ）・`cargo-llvm-cov` を使用。
- 会話・成果物は日本語。コード識別子・ファイルパス・固定トークン（`OK` / `Partial` / `N/A`、JSON キー）は英語のまま。
- 本セッションの実行環境は git worktree `stage1-selfhost` ブランチ。bare `git stash` / `git stash pop` は禁止。


## B. 規則束（org → team → project → phases/construction、逐語）


<!-- rules: aidlc/spaces/default/memory/org.md -->

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


<!-- rules: aidlc/spaces/default/memory/team.md -->

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


<!-- rules: aidlc/spaces/default/memory/project.md -->

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


<!-- rules: aidlc/spaces/default/memory/phases/construction.md -->

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


## C. 承認済み計画（code-generation-plan.md、逐語）

# code-generation-plan — U10 CI・品質管理の実装記録の是正

> Unit: u10-ci-governance（kind: packaging）。2026-09-06の再確認計画。
> 出典: `../nfr-requirements/security-requirements.md`・`tech-stack-decisions.md`（2026-09-06改訂、READY）、
> `../nfr-design/security-design.md`（2026-09-06改訂、READY）、`../revision-baseline-20260906.md`、
> `../ruleset-observed-20260906.json`、`../../../inception/requirements-analysis/requirements.md`（FR9.1〜9.5、NFR2、NFR4）、
> `../../../inception/units-generation/unit-of-work.md`（U10の責務・境界・合格）、`code-generation-questions.md`。
> 2026-08-22に承認した旧計画は `code-generation-plan-history-2026-08-22.md` に全文保存した。`superseding-decisions.md` が
> 「本計画」と呼ぶのはその履歴ファイルである。

## 1. 目的と変更範囲

CI・品質管理の実装（`.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、`scripts/coverage.sh`、
`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/`）はBolt B2（PR #25・#26）でmainへ
反映済みであり、2026-09-06改訂の要件・設計と一致することを検証スクリプトで確認済み（`verify-ci-governance.sh --with-ruleset` 20項目成功）。
一方、実装記録（`code-summary.md`・`traceability.json`）は2026-08-23の回復レビュー（NOT-READY: Critical 1・Major 2）以降凍結され、
review-threadゲート・必須チェック4件化・許容差0.01への引き締めが反映されておらず、traceabilityのtargetに説明文が混在している。

今回はワークスペースのファイルを変更しない。行うのは次の3点である。

1. 現行設定を改訂済み要件・設計へ照合し、Unit限定コマンドと受入の実測（カバレッジ2回測定・依存監査・unsafe不適合例の拒否）を実行して記録する。
2. `code-summary.md` を現行の事実で書き直し、旧版を履歴として保存する。
3. `traceability.json` の全15件を現行の実在ファイルへ対応付け、targetをパス単体にする。`source-manifest.json` を作る。

変更しないもの: CI定義・スクリプト・品質閾値（絶対床90%、相対許容差0.01ポイント、シード20260823、除外は `main.rs` の1ファイル）・
ruleset・依存・ツールチェーン・プロダクトコード。GitHubへの書込（ruleset変更、PR作成、コメント投稿）は行わない。
FR9.6（エラー様式規則の正本化）はU9の責務であり扱わない。

実装と要件・設計の不一致が新たに見つかった場合は、対象・再現手順・検査スクリプトへ追加する検出項目（Red）を含む変更案を報告し、
計画の変更を受けてから扱う。本計画を根拠に他Unitや凍結済みの要件・設計成果物まで変更しない。上流要件のR-01（Markdown表2行の
表示崩れ、Minor）は上流の所見として残し、本計画で解消扱いにしない。

## 2. 所有するファイルと保持する成果

| 区分 | 対象 | 扱い |
|---|---|---|
| ワークスペース設定（読取のみ） | `.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/{verify-ci-governance,ruleset-required-checks,toolchain-inputs}.sh` | 検証と照合のみ。差分を残さない（unsafe不適合例の確認で一時変更する場合も終了時に必ず戻す） |
| Unit記録（更新） | `code-summary.md`、`traceability.json`、`source-manifest.json` | 現行の事実で書く。旧 `code-summary.md` は `code-summary-history-2026-08-23.md` に全文保存済み |
| 計画と試験手順 | 本ファイル、`unit-test-instructions.md` | この計画承認の対象。完了チェック以外の変更が必要なら承認を更新。旧版は `*-history-2026-08-22.md` |
| 履歴（変更しない） | `superseding-decisions.md`、`pending-revision.md`、`developer-brief-3.md`、`developer-report-3.md`、`ruleset/` 配下 | 過去の裁定・ブリーフ・前後JSONの記録としてそのまま保持 |

過去のTDD証跡（2026-08-22のRed 1/14 → Green 15/0）、暫定許容差0.05、残差0.0175ポイントは歴史であり、今回の実施や現在の設定として
記載しない。今回変更しない既存ファイルはcode-summaryの照合欄で示し、変更済みと偽らない。source-manifestには実際に作成・変更・削除した
アプリケーション側パスだけを列挙する（今回の予定は空）。

## 3. 実行ステップ

- [ ] Step 1. ランナーと設定を確認する。`unit-test-instructions.md` のUnit限定コマンド（`bash -n` の個別実行、`verify-ci-governance.sh` の
      既定と `--with-ruleset`、`toolchain-inputs.sh`、`tools/lint` の自己テスト）を実行し、件数・結果・完了日時を記録する。
      `rustc -V` と `cargo llvm-cov --version`、`cargo audit --version` の有無も記録する。
- [ ] Step 2. 現行設定を要件FR9.1〜9.5・NFR2.1〜2.5・NFR4.1〜4.5と設計§2〜§5へ対応付ける。7ジョブとイベント別の集約条件、
      review-thread-resolutionのジョブ別権限とSHA固定（呼出先・ci_refの一致）、必須4コンテキスト・strict・bypassなし・キュー設定
      （`ruleset-observed-20260906.json`）、閾値・シード・除外式、toolchainの導出、`unsafe_code = "forbid"` の継承（全workspaceメンバーの
      `lints.workspace = true` と `tools/lint` の個別宣言）を確認する。
- [ ] Step 3. 受入を実測する。(a) `bash scripts/coverage.sh` を同一リビジョン・同一ツールチェーン・同一シードで2回実行し、生のhead値と
      差を記録する（差0.00ポイント未達なら未達のまま原因を記録し、閾値を変えない）。(b) `cargo audit` と
      `cargo audit --file tools/lint/Cargo.lock` を実行し、結果・走査crate数・advisory DBの取得可否を記録する（未導入なら未実行として記録し
      成功と書かない）。(c) workspaceメンバー1クレートと `tools/lint` に `unsafe` を含む一時変更を加えて `cargo check` が拒否することを
      確認し、直後に変更を戻して `git status` で差分がないことを確認する。
- [ ] Step 4. `code-summary.md` を現行の事実で書き直す。実装済みファイル一覧（review-thread-resolution.ymlを含む）、7ジョブとCI Successの
      集約条件、4コンテキスト、ジョブ別権限、SHA固定、閾値0.01、検査20項目、今回の実測、未検証範囲（全CI実行・キューの成功/失敗両経路の
      実働・外部再利用ワークフロー内部）を区別して記す。過去の裁定（暫定0.05、除外regexの訂正、ruleset適用、PR #25/#26）は履歴として
      `superseding-decisions.md` と履歴ファイルを参照する。
- [ ] Step 5. `traceability.json` を更新する。15件のIDを現行の実在ファイルへ対応付け、targetはワークスペース相対パス単体にし、日時・参照JSON
      等の注記はcode-summary側へ移す。`bun .claude/tools/aidlc-sensor-traceability.ts` で `invalid_targets` が0であることを確認する
      （`missing_from_upstream_ids` は他Unitの要求IDで、既知のノイズ）。`source-manifest.json` を strict schema で作る（`writes` は空配列）。
- [ ] Step 6. `git status` でワークスペース側に差分がないこと、記録側の変更が本ディレクトリに限られることを確認し、独立レビューへ引き渡す。
      親セッションがレビュー・Unit完了・次工程・commit・pushを処理する。

## 4. Testing Contractの適用

本Unitはpackagingで、プロダクトコードの層を持たない。今回は設定の再検証と記録の是正であり、新規プロダクションコード・新規テストはない。
DB・Repository・業務判断・HTTP API・フロントエンドの実装用ステップは架空に実行しない。

埋め込み契約のTDD方針は維持する。2026-08-22の実装では「設定の事実を機械検査する `verify-ci-governance.sh` を先に書き、現状でRed →
設定変更でGreen」と写した（履歴ファイル§3）。今後、設定の振る舞いを変更する場合は同スクリプトへ検出項目を先に追加して失敗出力を
記録してから設定を変え、成功中に整理する。既存成功ログから過去のRedを推定しない。

Standard戦略の「コンポーネントごと5〜8本」は、検査スクリプトの20項目（対象ファイルごとに2〜7項目）と `tools/lint` の既存自己テスト
（件数は実行時の結果を記録し31本に固定しない）で満たしている。既存スイートは緑のまま維持する。必須CI、カバレッジ90%床、相対差0.01ポイント、
固定シード20260823を維持する。Unit限定コマンドの成功を全CI・キュー完走・外部ワークフロー内部の検証の成功に読み替えない。

## 5. 要求からステップへの対応

| 要求 | Step | 確認対象 |
|---|---|---|
| FR9.1、NFR2.1、NFR4.5 | 1・2・4・5 | ruleset観測JSON（4コンテキスト・strict・bypassなし・SQUASH/ALLGREEN/同時1件）、`ruleset-required-checks.sh` の比較・保存・送信項目、前後JSONの記録 |
| FR9.2、NFR4.1・NFR4.2・NFR4.3 | 1〜5 | `rust-toolchain.toml` と `toolchain-inputs.sh` の導出、`rustc -V`、`cargo audit` ×2、`unsafe_code = "forbid"` の継承と不適合例の拒否、`permissions: contents: read` |
| FR9.3、NFR2.3 | 1・2・4 | `check` ジョブのworkspace 4ステップと `tools/lint` 3ステップ、`tools/lint` 自己テストの件数 |
| FR9.4、NFR2.4 | 1〜5 | `TOLERANCE=0.01`、`PROPTEST_RNG_SEED=20260823` の宣言（CIとローカル）、同一条件2回測定の生の値と差 |
| FR9.5、NFR2.5 | 1〜4 | 除外式が `main.rs` 1ファイルのみ、絶対ゲート90%の結果 |
| NFR2.2 | 2・4 | 7ジョブ、イベント別のCI Success集約条件（pull_requestではreview-thread success必須、merge_group/workflow_dispatchではskipped受理）、coverageの比較条件、`audit` の集約外 |
| NFR4.4 | 2・4 | workflow既定 `contents: read`、review-thread-resolutionの個別権限5種、外部呼出先とci_refのSHA一致、トークン非出力 |

## 6. 作業の進め方

計画承認後、開発担当が§2の範囲で実行する。ワークスペース設定は読取と一時的な不適合例の確認に限り、終了時に差分を残さない。
他者の変更を戻さず、commit・push・GitHubへの書込・外部投稿は親セッションに任せる。旧Boltブランチの作成・ruleset適用・PR作成の手順は
再実行しない。親セッションは全差分と検証結果を確認し、監査を含む作業ツリー全体を回収する。

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

## D. 承認済みテスト手順（unit-test-instructions.md、逐語）

# unit-test-instructions — U10 CI・品質管理

> 対象: u10-ci-governance（packaging）。現行 `code-generation-plan.md` とTesting Contract、`../nfr-requirements/security-requirements.md`
> のNFR2.1〜2.5 / NFR4.1〜4.5、`../nfr-design/security-design.md` §7に従う。以下はすべて本Unitに限定する。
> 2026-08-22の旧手順は `unit-test-instructions-history-2026-08-22.md` に全文保存した。

## 1. ランナーと設定

packaging Unitのため「単体テスト」は、設定の事実を機械検査するbashスクリプト `scripts/governance/verify-ci-governance.sh` と、本UnitがCIへ
組み込んだ `tools/lint` の既存自己テストである。追加の設定ファイル・ランナー・モックは導入しない。`jq` を使い、`--with-ruleset` 指定時のみ
`gh`（読取のGETだけ）でGitHubへアクセスする。Rustの版は `rust-toolchain.toml`、依存は各 `Cargo.lock` を正本とする。

## 2. Unit限定コマンド

ワークスペースルートで実行する。`bash -n` は最初のファイルしか解析しないため、ファイルごとに個別に実行する。

```sh
bash -n scripts/coverage.sh
bash -n scripts/governance/verify-ci-governance.sh
bash -n scripts/governance/ruleset-required-checks.sh
bash -n scripts/governance/toolchain-inputs.sh
bash scripts/governance/verify-ci-governance.sh                 # 設定の機械検査（ネットワーク不要）
bash scripts/governance/verify-ci-governance.sh --with-ruleset  # 上記 + gh api でruleset「main」の必須コンテキスト（読取のみ、ネットワーク要）
bash scripts/governance/toolchain-inputs.sh                     # rust-toolchain.toml から channel / components を導出
cargo test --manifest-path tools/lint/Cargo.toml                # tools/lint 自己テスト（CI組込み対象）
```

計画準備時（2026-09-06）の `verify-ci-governance.sh --with-ruleset` は20項目成功・失敗0であった（`../revision-baseline-20260906.md`）。
実行担当は上記を再実行し、件数・結果・完了日時を `code-summary.md` へ残す。ワークスペース全体の `cargo test --workspace` はCIの
品質ゲートであり、本ファイルのUnit限定コマンドではない。

## 3. 合格基準と検証範囲

- `bash -n` 4本がすべて終了コード0。
- `verify-ci-governance.sh` が既定で19項目、`--with-ruleset` で20項目すべて成功（期待値はスクリプト内の定数: channel `1.95.0`、
  components `rustfmt clippy llvm-tools`、profile `minimal`、`TOLERANCE=0.01`、除外式 `(^|/)modules/app/aidlc/src/main\.rs$`、
  シード `20260823`、必須コンテキスト集合 `CI Success` / `check` / `coverage` / `quint`）。期待値を書き換えて成功させない。
- `toolchain-inputs.sh` の出力が `channel=1.95.0`、`components=rustfmt,clippy,llvm-tools`。
- `tools/lint` 自己テストが成功し、対象が0件に減っていない。件数は実際の出力から記録し、過去の31本に固定しない。

受入（Unit限定コマンドの外側、計画Step 3）は次を実測して記録する。設定の存在と実働の成功を区別する。

- `bash scripts/coverage.sh` を同一リビジョン・同一ツールチェーン・同一シードで2回実行し、生のhead値（%）と差を記録する。絶対ゲート90%が
  2回とも成功すること。差0.00ポイントは受入目標であり、未達なら未達のまま原因を記録し、`TOLERANCE` や除外を変えない。
- `cargo audit`（workspace）と `cargo audit --file tools/lint/Cargo.lock` の結果、走査crate数、advisory DB取得可否。未導入・取得失敗は
  成功と書かない。
- `unsafe` を含む一時変更で `cargo check` が拒否されること（workspaceメンバー1クレートと `tools/lint`）。確認後に必ず戻す。
- `rustc -V` が1.95.0。

Unit限定コマンドと上記実測の成功は、全CI実行、マージキューの成功・失敗両経路の実働、レビュー再評価の反映、外部再利用ワークフロー内部の
検証の代替ではない。全体検証をUnitごとに繰り返すコマンドはここへ置かない。

## 4. データとテスト支援

検査対象は実ファイルと実ruleset（読取のみ）。`ruleset-required-checks.sh` は今回実行しない（`--dry-run` を含め、設定変更の意図がないため）。
過去の前後JSONは `ruleset/` 配下と `../ruleset-observed-20260906.json` を読取専用で参照する。認証トークン・認証ヘッダーを記録へ混ぜない。

## 5. 失敗時

失敗した検査名・コマンド・出力を記録する。設定と要件・設計の不一致であれば、`verify-ci-governance.sh` へ検出項目を先に追加する（Red）変更案を
親セッションへ返し、計画を更新してから設定を変える。今回の記録是正のために設定を壊して人工的なRedを作らない。

## E. 改訂済み要件・設計・改訂基準（逐語）

<!-- nfr-requirements/security-requirements.md -->

# security-requirements — U10 CI・品質管理（`u10-ci-governance`）

> 2026-09-06改訂。CI設定と承認済み方針へ要件を整合させる。今回の改訂は文書3点に限定し、GitHub設定・品質閾値・プロダクトコードは変更しない。

## Sources

- [Q1] `nfr-requirements-questions.md` の2026-09-06確認要約（Looks correct）。
- [requirements] `../../../inception/requirements-analysis/requirements.md` のFR9.1〜9.5、NFR2、NFR4。
- [contracts] `../../../inception/contract-design/contract-summary.md` と `../../../inception/units-generation/unit-of-work.md`。U10はpackagingで、製品の外部契約を所有しない。
- [local] リポジトリルートの `.github/workflows/ci.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/toolchain-inputs.sh`、`scripts/governance/ruleset-required-checks.sh`（2026-09-06読取）。
- [observed] `../ruleset-observed-20260906.json`。同日取得済みのGitHub設定を示す観測記録であり、将来の状態を保証するものではない。
- [history] `../code-generation/superseding-decisions.md` の過去の裁定。暫定許容差0.05などの旧値は現行値と区別する。

## 1. 範囲と信頼境界

対象はCI、品質検査、依存検査、マージ条件の管理である。FR9.6のエラー様式規則はU9の責務であり、ここでは変更しない。

信頼境界は、GitHub Actionsの実行環境とトークン、外部Action・再利用ワークフローの取得先、crates.io・RustSec advisory DB・Node/Quintの配布元、管理者権限で変更するrulesetに分かれる。SHA固定は特定版への固定であり、そのコード自体の安全性を証明するものではない。

観測済みのruleset「main」（ID 21190453）はactiveで、必須チェックは `check` / `quint` / `coverage` / `CI Success` の4つ、strict有効、bypassなし。削除・force push防止、マージキューのSQUASH・ALLGREEN・同時1件が設定されている。設定の存在と、成功・失敗両経路の実働確認は別の証拠として扱う。

`ci.yml` は `pull_request` / `merge_group` / `workflow_dispatch` で起動する。`CI Success` は基本3チェックと `aidlc-distribution`、`review-thread-resolution` の結果を集約する。`audit` は集約対象・必須チェックともに含めない。

## 2. 要求

| ID | 要求 | 測定可能な合格基準 | 出典 |
|---|---|---|---|
| NFR2.1 | 必須チェック4つをrulesetで強制し、既存のマージキュー・保護規則・bypassなしを維持する | 設定JSONで4コンテキストの集合とstrict=trueを確認する。必須検査失敗時にマージされない経路と、全成功時にキューを完走しsquash-mergeされる経路の両方について、対象変更・実行URL・結果を保存する | FR9.1, NFR2, Q1, observed |
| NFR2.2 | キュー用検査を実行し、CI Successが依存検査の失敗・取消・不正なスキップを成功へ読み替えない | check/quint/coverage/aidlc-distributionはすべてsuccess必須。変更提案ではreview-thread-resolutionもsuccess必須、merge_group/workflow_dispatchでは同検査のskippedを受理する。イベントごとの実行結果を確認する。coverageは変更提案時に絶対・相対ゲート、他2イベントでは絶対ゲートを実行する | NFR2, Q1, local |
| NFR2.3 | workspaceと独立クレートtools/lintを品質検査の対象にする | checkの実行ログでworkspaceのfmt/clippy/cargo lint/testと、tools/lintのmanifest-path指定によるfmt/clippy/testが成功する。テスト件数は実行時の結果を記録し、過去の31本に固定しない | FR9.3, NFR2 |
| NFR2.4 | シード20260823をCIとローカルで統一し、カバレッジ相対差の許容を0.01ポイントに維持する | 同一コード・ツールチェーン・シードで2回測定し、生のhead値と差を記録する。差0.00ポイントの再現性を受入目標とし、未達なら未達のまま原因を記録する。相対ゲートはhead >= base - 0.01で判定する。固定シードの存在だけで再現性達成とはしない | FR9.4, NFR2, Q1 |
| NFR2.5 | main.rsの配線ファイルだけを明示除外し、残るworkspace計測対象のカバレッジ90%以上を維持する | 除外式が `(^|/)modules/app/aidlc/src/main\.rs$` のみで、クレート全体の除外がないことを確認する。計測結果が90%以上でabsolute gate成功。tools/lintはworkspace外であり、この90%床の対象と誤記しない | FR9.5, NFR2 |
| NFR4.1 | workspaceとtools/lintの両Cargo.lockをcargo auditの対象とし、結果を可視化する | 両方の実行・結果を識別できるログを残す。脆弱性検出・DB取得失敗・未実行を成功と扱わない。先行ステップ失敗で後者がskippedなら両方成功とは扱わず、必要な再実行で確認する。auditは既存裁定によりadvisoryであり、単独の赤はrulesetによるマージ阻止を保証しない | FR9.2, NFR4, Q1 |
| NFR4.2 | Rust 1.95.0、rustfmt/clippy/llvm-tools、minimalをrust-toolchain.tomlで一元管理する | CI入力がtoolchain-inputs.shで同ファイルから導出され、ローカルとCIのrustcが指定版1.95.0に一致することをログで確認する | FR9.2, NFR4 |
| NFR4.3 | workspaceメンバーとtools/lintでunsafe_code=forbidを適用する | 全workspaceメンバーのlints継承とtools/lintの個別宣言を確認する。両範囲のclippyが成功し、適用検証ではunsafeを含む不適合例が拒否される | FR9.2, NFR4 |
| NFR4.4 | workflow既定をcontents: readとし、レビュー検査に必要な個別権限だけを付与する | review-thread-resolutionにcontents: read、checks: write、statuses: write、issues: read、pull-requests: readがあることを確認する。他ジョブの追加書込権限がないこと、外部呼出先とci_refが同じSHAで固定されること、トークンを出力しないことを設定・実行ログの検査対象にする | FR9.2, NFR4, Q1 |
| NFR4.5 | rulesetの変更内容と実行主体を追跡可能にする | ruleset-required-checks.shの手順、変更時の前後JSONと結果を保存する。既存規則・4コンテキスト・strict・bypassの維持を確認する。現在値と要求が同じ場合は変更不要として記録する | NFR4, Q1 |

### 運用規範

ツールチェーン・シード・依存・CI・Action参照版の更新は、レビュー対象の変更提案を経て行う。これはNFR4.2の版一致という測定基準とは別の運用規範である。ruleset変更は権限を持つ担当者が実行し、今回の要件改訂では実行しない。脆弱性検出時は依存更新を検討し、外部DB取得失敗時は原因と再実行結果を記録する。

### 現時点の確認と未検証事項

設定の読取と保存済みruleset JSONから、4コンテキスト、権限、シード、閾値、ツールチェーンの宣言を確認した。今回の要件改訂ではカバレッジ2回測定、cargo audit、全CI実行、キューの成功・失敗試験は実行していない。これらは後続の検証項目であり、達成済みとは記録しない。

旧レビューが扱った許容差0.05・残差0.0175ポイントは過去の実測と暫定裁定である。現行の `scripts/coverage.sh` は0.01であり、今回の要件は確認済み要約に従いこの値を維持する。

## 3. 脅威の検討（STRIDE、ガバナンス面）

| 区分 | 脅威 | 対応と限界 |
|---|---|---|
| Spoofing（なりすまし） | トークンや管理者権限の悪用 | GitHubの認証下でもトークンは秘密情報。個別権限と利用先を限定する（NFR4.4/4.5） |
| Tampering（改竄） | 不合格コードのマージ、依存・外部Actionの改竄 | 必須チェック・既存保護・lockファイル・依存検査を組み合わせる。auditは署名検証の代替ではない |
| Repudiation（否認） | 誰がどのrulesetを変更したか不明 | NFR4.5の前後JSON・実行結果・実行主体を記録する |
| Information Disclosure（情報漏洩） | トークンがログや外部コードに渡る | トークンをログ出力しない。外部再利用ワークフローと実行権限を明記する。公開ログでも秘密情報がないと無条件に断定しない |
| Denial of Service（利用不能） | 外部配布元障害、検査未実行、キュー停滞 | イベント別の実行経路を検証し、失敗を可視化する。auditだけは必須外という既存裁定を維持する |
| Elevation of Privilege（権限昇格） | checks/statusesへの書込権限を持つ外部ワークフローの悪用 | レビュー検査のみに個別権限を与え、呼出先とci_refをSHA固定する。固定版更新時も変更内容をレビューする |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| 公開CIログ・カバレッジ結果 | Public | 公開を前提に秘密情報の出力を防ぐ。検査結果の未実行・失敗を区別する |
| ruleset観測JSON・前後JSON | Internal（運用記録） | 内容を確認して記録に保存する。認証トークンや認証ヘッダーを混ぜない |
| GITHUB_TOKENなどの認証情報 | Secret | ジョブごとの権限を限定し、ログ・成果物へ出力しない |

## 5. 適用外と繰り延べ

- NFR1: U10は製品のupstream互換面を変更しないため直接の派生要件はない。
- NFR3: 製品の永続化・投影を持たないため対象外。ガバナンス変更の記録はNFR4.5で扱う。
- NFR5: U10固有のCI実行時間の数値目標は設けない。製品CLIの性能劣化測定要求を取り消すものではない。
- Dependabot（github-actions/cargo）の導入は既存裁定により見送り、後続の検討事項とする。
- 全ActionのSHA固定は未採用。現状では配布検証ジョブのcheckout/setup-bunとレビュー用外部ワークフローが固定され、他にはタグ・ブランチ参照が残る。全件固定済みとも全件未固定とも記載しない。

## Assumptions & Open Questions

None.


## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T13:55:52Z
**Iteration:** 1
**Request Challenge:** review:c23f88e6478d662f8377718fce748442

### Findings

本レビューはadvisory（承認判断の参考となる独立レビュー）。確認済みの2026-09-06要約と現行設定を基準とし、過去の暫定値を現在の要求として扱わない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
| R-01 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/nfr-requirements/security-requirements.md > §2 NFR2.5行、および aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/nfr-requirements/tech-stack-decisions.md > §1 カバレッジ行 | 除外式の縦棒がMarkdown表の区切りとして解釈されるため、両行ともヘッダー4列に対して5列になる。合格基準・出典や理由の表示がずれる。式そのものはcoverage.shと一致している。 | 両方の表内で正規表現の縦棒をMarkdown用にエスケープするか、正確な式を表の外へ移して参照する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| required-sections | PASS、security-requirements: H2=7、tech-stack-decisions: H2=5 | レビュー追記前の2成果物で必要な文書構造を確認した。 |
| upstream-coverage | PASS、未参照0 | 今回解決済みの入力requirements・contract-summaryと、指定成果物security-requirements・tech-stack-decisions・traceabilityを指定して検査した。静的定義の全入力を指定した初回はfunctional-spec・rules・technology-stackを未参照としたが、今回のUnitへ配送された入力集合とは異なるため、その結果を欠落所見にはしない。 |
| traceability | PASS、gaps/orphans/invalid_targets等すべて0 | NFR1〜5の網羅とN/A理由があり、NFR2/NFR4から計10件の派生要求へ対応する。 |
| NFR派生IDの行存在確認 | PASS | traceability.jsonの全OK targetがsecurity-requirementsの要求行として存在する。 |
| Markdown表の列数確認 | FAIL、2行 | R-01の2行のみ4列に対して5列。 |
| linter | 対象外、直接起動はno-eslint-config（終了127） | 今回はMarkdown/JSON文書で、TS/JSコードの成果物・対象スニペットはない。ESLintによる検証成功とは扱わない。 |
| type-check | 対象外、直接起動はno-tsconfig-found（終了1） | 今回はMarkdown/JSON文書で、TS/TSXコードの成果物・対象スニペットはない。TypeScript検査成功とは扱わない。 |
| doctor | 46 passed / 0 failed | 検査設定の確認に使用。未初期化submodule・runtime-graph未生成等のadvisoryは、本要件の設定照合結果とは分ける。 |
| 現行設定・観測JSONとの照合 | 一致 | 4必須コンテキスト、strict、bypassなし、SQUASH/ALLGREEN/同時1件、イベント別CI Success、audit必須外、ジョブ別権限、固定シード、0.01、90%床、main.rsのみ除外、Rust版とAction参照範囲を確認した。 |
| 上流契約の責務照合 | 一致 | U10はpackagingでFR9.1〜9.5・NFR2/NFR4を担当し、製品外部契約C1/C2は所有しない。FR9.6のU9帰属を変更していない。 |

### Summary

必須チェックとキュー正常系・異常系、外部ワークフローの権限境界、依存検査の限界、現行の品質閾値を検証可能な要求として記述できている。残る所見は表の表示崩れ1件であり、カバレッジ再測定・依存監査・実際のキュー完走等が未実施である点も明示されているためREADYとする。今回はGitHub書込、全CI実行、カバレッジ測定は行っていない。

<!-- nfr-requirements/tech-stack-decisions.md -->

# tech-stack-decisions — U10 CI・品質管理（`u10-ci-governance`）

> 2026-09-06改訂。既存設定と確認済み要約を記述し、過去の導入予定・暫定値を現行値へ整合する。

## Sources

- [Q1] `nfr-requirements-questions.md` の2026-09-06確認要約（Looks correct）。
- [requirements] `security-requirements.md` のNFR2.1〜2.5 / NFR4.1〜4.5、`../../../inception/requirements-analysis/requirements.md` のFR9.1〜9.5。
- [contracts] `../../../inception/contract-design/contract-summary.md`。U10は製品の外部契約を所有しない。
- [local] `.github/workflows/ci.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/`（リポジトリルート基準、2026-09-06読取）。
- [observed] `../ruleset-observed-20260906.json`、[history] `../code-generation/superseding-decisions.md`。

## 1. 選定

| 領域 | 現行の選定 | 理由・境界 | 不採用案・注意点 |
|---|---|---|---|
| マージの機械強制 | ruleset「main」、必須check/quint/coverage/CI Success、strict=true、bypassなし | 既存のSQUASH/ALLGREEN/同時1件のキューと削除・force push防止を維持する（NFR2.1） | classic branch protectionを重ねて二重管理しない |
| イベント別検査 | pull_request/merge_group/workflow_dispatch。concurrencyはworkflow名とrefで分離 | 変更提案とキューの検査が衝突しない。coverageの相対比較は変更提案時、他は絶対ゲート（NFR2.2） | キュー無効化や、全イベントに同じbase refを仮定する案は採らない |
| CI Success | aidlc-distribution/check/quint/coverageをsuccess必須とする。review-thread-resolutionは変更提案でsuccess、他イベントでskippedを受理 | 必須4コンテキストを増やさず配布物の同期・回帰とレビュー検査を集約する（NFR2.2） | auditは集約しない。失敗・取消を成功へ読み替えない |
| 外部レビュー検査 | `j5ik2o/ci/.github/workflows/review-thread-resolution.yml@9cf0e9a8cd74c72de704763025003ed3b7608c65`。ci_refも同じSHA | 未解決スレッドを検査する既存方針。外部コードと追加権限の信頼境界を明示する（NFR4.4） | 浮動参照や「全ジョブ読取専用」という説明を採らない |
| Rust | 1.95.0、rustfmt/clippy/llvm-tools、profile=minimal | rust-toolchain.tomlを正本とし、toolchain-inputs.shがchannel/componentsをCIへ渡す（NFR4.2） | stableという浮動版を指定しない。dtolnay/rust-toolchain@master自体はSHA未固定 |
| 依存検査 | auditジョブでcargo auditをworkspaceとtools/lintの両Cargo.lockへ実行 | 対象を明記し結果を可視化。advisoryとして必須チェック外に置く裁定を維持（NFR4.1） | 全依存監査成功・マージ阻止を、ジョブの存在だけで主張しない |
| unsafe禁止 | workspace.lints.rustとtools/lintの個別lints.rustでforbid | 独立クレートを含め適用する（NFR4.3） | クレート個別attributeだけに依存しない |
| 権限 | workflow既定contents: read。レビュー検査のみchecks/statuses: writeとissues/pull-requests: readを追加 | トークンを秘密情報として扱い、追加権限を限定（NFR4.4） | workflow全体をwriteへ広げない |
| tools/lintの品質 | manifest-path指定のfmt/clippy/testをcheckに含める | workspace外の独立クレートを明示検査（NFR2.3） | workspace検査や90%床に自動で含まれるとは扱わない |
| カバレッジ | 絶対90%、相対許容0.01ポイント、除外は `(^|/)modules/app/aidlc/src/main\.rs$` のみ | 配線ファイルだけを除き、その他のworkspace計測対象を維持（NFR2.5） | クレート単位除外は採らない |
| 性質検証の乱数 | PROPTEST_RNG_SEED=20260823をCIとcoverage.shで統一 | ランダム経路を固定し計測再現性を検証する（NFR2.4） | 過去の暫定0.05を現行値としない。シード固定だけで差0.00達成とはしない |
| 形式検証・配布検証 | Node 22/Quint 0.32.0でquint-gate.sh、Bun 1.3.13で配布同期・回帰試験 | 既存のCI検査範囲を維持する | 新たなクラウド資源やAWS Bedrockは導入しない |

## 2. 依存と変更の範囲

今回変更するのは要件・技術選定・対応表の3成果物のみ。Rust依存、lockファイル、CI設定、rulesetを新規作成・更新する作業ではない。

Action参照版の固定状況は次のとおりである。

- aidlc-distributionのcheckoutは `11d5960a326750d5838078e36cf38b85af677262`、setup-bunは `0c5077e51419868618aeaa5fe8019c62421857d6` に固定されている。checkoutはpersist-credentials=false。
- 外部レビュー検査は表のSHAへ固定されている。
- 他ジョブにはactions/checkout@v4、actions/setup-node@v4、Swatinem/rust-cache@v2、taiki-e/install-action@v2、dtolnay/rust-toolchain@masterが残る。全件SHA固定は本intentでは採用していない。
- Dependabot（github-actions/cargo）の導入は既存裁定で見送り。手動の変更提案による依存・参照版更新を維持する。

`scripts/governance/ruleset-required-checks.sh` は既存設定を保持しながら4コンテキストの集合とstrictを確認・補正する手順である。実際の変更時には前後JSONと結果を保存する。今回の要件更新ではこの書込処理を実行しない。

## 3. 確定事項と後続の検証

シード20260823、相対許容0.01ポイント、main.rsのみ除外、イベント別coverage、ruleset管理スクリプトの配置は確定済みであり、未決の技術選定として再掲しない。

残るのは受入の実測である。カバレッジ2回測定の差、CI上のRust版一致、両Cargo.lockの依存監査、マージキュー成功・失敗の実働を検証し、対象版と結果を保存する。今回読んだ設定・過去の実績を、現在版に対する新しい試験結果として扱わない。暫定0.05と旧ロック試験由来の残差は履歴に残し、今回承認された0.01への要件を覆さない。

## Assumptions & Open Questions

None.

<!-- nfr-design/security-design.md -->

# security-design — U10 CI・品質管理（`u10-ci-governance`）

> 2026-09-06改訂。更新済みの品質・安全性要件を、CI・設定管理・検証の設計に具体化する。今回更新する成果物は本書とtraceability.json。GitHub設定・コード・品質閾値は変更しない。

## Sources

- [Q1] `nfr-design-questions.md` の2026-09-06確認要約（Looks correct）。
- [requirements] `../nfr-requirements/security-requirements.md` のNFR2.1〜2.5 / NFR4.1〜4.5。
- [technology] `../nfr-requirements/tech-stack-decisions.md`。
- [contracts] `../../../inception/contract-design/contract-summary.md`、`../../../inception/domain-design/components.md`。U10はpackagingであり、製品CLI・フックの外部契約C1/C2を所有しない。
- [local] リポジトリルートの `.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/`（2026-09-06読取）。
- [observed] `../ruleset-observed-20260906.json`。同日取得済みの設定記録であり、実働試験の代わりにはしない。
- [history] `../code-generation/superseding-decisions.md`。暫定0.05などの旧裁定は過去の記録として区別する。

## 1. 設計方針と境界

CIは検査結果を計算し、GitHubのrulesetは必須結果とキュー条件をマージ判断へ適用する。設定の存在、検査の実行、結果の受理を分けて検証する。未実行・取消・取得失敗を成功へ読み替えない。

構成はCI定義、レビュー結果の再評価、品質設定、ruleset管理手順に分ける。外部依存にはAction・再利用ワークフロー、Rust/Node/Bun/Quintの配布元、crates.io、RustSec advisory DBがある。外部障害をマージ条件から隔離するのはauditだけで、他の必須検査が外部取得に失敗すればマージを止める。

トークンは秘密情報として扱う。SHA固定は取得版を固定する手段であり、提供元やコードの無害性を保証しない。参照版の変更は差分レビューと検証を伴う変更提案で行う。新規クラウド資源・AWS Bedrock・製品の永続化機構は導入しない。

## 2. CIの構成と結果の受理

`ci.yml` はmain向けの変更提案、merge_group、workflow_dispatchで起動する。concurrencyはworkflow名とrefで分離し、同一組の古い実行を取り消す。取消の結果を合格として受理しない。

| ジョブ | 責務 | 必須結果との関係 | 失敗時の扱い |
|---|---|---|---|
| check | workspaceのfmt/clippy/cargo lint/test、tools/lintのmanifest-path指定fmt/clippy/test | checkとして直接必須、CI Successもsuccess必須 | 不合格としてマージを止める |
| quint | Node 22 / Quint 0.32.0でquint-gate.shを実行 | quintとして直接必須、CI Successもsuccess必須 | モデル検査失敗・取得失敗を合格にしない |
| coverage | coverage.shで絶対・条件付き相対ゲートを評価 | coverageとして直接必須、CI Successもsuccess必須 | 閾値未達・計測失敗を合格にしない |
| aidlc-distribution | Bun 1.3.13で配布同期・ローカル修正と回帰試験を検査 | CI Success経由で必須 | 同期差分や回帰失敗でCI Successを止める |
| review-thread-resolution | SHA固定の外部ワークフローで未解決スレッドを検査 | 変更提案でCI Successがsuccess必須 | 失敗・取消・未実行は合格にしない |
| ci-success（表示名CI Success） | 上記結果をイベント別条件で集約 | CI Successとして直接必須 | always()で起動して結果を検査し、合わない結果を拒否 |
| audit | workspaceとtools/lintのCargo.lockをcargo auditへ渡す | 直接必須にもCI Successにも含めない | 赤・未実行を可視化し、脆弱性対応と取得失敗の再実行を分ける |

### イベントごとの違い

| イベント | check/quint/coverage/aidlc-distribution | レビュー検査 | coverage比較 |
|---|---|---|---|
| pull_request | 全件success必須 | success必須 | 絶対90%とbaseに対する相対差 |
| merge_group | 全件success必須 | skippedを受理 | 絶対90%のみ |
| workflow_dispatch | 全件success必須 | skippedを受理 | 絶対90%のみ |

CI Successは `needs` の結果を検査する。基本4検査のskippedやcancelledは受理しない。レビュー検査をスキップできるのは、変更提案以外の2イベントである。auditの先行コマンドが失敗して後続のtools/lint検査が走らなかった場合、そのロックファイルの監査は未実行として扱い、再実行で確認する。

### レビュー結果の再評価

`review-thread-resolution.yml` はレビュー・コメントの作成/変更/削除等、15分間隔、手動実行を契機に、同じ外部ワークフローで `Check unresolved comments` の状態を再評価する。手動指定では対象番号を指定でき、無指定は外部ワークフローへ空の入力を渡す。

再評価するコミットステータスと、`ci.yml` の実行時に集約したCI Successは別の出力である。再評価だけで既に完了したCI Successも自動更新されるとは、このローカル定義だけから保証しない。スレッドの解決・再開後にどの結果が更新され、最新のマージ条件へ反映されるかを実働検証で確認する。

## 3. 権限・秘密情報・外部コード

`ci.yml` のworkflow既定はcontents: readであり、追加権限はreview-thread-resolutionに限定する。別ファイルの再評価ワークフローでは、workflowとrefreshジョブに同じレビュー用権限を明示する。

| 対象 | 宣言する権限 | 目的と境界 |
|---|---|---|
| ci.ymlの通常ジョブ | contents: read | ソース・依存取得と検査。既定をwriteに広げない |
| ci.ymlのreview-thread-resolution | contents: read、issues: read、pull-requests: read、checks: write、statuses: write | レビューの読取と検査・状態の反映。外部ワークフローへ与える権限として明示 |
| review-thread-resolution.ymlのrefresh | 同上 | レビュー状態の再評価。別ワークフローの権限であり「全workflowが読取専用」とは記述しない |

両レビュー呼出の参照版とci_refは、以下の同一SHAを使う。

`9cf0e9a8cd74c72de704763025003ed3b7608c65`

呼出先は `j5ik2o/ci/.github/workflows/review-thread-resolution.yml`。更新時は参照版とci_refの一致、権限差分、入力・出力の契約、解決/未解決/検査不能の結果を確認する。トークンをログ・保存JSON・成果物へ出力しない。

配布検証ジョブのcheckout/setup-bunもSHA固定され、同ジョブのcheckoutはpersist-credentials=false。その他にはタグ/ブランチ参照が残る。特定ジョブの設定を全ジョブへ一般化しない。全Actionの一括SHA固定とDependabot導入は既存裁定により見送る。

## 4. rulesetの管理と復旧

観測済みのruleset「main」は4コンテキスト（check/quint/coverage/CI Success）、strict=true、bypassなし。deletion・non_fast_forward・merge_queueを維持し、キューはSQUASH・ALLGREEN・同時1件とする。

管理手順の置き場は `scripts/governance/ruleset-required-checks.sh`。設計上の操作順は次のとおり。

1. 名前から対象を解決し、現在のJSONを取得してbefore.jsonに保存する。認証情報は保存しない。
2. 必須コンテキストの集合とstrictを比較する。一致すればPUTを実行しない。
3. 不一致なら、既存rulesからrequired_status_checksだけを置換し、他の規則・conditions・bypass_actorsを保持した送信用JSONを作る。dry-runでは予定JSONを確認する。
4. 権限を持つ担当者が変更し、再取得したafter.jsonで結果を確認する。既存スクリプトの自動検査はコンテキスト集合・strict・保護規則の存在を確認するため、キューの具体値やbypassの不変性は前後JSONの比較でも確認する。
5. 必須検査失敗時にマージを止める経路と、全成功時にキューを完走する経路を実働で確認し、対象版・実行結果を保存する。

変更には保存先を明示し、記録欠落を避ける。誤設定時は管理者が現在値とbefore.jsonを比較し、並行して行われた正当な変更を上書きしないよう復元対象を決める。GET結果にはPUT非対応のフィールドも含まれるため、before.jsonを無加工でPUTしない。復元後も再取得・差分確認・成功/失敗両経路の検証を行う。

今回は要件に合う設定が観測されているため、設計更新のためのGitHub書込は行わない。

## 5. 品質設定と再現性

Rust版と構成要素は `rust-toolchain.toml` を正本にする。channel=1.95.0、components=rustfmt/clippy/llvm-tools、profile=minimal。CIはtoolchain-inputs.shからchannel/componentsを導出して渡す。ローカルとCIの実際の版一致は別途ログで検証する。

unsafe_code=forbidはworkspace.lints.rustで定義し、各メンバーのlints.workspace=trueで継承する。tools/lintは独立クレートなので個別のlints.rustへ同じ禁止を宣言する。ルートに書いただけで全クレートへ適用済みとは判断しない。

カバレッジの正本はcoverage.shで、CI側も同じシードを宣言する。

| 項目 | 値・方式 | 検証 |
|---|---|---|
| 絶対床 | 90.0% | workspaceの計測値が床以上 |
| 相対許容差 | 0.01ポイント | head >= base - 0.01。base比較は変更提案時 |
| シード | PROPTEST_RNG_SEED=20260823 | CIとローカル・headとbaseで同じ値 |
| 明示除外 | modules/app/aidlc/src/main.rsの1ファイルのみ | 下記の式が計測へ渡り、他ファイルやクレートを除外しない |
| 再現性 | 同一コード・版・シードの2回測定 | 生のhead値と差を記録し、差0.00ポイントの受入目標への達否を判定 |

除外式は表の区切りと混同されないよう、表の外に記載する。

`(^|/)modules/app/aidlc/src/main\.rs$`

シード固定は必要条件であり、再現性の実証ではない。取得失敗・計測失敗・残る非決定性は未達として記録する。tools/lintはworkspace外で、90%床の対象には含めない。過去の暫定0.05・旧ロック試験由来の残差は履歴として保持し、現在の0.01と混同しない。

## 6. 論理コンポーネントと障害の影響範囲

| コンポーネント | 配置 | 障害の影響 | 手当て |
|---|---|---|---|
| CI検査とCI Success | ci.yml | 個別実行の不合格。共有設定の誤りは同じ定義を使う複数実行へ波及する | 原因の検査単位を識別し、設定修正後に対象版を再検証 |
| レビュー検査・再評価 | ci.yml、review-thread-resolution.yml、外部固定版 | 対象変更のマージ阻止。外部実装・共通設定の障害や誤検知は複数の変更へ波及し得る | 対象・コミット・入力と出力を照合し、結果の更新経路を確認 |
| audit | ci.yml | 監査失敗/未実行は可視化されるが単独ではマージを止めない | 脆弱性対応と外部取得失敗を区別し、両ロックファイルの結果を確認 |
| ruleset | GitHub | 誤設定は全対象マージの停止または誤許可を招く | 前後JSON、差分確認、必要時の復旧と両経路の実働検証 |
| ツールチェーン・lints | TOML、導出スクリプト | 主にRustを使うcheck/coverage/auditへ影響。全ジョブが同じ障害になるとは限らない | 正本と導出入力・継承先を確認 |
| カバレッジ | coverage.sh | coverage不合格からCI Success・マージ条件へ伝播 | 対象ファイル・閾値・比較条件・再現性を確認 |

共有資源はランナー・キャッシュだけではない。外部配布元、再利用ワークフロー、トークンの権限、リポジトリ設定も共有の依存・境界である。キャッシュがあることを取得成功や検査成功の根拠にしない。

## 7. 要求対応と検証計画

| 要求 | 設計箇所 | 検証方法 |
|---|---|---|
| NFR2.1 | §4 必須4コンテキストとキュー | 設定JSON、失敗時停止と全成功時完走 |
| NFR2.2 | §2 イベント別集約 | 各イベントの結果・取消・スキップ、レビュー再評価の反映 |
| NFR2.3 | §2 check | workspaceとtools/lintの各検査ログ |
| NFR2.4 | §5 同条件計測 | 生の2回測定値、差、head/baseの比較 |
| NFR2.5 | §5 除外と床 | 除外範囲と絶対ゲート |
| NFR4.1 | §2 audit | 両ロックファイルの実行・未実行・失敗の識別 |
| NFR4.2 | §5 正本と導出 | ローカル/CIのRust版一致 |
| NFR4.3 | §5 lints継承 | 全メンバーと独立クレートの宣言、unsafe不適合例の拒否 |
| NFR4.4 | §3 権限と外部コード | ジョブ別権限、固定版の一致、秘密情報の非出力 |
| NFR4.5 | §4 設定管理 | 前後JSON、変更対象・実行者・結果、復旧手順 |

設定確認にはverify-ci-governance.shを使う。その成功は設定の存在の証拠であり、全CI・カバレッジ2回測定・依存監査・キュー完走・レビュー再評価の実働成功を意味しない。今回の設計更新ではそれらを実施せず、後続の受入項目として残す。

上流要件のR-01（Markdown表2行の表示崩れ）は上流文書のレビュー所見として残る。本設計では正規表現を表の外へ置き、表示崩れを持ち込まない。上流所見を本設計の完了だけで解消扱いにしない。

## Assumptions & Open Questions

None.


## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T14:08:56Z
**Iteration:** 1
**Request Challenge:** review:8656c6b1637c9f1712c8de1943c19cf7

### Findings

advisory（承認判断の参考となる独立レビュー）として、現在の設計本文に新規所見はない。上流nfr-requirementsのR-01は別文書の未解決所見として保持し、この判定で解消扱いにしない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| required-sections | PASS、H2=9、findings_count=0 | レビュー追記前のsecurity-design本文の構造を確認。 |
| upstream-coverage | PASS、未参照0 | 今回解決済みのsecurity-requirements・tech-stack-decisions・contract-summaryを指定し、security-design・traceabilityの成果物集合で検査した。 |
| traceability | PASS、gaps/orphans/invalid_targets等すべて0 | 対象UnitのNFR詳細要求と対応表に欠落・不正参照はない。 |
| 要求ID・設計節の独立照合 | PASS、10件 | 上流要求行、upstream_ids、coverageのID集合が一致し、各targetの設計節が本文に存在する。 |
| Bun.markdown.htmlによる表の描画確認 | PASS、本文6表 | HTMLへ描画した各表の全行がヘッダーと同じ列数（順に4/4/3/3/4/3列）。正規表現は表外にあり、上流R-01の表示崩れを持ち込んでいない。 |
| linter / type-check | 適用外 | 対象はMarkdown/JSON設計文書で、TS/JS等の実コード・対象スニペットはない。コード検査成功とは扱わない。 |
| 現行CI・設定との照合 | 一致 | CIの7ジョブ、3イベント別の集約条件、audit必須外、別ワークフローの再評価と権限、2呼出のSHA/ci_ref一致、4必須コンテキスト、strictとキュー設定を確認した。 |
| 品質設定・管理手順との照合 | 一致 | Rust 1.95.0と構成要素の導出、unsafe禁止の適用方法、シード20260823、0.01ポイント、90%床、main.rsのみの除外、rulesetスクリプトの比較・保存・送信項目と検査範囲を確認した。 |
| 上流境界との照合 | 一致 | U10はCI・設定管理のpackagingであり、製品外部契約C1/C2や製品の永続化責務を新たに所有していない。 |

### Summary

設計は10件の詳細要求を、具体的なCI定義・設定管理・復旧・受入方法へ対応付けている。レビュー状態の再評価と完了済みCI Successの更新を同一視せず、外部実装の未観測部分や実働未確認事項を区別しているためREADYとする。

本レビューではGitHub書込、全CI実行、依存監査、カバレッジ2回測定、キュー完走試験、外部再利用ワークフロー内部の検証を行っていない。それらは本文§7の後続検証として残る。

<!-- revision-baseline-20260906.md -->

# U10 CI・品質管理の改訂基準

## 位置付け

2026-09-06のmain `53b5667e52ed7d28a395458afb3fe254911b1b45`とGitHub rulesetの実測を基準とする引継ぎメモ。
要件・設計・実装記録の3つの`pending-revision.md`に残る2026-08-23の案より、現状の記述には本メモを優先する。
これは未回答の要約確認や未実施の独立レビューを置き換えるものではない。

## 改訂時に維持する事実

| 対象 | 現行設定と改訂方針 |
| --- | --- |
| 必須チェック | `check`・`quint`・`coverage`・`CI Success`の4つ。strict有効。 |
| 集約条件 | `aidlc-distribution`・`check`・`quint`・`coverage`の成功を必須とする。`pull_request`ではreview-threadの成功も必須。`merge_group`と`workflow_dispatch`ではreview-threadのskippedを必須とする。 |
| audit | workspaceと`tools/lint`の2つのCargo.lockを検査する。必須チェックおよびCI Success集約の対象外。失敗や未実施を成功と記載しない。 |
| 権限 | workflow既定は`contents: read`。review-threadジョブに限り`checks: write`・`statuses: write`・`issues: read`・`pull-requests: read`も付与する。 |
| 信頼境界 | review-threadの外部再利用ワークフローはSHA固定。配布物検証のcheckoutとBun導入もSHA固定。全ActionがSHA固定とは記載しない。トークンは秘密情報として扱う。 |
| カバレッジ | 絶対床90%、相対許容差0.01ポイント、固定シード20260823、除外は`modules/app/aidlc/src/main.rs`のみ。過去の暫定0.05へ戻さない。 |
| 再現性 | 設定・過去の結果と、同一リビジョンの2回実測を区別する。追加測定を行っていない時点で差0.00ポイント達成とは記載しない。 |
| ツールチェーン | Rust 1.95.0。`toolchain-inputs.sh`で正本ファイルからchannel/componentsを抽出してCI Actionの入力へ渡す。 |
| 品質検査 | workspaceと独立した`tools/lint`の双方にfmt・Clippy・テストを実行し、workspaceには`cargo lint`も実行する。 |

正本は`.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`scripts/governance/`。
`bash scripts/governance/verify-ci-governance.sh --with-ruleset`は今回20項目成功・失敗0。
この静的検査・ruleset検査だけで全受入条件を達成したとはみなさない。

## 後続で更新する文書

- 要件: `nfr-requirements/security-requirements.md`、`tech-stack-decisions.md`、`traceability.json`。NFR2.6のレビュー条件、正常系のマージキュー完走、権限と秘密情報の扱いを整合させる。
- 設計: `nfr-design/security-design.md`、`traceability.json`。配布物検証を含む集約条件、4コンテキスト、相対許容差0.01、信頼境界を整合させる。
- 実装記録: `code-generation/code-summary.md`、`traceability.json`。実ファイルと現在の検査範囲へ合わせ、targetに説明を混在させない。

以前の改訂案にあるレビュー所見は、古い要求値と切り分けて再評価する。今回のメモ追加で所見を解決済みにしない。
