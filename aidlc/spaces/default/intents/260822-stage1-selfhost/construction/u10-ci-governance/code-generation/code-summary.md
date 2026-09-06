# code-summary — U10 CI・品質管理（`u10-ci-governance`）

> Unit: `u10-ci-governance`（kind: packaging）。2026-09-06 の再確認による全面改訂。
> 対象リビジョン `e8ca4a5fb362284d5aa409e5342eda243ada4220`（ブランチ `stage1-selfhost`）。時刻はすべて UTC。
> 出典: `../nfr-requirements/security-requirements.md`・`tech-stack-decisions.md`（2026-09-06 改訂、READY）、
> `../nfr-design/security-design.md`（2026-09-06 改訂、READY）、`../revision-baseline-20260906.md`、
> `../ruleset-observed-20260906.json`、`../../../inception/requirements-analysis/requirements.md`（FR9.1〜9.5、NFR2、NFR4）、
> 承認済み `code-generation-plan.md`（承認指紋 `sha256:73fb6047d771f21ad6fa75a7cb9179c25d20dd34e637e9e3e0a03a60a4defe45`）と
> `unit-test-instructions.md`。
> 2026-08-23 時点の旧版は `code-summary-history-2026-08-23.md` に全文保存してある。過去の裁定は
> `superseding-decisions.md` と同履歴ファイルを参照する。

## 1. 今回の作業範囲

今回はワークスペースの CI・品質設定を **変更していない**。行ったのは次の 3 点である。

1. 現行設定を 2026-09-06 改訂の要件・設計へ照合した（§3〜§6、§9）。
2. Unit 限定コマンドと受入を実測して記録した（§7、§8）。
3. 実装記録（本ファイル・`traceability.json`）を現行の事実で書き直し、`source-manifest.json` を作成した。

CI 定義・スクリプト・品質閾値・ruleset・依存・ツールチェーン・プロダクトコードは変更していない。GitHub への書込（ruleset 変更、
PR 作成、コメント投稿、push）は行っていない。`scripts/governance/ruleset-required-checks.sh` は `--dry-run` を含め実行していない。
新規プロダクションコードと新規テストはなく、人工的な Red も作っていない。FR9.6（エラー様式規則の正本化）は U9 の責務であり扱わない。

CI・品質管理の実装自体は Bolt B2（PR #25・#26）で `main` へ反映済みである。本ファイルはその実装の**現況**を記述するものであり、
今回の作業でファイルを作成したという意味ではない。

## 2. 実装済みファイル

| ファイル | 責務 | 対応要求 |
|---|---|---|
| `.github/workflows/ci.yml` | 7 ジョブの CI 定義とイベント別の `CI Success` 集約 | FR9.2・FR9.3、NFR2.2・NFR2.3、NFR4.1・NFR4.2・NFR4.4 |
| `.github/workflows/review-thread-resolution.yml` | レビュースレッド状態の再評価（別ワークフロー） | NFR2.2、NFR4.4 |
| `scripts/coverage.sh` | カバレッジの絶対ゲートと条件付き相対ゲート | FR9.4・FR9.5、NFR2.4・NFR2.5 |
| `scripts/governance/verify-ci-governance.sh` | 設定の事実の機械検査（packaging Unit の「単体テスト」） | 上記全般の検証 |
| `scripts/governance/ruleset-required-checks.sh` | ruleset「main」の必須チェックを冪等に設定する手順 | FR9.1、NFR2.1、NFR4.5 |
| `scripts/governance/toolchain-inputs.sh` | `rust-toolchain.toml` から channel / components を導出 | FR9.2、NFR4.2 |
| `rust-toolchain.toml` | ツールチェーンの正本（channel・components・profile） | FR9.2、NFR4.2 |
| `Cargo.toml` | `[workspace.lints.rust]` の `unsafe_code = "forbid"` ほか | NFR4.3 |
| `tools/lint/Cargo.toml` | detached クレートの `[lints.rust]` 個別宣言 | NFR4.3 |
| 各 workspace メンバーの `Cargo.toml`（10 件） | `[lints] workspace = true` による継承 | NFR4.3 |

## 3. CI の構成と結果の受理

`ci.yml` は `pull_request`（`branches: [main]`）・`merge_group`・`workflow_dispatch` で起動する。`concurrency` は
`ci-${{ github.workflow }}-${{ github.ref }}` で分離し、`cancel-in-progress: true`。取消の結果は合格として受理しない。

| ジョブ（表示名） | 責務 | 必須結果との関係 |
|---|---|---|
| `aidlc-distribution` | Bun 1.3.13 で配布同期の検査と回帰試験 | `CI Success` 経由で必須 |
| `check` | workspace の fmt / clippy / `cargo lint` / test と、`tools/lint` の manifest-path 指定 fmt / clippy / test | ruleset で直接必須、`CI Success` でも success 必須 |
| `quint` | Node 22 / Quint 0.32.0 で `scripts/quint-gate.sh` | ruleset で直接必須、`CI Success` でも success 必須 |
| `coverage` | `scripts/coverage.sh` の絶対・条件付き相対ゲート | ruleset で直接必須、`CI Success` でも success 必須 |
| `review-thread-resolution`（`CI Review Thread Gate`） | SHA 固定の外部再利用ワークフローで未解決スレッドを検査 | `pull_request` では `CI Success` が success 必須 |
| `ci-success`（`CI Success`） | 上記結果をイベント別条件で集約 | ruleset で直接必須 |
| `audit` | workspace と `tools/lint` の 2 つの `Cargo.lock` を `cargo audit` へ渡す | 直接必須にも `CI Success` にも **含めない**（advisory） |

`ci-success` は `if: ${{ always() }}` で起動し、`needs` に `aidlc-distribution` / `check` / `quint` / `coverage` /
`review-thread-resolution` の 5 件を取る。判定は次のとおりで、skipped や cancelled を success へ読み替えない。

| イベント | `aidlc-distribution` / `check` / `quint` / `coverage` | レビュー検査 | coverage の比較 |
|---|---|---|---|
| `pull_request` | 全件 success 必須 | success 必須 | 絶対 90% と `origin/<base_ref>` に対する相対差 |
| `merge_group` | 全件 success 必須 | skipped を必須（他の結果は拒否） | 絶対 90% のみ |
| `workflow_dispatch` | 全件 success 必須 | skipped を必須（他の結果は拒否） | 絶対 90% のみ |

`review-thread-resolution.yml` は `pull_request_review`（submitted / edited / dismissed）、`pull_request_review_comment`
（created / edited / deleted）、`issue_comment`（created / edited / deleted）、`schedule`（`*/15 * * * *`）、`workflow_dispatch`
（`pr_number`・`wait_for_other_checks` の入力あり）を契機に、同じ外部ワークフローで `Check unresolved comments` の状態を
再評価する。この再評価と、`ci.yml` の実行時に確定した `CI Success` は別の出力であり、再評価だけで完了済みの `CI Success` が
自動更新されるとは、ローカル定義だけからは保証しない。

## 4. 権限・秘密情報・外部コード

`ci.yml` の workflow 既定は `permissions: contents: read`。追加権限は `review-thread-resolution` ジョブに限定する。

| 対象 | 宣言する権限 |
|---|---|
| `ci.yml` の通常ジョブ | `contents: read`（workflow 既定） |
| `ci.yml` の `review-thread-resolution` | `contents: read`、`checks: write`、`issues: read`、`pull-requests: read`、`statuses: write` の 5 種 |
| `review-thread-resolution.yml`（workflow 既定と `refresh` ジョブの両方） | 同じ 5 種 |

両レビュー呼出の参照先は `j5ik2o/ci/.github/workflows/review-thread-resolution.yml` で、参照版と入力 `ci_ref` はどちらも同一の
SHA `9cf0e9a8cd74c72de704763025003ed3b7608c65` である（`ci.yml` の 2 箇所と `review-thread-resolution.yml` の 2 箇所、計 4 箇所を
読取で照合済み）。SHA 固定は取得版の固定であり、外部コード自体の安全性を保証しない。

`aidlc-distribution` ジョブの `actions/checkout` は `11d5960a326750d5838078e36cf38b85af677262`、`oven-sh/setup-bun` は
`0c5077e51419868618aeaa5fe8019c62421857d6` に固定され、同ジョブの checkout は `persist-credentials: false`。他ジョブには
`actions/checkout@v4`・`actions/setup-node@v4`・`Swatinem/rust-cache@v2`・`taiki-e/install-action@v2`・
`dtolnay/rust-toolchain@master` のタグ／ブランチ参照が残る。**全 Action の SHA 固定は本 intent では採用していない**ため、
「全 Action が固定済み」とも「全 Action が未固定」とも記述しない。Dependabot の導入は既存裁定により見送っている。

## 5. ruleset の現況

`../ruleset-observed-20260906.json`（2026-09-06 取得の観測記録）と、今回の `verify-ci-governance.sh --with-ruleset` による
再取得の双方で次を確認した。観測記録は同日時点の設定の記録であり、将来の状態や実働を保証するものではない。

- ruleset「main」（id `21190453`）、`enforcement: active`、`conditions: null`、`bypass_actors: []`（bypass なし）。
- 必須チェックは `check` / `quint` / `coverage` / `CI Success` の 4 コンテキスト、`strict_required_status_checks_policy: true`。
- 規則は `deletion`・`non_fast_forward`・`merge_queue`・`required_status_checks` の 4 種。
- マージキューは `merge_method: SQUASH`、`grouping_strategy: ALLGREEN`、`max_entries_to_merge: 1`、`max_entries_to_build: 1`、
  `min_entries_to_merge: 1`、`min_entries_to_merge_wait_minutes: 0`、`check_response_timeout_minutes: 60`。
- `audit` は必須チェックに含まれない。

`ruleset-required-checks.sh` の期待値（`REQUIRED_CONTEXTS="check,quint,coverage,CI Success"`、`STRICT_POLICY="true"`）は
この観測と一致する。冪等判定は必須コンテキストの**集合**と strict フラグの一致で行い、一致すれば PUT しない。今回は
要求に合う設定が観測されているため、GitHub への書込は行っていない。

## 6. 品質設定と再現性

| 項目 | 値・方式 | 正本 |
|---|---|---|
| 絶対床 | 90.0%（`ABSOLUTE_THRESHOLD=90.0`） | `scripts/coverage.sh` |
| 相対許容差 | 0.01 ポイント（`TOLERANCE=0.01`、判定は `head >= base - 0.01`） | `scripts/coverage.sh` |
| PBT シード | `PROPTEST_RNG_SEED=20260823`（`coverage.sh` が export、`ci.yml` の `check` / `coverage` の 2 ジョブでも `env` に宣言） | `scripts/coverage.sh` と `ci.yml` |
| 明示除外 | `modules/app/aidlc/src/main.rs` の 1 ファイルのみ（クレート単位の `--exclude` は使わない） | `scripts/coverage.sh` |
| ツールチェーン | channel `1.95.0`、components `rustfmt` / `clippy` / `llvm-tools`、profile `minimal` | `rust-toolchain.toml` |

除外式は Markdown 表の区切りと混同されないよう表の外に記す。

`(^|/)modules/app/aidlc/src/main\.rs$`

`toolchain-inputs.sh` はこの `rust-toolchain.toml` から `channel=` と `components=`（カンマ区切り）を導出し、`check` /
`coverage` / `audit` の 3 ジョブが `>> "$GITHUB_OUTPUT"` 経由で `dtolnay/rust-toolchain@master` の入力へ渡す。ワークフローに
版を書き写していないため、正本は 1 箇所である。

`unsafe_code = "forbid"` は `[workspace.lints.rust]` で定義し、workspace メンバー 10 件すべてが `[lints] workspace = true` で
継承する。detached の `tools/lint` は継承しないため `[lints.rust]` に同じ禁止を個別宣言している。§8 の実測で、この 2 経路が
それぞれ単独で不適合例を拒否することを分離確認した。`tools/lint` は workspace 外であり、90% 床の対象には含めない。

## 7. Unit 限定コマンドの実測（2026-09-06）

すべてワークスペースルートで実行した。ログは
`/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-stage1-selfhost/bdae4b2f-d1d9-470f-bf7c-df8853392e07/scratchpad/`
配下に保存した（セッション固有の一時領域であり、リポジトリには含めない）。

| コマンド | 件数・出力 | 終了コード | 完了時刻（UTC） | ログ |
|---|---|---|---|---|
| `bash -n scripts/coverage.sh` | 構文エラーなし | 0 | 2026-09-06T14:37:06Z | `step1-bash-n.log` |
| `bash -n scripts/governance/verify-ci-governance.sh` | 構文エラーなし | 0 | 2026-09-06T14:37:06Z | `step1-bash-n.log` |
| `bash -n scripts/governance/ruleset-required-checks.sh` | 構文エラーなし | 0 | 2026-09-06T14:37:06Z | `step1-bash-n.log` |
| `bash -n scripts/governance/toolchain-inputs.sh` | 構文エラーなし | 0 | 2026-09-06T14:37:06Z | `step1-bash-n.log` |
| `bash scripts/governance/verify-ci-governance.sh` | PASS 19 / FAIL 0 | 0 | 2026-09-06T14:37:10Z | `step1-verify-default.log` |
| `bash scripts/governance/verify-ci-governance.sh --with-ruleset` | PASS 20 / FAIL 0 | 0 | 2026-09-06T14:37:18Z | `step1-verify-with-ruleset.log` |
| `bash scripts/governance/toolchain-inputs.sh` | `channel=1.95.0` / `components=rustfmt,clippy,llvm-tools` | 0 | 2026-09-06T14:37:24Z | `step1-toolchain.log` |
| `cargo test --manifest-path tools/lint/Cargo.toml` | 93 passed / 0 failed / 0 ignored | 0 | 2026-09-06T14:37:32Z | `step1-tools-lint-test.log` |

`bash -n` は最初のファイルしか解析しないため、4 本を個別に実行した。`tools/lint` の自己テスト件数は実測の 93 本であり、
2026-08-22 時点の 31 本ではない（件数は実行時の結果を記録する方針）。対象が 0 件に減っていないことも確認した。

検査 20 項目の内訳（既定 19 + `--with-ruleset` の 1）は次のとおりで、すべて PASS した。期待値はスクリプト内の定数であり、
書き換えていない。

| # | 検査名 | 対象ファイル |
|---|---|---|
| 1 | `toolchain-channel` | `rust-toolchain.toml` |
| 2 | `toolchain-components` | `rust-toolchain.toml` |
| 3 | `toolchain-profile` | `rust-toolchain.toml` |
| 4 | `workspace-unsafe-forbid` | `Cargo.toml` |
| 5 | `workspace-members-lints-inherit` | 各メンバーの `Cargo.toml`（10 件） |
| 6 | `tools-lint-unsafe-forbid` | `tools/lint/Cargo.toml` |
| 7 | `ci-merge-group-trigger` | `.github/workflows/ci.yml` |
| 8 | `ci-permissions-contents-read` | `.github/workflows/ci.yml` |
| 9 | `ci-toolchain-file-driven` | `.github/workflows/ci.yml` |
| 10 | `ci-review-thread-gate` | `.github/workflows/ci.yml` |
| 11 | `ci-success-aggregate` | `.github/workflows/ci.yml` |
| 12 | `ci-review-thread-refresh-workflow` | `.github/workflows/review-thread-resolution.yml` |
| 13 | `ci-tools-lint-steps` | `.github/workflows/ci.yml` |
| 14 | `ci-audit-job` | `.github/workflows/ci.yml` |
| 15 | `ci-proptest-seed-env` | `.github/workflows/ci.yml` |
| 16 | `ci-coverage-base-condition` | `.github/workflows/ci.yml` |
| 17 | `coverage-tolerance` | `scripts/coverage.sh` |
| 18 | `coverage-ignore-regex` | `scripts/coverage.sh` |
| 19 | `coverage-proptest-seed` | `scripts/coverage.sh` |
| 20 | `ruleset-required-checks`（`--with-ruleset` のみ） | GitHub ruleset「main」（`gh api` 読取） |

実行環境の版: `rustc 1.95.0 (59807616e 2026-04-14)`、`cargo 1.95.0 (f2d3ce0bd 2026-03-21)`、`cargo-llvm-cov 0.8.5`、
`cargo-audit-audit 0.22.2`、`jq-1.8.2`、`gh 2.98.0`。`rustc` の版は `rust-toolchain.toml` の `channel = "1.95.0"` と一致する。

## 8. 受入の実測（2026-09-06）

### (a) カバレッジ 2 回測定

同一リビジョン `e8ca4a5fb362284d5aa409e5342eda243ada4220`、同一ツールチェーン（`rustc 1.95.0`）、同一シード
（`PROPTEST_RNG_SEED=20260823`）で `bash scripts/coverage.sh`（引数なし = 絶対ゲートのみ）を 2 回実行した。

| 回 | 生の head 値 | 絶対ゲート（90.0%） | 終了コード | 開始〜終了（UTC） |
|---|---|---|---|---|
| 1 回目 | 99.14022164135578% | PASS | 0 | 14:37:56Z〜14:38:51Z |
| 2 回目 | 99.14022164135578% | PASS | 0 | 14:38:51Z〜14:39:42Z |

**差 0.00 ポイント**（全有効桁まで一致）。受入目標を満たした。ログは `step3a-coverage.log`。`TOLERANCE`・除外式・シードは
変更していない。この値は絶対ゲートの実測であり、相対ゲート（`--base` 指定時の base 側計測と比較）は base ref を持たない
ローカル実行では走らせていない。

### (b) 依存監査

| コマンド | 走査対象 | advisory DB | 結果 | 終了コード |
|---|---|---|---|---|
| `cargo audit` | `Cargo.lock`（125 crate dependencies） | 取得成功、1239 advisories 読込 | 脆弱性の検出なし | 0 |
| `cargo audit --file tools/lint/Cargo.lock` | `tools/lint/Cargo.lock`（5 crate dependencies） | 取得成功、1239 advisories 読込 | 脆弱性の検出なし | 0 |

いずれも `https://github.com/RustSec/advisory-db.git` の取得と crates.io index の更新に成功している。ログは
`step3b-audit.log`。実行時刻 2026-09-06T14:39:13Z〜14:39:16Z。

### (c) `unsafe` 不適合例の拒否

各クレートへ `#[allow(dead_code)] unsafe fn __aidlc_forbid_probe() {}` を一時追加して `cargo check` を実行し、直後に
`git checkout -- <file>` で戻した。4 件とも ``error: declaration of an `unsafe` function`` でコンパイルを拒否した。

| # | 対象 | コマンド | 拒否の根拠として報告された行 | ログ |
|---|---|---|---|---|
| A | `core-command-domain`（`modules/core/command/domain/src/lib.rs`） | `cargo check -p core-command-domain` | クレート属性 `#![forbid(unsafe_code)]`（lib.rs:27） | `step3c-unsafe-probe-workspace.log` |
| B | `core-query-use-case`（クレート属性を持たないメンバー） | `cargo check -p core-query-use-case` | `requested on the command line with -F unsafe-code`（= `[workspace.lints.rust]` + `lints.workspace = true` の継承経路） | `step3c-unsafe-probe-workspace-lints.log` |
| C | `tools/lint`（`src/main.rs`） | `cargo check --manifest-path tools/lint/Cargo.toml` | クレート属性 `#![forbid(unsafe_code)]`（main.rs:23） | `step3c-unsafe-probe-toolslint.log` |
| D | `tools/lint`（クレート属性を一時コメントアウトして分離） | 同上 | `requested on the command line with -F unsafe-code`（= `tools/lint/Cargo.toml` の `[lints.rust]` 単独） | `step3c-unsafe-probe-toolslint-manifest.log` |

A と C は crate 個別の属性が先に報告されるため、それだけでは manifest 側の強制力を分離できない。そこで B（属性を持たない
メンバー）と D（属性を一時無効化した `tools/lint`）を追加し、**manifest の lints 宣言だけでも拒否が成立する**ことを
確認した。これにより NFR4.3 の「workspace lints の継承」と「detached クレートの個別宣言」の両経路が実働で裏付けられた。
すべての一時変更は確認直後に戻し、`git status --short` でワークスペース側の差分が 0 件であることを毎回確認した。

## 9. 要件・設計との照合結果

| 要求 | 照合対象 | 結果 |
|---|---|---|
| FR9.1 / NFR2.1 / NFR4.5 | ruleset の 4 コンテキスト・strict・bypass なし・キュー設定、`ruleset-required-checks.sh` の期待値と冪等判定 | 一致（§5）。実働（失敗時の停止・全成功時のキュー完走）は今回未実施 |
| FR9.2 / NFR4.1 | `audit` ジョブの 2 ロックファイル指定、`cargo audit` 2 件の実測 | 一致（§3、§8(b)） |
| FR9.2 / NFR4.2 | `rust-toolchain.toml` の 3 値、`toolchain-inputs.sh` の導出、ローカル `rustc -V` | 一致（§6、§7） |
| FR9.2 / NFR4.3 | `[workspace.lints.rust]`、メンバー 10 件の継承、`tools/lint` の個別宣言、不適合例の拒否 | 一致（§6、§8(c)） |
| FR9.2 / NFR4.4 | workflow 既定 `contents: read`、レビュー検査の個別権限 5 種、外部呼出先と `ci_ref` の SHA 一致 | 一致（§4） |
| FR9.3 / NFR2.3 | `check` ジョブの workspace 4 ステップと `tools/lint` 3 ステップ、自己テストの実測件数 | 一致（§3、§7） |
| FR9.4 / NFR2.4 | `TOLERANCE=0.01`、シード 20260823 の CI とローカルの宣言、2 回測定の差 | 一致。差 0.00 ポイントを達成（§6、§8(a)） |
| FR9.5 / NFR2.5 | 除外式が `main.rs` 1 ファイルのみ、絶対ゲート 90% の結果 | 一致（§6、§8(a)） |
| NFR2.2 | 7 ジョブ、イベント別の `CI Success` 集約条件、`coverage` の比較条件、`audit` の集約外 | 一致（§3） |

**不一致は見つからなかった。** 検査 20 項目もすべて PASS しており、設定側に修正すべき差分はない。

補足として、記録上の数値の推移を 1 点挙げる。`team.md` の Testing Posture が引用する実測値は 94.87〜95.29% であるが、
今回の実測は 99.14022164135578% である。これは基準日以降にコードとテストが増えたことによる推移であり、設定・閾値の
不一致ではない（絶対床 90% を満たす点は変わらない）。閾値の変更は行っていない。

## 10. 未検証範囲

Unit 限定コマンドと §8 の実測の成功は、設定の存在と当該コマンドの成功を示すものであり、次を検証したことにはならない。

- **CI の全ジョブ実行**: 今回は GitHub 上の実行を起こしていない。`aidlc-distribution` / `quint` / `check` の CI 上の
  成否、CI 側の `rustc` 版一致はローカル実測では代替できない。
- **マージキューの成功・失敗両経路の実働**: 必須検査失敗時にマージが止まる経路と、全成功時にキューを完走して
  squash-merge される経路のいずれも今回は実行していない。
- **レビュー結果の再評価の反映**: `review-thread-resolution.yml` の再評価が、完了済みの `CI Success` と最新のマージ条件へ
  どのように反映されるかは未確認。
- **外部再利用ワークフローの内部**: `j5ik2o/ci` 側の実装は SHA 固定で参照しているだけであり、内部の挙動は検証していない。
- **相対ゲートの実働**: `coverage.sh --base` の base 側計測と比較は今回実行していない（絶対ゲートのみ 2 回）。
- **`ruleset-required-checks.sh` の書込経路**: `--dry-run` を含め実行しておらず、PUT 経路と前後 JSON の生成は未実測。

## 11. 過去の裁定（履歴の参照先）

次はいずれも**過去の事実**であり、今回の実施ではない。詳細は履歴ファイルを参照する。

| 事項 | 現行との関係 | 参照先 |
|---|---|---|
| 暫定 `TOLERANCE=0.05`、残差 0.0175 ポイント（`fs_workspace_lock` の並行テスト由来） | U3 のロック退役（ADR-007）後に 0.01 へ引き締め済み。現行は 0.01 で、今回の 2 回測定は差 0.00 | `superseding-decisions.md` #1 |
| 除外 regex の訂正（`^modules/...` 単独アンカーが llvm-cov の絶対パスに不一致だった） | 現行は `(^\|/)` アンカー。訂正後の式で除外が効いている | `superseding-decisions.md` #2 |
| ruleset への必須チェック適用（2026-08-22T23:43Z）と PR #25 の merge queue 完走 | 適用済み。その後 `CI Success` 追加で 4 コンテキストへ拡張 | `superseding-decisions.md` #3・#9、`ruleset/before.json`・`ruleset/after.json`、`ruleset/2026-08-23-ci-success/` |
| レビュースレッドゲートの追加（PR #26） | 現行の `review-thread-resolution` ジョブと集約条件 | `superseding-decisions.md` #9・#10 |
| 2026-08-22 の TDD 証跡（Red 1/14 → Green 15/0）、自己テスト 31 本 | 今回の実測は 20 項目 PASS・自己テスト 93 本。過去の件数へ固定しない | `code-summary-history-2026-08-23.md` §3 |

## 12. 引き継ぎ

- 本ファイル・`traceability.json`・`source-manifest.json` を今回更新した。ワークスペース側のファイルは 1 件も変更していない
  （`git status --short` で確認済み）。
- `traceability.json` の 15 件はすべて実在ファイルのワークスペース相対パス単体を `target` とし、注記は本ファイルへ移した。
  status はすべて `OK`（実測で未達が残る要求はない）。
- 未検証範囲（§10）は、実働検証を伴う後続の受入項目として残る。とくにマージキューの両経路とレビュー再評価の反映は、
  設定の存在確認では代替できない。
- `superseding-decisions.md`・`pending-revision.md`・`developer-brief-3.md`・`developer-report-3.md`・`ruleset/` 配下・
  各履歴ファイルは今回変更していない。
