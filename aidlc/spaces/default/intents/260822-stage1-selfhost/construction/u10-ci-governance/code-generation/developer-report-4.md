# developer-report-4 — U10 CI・品質管理の実装記録の是正（2026-09-06）

> 委任 4（Code Generation、Unit: `u10-ci-governance`、kind: packaging）の最終報告。
> 承認済み計画（承認指紋 `sha256:73fb6047d771f21ad6fa75a7cb9179c25d20dd34e637e9e3e0a03a60a4defe45`）の Step 1〜6 を実行した。
> Testing Contract 指紋 `sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`（ディスパッチ標識と一致を確認）。
> 対象リビジョン `e8ca4a5fb362284d5aa409e5342eda243ada4220`。時刻はすべて UTC。

## 1. 実行した Step と結果

| Step | 内容 | 結果 |
|---|---|---|
| Step 1 | ランナーと設定の確認（Unit 限定コマンド 8 本、ツール版の記録） | 完了。全コマンド終了コード 0。検査 19 項目（既定）／20 項目（`--with-ruleset`）すべて PASS |
| Step 2 | 現行設定を FR9.1〜9.5・NFR2.1〜2.5・NFR4.1〜4.5 と設計 §2〜§5 へ照合 | 完了。**不一致なし**（§4） |
| Step 3 | 受入の実測（カバレッジ 2 回・依存監査 2 件・unsafe 拒否 4 件） | 完了。カバレッジ差 **0.00 ポイント**（受入目標達成）、audit 2 件とも脆弱性なし、unsafe 拒否 4 件すべて確認 |
| Step 4 | `code-summary.md` を現行の事実で全面書き直し | 完了。H2 12 件、`## Review` 節なし。`required-sections` センサー `pass: true` |
| Step 5 | `traceability.json` 15 件の更新と `source-manifest.json` の作成 | 完了。`traceability` センサーの `invalid_targets` が 0 |
| Step 6 | ワークスペース差分ゼロの確認と計画チェックボックスの `[x]` 化 | 完了。ワークスペース側の差分 0 件。計画の変更はチェックボックスのみ（ハッシュで証明） |

作業順序は委任ブリーフ §A.2 に従い、Step 1〜3（ワークスペースに触れる作業）→ Step 4〜5（記録の更新）→ Step 6
（チェックボックス）の順で実施した。

## 2. Unit 限定コマンドの結果表

ログの保存先はいずれも
`/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-stage1-selfhost/bdae4b2f-d1d9-470f-bf7c-df8853392e07/scratchpad/`
配下（セッション固有の一時領域。リポジトリには含めない）。

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
| `bash scripts/governance/verify-ci-governance.sh --with-ruleset`（Step 6 の再実行） | PASS 20 / FAIL 0 | 0 | 2026-09-06T14:44:39Z | `step6-final-verify.log` |

`bash -n` は最初のファイルしか解析しないため 4 本を個別に実行した。`tools/lint` の自己テスト件数は実測 **93 本**であり、
2026-08-22 時点の 31 本ではない（件数を過去値に固定しない方針に従い実測値を記録した）。対象が 0 件に減っていないことも確認済み。
期待値（channel `1.95.0`、components `rustfmt clippy llvm-tools`、profile `minimal`、`TOLERANCE=0.01`、除外式、シード
`20260823`、必須コンテキスト 4 件）はスクリプト内の定数のまま、一切書き換えていない。

## 3. 受入の実測

### (a) カバレッジ 2 回測定

同一リビジョン `e8ca4a5fb362284d5aa409e5342eda243ada4220`、同一ツールチェーン（`rustc 1.95.0`）、同一シード
（`PROPTEST_RNG_SEED=20260823`）で `bash scripts/coverage.sh` を 2 回実行した。ログは `step3a-coverage.log`。

| 回 | 生の head 値 | 絶対ゲート（90.0%） | 終了コード | 実行時間帯（UTC） |
|---|---|---|---|---|
| 1 回目 | 99.14022164135578% | PASS | 0 | 14:37:56Z〜14:38:51Z |
| 2 回目 | 99.14022164135578% | PASS | 0 | 14:38:51Z〜14:39:42Z |

**差 0.00 ポイント**（全有効桁まで完全一致）。受入目標を満たした。`TOLERANCE`・除外式・シードは変更していない。
相対ゲート（`--base`）は base ref を持たないローカル実行では走らせていないため未実測である。

### (b) 依存監査

ログは `step3b-audit.log`（14:39:13Z〜14:39:16Z）。

| コマンド | 走査対象 | advisory DB | 結果 | 終了コード |
|---|---|---|---|---|
| `cargo audit` | `Cargo.lock`（125 crate dependencies） | 取得成功、1239 advisories 読込 | 脆弱性の検出なし | 0 |
| `cargo audit --file tools/lint/Cargo.lock` | `tools/lint/Cargo.lock`（5 crate dependencies） | 取得成功、1239 advisories 読込 | 脆弱性の検出なし | 0 |

いずれも `https://github.com/RustSec/advisory-db.git` の取得と crates.io index の更新に成功しており、未実行・取得失敗は
発生していない。`cargo-audit` は 0.22.2 が導入済みであった。

### (c) `unsafe` 不適合例の拒否

各対象へ `#[allow(dead_code)] unsafe fn __aidlc_forbid_probe() {}` を一時追加して `cargo check` を実行し、直後に
`git checkout -- <file>` で戻した。4 件とも `error: declaration of an 'unsafe' function` でコンパイルを拒否した。

| # | 対象 | コマンド | 拒否の根拠として報告された箇所 | ログ |
|---|---|---|---|---|
| A | `core-command-domain` | `cargo check -p core-command-domain` | クレート属性 `#![forbid(unsafe_code)]`（`lib.rs:27`） | `step3c-unsafe-probe-workspace.log` |
| B | `core-query-use-case`（クレート属性を持たないメンバー） | `cargo check -p core-query-use-case` | `requested on the command line with -F unsafe-code`（`[workspace.lints.rust]` + `lints.workspace = true` の継承経路） | `step3c-unsafe-probe-workspace-lints.log` |
| C | `tools/lint` | `cargo check --manifest-path tools/lint/Cargo.toml` | クレート属性 `#![forbid(unsafe_code)]`（`main.rs:23`） | `step3c-unsafe-probe-toolslint.log` |
| D | `tools/lint`（クレート属性を一時コメントアウト） | 同上 | `requested on the command line with -F unsafe-code`（`tools/lint/Cargo.toml` の `[lints.rust]` 単独） | `step3c-unsafe-probe-toolslint-manifest.log` |

A と C だけでは crate 個別属性が先に報告されるため manifest 側の強制力を分離できない。そこで計画 Step 3(c) の 2 件に
B・D を追加し、**manifest の lints 宣言だけでも拒否が成立する**ことを分離確認した。これにより NFR4.3 の 2 経路
（workspace lints の継承、detached クレートの個別宣言）が実働で裏付けられた。一時変更は毎回直後に戻し、
`git status --short` で差分 0 件を確認している。

### (d) ツールとツールチェーンの版

`rustc 1.95.0 (59807616e 2026-04-14)`、`cargo 1.95.0 (f2d3ce0bd 2026-03-21)`、`cargo-llvm-cov 0.8.5`、
`cargo-audit-audit 0.22.2`、`jq-1.8.2`、`gh 2.98.0`。`rustc` の版は `rust-toolchain.toml` の `channel = "1.95.0"` と一致する。

## 4. 要件・設計との照合で見つかった不一致

**なし。** 検査 20 項目すべて PASS し、独立に読取・照合した以下もすべて要件・設計の記述と一致した。

- `ci.yml` の 7 ジョブ（`aidlc-distribution` / `check` / `quint` / `coverage` / `review-thread-resolution` / `ci-success` /
  `audit`）と、イベント別の `CI Success` 集約条件（`pull_request` はレビュー検査 success 必須、`merge_group` /
  `workflow_dispatch` は skipped を必須。基本 4 検査の skipped・cancelled は拒否）。`audit` は `needs` に含まれない。
- `review-thread-resolution` のジョブ別権限 5 種（`contents: read` / `checks: write` / `issues: read` /
  `pull-requests: read` / `statuses: write`）と、workflow 既定 `permissions: contents: read`。
- 外部呼出先 `j5ik2o/ci/.github/workflows/review-thread-resolution.yml` と入力 `ci_ref` の SHA 一致
  （`9cf0e9a8cd74c72de704763025003ed3b7608c65`、`ci.yml` 2 箇所 + `review-thread-resolution.yml` 2 箇所の計 4 箇所）。
- ruleset「main」（id `21190453`）: 必須 4 コンテキスト（`check` / `quint` / `coverage` / `CI Success`）、`strict: true`、
  `bypass_actors: []`、`deletion` / `non_fast_forward` / `merge_queue` の維持、キューは SQUASH / ALLGREEN / 同時 1 件
  （`max_entries_to_merge: 1`、`max_entries_to_build: 1`、`check_response_timeout_minutes: 60`）。
  `../ruleset-observed-20260906.json` と `--with-ruleset` の再取得の双方で確認。
- 品質設定: 絶対床 90.0%、相対許容差 0.01、シード 20260823（`coverage.sh` の export と `ci.yml` の `check` / `coverage`
  2 ジョブの `env`）、除外式は `main.rs` 1 ファイルのみ。
- toolchain の導出: `rust-toolchain.toml`（channel / components / profile）→ `toolchain-inputs.sh` → 3 ジョブの
  `dtolnay/rust-toolchain@master` 入力。ワークフローに版の書き写しはない。
- `unsafe_code = "forbid"` の継承: workspace メンバー 10 件すべてが `[lints] workspace = true`、`tools/lint` は
  `[lints.rust]` で個別宣言。

**修正は一切行っていない**（不一致がなかったため、検出項目の追加提案もない）。

参考として、記録上の数値の推移を 1 点だけ挙げる。`team.md` の Testing Posture が引用する実測値は 94.87〜95.29% だが、
今回の実測は 99.14022164135578% である。これは基準日以降にコードとテストが増えたことによる推移であり、設定・閾値の
不一致ではない。閾値は変更していない。

## 5. 更新したファイルと、変更しなかったファイル

### 更新したファイル（すべて記録ディレクトリ内）

| ファイル | 変更内容 |
|---|---|
| `code-summary.md` | 現行の事実で全面書き直し（H2 12 件、`## Review` 節なし）。実測・照合結果・未検証範囲・過去の裁定の参照先を区別して記載 |
| `traceability.json` | 15 件すべてを実在ファイルのワークスペース相対パス単体へ対応付け、注記を `code-summary.md` へ移動。status はすべて `OK` |
| `source-manifest.json` | 新規作成。`{"stage":"code-generation","unit":"u10-ci-governance","version":1,"writes":[]}` |
| `code-generation-plan.md` | Step 1〜6 のチェックボックスのみ `[x]` 化。本文は 1 バイトも変更していない（下記のハッシュ照合で証明） |
| `developer-report-4.md` | 本ファイル（新規作成） |

計画の変更がチェックボックスのみであることは、編集後のファイルから `[x]` を `[ ]` へ戻したものが編集前の
sha256 `8770c462eaf27b3a1c5282e31a2af19a051e510184742aa83802c93d82b9eae3` と一致することで確認した。

### 変更しなかったファイル

- **ワークスペース側は 1 件も変更していない**（`git status --short -- ':!aidlc/'` が空であることを Step 3 の各プローブ直後と
  Step 6 で確認）。`.github/workflows/*`、`scripts/**`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、
  `Cargo.lock` 類、`modules/**` はすべて読取のみ。
- GitHub への書込（ruleset 変更、PR、コメント、push）は行っていない。`ruleset-required-checks.sh` は `--dry-run` を含め
  実行していない（`bash -n` の構文検査のみ）。
- commit は行っていない（親セッションが作業ツリー全体を回収する）。
- `superseding-decisions.md`、`pending-revision.md`、`developer-brief-3.md`、`developer-report-3.md`、`ruleset/` 配下、
  各履歴ファイル（`code-summary-history-2026-08-23.md` ほか）、`../nfr-requirements/`、`../nfr-design/`、他 Unit の記録は
  変更していない。
- なお `code-generation-questions.md` と `unit-test-instructions.md` にも作業ツリー上の差分があるが、これらは本委任の
  開始前に親セッションが更新したものであり、本委任では触れていない。

### センサー結果

| センサー | 結果 |
|---|---|
| `aidlc-sensor-traceability.ts --stage code-generation --output-path .../traceability.json` | `invalid_targets: []`（0 件）、`gaps: []`、`orphans: []`、`invalid_entries: []`。`missing_from_upstream_ids` は他 Unit の要求 ID 38 件で既知のノイズ、全体の `pass` は `false` のまま |
| `aidlc-sensor-required-sections.ts --stage code-generation --output-path .../code-summary.md` | `pass: true`、`h2_count: 12`、`findings_count: 0` |

追加で、`code-summary.md` の全 Markdown 表について、エスケープされていない縦棒による列ずれがないことを機械確認した
（上流所見 R-01 と同種の不具合を持ち込んでいない）。除外正規表現は §6 で表の外に置き、§11 の表内では縦棒を
エスケープしている。

## 6. 未検証範囲と親セッションへの引き継ぎ

Unit 限定コマンドと §3 の実測の成功は、設定の存在と当該コマンドの成功を示すものであり、次を検証したことにはならない。

1. **CI の全ジョブ実行** — GitHub 上の実行を今回起こしていない。`aidlc-distribution` / `quint` / `check` の CI 上の成否と、
   CI 側 `rustc` の版一致はローカル実測では代替できない。
2. **マージキューの成功・失敗両経路の実働** — 必須検査失敗時にマージが止まる経路と、全成功時にキューを完走して
   squash-merge される経路のいずれも未実施。NFR2.1 の実働受入はここに残る。
3. **レビュー結果の再評価の反映** — `review-thread-resolution.yml` の再評価が、完了済みの `CI Success` と最新のマージ条件へ
   どう反映されるかは未確認。
4. **外部再利用ワークフローの内部** — `j5ik2o/ci` 側の実装は SHA 固定で参照しているだけで、内部挙動は未検証。
5. **カバレッジ相対ゲートの実働** — `coverage.sh --base` の base 側計測と比較は未実行（今回は絶対ゲートのみ 2 回）。
6. **`ruleset-required-checks.sh` の書込経路** — `--dry-run` を含め未実行のため、PUT 経路と前後 JSON の生成は未実測。

引き継ぎ事項:

- 記録側の差分は上記 §5 の 5 ファイルに限られる。ワークスペース側は差分ゼロなので、親セッションは全差分レビュー後に
  監査シャードを含む作業ツリー全体を回収してよい。
- 独立レビューへの引き渡し準備は整っている（`code-summary.md` の末尾に `## Review` 節は置いていない）。
- 品質目標（90% 床・0.01・シード・除外・検査スクリプトの期待値）は入力として扱い、緩和・書き換え・無効化はしていない。
  未達の項目もない。
