# code-summary — U10 CI ガバナンス（`u10-ci-governance`）

> Code Generation（Construction 3.5）の実装要約（Unit: U10、Bolt: B2、ブランチ `bolt/b2-u10-ci-governance`、base = `origin/main`
> 9c4ee51 = PR #24 マージ後）。出典: `code-generation-plan.md`（Step 0〜11、承認指紋 `sha256:7f0e1353…f3c75`）、
> `unit-test-instructions.md`、開発エージェントの報告 `developer-report-3.md`（委任 3、Step 1〜9）、コンダクタによる差分レビューと
> 品質ゲート再実行（2026-08-23）。
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
| 5 | FR9.4 / 9.5: `scripts/coverage.sh` に `PROPTEST_RNG_SEED=20260823` export・`--ignore-filename-regex`・`TOLERANCE=0.01`、`ci.yml` の `check` / `coverage` に同シード env | 完了 `3dc1a3f`（除外 regex は `ba75234` で訂正 — §6） |
| 6 | FR9.1: `ci.yml` に `merge_group: {}`、coverage は `pull_request` 時のみ `--base` | 完了 `f7b8e3e`（**Green**: PASS 15 / FAIL 0） |
| 7 | `scripts/governance/ruleset-required-checks.sh`（`--dry-run` / `--out-dir`、required コンテキスト集合 + strict での冪等判定、前後 JSON、`jq` 検証）、`--dry-run` 確認 | 完了 `7af3194`（PUT は未実行） |
| 8 | Refactor（欠損ファイル処理の集約、検出力の再確認） | 完了 `43e1dd9` |
| 9 | 品質ゲート（fmt / clippy / lint / test / tools-lint 3 コマンド / verify）— コンダクタ再実行でも全緑 | 完了 |
| 10 | ruleset への required checks 適用（オーナー権限、前後 JSON を記録） | **未実施** — Bolt ゲート後にオーナー承認のうえ実行 |
| 11 | Bolt ゲート → PR → `merge_group` CI の実行確認 → merge queue 完走（正常系受入） | **未実施** — ゲート後 |

## 2. 作成・変更ファイル

- 新規: `rust-toolchain.toml`、`scripts/governance/verify-ci-governance.sh`（348 行、検査 15 + ruleset 1 項目）、
  `scripts/governance/ruleset-required-checks.sh`（211 行）
- 変更: `.github/workflows/ci.yml`（+70: `merge_group` / `permissions` / `@master` toolchain / `tools/lint` 3 ステップ / coverage 分岐 /
  `audit` ジョブ / `PROPTEST_RNG_SEED` env）、`Cargo.toml`（`[workspace.lints.rust] unsafe_code = "forbid"`）、`tools/lint/Cargo.toml`
  （`[lints.rust] unsafe_code = "forbid"`）、`scripts/coverage.sh`（+39/-16: シード export、除外定数、`TOLERANCE=0.01`、コメント）
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
| 品質ゲート（コンダクタ再実行） | fmt OK / clippy 0 警告 / `cargo lint` OK / `cargo test --workspace` 338 passed / tools-lint fmt・clippy OK・31 passed / verify 15 PASS |

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
  偽陽性になりうる。**オーナー裁定待ち**（§8 (a)）。
- `shellcheck` 未実行（ローカル未導入、計画上も任意）。

## 7. 後続への引き渡し

- U3（`u3-event-store-repository`、ロック退役 ADR-007）: ジッタ源 `fs_workspace_lock.rs` はロック退役で消える見込み — 退役後に
  `TOLERANCE` を再検討（0.01 へ）。
- U7: `unsafe_code` forbid 昇格で赤になるクレートは無かった（修正不要）。
- 後続 intent: Dependabot / アクションの SHA ピン留め、`audit` の required 化（運用 1 週間後）。
- U9: `tech-stack-decisions` / `security-design` の除外 regex リテラルと `TOLERANCE` の文面更新（裁定後）。

## 8. 未解決・要確認（ゲートで裁定）

- **(a) `TOLERANCE` の扱い（NFR2.4 未達）**: A `0.02`（実測ジッタ 0.0175pp を包む最小値、従来 0.5 の 25 倍厳格）/ B 0.01 のまま、
  `fs_workspace_lock.rs:237` の `Err` 分岐を決定的に覆う単体テストを追加（U10 境界外 — interface-adapter のテスト）/ C 0.01 のまま、
  稀な偽陽性は再実行で対処。
- **(b) 除外 regex の正本更新**（`(^|/)` アンカー）— tech-stack-decisions §1 / security-design §4 へ反映するか。
- **(c) `dtolnay/rust-toolchain@master` の CI 実挙動**: `components:` 入力なしで `rust-toolchain.toml` の 3 コンポーネントが入るかは
  本 Bolt の PR 初回 CI で確認。赤なら `rust-toolchain.toml` 側で解決（正本は 1 つ）。
- **(d) ruleset 適用（Step 10）の実行者とタイミング**: ゲート承認後、`scripts/governance/ruleset-required-checks.sh --out-dir
  <record>/construction/u10-ci-governance/code-generation/ruleset/` をオーナー権限で実行（`gh auth` はオーナーアカウント — コンダクタが
  承認のうえ実行可）。**PR の CI が緑になる前に required checks を有効化すると本 Bolt の PR 自身が `merge_group` で検証されるので順序は
  「PR 作成 → PR の CI 緑確認 → ruleset 適用 → queue 投入」** が安全。
- **(e) `strict_required_status_checks_policy: true` と merge queue の相互作用**: 正常系（緑 PR の queue 完走）は本 Bolt の PR で実地確認。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-22T23:32:21Z
**Iteration:** 1（advisory, unit: u10-ci-governance）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | traceability.json (`NFR2.4`)、code-summary.md §8 (a)、`scripts/coverage.sh:42` | 承認済み合格基準（NFR2.4「2 回計測で差 0.00pp」）が未達のまま実装済み。シード固定後も `fs_workspace_lock.rs:237`（`unstamped_is_over_grace` の `Err` 分岐、並行 FS テスト由来）が OS スケジューラ依存で ±1 行（0.0175pp）揺れ、実装済みの `TOLERANCE=0.01` を上回る。`pull_request` イベントでのみ相対ゲートが働く（`merge_group` は絶対ゲートのみ）ため実害は PR 単位の偶発的赤 → 再実行で収まる程度に限定されるが、要求の合格基準そのものは満たしていない。開発チームはこれを `traceability.json` で正直に `Deferred` とし、§8 (a) で選択肢 A/B/C を提示済み — 隠蔽ではなく適切に裁定待ちに回している点は評価できる。ゲート承認前にオーナー裁定が必要。 | オーナーが A（`TOLERANCE=0.02`）/ B（`fs_workspace_lock.rs` の `Err` 分岐を決定的に踏む単体テストを追加、U7 か後続 Unit）/ C（0.01 維持・再実行で対処）のいずれかを選び、選定後は `team.md` / `tech-stack-decisions.md` の「0.01 へ引き締める」記述と `traceability.json` の `NFR2.4` を `OK` に更新する。 |
| 2 | Minor | code-summary.md §8 (a) 選択肢 A | 選択肢 A（`TOLERANCE=0.02`）は実測ジッタ 0.0175pp に対し余裕が約 0.0025pp しかない。ジッタの発生源（`fs_workspace_lock_test.rs` の 4 スレッド×15 回の実 FS 競合）はスケジューラ依存で「常に ±1 行以内」という上限が数学的に保証されているわけではなく、負荷条件次第で±2 行（約 0.035pp）に広がる可能性を否定できない。8 回の観測（5549 が 4 回・5550 が 4 回）はサンプル数が少なく、稀な外れ値を捉えられていない可能性がある。 | A を採るなら本番相当の負荷（CI ランナー相当の同時実行数）で 20〜30 回程度の反復計測を行い、実際の分布上限を確認してから閾値を決める。恒久対応としては B（決定的単体テスト）の方が再発しない解決になる。 |
| 3 | Minor | code-summary.md §8 (c)、`.github/workflows/ci.yml`（`dtolnay/rust-toolchain@master`） | `components:` 入力を撤去し `rust-toolchain.toml` 駆動にした切替はローカル（macOS の `rustup`）でのみ実証済みで、GitHub Actions ランナー上で `@master` が `toolchain:` 入力なしのまま `rust-toolchain.toml` の 3 コンポーネント（`rustfmt`, `clippy`, `llvm-tools`）を自動取得するかは本 Bolt の PR の初回 CI 実行まで未確認（開発チームも §8 (c) で認めている既知のギャップ）。特に `llvm-tools` を欠くと `coverage` ジョブが `cargo-llvm-cov` のインストール後に失敗し得る。 | PR 作成後、初回 CI で `check` / `coverage` 両ジョブの toolchain インストールログを確認する。赤なら `ci.yml` 側に `components:` を戻すのではなく `rust-toolchain.toml` 側で解決する方針（正本を 1 つに保つ、§8 (c) の既定方針）を維持する。 |
| 4 | Minor | `../nfr-requirements/tech-stack-decisions.md` §1、`../nfr-design/security-design.md` §4 vs 実装（`scripts/coverage.sh:56`） | 実装は承認済みリテラル `^modules/app/aidlc/src/main\.rs$` を `(^|/)modules/app/aidlc/src/main\.rs$` に訂正している（llvm-cov がカバレッジデータへ絶対パスを記録するため `^` 単独アンカーが不活性だった、という実測に基づく正当な理由 — 訂正前後で `main.rs` が計測対象から外れる／外れないことを実地確認済み）。しかし上流の `tech-stack-decisions.md` §1 と `security-design.md` §4 は訂正前のリテラルのまま残っており、正本と実装の間に逐語の食い違いが生じている（nfr-design レビューの Finding #1 で既に一度指摘されていた箇所が、今回訂正はされたが正本更新は追随していない）。 | `code-summary.md` §8 (b) の記載どおり、ゲートで `tech-stack-decisions.md` §1 と `security-design.md` §4 のリテラルを `(^|/)` アンカー版に更新し、正本と実装を一致させる。 |

### Validation Tool Results

| チェック | コマンド | 結果 | 解釈 |
|---|---|---|---|
| 差分ファイル一覧 | `git diff origin/main..HEAD --stat -- . ':!aidlc'` | 7 ファイル変更（`.github/workflows/ci.yml` +70、`Cargo.toml` +5、`rust-toolchain.toml` +12（新規）、`scripts/coverage.sh` +39/-16、`scripts/governance/ruleset-required-checks.sh` +211（新規）、`scripts/governance/verify-ci-governance.sh` +348（新規）、`tools/lint/Cargo.toml` +4/-2） | code-summary §2 の「作成・変更ファイル」記述と完全一致。`modules/**/src/` に変更なし（プロダクトコード非改変の境界を遵守） |
| `bash scripts/governance/verify-ci-governance.sh` | 直接実行 | `--- 合計: PASS 15 / FAIL 0 ---`、exit 0 | code-summary §3 の Green 最終状態（15/0）を再現。developer-report-3.md の記録と一致 |
| `bash scripts/governance/ruleset-required-checks.sh --dry-run` | 直接実行 | 現在: `[なし] strict=false` → 期待: `[check coverage quint] strict=true`。組み立て JSON は `deletion` / `non_fast_forward` / `merge_queue`（SQUASH/ALLGREEN、既存パラメータ全一致）+ 新規 `required_status_checks` を含み `bypass_actors: []` を維持。PUT 未実行、exit 0 | code-summary §4「ruleset スクリプト」の記述と一致。読取専用の `gh api repos/amadeus-dlc/amadeus-ng/rulesets/21190453` でも現状 ruleset に `required_status_checks` ルールが無いことを独立に確認済み — dry-run の起点データが実物と一致 |
| `cargo fmt --all --check` | 直接実行 | exit 0 | 緑 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 直接実行 | exit 0（warning 0） | 緑。`unsafe_code = "forbid"` 昇格後も赤なし |
| `cargo lint` | 直接実行 | exit 0（所見 0 件） | 緑 |
| `cargo test --manifest-path tools/lint/Cargo.toml` | 直接実行 | `test result: ok. 31 passed; 0 failed` | code-summary §3 の記録と一致 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts`（code-summary.md） | 直接実行 | `{"pass":true,"h2_count":8,...}` | 8 個の H2 すべて既定の必須節を満たす |
| `bun .claude/tools/aidlc-sensor-traceability.ts`（code-generation/traceability.json） | 直接実行 | `{"pass":false,...,"missing_from_upstream_ids":[38 件: FR1〜FR8 系・NFR1・NFR3・NFR5 等],...}` | **誤検知（所見に数えない）**。`inception/units-generation/unit-of-work.md:192` に同一の既知の限界が記録済み: このセンサーは `story-map` の `USx.y` 形式行しか認識できず、本プロジェクト（stories.md 不在、FR 直結の story-map）では FR 直結行を誤検知する。手動突合（`unit-of-work-story-map.md:44-59`）で traceability.json の `upstream_ids`（FR9.1〜9.5・NFR2.1〜2.5・NFR4.1〜4.5、15 件）が U10 の割当範囲と過不足なく一致し、U9 割当の FR9.6 が正しく除外されていることを確認した |
| ruleset PUT ペイロードの内容照合（手動） | `gh api .../rulesets/21190453` の実測 JSON と dry-run 出力を突合 | `name`/`target`/`enforcement`/`conditions`/`bypass_actors`（空）/既存 3 規則のパラメータ（`merge_queue`: `SQUASH`, `max_entries_to_build:1`, `min/max_entries_to_merge:1`, `grouping_strategy:ALLGREEN`, `check_response_timeout_minutes:60`）が完全一致。追加は `required_status_checks`（`check`/`coverage`/`quint`、`strict:true`）のみ | ruleset-required-checks.sh:107-118 の `build_payload` が実測どおり既存規則を破壊せず新規則のみ追加する設計であることを実地で確認 |
| `verify-ci-governance.sh` の検出力（developer-report-3.md §3 の記述を確認） | — | 最終版スクリプトを変更前ツリー（`git archive ad3fc0e`）に当てて同一の PASS 1/FAIL 14 を再現、対象ファイル欠損ツリーで PASS 0/FAIL 15（黙って PASS にしない） | Construction Phase Guardrails の「テストは常に通るものであってはならない」要求を満たす。報告内容を裏取り済み |
| bash スクリプト品質（`verify-ci-governance.sh` / `ruleset-required-checks.sh` 目視） | — | 両スクリプトとも `set -euo pipefail`、`require_cmd`、`die`/`fail` によるエラーメッセージの明示化、`--dry-run` 既定安全側、bash 3.2 互換の注記あり | 品質基準を満たす |

### Summary

U10（packaging）の code-generation 成果物は、承認済み計画・設計との対応が高い精度で保たれており、TDD の証跡（Red 14 FAIL → Green 15/0、検出力の再現確認）・品質ゲート（fmt/clippy/lint/test/tools-lint 3 コマンド/verify、いずれもローカル再実行で緑を確認）・ruleset スクリプトの安全性（`--dry-run` 既定、既存 3 規則と `bypass_actors` の完全保持、冪等判定が規則の中身まで見る設計）のいずれも実地検証で裏付けが取れた。最大の技術的懸念は NFR2.4「2 回計測で差 0.00pp」が未達であること（Major #1）だが、これは隠蔽されておらず `traceability.json` で `Deferred` と正直に記録され、原因（`fs_workspace_lock.rs:237` の並行テスト由来の 1 行ジッタ、PBT 起因ではないことを実証済み）と選択肢 A/B/C がゲート裁定用に整理されている。残る所見（TOLERANCE=0.02 案の余裕の薄さ、`@master` トゥールチェーンの CI 実挙動未確認、除外 regex 訂正の正本未反映）はいずれも Minor で、実装そのものの正しさを損なうものではなく次のステップ（PR 初回 CI・ゲート裁定）で解消できる。Critical 所見は無く、Major は 1 件のみでいずれもオーナー裁定が必要な既知の未解決事項として適切に可視化されているため、READY と判定する。

**更新ファイル:** `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/code-generation/code-summary.md`（`## Review` セクションを末尾に追加）。`code-generation-plan.md`・`unit-test-instructions.md`・`traceability.json`・`developer-report-3.md` は編集していない。
