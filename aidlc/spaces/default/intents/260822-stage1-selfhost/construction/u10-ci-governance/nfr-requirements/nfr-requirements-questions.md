# nfr-requirements-questions — U10 CI ガバナンス（`u10-ci-governance`）

> NFR Requirements（Construction 3.2）の質問票（Unit: U10、kind: packaging）。出典:
> `../../../inception/requirements-analysis/requirements.md`（FR9.1〜9.5、NFR2 品質ゲート、NFR4 セキュリティ /
> サプライチェーン）、`../../../inception/units-generation/unit-of-work.md`（U10 の責務・境界・合格）、
> `../../../inception/contract-design/contract-summary.md`（外部契約は C1 / C2 のみ — U10 は契約面を持たない）、
> `../../../inception/practices-discovery/evidence.md`（確定アクション 1〜4）、`aidlc/spaces/default/codekb/docs/
> technology-stack.md`（Rust / cargo-llvm-cov / Quint / GitHub Actions）、`aidlc/spaces/default/memory/team.md`
> （Testing Posture / Code Style）、実地確認（`.github/workflows/ci.yml`、`scripts/coverage.sh`、`tools/lint/`、
> `gh api` の ruleset、ローカルのツールチェーン）。
>
> performance / scalability / reliability / observability の要求は packaging（CI・ガバナンス設定）には存在しない。
> 本ステージの成果物は `security-requirements.md`（NFR4.x + NFR2.x の品質ゲート要求）/ `tech-stack-decisions.md` /
> `traceability.json` の 3 つ。
>
> **ブロッキングの質問なし。** 要求値は practices-discovery のインタビュー裁定（Q4〜Q8）と requirements.md FR9 で
> 確定済み。実地確認で分かった 2 点（前提 P1・P2）を含む前提を確認して成果物へ進む。

## 以前の前提（2026-08-23の記録）

以下のP1〜P8は当時の記録。現在の設定と異なる点は末尾の2026-09-06確認要約で更新し、以前の回答を今回の確認に流用しない。

- P1. **branch protection の実態**（実地 `gh api`、2026-08-23）: `main` には classic branch protection は無い（404）が、
  **ruleset「main」（active）** が既に存在し、`deletion` / `non_fast_forward` / `merge_queue`（SQUASH、ALLGREEN、
  同時 1 件）を含む。**required status checks の規則は無い**。FR9.1 は classic protection を新設するのではなく、
  **この ruleset に `required_status_checks`（check / quint / coverage、strict）を追加する**形で満たす。
- P2. **merge queue と CI トリガの不整合**: `ci.yml` は `pull_request`（+ `workflow_dispatch`）でしか走らない。merge queue
  は `merge_group` イベントでチェックを要求するため、required checks を足すと queue のチェックが永久に走らず詰まる
  （逆に現状は required checks が無いので ALLGREEN でも即マージされる）。FR9.1 と同じ Bolt で `on: merge_group:` を
  `ci.yml` に追加し、coverage ジョブの相対ゲートは `merge_group` では base ref が無いため絶対ゲートのみ（または
  `github.event.merge_group.base_ref`）にする。
- P3. **ツールチェーン固定**: `rust-toolchain.toml` に `channel = "1.95.0"`（ローカル実測 rustc 1.95.0）、
  `components = ["rustfmt", "clippy", "llvm-tools"]`、`profile = "minimal"`。CI の `dtolnay/rust-toolchain@stable`
  は toolchain ファイルを尊重する形（`@master` + ファイル参照、または同アクションの `toolchain:` 省略）に改める。
  バージョン更新は PR で行う。
- P4. **cargo audit**: CI に `audit` ジョブを追加（`taiki-e/install-action` で `cargo-audit` 導入 — 既存の
  cargo-llvm-cov 導入と同じ流儀）。対象は `Cargo.lock`（workspace、74 パッケージ）と `tools/lint/Cargo.lock`
  （5 パッケージ）の 2 つ。advisory DB 取得失敗は再実行（外部依存マップ）。新規 advisory で赤になったら依存更新 PR。
- P5. **`unsafe_code = "forbid"` の workspace lints 昇格**と `permissions: contents: read` の明示（workflow 直下）。
  `tools/lint` は detached クレートなので `[lints.rust] unsafe_code = "forbid"` を個別に書く。
- P6. **`tools/lint` の CI 3 ステップ**（`cargo fmt --manifest-path tools/lint/Cargo.toml --check` / `cargo clippy
  --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings` / `cargo test --manifest-path tools/lint/Cargo.toml`）
  を `check` ジョブに追加。`tools/lint` には 31 本の赤例テスト（インライン）がある。
- P7. **カバレッジ**: `scripts/coverage.sh` に composition root（`modules/app/aidlc/src/main.rs`）の除外
  （`--ignore-filename-regex`）を追加。PBT のシード固定（proptest を使う core-domain 10 箇所 + canon-json）で計測を
  決定化し、`TOLERANCE` を 0.5 → 0.01 へ引き締める。固定の手段（`ProptestConfig` の `rng_seed` / `RngAlgorithm` 等）は
  code-generation で確定し、決定化の実証（2 回計測の差 = 0.00）を受入とする。
- P8. **境界**: プロダクトコードは触らない（`unsafe_code` 昇格で赤になるクレートがあれば U7 で直す）。GitHub 設定
  （ruleset 変更）はオーナー権限が要る — `gh api` の手順をスクリプト化してオーナーが実行、結果を `gh api` で確認する。

## 以前に確認済みのまとめ

- U10 に NFR の質問はなし（packaging — 性能 / スケール / 信頼性 / 観測の要求は存在しない）。要求値は FR9 と
  practices-discovery の裁定で確定済み
- 実地確認の発見（P1・P2）: `main` には既に ruleset（merge queue SQUASH / ALLGREEN）があり required checks だけが無い →
  ruleset に required_status_checks を追加し、同時に `ci.yml` へ `merge_group` トリガを足す（足さないと queue が詰まる）
- ツールチェーン 1.95.0 固定（P3）、cargo audit を 2 つの Cargo.lock に（P4）、unsafe forbid 昇格 + permissions（P5）、
  tools/lint の CI 3 ステップ（P6）、カバレッジ除外 + PBT シード固定 + 許容 0.01（P7）、境界（P8）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

## Consolidated Summary Confirmation

2026-09-06、CI・品質管理の要件3成果物を現行設定と承認済み方針へ整合させる。今回の要件整理ではGitHub設定や品質閾値を変更しない。

- mainのrulesetは既にactiveで、必須チェックはcheck・quint・coverage・CI Successの4つ、strict有効、bypassなし。マージキューはSQUASH・ALLGREEN・同時1件。取得結果は `../ruleset-observed-20260906.json` に保存した。合格条件には失敗時にマージを止める経路と、全緑時にキューを完走する経路の両方を記載する。
- CI Successは基本の3チェックに加え、配布物の同期・回帰試験と、変更提案時の未解決レビュースレッド検査を集約する。merge_groupではレビュースレッド検査のskippedを許容する。auditジョブは既存裁定どおり別のadvisoryとして実行し、未実行・失敗を成功に読み替えない。
- workflow既定はcontents: read。review-thread-resolutionだけにchecks/statusesのwriteとissues/pull-requestsのreadを付与している事実、SHA固定の外部再利用ワークフローとの信頼境界を記載する。トークンは秘密情報であり、ログへ出さない。「全ジョブ読取専用」「秘密情報なし」という過大な説明を修正する。
- カバレッジ90%床・相対差0.01ポイント・固定シード20260823・main.rsだけの除外を維持する。暫定0.05は過去の裁定として区別する。再現性の受入は実測結果で判断し、今回まだ再計測していないものを達成済みと主張しない。
- Rust 1.95.0固定、workspaceとtools/lint双方の品質・依存検査を現行設定として記載する。測定可能な合格条件と、更新を変更提案経由で行う運用規範を分ける。ActionのSHA固定は実際の適用範囲を示し、全件固定とは主張しない。Dependabot導入見送りは既存裁定として明記する。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
