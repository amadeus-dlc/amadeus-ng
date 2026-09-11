# U2実装への引継ぎ

Conversation language: 日本語

## 許可と担当

U2のPlan Approvalは人間の番号1を意味上のApprove Planへ対応付け、正規answerでPLAN_APPROVAL_RECORDEDを記録済み。begin --unit u2-workflow-authorityもstatus:generationで成功。next再発行でこの許可を失効させない。

あなたがU2の実装担当。所有範囲は承認済み計画に示すRustの対象操作・型・テスト・必要な設定とU2成果記録。親は同じソースを編集せず、検証準備とレビュー・統合の記録を担当する。単独作業ではないため既存の他者変更は保持する。U1の受入基盤と前提修正が未コミットで存在する。RustコードはU1で変更していない。

子エージェントは禁止。1振る舞いずつRed→Green→Refactorで9ステップを実施する。各変更前の失敗コマンド出力を残す。承認済み計画本文・Testing Contract・unit-test-instructionsを勝手に変更しない。完了したステップのチェックだけを更新する。独立レビュー部分は親が行うため、その未実施を成功にしない。

CQSには例外を認めない。コマンドユースケースの成功はResult<(), E>。集約は単一イベントを作り、RepositoryがSQLiteへ追記、RMUがreadモデルを構築、QueryはそのモデルをIDで返す。報告識別子は呼出側が持つ。最新行やCommand戻り値で結果を代用しない。Queryはドメイン非依存、業務判断を持たない。型・命名・配置は先にcoding-rulesと該当設計スキルを読む。先に触れた設計スキル実体は /Users/j5ik2o/.claude/plugins/cache/ai-tools/software-design/1.0.0/skills/ にある（j5ik2o:clean-architecture, j5ik2o:ddd-repository-design, j5ik2o:ddd-repository-placement, j5ik2o:ddd-domain-building-blocks）。必要なSKILL.mdとreferencesを読む。

上流と契約・実コードの意味が食い違い裁定が必要なら、具体的な入力・両方の観測・出典で親へ相談し、それ以外の独立作業は続ける。未実装のまま成功としない。状態・監査・承認は正式ツール以外で編集しない。実テスト内の合成入力と本リポジトリの人間承認を混同しない。

9ステップ完了時はsource-manifest.json、traceability.json、code-summary.mdとRed/Green証跡を整え、全編集を止めて親へ報告する。レビュー呼出し・ライフサイクル完了・commit/push/merge/tagは親が担当する。Step 2/3のCQS経路が通った時点と、以後の大きな節目で親へ進捗を知らせる。

## 基準と入力

- HEAD/main/origin/mainのローカル記録は1dc727e00a26c27258d916cb3a5e0592c6c48c1c。U1開始前cargo test --workspaceは2354件成功（/tmp/amadeus-stage1-cargo-baseline.log）。
- U1はUNIT_COMPLETED、レビュー第2回READY、R-01〜R-03すべてResolved。77テストの独立検証、固定元277ファイル、345保存ファイル、142プロセス観測と32hashケース。保存コーパスは tests/golden/upstream-a277af21/。
- U1成果は aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u1-upstream-acceptance/code-generation/ 内の capture-evidence.md、stage1-case-inventory.md、legacy-consumer-inventory.md、legacy-differences.json/patch。これらを入口にコードと観測を確認する。
- 上流の固定展開先 /tmp/amadeus-u1-upstream/dist/claude/ は検証済み。必要なら固定git objectから再取得・検証する。配布元fork作業ツリーと混同しない。
- requirements.md（同record/inception/requirements-analysis）: FR1–FR4、NFR1–4がU2担当。2.7.1適合、Rust正本、必要な経路だけ。
- contract-summary.md（同record/inception/contract-design）: C1–C6、C7のHEALTH_CHECKED更新契約。全文を読む。
- unit-of-work.mdとbolt-plan.md（同record/inception各工程）: B1=U1+U2、B2=U3、B3=U4の直列統合。U2にdoctor検査本体や配布接続を吸収しない。

U1のsource-manifestで既にレビューされたファイルを更新する必要があれば親へ通知する。後続変更によるU1レビューの失効を隠さず、最終的に必要な再確認をする。U1コーパスの期待バイトを実装へ合わせて変更しない。

## 適用する規則束

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

# U2: 作業進行・監査・承認の実装計画

## 対象と完了条件

FR1–FR4とNFR1–NFR4を、承認済み契約C1–C6、およびC7の診断実施記録に沿って実装する。U1が採取した本家2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の観測を比較基準にする。既存のRust構成、SQLiteイベントストア、RMU（イベントから読取りモデルを構築する処理）を使う。

U1の独立レビュー第2回はREADY、R-01〜R-03はResolvedで、UNIT_COMPLETEDを記録済み。U2では実装と受入検査を2.7.1へ移行する。B1はU1とU2をまとめた [Pull Request](https://github.com/amadeus-dlc/amadeus-ng/pulls) とし、全CIとレビュー収束後にmainへsquash-mergeする。今回の実装計画承認は、未検証の統合やセルフホスト切替を承認済みとするものではない。

## 現コードで確かめた差分

- HEAD、ローカルmain、origin/mainの記録はいずれも `1dc727e00a26c27258d916cb3a5e0592c6c48c1c`。U1ではRustを変更しておらず、開始時のworkspace検査2,354件は成功している。
- `CommitVerdictUseCase::execute` は `Result<CommitOutcome, CommitError>` を返す。no-opは保存せず戻り値を返し、成功した遷移の表示材料も返す。これを正当化するコメントが残っている。
- `modules/app/aidlc/src/runtime.rs:313–329` はその戻り値を保持し、投影後に `committed_directive` へ渡す。報告識別子に対応する結果のクエリはない。
- `IntentExecutionEvent` は現在16変種で、Reportedはない。集約の `report_dispatch` と `apply_report` が判断と遷移の既存境界である。
- クエリ側にはDAOとViewの独立したクレートがあり、`read_next_answer` 等を読む。報告結果の読取りモデルをこの構成へ追加する。
- `cli/request.rs` のdecision/answer/linkはLogNotWired。parse_nextには必要な入口観測の不足がある。harness-claudeとharness-infrastructureはまだ実装本体を持たない。
- U1の142プロセス観測には、Brownfieldのbugfix 9段開始、受領・保護・補助操作、診断20シナリオがある。採取と実際のRust適合は区別する。

## 報告処理の責任と順序

CQS（更新と読取りの分離）の例外は設けない。

1. 呼出側で報告識別子を用意し、実行識別子・報告要求とともに更新ユースケースへ渡す。外部CLIへreport-idフラグを追加しない。
2. 集約が受理・拒否・遷移・no-opを判断し、報告事実を単一イベントとして生成する。集約全体や輸送用封筒のコピーを結果にしない。
3. Repositoryが既存SQLiteストアへ追記する。更新ユースケースの成功は `Result<(), E>`。表示材料を返す別経路を残さない。
4. RMUが報告事実と遷移を投影し、report_idを自然キーとする結果行とチェックポイントを整合して更新する。
5. クエリAPIがreport_idで結果Viewを取得し、出力側が2.7.1の文言へ描画する。クエリ側はドメインへ依存せず、現在状態から報告結果を判断し直さない。

適用段階・適用操作・no-op理由、表示に必要な報告時のscope等を事実として保持する。後続の報告で現在状態が変わっても、別の報告結果と取り違えない。成功するno-opも内部の報告事実を追記するが、本家にない公開監査イベントや遷移を作らない。拒否は型付きエラーで返す。既存の1回だけの楽観競合再試行では、report_idと最初の対象を保持する。

## テストする公開境界

| 境界 | 検証すること | 主なテスト |
| --- | --- | --- |
| 集約・更新ユースケース | 報告の単一イベント、拒否、no-op、競合時の対象固定、成功戻り値がunit | 既存commit_verdict_use_case内テスト、ドメインの報告テスト |
| Repository → RMU → Query | 保存した報告をIDで取得、取り違え防止、投影失敗/再開、結果欠落の明示 | 新規report_result_contract / report_result_projection_contract / report_result_dao_contract |
| CLI | 開始・next/continue/report、質問/回答/引継ぎ/レビュー、実装計画承認 | 新規stage1_authority_contract / upstream_271_contract、既存CLI境界テスト |
| Claudeフック | JSON封筒、正常・拒否・無関係入力、実際の人間応答との対応 | 新規harness-claudeのhook_contract |
| 補助更新と投影ファイル | session・保存・規則・診断記録の更新、状態/監査の逐語、非適用時の無変更 | 上記CLI/フック契約と既存projection/publication契約 |

Standardの各コンポーネント5–8件を基準にする。主要4フックはそれぞれ正常・不正入力・適用外・状態/セッション境界を検証する。受領・永続化等で契約が求める追加ケースも含め、件数を合わせるための同義テストは作らない。実SQLiteと公開API/プロセスを使い、実装内部の呼出し回数だけを検査しない。

## 実装手順

各項目内の振る舞いを1件ずつRed（失敗の実行確認）→Green（最小実装）→Refactor（成功を維持した整理）で進める。全テストを先に書いてから実装する方式は採らない。

報告イベントが保存DTOとRMUへの入力契約になるため、Step 2でその振る舞いを固定し、Step 3で保存・投影を接続する。これは既存のイベントソーシングの依存順によるもので、テスト先行の順序を変えない。

- [ ] **Step 1 — 入力と実行基盤の確認**（FR1、NFR2–NFR4）
  - U1の採取証跡・ケース引用・旧参照一覧を読み、必要操作と既存入口を対応付ける。コードを変える前に既存のreportテストを実行し、単位限定のランナーを確認する。
  - 現在のRust検査基準とB1統合時のmain差分を確認する。新テストファイルを追加したときは、未検出・ビルド環境不足をRedに数えない。
  - データベース・Repository・ドメイン・APIの既存配置と依存方向を維持し、対象機能がない層を形式のために増やさない。
- [ ] **Step 2 — 報告事実をTDDで実装**（FR1・FR2、NFR1・NFR2）
  - Red: 成功する報告と3種類のno-opについて、呼出側が指定した報告識別子に対応する単一の報告イベントが得られることを検証する。拒否では成功結果を作らない。
  - Green: 既存集約の報告判断・適用へ必要な差分を加え、Reportedの事実を生成する。報告による遷移と結果を同じ保存単位にする。
  - Red→Greenを繰り返し、楽観競合の有限再試行と対象固定を検証する。競合相手が前進しても次の段階へ誤って報告しない。
  - Refactor: CommitOutcomeの公開成功戻り値と例外の正当化コメントを削除する。別名や旧APIを並立させず、呼出側を合わせて変更する。
- [ ] **Step 3 — 保存・投影・結果クエリをTDDで接続**（FR1・FR2、NFR1・NFR2）
  - Repositoryの書込DTOとRMUの読込DTOをそれぞれ追加し、ドメインへserdeやSQLを持ち込まない。未知/破損した事実を成功に丸めない。
  - Red: SQLiteへ追記した報告を投影してIDで取得する契約、未取得、異なるreport_id、再投影、途中失敗と復旧を検証する。
  - Green: 報告結果のread表、行モデル、Query側の独立View/DAO/取得ユースケースを追加する。結果とチェックポイントを同時に確定し、RMU以外が読取りモデルを構築しない。
  - 出力側を、更新成功→RMU→report_idのクエリ→逐語描画へ切り替える。投影/取得の失敗時に旧戻り値や「最新の結果」で代用しない。
  - Refactor: 既存publication/crash-recovery契約を保ち、報告結果のために別ストアや別の状態正本を作らない。
- [ ] **Step 4 — 作業開始と進行のCLIを2.7.1へ合わせる**（FR1・FR2、NFR1・NFR2）
  - Red: U1の同じ入力・初期ファイル・環境を使い、開始・next/continue/reportの値と状態/監査の違いを検出する。
  - Green: 必要なCLI観測を接続する。ワークスペース走査は本リポジトリのBrownfield判定・bugfix 9段の開始に必要な実走査を行い、空の既定値で成功させない。
  - 継続トークンを不透明な入力として渡し、古い/不正/別対象のトークンを拒否する。narrationやconductor_persona等の対象出力を、既知欠落を許す検査のまま残さない。
  - Refactor: 必要な分岐と文言を既存の入力・出力境界へ置く。Kiro専用や自律実行の未接続分岐を一律に実装しない。
- [ ] **Step 5 — 質問・回答・承認受領をTDDで実装**（FR2・FR3・FR4、NFR1・NFR2）
  - decision/answer/link/review、内容確認、実装計画承認の必要な入力・受理・拒否を接続する。
  - 実際の人間応答の記録と、CLIが意味上の回答を保存する操作を分離する。CLIや監査行の手書きだけで実行許可が発行されないようにする。
  - Red→Greenで回答前・別セッション・古い質問・対象/内容変更・受領再利用・レビュー確定後の変更を検証する。状態を使う判断は集約へ置き、Queryで再判定しない。
  - 番号回答は、利用者が是正を指示した入力保持と既存の意味への変換を保つ。承認対象・セッション・真正な応答の照合は弱めない。
  - Refactor: 受領を使う入口を同じRustの更新経路へ統一し、TypeScriptへの更新フォールバックを作らない。
- [ ] **Step 6 — 主要4フックをTDDで実装**（FR2・FR3、NFR1・NFR2）
  - 停止制御、人間応答記録、状態遷移保護、ファイル保存監査について、それぞれ本家の正常・拒否・無視を先に失敗させてから実装する。
  - `harness-claude`がJSON入力/出力とClaude固有の接続を、`harness-infrastructure`が必要な汎用機構を扱う。状態・監査・許可の更新はコマンド→イベント→SQLite→RMUを通す。
  - バイナリに `aidlc hook <name>` の接続入口を設け、stdinの封筒とフック別のstdout/stderr/exitを保持する。配布設定へ実際に結び付ける工程はU4。
  - 正当な質問・承認待ち、停止フックの再入、進捗なし反復、不正JSON、無関係なツール入力を区別する。不要な初期化を起こさない。
- [ ] **Step 7 — 必須の補助更新をTDDで実装**（FR2–FR4、NFR1・NFR2）
  - U1の実測一覧に沿ってsession-start/end、subagent完了、規則受渡し、review-freeze、通常のreviewer-scope、TaskUpdateによる同期、runtime-graph再構築を接続する。PreCompactは発火条件に対応する範囲を検証する。
  - learningsの表示とpersistを区別し、空選択・追加・重複抑止・不正選択を検証する。規則/監査を更新するpersistはRustへ接続する。
  - U3が使う診断実施の記録コマンドを用意し、既存監査がある場合のHEALTH_CHECKED投影と失敗を検証する。doctorの検査・表示そのものはU3。
  - fold-usageは通常の配布登録があるが、U1では無効時だけを採取している。必要性と副作用を実コードで確認し、正本更新に必要な経路なら実装対象に含める。未検証の有効経路を成功扱いしない。表示候補の再利用判断はU4へ渡す。
  - センサーやプラグインの製品実装を追加しない。新しい上流観測が必要な場合は固定元から採り、既存の期待バイトを書き換えない。既存Unitのレビュー対象を変更するときは、その受領の再確認も行う。
- [ ] **Step 8 — 旧受入参照・比較項目を移行**（FR1、NFR1–NFR4）
  - U1のlegacy-consumer-inventoryと全数差分を使い、既存CLI・配布JSON・hash・状態/監査投影・補完の比較を2.7.1へ揃える。
  - キー集合だけの一致、固定slugの読み替え、既知欠落を許す条件、駆動できないままの必要ケースを見直す。同じ前提を再現し、全観測面を比較する。
  - 不正UTF-8とID対応を保つ比較をRust側にも適用する。公開の固定文言・監査語彙・識別子を正規化で消さない。
  - 旧2.6.40の値を現行互換の根拠として残さない。歴史的入力や固定文字列の検査は用途を区別し、単なる一括文字列置換で移行しない。
  - 差の意味が承認済み契約で決まらない場合は、入力・両出力・該当ソースを提示して裁定を求める。
- [ ] **Step 9 — B1の統合検証と引継ぎ**（FR1–FR4、NFR1–NFR4）
  - 対象テスト、fmt/clippy/lint、workspaceテスト、独自lint自体、Quint/ITF、90%床と相対ゲートを確認する。既存CIの依存監査も成功条件に含める。
  - releaseバイナリによるCLI/フック契約を検証し、source-manifest、traceability、code-summary、Red/Greenログを揃えて独立レビューを受ける。
  - B1の変更を具体的な差分と検証結果にまとめ、全CI・競合・レビューを収束させて統合する。U3/U4へ入口・診断記録・残る接続作業を引き継ぐ。
  - 実際のClaudeと人間による一周、同バイナリのdoctor green、安定タグへの切替は後続の達成条件として残す。単体/統合テストで代用しない。

## 変更範囲

既存の `modules/app/aidlc/`、`modules/harness/{claude,infrastructure}/`、`modules/core/command/{domain,use-case,interface-adapter}/`、`modules/core/read-model-updater/`、`modules/core/query/{use-case,interface-adapter}/` の対象操作とテスト。crate依存の変更もこの境界内で行う。

報告結果は既存orchestration配下の報告イベント・要求・DTO・read表・DAO/Viewへ追加する。新しい公開型は規則どおり1ファイルに1型とし、フィールドを公開しない。語彙・Repository配置・DIP・依存方向はcoding-rulesと設計スキル正典に従う。既存コード全体の説明を書き直さない。

追加契約テストは下の手順書のファイルを基本とする。実装中に必要な分割を行った場合はsource-manifestとcode-summaryへ理由を残し、受入条件を弱めない。

## 品質と比較の限界

workspace行カバレッジ90.0%、相対条件 `head >= base - 0.01`、seed 20260823と既存除外を維持する。静的検査や形式モデルに合わせて閾値・アサートを下げない。新規テストを必要とする受領や失敗経路は、正常系だけで済ませない。

本家配布資産は引き続きvendorから使う。更新処理のRust化と配布接続は別責任であり、現在のホストを作業途中で未検証のnative版へ切り替えない。会話で決まった2.7.1とCQS規則を優先し、Testing Contract内の過去の「版は未裁定」という引用は再裁定事項にしない。

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

- [要求書](../../../inception/requirements-analysis/requirements.md)、[契約書](../../../inception/contract-design/contract-summary.md)、[単位定義](../../../inception/units-generation/unit-of-work.md)、[B1実行計画](../../../inception/delivery-planning/bolt-plan.md)。
- U1の[採取証跡](../../u1-upstream-acceptance/code-generation/capture-evidence.md)、[追加ケース引用](../../u1-upstream-acceptance/code-generation/stage1-case-inventory.md)、[旧参照一覧](../../u1-upstream-acceptance/code-generation/legacy-consumer-inventory.md)。
- `commit_verdict_use_case.rs`、`runtime.rs:235–330`、`cli/request.rs:24–87,190–`、`intent_execution_event.rs`、`read_tables/`、Query側の既存DAO/ユースケース。

## Assumptions & Open Questions

採取済みの契約とコードを入力に実装する。新たに見つかる観測差は本計画の停止条件に従う。U1で未検証の利用量集計有効時等について、根拠なしに互換・不要と断定しない。


## 承認済みテスト手順（全文）

# U2のテスト手順

## 実行基盤と最初のコマンド

既存Rust workspaceのCargo/tokio/tempfileと、既存のSQLiteイベントストアを使う。最初に既存の報告ユースケースのテストを実行してランナーを確認する。

```bash
cargo test -p core-command-use-case --lib orchestration::commit_verdict_use_case::tests::
```

U1開始時の全Rust検査は2,354件成功。これはU2実装後の成功を意味しない。新規ファイルは承認後にテストを先に追加し、そのテストが意図した振る舞いで失敗したことを確認してから実装する。未検出・依存不足・構文不正をRedの代わりにしない。

## 単位限定のコマンド

次の新規integration test targetを追加する。通常のテストは実SQLite・公開APIを使い、実行用バイナリのテストはCargoがビルドしたaidlcを子プロセスで呼ぶ。既存multi-call面と新しいhook入口の引数を、その公開パーサへ渡す。

| コマンド | 対象 |
| --- | --- |
| `cargo test -p core-command-interface-adapter --test report_result_contract` | 報告イベントの保存と再構成 |
| `cargo test -p core-read-model-updater --test report_result_projection_contract` | 報告結果の投影・チェックポイント・復旧 |
| `cargo test -p core-query-interface-adapter --test report_result_dao_contract` | ID指定の結果取得・不在・ドメイン非依存 |
| `cargo test -p aidlc --test stage1_authority_contract` | 質問/回答/引継ぎ/レビューと真正な許可の境界 |
| `cargo test -p aidlc --test upstream_271_contract` | 本家2.7.1の同条件CLI/フック・状態/監査比較 |
| `cargo test -p harness-claude --test hook_contract` | 主要4フックと必要な追加イベントの入力/出力 |
| `cargo test -p aidlc --release --test upstream_271_contract` | releaseバイナリによる同じ契約の確認 |

これらは計画段階では未作成である。必要なdev-dependenciesは各所有crateのCargo.tomlへ追加し、既存ランナーを使う。テストを引数フィルターで増やす場合も、実行件数が0になっていないことを確認する。

## 既存契約の回帰コマンド

U2が変更する境界の既存検査を対象ファイルで実行する。

```bash
cargo test -p core-command-interface-adapter --test commit_verdict_use_case_wiring_test
cargo test -p core-command-interface-adapter --test intent_execution_repository_contract
cargo test -p core-read-model-updater --test publication_recovery_contract
cargo test -p core-read-model-updater --test projection_golden_test
cargo test -p core-read-model-updater --test audit_block_golden_test
cargo test -p core-query-interface-adapter --test read_model_dao_contract
cargo test -p aidlc --test intent_lifecycle
cargo test -p aidlc --test next_branches
cargo test -p aidlc --test steering_across_processes
cargo test -p aidlc --test cli_golden_test
cargo test -p core-command-interface-adapter --test golden_parity_test
cargo test -p core-infrastructure --test golden_hash_canonical
cargo test -p core-infrastructure --test golden_corpus_read
```

workspace全体・カバレッジ・Quint/ITF・CI・依存監査は、承認済み実行計画の共通検査としてB1統合前に行う。本手順の単位限定コマンドと区別する。

## 必須ケース

| 対象 | 正常と拒否/失敗 |
| --- | --- |
| report | 遷移する報告、3種類のno-op、構文/状態拒否、1回の競合再試行と2回目の伝播、報告ID/対象の保持 |
| 保存と結果 | SQLite追記→RMU→ID指定クエリ、別報告の取り違え、永続化失敗、投影失敗/再開、投影済み結果の不在、反復投影 |
| CLI進行 | Brownfield/bugfix 9段開始、必要なnext観測、複数部continue、期限/対象が合わないトークン、ゲート待ちと承認後の進行 |
| 受領 | decision/answer/link/review、内容確認、実装計画承認、回答前/別session/古い質問/内容変更/対象変更/受領再利用の拒否 |
| フック | 正常JSON、不正JSON/不足値、無関係入力、停止再入と正当な待機、実際のファイル保存、正規入口以外の遷移拒否 |
| 補助更新 | セッション帰属、規則保存の空/追加/重複/不正、同期とruntime graph、初回に不要な正本を作らないこと、診断実施監査の成功/失敗 |

Standardの各コンポーネント5–8件を基準に、契約の拒否条件を優先する。単に実装と同じ計算を書いたアサートや、コマンドの戻り値だけを観察するテストに偏らない。

## データ・代替処理・品質基準

U1の `tests/golden/upstream-a277af21/` と来歴・生観測・比較規則を入力にする。期待出力の手修正や固定文字列を消す正規化は行わない。反復コマンドの出力だけでなく、状態・監査の変化/無変更を同じ条件で比較する。新しい上流採取が必要なら固定コミットから実行して追加し、既存の比較基準の変更を隠さない。

テストの人間応答・会話は一時ワークスペース内の明示した合成入力とする。実際のユーザー承認や本リポジトリの受領を書き換えない。時計・乱数・プロセス/ファイル障害等の外部境界だけを必要に応じて制御し、Repository/RMU/Queryは実物の連携を検証する。

行カバレッジ床90.0%、相対ゲート0.01ポイント、seed20260823、既存Quint3モデルとITFを維持する。Red/Greenの実行コマンド・失敗理由・終了状態をU2記録へ残す。実地Claudeスモークとnative doctor成功は後続で実施し、このテストの成功で置き換えない。



