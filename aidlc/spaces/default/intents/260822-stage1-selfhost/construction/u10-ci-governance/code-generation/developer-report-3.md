AIDLC-UNIT: u10-ci-governance
AIDLC-TESTING-CONTRACT: sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3

# developer-report-3 — U10 CI ガバナンス、計画 Step 1〜9

ブランチ `bolt/b2-u10-ci-governance`（`origin/main` = PR #24 マージ後）。push / PR / ブランチ切替は行っていない。
ruleset の PUT（Step 10）は担当外のため実行していない（`--dry-run` まで）。

## 1. 実行した Step と結果

| Step | 内容 | 結果 |
|---|---|---|
| 1 | `scripts/governance/` 作成、`verify-ci-governance.sh` の骨格、`bash -n`、ランナー疎通 | 完了。`bash -n` exit 0、`cargo test --manifest-path tools/lint/Cargo.toml` 31 本緑 |
| 2 | **Red**: 検査項目を実装し現状ツリーで実行 | 完了。15 項目中 **14 FAIL / 1 PASS**、exit 1（§3） |
| 3 | FR9.2: `rust-toolchain.toml`、`unsafe_code = "forbid"` ×2、`permissions`、toolchain `@master`、`audit` ジョブ | 完了。8 項目が PASS へ（PASS 1→9）。`cargo build --workspace` / `cargo clippy` 緑 |
| 4 | FR9.3: `check` ジョブに `tools/lint` の fmt / clippy / test 3 ステップ | 完了。1 項目 PASS（PASS 9→10）。ローカル 3 コマンドとも exit 0 |
| 5 | FR9.4 / FR9.5: `PROPTEST_RNG_SEED` export、`--ignore-filename-regex`、`TOLERANCE=0.01`、`ci.yml` の env | 完了。4 項目 PASS（PASS 10→14） |
| 6 | FR9.1: `merge_group: {}` と coverage の条件分岐 | 完了。**15 PASS / 0 FAIL、exit 0（Green）** |
| 7 | `ruleset-required-checks.sh`（`--dry-run` / `--out-dir`、集合による冪等判定、前後 JSON、`jq` 検証） | 完了。`--dry-run` の組み立て JSON を確認（§5）。`--with-ruleset` は実行前のため FAIL のまま（想定どおり） |
| 8 | **Refactor**: 欠損ファイル処理の集約、`bash -n`、shellcheck | 完了（shellcheck はローカル未導入のため未実行）。検出力が落ちていないことを再測で確認（§3） |
| 9 | 品質ゲートと意味単位コミット | 完了。fmt / clippy / `cargo lint` / `cargo test --workspace` すべて exit 0（§7）。コミット 8 本（§8） |

Step 9 の実行中に **承認済み設計のカバレッジ除外リテラルが不活性である**ことが実測で判明したため、
最小限の訂正を入れて再度 Green を確認した（§6 の逸脱 1）。

## 2. 作成・変更ファイル一覧（ワークスペース相対）

新規:
- `rust-toolchain.toml`
- `scripts/governance/verify-ci-governance.sh`
- `scripts/governance/ruleset-required-checks.sh`
- `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/code-generation/developer-report-3.md`（本ファイル）

変更:
- `.github/workflows/ci.yml`
- `Cargo.toml`（`[workspace.lints.rust]` のみ）
- `tools/lint/Cargo.toml`（`[lints.rust]` のみ）
- `scripts/coverage.sh`

`modules/**/src/` は一切触っていない。`code-generation-plan.md` / `code-generation-questions.md` /
`unit-test-instructions.md` も 1 バイトも変更していない。

## 3. TDD の証跡

### Red（Step 2、変更前ツリー、exit 1）

```
=== CI ガバナンス検査 (/Users/j5ik2o/orca/workspaces/amadeus-ng/docs) ===
[FAIL] toolchain-channel — rust-toolchain.toml が存在しない (channel = "1.95.0" が必要)
[FAIL] toolchain-components — rust-toolchain.toml が存在しない (components: rustfmt clippy llvm-tools が必要)
[FAIL] toolchain-profile — rust-toolchain.toml が存在しない (profile = "minimal" が必要)
[FAIL] workspace-unsafe-forbid — Cargo.toml の [workspace.lints.rust] に unsafe_code = "forbid" が無い
[FAIL] tools-lint-unsafe-forbid — tools/lint/Cargo.toml の [lints.rust] に unsafe_code = "forbid" が無い (detached クレートは workspace lints を継承しない)
[FAIL] ci-merge-group-trigger — on: に merge_group トリガが無い (merge queue のチェックが走らない)
[FAIL] ci-permissions-contents-read — workflow 直下に permissions: contents: read が無い (既定権限のまま)
[FAIL] ci-toolchain-file-driven — toolchain が rust-toolchain.toml 駆動になっていない (@master が無い / @stable が残っている / components: 入力が残っている)
[FAIL] ci-tools-lint-steps — check ジョブに tools/lint の fmt / clippy / test 3 ステップが揃っていない
[FAIL] ci-audit-job — audit ジョブが無い / cargo-audit の導入・2 ロックファイルの監査が揃っていない
[FAIL] ci-proptest-seed-env — PROPTEST_RNG_SEED: "20260823" の指定が 0 箇所 (check / coverage の 2 箇所以上が必要)
[PASS] ci-coverage-base-condition — coverage は pull_request のときだけ --base を使い、それ以外は絶対ゲートのみ (NFR2.2)
[FAIL] coverage-tolerance — TOLERANCE=0.01 でない (シード固定後は 0.01 が要求値)
[FAIL] coverage-ignore-regex — --ignore-filename-regex '^modules/app/aidlc/src/main\.rs$' が無い (composition root の除外が未設定)
[FAIL] coverage-proptest-seed — PROPTEST_RNG_SEED=20260823 の export が無い (PBT のシードが固定されず計測が揺れる)
--- 合計: PASS 1 / FAIL 14 ---
exit=1
```

計画 §4.1 は「9 項目 FAIL」を予期していたが、これは**検査の粒度の違い**であって不一致ではない。
検査を対象ファイルごとに細かく割ったため、計画の 9 グループが 14 項目に展開されている
（toolchain → channel / components / profile の 3、unsafe → workspace / tools-lint の 2、
proptest-seed → ci.yml / coverage.sh の 2、toolchain の CI 側 `ci-toolchain-file-driven` が 1）。
`unit-test-instructions.md` §3 の「合計 15 項目以上」は満たしている（ruleset を含め 16 項目）。

### Green の推移

| 時点 | PASS / FAIL | exit |
|---|---|---|
| Step 2（Red） | 1 / 14 | 1 |
| Step 3 後 | 9 / 6 | 1 |
| Step 4 後 | 10 / 5 | 1 |
| Step 5 後 | 14 / 1 | 1 |
| **Step 6 後（Green）** | **15 / 0** | **0** |
| Step 8（Refactor）後 | 15 / 0 | 0 |

Green（最終、exit 0）:

```
=== CI ガバナンス検査 (/Users/j5ik2o/orca/workspaces/amadeus-ng/docs) ===
[PASS] toolchain-channel — rust-toolchain.toml の channel が "1.95.0" に固定されている
[PASS] toolchain-components — rust-toolchain.toml の components に rustfmt clippy llvm-tools が揃っている
[PASS] toolchain-profile — rust-toolchain.toml の profile が "minimal"
[PASS] workspace-unsafe-forbid — Cargo.toml の [workspace.lints.rust] に unsafe_code = "forbid"
[PASS] tools-lint-unsafe-forbid — tools/lint/Cargo.toml の [lints.rust] に unsafe_code = "forbid"
[PASS] ci-merge-group-trigger — on: に merge_group トリガがある (NFR2.2)
[PASS] ci-permissions-contents-read — workflow 直下に permissions: contents: read がある (NFR4.4)
[PASS] ci-toolchain-file-driven — toolchain が dtolnay/rust-toolchain@master + rust-toolchain.toml 駆動 (NFR4.2)
[PASS] ci-tools-lint-steps — check ジョブに tools/lint の fmt / clippy / test 3 ステップがある (NFR2.3)
[PASS] ci-audit-job — audit ジョブが cargo-audit を導入し 2 つの Cargo.lock を監査する (NFR4.1)
[PASS] ci-proptest-seed-env — check / coverage に PROPTEST_RNG_SEED: "20260823" がある (2 箇所、NFR2.4)
[PASS] ci-coverage-base-condition — coverage は pull_request のときだけ --base を使い、それ以外は絶対ゲートのみ (NFR2.2)
[PASS] coverage-tolerance — TOLERANCE=0.01 に引き締められている (NFR2.4)
[PASS] coverage-ignore-regex — cargo llvm-cov に --ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$' を渡し composition root だけを除外している (NFR2.5)
[PASS] coverage-proptest-seed — PROPTEST_RNG_SEED=20260823 を export して計測を決定化している (NFR2.4)
--- 合計: PASS 15 / FAIL 0 ---
exit=0
```

`--with-ruleset` 付き（ruleset は未変更のため 1 項目だけ FAIL、exit 1 — 想定どおり）:

```
[FAIL] ruleset-required-checks — ruleset「main」(id=21190453) の required checks が期待と違う
       (実際: [なし] strict=false / 期待: [check coverage quint] strict=true)
--- 合計: PASS 15 / FAIL 1 ---
```

### 検査そのものの検出力（テストが常に通るものになっていないことの確認）

Refactor と除外検査の書き換えのあと、**最終版の検査スクリプトを変更前ツリーに当てて**同じ Red を再現した:

- 変更前ツリー（`git archive ad3fc0e` を展開したツリーに最終版スクリプトを配置）→ **PASS 1 / FAIL 14、exit 1**（Red と同一）
- 対象ファイルを 1 つも置かないツリー → **PASS 0 / FAIL 15、exit 1**（ファイル欠損を黙って PASS にしない）
- 未知の引数 → exit 2、`--help` → exit 0

### `tools/lint` 自己テスト

```
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## 4. 受入の実測

### `scripts/coverage.sh` 2 回実行（NFR2.4）— **差 0.00pp は未達**

除外リテラル訂正後の 2 回:

| 実行 | head line coverage | 絶対ゲート |
|---|---|---|
| 1 回目 | 97.09536307961505%（5549/5715 行） | `[PASS]` |
| 2 回目 | 97.11286089238845%（5550/5715 行） | `[PASS]` |
| 差 | **0.0175pp**（1 行分） | — |

原因を実測で特定した（同条件・同シードで 8 回計測し、per-file JSON を突合）:

- 差はちょうど **1 行**。ファイルは `modules/core/interface-adapter/src/workspace/fs_workspace_lock.rs`（442/446 ⇔ 443/446）。
- 行は **237 行目**、`unstamped_is_over_grace` の `Err(_) => false`（stat できない = dir が消えた側の分岐）。
- これを踏ませるのは `modules/core/interface-adapter/tests/fs_workspace_lock_test.rs` の並行テスト
  （4 スレッド × 15 回の実ファイルシステム競合）で、**OS スケジューラ次第**で通ったり通らなかったりする。
- **PBT のシードは無関係**。`PROPTEST_RNG_SEED` が proptest 1.11 に読まれていることは別途実証した
  （不正値 `PROPTEST_RNG_SEED=not-a-number` を渡すと
  `proptest: The env-var PROPTEST_RNG_SEED=not-a-number can't be parsed as u64, using default of random.` が出る）。
  シード固定後も per-file の差はこの 1 行だけで、他の全ファイルは 2 回とも完全一致した。
- 観測分布: 8 回中 5549 行が 4 回、5550 行が 4 回（ほぼ半々）。

つまり **NFR2.4 の合格基準「2 回計測で差 0.00pp」は満たしていない**。さらに残ったジッタ 0.0175pp は
新しい `TOLERANCE=0.01` を上回るため、相対ゲートが偽陽性で赤になりうる。裁定が要る（§9 (a)）。

### 除外の効き（NFR2.5）

- 訂正前（承認済みリテラル `^modules/...`）: `modules/app/aidlc/src/main.rs` が **0/2 行として計測対象に残っていた**（除外が効いていない）。
- 訂正後（`(^|/)modules/...`）: 計測対象ファイル一覧から消え、総行数 5717 → **5715**、カバレッジ 97.0614% → **97.0954%**（+0.034pp）。
- 除外されたのは `main.rs` の 1 ファイルのみ（`modules/app` 配下の他ファイルは 0 件）。

### `cargo audit`

ローカルに未導入だったので `cargo install cargo-audit --locked` で導入した（cargo-audit 0.22.2、exit 0）。

| コマンド | 結果 |
|---|---|
| `cargo audit` | exit 0（advisory 1225 件ロード、74 crate を走査、脆弱性なし） |
| `cargo audit --file tools/lint/Cargo.lock` | exit 0（5 crate を走査、脆弱性なし） |

### `tools/lint` 3 コマンド

| コマンド | 結果 |
|---|---|
| `cargo fmt --manifest-path tools/lint/Cargo.toml --all --check` | exit 0 |
| `cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings` | exit 0 |
| `cargo test --manifest-path tools/lint/Cargo.toml` | exit 0（31 passed） |

### toolchain 固定

`rust-toolchain.toml` を置いた直後に rustup が `1.95.0-aarch64-apple-darwin` へ切り替え、
不足していた `llvm-tools` を自動取得した（`info: downloading component llvm-tools`）。
`rustc --version` = `rustc 1.95.0 (59807616e 2026-04-14)`。

## 5. ruleset スクリプトの `--dry-run` 出力

`bash scripts/governance/ruleset-required-checks.sh --dry-run --out-dir <dir>` → exit 0、**PUT は未実行**。

標準エラーの進行ログ:

```
==> ruleset「main」を amadeus-dlc/amadeus-ng から解決中
==> <dir>/before.json に保存しました
現在の required checks: [なし] strict=false
期待する required checks: [check coverage quint] strict=true
==> --dry-run: PUT は実行しません。組み立てた JSON を印字します
```

組み立てた PUT ペイロード（要約 — 既存 3 規則と `bypass_actors: []` を維持したうえで
`required_status_checks` を追加）:

```json
{
  "name": "main",
  "target": "branch",
  "enforcement": "active",
  "conditions": { "ref_name": { "exclude": [], "include": ["~DEFAULT_BRANCH"] } },
  "bypass_actors": [],
  "rules": [
    { "type": "deletion" },
    { "type": "non_fast_forward" },
    { "type": "merge_queue",
      "parameters": { "merge_method": "SQUASH", "max_entries_to_build": 1,
                      "min_entries_to_merge": 1, "max_entries_to_merge": 1,
                      "min_entries_to_merge_wait_minutes": 0,
                      "grouping_strategy": "ALLGREEN",
                      "check_response_timeout_minutes": 60 } },
    { "type": "required_status_checks",
      "parameters": { "strict_required_status_checks_policy": true,
                      "required_status_checks": [ {"context":"check"}, {"context":"quint"}, {"context":"coverage"} ] } }
  ]
}
```

`GET` レスポンスの `id` / `node_id` / `_links` / `created_at` などは PUT に送れないので、
PUT が受け付けるフィールドだけを選び直している。`rules[]` は全置換されるため既存 3 規則を載せ直す。
冪等判定は「規則 type の有無」ではなく **required コンテキスト集合 + strict フラグの一致**
（nfr-design レビュー Minor 2 の引き取り）。実行後は `after.json` を取り直し、
3 コンテキスト・strict・**既存 3 規則が消えていないこと**を `jq` で検証して終了コードに反映する。

異常系も確認済み: 未知の引数 → exit 2、`--out-dir` の値なし → exit 1、
存在しない ruleset 名 → `エラー: ruleset「does-not-exist」が amadeus-dlc/amadeus-ng に見つかりません` で exit 1。

前後 JSON は記録ディレクトリに保存していない（本委任では `aidlc/` 配下に本レポート以外を書かない方針のため）。
Step 10 でオーナーが `--out-dir <record>/construction/u10-ci-governance/code-generation/ruleset/` を付けて実行すれば保存される。

## 6. 計画からの逸脱・設計判断

**逸脱 1（要オーナー確認）— カバレッジ除外リテラルのアンカー訂正。**
承認済みの `--ignore-filename-regex '^modules/app/aidlc/src/main\.rs$'`
（tech-stack-decisions §1、nfr-design レビュー Minor 1 が正本と裁定したもの）は**不活性**だった。
llvm-cov はカバレッジデータに**絶対パス**を記録するため（実測: `/Users/.../docs/modules/app/aidlc/src/main.rs`）、
`^modules/...` はどのパスにも一致しない。実測でも訂正前は `main.rs` が 0/2 行として計測対象に残っていた。

「相対パス基準」の意図（= リポジトリルート直下の `modules/...` というパス断片に限定し、
別クレートの同名ファイルを巻き込まない）を保ったまま実効化するため、先頭アンカーを
`(^|/)` に変えた: `'(^|/)modules/app/aidlc/src/main\.rs$'`。
相対ゲートの base 側は一時 worktree の**別の絶対パス**で計測されるので、この形でないと head と base で
除外条件がずれるという理由もある。承認済み文書のリテラルからの逸脱なので、正本側の更新はオーナー裁定を待つ。

**逸脱 2（軽微）— 除外検査の書き方。**
`verify-ci-governance.sh` の `coverage-ignore-regex` は当初「`--ignore-filename-regex '<regex>'` という
文字列がそのまま書かれているか」で見ていたが、`scripts/coverage.sh` 側で正規表現を定数
（`IGNORE_FILENAME_REGEX`）に切り出したため、検査を「`--ignore-filename-regex` を `cargo llvm-cov` に
渡していること」と「その正規表現が期待値であること」の 2 事実に分けた。インライン記述でも定数経由でも
成立する。検出力は不変であることを、最終版スクリプトを変更前ツリーに当てて同じ 14 FAIL を再現して確認した。

**逸脱 3（説明のみ）— 検査項目の粒度。**
計画 §4.1 の「9 項目 FAIL」に対し実際は 14 FAIL。項目を細分したためで、対応関係は §3 に記載。
`unit-test-instructions.md` §3 の「合計 15 項目以上」は満たしている。

**実施しなかったこと（担当外・計画どおり）**: ruleset の PUT（Step 10）、Bolt ゲート / PR（Step 11）、
push / ブランチ切替。`shellcheck` はローカル未導入のため未実行（計画上も任意）。

## 7. 品質ゲートの結果

| # | コマンド | exit |
|---|---|---|
| 1 | `cargo fmt --all --check` | 0 |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 3 | `cargo lint` | 0 |
| 4 | `cargo test --workspace` | 0（**338 passed**、0 failed） |
| 5 | `cargo fmt --manifest-path tools/lint/Cargo.toml --all --check` | 0 |
| 6 | `cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings` | 0 |
| 7 | `cargo test --manifest-path tools/lint/Cargo.toml` | 0（**31 passed**） |
| 8 | `bash scripts/governance/verify-ci-governance.sh` | 0（15 PASS / 0 FAIL） |
| 9 | `bash -n` × 3 本（coverage / verify / ruleset） | 0 |
| 10 | `bash scripts/coverage.sh`（絶対ゲート） | 0（`[PASS] absolute gate` 97.10% >= 90.0%） |

`unsafe_code = "forbid"` の昇格でワークスペースに赤は出なかった（unsafe 使用ゼロ、`tools/lint` も含む）。
`Cargo.lock` / `tools/lint/Cargo.lock` は不変（依存追加なし）。

## 8. コミット一覧

```
ba75234 fix(coverage): anchor the exclusion regex so it actually matches
43e1dd9 refactor(governance): unify the missing-file handling in the verifier
7af3194 chore(governance): add an idempotent script for ruleset required status checks
f7b8e3e chore(ci): add the merge_group trigger for merge queue checks
3dc1a3f chore(coverage): make measurement deterministic and exclude the composition root
cceb1bc chore(ci): run fmt, clippy, and self-tests for the detached tools/lint crate
7702372 chore(ci): add least-privilege permissions, toolchain-file setup, and audit job
225c4c6 chore(toolchain): pin Rust to 1.95.0 and forbid unsafe_code workspace-wide
ad3fc0e test(governance): add CI governance configuration verifier
```

（`055195b` は本委任の前からブランチにあった記録コミット。作業ツリーの未追跡・変更ファイルは
`developer-brief-3.md`・監査シャード・本レポートのみで、いずれもコミットしていない。）

## 9. 未解決・要確認事項

**(a) NFR2.4「2 回計測で差 0.00pp」が未達 — `TOLERANCE` の扱い（最優先、オーナー裁定が要る）。**
シード固定後も ±1 行（**0.0175pp**）のジッタが残り、これは新しい `TOLERANCE=0.01` を上回るため、
相対ゲートが偽陽性で赤になりうる。原因は PBT ではなく
`modules/core/interface-adapter/src/workspace/fs_workspace_lock.rs:237`（`unstamped_is_over_grace` の
`Err(_)` 分岐）を並行テストが踏むかどうかがスケジューラ依存であること。承認済みの 0.01 をそのまま
実装してあるので、変えるなら裁定が要る。選択肢:

- **A（推奨）**: `TOLERANCE=0.02` にする。実測ジッタ 0.0175pp を包む最小の値で、従来の 0.5 に比べれば 25 倍厳格。
  team.md / tech-stack-decisions の「0.01 へ引き締める」を実測に合わせて 0.02 へ改める必要がある。
- **B**: `fs_workspace_lock.rs:237` の `Err` 分岐を決定的に覆う単体テストを足して 0.01 のまま維持する。
  `modules/core/interface-adapter` のテストを触るので U10 の境界外（U7 か後続 Unit のスコープ）。
- **C**: 0.01 のまま運用し、稀な偽陽性は再実行でしのぐ。

**(b) カバレッジ除外リテラルの正本更新。** §6 逸脱 1 の訂正（`(^|/)` アンカー）を
`tech-stack-decisions.md` §1 と `security-design.md` §4 に反映するかどうか。実装は訂正版で動いている。

**(c) `dtolnay/rust-toolchain@master` の CI 実挙動。** ローカルでは rustup が `rust-toolchain.toml` の
`components` を読んで `llvm-tools` を自動取得することを確認したが、GitHub Actions 上で
`components:` 入力なしの `@master` が同じ 3 コンポーネントを入れるかは PR の初回 CI で要確認。
coverage ジョブが `llvm-tools` 不足で赤になったら、`ci.yml` 側に `components:` を戻すのではなく
`rust-toolchain.toml` の記述で解決する（正本を 1 つに保つため）。

**(d) `audit` ジョブの required 化。** 設計どおり今回は required 外。運用 1 週間後に再判断（nfr-design §2）。

**(e) `merge_group` と `strict_required_status_checks_policy: true` の相互作用。**
strict（ブランチ最新性要求）と merge queue の ALLGREEN グルーピングを重ねると
キュー投入がブロックされる事象が知られている。NFR2.1 の正常系受入（緑の PR が queue を通って
squash-merge まで完走）は本 Bolt の PR 自身で実地確認が要る（Step 10 / 11）。

**(f) ruleset の PUT は未実行。** `--dry-run` までしか行っていないので、`verify-ci-governance.sh --with-ruleset`
は現時点で FAIL のまま。Step 10 でオーナーが
`bash scripts/governance/ruleset-required-checks.sh --out-dir <record>/construction/u10-ci-governance/code-generation/ruleset/`
を実行し、その後 `--with-ruleset` の PASS を記録すること。誤設定時は `before.json` を PUT し直せば復元できる。
