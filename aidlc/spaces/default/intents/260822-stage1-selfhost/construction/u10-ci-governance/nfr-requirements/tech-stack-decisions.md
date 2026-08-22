# tech-stack-decisions — U10 CI ガバナンス（`u10-ci-governance`）

> NFR Requirements（Construction 3.2）成果物（Unit: U10、kind: packaging）。出典: `security-requirements.md`
> （NFR2.1〜2.5 / NFR4.1〜4.5）、`../../../inception/requirements-analysis/requirements.md`（FR9.1〜9.5、制約 C1〜C2）、
> `../../../inception/contract-design/contract-summary.md`（U10 は契約面を持たない）、`aidlc/spaces/default/codekb/docs/
> technology-stack.md`（GitHub Actions 1 本 3 ジョブ、`dtolnay/rust-toolchain@stable`、`Swatinem/rust-cache@v2`、
> `taiki-e/install-action@v2` で cargo-llvm-cov、Node 22 + quint 0.32.0）、`aidlc/spaces/default/memory/team.md`
> （Code Style: サプライチェーン/ハードニングの採用事項）、確認事項 `nfr-requirements-questions.md`（前提 P1〜P8）。

## 1. 選定

| 領域 | 選定 | 理由 | 代替案（不採用の理由） |
|---|---|---|---|
| マージの機械強制 | 既存 **ruleset「main」** に `required_status_checks`（`check` / `quint` / `coverage`、`strict_required_status_checks_policy: true`）を追加。`bypass_actors` は空のまま | ruleset は既に active で merge queue（SQUASH / ALLGREEN）を持つ。classic branch protection を併設すると二重管理になる（前提 P1） | classic branch protection: ruleset と重複し、merge queue 設定と分裂する |
| merge queue との整合 | `ci.yml` に `on: merge_group:` を追加。`concurrency.group` はイベントに依らず `github.ref` で分離。coverage ジョブは `pull_request` 時のみ `--base origin/<base_ref>`、`merge_group` 時は絶対ゲートのみ | merge queue は `merge_group` イベントのチェックを要求する（前提 P2）。相対ゲートは base ref を要する | queue を無効化: 既存のオーナー設定（SQUASH / ALLGREEN）を壊す |
| ツールチェーン固定 | `rust-toolchain.toml`: `channel = "1.95.0"`、`components = ["rustfmt", "clippy", "llvm-tools"]`、`profile = "minimal"`。CI は `dtolnay/rust-toolchain@master`（toolchain ファイルを読む形）に変更し `components:` 指定を撤去 | ローカル実測 rustc 1.95.0。floating stable による突然赤の解消（インタビュー Q6 選択肢 B）。`llvm-tools` は coverage ジョブの要件 | `channel = "stable"`: 固定にならない。`@stable` アクションのまま: ファイルを無視して最新 stable を入れる |
| 脆弱性監査 | 新規 `audit` ジョブ: `taiki-e/install-action@v2`（`tool: cargo-audit`）→ `cargo audit` を リポジトリルートと `tools/lint/` で各 1 回（`--file tools/lint/Cargo.lock`） | 既存の cargo-llvm-cov 導入と同じ流儀で依存が増えない。2 つの `Cargo.lock` を明示的に網羅（前提 P4） | `rustsec/audit-check` アクション: 1 ロックファイル前提で 2 回の設定が冗長。`cargo deny`: ライセンス検査まで含む広い道具 — 本 intent のスコープ外 |
| `unsafe_code` | `[workspace.lints.rust] unsafe_code = "forbid"`（`Cargo.toml`）。`tools/lint/Cargo.toml` の `[lints.rust]` にも `unsafe_code = "forbid"` | 全メンバーへ一括適用（`main.rs` の漏れを構造的に塞ぐ — インタビュー Q6 選択肢 C）。detached クレートは継承しない | クレート個別 attribute のまま: 漏れが再発する |
| CI 権限 | `ci.yml` 直下に `permissions: contents: read` | least privilege（インタビュー Q6 選択肢 D）。3 ジョブ + audit は読取のみで足りる | ジョブ個別指定: 既定権限の範囲が残る |
| `tools/lint` の CI | `check` ジョブに 3 ステップ追加: `cargo fmt --manifest-path tools/lint/Cargo.toml --all --check` / `cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings` / `cargo test --manifest-path tools/lint/Cargo.toml` | detached クレートのため `--workspace` に載らない（設計監査 C27）。`Swatinem/rust-cache` は既に `tools/lint -> target` をキャッシュ対象に含む | workspace メンバー化: 「coverage 対象に載せない」という tools/lint の設計意図に反する |
| カバレッジ除外 | `scripts/coverage.sh` の `cargo llvm-cov` 呼出に `--ignore-filename-regex '^modules/app/aidlc/src/main\.rs$'`（相対パス基準）を追加 | composition root の配線部のみ除外（インタビュー Q5 選択肢 B）。ファイル単位で最小 | クレート単位除外（`--exclude aidlc`）: 配線以外のコードまで除外してしまう |
| PBT シード固定 | proptest の `ProptestConfig` に決定的 RNG（`rng_algorithm: RngAlgorithm::ChaCha` + 固定シード、または環境変数 `PROPTEST_RNG_SEED` の利用可否を code-generation で確認）を workspace 共通のヘルパで与え、`TOLERANCE=0.01` | 同一コードで計測が揺れる原因が PBT のランダム経路（実測 ±0.4pp）。決定化すれば相対ゲートを締められる（インタビュー Q7） | `PROPTEST_CASES` を減らす: 検査力が落ちる。`TOLERANCE` 据え置き: 実際の減衰を見逃す |

## 2. 依存の差分（予定）

| 種別 | 追加・変更 | 備考 |
|---|---|---|
| GitHub Actions | `taiki-e/install-action@v2`（cargo-audit — 既存利用のアクション）、`dtolnay/rust-toolchain@master`（既存 `@stable` から変更） | 新しいアクション提供者は増えない。SHA ピン留めは見送り（practices-discovery の裁定） |
| ファイル | `rust-toolchain.toml`（新規）、`Cargo.toml`（workspace lints）、`tools/lint/Cargo.toml`（lints）、`.github/workflows/ci.yml`、`scripts/coverage.sh`、`scripts/governance/ruleset-required-checks.sh`（新規 — `gh api` 手順） | プロダクトコードの変更なし（PBT シード固定のテスト側ヘルパを除く） |
| Rust クレート | なし（proptest は既存） | `Cargo.lock` 不変が期待値 |
| GitHub 設定 | ruleset「main」へ `required_status_checks` 追加（オーナー実行） | 変更前後の JSON を記録 |

## 3. 未決（後続で確定）

- PBT シード固定の具体手段（`ProptestConfig { rng_algorithm, .. }` + 固定シードのヘルパ関数 vs 環境変数）— code-generation
  で proptest 1.11 の API を確認して決める。受入は「2 回計測で差 0.00pp」。
- `merge_group` イベント時の coverage 相対ゲート: base を `github.event.merge_group.base_sha` で取る案があるが、
  まずは絶対ゲートのみ（PR 時に相対ゲートは済んでいる）。
- ruleset 変更の手順スクリプトの置き場（`scripts/governance/`）と、実行結果 JSON の保存先（記録ディレクトリ）。
