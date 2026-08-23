# security-design — U10 CI ガバナンス（`u10-ci-governance`）

> NFR Design（Construction 3.3）成果物（Unit: U10、kind: packaging）。出典: `../nfr-requirements/security-requirements.md`
> （NFR2.1〜2.5 品質ゲートの機械強制、NFR4.1〜4.5 サプライチェーン / 最小権限、STRIDE、レビュー Minor 3 件）、
> `../nfr-requirements/tech-stack-decisions.md`（ruleset への required checks 追加、`merge_group` トリガ、toolchain 1.95.0、
> `cargo audit` ×2、`unsafe_code` forbid 昇格、`permissions`、`tools/lint` CI、カバレッジ除外、PBT シード固定）、
> `../../../inception/contract-design/contract-summary.md`（U10 は契約面を持たない — C1 / C2 に影響なし）、確認事項
> `nfr-design-questions.md`（前提 P1〜P4、Looks correct）。
>
> packaging Unit のため、設計 = CI ワークフロー・スクリプト・GitHub 設定の**形**と**障害ドメイン**の確定。
> 設計ステージの制約に従い、設定の例示は ≤15 行の断片のみ（完全なファイルは code-generation で書く）。
> logical-components は本 Unit の produces に無いため、論理コンポーネントは §5 に節として置く。

## 1. 設計方針

ガバナンスは **(a) 機械強制を GitHub 側（ruleset）と CI 側（ワークフロー）の 2 層で重ねる**、**(b) 変更は冪等な
スクリプトで行い前後を記録する**、**(c) 外部依存の一時障害をマージ停止の原因にしない**、の 3 点で設計する。
プロダクトコードは触らない（NFR 要求 §1 の境界）。

## 2. CI ワークフロー（NFR2.2 / NFR2.3 / NFR2.5 / NFR4.1 / NFR4.2 / NFR4.4）

- **トリガ**: `pull_request`（`branches: [main]`）+ `merge_group` + `workflow_dispatch`。merge queue は `merge_group` イベントで
  required checks を要求するため必須（NFR2.2）。`concurrency.group` は `ci-${{ github.workflow }}-${{ github.ref }}` のまま
  （`merge_group` の ref は `gh-readonly-queue/...` で PR の ref と衝突しない）。
- **権限**: workflow 直下に `permissions: contents: read`（NFR4.4）。ジョブ個別の昇格なし。
- **toolchain**: `dtolnay/rust-toolchain@master` に `rust-toolchain.toml`（`channel = "1.95.0"`、`components = ["rustfmt", "clippy", "llvm-tools"]`、
  `profile = "minimal"`）から**導出した** `toolchain` / `components` 入力を渡す（NFR4.2。**実装時の実測**: `@master` は `toolchain:` 入力必須で
  ファイルを自動では読まないため、`scripts/governance/toolchain-inputs.sh` が導出する — 正本は 1 つ）。ci.yml にバージョン・コンポーネントのリテラルは書かない。
- **ジョブ**（required checks のコンテキスト名は既存の 3 つを維持）:

| ジョブ | ステップ | required | 備考 |
|---|---|---|---|
| `check` | `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo lint` → `cargo test --workspace` → **`cargo fmt --manifest-path tools/lint/Cargo.toml --all --check`** → **`cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings`** → **`cargo test --manifest-path tools/lint/Cargo.toml`** | はい | NFR2.3（太字が追加分）。`Swatinem/rust-cache` は既に `tools/lint -> target` を含む |
| `quint` | `scripts/quint-gate.sh`（Node 22 + quint 0.32.0） | はい | 変更なし |
| `coverage` | `pull_request`: `scripts/coverage.sh --base "origin/${{ github.base_ref }}"`。`merge_group` / `workflow_dispatch`: `scripts/coverage.sh`（絶対のみ） | はい | NFR2.4 / 2.5。`merge_group` は base ref を持たないため絶対ゲートのみ（PR 時に相対は済む） |
| `audit` | `taiki-e/install-action@v2`（`tool: cargo-audit`）→ `cargo audit` → `cargo audit --file tools/lint/Cargo.lock` | **いいえ** | NFR4.1。advisory DB / ネットワークの一時障害で全マージが止まらないよう required 外（前提 P1）。赤は PR で可視、依存更新 PR で対応。運用 1 週間後に required 化を再判断 |

```yaml
# ci.yml の形（例示、抜粋）
on:
  pull_request: { branches: [main] }
  merge_group: {}
  workflow_dispatch: {}
permissions:
  contents: read
```

## 3. ruleset の変更（NFR2.1 / NFR4.5）

- 対象: 既存 ruleset「main」（`deletion` / `non_fast_forward` / `merge_queue` SQUASH ALLGREEN）。classic branch protection は使わない。
- 追加する規則: `required_status_checks` — `required_status_checks: [{context: "check"}, {context: "quint"}, {context: "coverage"}]`、
  `strict_required_status_checks_policy: true`。既存規則と `bypass_actors: []` は維持。
- 手順スクリプト `scripts/governance/ruleset-required-checks.sh`（bash + `gh api`、オーナー権限で実行）:
  1. `GET /repos/{owner}/{repo}/rulesets/{id}` → `before.json` として記録ディレクトリ（`<record>/construction/u10-ci-governance/
     code-generation/ruleset/`）に保存。
  2. `rules[]` の `required_status_checks` が期待（3 コンテキスト + strict）と**一致していれば何もしない**（冪等判定は規則タイプの有無ではなく
     コンテキスト集合 + strict フラグの一致 — レビュー Minor 2 の引き取り）。無い / ずれていれば追加・補正した JSON を組み立て
     `PUT /repos/{owner}/{repo}/rulesets/{id}`（既存 3 規則と `bypass_actors` を載せ直す — PUT は `rules[]` を全置換）。
  3. `GET` し直して `after.json` を保存し、3 コンテキストの存在を `jq` で検証して終了コードに反映。
- 受入（正常系 — NFR 要求レビュー Minor 3 の引き取り）: required checks 追加後、緑の PR 1 本（本 Bolt の PR 自身で可）が
  merge queue を通って squash-merge まで完走すること。否定系: 赤のまま queue に入れた PR がマージされないこと（実地 1 回）。

```text
# 追加する規則（例示）
{ "type": "required_status_checks",
  "parameters": { "strict_required_status_checks_policy": true,
    "required_status_checks": [ {"context":"check"}, {"context":"quint"}, {"context":"coverage"} ] } }
```

## 4. ワークスペース設定（NFR4.2 / NFR4.3 / NFR2.4 / NFR2.5）

- `rust-toolchain.toml`（新規）: `[toolchain] channel = "1.95.0"`、`components = ["rustfmt", "clippy", "llvm-tools"]`、`profile = "minimal"`。
- `Cargo.toml`: `[workspace.lints.rust]` に `unsafe_code = "forbid"` を追加。クレート個別の `#![forbid(unsafe_code)]` は残してよい
  （重複は無害）。`tools/lint/Cargo.toml` の `[lints.rust]` に `unsafe_code = "forbid"`。
- `scripts/coverage.sh`: `measure_line_coverage` の `cargo llvm-cov --workspace ...` に `--ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$'`
  を追加（ファイル 1 本に限定、NFR2.5。**実装時の実測**: llvm-cov はカバレッジデータに絶対パスを記録するため tech-stack-decisions §1 の
  `^modules/...` 単独アンカーは不活性 — `(^|/)` で実効化、2026-08-22 UTC）。`TOLERANCE` は承認値 0.01 → 実装時の残ジッタ 0.0175pp により
  **暫定 0.05**（Bolt B2 ゲート裁定、U3 ロック退役後に 0.01）。コメントの較正根拠を更新。
- PBT シード固定（NFR2.4）: **(a) 環境変数 `PROPTEST_RNG_SEED` を採用**（proptest 1.11 の `RngSeed::Fixed`、テストコード変更なし — code-generation で確定）。検討した候補は (a) 環境変数 `PROPTEST_RNG_SEED`
  （`scripts/coverage.sh` と CI で固定値を与える。proptest 1.11 で対応していれば最小変更）、(b) テスト側ヘルパで
  `TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &SEED))` を用いる（テストコードの変更が要る —
  境界「プロダクトコードは触らない」には抵触しないが core-domain のテストを触る）。(a) を第一候補とする。
  受入は 2 回計測の差 0.00pp。

## 5. 論理コンポーネントと障害ドメイン（前提 P4）

| コンポーネント | 置き場 | 障害 | 影響範囲（ブラストラディウス） | 手当て |
|---|---|---|---|---|
| CI ワークフロー | `.github/workflows/ci.yml` | 設定誤り・ステップ赤 | 当該 PR のみ（マージ不可） | PR 内で修正。`workflow_dispatch` で手動再実行可 |
| `audit` ジョブ | 同上 | advisory DB / ネットワーク障害 | 当該 PR の `audit` 赤のみ（required 外なのでマージは止まらない） | 再実行。真の advisory は依存更新 PR |
| ruleset「main」 | GitHub 設定 | 誤設定（コンテキスト名の綴り違い等） | **全 PR のマージ停止** | 手順スクリプトの前後 JSON、`jq` 検証、正常系 PR の完走確認。誤設定時は `before.json` から復元（`PUT`） |
| toolchain 固定 | `rust-toolchain.toml` | 指定版の取得失敗 / ローカルとの不一致 | 全ジョブ赤 | `rustup` が自動取得。更新は PR でのみ |
| workspace lints | `Cargo.toml` / `tools/lint/Cargo.toml` | `unsafe_code` forbid で既存コードが赤 | 全ジョブ赤 | 現状 unsafe 使用ゼロ（practices-discovery 実測）。赤なら U7 で修正（境界） |
| カバレッジゲート | `scripts/coverage.sh` | 除外 regex の誤り / 決定化されていない揺れ | `coverage` ジョブ赤 | 除外はファイル 1 本、2 回計測で差 0.00pp を受入 |

共有資源: GitHub Actions のランナーとキャッシュ（`Swatinem/rust-cache`）のみ。秘密情報なし（`GITHUB_TOKEN` read）。

## 6. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR2.1 | ruleset に `required_status_checks`（3 コンテキスト、strict）を冪等スクリプトで追加、否定系 + 正常系の実地確認（§3） |
| NFR2.2 | `merge_group` トリガ追加、coverage は merge_group 時に絶対ゲートのみ（§2） |
| NFR2.3 | `check` ジョブに `tools/lint` の fmt / clippy / test（§2） |
| NFR2.4 | `TOLERANCE=0.01` + PBT シード固定（§4、手段は code-generation で確定） |
| NFR2.5 | `--ignore-filename-regex` で `main.rs` のみ除外（§4） |
| NFR4.1 | `audit` ジョブ（2 ロックファイル、required 外）（§2） |
| NFR4.2 | `rust-toolchain.toml` 1.95.0 + `dtolnay/rust-toolchain@master`（§2 / §4） |
| NFR4.3 | `[workspace.lints.rust] unsafe_code = "forbid"` + `tools/lint` 個別（§4） |
| NFR4.4 | `permissions: contents: read`（§2） |
| NFR4.5 | 前後 JSON を記録する手順スクリプト（§3） |

## 7. 見送り（後続 intent）

- Dependabot（`github-actions` / `cargo`）と GitHub Actions の SHA ピン留め — practices-discovery の裁定どおり本 intent では
  見送り（NFR 要求レビュー Minor 1 の引き取り — 対称に明記）。
- `audit` の required 化 — 運用 1 週間後に再判断（§2）。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T01:19:03Z
**Iteration:** 2（advisory, recovery, unit: u10-ci-governance）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | §2（CI ワークフロー ジョブ表）／§5（論理コンポーネント表） | `.github/workflows/ci.yml` には `review-thread-resolution`（`j5ik2o/ci` の再利用ワークフロー `review-thread-resolution.yml` を SHA `9cf0e9a8cd74c72de704763025003ed3b7608c65` で呼び出す）と `ci-success`（`check`/`quint`/`coverage`/`review-thread-resolution` の結果を集約する `needs` ジョブ）の 2 ジョブが実在するが、§2 のジョブ表・§5 のコンポーネント表のどちらにも行がない。オーナー指示（`superseding-decisions.md` #9、2026-08-23T00:40Z）による追加で、`.github/workflows/review-thread-resolution.yml` も新設されている。 | §2 に `review-thread-resolution` と `ci-success` の行を追加し（トリガ条件 `pull_request` のみ／`merge_group` では `review-thread-resolution` の `skipped` を許容、`ci-success` が最終ゲート）、§5 に「CI Review Thread Gate」をコンポーネントとして追加して障害ドメイン（レビュースレッド未解決 PR のマージ阻止・ボットコメント誤検知時の影響範囲）を記述する。 |
| 2 | Major | §2「権限」／§5「共有資源」 | §2 は「ジョブ個別の昇格なし」、§5 は「秘密情報なし（`GITHUB_TOKEN` read）」と明記しているが、実装では `review-thread-resolution` ジョブがジョブレベルで `permissions: { contents: read, checks: write, issues: read, pull-requests: read, statuses: write }` を個別付与しており、両記述と矛盾する（`ci.yml` 該当ジョブ、`.github/workflows/review-thread-resolution.yml` も同じ 5 権限）。セキュリティ設計文書内の権限モデルに関する断定が実態と食い違っている。 | §2 の「ジョブ個別の昇格なし」を「`review-thread-resolution` ジョブのみ `checks: write` / `statuses: write` / `issues: read` / `pull-requests: read` を個別付与（レビュースレッド状態をコミットステータスへ反映するため）」に訂正し、§5 の「秘密情報なし」も同様に訂正する。 |
| 3 | Major | §3（ruleset 変更）／`traceability.json` NFR2.1 | §3 の本文（「required checks のコンテキスト名は既存の 3 つを維持」）と例示 JSON、および `traceability.json` の NFR2.1 `target` はいずれも `required_status_checks` を `check` / `quint` / `coverage` の 3 コンテキストとしているが、実際の ruleset（`code-generation/ruleset/2026-08-23-ci-success/after.json`、`updated_at: 2026-08-23T09:15:41+09:00`）と `scripts/governance/ruleset-required-checks.sh` の `REQUIRED_CONTEXTS="check,quint,coverage,CI Success"` はいずれも 4 コンテキスト（+ `CI Success`）を要求している。 | §3 の本文・例示 JSON を 4 コンテキスト（`check` / `quint` / `coverage` / `CI Success`）に更新し、`traceability.json` NFR2.1 の `target` も同期する。 |
| 4 | Minor | §1（信頼境界）／§5 | 新規の外部再利用ワークフロー `j5ik2o/ci/.github/workflows/review-thread-resolution.yml`（サードパーティ組織リポジトリ、SHA ピン留め）が、CI ワークフローの信頼境界として設計のどこにも分析されていない。SHA ピン留め自体は健全な選択だが、「なぜこの外部ワークフローを信頼するか」「更新時の検証手順」は§1/§4のどの上流 NFR にも対応が無い（本 intent の NFR2.x/NFR4.x はこの機能追加より前に確定した要求）。 | §1 または §5 に「外部再利用ワークフローの信頼境界」を一項目として追加し、SHA 更新時の検証手順（例: 差分レビュー必須）を明記する。あわせて上流の `security-requirements.md` 側で対応する NFR ID の新設を検討する（本 intent の nfr-requirements ゲートで扱うべき事項として申し送り）。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts --stage nfr-design` | `{"pass":true,"gaps":[],"orphans":[],"findings_count":0}` | 構造的には全 NFR ID（NFR2.1〜2.5 / NFR4.1〜4.5）に coverage エントリと非空の `target` があり合格。ただし `target` の**内容が実装と一致しているか**まではこのセンサーの検証範囲外 — 上記所見 #3 はセンサー通過後に手動突合で発見した不一致 |
| `aidlc-sensor-required-sections.ts --stage nfr-design` | `{"pass":true,"h2_count":7,"findings_count":0}` | H2 見出し 7 個（登録済み既定の ≥2 を満たす）で合格 |
| 実装照合（`ci.yml` / `review-thread-resolution.yml` / `ruleset/2026-08-23-ci-success/after.json` / `scripts/governance/*.sh` / `scripts/coverage.sh` / `rust-toolchain.toml` / `Cargo.toml` / `tools/lint/Cargo.toml`） | TOLERANCE=0.05・`(^|/)modules/app/aidlc/src/main\.rs$`・`rust-toolchain.toml`（1.95.0 / rustfmt,clippy,llvm-tools）・`unsafe_code = "forbid"`（両 Cargo.toml）・`PROPTEST_RNG_SEED`・ruleset 冪等判定ロジックは設計と**一致**。`review-thread-resolution` / `ci-success` ジョブと required checks 4 コンテキストは設計と**不一致**（所見 #1〜#3） | iteration 1→2 で意図された更新（TOLERANCE・regex・toolchain 導出・PBT シード・ruleset 適用実績・UTC 表記）は正しく反映済み。その後のオーナー指示（CI Review Thread Gate、2026-08-23T00:40Z）が未反映という新規ギャップ |

### Summary

TOLERANCE・カバレッジ除外正規表現・toolchain 導出・PBT シード固定・ruleset 冪等判定など、iteration 1 からの既知の更新は実装と正しく同期している。一方で、それより後にオーナー指示で追加された CI Review Thread Gate（`review-thread-resolution` ジョブ・`ci-success` 集約ジョブ・外部再利用ワークフロー・required checks の 4 コンテキスト化）が `security-design.md` のどのセクションにも反映されておらず、§2/§5 の「ジョブ個別の昇格なし」「秘密情報なし」という明示的な記述は実装と矛盾している（所見 #1〜#3、Major 3 件）。いずれも実装済みコードとの単純な同期作業で解消できる範囲だが、セキュリティ設計文書が実際の権限モデル・必須チェック集合について誤った断定をしている状態は看過できないため、advisory 目安（Major ≤ 2 で READY）に従い NOT-READY とする。人間の承認ゲートでは、この同期修正を Request Changes として本ステージへ差し戻すか、次サイクルの回復レビューへ申し送るかの判断を推奨する。
