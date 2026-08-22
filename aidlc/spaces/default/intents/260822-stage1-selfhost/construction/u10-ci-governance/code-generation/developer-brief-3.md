AIDLC-UNIT: u10-ci-governance
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

# 開発ブリーフ（委任 3）— U10 CI ガバナンス、計画 Step 1〜9

> **履歴メモ（2026-08-22 UTC、PR #25 レビュー指摘の引き取り）**: 本ブリーフは委任時点の記録。§6 の計画全文に含まれる Step 0（ブランチ作成）はコンダクタが委任前に完了済みで、開発エージェントは実行しない（§0 のとおり）。

Conversation language: 日本語（会話・人間可読成果物・コメント・コミット本文の説明は日本語。コード識別子・固定トークン・
コミット件名の型（chore/feat/test）は英語のまま）

## 0. あなたの役割と境界

- あなたは aidlc-developer-agent。承認済み計画（下記 §6、**Step 1〜9 だけ**）を順に実行する。Step 0（ブランチ作成）は
  コンダクタが完了済み（ブランチ `bolt/b2-u10-ci-governance`、`origin/main` = PR #24 squash-merge 後）。Step 10（ruleset の
  PUT 実行）と Step 11（Bolt ゲート / PR）は担当外 — ruleset スクリプトは書いて `--dry-run` まで。
- **プロダクトコード（`modules/**/src/`）は触らない。** 触ってよいのは `.github/workflows/ci.yml`、`Cargo.toml`（`[workspace.lints.rust]` のみ）、
  `tools/lint/Cargo.toml`、`rust-toolchain.toml`（新規）、`scripts/coverage.sh`、`scripts/governance/`（新規）。
- `code-generation-plan.md` / `code-generation-questions.md` / `unit-test-instructions.md` は承認指紋の対象なので**1 バイトも変更しない**
  （チェックボックスも触らない）。他の `aidlc/` 配下（メモリ・状態・監査）も編集しない。`aidlc-*.ts` の状態変更コマンドは呼ばない。
- Testing Contract（§5）は Part 2 の権威。packaging への写し（計画 §3）: `scripts/governance/verify-ci-governance.sh` を先に書いて
  現状ツリーで **Red**（失敗項目一覧と終了コード）を記録 → 設定変更で **Green** → Refactor。Red / Green の出力は最終返答に写す。
- 意味単位でコミット（`chore(ci): …` / `chore(toolchain): …` / `chore(coverage): …` / `chore(governance): …`）。**push / PR /
  ブランチ切替はしない。** 設計判断を変えたくなったら推測で進めず「要確認」として返答に書く。
- 終了時の返答（最終テキスト = 戻り値）は §8 の形で書き、同じ内容を
  `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/code-generation/developer-report-3.md`
  にも保存する（通知経路に依存しないため）。

## 1. ワークスペース（実地の現状、2026-08-23）

- Rust 1.95.0（ローカル）、cargo 1.95.0、`rust-toolchain.toml` 不在、CI は `dtolnay/rust-toolchain@stable`。bun 1.3.13、jq、gh（認証済み）。
- `.github/workflows/ci.yml`: `on: pull_request(main) + workflow_dispatch`、`permissions` 未指定、3 ジョブ（check / quint / coverage）、
  `Swatinem/rust-cache@v2`（workspaces: `. -> target`、`tools/lint -> target`）、`taiki-e/install-action@v2`（cargo-llvm-cov）。
- `scripts/coverage.sh`: `ABSOLUTE_THRESHOLD=90.0`、`TOLERANCE=0.5`、`measure_line_coverage()` が `cargo llvm-cov --workspace --json --summary-only`
  を実行。除外設定なし。`--base <ref>` で相対ゲート。
- `Cargo.toml`: `[workspace.lints.rust]` に `missing_docs` / `unsafe_op_in_unsafe_fn` / `dropping_copy_types` / `unreachable_pub`（`unsafe_code`
  は未昇格）。`tools/lint/Cargo.toml`: detached（`[workspace]` 空）、`[lints.rust] missing_docs = "deny"`。`.cargo/config.toml` の
  `lint` alias。`tools/lint` のテストはインライン（`cargo test --manifest-path tools/lint/Cargo.toml`）。
- `modules/app/aidlc/src/main.rs`: composition root（現状 `const fn main() {}` のスタブ）。
- proptest 1.11.0: `PROPTEST_RNG_SEED`（`RngSeed::Fixed(u64)`）を環境変数で受ける（`config.rs:40`）— シード固定はテストコード変更なしで可能。
- GitHub ruleset「main」（id 21190453、active）: `deletion` / `non_fast_forward` / `merge_queue`（SQUASH、ALLGREEN、同時 1 件）、
  `required_status_checks` **無し**、`bypass_actors: []`。`gh api repos/amadeus-dlc/amadeus-ng/rulesets/21190453` で読める。
  ruleset の `PUT /repos/{owner}/{repo}/rulesets/{id}` は `rules[]` を**全置換**するため、既存 3 規則を維持した JSON を組み立てること。
- 既存テスト: ワークスペース 338、`tools/lint` 31（インライン）。現状ゲート: fmt / clippy / lint / test 全緑。

## 2. 読むべき設計成果物（本 Unit のみ）

`aidlc/spaces/default/intents/260822-stage1-selfhost/` 配下:
- `construction/u10-ci-governance/nfr-requirements/security-requirements.md` — NFR2.1〜2.5 / NFR4.1〜4.5、合格基準、STRIDE
- `construction/u10-ci-governance/nfr-requirements/tech-stack-decisions.md` — 選定（ruleset / merge_group / toolchain / audit / lints / permissions / tools-lint / 除外 / PBT）
- `construction/u10-ci-governance/nfr-design/security-design.md` — CI 4 ジョブの形、ruleset 手順、ワークスペース設定、障害ドメイン（レビュー Minor 2: 除外 regex は `^modules/app/aidlc/src/main\.rs$` が正本 / 冪等判定はコンテキスト集合）
- `construction/u10-ci-governance/code-generation/code-generation-questions.md` — Q1 = A、Approve Plan
- `inception/requirements-analysis/requirements.md`（FR9.1〜9.5、NFR2 / NFR4）、`inception/units-generation/unit-of-work.md`（U10）、
  `inception/practices-discovery/evidence.md`（確定アクション 1〜4）

## 3. コーディング規則（正本、必読）

`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` の README と全ルール。本 Unit は Rust コードを書かないが、bash スクリプトにも
「小さく・読める・副作用は明示」を適用する: `set -euo pipefail`、関数分割、日本語メッセージ、`--dry-run` の既定安全側、`jq` で機械検証。

## 4. ルール束（org / team / project / construction phase — 逐語）

### aidlc/spaces/default/memory/team.md

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


### aidlc/spaces/default/memory/phases/construction.md

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


### aidlc/spaces/default/memory/org.md（抜粋: Way of Working / Testing Posture / Code Style — 全文は同ファイル）

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

## 5. Testing Contract（計画に埋め込み済み・権威）


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


## 6. 承認済み計画（code-generation-plan.md 全文）

# code-generation-plan — U10 CI ガバナンス（`u10-ci-governance`）

> Code Generation（Construction 3.5）の計画（Unit: U10、kind: packaging、Bolt: B2、規模 M）。出典:
> `../nfr-requirements/security-requirements.md`（NFR2.1〜2.5 / NFR4.1〜4.5）、`../nfr-requirements/tech-stack-decisions.md`、
> `../nfr-design/security-design.md`（CI 4 ジョブ・ruleset 手順・ワークスペース設定・障害ドメイン、レビュー Minor 2 件）、
> `../../../inception/requirements-analysis/requirements.md`（FR9.1〜9.5、NFR2 / NFR4）、`../../../inception/units-generation/
> unit-of-work.md`（U10 の責務・境界・合格）、`../../../inception/contract-design/contract-summary.md`（U10 は契約面を
> 持たない）、`../../../inception/delivery-planning/bolt-plan.md`（B2 = U10、2026-08-23 改訂）、`aidlc/spaces/default/knowledge/
> aidlc-shared/coding-rules/`、`code-generation-questions.md`（Q1、Plan Approval）。
>
> 実装はワークスペースルート（`.github/workflows/ci.yml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`rust-toolchain.toml`、
> `scripts/coverage.sh`、`scripts/governance/`）に書く。記録ディレクトリにはコードを置かない。**プロダクトコードは触らない**
> （`unsafe_code` 昇格で赤になるクレートがあれば U7 — 現状 unsafe 使用ゼロ）。

## 1. 前提と範囲

- **作るもの**: FR9.2（toolchain 固定・`unsafe_code` forbid 昇格・`permissions`・`cargo audit`）→ FR9.3（`tools/lint` の CI 3 ステップ）
  → FR9.4（PBT シード固定・`TOLERANCE` 0.01）→ FR9.5（カバレッジ除外）→ FR9.1（ruleset に required checks + `merge_group`
  トリガ）の順（story map の U10 実装順）。加えて検証スクリプト `scripts/governance/verify-ci-governance.sh`（packaging の
  「テスト」— 設定の事実を機械検査する）。
- **作らないもの**: FR9.6（エラーハンドリング規則の正本化 — U9）、Dependabot / SHA ピン留め（後続 intent）、`audit` の required 化
  （運用後に再判断）、macOS CI / push トリガ（後続 intent）。
- **ブランチ（Q1）**: PR は直列運用（team.md）。B2 のブランチ `bolt/b2-u10-ci-governance` は **PR #24（B1）が `main` に
  squash-merge された後に `main` から切る**（Q1 = A）。#24 マージ前は計画承認まで進め、実装の委任はマージ後に行う。
- **GitHub 設定の実行**: ruleset 変更はオーナー権限が要る。開発エージェントは手順スクリプトを書き `--dry-run` と `jq` 検証まで行い、
  **実行はオーナー**（または `gh auth` がオーナー権限ならコンダクタが実行を提案して承認後に実行）。実行結果（前後 JSON）は
  `<record>/construction/u10-ci-governance/code-generation/ruleset/` に保存する。
- **受入**（unit-of-work U10）: CI 3 ジョブ緑、`cargo audit` clean、`gh api` で required checks が確認できる。正常系: 緑 PR が
  merge queue を通って squash-merge 完走（本 Bolt の PR 自身で確認）。

## 2. 変更の一覧（設計の写し — 実装の契約）

| 対象 | 変更 | 要求 |
|---|---|---|
| `rust-toolchain.toml`（新規） | `[toolchain] channel = "1.95.0"`、`components = ["rustfmt", "clippy", "llvm-tools"]`、`profile = "minimal"` | NFR4.2 |
| `Cargo.toml` | `[workspace.lints.rust]` に `unsafe_code = "forbid"` | NFR4.3 |
| `tools/lint/Cargo.toml` | `[lints.rust]` に `unsafe_code = "forbid"` | NFR4.3 |
| `.github/workflows/ci.yml` | `on:` に `merge_group: {}`; 直下に `permissions: contents: read`; toolchain を `dtolnay/rust-toolchain@master`（`components:` 撤去、`rust-toolchain.toml` 駆動）; `check` に `tools/lint` の fmt / clippy / test 3 ステップ; `coverage` は `pull_request` 時のみ `--base`、それ以外は絶対; 新規 `audit` ジョブ（`taiki-e/install-action@v2` `tool: cargo-audit` → `cargo audit` → `cargo audit --file tools/lint/Cargo.lock`） | NFR2.2 / 2.3 / 4.1 / 4.2 / 4.4 |
| `scripts/coverage.sh` | `cargo llvm-cov` に `--ignore-filename-regex '^modules/app/aidlc/src/main\.rs$'`（相対パス基準 — tech-stack-decisions の表記を正本とする、nfr-design レビュー Minor 1）; `TOLERANCE=0.01`; `PROPTEST_RNG_SEED`（固定値、例 `20260823`）を計測時に export; コメントの較正根拠を更新 | NFR2.4 / 2.5 |
| `.github/workflows/ci.yml`（`check` / `coverage`） | `env: PROPTEST_RNG_SEED: "20260823"` を `cargo test` / coverage 実行に適用（ローカルと CI で同じシード） | NFR2.4 |
| `scripts/governance/ruleset-required-checks.sh`（新規） | `gh api` で ruleset「main」の前 JSON 取得 → `required_status_checks`（check / quint / coverage、strict）が**期待どおりのコンテキスト集合で**無ければ追加 / 補正した JSON を `PUT`（既存規則・`bypass_actors` 維持）→ 後 JSON 取得 → `jq` 検証。`--dry-run`（PUT しない）と `--out-dir <dir>`（前後 JSON の保存先）。冪等判定はコンテキスト集合の一致（nfr-design レビュー Minor 2） | NFR2.1 / 4.5 |
| `scripts/governance/verify-ci-governance.sh`（新規） | packaging の「テスト」: (1) `rust-toolchain.toml` の channel / components、(2) `Cargo.toml` と `tools/lint/Cargo.toml` の `unsafe_code = "forbid"`、(3) `ci.yml` の `merge_group` / `permissions` / `audit` ジョブ / `tools/lint` 3 ステップ / `@master` toolchain、(4) `scripts/coverage.sh` の `TOLERANCE=0.01` / 除外 regex / `PROPTEST_RNG_SEED`、(5) `--with-ruleset` 指定時のみ `gh api` で required checks の存在（ネットワーク要）を検査し、失敗項目を列挙して非 0 終了 | 全要求の機械検査 |

## 3. テスト戦略（Testing Contract: tdd / standard の packaging への適用）

packaging Unit にはプロダクトコードの「層」が無い。Testing Contract の `plan_profile` を次のように写す（方法論は変えない）:

- **ランナー**: bash（`scripts/governance/verify-ci-governance.sh`）+ `cargo test --manifest-path tools/lint/Cargo.toml`（既存 31 本）+
  `bash -n` 構文検査。最初の Red の前にランナーが走ることを確認（brownfield: `bash -n scripts/coverage.sh` は現時点で exit 0）。
- **Red**: `verify-ci-governance.sh` を先に書き、現在のツリーに対して実行して**失敗項目一覧**（toolchain ファイル無し・
  `unsafe_code` 未昇格・`merge_group` 無し・`permissions` 無し・`audit` 無し・`tools/lint` ステップ無し・`TOLERANCE=0.5`・除外無し・
  `PROPTEST_RNG_SEED` 無し）を記録する。
- **Green**: 変更を順に入れ、失敗項目が 1 つずつ消えることを記録。全項目 PASS で Green。
- **Refactor**: スクリプトの重複整理、メッセージの日本語化、`shellcheck`（導入済みなら）。
- **受入（Bolt の外側のゲート）**: CI 3 ジョブ + `audit` の実行結果、`scripts/coverage.sh` 2 回実行で差 0.00pp（NFR2.4）、
  ruleset 変更後の正常系 PR 完走。
- Standard 戦略の「コンポーネントごと 5〜8 本」は、`verify-ci-governance.sh` の検査項目（対象ファイルごとに 2〜5 項目、
  合計 15 項目以上）と `tools/lint` 既存テスト 31 本で満たす。

## 4. 実装ステップ（番号順）

### 4.0 コンダクタ（承認後・委任前）

- [ ] Step 0. PR #24 のマージを確認 → `git switch main && git pull` → `bun .claude/tools/aidlc-bolt.ts start --name B2 --batch 1` →
      `git switch -c bolt/b2-u10-ci-governance`。

### 4.1 骨格とランナー（開発エージェント）

- [ ] Step 1. `scripts/governance/` を作り、`verify-ci-governance.sh` の骨格（検査関数の枠、`--with-ruleset` オプション、
      結果一覧と終了コード）を書く。`bash -n` で構文確認。`cargo test --manifest-path tools/lint/Cargo.toml` が走ることを確認
      （既存 31 本緑）。
- [ ] Step 2. **Red**: `verify-ci-governance.sh` の検査項目（§2 の (1)〜(4)）を実装し、現在のツリーで実行 → 失敗項目一覧を記録
      （期待: toolchain / unsafe / merge_group / permissions / audit / tools-lint-steps / tolerance / ignore-regex / proptest-seed
      の 9 項目が FAIL）。

### 4.2 FR9.2 サプライチェーン（Green 1）

- [ ] Step 3. `rust-toolchain.toml` 新規、`Cargo.toml` と `tools/lint/Cargo.toml` に `unsafe_code = "forbid"`、`ci.yml` に
      `permissions: contents: read` と toolchain の `@master` 化、`audit` ジョブ追加。`cargo build --workspace` と
      `cargo clippy --workspace --all-targets -- -D warnings` が緑（`tools/lint` も `--manifest-path` で確認）。検査を再実行し
      該当 5 項目が PASS に変わることを記録。

### 4.3 FR9.3 tools/lint の CI（Green 2）

- [ ] Step 4. `check` ジョブに `tools/lint` の fmt / clippy / test 3 ステップを追加。ローカルで同じ 3 コマンドを実行して緑。
      検査 1 項目 PASS。

### 4.4 FR9.4 / FR9.5 カバレッジ（Green 3）

- [ ] Step 5. `scripts/coverage.sh`: `PROPTEST_RNG_SEED` の export、`--ignore-filename-regex`、`TOLERANCE=0.01`、コメント更新。
      `ci.yml` の `check` / `coverage` に `PROPTEST_RNG_SEED` env。ローカルで `bash scripts/coverage.sh` を 2 回実行し `head`
      値が一致（差 0.00pp）することを記録（NFR2.4 の受入）。検査 3 項目 PASS。

### 4.5 FR9.1 ruleset と merge queue（Green 4）

- [ ] Step 6. `ci.yml` に `merge_group: {}` と coverage の条件分岐（`pull_request` のみ `--base`）。検査 1 項目 PASS。
- [ ] Step 7. `scripts/governance/ruleset-required-checks.sh` を書く（`--dry-run` / `--out-dir`、コンテキスト集合で冪等判定、
      前後 JSON 保存、`jq` 検証）。`--dry-run` で組み立て JSON を確認（PUT はしない）。`verify-ci-governance.sh --with-ruleset` は
      この時点では FAIL のまま（実行前）— 記録。
- [ ] Step 8. **Refactor**: スクリプト整理、`bash -n`、`shellcheck`（あれば）。全検査（ruleset 以外）PASS を再確認。

### 4.6 品質ゲートとコミット

- [ ] Step 9. `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` → `cargo test --workspace`
      → `tools/lint` 3 コマンド → `bash scripts/coverage.sh`（絶対ゲート）。意味単位でコミット（`chore(ci): …` / `chore(toolchain): …` /
      `chore(coverage): …` / `chore(governance): …`）。push / PR はしない。

### 4.7 コンダクタ / オーナー（委任後）

- [ ] Step 10. ruleset 変更の実行: `scripts/governance/ruleset-required-checks.sh --out-dir <record>/construction/u10-ci-governance/
      code-generation/ruleset/` をオーナー権限で実行（コンダクタが `gh auth status` でオーナー権限を確認できれば承認後に実行）。
      `verify-ci-governance.sh --with-ruleset` PASS を記録。
- [ ] Step 11. Bolt ゲート → PR 作成（`bolt/b2-u10-ci-governance` → `main`）→ PR の CI で `merge_group` を含む 4 ジョブの実行を確認 →
      オーナーが merge queue でマージ（正常系の受入）。

## 5. トレーサビリティ（要求 → ステップ）

| 要求 | ステップ | 主な成果物 |
|---|---|---|
| FR9.1 / NFR2.1 / NFR4.5 | 6, 7, 10, 11 | `scripts/governance/ruleset-required-checks.sh`、`ci.yml`（merge_group）、ruleset 前後 JSON |
| FR9.2 / NFR4.1 / NFR4.2 / NFR4.3 / NFR4.4 | 3 | `rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`ci.yml`（permissions / audit / toolchain） |
| FR9.3 / NFR2.3 | 4 | `ci.yml`（check ジョブ） |
| FR9.4 / NFR2.4 | 5 | `scripts/coverage.sh`（TOLERANCE / PROPTEST_RNG_SEED）、`ci.yml`（env） |
| FR9.5 / NFR2.5 | 5 | `scripts/coverage.sh`（ignore regex） |
| NFR2.2 | 6 | `ci.yml`（merge_group + coverage 分岐） |
| 全要求の機械検査 | 1, 2, 8 | `scripts/governance/verify-ci-governance.sh` |

## 6. 委任の形

- 1 回の委任（Step 1〜9）を aidlc-developer-agent へ。冒頭行 `AIDLC-UNIT: u10-ci-governance` と `AIDLC-TESTING-CONTRACT: <contract_sha256>`。
  委任は PR #24 のマージ後（Q1 = A）。計画・質問票・本ファイルのバイト列は承認後に変更しない（指紋）。
- ruleset の実行（Step 10）はオーナー権限の操作のため委任しない。

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


## 7. 承認済み単体テスト手順（unit-test-instructions.md 全文）

# unit-test-instructions — U10 CI ガバナンス（`u10-ci-governance`）

> Code Generation（Construction 3.5）の単体テスト手順（Unit: U10、kind: packaging）。出典: `code-generation-plan.md`
> （Testing Contract: methodology tdd / strategy standard / scope classic、§3 テスト戦略）、`aidlc/spaces/default/memory/team.md`
> Testing Posture、`../nfr-requirements/security-requirements.md`（NFR2.1〜2.5 / NFR4.1〜4.5 の合格基準）、
> `../nfr-design/security-design.md`。
>
> **すべてのコマンドは本 Unit に限定する。** packaging Unit のため「単体テスト」= 設定の事実を機械検査するスクリプトと、
> 本 Unit が CI に組み込む `tools/lint` の既存テスト。ワークスペース全体の `cargo test --workspace` は品質ゲート（計画 Step 9）で
> あり本ファイルの Unit 限定コマンドではない。

## 1. フレームワークと設定

- ランナー: bash（`scripts/governance/verify-ci-governance.sh` — 本 Unit で新規作成する検査スクリプト）。追加の
  設定ファイル不要。`jq` と `gh`（`--with-ruleset` 時のみ）を使う。
- `tools/lint` の自己テスト: Rust 標準 `cargo test`（`--manifest-path tools/lint/Cargo.toml`、既存 31 本のインラインテスト）。
- 構文検査: `bash -n`（`shellcheck` が導入済みなら併用 — 任意）。

## 2. 実行コマンド（本 Unit 限定）

最初の Red の前に走ることを確認済み（brownfield 実測 2026-08-23: `bash -n scripts/coverage.sh` exit 0、
`cargo test --manifest-path tools/lint/Cargo.toml` は CI 外だがローカルで実行可能）:

```bash
bash -n scripts/coverage.sh scripts/governance/verify-ci-governance.sh scripts/governance/ruleset-required-checks.sh
bash scripts/governance/verify-ci-governance.sh                 # 設定の機械検査（ruleset 以外）— Red → Green の主体
bash scripts/governance/verify-ci-governance.sh --with-ruleset  # 上記 + gh api で ruleset の required checks（ネットワーク要）
cargo test --manifest-path tools/lint/Cargo.toml                # tools/lint 自己テスト（CI 組込み対象）
```

Red の記録: 変更前のツリーで `verify-ci-governance.sh` を実行し、失敗項目一覧（終了コード非 0）を `code-summary.md` に写す。
Green は同コマンドの PASS 一覧。

## 3. 期待するテスト量と受入

- 検査項目: 対象ファイルごとに 2〜5 項目、合計 15 項目以上（`rust-toolchain.toml` 3 / `Cargo.toml` 1 / `tools/lint/Cargo.toml` 1 /
  `ci.yml` 6 / `scripts/coverage.sh` 3 / ruleset 1）。`tools/lint` 既存 31 本は緑のまま。
- 受入（Bolt の外側）: `scripts/coverage.sh` を 2 回実行して `head` の line coverage が一致（差 0.00pp — NFR2.4）、
  CI 4 ジョブ（check / quint / coverage / audit）緑、ruleset 変更後に `verify-ci-governance.sh --with-ruleset` PASS、
  緑 PR が merge queue を通って squash-merge 完走（NFR2.1 正常系）。
- カバレッジ: 本 Unit はプロダクトコードを変更しないためワークスペースのカバレッジ値は不変（97.06% 付近）。除外設定により
  `main.rs` が計測対象から外れる分の差は code-summary に記録する。

## 4. モック・スタブの方針

- 使わない。検査対象は実ファイルと実 ruleset。`--with-ruleset` を付けない既定では GitHub へアクセスしない（ネットワーク不要）。
- ruleset 変更スクリプトの `--dry-run` は PUT を行わず組み立て JSON を出力する（副作用なし）。

## 5. テストデータ

- 検査の期待値はスクリプト内の定数（channel `1.95.0`、`TOLERANCE=0.01`、除外 regex、required コンテキスト `check` / `quint` /
  `coverage`、`PROPTEST_RNG_SEED` の固定値）。変更は PR でのみ。
- ruleset の前後 JSON は `<record>/construction/u10-ci-governance/code-generation/ruleset/{before,after}.json` に保存（秘密情報なし）。

## 8. 返答（戻り値）の形

最終テキストは次の見出しで（日本語）。同じ内容を `developer-report-3.md`（§0 のパス）にも保存する:

1. **実行した Step と結果**（Step 1〜9 それぞれ: 完了 / 未完了 + 理由）
2. **作成・変更ファイル一覧**（ワークスペース相対パス）
3. **TDD の証跡**（`verify-ci-governance.sh` の Red 出力（失敗項目一覧・終了コード）→ 各 Green 後の出力、`tools/lint` テストの要約行）
4. **受入の実測**（`bash scripts/coverage.sh` 2 回の `head` 値と差、`cargo audit` の結果（導入できれば）、`tools/lint` 3 コマンドの結果）
5. **ruleset スクリプトの `--dry-run` 出力**（組み立て JSON の要約 — 既存 3 規則 + required_status_checks）
6. **計画からの逸脱・設計判断**（あれば。なければ「なし」）
7. **品質ゲートの結果**（fmt / clippy / cargo lint / cargo test --workspace の最終実行結果）
8. **コミット一覧**（`git log --oneline origin/main..HEAD`）
9. **未解決・要確認事項**（例: `dtolnay/rust-toolchain@master` の挙動確認、`audit` の required 化判断）
