# U1実装への引継ぎ

Conversation language: 日本語

## 現在の許可と責任

ユーザーの正確な「Approve Plan」に対し、正規のanswerコマンドはPLAN_APPROVAL_RECORDEDを返した。begin --unit u1-upstream-acceptanceもstatus:generationで成功した。nextを再発行して許可を失効させない。

あなたはU1のみを実装する。所有範囲はscripts/goldens/、新規tests/golden/upstream-a277af21/、そのCI接続、U1成果記録。既存2.6.40ゴールデンとRust本体は変更しない。原本と比較の移行はU2へ渡す。単独で作業していない。親は番号回答の修正検証をしているため、scripts/aidlc-plan-progress.test.ts、scripts/aidlc-sync/patches/numeric-human-response.patch、3配布先hookとinstalled.jsonは親の所有。既存変更を巻き戻さず共存すること。

子エージェントの作成は禁止。ツール拒否や仕様矛盾があれば具体的根拠を親へ返す。実装量を理由に未実装を完了としない。計画のチェックは実際に完了したステップだけ更新する。ステップの本文、Testing Contract、unit-test-instructions.mdは承認済みのため変えない。署名の曖昧さを推測で埋めず本家コードを測定する。

既存テストは親がcargo test --workspaceを実行し2354件すべて成功。ログ/tmp/amadeus-stage1-cargo-baseline.log、exit0。Bun共通検査は親が現在実行中。あなたはU1固有テストを先に用意して振る舞いのRedを確認し、最小実装でGreen、整理して再検証する。TDDのseamsは承認済み計画の3境界。

完了時は親へ変更ファイル・テスト結果・Red/Green証跡・二回採取と全差分の結果・残課題を返す。U1のsource-manifest.jsonとtraceability.jsonとcode-summary.mdに必要な事実を記録する。独立レビューとライフサイクルの記録は親が担当する。レビュー呼出し、承認捏造、完了報告コマンド、コミット、push、タグ作成はしない。

## 設計入力の要約と正本

- aidlc/spaces/default/intents/260907-selfhost-stage1/inception/requirements-analysis/requirements.md: FR1は本家2.7.1への採取・比較・実装適合。NFR1–4は観測互換/TDD・品質/CI全ジョブ/限定変更。
- aidlc/spaces/default/intents/260907-selfhost-stage1/inception/units-generation/unit-of-work.md: U1は採取・比較、U2がRust適合と旧受入参照移行。B1=U1+U2。
- aidlc/spaces/default/intents/260907-selfhost-stage1/inception/contract-design/contract-summary.md: C1のバイト/入出力/来歴/正規化、C2/C3/C6/C7が必須採取対象。ファイル全文を必ず読む。
- aidlc/spaces/default/intents/260907-selfhost-stage1/inception/requirements-analysis/requirements-analysis-questions.md: 実測した呼出し表がある。Step5の必要範囲はこれと配布コードから定める。
- aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md: 規則の衝突優先順。古い2.6.40表記に対し、人間の最新裁定で2.7.1採用が確定済み。
- 本家a277af218f0df7f325d3b8be7b6d90fce2c5bd40はvendor Gitオブジェクトにも存在する。ただし配布元のfork作業ツリーを本家として採取しない。既存のscripts/goldensを読む。
- 未実施の実地スモークや保護無効採取を実地成功として扱わない。CQS例外不可、Rust内部構造への変更はU2が担当。

## 適用する規則束（順番と本文を保持）

### aidlc/spaces/default/memory/org.md

# Org-Level Rules

> Framework defaults. Read with `team.md` and `project.md` from the active
> space. The resolver loads every applicable layer; narrower layers add
> specialisation and must not contradict broader policy.


### aidlc/spaces/default/memory/org.md

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


### aidlc/spaces/default/memory/org.md

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


### aidlc/spaces/default/memory/org.md

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

Build and Test verifies defined coverage floors and affirmed quality targets;
they may not be weakened to make a step pass.

Affirm a stricter posture in `team.md` if the team commits to one.


### aidlc/spaces/default/memory/org.md

## Deployment

We **deploy on merge** to staging environments. Production deploys gate
on a separate manual approval — typically tech lead + product owner
sign-off in CodePipeline or a CD platform's environment protection.

Teams that have invested in test coverage and observability sometimes
graduate to continuous deployment to production (every commit
auto-deploys); that's a team decision, not a framework default.


### aidlc/spaces/default/memory/org.md

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


### aidlc/spaces/default/memory/org.md

## Forbidden

<!-- Things agents must never do -->
<!-- Example: Do not ask questions about topics already decided in previous stages -->


### aidlc/spaces/default/memory/org.md

## Mandated

- **Conversation language — resolution**: Every artifact a person reads or reviews is written in the workflow's established conversation language. The orchestrator resolves that language from the human's substantive prose and MUST state it as a `Conversation language: <language>` line in every delegated brief, because a delegated agent or reviewer never sees the conversation and some stages hand it nothing else (a greenfield run of a stage whose `consumes` are all `conditional_on: brownfield` reaches its lead with no upstream artifact at all). Delegated agents and reviewers resolve the language in this order and stop at the first source that answers: (1) the `Conversation language:` line in your brief — AUTHORITATIVE for delegated work, because the orchestrator regenerates it on every dispatch from the live conversation and it is therefore never staler than a persisted rule; (2) an explicit conversation-language rule in `aidlc/spaces/<active-space>/memory/project.md` — the FALLBACK for a brief that states no language, and the ONLY file a language switch is ever persisted to, so `project.md` ALWAYS outranks a conversation-language rule in `team.md`, which can only ever be a team default and NEVER the record of a switch (cross-file position is NOT recency: the runtime rule chain concatenates `org → team → project → phase`, so `team.md` reaches you before `project.md` in every bundle no matter which was written last, and the winner is this stated precedence rather than the later position); within `project.md`, when it carries more than one conversation-language rule the LAST one under `## Corrections` is the current one (this tie-break governs conversation-language rules ONLY and leaves the additive rule model untouched; the learnings write path appends and never replaces, so a superseded language rule can still be on disk); (3) the verbatim initial description decoded from `<record>/project-description.json`, falling back to `## Project Information` → `**Project**` in legacy records, when it carries a real language signal (not the `[Project description]` placeholder, not a bare identifier or path); (4) any artifact or draft you were handed — the directive's `consumes[]` contracts, the artifact you were dispatched to review, or the lead draft you were dispatched against. Every source is readable on every harness: the rule bundle carries (2) through the dispatch-rules hook on Claude, Codex, and opencode and through always-included agent resources or workspace steering on Kiro, and neither `aidlc-state.md` nor the handed artifacts fall inside the per-unit reviewer read-scope bound.
- **Conversation language — stability**: The established conversation language holds for the whole session, and inside that session for every stage, dispatch, reviewer pass and approval gate of the workflow — nothing but a session boundary ends it. A turn that carries no language signal never changes it — `Approve`, `Looks correct`, an option letter or number, pasted code, a quoted error or stack trace, a bare file path or identifier. Only an explicit human request to switch languages changes it, and that switch takes effect IMMEDIATELY: everything written from that point follows the new language, and the orchestrator states the new language in the `Conversation language:` line of every subsequent delegated brief. Persistence is a separate, later step and never the activation step: the §13 learnings ritual is the ONLY sanctioned write path for persisting a conversation-language switch into `aidlc/spaces/<active-space>/memory/` and it is human-gated, so NEVER edit a memory file directly to record a switch — a direct write skips the tool's audit event, its duplicate key, and its admission conflict-check, and "do not wait for persistence" is never licence to bypass that gate (this bounds the persistence of a language switch and forbids a direct agent edit; it does not govern the deterministic memory writers a stage invokes by contract, such as `aidlc-state.ts practices-promote`, which own the stamped `## Mandated` / `## Forbidden` rules and the five replaced `team.md` sections rather than the `## Corrections` language record). When the ritual offers it, the switch is recorded as a single-line rule under `## Corrections` in `project.md` and NEVER in `team.md`, so the cross-file precedence in (2) never has to arbitrate one switch against another; when the human declines, it is simply not persisted, and the `Conversation language:` line the orchestrator states in every brief carries it for the rest of the session. A session boundary is where that carrier ends, and a workflow outlives it: the resume context the engine injects at session start carries scope, phase, stage, status, agent and next action but NO language, so on the FIRST turn of a new session the orchestrator MUST re-resolve the language before it dispatches anything — the persisted rule from (2), else the human-readable artifacts this workflow has already produced, which record the language the human was last served in, else the verbatim initial description from (3) — and when every one of those is silent it ASKS the human rather than defaulting to English. Re-resolving is not a switch: it is never announced as one and never persisted as one. An unpersisted switch therefore does not outlive its session, which is exactly what persistence buys — a human who wants a switch to survive a resume accepts the ritual, and a human who declines is served correctly for the rest of the session and re-resolved from disk in the next one. A persisted rule NEVER outranks the brief, and never outranks a later explicit human request to switch: it is the fallback for a brief that states no language, and because the learnings write path appends rather than replaces, a superseded language rule can outlive the switch — the LAST conversation-language rule under `## Corrections` is the current one.
- **Conversation language — what to localize**: Write in the resolved conversation language every artifact a person reads or reviews — requirements, user stories, plans, specs, reviews, questions, discovered practices, affirmed team and project rules, evidence, decision rationale, and any other explanatory prose — and the agent's own human-facing conversational output in every turn, for the orchestrator and delegated agents alike: conversational chat messages, status updates, progress reports, and transitional narration between tool calls. Structured-question `prompt`, `header`, `options[].description`, and free-text follow-ups are human-facing prose and follow the same rule; only `options[].label` literals the protocol spells verbatim are preserved tokens. This includes the descriptive text of a rule shaped as `ALWAYS …` / `NEVER …`, where the leading marker is a fixed token but the sentence it introduces is not. A Markdown artifact is not English merely because a tool parses part of it: localize the prose that surrounds a preserved token. Verbatim human input echoed into an artifact is always kept exactly as the human wrote it.
- **Conversation language — preserved tokens**: Any literal a stage file or the stage protocol spells in backticks and tells you to write exactly is a fixed token — keep it English, character for character, and localize only the prose around it. This covers option labels and sentinel VALUES, not just syntax: `[Answer]:` tags with their option letters, the mandatory final option `X. Other (please specify)`, the assumption-confirmation options `A. Accept assumptions` / `B. Convert to follow-up questions` (the engine compares the filled answer against the literal), the `None.` / `None` sentinels under `## Assumptions & Open Questions` and `## Positions`, the `AGREE:` / `OBJECT:` position prefixes, and the `**Collaborator:** <agent-slug>` first line the engine matches exactly before it accepts a stage. Glossing such a literal when you PRESENT it to the human is fine; what you WRITE into an artifact is the literal itself. Also preserved: the source-register tags `[desc]`, `[scope]`, `[assumption]`, `[Q<n>]`, `[memory:M<n>]` with their literal prefixes (`Initial description:`, `Workflow-selected scope:`); the H2 headings the claim-sources sensor matches verbatim (`## Sources`, `## Assumptions & Open Questions`, `## Assumption Confirmation`, `## Review`) plus any other H2 taken from a stage template, which the `required-sections` sensor matches verbatim whenever a template is supplied (the framework ships none, so a team's `aidlc/spaces/<active-space>/memory/templates/` is what arms that check); the reviewer verdicts `READY` and `NOT-READY`; YAML keys and enum values inside fenced blocks (`units`, `name`, `kind`, `depends_on`, `service | spec | ui | packaging | library`); the field labels, status values, and checkbox states of `aidlc-state.md` and the audit shards; the verbatim initial description decoded from `<record>/project-description.json` (the `**Project**` state field is only its safe single-line preview); stable IDs (`FR-1`, `ENT-001`, `BR1.1`); enum and classification values; code and identifiers; file paths; mermaid keywords; and cross-references.


### aidlc/spaces/default/memory/org.md

## Corrections

<!-- Self-learning loop appends here. -->
<!-- Use team.md to record team-wide additions and project.md for
     project-specific specialisation. The loader resolves org → team →
     project at session start and retains every applicable rule. -->

### aidlc/spaces/default/memory/team.md

# Team-Level Rules

> This team's affirmed practices and corrections. Loaded after `org.md` as
> strict-additive guidance; contradictions with broader policy are rejected.
> Populated by the practices-discovery affirmation gate. Edit at the gate,
> not directly.


### aidlc/spaces/default/memory/team.md

## Way of Working

私たちは `main` を統合先とする短命ブランチで作業する。Bolt（実装から統合までの作業単位）を [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) 1 本に対応させ、直列 1 本で進め、squash-merge（履歴を1コミットにまとめる統合）する。根拠は依頼原文の「進め方の規律」と `memory/org.md` の統合先指定である。

マージ前に CI（自動検証）全ジョブの成功とレビュー収束を確認する。現在の CI は未解決レビュースレッドを検査する。必要なレビュー指摘と競合が残らず、更新後の検査が成功するまで修正する。具体的な作業順・各変更の着地条件は後続の実行計画で決める。

基準は現行 `main` のコードとする。既存コードの再設計・説明の書き直しをせず、必要な契約差分に限定する。過去の削除済み記録を前提にしない。


### aidlc/spaces/default/memory/team.md

## Walking Skeleton

私たちは walking skeleton（各部分を接続し、最小構成で端から端まで動かす先行実装）を独立した先行作業単位として設けず、通常の作業単位で進める。要求分析で確定する必須差分を依存順に実装する。根拠は [確認事項](practices-discovery-questions.md) Q1 の回答「A. 通常の作業単位で進める」と全体の内容確認である。

完了条件の実地スモーク（本リポジトリで小さな bugfix 相当の intent を開始から完了まで通す確認）は省略しない。自律実行は行わない。


### aidlc/spaces/default/memory/team.md

## Testing Posture

- **Methodology**: tdd
- **Ordering**: 各変更で失敗するテストを先に実行して red を確認し、最小の実装で green にしてから、テストの成功を維持しながら refactor する。
- 上記は依頼原文で確定済みであり、`org.md` の未確定時の `test-after` 既定は適用しない。
- workspace の行カバレッジ床 90.0% とベース比較を維持する。現設定の相対許容は 0.01 パーセントポイント、乱数シードは `20260823`、除外は `modules/app/aidlc/src/main.rs` 1 ファイルである。相対条件は `head >= base - 0.01` であり、絶対床90.0%に許容誤差を適用しない。これらは実測設定として保持し、テストを通すために緩和しない。
- Quint（状態遷移を検査する仕様言語）3モデル、ITF（モデルの実行トレースを実装で再生する形式）、ゴールデン（観測結果の比較用データ）を外側の受入検査とする。ゴールデンの採用版は未裁定である。
- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo lint`、`cargo test --workspace`、独立した `tools/lint` の検査を維持する。
- `cargo audit` は依存ライブラリの脆弱性検査である。現在の CI 集約では必須対象外だが、今回の「CI 全ジョブ green」はこのジョブの成功も要求する。CI設定の変更が承認されたという意味ではない。
- ゴールデンのバイト一致とキー集合の比較を区別し、未検証ケースと既知の出力差を要求分析へ渡す。既存検査の成功を完全互換の証明へ拡大解釈しない。具体的な限界は [evidence.md](evidence.md) に記録する。
- 本工程ではテスト・カバレッジを再実行していない。既存設定の存在と、検査の成功を区別する。


### aidlc/spaces/default/memory/team.md

## Deployment

私たちは本リポジトリと Claude Code を対象に、`target/release/aidlc` を使う実地スモーク、同バイナリの自己診断、CI全ジョブ成功を確認してセルフホストへ切り替える。現在のバイナリがこの条件を満たすとは扱わない。`runtime::run` を使う統合検査や、試験装置が監査へ人間の応答を置く検査を、releaseバイナリ・Claude Code・実フック・人間の承認による実地スモークの代わりにしない。

切替後は「ホスト = 直近の安定タグ、ターゲット = 開発版」の2版運用とする。切替対象のタグと実体、参照先、復帰先と復帰の検証は後続の実行計画・切替工程で具体化する。今はタグを作成しない。

`org.md` の staging 自動配備は複数環境を想定した既定であり、本件で staging の新設やクラウド配備を追加しない。現物のワークフローから自動配備処理は確認できなかった。


### aidlc/spaces/default/memory/team.md

## Code Style

私たちは `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` と `memory/` を規則の正本とし、README の衝突優先順と各規則の射程を守る。観測互換の契約を最優先にし、上流と実装の不一致を読み替えず人間の裁定を求める。

Rust の整形は `rustfmt.toml`、静的検査は `Cargo.toml` と `cargo lint` に従う。現物の Rust は `1.95.0`、整形幅は100、改行はUnixである。CQRS（書込みと読取りの責務分離）、依存方向、型のカプセル化、境界でのエラー文言変換など、既存規則を変更箇所へ適用する。規則の存在と機械検出の範囲を同一視しない。テスト内の unwrap・expect は `clippy.toml` の許容範囲に従い、一律禁止と読み替えない。具体的な型・戻り値・配置は正本を参照し、この規則記録で新設しない。

ステージ・エージェント・プロトコル・コンパイル済みグラフは配布資産を再利用する。独自にステージ本文を作らない。配布ファイルの修正は同期用パッチに記録する。

会話・成果物は日本語とし、術語は初出で注釈を付ける。固定の契約トークンは逐語で保つ。規則は `memory/`、参照資料は `knowledge/documents/`、横断知識は `knowledge/aidlc-shared/`、工程成果物は intent 記録へ置く。


### aidlc/spaces/default/memory/team.md

## Forbidden

<!-- Team-specific forbidden patterns -->


### aidlc/spaces/default/memory/team.md

## Mandated

<!-- Team-specific mandates -->


### aidlc/spaces/default/memory/team.md

## Corrections

<!-- Self-learning loop appends here. -->

### aidlc/spaces/default/memory/project.md

# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).


### aidlc/spaces/default/memory/project.md

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->


### aidlc/spaces/default/memory/project.md

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->


### aidlc/spaces/default/memory/project.md

## Testing Posture

<!-- Project-specific specialisation. -->


### aidlc/spaces/default/memory/project.md

## Deployment

<!-- Project-specific specialisation. -->


### aidlc/spaces/default/memory/project.md

## Code Style

<!-- Project-specific specialisation. -->


### aidlc/spaces/default/memory/project.md

## Tech Stack

<!-- Technology choices locked for this project. -->


### aidlc/spaces/default/memory/project.md

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->


### aidlc/spaces/default/memory/project.md

## Scope Overrides

<!-- Custom scope rules for this project. -->


### aidlc/spaces/default/memory/project.md

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER 既存コードを再設計・再文書化したり、削除済みの過去記録を現存する根拠として扱ったりする。（根拠: 基準と正本） (affirmed 2026-09-07)
- NEVER 配布ステージ類を独自に書き直す。（根拠: 基準と正本） (affirmed 2026-09-07)
- NEVER 上流仕様との不一致を独自に読み替えて実装を進める。（根拠: 進め方の規律） (affirmed 2026-09-07)
- NEVER AIの判断だけで GitHub Issue を起票する。（根拠: 進め方の規律） (affirmed 2026-09-07)
- NEVER リポジトリ直下に手書きの文書ツリーを新設する。（根拠: 基準と正本） (affirmed 2026-09-07)
- NEVER 今回の実装範囲に自律実行、センサー・プラグイン・他ハーネス、配布一般化、OTel、インストーラ、仕様12・13号の全文執筆、スモークで踏まない既存課題を追加する。（根拠: スコープ外） (affirmed 2026-09-07)

### aidlc/spaces/default/memory/project.md

## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

- ALWAYS 今回は独立した先行の最小通し実装を設けず、通常の作業単位で必須差分を依存順に実装する。（根拠: 確認事項Q1、内容確認） (affirmed 2026-09-07)
- ALWAYS 現行 `main` のコードを実測し、実装に必要な契約差分だけを設計成果物に記録する。（根拠: 基準と正本、進め方の規律） (affirmed 2026-09-07)
- ALWAYS 実装を TDD の red → green → refactor の順で進め、Quint・ITF・ゴールデンを外側の受入ゲートとする。（根拠: 進め方の規律） (affirmed 2026-09-07)
- ALWAYS 既存の90%カバレッジ床・相対ゲート・CIを維持する。（根拠: 現状の品質ゲート、完了条件、承認済み計画） (affirmed 2026-09-07)
- ALWAYS Boltを [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) 1本に対応させ、直列1本・squash-mergeで進め、CI成功と収束ルールをマージ条件にする。（根拠: 進め方の規律） (affirmed 2026-09-07)
- ALWAYS 上流仕様と現行コードの不一致を人間へ提示して裁定を求める。ゴールデンの版差は切替条件2を判定する前に裁定を受ける。（根拠: 基準と正本、進め方の規律） (affirmed 2026-09-07)
- ALWAYS 本リポジトリで `target/release/aidlc` による開始・質問・ゲート承認・完了の実地スモーク、同バイナリの自己診断成功、CI全ジョブ成功を切替の完了条件にする。（根拠: 目的と到達条件） (affirmed 2026-09-07)
- ALWAYS 切替後はホストを直近の安定タグ、ターゲットを開発版として2版運用する。（根拠: 目的と到達条件） (affirmed 2026-09-07)
- ALWAYS 配布元のステージ・エージェント・プロトコル・コンパイル済みグラフを再利用する。（根拠: 基準と正本） (affirmed 2026-09-07)
- ALWAYS 規則は `memory/`、参照資料は `knowledge/documents/`、横断知識は `knowledge/aidlc-shared/`、工程成果物は intent 記録に置き、参照資料は `knowledge onboard` で目録化する。（根拠: 基準と正本） (affirmed 2026-09-07)
- ALWAYS 会話と成果物を日本語で記述し、術語に初出の注釈を添える。（根拠: 進め方の規律） (affirmed 2026-09-07)

### aidlc/spaces/default/memory/project.md

## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->

### aidlc/spaces/default/memory/phases/construction.md

# Construction Phase Guardrails

These rules apply to every stage whose `phase: construction` declaration
imports them as the matching phase rule.


### aidlc/spaces/default/memory/phases/construction.md

## Code Completeness

- Generate complete, runnable files — no partial implementations, no placeholder stubs unless explicitly marked TODO with a rationale
- Every generated module must be independently executable or clearly document its dependencies
- Do not leave unresolved import errors, missing type definitions, or broken references


### aidlc/spaces/default/memory/phases/construction.md

## Error Handling

- Always include error handling at integration boundaries (API calls, database operations, file I/O, external services)
- Errors must be surfaced to the caller or logged — silent failures are not acceptable
- Distinguish between recoverable errors (retry/fallback) and fatal errors (fail fast)


### aidlc/spaces/default/memory/phases/construction.md

## Testing Standards

- Test files must cover the happy path and at least two error/edge cases
- Tests must be runnable without manual setup beyond documented prerequisites
- Do not generate tests that always pass regardless of implementation (e.g., `assert True`)


### aidlc/spaces/default/memory/phases/construction.md

## Security

- Never hardcode credentials, API keys, or secrets — use environment variables or a secrets manager
- Validate and sanitize all inputs at system boundaries
- Flag any code that bypasses authentication or authorization checks


### aidlc/spaces/default/memory/phases/construction.md

## Corrections


## 承認済み実装計画（全文）

# U1: 本家2.7.1の受入基盤の実装計画

## 対象と完了条件

本家固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `dist/claude/` を実行し、配布JSON・CLI・フック・正規化/ハッシュの比較材料を `tests/golden/upstream-a277af21/` に採取する。契約C1の来歴、入力、標準出力・標準エラー、終了コード、状態・監査の差分を追跡可能にする。

FR1のうち採取と比較基盤を担当する。U2が既存のRust実装・テスト参照・比較項目を2.7.1に揃えた後に、B1として統合する。U1の検査成功だけでFR1やセルフホストの達成とはしない。既存の2.6.40採取バイトを上書きしない。現行受入参照を残さない移行・不要資産の整理はU2で一括確認する。

会話で裁定済みの2.7.1を採用する。下のTesting Contractに含まれる過去の「採用版は未裁定」は採取時点の記録であり、再裁定事項ではない。CQS（更新と読取りの分離）の例外は設けず、Rustの報告イベント・RMU・クエリ変更はU2が担当する。

## 現コードで確認した差分

- `recapture-cli.sh:24–35` は2.6.40、配布262ファイル、旧マニフェストへ固定されている。
- `capture-cli.ts:172–183` は複数の保護を無効化して採取する。これだけで承認保護を検証したとは扱えない。
- `capture-cli.ts:292–310` は既存の正規化設定と採取元メタデータを入力とし、CLIと主要4フックを採る。主なケースはclassicの連続操作である。
- `capture-supplemental.ts:9–25` も旧ピン固定。複数部分の継続配送と会話中の停止制御などを補っている。
- 旧正規化はUUID形と長いbase64url文字列を一括置換する。C1が求める入力間の識別子対応を新基盤で検証する必要がある。
- `git show a277af21:dist/claude/.claude/tools/aidlc-version.ts` の版宣言は `2.7.1`。配布元forkの作業ツリーは採取元として流用しない。
- 今回は調査と計画のみ。採取・新規テスト・Rustの全件テストは未実施。

## テストする公開境界

Plan Approvalは次の3境界も確認対象とする。内部関数の呼出し順や実装の字面をテストしない。

| 境界 | 観測する契約 | テストファイル |
| --- | --- | --- |
| 採取入口 | 固定した配布物だけを受理し、異なる版・改変・欠落を拒否し、失敗時に既存の採取先を壊さない | `scripts/goldens/capture-source.test.ts` |
| 採取結果 | 入力・出力・状態/監査・前提条件・来歴を再生可能な形で保存し、プロセス失敗を取りこぼさない | `scripts/goldens/capture-corpus.test.ts` |
| コーパス検査・比較CLI | 同一条件の再採取を照合し、固定文言・ID対応・欠落・余分なファイル・不正な正規化を検出する | `scripts/goldens/compare-corpus.test.ts` |

Standardに従い各境界5–8件を目安に、下のテスト手順書の7件ずつを実装する。新しい業務層、DB層、UIはこのpackaging単位に存在しないため追加しない。実ファイルと子プロセスを使い、時刻・乱数・取得失敗等の外部境界だけ必要に応じて制御する。

## 実装順

各振る舞いを1件ずつ、Red（失敗確認）→Green（最小実装）→Refactor（成功を維持した整理）の順に完了する。全テストを先に書く方式は採用しない。

- [ ] **Step 1 — 実行基盤と開始時点の確認**（NFR2・NFR3・NFR4）
  - 実装前に既存Bun検査と `cargo test --workspace` の基準結果を取る。失敗があれば変更による失敗と区別する。
  - Bunの既存テストランナーを使用する。承認後、最初のテストファイルを用意して `bun test ./scripts/goldens/capture-source.test.ts` がテストを実行することを確認する。コマンド不在・テスト未検出はRedに数えない。
  - 既存作業を保持し、U1の変更範囲を下表へ限定する。Rustの本体と受入参照移行には着手しない。
- [ ] **Step 2 — 採取元の検証をTDDで実装**（FR1・NFR1・NFR2）
  - Red: 固定元の正常受理から始め、異なるコミット、ファイル改変/欠落/追加、取得失敗、出力先保護の各ケースを順番に追加する。
  - Green: 既存採取入口を2.7.1へ切り替え、固定コミットから配布ファイル一覧とSHA-256を実測して固定する。実行前に全体の一致を検証する。取得方法が違っても同じ検証を必須にする。
  - Refactor: CLI/フック・ハッシュ・補完採取で必要な固定情報と検証だけを共有する。任意の版を信用する汎用ダウンローダーには広げない。
- [ ] **Step 3 — 観測保存をTDDで実装**（FR1・NFR1・NFR2・NFR4）
  - Red: 採取された標準入出力・終了値・状態/監査・初期ファイル・環境・逐次入力が欠けるケースを公開採取入口で再現する。
  - Green: 検証済み配布物を隔離した一時ワークスペースへ置き、既存のCLI・4フック・hash-canonical・補完ケースを2.7.1で実行する。生の観測と比較用データを区別し、失敗やsignal、初期化失敗を成功ケースへ丸めない。
  - 外部の実セッション・認証情報を使わない。親のAIDLC設定が採取へ混入しない環境を作り、採用設定とテスト用の合成前提を来歴へ記録する。
  - Refactor: 重複するファイル保存・子プロセス処理を必要な範囲で整理する。
- [ ] **Step 4 — 比較と正規化をTDDで実装**（FR1・NFR1・NFR2）
  - Red: 固定文言の1バイト差、入力間のIDの取り違え、未知の差、ファイル欠落/追加、正規化の非対称を検出する試験を順番に追加する。
  - Green: `verify-corpus.ts` のCLIで採取来歴・必要ファイル・比較規則・再採取差分を検証する。終了値・標準出力/エラー・状態/監査の各面を比較し、キー集合だけの一致で成功としない。
  - 環境依存値は実測値に基づき対称に正規化する。識別子は役割ごとに対応関係を保ち、固定ID、固定コミット、監査語彙、状態値を消さない。継続トークンは返された値を次の入力に用い、別トークンを渡した拒否も別途検証する。
  - Refactor: 同一規則を両辺へ適用する責務を整理する。採取済み期待値を手で修正しない。
- [ ] **Step 5 — スモークで使う契約の採取をTDDで補う**（FR1・NFR1・NFR2・NFR4）
  - 対象は要求確認の実測表と契約C2・C3・C6・C7から決め、固定コミットの該当ファイル/行を一覧に引用する。
  - CLIの開始・継続・報告、decision/answer/link/review、内容確認・実装計画承認、主要4フックと実発火する追加フック、更新補助処理、診断の採用部分をケースへ結び付ける。
  - Red→Green→Refactorを繰り返し、保護を有効にした不正入力、回答前、古い受領、別セッション、対象/内容変更後の拒否を採取する。既存の保護無効ケースと別に分類する。
  - 合成会話/初期ファイルを用いた契約テストは実地スモーク成功に数えない。採れない必要ケースは理由・根拠・引継ぎ先を記録し、必要範囲が未検証ならU1完了にしない。Rust固有の投影障害等は本家の観測を捏造せずU2の内部契約テストへ渡す。
- [ ] **Step 6 — 独立した2回の再採取と全差分確認**（FR1・NFR1・NFR2）
  - クリーンな2つの出力先で採取し、配布JSONは完全バイト一致、決定的出力は完全一致、非決定値を含む出力はC1の明示規則適用後の一致を確認する。
  - 取得日時等の来歴差はフィールド単位で報告する。未知の差を一括除外しない。hash-canonicalの関数本体は固定元から抽出した実バイトと照合する。
  - 旧データとの差分を全数分類し、Rustの既存ゴールデン参照・キー集合比較・スキップの移行一覧をU2へ渡す。
- [ ] **Step 7 — CI接続とU1の引継ぎ**（FR1・NFR2・NFR3・NFR4）
  - 3つのBunテストを既存CIの検査対象へ追加する。通常CIでは保存コーパスを検査し、再採取時だけ固定元の取得を行う。既存検査・カバレッジ閾値は維持する。
  - 単位テストと採取検証を実行し、Red/Greenのコマンド・結果、未検証事項、U2への変更一覧を `code-summary.md` に残す。採取手順と根拠は同じ記録内に置く。
  - 全変更ファイルを `source-manifest.json`、FR1とNFR1–NFR4の対応を `traceability.json` に記録し、独立レビューを受ける。
  - U1単独ではmainへ統合しない。B1全体のカバレッジ・Quint/ITF・CI全ジョブ・レビュー収束はU2完了時にも確認する。

## 変更するファイルと責任

| 対象 | 変更内容 |
| --- | --- |
| `scripts/goldens/recapture-cli.sh`、`recapture-hash-canonical.sh` | ピン更新、検証、明示的な採取先指定 |
| `scripts/goldens/capture-cli.ts`、`capture-hash-canonical.ts`、`capture-supplemental.ts` | 2.7.1での採取、前提と全観測面の保存 |
| `scripts/goldens/upstream-source.ts` | 固定元情報と採取前検証の共有部分 |
| `scripts/goldens/capture-stage1.ts` | 実測したスモーク必須契約の追加採取 |
| `scripts/goldens/verify-corpus.ts` | 公開のコーパス検証・比較CLI |
| `scripts/goldens/capture-source.test.ts`、`capture-corpus.test.ts`、`compare-corpus.test.ts` | 上記3境界のテスト |
| `tests/golden/upstream-a277af21/` | 本家からの新規採取物、来歴、明示的な比較設定 |
| `.github/workflows/ci.yml` | U1検証を既存Bun検査へ追加 |
| 本単位の記録ディレクトリ | 実装/テスト手順、採取根拠、差分一覧、結果、変更ファイル一覧 |

本家・vendor・配布ステージ本文は編集しない。旧採取物は差分比較の入力として保全する。既存の自動検査修正と専用スコープはB1の前提変更で、U1の新規実装と混同しない。

## 品質基準と停止条件

workspace行カバレッジ床90.0%、相対条件 `head >= base - 0.01`、乱数シード20260823、既存除外1ファイルを維持する。U1のBun検査をRustカバレッジの代わりにしない。ネットワーク取得不能、必要な採取契約の矛盾、再現しない未知差、実装計画承認の拒否はそれぞれ明示し、成功扱いで先へ進まない。

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "各変更で失敗するテストを先に実行して red を確認し、最小の実装で green にしてから、テストの成功を維持しながら refactor する。",
  "scope": "selfhost-stage1",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 各変更で失敗するテストを先に実行して red を確認し、最小の実装で green にしてから、テストの成功を維持しながら refactor する。\n- 上記は依頼原文で確定済みであり、`org.md` の未確定時の `test-after` 既定は適用しない。\n- workspace の行カバレッジ床 90.0% とベース比較を維持する。現設定の相対許容は 0.01 パーセントポイント、乱数シードは `20260823`、除外は `modules/app/aidlc/src/main.rs` 1 ファイルである。相対条件は `head >= base - 0.01` であり、絶対床90.0%に許容誤差を適用しない。これらは実測設定として保持し、テストを通すために緩和しない。\n- Quint（状態遷移を検査する仕様言語）3モデル、ITF（モデルの実行トレースを実装で再生する形式）、ゴールデン（観測結果の比較用データ）を外側の受入検査とする。ゴールデンの採用版は未裁定である。\n- `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo lint`、`cargo test --workspace`、独立した `tools/lint` の検査を維持する。\n- `cargo audit` は依存ライブラリの脆弱性検査である。現在の CI 集約では必須対象外だが、今回の「CI 全ジョブ green」はこのジョブの成功も要求する。CI設定の変更が承認されたという意味ではない。\n- ゴールデンのバイト一致とキー集合の比較を区別し、未検証ケースと既知の出力差を要求分析へ渡す。既存検査の成功を完全互換の証明へ拡大解釈しない。具体的な限界は [evidence.md](evidence.md) に記録する。\n- 本工程ではテスト・カバレッジを再実行していない。既存設定の存在と、検査の成功を区別する。"
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
  "input_sha256": "sha256:b510bda44274b1fcf27f9154152da5867f09514c2073fcd78fe101cf98ea7d07",
  "contract_sha256": "sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55"
}
```

## Sources

- [要求書](../../../inception/requirements-analysis/requirements.md): FR1、NFR1–NFR4。
- [単位定義](../../../inception/units-generation/unit-of-work.md): U1の責任とU2への引継ぎ。
- [契約書](../../../inception/contract-design/contract-summary.md): C1、C2・C3・C6・C7の採取対象。
- [実行計画](../../../inception/delivery-planning/bolt-plan.md): B1=U1+U2、直列統合。
- `scripts/goldens/` の既存5ファイル、旧 `normalization.json`・来歴、固定コミットの版宣言。

## Assumptions & Open Questions

採取する公開境界と実装順は本計画の承認対象。採取ツリーの件数・SHA-256、2.7.1で変化した出力の一覧は実行で確定し、未測定値を記入しない。上流と契約の意味が食い違う場合は裁定を求める。



## 承認済みテスト手順（全文）

# U1のテスト実行手順

## 前提と起動

既存のBunテストランナーとGit、Bash、curl、tar、shasumを使う。新規テスト依存や別ランナーを追加しない。実装計画承認後、最初のテストファイルを作成して最初のコマンドがテストを検出・実行することを確認する。ファイル不在やコマンド未検出はTDDのRedではない。

現在は計画段階であり、下記の新規テストファイル・検証CLIはまだ存在しない。承認後のStep 1で実行可能にし、最初の振る舞いの失敗を確認してから実装する。

## この単位の実行コマンド

最初の採取元検証:
```bash
bun test ./scripts/goldens/capture-source.test.ts
```

採取結果保存:
```bash
bun test ./scripts/goldens/capture-corpus.test.ts
```

正規化・比較:
```bash
bun test ./scripts/goldens/compare-corpus.test.ts
```

U1全体:
```bash
bun test ./scripts/goldens/capture-source.test.ts ./scripts/goldens/capture-corpus.test.ts ./scripts/goldens/compare-corpus.test.ts
```

保存コーパスの検査（Step 4以降）:
```bash
bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21
```

再採取は承認後に更新する既存の2つの再採取スクリプトと追加採取入口を用いる。実際に使用した出力先・全引数・固定元・環境は `capture-evidence.md` に保存する。検証CLIの第2引数へ独立再採取のディレクトリを指定すると、最初のコーパスと比較する。通常の単位テストはネットワークへ依存させない。

## ケースと期待する検出力

各行を順番にRed→Green→Refactorで実装する。ケースのまとめ実装をしない。

| 境界 | 予定する7件 |
| --- | --- |
| 採取元 | 正しい固定元を受理／違うコミットを拒否／1バイトの改変を拒否／必要ファイル欠落を拒否／余分なファイルを拒否／取得失敗を明示／検証失敗時に既存出力を保全 |
| 採取結果 | 正常入出力と終了値／非ゼロ終了とstderr／状態差分と監査追記／初期ファイルと引数・環境の再現／初期化失敗やsignalの伝播／親環境混入防止／保護有効・無効と合成前提の来歴区別 |
| 比較 | 同一結果の一致／固定文言の1バイト差を検出／実測時刻・パスのみ対称正規化／異なるID対応を検出／ファイル欠落を検出／余分なファイルを検出／不正規則・未知の差を拒否 |

上記に加え、固定2.7.1からの実採取で主要CLI・4フック・hash-canonicalとスモーク必須経路を検証する。承認前・古い受領・別セッション・内容/対象変更後の拒否を含める。本家の初期化・入出力形式に合わせた具体ケースを引用と結び付け、ケース一覧へ反映する。Rust固有の永続化・投影障害はU2へ引き継ぐ。

## データと代替実装

公開の採取入口・比較CLIを通す。テスト用の一時ディレクトリと子プロセスを使い、内部のヘルパーや呼出し回数には依存しない。小さな既知のバイト列と期待値を独立に用意して比較器の検出力を検証し、期待値を比較器自身で作らない。

実採取は固定コミットの配布実体を使用する。合成された会話・初期状態、保護の無効化、時刻や乱数の制御はケース別に記録する。本リポジトリの本物の承認・セッションをテスト用に偽装しない。生の採取結果と比較用の正規化結果を区別し、原データの書換えを禁止する。

## 判定と品質目標

Standardの各境界5–8件、正常系と複数の拒否/境界ケースを満たす。意図した検査を1箇所壊すと失敗する対照を含める。2回の独立採取が比較規則の範囲で一致し、未知の差がゼロであることを確認する。

Rust workspaceの90%床、相対ゲート、既存Quint/ITF、CI全ジョブは共通条件として維持する。その実行入口は承認済み実行計画を参照し、本手順のU1限定コマンドと区別する。U1の成功は採取基盤の成功であり、Rustの2.7.1適合と実地スモーク完了の証拠ではない。

Red/Greenごとのコマンド・失敗理由・終了結果と、再採取・比較の結果を同じ単位の `code-summary.md` / `capture-evidence.md` に記録する。



## ステージ本文（全文）

---
slug: code-generation
phase: construction
execution: ALWAYS
condition: Always executes for every unit in the execution plan.
lead_agent: aidlc-developer-agent
support_agents: []
mode: subagent
reviewer: aidlc-architecture-reviewer-agent
review_artifact: code-generation-plan
reviewer_max_iterations: 2
for_each: unit-of-work
workspace_requires: true
produces:
  - code-generation-plan
  - unit-test-instructions
  - code-summary
  - traceability
consumes:
  - artifact: functional-spec
    required: false
  - artifact: rules
    required: false
  - artifact: entities
    required: false
  - artifact: contract-summary
    required: false
  - artifact: performance-design
    required: false
  - artifact: security-design
    required: false
  - artifact: infrastructure-specification
    required: false
  - artifact: unit-of-work
    required: true
  - artifact: requirements
    required: true
requires_stage:
  - units-generation
  - functional-design
  - nfr-requirements
  - nfr-design
  - infrastructure-design
sensors:
  - required-sections
  - linter
  - type-check
  - traceability
scopes:
  - enterprise
  - feature
  - mvp
  - poc
  - bugfix
  - refactor
  - security-patch
  - classic
  - workshop
  - express
inputs: ALL prior design artifacts for this unit
outputs: application code + code-generation-plan.md, code-generation-questions.md, unit-test-instructions.md, code-summary.md, traceability.json (under this stage's per-unit record dir, engine-resolved)
---

# Code Generation

## Steps

### Critical Rules

- Application code goes to workspace root, NEVER to the record dir
- Brownfield: modify files in-place. NEVER create duplicates like ClassName_modified.java
- Add data-testid attributes to interactive UI elements for test automation
- Before review, write `source-manifest.json` listing every application-source path this unit created, modified, or deleted, including shell-, scaffolding-, and generator-written files
- Measurable quality targets from NFR Requirements, NFR Design, and the Testing
  Contract coverage floor are inputs, not suggestions. NEVER relax, lower, or
  disable a defined target, including threshold settings in test or build
  configuration, to make a step pass; surface the gap instead.

### Step 1: Read All Unit Artifacts

Read all design artifacts for the current unit:
- Functional design from `<record>/construction/{unit-name}/functional-design/` (if exists)
- NFR requirements from `<record>/construction/{unit-name}/nfr-requirements/` (if exists)
- NFR design from `<record>/construction/{unit-name}/nfr-design/` (if exists)
- Infrastructure design from `<record>/construction/{unit-name}/infrastructure-design/` (if exists)
- Domain design (component catalogue) from `<record>/inception/domain-design/components.md` (if exists)
- Contracts from `<record>/inception/contract-design/contract-summary.md` (if exists)
- Unit definition from `<record>/inception/units-generation/unit-of-work.md` (if exists)
- Story map from `<record>/inception/units-generation/unit-of-work-story-map.md` (if exists)
- Requirements from `<record>/inception/requirements-analysis/requirements.md` (if exists)

Incremental scopes (bugfix, poc, refactor, security-patch) and the zero-Unit
`express` scope skip Units Generation by design. When those inputs are absent,
scope the work from Requirements Analysis and the workspace; on brownfield, also
use the reverse-engineered code knowledge base at
`aidlc/spaces/<active-space>/codekb/<repo>/`. Never invent the content of a
missing artifact.

For a zero-Unit directive (`directive.unit` absent and no Unit DAG), run exactly
one implementation iteration and write this stage's artifacts under
`<record>/construction/code-generation/` with no synthetic Unit segment. This is
ordinary stage work: no Bolt, walking-skeleton, ladder, per-Unit receipt, or
swarm ceremony applies.

For every later path in this stage, set `<code-generation-record>` from the
directive exactly once:

- `directive.unit` present:
  `<record>/construction/<directive.unit>/code-generation/`
- `directive.unit` absent:
  `<record>/construction/code-generation/`

### Step 2: PART 1 — Planning

Create a detailed code generation plan at
`<code-generation-record>/code-generation-plan.md` with checkboxes for each
implementation step. Include story-to-code-step traceability — map each plan
step back to the user story it implements.

Plan should cover (as applicable to the unit):
- [ ] Business logic implementation
- [ ] API/endpoint layer
- [ ] Repository/data access layer
- [ ] Database migrations/schema changes
- [ ] Unit tests
- [ ] Integration tests
- [ ] Configuration files
- [ ] Documentation (inline and API docs)
- [ ] Deployment artifacts (Dockerfiles, IaC)

**Test files are MANDATORY in the plan.** Consult the active test strategy (stage-protocol.md §8 "Test Strategy") to determine test scope and volume:
- **Minimal strategy**: Requirement-driven tests (1 per requirement, happy-path unit floor per component); unit tests are the default, but a `bugfix` / `security-patch` targeted regression uses the narrowest level that reproduces the defect
- **Standard strategy**: Unit test files per component (5-8 tests each) + integration test stubs for key boundaries
- **Comprehensive strategy**: Unit + integration + E2E test files per component (10-15 tests each)

Apply the active scope's floor additively:
- `mvp`, `enterprise`, `feature`, `infra`: the selected strategy plus 80% line coverage and CI execution before merge.
- `bugfix`, `security-patch`: the selected strategy plus a targeted regression for the bug/vulnerability at the narrowest level that reproduces it, even when that adds one integration/E2E test beyond Minimal's unit-test default; the existing suite remains green.
- `poc`, `refactor`, `workshop`: the selected strategy still applies; the scope adds no extra new-test floor, and the existing suite remains green.

The selected strategy and scope floor are both obligations. Neither replaces the other.

The plan MUST include steps for:
- [ ] Test files appropriate to the active test strategy
- [ ] Test configuration (vitest.config, jest.config, or equivalent)

If the plan presented to the user omits test file steps, add them before presenting. Tests are not deferred to Build and Test — that stage verifies and extends, not creates from scratch.

**Test ordering follows one deterministic Testing Contract.** Run:

```bash
bun .codex/tools/aidlc-testing-posture.ts render
```

Paste the command's complete `## Testing Contract` JSON block into `code-generation-plan.md` unchanged. The resolver reads all `## Testing Posture` sections additively and selects the narrowest explicit methodology/order statement; coverage, tooling, integration, or scope notes remain applicable but cannot erase a broader methodology. A contradictory narrower methodology is an error, not an override: halt and ask for the memory rule to be revised.

Use the contract's `plan_profile.steps` as the required ordering baseline, adapting names and omitting genuinely inapplicable layers without changing the methodology:
- **TDD**: for every applicable testable layer — data-model/database behavior, repository/data access, business logic, API/endpoint, and frontend behavior — plan Red (failing tests), Green (minimal implementation), then Refactor while green.
- **BDD**: define executable behavior/scenario examples before each observable feature slice, implement that slice across every required layer, run scenarios green, then refactor. Do not turn BDD into layer-local TDD.
- **ATDD**: write executable acceptance tests before the complete cross-layer feature implementation, implement against that acceptance contract, run acceptance green, then refactor. Do not split acceptance intent into unrelated per-layer Red steps.
- **Custom/mixed**: preserve the contract's exact `ordering` text, such as scenario-first BDD with lower-level unit tests after implementation. Never coerce a mixed posture into TDD.
- **Test-after**: for every applicable testable layer, implement the layer and then write/run that layer's tests.

The contract always puts test-runner readiness before the first executable test step. On greenfield work, bootstrap the minimal runner/configuration and dependency needed to execute the exact unit-scoped command before the first TDD Red, BDD scenario, or ATDD acceptance step. On brownfield work, verify that command before the first test-first step. Record the exact command in `unit-test-instructions.md`; a Red/Green step is invalid if no runnable command exists.

Number each plan step sequentially (Step 1, Step 2, etc.) for clear execution ordering and traceability. Preserve dependency ordering inside the selected methodology, and deviate only when the architecture requires it (for example, event-driven systems or independently deployable services).

Also create
`<code-generation-record>/unit-test-instructions.md`
before Plan Approval. Consult the active test strategy (stage-protocol.md §8
"Test Strategy") and use the matching unit-test scope:

- **Minimal strategy**: Requirement-driven unit tests (1 test per requirement,
  happy-path floor per component), approximately 5-15 tests total
- **Standard strategy**: 5-8 tests per component, with key behavior coverage
- **Comprehensive strategy**: 10-15 tests per component, with thorough coverage

Scope floors remain additive here: a Minimal `bugfix` / `security-patch` still
includes its targeted regression at the narrowest level that reproduces the
defect.

Include:
- Test framework setup and configuration
- How to run THIS UNIT's tests, including the exact command that is runnable before the first test-first cycle
- Expected coverage targets
- Mocking/stubbing guidance
- Test data management

Every run command in this file MUST be scoped to this unit only, using exact
test file paths or an exact unit filter. A bare project-wide command like
`npm test` is not acceptable. Build and Test executes every unit's commands,
so an unscoped command would rerun the whole suite once per unit.

Present a summary of the unit test instructions together with the plan summary
to the user.

### Step 3: Plan Approval

Before presenting the approval, create or update
`<code-generation-record>/code-generation-questions.md`
with a **Plan Approval** question that covers both
`code-generation-plan.md`, its embedded Testing Contract, and
`unit-test-instructions.md`. For a revision, reset the existing Plan Approval
`[Answer]:` to blank before regenerating anything. After both files are final,
run:

Run the unit-bound form when `directive.unit` is present:

```bash
bun .codex/tools/aidlc-testing-posture.ts fingerprint --unit "<directive.unit>"
```

For a zero-Unit directive, use the explicit `--stage-level` target; the tool then resolves the stage-level
`<record>/construction/code-generation/` evidence:

```bash
bun .codex/tools/aidlc-testing-posture.ts fingerprint --stage-level
```

Write the returned hash into the Plan Approval section as
`[Approval Fingerprint]: sha256:<hash>`, followed by both options below and a
blank `[Answer]:` tag:

- "Approve Plan" — proceed to code generation
- "Request Changes" — revise the plan

When the active directive carries `legacy_plan_approval_choices`, those two
nonce-labelled values are the presentation-only choices for legacy Kiro IDE.
Present them exactly and never write them into the questions file, audit, plan,
instructions, or any other shared artifact. Map the selected protected label
back to canonical `Approve Plan` or `Request Changes` for `[Answer]:`,
`--details`, and all subsequent lifecycle logic.

Before presenting, record the exact prompt identity:

```bash
bun .codex/tools/aidlc-log.ts decision --stage code-generation \
  --checkpoint plan-approval \
  --session "<Runtime Session from SessionStart context>" \
  --questions-file "<code-generation-record>/code-generation-questions.md" \
  --decision "Approve this exact Code Generation plan?" \
  --options "Approve Plan,Request Changes" \
  --unit "<directive.unit>"
```

For zero-Unit work replace `--unit "<directive.unit>"` with `--stage-level`.
Then present the structured question and STOP the turn. Fill `[Answer]:` only
after the human explicitly responds, using the exact unlettered choice
`Approve Plan` or `Request Changes`, then immediately run the matching receipt:

```bash
bun .codex/tools/aidlc-log.ts answer --stage code-generation \
  --checkpoint plan-approval \
  --session "<same Runtime Session>" \
  --questions-file "<code-generation-record>/code-generation-questions.md" \
  --details "<exact choice>" \
  --unit "<directive.unit>"
```

Again use `--stage-level` instead of `--unit` for zero-Unit work. The markdown
answer and `PLAN_APPROVAL_RECORDED` audit row are context/provenance only.
Generation remains blocked until this command consumes the protected
session-bound challenge/response and writes its runtime receipt under
`aidlc/.aidlc-sessions/`. A conductor-authored answer or forged audit row cannot
create that authority.

On "Request Changes", record that choice through the same answer command, revise
the plan and unit test instructions as needed, reset `[Answer]:` to blank,
regenerate the Testing Contract and fingerprint, record a fresh decision, and
present the question again. Any post-approval file, testing-posture, scope,
strategy, project-type, active-target, stage-attempt, or directive reissue
invalidates the fingerprint/receipt and reopens Plan Approval. Do not begin Step
4, dispatch the developer agent, or infer approval from a forwarding-loop
continuation. Only the matching durable receipt authorizes generation.

> **Build-and-Test loop-back:** The construction protocol module
> (`aidlc-common/protocols/stage-protocol-construction.md`) defines this replay.
> A jump/reissued directive changes the Plan
> Approval authority epoch. Preserve the Loop-Back Log, but reset the Plan
> Approval `[Answer]:`, regenerate the fingerprint under the replayed
> code-generation directive, and run the full decision/human-turn/answer receipt
> sequence again. The earlier "Retry with fix" choice authorizes the jump; it
> does not mint approval for plan bytes or a directive the human has not yet
> reviewed.

### Step 4: PART 2 — Generation

Before delegating, display to the user:
"Generating code for [N] plan steps. This may take several minutes depending on project complexity. I'll show a summary when complete."

Delegate to Task tool with subagent_type="aidlc-developer-agent".

The aidlc-developer-agent persona and its knowledge are loaded automatically by the named agent. Do NOT manually inject the persona in the prompt.

Include in the delegation prompt:
- As the first line, the exact target marker. Use
  `AIDLC-UNIT: <directive.unit>` when `directive.unit` is present. For a
  zero-Unit directive use `AIDLC-STAGE: code-generation`. This marker identifies
  the one approval authority whose plan authorizes the dispatch; do not repeat
  either marker for contextual dependencies.
- As the second line, `AIDLC-TESTING-CONTRACT: <contract_sha256>` copied from
  the approved plan's Testing Contract. The plan-approval guard rejects a
  missing, different, or stale hash.
- Design artifacts for the CURRENT UNIT ONLY (not all units)
- A 1-2 line summary of each inception-phase artifact with its file path (requirements summary, stories summary, app design summary) — the subagent can Read specific files if it needs full content
- The approved code-generation-plan.md (full content)
- The approved unit-test-instructions.md (full content)
- Project workspace details (languages, frameworks, conventions from aidlc-state.md)
- Instructions to execute each plan step sequentially and mark checkboxes as completed
- The instruction that the approved Testing Contract embedded in the plan is
  authoritative for Part 2. The subagent must not independently re-resolve or
  reinterpret memory. TDD records each Red command's failing output before
  Green; BDD and ATDD follow their scenario/acceptance-first cross-layer
  profiles; custom/mixed follows the exact approved ordering.
- The instruction that measurable quality targets from NFR Requirements, NFR
  Design, and the Testing Contract coverage floor are inputs, not suggestions.
  The subagent must NEVER relax, lower, or disable a defined target, including
  threshold settings in test or build configuration, to make a step pass; it
  must surface the gap instead.

The subagent generates all code, test files, and configuration artifacts in the workspace.

### Step 5: Generate Code Summary

After subagent completes, create `<code-generation-record>/code-summary.md`
documenting:
- Files created/modified
- Key implementation decisions
- Test coverage summary
- Any deviations from the plan

Create `<record>/construction/{unit-name}/code-generation/source-manifest.json`
with this strict schema:

```json
{
  "stage": "code-generation",
  "unit": "u1-auth",
  "version": 1,
  "writes": [
    { "path": "src/auth/login.ts" },
    { "path": "src/auth/generated/" },
    { "repo": "repo-a", "path": "src/api/routes.ts" }
  ]
}
```

List every application-source path this unit created, modified, or deleted,
including files written by shell commands, scaffolding, or generators. Use a
trailing `/` directory claim for generated trees. In the main workspace,
multi-repo entries name their recorded `repo`; inside the worktree hosting the Bolt, paths are
relative to its single selected repo and MUST omit `repo`. The engine refuses
to record the unit review without this manifest, and unclaimed changed paths
block stage completion.

Create
`<code-generation-record>/traceability.json`.
Enumerate every assigned AC, detailed `NFRx.y`, and `BRx.y` (or direct `FR` /
`NFR` IDs when incremental scope skipped the design chain). Every `OK` target
must be one existing workspace-relative implementation or test file:

```json
{
  "stage": "code-generation",
  "unit": "u1-auth",
  "upstream_ids": ["AC1.1.1", "NFR1.1", "BR1.1"],
  "coverage": [
    { "id": "AC1.1.1", "status": "OK", "target": "src/auth/login.ts" },
    { "id": "NFR1.1", "status": "OK", "target": "src/cache/redis.ts" },
    { "id": "BR1.1", "status": "OK", "target": "src/auth/policy.ts" }
  ]
}
```

### Step 6: Completion Handoff

Hand completion to `stage-protocol.md` via
`bun .codex/tools/aidlc-orchestrate.ts report --stage code-generation --result <outcome>`.
That `report` call owns every lifecycle transition and advancement; never perform one in prose, and never narrate this bookkeeping to the user.

### Step 7: Completion

Present completion message and approval gate:

```
# :computer: Code Generation Complete — {unit-name}
```

Summary of code produced (files, tests, key decisions), then:

```
**Review:** `<code-generation-record>/`
```

Approval gate: strictly 2-option (Approve / Request Changes).

> **Note — orchestrator-managed completion gating.** Step 3 Plan Approval is a mandatory hard stop in every execution mode, including during Construction, except for the explicit Build-and-Test loop-back replay carve-out above: generation must never begin before the human chooses "Approve Plan", and the carve-out reuses that preserved approval rather than inferring a new one. Only the Step 7 completion approval gate is suppressed by the orchestrator during normal Construction. On the default stage-major walk a single stage-level gate covers every Unit after the last Unit settles. Under an autonomous swarm the engine presents that Code Generation stage gate only after the final DAG batch has converged (intermediate batches merge without a gate). The completion gate still exists here for direct-invocation use (e.g., `/aidlc --stage code-generation` re-running a single Unit), and subagents invoked via Task must NOT invoke that completion gate themselves — the orchestrator owns completion-gate presentation.

## Sensors

This stage produces TypeScript/JavaScript code in the active Bolt
worktree. Generated code lives at the workspace root (NEVER under
the record dir); the planning, plan-approval, and summary artefacts
(`code-generation-plan.md`, `code-generation-questions.md`,
`unit-test-instructions.md`, `code-summary.md`) live under
`<code-generation-record>/`.

Imports: `required-sections`, `linter`, `type-check`, `traceability`.

`required-sections` checks each planning and summary artefact for at least two
H2 headings. `linter` and `type-check` run against matching generated code,
and `traceability` verifies the per-Unit coverage table and every `OK` target.

`upstream-coverage` is intentionally NOT imported because the stage consumes a
broad, scope-dependent design set. `source-manifest.json` is
engine-validated against its strict schema and source binding, while
`traceability.json` is owned by the `traceability` sensor; neither structured
file is subject to the `required-sections` floor.

## Learn

Follow stage-protocol.md §13: maintain `<record>/<phase>/<stage>/memory.md`
under the four standard headings while working; before the approval gate,
surface candidates with `aidlc-learnings.ts`;
still ask the mandatory "Anything to add for next time?" question, and persist confirmed selections
with the tool. The memory file stays in the artefact directory, and the stage
file remains immutable.


