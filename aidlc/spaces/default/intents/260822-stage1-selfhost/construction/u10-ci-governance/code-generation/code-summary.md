# code-summary — U10 CI ガバナンス（`u10-ci-governance`）

> Code Generation（Construction 3.5）の実装要約（Unit: U10、Bolt: B2、ブランチ `bolt/b2-u10-ci-governance`、base = `origin/main`
> 9c4ee51 = PR #24 マージ後）。出典: `code-generation-plan.md`（Step 0〜11、承認指紋 `sha256:7f0e1353…f3c75`）、
> `unit-test-instructions.md`、開発エージェントの報告 `developer-report-3.md`（委任 3、Step 1〜9）、コンダクタによる差分レビューと
> 品質ゲート再実行（2026-08-22 UTC）。
>
> 計画ファイルのチェックボックスは承認時のバイト列のまま（承認指紋がファイル全体に掛かるため）。Step の完了状況は本ファイル §1 が正本。

## 1. Step ごとの完了状況

| Step | 内容 | 状況 |
|---|---|---|
| 0 | PR #24 マージ確認 → `origin/main` から `bolt/b2-u10-ci-governance` → `aidlc-bolt.ts start --name B2` → 記録コミット `055195b` | 完了（コンダクタ） |
| 1 | `scripts/governance/verify-ci-governance.sh` の骨格、`bash -n`、ランナー疎通（`tools/lint` 31 本緑） | 完了 `ad3fc0e` |
| 2 | **Red**: 検査 15 項目を現状ツリーで実行 → PASS 1 / FAIL 14、exit 1 | 完了（証跡 §3） |
| 3 | FR9.2: `rust-toolchain.toml`（1.95.0 / rustfmt, clippy, llvm-tools / minimal）、`unsafe_code = "forbid"` を workspace と `tools/lint` に、`ci.yml` に `permissions: contents: read`・toolchain `@master`（ファイル駆動）・`audit` ジョブ | 完了 `225c4c6` `7702372` |
| 4 | FR9.3: `check` ジョブに `tools/lint` の fmt / clippy / test 3 ステップ | 完了 `cceb1bc` |
| 5 | FR9.4 / 9.5: `scripts/coverage.sh` に `PROPTEST_RNG_SEED=20260823` export・`--ignore-filename-regex`・`TOLERANCE`（承認値 0.01 → Bolt B2 ゲート裁定で暫定 **0.05**、`07b6a94`）、`ci.yml` の `check` / `coverage` に同シード env | 完了 `3dc1a3f`（除外 regex は `ba75234` で訂正 — §6） |
| 6 | FR9.1: `ci.yml` に `merge_group: {}`、coverage は `pull_request` 時のみ `--base` | 完了 `f7b8e3e`（**Green**: PASS 15 / FAIL 0） |
| 7 | `scripts/governance/ruleset-required-checks.sh`（`--dry-run` / `--out-dir`、required コンテキスト集合 + strict での冪等判定、前後 JSON、`jq` 検証）、`--dry-run` 確認 | 完了 `7af3194`（PUT は未実行） |
| 8 | Refactor（欠損ファイル処理の集約、検出力の再確認） | 完了 `43e1dd9` |
| 9 | 品質ゲート（fmt / clippy / lint / test / tools-lint 3 コマンド / verify）— コンダクタ再実行でも全緑 | 完了 |
| 10 | ruleset への required checks 適用（オーナー権限、前後 JSON を記録） | **完了**（2026-08-22T23:43Z、オーナー承認のうえコンダクタが実行。`ruleset/before.json` / `after.json`、`verify --with-ruleset` 16/16 PASS） |
| 11 | Bolt ゲート → PR → `merge_group` CI の実行確認 → merge queue 完走（正常系受入） | **完了**（PR #25: `merge_group` CI 4 ジョブ緑 → squash-merge 2026-08-22T23:44Z。初回 CI の toolchain 赤は `75bf0fe` で修正 — §6） |

## 2. 作成・変更ファイル

- 新規: `rust-toolchain.toml`、`scripts/governance/verify-ci-governance.sh`（348 行、検査 15 + ruleset 1 項目）、
  `scripts/governance/ruleset-required-checks.sh`（211 行）
- 変更: `.github/workflows/ci.yml`（+70: `merge_group` / `permissions` / `@master` toolchain / `tools/lint` 3 ステップ / coverage 分岐 /
  `audit` ジョブ / `PROPTEST_RNG_SEED` env）、`Cargo.toml`（`[workspace.lints.rust] unsafe_code = "forbid"`）、`tools/lint/Cargo.toml`
  （`[lints.rust] unsafe_code = "forbid"`）、`scripts/coverage.sh`（+39/-16: シード export、除外定数、`TOLERANCE`（暫定 0.05）、コメント）、`scripts/governance/toolchain-inputs.sh`（新規 — `75bf0fe`、`rust-toolchain.toml` から `@master` の入力を導出）
- プロダクトコード（`modules/**/src/`）の変更なし。`Cargo.lock` / `tools/lint/Cargo.lock` 不変（依存追加なし）。

## 3. TDD の証跡（packaging への写し）

- **Red**（Step 2、変更前ツリー）: `bash scripts/governance/verify-ci-governance.sh` → `--- 合計: PASS 1 / FAIL 14 ---`、exit 1。
  FAIL: toolchain-channel / -components / -profile、workspace-unsafe-forbid、tools-lint-unsafe-forbid、ci-merge-group-trigger、
  ci-permissions-contents-read、ci-toolchain-file-driven、ci-tools-lint-steps、ci-audit-job、ci-proptest-seed-env、coverage-tolerance、
  coverage-ignore-regex、coverage-proptest-seed（PASS: ci-coverage-base-condition — 既存の `if:` 分岐が要件を満たしていた）。
  計画の「9 項目」は検査を対象ファイル単位に細分したため 14 項目に展開（対応表は developer-report-3.md §3）。
- **Green の推移**: Step 3 後 9/6 → Step 4 後 10/5 → Step 5 後 14/1 → **Step 6 後 15/0、exit 0**。Refactor 後も 15/0。
- **検出力の確認**: 最終版スクリプトを変更前ツリー（`git archive ad3fc0e`）に当てて同じ PASS 1 / FAIL 14 を再現。対象ファイルを
  置かないツリーでは PASS 0 / FAIL 15（欠損を黙って PASS にしない）。
- `--with-ruleset`: ruleset 未変更のため 1 項目 FAIL（`実際: [なし] strict=false / 期待: [check coverage quint] strict=true`）— 想定どおり。
- `tools/lint` 自己テスト: `test result: ok. 31 passed`。

## 4. 受入の実測

| 項目 | 実測 |
|---|---|
| `cargo audit`（ローカルに `cargo install cargo-audit --locked` で 0.22.2 導入） | workspace: 74 crate 走査・脆弱性なし（exit 0）。`--file tools/lint/Cargo.lock`: 5 crate・脆弱性なし |
| toolchain 固定 | `rust-toolchain.toml` 配置で rustup が 1.95.0 へ切替、`llvm-tools` を自動取得。`rustc 1.95.0 (59807616e 2026-04-14)` |
| `tools/lint` 3 コマンド | fmt / clippy / test すべて exit 0 |
| カバレッジ除外（NFR2.5） | 訂正後 `main.rs` が計測対象から消え、総行数 5717 → 5715。除外は `main.rs` 1 ファイルのみ |
| **カバレッジ決定化（NFR2.4）** | **未達**: 同条件 8 回計測で 5549 行 / 5550 行がほぼ半々（差 **0.0175pp** = 1 行）。原因は PBT ではなく `modules/core/interface-adapter/src/workspace/fs_workspace_lock.rs:237`（`unstamped_is_over_grace` の `Err(_)` 分岐）を並行テスト（4 スレッド × 15 回の実 FS 競合）が踏むかがスケジューラ依存。`PROPTEST_RNG_SEED` が読まれていることは不正値で実証済み。他の全ファイルは 2 回とも完全一致 |
| ruleset スクリプト `--dry-run` | exit 0、PUT 未実行。組み立て JSON は既存 3 規則 + `bypass_actors: []` を維持し `required_status_checks`（check / quint / coverage、strict）を追加。異常系（未知引数 exit 2、`--out-dir` 値なし exit 1、存在しない ruleset 名 exit 1）確認済み。コンダクタも `--dry-run` を再実行し同じ JSON を確認 |
| 品質ゲート（コンダクタ再実行） | fmt OK / clippy 0 警告 / `cargo lint` OK / `cargo test --workspace` 338 passed / tools-lint fmt・clippy OK・31 passed / verify 15 PASS（ruleset 適用後は 16 PASS） |

## 5. 主要な実装判断

- **toolchain はファイル駆動**（`dtolnay/rust-toolchain@master` + `rust-toolchain.toml`、`components:` 入力撤去）— 正本を 1 つに。
- **`audit` は required 外**（advisory DB の一時障害で全マージを止めない — nfr-design §2）。
- **除外 regex のアンカー**: llvm-cov はカバレッジデータに**絶対パス**を記録するため、承認済みリテラル `^modules/app/aidlc/src/main\.rs$`
  は**不活性**（`main.rs` が 0/2 行で計測対象に残っていた）。`(^|/)modules/app/aidlc/src/main\.rs$` に訂正し実効化（相対パス基準の意図 =
  ルート直下の `modules/...` 断片に限定 — を維持。相対ゲートの base 側は別 worktree の絶対パスで計測されるためこの形が必要）。
- **冪等判定**は required コンテキスト集合 + strict フラグの一致（nfr-design レビュー Minor 2 の引き取り）。

## 6. 計画からの逸脱

- **除外 regex のアンカー訂正**（承認済み tech-stack-decisions §1 / security-design §4 のリテラルから逸脱、理由は §5）— 正本の更新は
  ゲートで確認。
- 検査項目の粒度（9 → 14）— 説明のみ。検査を「`--ignore-filename-regex` を渡している」+「regex が期待値」の 2 事実に分割（定数化に対応）。
- `TOLERANCE=0.01` は承認どおり実装したが NFR2.4 の受入（差 0.00pp）は未達 — 残ジッタ 0.0175pp > 0.01 のため相対ゲートが
  偽陽性になりうる。**Bolt B2 ゲートのオーナー裁定（2026-08-22T23:34Z）: 暫定 0.05**（`07b6a94`、検査スクリプトの期待値も同期）。U3 の
  ロック退役後に 0.01 へ。
- CI 初回実行で `dtolnay/rust-toolchain@master` が `toolchain:` 入力必須と判明（設計 security-design §2 の前提が誤り）→
  `scripts/governance/toolchain-inputs.sh` で `rust-toolchain.toml` から入力を導出（`75bf0fe`、正本は 1 つのまま）。
- `shellcheck` 未実行（ローカル未導入、計画上も任意）。

## 7. 後続への引き渡し

- U3（`u3-event-store-repository`、ロック退役 ADR-007）: ジッタ源 `fs_workspace_lock.rs` はロック退役で消える見込み — 退役後に
  `TOLERANCE` を再検討（0.01 へ）。
- U7: `unsafe_code` forbid 昇格で赤になるクレートは無かった（修正不要）。
- 後続 intent: Dependabot / アクションの SHA ピン留め、`audit` の required 化（運用 1 週間後）。
- U9: `tech-stack-decisions` / `security-design` の除外 regex リテラルと `TOLERANCE` の文面更新（裁定後）。

## 8. 未解決・要確認（ゲートで裁定）

- **(a) `TOLERANCE` の扱い（NFR2.4 未達）— 裁定済み**: 暫定 0.05（レビュー Minor 2 の「0.02 は余裕が薄い」を受けて 0.02 ではなく 0.05）。
  U3 のロック退役後に 0.01 へ（U3 へ引き継ぎ）。
- **(b) 除外 regex の正本更新**（`(^|/)` アンカー）— tech-stack-decisions §1 / security-design §4 へ反映するか。
- **(c) `dtolnay/rust-toolchain@master` の CI 実挙動 — 解決済み**: `toolchain:` 入力必須でファイルを自動では読まなかった（初回 CI 赤）。
  `toolchain-inputs.sh` でファイルから導出して渡す形に修正（`75bf0fe`）、2 回目の CI は 4 ジョブ緑。
- **(d) ruleset 適用（Step 10）— 実施済み（2026-08-22T23:43Z）**: ゲート承認後、`scripts/governance/ruleset-required-checks.sh --out-dir
  <record>/construction/u10-ci-governance/code-generation/ruleset/` をオーナー権限で実行（`gh auth` はオーナーアカウント — コンダクタが
  承認のうえ実行可）。**PR の CI が緑になる前に required checks を有効化すると本 Bolt の PR 自身が `merge_group` で検証されるので順序は
  「PR 作成 → PR の CI 緑確認 → ruleset 適用 → queue 投入」** が安全。
- **(e) `strict_required_status_checks_policy: true` と merge queue の相互作用 — 実地で問題なし**: PR #25 が required checks 下で queue を完走（23:44Z）。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T01:33:00Z
**Iteration:** 2（recovery, unit: u10-ci-governance）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Critical | code-summary.md §1（Step 1・Step 10）、§2、§3、§5 | 本ファイルは review-thread ゲート（superseding-decisions.md #9、オーナー指示 2026-08-23T00:40Z）を一切記載していない。実装（`.github/workflows/ci.yml`・`.github/workflows/review-thread-resolution.yml`・`scripts/governance/verify-ci-governance.sh`・`scripts/governance/ruleset-required-checks.sh`、いずれも実測確認済み）には (a) 新規ジョブ `review-thread-resolution`（`j5ik2o/ci` の再利用ワークフローを SHA `9cf0e9a8cd74c72de704763025003ed3b7608c65` で固定呼び出し、ジョブ個別に `checks: write` / `issues: read` / `pull-requests: read` / `statuses: write` を付与）と集約ジョブ `ci-success`、(b) 新規ファイル `.github/workflows/review-thread-resolution.yml`（レビュー系イベント + 15分毎 cron で再評価）、(c) ruleset の required checks が `check` / `quint` / `coverage` に加え `CI Success` の4コンテキストへ拡張（`ruleset/2026-08-23-ci-success/after.json` で確認）が存在するが、本ファイルの §2「作成・変更ファイル」に `review-thread-resolution.yml` の記載が無く、§5「主要な実装判断」にもジョブ個別 `permissions` 付与への言及が無い。§1 Step 1 は「検査 15 項目」、Step 10 は「`--with-ruleset` 16/16 PASS」と記すが、`verify-ci-governance.sh` の実行結果は現在 PASS 19/19（`--with-ruleset` で PASS 20/20）— 検査が3項目（`ci-review-thread-gate` / `ci-success-aggregate` / `ci-review-thread-refresh-workflow`）増えた事実が反映されていない。本 Unit は CI ガバナンスの監査台帳そのものが成果物であり、この欠落は「実装は正しいが記録に無い」ため、次にこのファイルだけを読む人間・エージェントは review-thread ゲートの存在（と SHA 固定・権限付与という安全性に関わる設計判断）を知りえない。このゲート dispatch 自身が「review-thread gate の追加で code-summary.md / traceability.json … が更新された」と前提しているが、実測はそれに反する（stale-receipt）。 | §1〜§5 に review-thread ゲート（新規ファイル・新規ジョブ2つ・ジョブ個別 permissions・ruleset 4コンテキスト化）を追記し、Step 1/10 の検査数値を実測（19 / 20）に更新する。 |
| 2 | Major | traceability.json（FR9.1 / NFR2.1 / NFR4.5 の `target`） | `aidlc-sensor-traceability.ts` を実行すると `invalid_targets` に上記3件が挙がる（`target file does not exist: scripts/governance/ruleset-required-checks.sh（2026-08-22T23:43Z に適用済み — ruleset/after.json）` 等）。原因は `target` フィールドに全角括弧付き注記を連結しているため、実在ファイルへのパスとして解決できないこと。ステージ定義（`code-generation.md`）は「Every `OK` target must be one existing workspace-relative file」と明記し、Sensors 節も「traceability … verifies every `OK` target is an existing workspace-relative file」としている — 本ファイルはこの契約に違反しており、機械検証がFAILする。 | `target` は `scripts/governance/ruleset-required-checks.sh` のようにファイルパス単体にし、適用日時・参照JSON等の注記は `status`/`target` 以外のフィールドか code-summary.md 側の説明に移す。 |
| 3 | Major | traceability.json（FR9.1 / NFR2.1 / NFR4.4 / NFR4.5 の内容） | Finding #1 と同根: traceability.json も review-thread ゲート追加後の状態（ruleset 4コンテキスト、`review-thread-resolution.yml` の新規追加、ジョブ個別 `permissions` 付与）を反映していない。superseding-decisions.md #11 はまさにこの不一致を `nfr-requirements/security-requirements.md` について Major 指摘済み（回復レビュー 2026-08-23T00:36Z）であり、同じ齟齬が code-generation 側の成果物にも及んでいることが今回の実測で確認できた。 | FR9.1 / NFR2.1 / NFR4.5 の `target` を `.github/workflows/ci.yml` と `.github/workflows/review-thread-resolution.yml` の組へ更新し、NFR4.4 の記述（もしあれば）にジョブ個別 permissions 付与を反映する。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `bash scripts/governance/verify-ci-governance.sh` | PASS 19 / FAIL 0（`--with-ruleset` 併用で PASS 20 / FAIL 0、GitHub 上の実 ruleset と一致） | 実装（スクリプト・`ci.yml`・`review-thread-resolution.yml`・ruleset）自体は全検査を満たしており健全。問題は記録側（Finding 1〜3）にある |
| `bash scripts/governance/toolchain-inputs.sh` | `channel=1.95.0` / `components=rustfmt,clippy,llvm-tools` — `rust-toolchain.toml` と完全一致 | FR9.2 / NFR4.2 の toolchain 入力導出は健全 |
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage code-generation`（traceability.json） | `pass:false`、`invalid_targets` 3件（FR9.1 / NFR2.1 / NFR4.5）、`missing_from_upstream_ids` 多数 | `invalid_targets` は Finding 2 の直接証拠。`missing_from_upstream_ids`（FR1〜FR8, NFR1/3/5 等）はステージ定義が明記する通り `upstream-coverage` センサーを code-generation にインポートしていないため許容される既知のノイズ（per-unit の狭い upstream_ids 設計と整合） |
| `bun .claude/tools/aidlc-sensor-required-sections.ts --stage code-generation`（code-summary.md） | `pass:true`、H2 8件 | 文書形状は充足 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts --stage code-generation`（unit-test-instructions.md） | `pass:true`、H2 5件（凍結文書、参考確認のみ） | 文書形状は充足 |

### Summary

実装そのもの（`verify-ci-governance.sh` 19/19・`--with-ruleset` 20/20、`toolchain-inputs.sh`、review-thread ゲートの SHA固定・権限付与、ruleset の4コンテキスト化）は実測ですべて健全であり、superseding-decisions.md の記録（#1〜#11）とも整合している。しかし本ステージの成果物である `code-summary.md` と `traceability.json` は、その実装のうち review-thread ゲート追加（superseding #9、オーナー指示 2026-08-23T00:40Z）を一切反映しておらず、traceability.json は3件の `target` がセンサー実行で機械的に無効と判定される。今回の回復レビューの前提（「code-summary.md / traceability.json … が更新された」）自体が実態と食い違っている（stale-receipt）ため、Critical 1件・Major 2件で NOT-READY とする。
