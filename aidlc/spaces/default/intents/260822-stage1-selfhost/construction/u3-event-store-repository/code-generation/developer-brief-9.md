AIDLC-UNIT: u3-event-store-repository
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

# developer-brief-9 — 委任 9: U3 再走（Quint 凡例の追従 + 受入の再実測 + 記録の現行化）（U3 / Bolt b52）

Conversation language: 日本語（コメント・報告・コミット文はすべて日本語。コード識別子・固定トークンは英語のまま）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（kind: library）の 2026-09-07 再走（Modify）における委任 9。
リポジトリルートは git worktree `/Users/j5ik2o/orca/workspaces/amadeus-ng/stage1-selfhost`、ブランチ `stage1-selfhost`。**このディレクトリの
外へ `cd` しない**（元リポジトリ `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs` を触らない）。ワークスペースのコードは `origin/main` =
`f2b6b6a9` と同一で、他のエージェントは走っていない。

承認済みの計画は付録 B（`code-generation-plan.md` 逐語）、テスト手順は付録 C（`unit-test-instructions.md` 逐語）。**Step 1〜7 をこの順に**実行し、
各 Step の結果を報告に書く。計画の Testing Contract（付録 B 末尾の JSON）が Part 2 の権威であり、memory を独自に再解釈しない。NFR 要求・NFR 設計・
Testing Contract の数値目標（coverage 90% 床、`TOLERANCE`、シード、lint の deny）は入力であり、**通すために緩めない**。届かなければ届かないと書く。

### 所有ファイル（書いてよいもの）

- `formal/orchestration/journal_protocol.qnt` — **凡例コメント 5 行だけ**（`:10` / `:11` / `:15` / `:22` / `:23`、計画 Step 3）。状態機械本体
  （`var` / `action` / `val` / `run` / witness）は 1 文字も変えない。
- Unit 記録（新規 / 書換）: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/` 配下の
  `developer-report-11.md`（新規、1 回の Write）、`code-summary.md`（書換）、`traceability.json`（書換）、`source-manifest.json`（新規）。
- `target/` 配下（ビルド・カバレッジの生成物）。

### 触ってはいけないもの

`modules/**`（読取と実行のみ）、`tests/**`、`scripts/**`、`.github/**`、`Cargo.toml` / `Cargo.lock`、`tools/**`、`docs/**`、`.claude/**`、
上記以外の `aidlc/**`（計画・テスト手順・質問票・履歴ファイル・他 Unit・memory を含む）。
**`git add` / `git commit` / `git push` / `git stash` / GitHub（`gh`）は使わない。`bun .claude/tools/aidlc-*.ts` は traceability センサー 1 本
（計画 Step 6 の `aidlc-sensor-traceability.ts`）を除き実行しない。環境変数 `AIDLC_*` を設定してフックを回避しない。**

## この Bolt が何か（初見向けの説明）

U3 は「集約 `IntentExecution` をイベントソーシングで SQLite に保存し、再構成する Repository」の Unit で、実装自体は Bolt B5（PR #29）以降で
完成している。今回は unit-major 反復の再走で、設計文書 3 段（機能設計 / NFR 要求 / NFR 設計）を現行コードに合わせて書き直した直後の
code-generation 段である。設計文書は現行コードを実測して書かれており、振る舞いに関わる不一致は無い。見つかっている唯一の不一致は Quint
モデル（形式検証の状態機械記述）の**凡例コメント**が、2026-08-30 の集約改名（B12: `WorkflowExecution` → `Intent` + `IntentExecution`）より前の
旧名を 5 行残していることで、これを現行名へ直す。あとは受入コマンドを実測して記録を現行の事実に書き直す。

## 実行上の注意（このセッションのフックについて）

- 実行前に `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md` を読む（オーナー規律。今回コードは書かないが、照合の判断基準になる）。
- Bash はフック（plan-approval-guard）の監視下にある。**1 回の呼び出しに 1 コマンド**、`$(...)`・バッククォート・`;` 連結・heredoc を避ける。
  パイプ（`| tail -n 40` など）は使ってよい。ファイルの作成・書換は Write / Edit ツールで行う（シェルのリダイレクトで書かない）。
  フックに拒否されたら、同じ意図をより単純な単一コマンドに言い換える。回避策（環境変数・別ディレクトリ経由）は禁止。
- `scripts/coverage.sh` は数分かかる。Bash の `timeout` を 600000 に設定し、必要なら `run_in_background` で回して完了を待つ。2 回とも同じ
  リビジョン（Step 3 の凡例追従の後）で実行し、生の head 値を記録する。
- 実測の原則: **鵜呑みにしない**。計画や設計の行番号・件数は `grep -n` / 実行結果で確かめ、違えば違うと書く。成功ログを推定で書かない。
  失敗・未達・未導入は隠さず、そのまま報告する。

## 報告（`developer-report-11.md`、Write 1 回 + 最終メッセージに同内容の要約）

1. §0 環境: `rustc -V` / `cargo llvm-cov --version` / `cargo audit --version` / `quint --version`、開始・終了時刻、所要時間。
2. §1 Unit 限定コマンド 11 本: バイナリごとの件数（passed / failed / ignored）、終了コード、完了時刻。Step 3 の後に再実行した ITF 適合・
   クラッシュ再構成の結果も別行で。
3. §2 設計との照合表: 計画 Step 2 (a)〜(g) を項目ごとに「設計の主張 / 実測 / 一致・不一致」の 3 列で。不一致には Red テスト案を添える。
4. §3 凡例追従: `git diff formal/orchestration/journal_protocol.qnt` の逐語、`git diff --stat` が 1 ファイル 5 行であること、旧名 grep の
   前後の結果。
5. §4 受入: 計画 Step 4 (a)〜(f) の結果（コマンド・終了コード・要点の出力。coverage は 2 回の生の値と差、`cargo audit` は走査 crate 数と
   advisory DB 取得可否、退役 grep は 0 件の証拠、`cargo test --workspace` は総数と所要時間）。
6. §5 記録の現行化: `code-summary.md` / `traceability.json` / `source-manifest.json` に書いた内容の要約と、traceability センサーの出力
   （`invalid_targets` が空であること）。
7. §6 `git status --short` の逐語（ワークスペース側の差分が `journal_protocol.qnt` だけ、記録側が本ディレクトリだけ）。
8. §7 未検証範囲と申し送り（計画 Step 5 の §6 / §7 に載せたもの、実行中に気づいたこと）。

## 設計文書（現在の Unit のみ。必要な箇所を Read する）

- `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md` — §3.1 / §3.2 の
  store / find_by_id 手順、§5 公開 API、§7 検証表。
- 同 `functional-design/rules.md` — BR1.1〜BR5.2（23 本、YAML の `logic` 欄）。
- 同 `functional-design/entities.md` — 集約・DTO・Repository の構造（YAML）。
- 同 `nfr-requirements/security-requirements.md` — NFR1.1〜NFR4.7（20 本）の合格基準。
- 同 `nfr-design/security-design.md` — §2 検査点表（層 (0)〜(4) の関数・行番号・テスト名）、§3 原子性と競合、§4 障害ドメイン、§6 退役の維持。
- 同 `nfr-design/logical-components.md` — §1 コンポーネント一覧と依存、§4 テスト配置の件数。
- 上流（要約のみ、必要なら Read）: `inception/requirements-analysis/requirements.md`（FR1.2 原子的保存 / FR1.3 Repository / NFR3 監査完全性。
  `:44-47` / `:63` / `:133-135` / `:168-170` / `:186` は旧名・`audit_lock.qnt` のまま失効している — 直さず申し送りに書く）、
  `inception/contract-design/contract-summary.md`（C3 Repository 契約 / C6 ストア契約）、`inception/units-generation/unit-of-work.md`（U3 の
  合格条件 3 つ: 契約テスト両バックエンド・ITF 適合・クラッシュ後の再構成）。

## ワークスペースの前提（aidlc-state.md / 実測）

Rust 2024 edition、ツールチェーン 1.95.0（`rust-toolchain.toml`）、ワークスペース lints 47 本 deny（`unwrap_used` / `expect_used` /
`missing_docs` / `unreachable_pub` / `indexing_slicing` / `panic` / `print_stdout` など、`Cargo.toml` `[workspace.lints]`）、`cargo lint`（`tools/lint`）、
Quint 0.32.0、`cargo-llvm-cov`、`cargo-audit`。テストは `cargo test`（非同期は `tokio` current_thread）。CI は `.github/workflows/ci.yml` の
7 ジョブ（aidlc-distribution / check / quint / coverage / audit / review-thread-resolution / ci-success）。

---

以下、付録 A（規則束: org / team / project / phases/construction の逐語）、付録 B（承認済み `code-generation-plan.md` の逐語）、付録 C
（`unit-test-instructions.md` の逐語）。
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
# code-generation-plan — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> Unit: U3（kind: library）。**2026-09-07 の再走（Modify）計画**。出典: `../functional-design/{functional-spec,rules,entities}.md`（2026-09-05 是正・
> 2026-09-07 再レビュー READY）、`../nfr-requirements/{security-requirements,tech-stack-decisions}.md`（2026-09-07 再走 READY、NFR1.1〜NFR4.7）、
> `../nfr-design/{security-design,logical-components}.md`（2026-09-07 再走 READY）、`../../../inception/contract-design/contract-summary.md`（C3 / C6）、
> `../../../inception/units-generation/unit-of-work.md`（U3）、`../../../inception/requirements-analysis/requirements.md`（FR1.2 / FR1.3 / NFR3）、
> `code-generation-questions.md`。2026-08-23 に承認した旧計画（Bolt B5 = PR #29、指紋 `04a8a9e1…`）は `code-generation-plan-history-2026-08-23.md` に
> 全文保存した。旧テスト手順・旧質問票・旧要約・旧 traceability も同名の `*-history-2026-08-23.*` に保存済み。

## 1. 目的と変更範囲

U3 の実装（`IntentExecutionRepositoryImpl<S>`・永続化 DTO・`SnapshotStrategy`・`store_failure`・`StorePath`・`journal_protocol.qnt`・
ITF 適合・クラッシュ再構成）は Bolt B5（PR #29）で main に入り、その後 B7（PR #31、本家 event-store-adapter-rs v3 `EventEnvelope` API）・
B12 / B13（集約分割 `Intent` + `IntentExecution`・版の内側化、2026-08-30）・b40（イベント ID）・2026-09-05 是正（書込前 ID 照合）を経て現行の形に
なっている。2026-09-07 に再走した設計 3 段は現行コードを実測して書かれ、行番号・件数・依存はすべて一致した（各成果物の末尾レビュー READY）。
ワークスペースのコードは `origin/main` = `f2b6b6a9` と同一である（`git diff --stat origin/main HEAD -- modules tests formal scripts Cargo.toml
Cargo.lock .github tools` が空、計画準備時の実測）。

計画準備で見つかった設計と実物の不一致は **1 件**だけである: `formal/orchestration/journal_protocol.qnt` の凡例コメント（モデル型 ↔ Rust 対応表、
ADR 0003 決定 6）が B12 以前の旧名を使っている — `:10` / `:11` の `WorkflowExecution::version()` / `::seq_nr()`、`:15` / `:22` / `:23` の
`WorkflowExecutionRepository::find_by_id` / `::store`。`WorkflowExecution` の文字列は `modules tests scripts .github Cargo.toml tools` で 0 件で、
現行名は `IntentExecution` / `IntentExecutionRepository`（`intent_execution.rs:254` `with_version`、use-case `port/intent_execution_repository.rs`）。
functional-spec §5（`:137`「旧 `WorkflowExecutionRepository` の名前を現行公開 API に残さない」）と BR5.1（仕様・正本の同期）に照らし、凡例だけが
取り残されている。同じ凡例の U4 側の名前（`:12` `JournalReader::checkpoint(ProjectionName)`、`:25` `events_after` / `advance_checkpoint`）は現行
`read-model-updater/src/orchestration/journal_reader.rs:77,85,109` と一致するので触らない。

今回のワークスペース側の変更は**この 1 ファイル・コメント 5 行だけ**とし、プロダクトコード・テスト・依存・スクリプト・CI・Quint の状態機械本体
（var / action / 不変条件 / witness）は変更しない。行うのは次の 5 点である。

1. Unit 限定コマンド（`unit-test-instructions.md` §2）と受入（BR5.2、NFR2.3 / NFR2.4 / NFR2.5 / NFR4.1）を実測して記録する。
2. 再走した設計 3 段の主張（検査点の関数・行番号・テスト名、コンポーネント配置と依存、件数）を現行コードで照合し、一致・不一致の表を作る。
3. `journal_protocol.qnt` の凡例 5 行を現行名へ追従させ、`scripts/quint-gate.sh`（typecheck・不変条件 8・witness 4）と ITF 適合テストで検証する。
4. `code-summary.md` を現行の事実で書き直す（B5 の TDD 証跡・裁定表・コミット列は履歴ファイルに残し、本版は「現行の実装がどう検証されたか」を書く）。
5. `traceability.json` を現行 ID（FR1.2 / FR1.3 / NFR3、BR1.1〜BR5.2 の 23 件、NFR1.1〜NFR4.7 の 20 件 = 46 件）の**実在ファイル**へ対応付け、
   `source-manifest.json`（`writes` = `journal_protocol.qnt` 1 件）を作る。

変更しないもの: `modules/` 配下のプロダクトコードとテスト、`Cargo.toml` / `Cargo.lock`、`tests/conformance/`（ITF fixture はコメントを含まないため
凡例変更の影響を受けない）、`scripts/`、`.github/`、`docs/specs/`（`WorkflowExecution` の残り 4 ファイルは取り消し線付きの履歴記述で U9 の所有）、
凍結中の設計文書（FD / NFR 要求 / NFR 設計 — 各 `pending-revision.md` の確定文面はステージゲートの Request Changes 経路で折り戻す）、他 Unit の記録、
上流の `requirements.md`（NFR3 の失効はオーナー裁定待ち）。GitHub への書込（PR 作成・コメント）は委任先では行わない。

照合で不一致が**新たに**見つかった場合は、対象・再現手順・**先に書く Red テスト案**を `developer-report-11.md` に報告し、計画の変更を受けてから
扱う。本計画を根拠に凡例 5 行以外のコードを直さない。U7 の裁定事項 3 件（複数プロセスの並行モデルと `reopened()` / 兄弟接続、登録簿の直列化、
`SnapshotStrategy` 既定値）は先取りしない。

## 2. 所有するファイルと保持する成果

| 区分 | 対象 | 扱い |
|---|---|---|
| ワークスペース（変更） | `formal/orchestration/journal_protocol.qnt` 凡例コメント 5 行（`:10` / `:11` / `:15` / `:22` / `:23`） | 旧名 → 現行名。状態機械本体は触らない。`source-manifest.json` の `writes` に載せる |
| ワークスペース（読取のみ） | `modules/core/command/{domain,use-case,interface-adapter}/`、`modules/app/aidlc/tests/{journal_protocol_conformance,crash_reconstruction_test}.rs`、`modules/app/aidlc/src/runtime.rs`（配線の実測のみ）、`tests/conformance/fixtures/journal_protocol/`、`scripts/{quint-gate,coverage}.sh`、`Cargo.toml` / `Cargo.lock`、`.github/workflows/ci.yml` | 検証と照合のみ。差分を残さない |
| Unit 記録（更新） | `code-summary.md`、`traceability.json`、`source-manifest.json`、`developer-report-11.md`（新規） | 現行の事実で書く。旧 `code-summary.md` / `traceability.json` は `*-history-2026-08-23.*` に保存済み |
| 計画と試験手順 | 本ファイル、`unit-test-instructions.md` | この計画承認の対象。完了チェック以外の変更が必要なら承認を更新 |
| 履歴（変更しない） | `*-history-2026-08-23.*`、`developer-brief-1〜8.md`、`developer-report-1〜10.md`、`handoff-b5*.md`、`coverage-gaps-b5.md`、`naming-audit-report.md` | B5 の記録としてそのまま保持 |

過去の TDD 証跡（B5 の Red / Green）・B5 時点の件数（674 / 98.42%）・B5 の裁定表は歴史であり、今回の実施や現在の状態として記載しない。
今回変更しない既存ファイルは code-summary の照合欄で示し、変更済みと偽らない。source-manifest には実際に作成・変更・削除したアプリケーション側
パスだけを列挙する（今回の予定は `formal/orchestration/journal_protocol.qnt` の 1 件）。

## 3. 実行ステップ

- [ ] Step 1. ランナーと設定を確認する。`rustc -V`（`rust-toolchain.toml` = 1.95.0）、`cargo llvm-cov --version`、`cargo audit --version`、
      `quint --version`（0.32.0）の有無と版を記録する。`unit-test-instructions.md` §2 の Unit 限定コマンド 11 本を順に実行し、テストバイナリごとの
      件数・結果・完了時刻を記録する（期待件数は同 §2。件数が違えば違うまま記録し、理由を調べる）。
- [ ] Step 2. 設計との照合。次を現行コードで突き合わせ、一致 / 不一致の表を `developer-report-11.md` に書く（引用行番号は全ファイル `grep -n`
      の絶対行）: (a) `security-design.md` §2 検査点表 — 層 (0) `store:447-452` / (0') `write_error:277-303` / (1) `read_error:245-265` /
      (2) `IntentExecutionDto::to_domain:208-293` + `IntentExecution::new:290-337` / (3) 差分ループ `:378-438` / (4) `replay:352` / `apply_event:1514` /
      誕生変換 `:2373` と、各層に対応づけたテスト名の実在; (b) §3 の分岐 `:465` と `stored_version:313`、`reopened:209`; (c) §4 の写像表
      `store_failure.rs:20-34`; (d) `logical-components.md` §1 のコンポーネント一覧（ファサード `orchestration/mod.rs:38-66` の `pub use`
      （`:38-41` / `:46` / `:52` / `:62` / `:65-66`）、`dto/` 32 エントリ・イベント変種 DTO 16）と依存（3 クレートの `Cargo.toml` — domain / use-case に
      event-store-adapter-rs なし、domain に serde なし（`serde_json` は dev のみ）、`cargo tree` で `thiserror` は推移のみ、`interface-adapter/src/` に
      `CREATE TABLE` / `PRAGMA` / `busy_timeout` 0 件）; (e) `functional-spec.md` §3.1 / §3.2 の手順（genesis `seq_nr == 1` → `persist_event_and_snapshot`、
      基底欠落 + journal あり → `Corrupt(MissingSnapshot)`、`with_version(snapshot.version())`）; (f) `rules.md` BR1.1〜BR5.2 の logic 欄;
      (g) 旧名 grep — `WorkflowExecution` を `modules tests scripts .github Cargo.toml tools formal` で grep し、計画準備時の実測（`journal_protocol.qnt`
      の凡例 5 行のみ、他 0 件）と一致することを Step 3 の前に確認する。不一致は Red テスト案（どのテストが、何を assert すれば現行コードで落ちるか）を
      添えて報告する。
- [ ] Step 3. Quint 凡例の追従。`formal/orchestration/journal_protocol.qnt` の 5 行だけを書き換える: `:10` `WorkflowExecution::version()` →
      `IntentExecution::version()`、`:11` `WorkflowExecution::seq_nr()` → `IntentExecution::seq_nr()`、`:15` `WorkflowExecutionRepository::find_by_id` →
      `IntentExecutionRepository::find_by_id`（「`with_version` で載せた値」は現行 `intent_execution.rs:254` と一致するので保持）、`:22` 同 `find_by_id`、
      `:23` `WorkflowExecutionRepository::store` → `IntentExecutionRepository::store`。`git diff --stat` がこの 1 ファイル・5 行の変更だけであることを確認し、
      (g) の grep を再実行して `formal` を含む全範囲で 0 件になったことを記録する。検証は Step 4 (b) の quint-gate（typecheck を含む）と Step 1 の
      ITF 適合 / クラッシュ再構成の再実行。コメント行にはテストが無いため TDD の Red は作らない（§4）。
- [ ] Step 4. 受入を実測する（BR5.2）。(a) `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` /
      `cargo test --manifest-path tools/lint/Cargo.toml`（NFR2.4）; (b) `bash scripts/quint-gate.sh`（NFR2.5 — journal_protocol の typecheck /
      不変条件 8（conflict_rejected / snapshot_tracks_journal / version_equals_journal / checkpoint_monotone / checkpoint_bounded / projection_idempotent /
      truth_is_journal / no_lost_update）/ witness 4（w_conflict / w_crash_then_catchup / w_interleaved_writers / w_idempotent_catchup）を含む全ステップ、
      Step 3 の後に実行）; (c) `bash scripts/coverage.sh` を同一リビジョン・同一ツールチェーン・同一シード（`PROPTEST_RNG_SEED=20260823`、スクリプト
      内で固定）で 2 回実行し、生の head 値（%）と差を記録する（絶対 90% 床が 2 回とも成功、差 0.00 ポイントが受入目標。未達なら未達のまま原因を記録し、
      `TOLERANCE` / 除外 / シードを変えない）（NFR2.3）; (d) `cargo audit` と `cargo audit --file tools/lint/Cargo.lock`（NFR4.1、`ci.yml:186-190` と同じ
      2 件 — 結果・走査 crate 数・advisory DB 取得可否。未導入・取得失敗は成功と書かない）; (e) 退役 grep（`WorkspaceLock|FsWorkspaceLock|LockProtocol|
      LockIdentity|ProcessProbe|audit_lock|within_write_transaction|reap_eligible|OwnerStamp|AcquireBudget|LockGuard|process_alive|reap-decision-locality`
      を `modules tools scripts formal .github Cargo.toml` で 0 件（計画準備時の実測 0 件）、`ls formal/orchestration/` = engine_loop / journal_protocol /
      stop_hook）（NFR1.2 / BR3.1）; (f) `PROPTEST_RNG_SEED=20260823 cargo test --workspace` の総数と結果（全体ゲートとして 1 回だけ。Unit 限定コマンド
      ではないことを明記）。
- [ ] Step 5. `code-summary.md` を現行の事実で書き直す。§1 結果（Step 1 / 4 の実測表）、§2 変更ファイル（Step 3 の 5 行、`git diff` の逐語）と現行の
      実装ファイル一覧（`modules/core/command/interface-adapter/src/orchestration/{intent_execution_repository_impl,snapshot_strategy,store_failure}.rs` +
      `dto/` 32、use-case の `port/{intent_execution_repository,repository_error}.rs`、domain の `intent_execution.rs`（`new` :290 / `replay` :352 /
      `with_version` :254）と `workspace/{store_path,intent_dir_name}.rs`、formal + fixture 8、app tests 2 本 — B5 以降の来歴を 1 行ずつ）、
      §3 設計との照合表（Step 2）、§4 テスト配置の件数（logical-components §4 と一致するか）、§5 依存（`cargo tree` の実測）、§6 未検証範囲（全 CI
      実行・複数プロセス並行・`reopened()` 複数ハンドルと兄弟接続の並行・末尾欠落の検出）、§7 申し送り（U7 裁定 3 件、上流 `requirements.md` の旧名 /
      `audit_lock.qnt`（`:44-47` / `:63` / `:133-135` / `:168-170` / `:186`）の失効、`docs/specs/` 4 ファイルの取り消し線記述は U9 所有、pending-revision
      の折り戻し先）、§8 B5 からの変更（本再走で書き直した理由）。B5 の裁定・TDD 証跡・コミット列は履歴ファイルを参照する。
- [ ] Step 6. `traceability.json` を 46 ID で書き直す（target はワークスペース相対パス 1 本。FR1.2 → `intent_execution_repository_impl.rs`、FR1.3 →
      同、NFR3 → `modules/app/aidlc/tests/crash_reconstruction_test.rs`、BR / NFR は設計の該当ファイルへ）。`bun .claude/tools/aidlc-sensor-traceability.ts
      --stage code-generation --output-path <traceability.json>` で `invalid_targets` 0 を確認する（`missing_from_upstream_ids` は他 Unit の
      ID で既知のノイズ）。`source-manifest.json` を strict schema（`{"stage","unit","version":1,"writes":["formal/orchestration/journal_protocol.qnt"]}`）
      で作る。
- [ ] Step 7. `git status` でワークスペース側の差分が `journal_protocol.qnt` だけであること、記録側の変更が本ディレクトリに限られることを確認し、
      `developer-report-11.md` に Step 1〜6 の結果と所要時間を書いて親セッションへ返す。親セッションが独立レビュー・Unit 完了・commit・PR を処理する。

## 4. Testing Contract の適用

本 Unit は library で、Testing Contract（tdd / standard）の層は「Data model」= DTO と値オブジェクト、「Repository」= `IntentExecutionRepositoryImpl`、
「Business logic」= 検査点と Quint 協定、「API」= 契約テスト両バックエンド・ITF・クラッシュ再構成。**今回は新規プロダクションコード・新規テストが
無い**（変更は Quint モデルのコメント 5 行で、振る舞いを持たない）ため、TDD の Red / Green / Refactor ステップは架空に実行しない。既存の検証
（契約 22・実装固有 23・本家適合 10・インライン 45・クラッシュ 5・ITF fixture 8）を再実行し、Standard 戦略「コンポーネントごと 5〜8 本」は既存件数で
満たす。既存スイートは緑のまま維持する。凡例変更の受入は `scripts/quint-gate.sh` の typecheck と不変条件 / witness の全緑（コメント変更で状態機械が
壊れていないことの機械確認）。

照合で不一致が**新たに**見つかった場合の手順は TDD を守る: 現行コードで落ちる Red テストを先に書いて失敗出力を報告に記録し、計画の変更を受けてから
Green にする。既存の成功ログから過去の Red を推定しない。

## 5. 要求からステップへの対応

| 要求 | BR | Step | 確認対象 |
|---|---|---|---|
| FR1.2（原子的保存と楽観競合制御） | BR1.3 / BR2.3 / BR2.4 / BR3.2 | 1〜2, 4 | 契約テスト（genesis 版採番・Conflict 3 種・rehydrated 版の成功）、`store:441-481` の分岐、ITF `conflict_rejected` / `no_lost_update`、クラッシュ再構成 5 |
| FR1.3（Repository） | BR1.1 / BR1.2 / BR1.4 / BR1.5 / BR2.1 / BR2.2 / BR2.5〜2.8 | 1〜2 | `find_by_id:331-439` の手順、ポート署名、DTO `to_domain`、両バックエンド同一関数群 |
| NFR3（監査完全性） | BR1.2 / BR3.3 / BR3.5 / BR5.2 | 1〜4 | 差分行の検査（SequenceGap / ForeignManifest / aggregate_id）、ITF 8 トレース、quint-gate（凡例追従後） |
| NFR1.1 / NFR1.2 / NFR1.3 | BR2.1 / BR2.2 / BR3.1 / BR3.2 | 2, 4 | `deviations.md` #4、退役 grep 0 件、ピン `=3.0.0` 不変 |
| NFR2.1〜NFR2.5 | BR2.7 / BR3.4 / BR5.2 | 1, 4 | Unit 限定コマンド、lints / `cargo lint`、coverage 2 回、quint-gate |
| NFR3.1〜NFR3.5 | BR1.2 / BR1.3 / BR1.5 | 2 | 検査点の四層 + 書込前 (0)、`with_version` の版保持、U4 所有面への非干渉 |
| NFR4.1〜NFR4.7 | BR2.1 / BR2.8 / BR4.1〜4.3 | 2, 4 | `cargo audit` 2 件、`cargo tree`、`unsafe_code = forbid`、`store_failure` 写像表、`StorePath`、`reopened()` と CAS |
| BR5.1（仕様・正本の同期） | — | 3, 5 | Quint 凡例の旧名追従（Step 3）。code-summary §7 に折り戻し先（FD / NFR / ND の pending-revision）と上流の失効箇所を明記。設計文書の正本は本 Bolt では変更しない |

## 6. 作業の進め方

- 委任は 1 回（`aidlc-developer-agent`、**Opus** — 設計 3 段の照合表の読解と Red 案の起草を要するため）。ブリーフは `developer-brief-9.md`
  （規則束・本計画・`unit-test-instructions.md`・設計 3 段の逐語連結）経由とし、プロンプトには先頭 2 行のマーカー（`AIDLC-UNIT` /
  `AIDLC-TESTING-CONTRACT`）と要点再掲を置く。
- 開発担当がワークスペースで書き換えるのは `journal_protocol.qnt` の 5 行だけ（受入 (c) のカバレッジ計測は `target/` 配下だけを書く）。計画・
  テスト手順・質問票を書き換えない。`git` の書込操作・push・GitHub・`aidlc-*.ts` の実行をしない。報告は `developer-report-11.md`（1 回の Write）と
  最終メッセージの両方に書く。
- 親セッションは報告の全項目を独立に再実測してから code-summary / traceability の内容を受け入れ、独立レビュー（advisory、iteration 1）へ渡す。
  本 Bolt はワークスペース差分を持つので、Unit 完了後に Bolt 単位の PR（slug `b52-u3-event-store-repository`、直列運用、squash-merge）を開き、
  収束ルール（必須 CI green ∧ unresolved 0 ∧ 全コメント返信済み、最新 head 再実測）で畳む（オーナー包括承認 2026-08-29）。

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
# unit-test-instructions — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> 対象: u3-event-store-repository（library）。現行 `code-generation-plan.md`（2026-09-07 再走）と Testing Contract、
> `../nfr-requirements/security-requirements.md` NFR2.1〜NFR2.5 / NFR4.1〜NFR4.7、`../nfr-design/logical-components.md` §4、
> `../functional-design/rules.md` BR5.2 に従う。以下はすべて本 Unit に限定する。2026-08-23 の旧手順（旧クレート名・旧テスト名）は
> `unit-test-instructions-history-2026-08-23.md` に全文保存した。

## 1. ランナーと設定

`cargo test`（Rust 1.95.0、`rust-toolchain.toml` で固定）。非同期ポートの契約テストは `tokio`（dev-dependency、`current_thread`）で回す。
SQLite バックエンドは `tempfile` の一時ディレクトリに `intents/.aidlc-store.sqlite` を作る。PBT はアダプタに無い（`proptest` を持つのは
`core-command-domain` / `core-infrastructure` / `core-query/use-case`）。ワークスペース全体のシード固定 `PROPTEST_RNG_SEED=20260823` は
本 Unit のコマンドには効かないが、受入 (f) の `cargo test --workspace` では必ず付ける。追加のランナー・モック・設定ファイルは導入しない。

## 2. Unit 限定コマンド

ワークスペースルートで実行する。`--locked` で `Cargo.lock` を変えない。

```sh
cargo test --locked -p core-command-interface-adapter --test intent_execution_repository_contract   # 契約 11 関数 × memory / SQLite
cargo test --locked -p core-command-interface-adapter --test intent_execution_repository_impl_test  # 実装固有（基底 + 差分・破損境界・I/O）
cargo test --locked -p core-command-interface-adapter --test upstream_event_store_conformance       # 本家適合 5 関数 × 2 バックエンド
cargo test --locked -p core-command-interface-adapter --lib orchestration::intent_execution_repository_impl  # インライン（CorruptDetail・封筒・写像・reopened）
cargo test --locked -p core-command-interface-adapter --lib orchestration::store_failure            # rusqlite code → ErrorKind の写像
cargo test --locked -p core-command-interface-adapter --lib orchestration::snapshot_strategy        # 既定 10 と任意間隔
cargo test --locked -p core-command-interface-adapter --lib orchestration::dto                      # DTO の往復・拒否
cargo test --locked -p core-command-domain --lib workspace::store_path                              # StorePath::for_space
cargo test --locked -p core-command-domain --lib workspace::intent_dir_name                         # IntentDirName の文法
cargo test --locked -p aidlc --test crash_reconstruction_test                                       # クラッシュ再構成（新接続で同値ほか）
cargo test --locked -p aidlc --test journal_protocol_conformance                                    # ITF 8 トレースの再生（every(1) 明示）
```

計画準備時（2026-09-07、ワークスペースは `origin/main` = `f2b6b6a9` と同一）の実測: 契約 22 / 実装固有 23 / 本家適合 10 / クラッシュ再構成 5、
いずれも PASS。`#[test]` + `#[tokio::test]` 属性の合計は impl 10（5 + 5）/ store_failure 4 / snapshot_strategy 2 / dto 29 / store_path 4 /
intent_dir_name 9 / journal_protocol_conformance 5（2 + 3）/ crash_reconstruction 5（契約と本家適合はマクロ展開のため属性数 2 と実行件数が異なる）。
実行担当は上記を再実行し、バイナリごとの件数・結果・完了時刻を `developer-report-11.md` へ残す。ITF 適合とクラッシュ再構成は、計画 Step 3
（Quint 凡例コメント 5 行の追従）の**後**にもう一度実行し、両方の結果を残す。ワークスペース全体の `cargo test --workspace` は CI の品質ゲートで
あり、本ファイルの Unit 限定コマンドではない。

## 3. 合格基準と検証範囲

- §2 の 11 コマンドがすべて終了コード 0、`failed` 0、`ignored` 0。
- 件数が計画準備時と違う場合は違うまま記録し、原因（テスト追加・削除・フィルタ不一致）を `git log -p` で調べて報告する。件数を合わせるために
  テストを足したり消したりしない。
- 受入（Unit 限定コマンドの外側、計画 Step 4）は次を実測して記録する。設定の存在と実働の成功を区別する。
  - `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `cargo test --manifest-path tools/lint/Cargo.toml`
    がすべて終了コード 0（`tools/lint` の自己テスト件数は出力から記録）。
  - `bash scripts/quint-gate.sh` が `[PASS] quint gate: all steps green`（journal_protocol の typecheck / 不変条件 8 / witness 4 を含む。計画 Step 3
    の凡例追従の**後**に実行し、コメント変更で状態機械が壊れていないことの機械確認とする）。
  - `bash scripts/coverage.sh` を同一リビジョン・同一ツールチェーン・同一シードで 2 回実行し、生の head 値（%）と差を記録する。絶対ゲート 90% が
    2 回とも成功すること。差 0.00 ポイントは受入目標であり、未達なら未達のまま原因を記録し、`TOLERANCE` / 除外 / シードを変えない。
  - `cargo audit`（workspace）と `cargo audit --file tools/lint/Cargo.lock` の結果、走査 crate 数、advisory DB 取得可否。未導入・取得失敗は
    成功と書かない。
  - 退役 grep（計画 Step 4 (e) の語彙）が `modules tools scripts formal .github Cargo.toml` で 0 件、`ls formal/orchestration/` が
    engine_loop / journal_protocol / stop_hook の 3 つ。
  - 旧名 grep（`WorkflowExecution`）が計画 Step 3 の前は `formal/orchestration/journal_protocol.qnt` の凡例 5 行（`:10` / `:11` / `:15` / `:22` /
    `:23`）だけ、Step 3 の後は `modules tests scripts .github Cargo.toml tools formal` で 0 件。`git diff --stat` がその 1 ファイル・5 行だけ。
  - `PROPTEST_RNG_SEED=20260823 cargo test --workspace` の総数と結果（全体ゲートとして 1 回。所要時間も記録）。
  - `rustc -V` が 1.95.0。

Unit 限定コマンドと上記実測の成功は、全 CI 実行・マージキューの完走・複数プロセスの並行書込・`reopened()` 複数ハンドルと兄弟接続の並行・
末尾欠落の検出の代替ではない（設計が未検証範囲として明記しているもの）。全体検証を Unit ごとに繰り返すコマンドはここへ置かない。

## 4. データとテスト支援

テストデータは各テストファイルのフィクスチャ（`IntentExecution::start` による genesis、`StageEntries` の合成計画、UUIDv7 リテラル）と
`tests/conformance/fixtures/journal_protocol/*.itf.json`（8 本、`#meta` 正規化済み）。テストダブルは使わず、`IntentExecutionRepositoryImpl::in_memory()`
（本家 memory バックエンド）と `open(&StorePath)`（本家 SQLite）を同じ契約で走らせる（BR2.7）。ネットワークは使わない（`cargo audit` の advisory DB
取得を除く）。認証トークン・環境変数を記録へ混ぜない。

## 5. 失敗時

失敗したコマンド・テスト名・出力を `developer-report-11.md` に記録する。計画 Step 3 の凡例追従以外で設計と実装の不一致が見つかれば、現行コードで
落ちる Red テスト案（対象ファイル・assert の内容・期待する失敗出力）を親セッションへ返し、計画を更新してからコードを変える。今回の記録現行化の
ためにコードを壊して人工的な Red を作らない。閾値・シード・除外・依存を変えて成功させない。
