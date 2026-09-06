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

- [x] Step 1. ランナーと設定を確認する。`unit-test-instructions.md` のUnit限定コマンド（`bash -n` の個別実行、`verify-ci-governance.sh` の
      既定と `--with-ruleset`、`toolchain-inputs.sh`、`tools/lint` の自己テスト）を実行し、件数・結果・完了日時を記録する。
      `rustc -V` と `cargo llvm-cov --version`、`cargo audit --version` の有無も記録する。
- [x] Step 2. 現行設定を要件FR9.1〜9.5・NFR2.1〜2.5・NFR4.1〜4.5と設計§2〜§5へ対応付ける。7ジョブとイベント別の集約条件、
      review-thread-resolutionのジョブ別権限とSHA固定（呼出先・ci_refの一致）、必須4コンテキスト・strict・bypassなし・キュー設定
      （`ruleset-observed-20260906.json`）、閾値・シード・除外式、toolchainの導出、`unsafe_code = "forbid"` の継承（全workspaceメンバーの
      `lints.workspace = true` と `tools/lint` の個別宣言）を確認する。
- [x] Step 3. 受入を実測する。(a) `bash scripts/coverage.sh` を同一リビジョン・同一ツールチェーン・同一シードで2回実行し、生のhead値と
      差を記録する（差0.00ポイント未達なら未達のまま原因を記録し、閾値を変えない）。(b) `cargo audit` と
      `cargo audit --file tools/lint/Cargo.lock` を実行し、結果・走査crate数・advisory DBの取得可否を記録する（未導入なら未実行として記録し
      成功と書かない）。(c) workspaceメンバー1クレートと `tools/lint` に `unsafe` を含む一時変更を加えて `cargo check` が拒否することを
      確認し、直後に変更を戻して `git status` で差分がないことを確認する。
- [x] Step 4. `code-summary.md` を現行の事実で書き直す。実装済みファイル一覧（review-thread-resolution.ymlを含む）、7ジョブとCI Successの
      集約条件、4コンテキスト、ジョブ別権限、SHA固定、閾値0.01、検査20項目、今回の実測、未検証範囲（全CI実行・キューの成功/失敗両経路の
      実働・外部再利用ワークフロー内部）を区別して記す。過去の裁定（暫定0.05、除外regexの訂正、ruleset適用、PR #25/#26）は履歴として
      `superseding-decisions.md` と履歴ファイルを参照する。
- [x] Step 5. `traceability.json` を更新する。15件のIDを現行の実在ファイルへ対応付け、targetはワークスペース相対パス単体にし、日時・参照JSON
      等の注記はcode-summary側へ移す。`bun .claude/tools/aidlc-sensor-traceability.ts` で `invalid_targets` が0であることを確認する
      （`missing_from_upstream_ids` は他Unitの要求IDで、既知のノイズ）。`source-manifest.json` を strict schema で作る（`writes` は空配列）。
- [x] Step 6. `git status` でワークスペース側に差分がないこと、記録側の変更が本ディレクトリに限られることを確認し、独立レビューへ引き渡す。
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

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T14:56:14Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor（提案） | `traceability.json` の `NFR4.3` 行 | `target` は `Cargo.toml` 単体である。NFR4.3 の受入基準（`../nfr-requirements/security-requirements.md` 行35）は「全workspaceメンバーのlints継承とtools/lintの個別宣言」の両方を要求しており、後者の正本は `tools/lint/Cargo.toml` である。`code-summary.md` §6・§8(c) では両ファイルと両経路の分離実測が記述されているため実害はないが、`traceability.json` 単体だけを読む場合は `tools/lint/Cargo.toml` 側の宣言箇所へたどれない。 | 現行のスキーマ制約（1 ID = 1 target）を踏まえ、必須ではないが、次回以降に schema が複数 target を許容するようになった場合は `tools/lint/Cargo.toml` も追加することを検討する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（`code-generation-plan.md`） | `pass:true`、H2 7件 | 文書形状は充足 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（`unit-test-instructions.md`） | `pass:true`、H2 5件 | 文書形状は充足 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（`code-summary.md`） | `pass:true`、H2 12件 | 文書形状は充足 |
| `bun .claude/tools/aidlc-sensor-traceability.ts`（`traceability.json`） | `pass:false`（`invalid_targets`・`gaps`・`orphans`・`invalid_entries` はすべて0件。`missing_from_upstream_ids` 38件のみ） | `missing_from_upstream_ids` は依頼書が明記するとおり他Unitの要求ID（FR1〜FR8、NFR1・NFR3・NFR5等）と、本Unitの担当外である `FR9.6`（U9の責務）・上位ID（`FR9`・`NFR2`・`NFR4` 自体）であり、既知のノイズ。実質的な機械検証観点（target の実在性・重複・抜け漏れ）はすべて健全 |
| `linter` / `type-check` センサー | 対象外 | 本ステージの生成物はMarkdown/JSONのみで、TS/JS生成コードが無いため非適用（依頼書の指示どおり） |
| `bash scripts/governance/verify-ci-governance.sh`（引数なし、再実行） | `PASS 19 / FAIL 0` | `code-summary.md` §7の実測記載と一致。ライブ再検証で追認 |
| `bash scripts/governance/verify-ci-governance.sh --with-ruleset`（再実行、`gh api` 読取のみ） | `PASS 20 / FAIL 0`。ruleset「main」(id=21190453) の必須チェックが `[CI Success\|check\|coverage\|quint]` + strict | `code-summary.md` §5・§7の記載と完全一致。GitHubへの書込は発生していない |
| `bash scripts/governance/toolchain-inputs.sh`（再実行） | `channel=1.95.0` / `components=rustfmt,clippy,llvm-tools` | `rust-toolchain.toml`・`code-summary.md` §6・§7と一致 |
| `rustc -V`（再実行） | `rustc 1.95.0 (59807616e 2026-04-14)` | `code-summary.md` §7の実行環境記載と一致 |
| `cargo test --manifest-path tools/lint/Cargo.toml`（再実行） | `93 passed; 0 failed; 0 ignored` | `code-summary.md` §7・§11の「実測93本、旧31本に固定しない」という記載と完全一致 |
| `cargo audit` / `cargo audit --file tools/lint/Cargo.lock`（再実行） | workspace: 125 crate dependencies、`tools/lint`: 5 crate dependencies、いずれも advisory DB 1239件読込 | `code-summary.md` §8(b)の件数と完全一致 |
| `git status --short` によるワークスペース側差分の確認 | ワークスペースのアプリケーション側パスに変更なし（`aidlc/` 配下の記録ファイルのみ） | `source-manifest.json` の `writes: []` および計画・要約の「ワークスペースを変更していない」という記述と一致 |
| ファイル存在確認 | `traceability.json` の6つの重複除去済み `target`（`scripts/governance/ruleset-required-checks.sh`、`rust-toolchain.toml`、`.github/workflows/ci.yml`、`scripts/coverage.sh`、`scripts/governance/toolchain-inputs.sh`、`Cargo.toml`）はすべて実在する | 「実在ファイル単体」の契約を満たす |

### Summary

`code-summary.md`・`traceability.json`・`unit-test-instructions.md`・`code-generation-plan.md` の記述を実ファイル（`.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/*.sh`）と照合し、さらに検証ツールと推奨コマンド（`verify-ci-governance.sh` 通常/`--with-ruleset`、`toolchain-inputs.sh`、`rustc -V`、`tools/lint` 自己テスト、`cargo audit` 2件）を実際に再実行してすべて記載どおりの結果を得た。2026-08-23の回復レビュー（NOT-READY: Critical 1・Major 2 — review-threadゲート未記載、traceabilityのtargetに注記混在、検査数値が古い）は、今回の`code-summary.md`が review-thread ゲート・4コンテキスト集約・ジョブ別権限を明記し、`traceability.json`の全15件のtargetがパス単体になり、検査数値が実測（19/20、自己テスト93本）に更新されていることで解消されている。過去の事実（暫定許容差0.05、旧31本、PR #25/#26の出来事）は履歴セクションに区別して記載されており、今回の実施と混同していない。上流要件のR-01（Markdown表の表示崩れ、Minor）は本ステージの成果物では表の外に式を出すことで同種の不具合を回避しており、上流の未解決所見として引き続き残るのみである。Critical・Majorに該当する所見はなく、Minorの提案が1件（traceability.jsonのスキーマ制約に起因する副次的な参照可能性の限界）のみで、いずれも承認判断を妨げない。
