# nfr-design-questions — U10 CI ガバナンス（`u10-ci-governance`）

> NFR Design（Construction 3.3）の質問票（Unit: U10、kind: packaging）。出典: `../nfr-requirements/security-requirements.md`
> （NFR2.1〜2.5 / NFR4.1〜4.5、レビュー Minor 3 件）、`../nfr-requirements/tech-stack-decisions.md`（選定と未決）、
> `../../../inception/contract-design/contract-summary.md`（U10 は契約面を持たない）。performance / scalability /
> reliability / observability の設計は packaging には存在せず、本ステージの成果物は `security-design.md` /
> `traceability.json` の 2 つ（エンジンの produces どおり。論理コンポーネントは security-design 内に節として置く）。
>
> **質問なし。** 設計パターンの選択余地（耐障害・スケール・キャッシュ・観測）は CI・ガバナンス設定には無く、
> 設計は NFR 要求・技術選定から一意に決まる。次の前提を確認して成果物へ進む。

## 前提（確認事項）

- P1. **CI ワークフローの形**: `ci.yml` 1 本、トリガ `pull_request`（main）+ `merge_group` + `workflow_dispatch`、
  workflow 直下に `permissions: contents: read`、`concurrency` は `github.ref` 単位のまま。ジョブは 4 つ —
  `check`（fmt / clippy / cargo lint / test + `tools/lint` の fmt / clippy / test）、`quint`、`coverage`（PR 時は
  `--base origin/<base_ref>` の相対ゲート、`merge_group` / `workflow_dispatch` 時は絶対ゲートのみ）、`audit`（新規:
  `cargo audit` を workspace と `tools/lint` の 2 ロックファイルに）。toolchain は `rust-toolchain.toml`（1.95.0）を
  `dtolnay/rust-toolchain@master` が読む。required checks のコンテキスト名は既存の `check` / `quint` / `coverage`
  （`audit` は required に含めない — advisory DB の一時障害で全マージが止まるのを避ける。含めるかは運用 1 週間後に再判断）。
- P2. **ruleset 変更の手順**: `scripts/governance/ruleset-required-checks.sh`（bash + `gh api`）— 変更前の ruleset JSON を
  取得して記録 → `rules[]` に `required_status_checks`（`check` / `quint` / `coverage`、`strict_required_status_checks_policy:
  true`）を**追加**（既存の deletion / non_fast_forward / merge_queue は維持）→ 変更後 JSON を取得して記録。冪等（既に
  存在すれば何もしない）。実行はオーナー権限。正常系の受入 = 緑の PR 1 本が merge queue を通って squash-merge まで完走
  （レビュー Minor 3 の引き取り）。
- P3. **カバレッジの決定化と除外**: `scripts/coverage.sh` に `--ignore-filename-regex` で `modules/app/aidlc/src/main.rs`
  のみ除外、`TOLERANCE=0.01`。PBT シード固定は proptest の API（`PROPTEST_RNG_SEED` 環境変数 または `TestRunner` ヘルパ）を
  code-generation で確認して決め、受入は「2 回計測で差 0.00pp」。
- P4. **障害ドメイン**: CI 設定の誤りは当該 PR の赤に閉じる（他へ波及しない）。ruleset の誤設定は**全マージを止める**
  （ブラストラディウス最大）— 手順スクリプトの前後 JSON と正常系 PR での確認で抑える。`audit` の外部依存障害は
  required に含めないことで隔離。Dependabot と GitHub Actions の SHA ピン留めは本 intent では見送り（後続 intent、
  レビュー Minor 1 の引き取り）。

## Consolidated Summary Confirmation

- U10 に固有の NFR 設計質問はなし（packaging — 耐障害 / スケール / キャッシュ / 観測のパターンは不要）
- CI ワークフロー（P1）: 4 ジョブ、`merge_group` トリガ追加、`permissions: contents: read`、toolchain ファイル駆動、
  required checks は `check` / `quint` / `coverage` の 3 つ（`audit` は required に含めない）
- ruleset 変更（P2）: 前後 JSON を記録する冪等スクリプトをオーナー権限で実行、正常系 PR の完走を受入に追加
- カバレッジ（P3）: `main.rs` のみ除外、`TOLERANCE=0.01`、PBT シード固定の手段は code-generation で確定
- 障害ドメイン（P4）: PR 単位に閉じる / ruleset 誤設定は前後 JSON + 正常系確認で抑える / Dependabot・SHA ピン留めは見送り

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
