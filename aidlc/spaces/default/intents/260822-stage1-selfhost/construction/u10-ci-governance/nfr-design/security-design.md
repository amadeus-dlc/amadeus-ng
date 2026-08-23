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

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-22T17:29:52Z
**Iteration:** 1（advisory, unit: u10-ci-governance）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Minor | security-design.md §4（カバレッジ除外行）vs `../nfr-requirements/tech-stack-decisions.md` §1「カバレッジ除外」行 | 上流の承認済み決定（tech-stack-decisions.md、iteration 1 で別途レビュー対象外だが既存READY文書）は `--ignore-filename-regex '^modules/app/aidlc/src/main\.rs$'`（先頭 `^` 付き、「相対パス基準」の注記あり）と明記しているが、本設計 §4 は同じ箇所を `--ignore-filename-regex 'modules/app/aidlc/src/main\.rs$'`（`^` なし、相対パス基準の注記も欠落）と書いている。末尾 `$` があるため実害（誤除外）は無い（末尾一致で機能は保たれる）が、承認済み決定と設計の間で逐語の食い違いが生じており、code-generation 実装者がどちらを正本とすべきか判断に迷う余地がある。 | `^` アンカーと「相対パス基準」の注記を tech-stack-decisions.md の記述に合わせて復元するか、意図的な変更であれば根拠を一文添える。 |
| 2 | Minor | security-design.md §3 手順 2（ruleset 冪等スクリプトの判定ロジック） | 冪等性の判定が「`rules[]` に `required_status_checks` が**存在するか**」の二値判定のみで、存在する場合の中身（3 コンテキスト `check`/`quint`/`coverage`、`strict_required_status_checks_policy: true`）が期待値と一致するかは検証しない設計になっている。初回実行（現状 ruleset に該当ルールなし、`gh api repos/amadeus-dlc/amadeus-ng/rulesets/21190453` で確認済み）では問題にならないが、将来誰かが手動で異なるコンテキスト集合の `required_status_checks` を設定した場合、再実行しても「既に存在する」判定で無変更のまま終了し、NFR4.5（追跡可能な変更管理）が期待する「意図した状態への収束」を保証できない。 | 存在チェックを「ルール type の有無」ではなく「`parameters` が期待値（3 コンテキスト + `strict: true`）と一致するか」に変更し、不一致なら更新するロジックに改める（code-generation 実装時の注記として §3 に一文追加でも足りる）。 |

NFR 要求レビュー（`../nfr-requirements/security-requirements.md` `## Review`）の Minor 3 件の引き取り状況: Minor 1（Dependabot の非対称記載）は §7 で SHA ピン留めと対称に明記済み — 解消。Minor 3（merge queue 正常系の受入未記載）は §3「受入（正常系...）」と `traceability.json` NFR2.1 の target に明記済み — 解消。Minor 2（NFR4.2 合格基準に機械検証可能な基準と運用規範が混在）は要求文書側の記述粒度の指摘であり、本設計ステージが取り込む必要のある設計ギャップではない（design 側は §6 で NFR4.2 に `rust-toolchain.toml` 1.95.0 + `dtolnay/rust-toolchain@master` という具体的手当てを示しており、要求側の文言分割は nfr-requirements の管轄）— 対応不要と判断。

### Validation Tool Results

| チェック | コマンド | 結果 | 解釈 |
|---|---|---|---|
| traceability センサー | `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-design --output-path .../nfr-design/traceability.json` | `{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[],"findings_count":0}` | upstream_ids（NFR2.1〜2.5, NFR4.1〜4.5 の 10 件）と coverage 行が過不足なく一致 |
| required-sections センサー（security-design.md） | `bun .claude/tools/aidlc-sensor-required-sections.ts --stage nfr-design --output-path .../security-design.md` | `{"pass":true,"h2_count":7,...}` | 7 H2、既定の ≥2 を満たす |
| upstream-coverage センサー（security-design.md） | `bun .claude/tools/aidlc-sensor-upstream-coverage.ts --stage nfr-design --output-path .../security-design.md` | `{"pass":true,"reason":"no upstream","findings_count":0}` | 直接パス起動では upstream 解決対象なしとして pass（非ブロッキング） |
| traceability.json の target ↔ security-design.md 節番号突合（手動） | — | 10 件全一致（NFR2.1→§3、NFR2.2→§2、NFR2.3→§2、NFR2.4→§4、NFR2.5→§4、NFR4.1→§2、NFR4.2→§2/§4、NFR4.3→§4、NFR4.4→§2、NFR4.5→§3） | target が指す節が実在し内容と整合。GAP なし |
| `.github/workflows/ci.yml` 実物突合 | `cat .github/workflows/ci.yml` | `merge_group` 無し、`permissions` 未指定、`tools/lint` ステップ無し、`coverage` ジョブは既に `if: github.event_name != 'pull_request'` で絶対ゲート分岐済み | 「現状」記述と一致。§2 の `coverage` 行が主張する merge_group 時の挙動は、既存の `!= 'pull_request'` 条件が `merge_group` にも自然にヒットするため追加変更なしで成立することを確認（設計の正しさを裏付け） |
| `scripts/coverage.sh` 実物突合 | `cat scripts/coverage.sh` | `TOLERANCE=0.5`（現状）、`--ignore-filename-regex` 相当の除外設定は無し | NFR2.4/2.5 の「現状」記述と整合。§4 の変更対象行が実在することを確認 |
| `Cargo.toml` / `tools/lint/Cargo.toml` 実物突合 | `cat Cargo.toml`, `cat tools/lint/Cargo.toml` | `[workspace.lints.rust]` に `unsafe_code` 無し、`tools/lint` の `[lints.rust]` は `missing_docs` のみ | NFR4.3 の「現状」記述と一致 |
| `main.rs` 実物確認 | `head -30 modules/app/aidlc/src/main.rs` | `const fn main() {}` のみ（composition root 未実装） | カバレッジ除外対象がまだ空実装であることを確認。除外設定自体は正当 |
| GitHub ruleset 実地確認（読取） | `gh api repos/amadeus-dlc/amadeus-ng/rulesets/21190453` | `rules` は `deletion`/`non_fast_forward`/`merge_queue`(SQUASH/ALLGREEN) のみ、`required_status_checks` 無し、`bypass_actors` 空 | §3 の「既存規則の維持」設計（前後 JSON 取得 → 既存 rules に追加 → PUT）が正しい起点データに基づいていることを確認 |
| GitHub ruleset PUT の全置換セマンティクス（Web調査） | WebSearch/WebFetch（GitHub Docs、コミュニティ議論） | ドキュメントは明示的でないが、`rules` 配列は提供時に全置換される挙動が実務上一般的に報告されている | §3 の手順（before.json 取得 → 既存 rules を保持したまま新規則を配列に追加 → PUT）はこの全置換前提でも安全に動作する設計になっている — 妥当 |
| `strict_required_status_checks_policy` と merge queue の相互作用（Web調査） | WebSearch（GitHub Docs） | 公式ドキュメントに明示的な非推奨・警告の記述なし。ただし実務上の既知の摩擦点であり、本 Unit の `merge_queue` 設定（`max_entries_to_build:1` / `min,max_entries_to_merge:1`／ALLGREEN）は実質的にキューを直列処理する構成のため、リスクは低い | §3 が正常系（緑PRの完走）の実地確認を受入に含めている（NFR 要求レビュー Minor 3 の引き取り）ことで、ドキュメント上の不確実性を経験的に担保している — 妥当な設計判断 |
| `cargo audit --file` フラグの実在確認 | WebFetch（rustsec/rustsec リポジトリ `cargo-audit/src/commands/audit.rs`） | `#[arg(short = 'f', long = "file", help = "Cargo lockfile to inspect ...")]` が実在 | §2 の `cargo audit --file tools/lint/Cargo.lock` は実在するフラグの正しい用法 |
| `cargo llvm-cov --ignore-filename-regex` の実在確認 | WebSearch | 実在するオプション（正規表現でファイルパスを除外） | §4 の使用法自体は妥当（Finding #1 はパターン文字列の逐語不一致の指摘であり、オプション自体の妥当性とは別） |
| `PROPTEST_RNG_SEED` 環境変数の実在確認 | WebSearch（proptest ドキュメント） | 実在する環境変数（未設定時はランダムシード） | §4 の PBT シード固定の第一候補が実在する API に基づいていることを確認（虚偽記載なし） |
| ruleset required checks コンテキスト名 ↔ ci.yml ジョブ名突合 | 手動 | `check`/`quint`/`coverage` の 3 つが ci.yml の `jobs.<id>.name` と完全一致 | §3 の `required_status_checks` コンテキスト名が実装済みジョブ名と食い違わないことを確認 |
| `scripts/governance/ruleset-required-checks.sh` / `rust-toolchain.toml` 未作成確認 | `test -f ...` | いずれも不在 | 設計ステージの制約（完全な実装は code-generation で書く）どおり、まだ実装物を先取りしていないことを確認 |

### Summary

U10（packaging）の NFR 設計は、上流要求（NFR2.1〜2.5・NFR4.1〜4.5 の全10件）と traceability.json の対応が過不足なく一致し、target が指す節もすべて本文に実在した。GitHub ruleset の PUT 全置換セマンティクスに対して「前後 JSON を取得し既存 3 規則を保持したまま新規則を配列へ追加する」という手順は、公式ドキュメントの記述が曖昧な中でも安全側に倒れた妥当な設計であり、`strict_required_status_checks_policy` と merge queue の相互作用という既知のリスクに対しても正常系の実地確認を受入基準に加えて経験的に担保している。`cargo audit --file`・`--ignore-filename-regex`・`PROPTEST_RNG_SEED` はいずれも実在する API/フラグであることを外部調査で確認でき、虚偽・誇張の技術記載は見つからなかった。`audit` を required から外す判断も、project.md の Mandated 記述（CI 3 ジョブのみを branch protection の required にする）と整合しており根拠がある。設計ステージの制約（コード例示 ≤15 行）も遵守している。2 件の Minor 所見（カバレッジ除外パターンが上流決定と逐語不一致・ruleset 冪等スクリプトが規則の中身までは照合しない設計）はいずれも実装を妨げるものではなく、developer が本書だけで Bolt B2 を実装できる水準にある。Critical / Major 所見は無し。

**更新ファイル:** `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/nfr-design/security-design.md`（`## Review` セクションを末尾に追加）。`traceability.json` と `nfr-design-questions.md` は編集していない。
