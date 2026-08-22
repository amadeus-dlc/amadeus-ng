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
