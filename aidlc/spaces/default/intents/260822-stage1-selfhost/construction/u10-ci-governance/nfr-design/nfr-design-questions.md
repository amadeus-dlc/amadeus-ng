# nfr-design-questions — U10 CI ガバナンス（`u10-ci-governance`）

> NFR Design（Construction 3.3）の質問票（Unit: U10、kind: packaging）。出典: `../nfr-requirements/security-requirements.md`
> （NFR2.1〜2.5 / NFR4.1〜4.5、レビュー Minor 3 件）、`../nfr-requirements/tech-stack-decisions.md`（選定と未決）、
> `../../../inception/contract-design/contract-summary.md`（U10 は契約面を持たない）。performance / scalability /
> reliability / observability の設計は packaging には存在せず、本ステージの成果物は `security-design.md` /
> `traceability.json` の 2 つ（エンジンの produces どおり。論理コンポーネントは security-design 内に節として置く）。
>
> **質問なし。** 設計パターンの選択余地（耐障害・スケール・キャッシュ・観測）は CI・ガバナンス設定には無く、
> 設計は NFR 要求・技術選定から一意に決まる。次の前提を確認して成果物へ進む。

## 以前の前提（2026-08-23の記録）

P1〜P4は当時の記録として保持する。今回の設計更新には末尾の2026-09-06確認要約を用い、旧回答を新しい確認として流用しない。

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

## 以前に確認済みのまとめ

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

## Consolidated Summary Confirmation

2026-09-06、更新済みのCI・品質管理要件をsecurity-design.mdとtraceability.jsonの2成果物へ具体化する。GitHub設定や実装を変更する作業ではなく、現行の構成・失敗時の扱い・検証方法を設計書へ反映する。

- CIの構成をcheck・quint・coverage・audit・aidlc-distribution・review-thread-resolution・ci-successに合わせる。CI Successは基本3検査と配布検証をsuccess必須とし、レビュー検査は変更提案時にsuccess、merge_group/workflow_dispatch時にskippedを受理する。レビュー結果を再評価する別ワークフローも含め、状態の更新経路を記載する。
- ruleset「main」は必須4コンテキスト（check/quint/coverage/CI Success）、strict有効、bypassなし、SQUASH・ALLGREEN・同時1件を維持する設計にする。管理手順は取得・比較・必要時のみ変更・再取得検証と前後JSON保存に分ける。設定が存在することと、全成功時の完走・失敗時の停止の実働証拠を区別する。
- 権限はworkflow既定contents: readとレビュー検査の個別書込権限を区別する。SHA固定の外部再利用ワークフロー、トークンの秘密情報としての扱い、外部配布元との境界を記載する。全ActionがSHA固定済み、全ジョブが読取専用、秘密情報なしとは説明しない。
- Rust 1.95.0と構成要素はrust-toolchain.tomlを正本としてCIへ導出し、unsafe forbidはworkspaceの継承とtools/lintの個別宣言で適用する。カバレッジは90%床・相対許容0.01ポイント・シード20260823・main.rsのみ除外を維持し、2回測定の差は実測して判定する。
- 障害の影響範囲を、個別実行の失敗、共有されたCI設定・配布元の障害、全マージへ影響するruleset誤設定に分ける。auditは必須外という既存裁定を維持し、2つのCargo.lockの実行結果・未実行・取得失敗を区別する。Dependabotと全Action一括SHA固定の見送りも記録する。新規クラウド資源やAWS Bedrockは導入しない。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
