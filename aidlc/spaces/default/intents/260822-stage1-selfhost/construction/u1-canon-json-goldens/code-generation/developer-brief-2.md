AIDLC-UNIT: u1-canon-json-goldens
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

# 開発ブリーフ（委任 2 / 2）— U1 CLI / フック実行出力ゴールデンの採取、計画 Step 17〜19

Conversation language: 日本語（会話・人間可読成果物・コメント・rustdoc・コミット本文の説明は日本語。コード識別子・
固定トークン・コミット件名の型（feat/test/chore）は英語のまま）

## 0. あなたの役割と境界

- あなたは aidlc-developer-agent。承認済み計画（下記 §6、**Step 17〜19 だけ**）を順に実行する。Step 1〜16 は
  委任 1 が完了済み（ブランチ `bolt/b1-u1-canon-json-goldens` に 6 コミット: workspace 配線 / hash-canonical
  受入表採取 / canon-json 実装 / PBT / to_value + ファサード / contract-observed ケース）。**canon-json 本体と
  hash-canonical コーパスは変更しない**（比較器の追加・README の節追記は可）。
- 委任 1 の成果を先に把握する: `git log --oneline main-sync..HEAD`、`tests/golden/upstream-3c3146cf/README.md`
  （採取ゴールデン節・正規化規則節）、`tests/golden/upstream-3c3146cf/normalization.json`、
  `scripts/goldens/recapture-hash-canonical.sh` と `capture-hash-canonical.ts`（同じ流儀 — 取得 → sha256 照合 →
  実行 → 来歴 — で CLI / フック採取を作る）、`modules/shared/canon-json/tests/golden_hash_canonical.rs`（テスト側の
  コーパス読取の流儀）。
- 計画ファイル `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u1-canon-json-goldens/code-generation/code-generation-plan.md`
  の Step 17〜19 のチェックボックスを完了時に `[x]` にする。他の記録ファイルは編集しない。
- 下記 §5 の Testing Contract は Part 2 の権威。比較器の Red → Green → Refactor を守る（テスト支援コードも
  テストファーストで）。
- 意味単位でコミット（`test(goldens): …`、`chore(scripts): …`）。**push / PR / ブランチ切替はしない。**
  `aidlc-*.ts` の状態変更コマンドは呼ばない。
- 採取できない遷移・ケースは `cases-missing.json` に理由付きで記録する — **捏造しない**。upstream ピンの
  コードを bun で**実行して**採る（現在インストール済みの 2.6.54 シェルは使わない）。
- 終了時の返答（最終テキスト = 戻り値）は §8 の形で書く。

## 1. ワークスペース（委任 1 から更新）

- ブランチ `bolt/b1-u1-canon-json-goldens`、作業ディレクトリ `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`。
- bun 1.3.13、node 24、git、curl、ネットワーク可。upstream ピン `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820`（v2.6.40）。
  ピンの取得: `git init && git remote add origin https://github.com/awslabs/aidlc-workflows && git fetch --depth 1
  origin <sha> && git checkout FETCH_HEAD`（GitHub は SHA 指定の shallow fetch を許可）。失敗時は raw 取得へ
  フォールバック。配布シェルは `dist/claude/`（`.claude/tools/*.ts`、`.claude/hooks/*.ts`、`.claude/tools/data/`）。
- upstream の CLI 面・フックの仕様は `docs/upstream/specs/`（`02-orchestration-engine.md`、`03-state-audit-runtime.md`、
  `07-hooks.md`、`09-cli-tools.md`）。本プロジェクトの契約面は
  `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md` C1（CLI 面）・
  C2（フック 4 本: stop-forwarding-loop / record-human-turn / state-transition-guard / write-audit-log — upstream の
  対応フックファイル名は dist を見て写像し README に表で記す）・C7（ゴールデン layout）。
- 正規化（BR2.2）: `<TS>`（ISO 8601 UTC）、`<CLONE>`（`<host>-<clone>` シャード名）、`<ROOT>`（作業ツリー絶対パス）、
  `<SESSION>`（セッション ID）— 規則の正本は `tests/golden/upstream-3c3146cf/normalization.json`。必要なら規則を
  **追加**してよい（既存規則の意味は変えない）。
- 既存ファイルのバイト不変: `tests/golden/upstream-3c3146cf/{stage-graph.json,scope-grid.json}`、README は節の追記のみ。
  `hash-canonical/` 配下は変更しない。

## 2. 読むべき設計成果物（本 Unit のみ）

`aidlc/spaces/default/intents/260822-stage1-selfhost/` 配下:
- `construction/u1-canon-json-goldens/functional-design/functional-spec.md` — W4（採取）・W5（比較）、エラー経路
- `construction/u1-canon-json-goldens/functional-design/rules.md` — BR2.1〜BR2.5（特に BR2.4 の範囲）
- `construction/u1-canon-json-goldens/functional-design/entities.md` — GoldenCase / NormalizationRule / GoldenCorpus
- `construction/u1-canon-json-goldens/nfr-requirements/security-requirements.md` — NFR1.3 / NFR4.4（秘密情報・PII・環境固有値を含めない）
- `construction/u1-canon-json-goldens/nfr-design/logical-components.md` — 比較器の置き場（テスト支援、プロダクトに混ぜない）
- `inception/contract-design/contract-summary.md` — C1 / C2 / C7
- `inception/requirements-analysis/requirements.md` — FR7.2

## 3. コーディング規則（正本、必読）

`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` の README と全ルールファイルを読む。テスト支援コードにも
`unreachable_pub` / `missing_docs`（テストは対象外だが統合テスト側の pub 要素に注意）・フィールド private の規則を
適用し、`clippy --all-targets -D warnings` を緑に保つ。

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

# code-generation-plan — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Code Generation（Construction 3.5）の計画（Unit: U1、kind: library、Bolt: B1、規模 M）。出典:
> `../functional-design/functional-spec.md`（W1〜W5、インターフェイス）、`../functional-design/rules.md`（BR1.1〜BR1.8 /
> BR2.1〜BR2.5）、`../functional-design/entities.md`、`../nfr-requirements/security-requirements.md`（NFR1.x / NFR2.x /
> NFR4.x）、`../nfr-requirements/tech-stack-decisions.md`、`../nfr-design/security-design.md`、
> `../nfr-design/logical-components.md`、`../../../inception/contract-design/contract-summary.md`（C7 — Q2 = A で layout
> 改訂済み）、`../../../inception/units-generation/unit-of-work.md`（U1）、`../../../inception/requirements-analysis/
> requirements.md`（FR7.1〜7.3、NFR1/NFR2/NFR4）、`docs/adr/0001-canonical-json-serializer.md`、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（全規則）、`code-generation-questions.md`（Q1 = A、Q2 = A）。
>
> 実装はワークスペースルート（`modules/shared/canon-json/`、`tests/golden/upstream-3c3146cf/`、`scripts/goldens/`）に書く。
> 記録ディレクトリにはコードを置かない。brownfield: 既存ファイルはその場で変更し、複製ファイルを作らない。

## 1. 前提と範囲

- **作るもの**: (1) `canon-json` クレートの実体化（既存スタブ `modules/shared/canon-json/`）。(2) upstream ピン
  `3c3146cf`（v2.6.40）から採取したゴールデン（hash-canonical 受入表 = FR7.1、CLI / フック実行出力 = FR7.2）と
  再採取スクリプト。(3) ゴールデン比較器（テスト側）。
- **作らないもの**: 他 Unit のコード（U3 の WorkflowDefinitionRepositoryImpl の `serde_json` → canon-json 置換、
  U6 の continue_token、U7 の CLI）。ADR 0001「未確定事項」の確定記述は U9（canon-docs）へ引き渡す（採取値は
  ゴールデン README に記録して渡す）。`cargo audit` / `rust-toolchain.toml` / `unsafe_code` の workspace lint 昇格は U10。
- **ブランチ（Q1 = A）**: `main-sync` から `bolt/b1-u1-canon-json-goldens` を切る。最初のコミットは aidlc 記録
  （`aidlc/` 配下）のみ、以降はコードのコミット。PR は Bolt ゲート承認後にコンダクタが 1 本だけ開く（直列運用）。
  開発エージェントは push / PR を行わない。
- **ゴールデン配置（Q2 = A）**: `tests/golden/upstream-3c3146cf/{hash-canonical,cli,hooks}/`。既存の
  `stage-graph.json` / `scope-grid.json` と README は同ディレクトリ直下のまま**バイト不変**（README の規定）。
  README には節を追記する（既存文のバイトは変えない）。
- **採取環境**: bun 1.3.13、ネットワーク有り（`raw.githubusercontent.com/awslabs/aidlc-workflows/3c3146cf…` は
  HTTP 200 を実測）。upstream ピンのコードを**実行して**採取する（BR2.1）。採取に失敗したケースは欠落として
  記録し、捏造しない（W4 のエラー経路）。
- **コーディング規則**（正本 `coding-rules/`）: フィールド既定 private（アクセサ公開）、モジュール既定 private
  （公開は `lib.rs` の `pub use` 列挙のみ、利便再エクスポート禁止）、ドメイン同値は `PartialEq`/`Eq`、`unwrap` /
  `expect` はプロダクトコード禁止、`missing_docs` deny（全公開要素に rustdoc）、thiserror / anyhow は使わず手実装
  エラー enum + `fmt::Display`。Tell-Don't-Ask。Repository 語彙の造語禁止（本 Unit に Repository は無い）。

## 2. 公開 API（設計の写し — 実装の契約）

`modules/shared/canon-json/src/lib.rs` が公開するのは次だけ（`pub use` 列挙。モジュール `value` / `profile` / `writer` /
`canonical` / `digest` / `parse` はすべて private）:

```text
JsonValue { Null, Bool(bool), Number(Number), String(String), Array(Vec<JsonValue>), Object(ObjectMembers) }
Number { PosInt(u64), NegInt(i64), Float(f64) }            // 非負は u64 優先、負は i64、小数・非有限は f64
ObjectMembers                                               // 挿入順保持・キー一意（同名挿入は値を置換し位置は維持 = JS）
SerializationProfile { ContractPretty, ContractCompact, HashCanonical }  // indent() / trailing_newline() / key_order() / purpose()
Digest { family(), hex(), rendered() }   DigestFamily { CanonicalPrefixed, CompactRaw }
ParseError { Syntax { offset, detail }, TooDeep { limit }, Encoding }   ToValueError
serialize(&JsonValue, SerializationProfile) -> String
hash_canonical(&JsonValue) -> Digest            // "sha256:" + hex（hash-canonical 出力の UTF-8 バイト列）
hash_compact(&JsonValue) -> Digest              // 生 hex（contract-compact 出力の UTF-8 バイト列）
parse(&str) -> Result<JsonValue, ParseError>    // 挿入順保持、深さ上限 128（TooDeep）
parse_bytes(&[u8]) -> Result<JsonValue, ParseError>   // 不正 UTF-8 → Encoding
to_value<T: Serialize>(&T) -> Result<JsonValue, ToValueError>   // 型付き struct → 宣言順（契約経路の唯一の変換点）
```

設計からの差分（記録）: functional-spec §2 は `to_value -> JsonValue` だが、`serde_json::to_value` は失敗し得る
（非文字列キーのマップ等）ため `Result` で返す（`unwrap` 禁止の帰結）。`parse_bytes` は `ParseError::Encoding` を
到達可能にするために追加する。

## 3. 規則の実装方針（BR → コード）

| 規則 | 実装 |
|---|---|
| BR1.1 / BR1.2 キー順 | contract-*: integer-like キー（0〜2^32-2 の正準十進表記）を数値昇順で先頭、残りは挿入/宣言順。hash-canonical: integer-like を数値昇順で先頭、残りを **UTF-16 コード単位順**（`encode_utf16` で比較）。upstream の `canonicalize` は `Object.keys().sort()` → `Object.fromEntries` なので、integer-like 先頭は JS のプロパティ順により hash-canonical でも成立する — ゴールデンで固定 |
| BR1.3 数値 | `PosInt` / `NegInt` は十進。ただし **|v| > 2^53** の整数は JS が f64 として扱うため f64 経路（`v as f64`）で書く（JS の丸めと一致）。`Float`: 非有限 → `null`、`0.0` / `-0.0` → `0`、それ以外は `format!("{:e}")` の最短桁 + 指数から ECMA-262 `Number::toString`（k, n 規則: 1e-6 ≤ \|x\| < 1e21 は非指数表記、それ以外は `d.ddde±N`、`e+` 書式）を組み立てる |
| BR1.4 エスケープ | `"`, `\\`, U+0000〜U+001F（`\b \f \n \r \t` 短縮、他は `\u00xx` 小文字 hex）。非 ASCII と `/` と U+2028/2029 は生出力。孤立サロゲートは Rust 文字列に存在しないため該当なし（入力側: serde_json は `"\ud800"` を拒否 → `ParseError::Syntax`。既知の非対称として README に記録、契約 JSON には現れない） |
| BR1.5 体裁 | pretty = 2 スペース・`"key": value`・メンバごと改行・末尾 `\n`・空は `[]` / `{}`。compact / canonical = 空白なし |
| BR1.6 ダイジェスト | `sha2::Sha256` で直列化文字列の UTF-8 バイト列をハッシュ。`Digest::rendered()` が族ごとの表記 |
| BR1.7 直接呼び出し禁止 | ルート `clippy.toml` の `disallowed-methods` に `serde_json::to_string` / `to_string_pretty` / `to_vec` / `to_vec_pretty` / `to_writer` / `to_writer_pretty` / `to_value` を登録（reason に canon-json 経由を明記）。canon-json 内の唯一の呼出点（`to_value`、`from_str` は禁止対象外）は `#[allow(clippy::disallowed_methods)]` + 理由コメント。既存コードに禁止関数の呼出があれば棚卸しして code-summary に列挙し、lint が赤になる箇所だけ最小修正（他 Unit の設計には踏み込まない） |
| BR1.8 preserve_order | `[workspace.dependencies]` に `serde_json = { version = "1", features = ["preserve_order"] }`・`serde = { version = "1", features = ["derive"] }`・`sha2`・`proptest` を置き、既存クレート（core-domain dev-dep、core-interface-adapter）も `serde_json.workspace = true` に揃える（フィーチャ統合をクレート単独ビルドでも保証） |
| NFR4.3 深さ上限 | `parse` 前に文字列リテラル外の `{` `[` を数える軽量スキャンで深さ > 128 を `TooDeep { limit: 128 }` として決定的に拒否（serde_json 既定の再帰上限と同値。`disable_recursion_limit` は使わない）。上限は `const` で 1 か所 |
| BR2.1 来歴 | 各コーパスに `provenance.json`（upstream commit、取得 URL、抽出スニペットの sha256、captured_at、command、bun version）。ケース単位でも `provenance` を持つ |
| BR2.2 正規化 | 比較器 `normalize()`: `<TS>`（ISO 8601 UTC）、`<CLONE>`（`<host>-<clone>` シャード名）、`<ROOT>`（作業ツリー絶対パス）、`<SESSION>`（セッション ID）。規則は `tests/golden/upstream-3c3146cf/normalization.json` に固定し、期待値・実測値の双方に適用 |
| BR2.3 受入表 | 入力クラス: ネスト / integer-like キー（混在・ソート） / 非有限数 / 負ゼロ / 指数表記（1e21, 1e-7, 123e-20 など閾値の両側） / 2^53 超の整数 / 非 ASCII 文字列 / エスケープ（制御文字・`"`・`\\`・`/`・U+2028） / 空の配列・オブジェクト / 型付き struct のフィールド順 / 浮動小数の整数値（1.0） |
| BR2.4 CLI 範囲 | next（開始）・report awaiting-approval / approved / rejected / revised・continue（load-steering）・skip・jump・park / unpark・recompose・set-autonomy、フック 4 本 × 許可 / 拒否 / 無視 2〜3 件 |
| BR2.5 更新方針 | README に「ピン更新 intent でのみ更新」を明記。更新手順 = 再採取スクリプト |

## 4. 棚卸し（code-generation で確定し code-summary に記録する事項）

- [ ] I1. 契約 JSON の実測最大ネスト深さ（`tests/golden/upstream-3c3146cf/*.json`、`.claude/tools/data/*.json`、
      `.claude/tools/data/scopes/`、ゴールデン入力）が 128 を十分下回ること（想定 10 段未満）。
- [ ] I2. 契約 JSON に integer-like キーが現れないこと（現れる場合は箇所と写像を記録）。
- [ ] I3. 契約 JSON のキーが ASCII のみであること（UTF-16 順 = バイト順の前提）。
- [ ] I4. 契約 JSON に浮動小数フィールドが現れないこと（現れる場合は一覧）。
- [ ] I5. ワークスペース内の `serde_json::to_*` / `to_value` 直接呼出の棚卸し（lint 導入の影響範囲）。
- [ ] I6. `preserve_order` 有効化で既存テスト（ITF 準拠 2 本の `serde_json::Value` 利用）が緑のままであること。
- [ ] I7. `components.md` の CanonJson `external_dependencies: []` を実依存（sha2 / serde / serde_json）へ更新
      （記録側、コンダクタが実施 — nfr-design レビュー Minor 1）。

## 5. 実装ステップ（TDD、レイヤーごとに Red → Green → Refactor）

Testing Contract の `plan_profile.steps` を基線とし、ライブラリに存在しない層（Repository / Frontend）は省く。
「Data model」= 値・プロファイル・ダイジェストの型、「Business logic」= writer / canonical / digest / parse、
「API」= ファサードと `to_value`。各 Red ステップでは失敗するコマンド出力（失敗テスト名と要約行）を
`code-summary.md` に記録してから Green に進む。

### 5.0 コンダクタ（承認後・委任前）

- [ ] Step 0. Bolt 開始と ブランチ: `bun .claude/tools/aidlc-bolt.ts start --name B1 --batch 1` →
      `git switch -c bolt/b1-u1-canon-json-goldens`（`main-sync` から）→ aidlc 記録を 1 コミット
      （`chore(aidlc): record inception and U1 design for stage-1 self-host`）。

### 5.1 骨格（開発エージェント — 委任 1）

- [ ] Step 1. プロジェクト構造と設定: `Cargo.toml` の `[workspace.dependencies]` に serde / serde_json(preserve_order) /
      sha2 / proptest を追加し、`core-domain`（dev-dep）・`core-interface-adapter` を `.workspace = true` に揃える。
      `modules/shared/canon-json/Cargo.toml` に `serde` / `serde_json` / `sha2`（runtime）、`proptest`（dev）を追加。
      `clippy.toml` に `disallowed-methods`（BR1.7）。`lib.rs` に private モジュール 6 本の空殻と `pub use` 列挙の枠。
      `cargo build -p canon-json` と `cargo clippy --workspace --all-targets -- -D warnings` が緑（I5 / I6 の棚卸しを
      ここで実施）。
- [ ] Step 2. テストランナー確認: `cargo test -p canon-json`（brownfield — 実測済み: 0 tests, exit 0）。
      統合テストの置き場 `modules/shared/canon-json/tests/` と、ゴールデンを `env!("CARGO_MANIFEST_DIR")/../../../
      tests/golden/upstream-3c3146cf/` で読む経路を決め、`unit-test-instructions.md` のコマンドで走ることを確認。

### 5.2 ゴールデン採取 — hash-canonical 受入表（FR7.1 / BR2.1 / BR2.3）

- [ ] Step 3. 再採取スクリプト: `scripts/goldens/recapture-hash-canonical.sh`（bash, `set -euo pipefail`）と
      `scripts/goldens/capture-hash-canonical.ts`（bun）。手順: 使い捨てディレクトリに upstream ピン
      `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820` の `dist/claude/.claude/tools/aidlc-testing-posture.ts` を取得 →
      `canonicalize` / `sha256` / `hashObject`（upstream 仕様 09-cli-tools.md §8.4 が指す `:104-123`）をスニペットとして
      抽出し sha256 で照合（期待値はスクリプトに固定） → `export` を付けた一時モジュールとして bun から import →
      入力クラス（§3 BR2.3 行）ごとに JS 値を評価し、`JSON.stringify(canonicalize(v))` / `hashObject(v)`（正準族）、
      `JSON.stringify(v)` と `sha256(JSON.stringify(v))` 生 hex（非正準族）、`JSON.stringify(v, null, 2) + "\n"`
      （pretty）を採る → `tests/golden/upstream-3c3146cf/hash-canonical/cases.json` と `provenance.json` に書く。
      入力は JSON テキスト（`input`）で表し、JSON で表せない NaN / ±Infinity のクラスだけ `input_js`（JS 式文字列）
      + Rust 側の構築手順（`construct`）を持つ。ケース ID は `hash-canonical/<class>/<case>`。
- [ ] Step 4. 採取の実行とレビュー: スクリプトを実行してコーパスを生成し、`git diff` で内容を目視（秘密情報・
      絶対パス無し）。`README.md` に「採取ゴールデン」節を追記（採取手順・来歴・正規化規則・更新方針 BR2.5・
      既知の非対称: 孤立サロゲート）。

### 5.3 canon-json — Data model 層（value / profile / digest の型）

- [ ] Step 5. Red: `JsonValue` / `Number` / `ObjectMembers`（挿入順・同名置換・アクセサ）、`SerializationProfile`
      3 値の属性（indent / trailing_newline / key_order / purpose）、`Digest` / `DigestFamily` の `rendered()`、
      `ParseError` / `ToValueError` の `Display` を対象に失敗テスト（各コンポーネント 5〜8 本）を書き、失敗出力を記録。
- [ ] Step 6. Green: 最小実装（フィールド private + アクセサ、`PartialEq` 導出、手実装 `Display`）。
- [ ] Step 7. Refactor: 命名・rustdoc（`missing_docs`）・`must_use` 整理。テスト緑のまま。

### 5.4 canon-json — Business logic 層（writer / canonical / digest / parse）

- [ ] Step 8. Red: (a) ゴールデン受入表テスト `tests/golden_hash_canonical.rs` — 全行で hash-canonical 出力と
      `sha256:` ダイジェスト、compact 出力と生 hex、pretty 出力を比較（失敗 = 行ごとの diff を表示）。
      (b) ユニット: 数値表記クラス（整数・2^53 超・1.0・1e21 / 1e-7 境界・負ゼロ・非有限）、エスケープクラス、
      キー順（integer-like 混在・UTF-16 順）、体裁（pretty の入れ子・空）、parse（不正 JSON の offset、深さ 128 超
      → TooDeep、`parse_bytes` の不正 UTF-8 → Encoding、重複キーは後勝ち・位置維持）。失敗出力を記録。
- [ ] Step 9. Green: writer（プロファイル分岐・数値ライタ・最小エスケープ・体裁）、canonical（再帰ソート）、
      digest（sha2）、parse（深さスキャン → `serde_json::from_str` preserve_order → `JsonValue` 変換）。
- [ ] Step 10. Refactor: 数値ライタの分離、重複排除、rustdoc。ゴールデン全行一致・ユニット緑のまま。
- [ ] Step 11. PBT（proptest、`src/` 同居）: 決定性（同入力 → 同出力）、`parse(serialize(v, compact)) == v`
      （NaN を含まない生成器）、`hash_canonical` の冪等性、canonical ソートの冪等性。ケース数は既定（シード固定は U10）。

### 5.5 canon-json — API 層（ファサードと to_value）

- [ ] Step 12. Red: `#[derive(Serialize)]` の struct が宣言順の `JsonValue` になること（ネスト・`Option` の `None`
      スキップ有無は serde の既定どおり — テストで固定）、`to_value` の失敗経路（非文字列キーのマップ → `ToValueError`）、
      ファサードが設計の列挙どおりの項目だけを公開していること（`lib.rs` を読む軽量テスト or doc test）。
- [ ] Step 13. Green: `to_value`（`serde_json::to_value` → `JsonValue` 変換、`#[allow(clippy::disallowed_methods)]`
      + 理由コメント）、`lib.rs` の `pub use` 列挙。
- [ ] Step 14. Refactor: クレート rustdoc（`//!`）に 3 プロファイル・2 族・禁止規則・深さ上限を記す。

### 5.6 棚卸しと品質ゲート（委任 1 の締め）

- [ ] Step 15. 棚卸し I1〜I6 を実施し結果を `code-summary.md` 用に報告（小さな検査テスト or bun/シェルのワンライナー。
      数値はすべて実測）。
- [ ] Step 16. 品質ゲート: `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →
      `cargo lint` → `cargo test --workspace` → `cargo llvm-cov -p canon-json --summary-only`（導入済みなら。
      canon-json は 100% 近傍を目標、床 90%）。コミットは意味単位（`feat(canon-json): …` / `test(goldens): …`）。

### 5.7 ゴールデン採取 — CLI / フック実行出力（FR7.2 / BR2.4 — 委任 2）

- [ ] Step 17. 再採取スクリプト `scripts/goldens/recapture-cli.sh` + `capture-cli.ts`: 使い捨てディレクトリに upstream
      ピンを取得（`git init && git fetch --depth 1 origin 3c3146cf… && git checkout FETCH_HEAD`、失敗時は raw 取得に
      フォールバック）→ `dist/claude/` を使い捨てワークスペースへ配置 → BR2.4 の主要遷移を bun で順に実行
      （`AIDLC_SKIP_HUMAN_PRESENCE_GUARD` 等、非対話化に必要な env を記録）→ 各ステップの stdout JSON・
      `aidlc-state.md` 差分・監査シャード差分を採り、`normalization.json` の規則で正規化 →
      `tests/golden/upstream-3c3146cf/cli/<verb>/<case>/{argv,stdin,stdout.json,state.diff,audit.md}`。
      フック 4 本（stop-forwarding-loop / record-human-turn / state-transition-guard / write-audit-log に対応する
      upstream のフックファイル）に代表 stdin JSON を与え `hooks/<hook>/<case>/{stdin.json,exit,stderr}`。
      非対話で再現できない遷移は**欠落として**`cases-missing.json` に理由つきで記録（捏造しない。後続 Bolt U6 / U7 で追加）。
- [ ] Step 18. 比較器の整備: `modules/shared/canon-json/tests/support/`（または dev-dependency のテスト支援モジュール）に
      `normalize()`・コーパス読取・行ごと diff を置き、cli / hooks 族は「読めて正規化できる」ことまでをテストで固定
      （実装突合せは U6 / U7）。README に cli / hooks 節を追記。
- [ ] Step 19. 品質ゲート再実行（Step 16 と同じ）とコミット。

## 6. トレーサビリティ（ストーリー → ステップ）

| 要求 / 規則 | ステップ | 主な成果物 |
|---|---|---|
| FR7.1 受入表採取 | 3, 4 | `scripts/goldens/recapture-hash-canonical.sh`, `capture-hash-canonical.ts`, `tests/golden/upstream-3c3146cf/hash-canonical/cases.json` |
| FR7.2 CLI / フックゴールデン | 17, 18 | `scripts/goldens/recapture-cli.sh`, `capture-cli.ts`, `tests/golden/upstream-3c3146cf/{cli,hooks}/` |
| FR7.3 canon-json 実装（受入表全行一致） | 5〜14 | `modules/shared/canon-json/src/*.rs`, `tests/golden_hash_canonical.rs` |
| BR1.1〜BR1.6 | 8〜10 | `writer.rs`, `canonical.rs`, `digest.rs` |
| BR1.7 | 1, 13 | `clippy.toml`, `value.rs`（allow 箇所） |
| BR1.8 | 1 | `Cargo.toml`（workspace.dependencies） |
| BR2.1 / BR2.2 / BR2.5 | 3, 4, 17, 18 | `provenance.json`, `normalization.json`, README |
| BR2.3 | 3, 8 | `cases.json`, `tests/golden_hash_canonical.rs` |
| BR2.4 | 17 | `tests/golden/upstream-3c3146cf/cli/`, `hooks/` |
| NFR1.1〜1.3 | 3, 8〜10, 18 | ゴールデン + 比較器 |
| NFR2.1〜2.3 | 5, 8, 11, 16 | Red 記録、カバレッジ、PBT |
| NFR4.1 / NFR4.2 | 1 | `Cargo.toml`（依存 3 つ）、`#![forbid(unsafe_code)]` 維持 |
| NFR4.3 | 8, 9 | `parse.rs`（深さスキャン・ParseError） |
| NFR4.4 | 4, 17 | 正規化・README・目視 |

## 7. 委任の形

- 委任 1（Step 1〜16）と委任 2（Step 17〜19）を同じ承認済み計画・同じ指紋の下で**直列に** aidlc-developer-agent へ
  委任する（1 回の委任に詰め込むと文脈が長すぎるため）。各委任の冒頭行は `AIDLC-UNIT: u1-canon-json-goldens` と
  `AIDLC-TESTING-CONTRACT: <contract_sha256>`。
- 失敗時はコンダクタが halt-and-ask（retry / skip / abort）を出す。

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

# unit-test-instructions — U1 canon-json とゴールデン（`u1-canon-json-goldens`）

> Code Generation（Construction 3.5）の単体テスト手順（Unit: U1、kind: library）。出典: `code-generation-plan.md`
> （Testing Contract: methodology tdd / strategy standard / scope classic）、`aidlc/spaces/default/memory/team.md`
> Testing Posture（TDD、カバレッジ 90% 床、`cargo test --workspace` / `cargo-llvm-cov`）、
> `../nfr-requirements/security-requirements.md`（NFR2.1〜2.3）、`../nfr-design/logical-components.md` §4（テスト配置）。
>
> **すべてのコマンドは本 Unit（クレート `canon-json`）に限定する。** Build and Test は Unit ごとにここのコマンドを
> 実行するため、`cargo test --workspace` のような全体コマンドは本ファイルには書かない（品質ゲートとしての全体実行は
> 計画 Step 16 / 19 の範囲）。

## 1. フレームワークと設定

- テストランナー: Rust 標準（`cargo test`）。追加設定ファイル不要（`Cargo.toml` の `[dev-dependencies]` に
  `proptest`（workspace 経由）を置く）。
- ユニットテスト: 各モジュールのインライン `#[cfg(test)] mod tests`（`clippy.toml` によりテスト内の `unwrap` /
  `expect` は許可）。
- 統合テスト（受入）: `modules/shared/canon-json/tests/golden_hash_canonical.rs`（受入表の全行比較）、
  `modules/shared/canon-json/tests/golden_corpus_read.rs`（cli / hooks コーパスの読取と正規化 — 委任 2 で追加）。
  テスト支援（コーパス読取・`normalize()`・行 diff）は `modules/shared/canon-json/tests/support/mod.rs`。
- PBT: `proptest`、各モジュールの `#[cfg(test)]` 内（決定性・往復・冪等性）。

## 2. 実行コマンド（本 Unit 限定）

最初の Red の前に走ることを確認済み（brownfield 実測 2026-08-22: `running 0 tests` / exit 0）:

```bash
cargo test -p canon-json
```

用途別:

```bash
cargo test -p canon-json --lib                          # インラインユニット + PBT のみ
cargo test -p canon-json --test golden_hash_canonical   # hash-canonical 受入表の全行比較（FR7.3 の合格判定）
cargo test -p canon-json --test golden_corpus_read      # cli / hooks コーパスの読取・正規化（委任 2 以降）
cargo test -p canon-json --doc                          # rustdoc 例
```

Red の記録: 失敗するテストを書いたら上記コマンドを実行し、`test result: FAILED. N passed; M failed` の要約行と失敗
テスト名を `code-summary.md` に写す（TDD の証跡、NFR2.1）。

## 3. 期待するテスト量とカバレッジ

- Standard 戦略: コンポーネント（value / profile / writer / canonical / digest / parse / facade+to_value / 比較器）
  ごとに 5〜8 本のユニットテスト、境界（ゴールデン読取）に統合テスト。目安 50〜70 本 + PBT 4 本 + 受入表の行数。
- カバレッジ: ワークスペース床 90%（`scripts/coverage.sh`）。canon-json 単体は 100% 近傍を目標:
  `cargo llvm-cov -p canon-json --summary-only`（`cargo-llvm-cov` 導入済みの環境で）。
- ゴールデン受入表は**全行一致**が合格（1 行でも不一致なら FR7.3 不合格。実装を直し、ゴールデンは直さない）。

## 4. モック・スタブの方針

- 使わない。canon-json は純粋関数群で外部 I/O を持たない。
- ゴールデンがオラクル（フィクスチャ）。ネットワークはテストでは使わない（再採取スクリプトのみが使う）。

## 5. テストデータ

- ゴールデン: `tests/golden/upstream-3c3146cf/hash-canonical/cases.json`（+ `provenance.json`）、
  `cli/<verb>/<case>/…`、`hooks/<hook>/<case>/…`、正規化規則 `normalization.json`。テストからは
  `concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests/golden/upstream-3c3146cf")` で解決する。
- ゴールデンは**読み取り専用**。更新は upstream ピン更新の intent でのみ（BR2.5）。
- PBT の生成器: NaN / ±Infinity を含まない `JsonValue`（往復性質用）と、含む生成器（非有限 → `null` の性質用）を
  分ける。失敗ケースは proptest の既定どおり `proptest-regressions/` に残す（コミット対象）。
- 一時ファイルは不要（`tempfile` は使わない）。

## 8. 返答（戻り値）の形

最終テキストは次の見出しで（日本語）:

1. **実行した Step と結果**（Step 1〜16 それぞれ: 完了 / 未完了 + 理由）
2. **作成・変更ファイル一覧**（ワークスペース相対パス）
3. **TDD の証跡**（各 Red の失敗コマンドと要約行 `test result: FAILED. …`、続く Green の要約行）
4. **テスト数とカバレッジ**（`cargo test -p canon-json` の合計、PBT 本数、受入表の行数、`cargo llvm-cov -p canon-json --summary-only` の値 — 未導入なら未計測と明記）
5. **棚卸し I1〜I6 の実測結果**（数値・箇所・根拠コマンド）
6. **ゴールデン採取の来歴**（取得 URL、抽出スニペットの sha256、captured_at、bun version、欠落ケースがあれば理由）
7. **計画からの逸脱・設計判断**（あれば。なければ「なし」）
8. **品質ゲートの結果**（fmt / clippy / cargo lint / cargo test --workspace の最終実行結果）
9. **コミット一覧**（`git log --oneline main-sync..HEAD`）
10. **未解決・要確認事項**
