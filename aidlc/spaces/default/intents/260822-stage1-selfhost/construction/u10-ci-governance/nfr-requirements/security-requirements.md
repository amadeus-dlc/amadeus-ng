# security-requirements — U10 CI ガバナンス（`u10-ci-governance`）

> NFR Requirements（Construction 3.2）成果物（Unit: U10、kind: packaging）。出典:
> `../../../inception/requirements-analysis/requirements.md`（FR9.1〜9.5、NFR2 品質ゲート維持、NFR4 セキュリティ /
> サプライチェーン）、`../../../inception/units-generation/unit-of-work.md`（U10 の責務・境界・合格）、
> `../../../inception/contract-design/contract-summary.md`（外部契約は C1 / C2 のみ — U10 は契約面を持たない）、
> `../../../inception/practices-discovery/evidence.md`（確定アクション 1〜4、実測）、`aidlc/spaces/default/codekb/docs/
> technology-stack.md`（GitHub Actions 1 本 3 ジョブ、cargo-llvm-cov、Quint 0.32.0、rust-toolchain.toml 不在）、
> `aidlc/spaces/default/memory/team.md`（Testing Posture / Code Style の確定事項）、確認事項 `nfr-requirements-questions.md`
> （前提 P1〜P8、Looks correct）、実地確認（2026-08-23: `gh api` の ruleset、`ci.yml`、`scripts/coverage.sh`、`tools/lint/`）。
>
> packaging Unit のため「セキュリティ要求」= サプライチェーン・最小権限・機械強制の要求であり、NFR2（品質ゲート維持）の
> 機械強制要求も同じ文書に置く（品質ゲートの機械強制はガバナンスの一部）。各要求は Inception の NFR ID を継承し
> 枝番を付ける（NFR2.x / NFR4.x）。FR9.1〜9.5 は本 Unit の機能要求としてそのまま参照する。

## 1. 範囲と信頼境界

- 対象はリポジトリの**ガバナンス面**だけ: `.github/workflows/ci.yml`、`Cargo.toml`（workspace lints）、`rust-toolchain.toml`、
  `scripts/coverage.sh`、`tools/lint/Cargo.toml`、GitHub の ruleset。プロダクトコードは触らない（`unsafe_code` 昇格で
  赤になるクレートがあれば U7 で直す — 前提 P8）。
- 信頼境界: (a) GitHub Actions の実行環境（`GITHUB_TOKEN` の権限）、(b) 外部ネットワーク（crates.io、RustSec advisory DB、
  GitHub Actions のアクション取得、Node / quint）、(c) GitHub の ruleset（オーナー権限でのみ変更可）。
- 実地の現状（2026-08-23）: `main` に **ruleset「main」（active）** — `deletion` / `non_fast_forward` / `merge_queue`（SQUASH、
  ALLGREEN、同時 1 件）。required status checks は**無い**。CI は `pull_request` と `workflow_dispatch` でのみ起動。
  `permissions` 未指定（既定権限）。toolchain は `dtolnay/rust-toolchain@stable`（floating）。`cargo audit` 無し。
  `unsafe_code` forbid はクレート個別 attribute（`modules/app/aidlc/src/main.rs` に漏れ）。`tools/lint` は detached クレートで
  CI の fmt / clippy / test が届いていない（設計監査 C27）。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR2.1 | `main` へのマージは CI 3 ジョブ（`check` / `quint` / `coverage`）の緑を**機械強制**する — ruleset「main」に `required_status_checks`（strict、3 コンテキスト）を追加する | `gh api repos/amadeus-dlc/amadeus-ng/rulesets/<id>` の `rules[]` に 3 コンテキストの `required_status_checks` が現れ、赤のまま merge queue に入れた PR がマージされない（実地 1 回） | FR9.1, NFR2, 前提 P1 |
| NFR2.2 | merge queue のチェックが走る — `ci.yml` に `merge_group` トリガを追加し、3 ジョブが `merge_group` イベントでも実行される（coverage は base ref の無い文脈では絶対ゲートのみ） | `merge_group` イベントの workflow run で 3 ジョブ成功が Actions 履歴で確認できる | 前提 P2, NFR2 |
| NFR2.3 | `tools/lint` も CI の品質ゲート下に置く — `check` ジョブに fmt / clippy（`-D warnings`）/ 自己テスト（赤例 31 本）の 3 ステップ | `check` ジョブのログに 3 ステップが現れ緑 | FR9.3, 設計監査 C27 |
| NFR2.4 | カバレッジ計測の決定化 — PBT のシードを固定し、同一コードの 2 回計測で line coverage の差が 0.00pp | `scripts/coverage.sh` を 2 回実行し `head` 値が一致（実地）。`TOLERANCE=0.01` で相対ゲートが誤検知しない | FR9.4, NFR2 |
| NFR2.5 | composition root（`modules/app/aidlc/src/main.rs`）だけをカバレッジ計測から除外し、それ以外は 90% 床を維持 | `scripts/coverage.sh` の除外設定が `main.rs` 1 ファイルに限定され、`[PASS] absolute gate` | FR9.5, NFR2 |
| NFR4.1 | 依存の脆弱性監査 — `cargo audit` を CI で実行し、workspace `Cargo.lock` と `tools/lint/Cargo.lock` の**両方**を対象にする。既知の脆弱性があれば CI 赤 | `audit` ジョブが 2 つのロックファイルに対して実行され緑（advisory DB 取得失敗は再実行） | FR9.2, NFR4 |
| NFR4.2 | ツールチェーンの固定 — `rust-toolchain.toml`（`channel = "1.95.0"`、`components = [rustfmt, clippy, llvm-tools]`、`profile = "minimal"`）。CI はこのファイルを尊重する | ローカルと CI の `rustc --version` が同一。toolchain 更新は PR でのみ | FR9.2, NFR4, 前提 P3 |
| NFR4.3 | `unsafe_code = "forbid"` を `[workspace.lints.rust]` に昇格し全メンバーに適用。detached の `tools/lint` は `[lints.rust]` に個別記載 | `cargo clippy --workspace --all-targets -- -D warnings` 緑、`main.rs` を含む全クレートで unsafe が拒否される | FR9.2, NFR4 |
| NFR4.4 | CI の最小権限 — `ci.yml` の workflow 直下に `permissions: contents: read` を明示（ジョブ個別の昇格なし） | `ci.yml` に記載があり、3 ジョブ + audit が read 権限で成功 | FR9.2, NFR4 |
| NFR4.5 | ガバナンス変更の追跡可能性 — ruleset 変更は `gh api` の手順をスクリプトに残し、変更前後の ruleset JSON を記録（オーナー権限で実行） | 手順スクリプトと実行結果（前後 JSON）がリポジトリまたは記録に残る | NFR4, 前提 P8 |

## 3. 脅威の検討（STRIDE、ガバナンス面）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing | 該当なし（CI は GitHub の認証下） | — |
| Tampering | 赤のままのマージ、`main` への直接 push / force push、依存の改竄 | NFR2.1（required checks）、既存 ruleset の `non_fast_forward` / `deletion`、`Cargo.lock` コミット + NFR4.1 audit、NFR4.2 toolchain 固定 |
| Repudiation | ruleset の変更履歴 | NFR4.5（前後 JSON の記録）。GitHub の監査ログも参照可能 |
| Information Disclosure | CI ログへの秘密情報 | 秘密情報を扱わない（`GITHUB_TOKEN` のみ、read 権限 — NFR4.4） |
| Denial of Service | advisory DB / crates.io / アクション取得の一時障害で CI 赤 | 再実行で回復（外部依存マップ）。SHA ピン留めは本 intent では見送り（practices-discovery の裁定） |
| Elevation of Privilege | workflow の書込権限による悪用 | NFR4.4 `contents: read`。ruleset 変更はオーナーのみ（`bypass_actors` 空を維持） |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| CI ログ・カバレッジ数値 | Public（公開リポジトリ） | 秘密情報なし |
| ruleset JSON（前後） | Internal | 記録ディレクトリに保存可（秘密情報なし） |
| `GITHUB_TOKEN` | Secret（GitHub 管理） | read 権限に限定、ログ出力しない |

## 5. 適用外

- NFR1（upstream 互換）: U10 はプロダクトの振る舞いを変えない — 適用外。
- NFR3（監査完全性）: 永続化・投影を持たない — 適用外。
- NFR5（性能）: 非目標。CI 実行時間の数値目標は立てない（`cargo audit` / `tools/lint` 3 ステップの追加で `check` ジョブが
  数分伸びる見込み — 許容）。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-22T17:20:29Z
**Iteration:** 1（advisory, unit: u10-ci-governance）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Minor | security-requirements.md §5 適用外 / §3 STRIDE（Denial of Service 行） | practices-discovery の devsecops 寄稿（`inception/practices-discovery/contributions/aidlc-devsecops-agent.md:28`）は、SHA ピン留めと Dependabot（`github-actions` + `cargo` エコシステム）をセットの任意事項として扱っている。STRIDE の DoS 行は SHA ピン留めの見送りを明示するが、Dependabot には本文中どこにも言及がない — 見送りなのか未検討なのか成果物だけでは判別できない。 | §5 か STRIDE の DoS 行に一文追加し、「Dependabot は SHA ピン留めとセットで本 intent では見送り、後続 intent で検討」と明記する（SHA ピン留めと対称にする）。 |
| 2 | Minor | security-requirements.md §2 表、NFR4.2 合格基準 | 合格基準が「ローカルと CI の `rustc --version` が同一」（実測可能）と「toolchain 更新は PR でのみ」（運用規範、機械検証不可）を1セルに混在させている。 | 機械検証可能な基準と運用ルールを分けて書く。運用ルールは §1 の信頼境界か注記に移すと、後続 build-and-test / ci-pipeline での受入試験がしやすい。 |
| 3 | Minor | security-requirements.md §2 表、NFR2.1・NFR2.2 合格基準 | NFR2.1 の合格基準は「赤のまま merge queue に入れた PR がマージされない」という否定側のみを検証する。tech-stack-decisions.md §1 が採用する `strict_required_status_checks_policy: true` は merge queue の ALLGREEN グルーピングと重ねると、ブランチ最新性要求がキュー投入をブロックする相互作用が知られており、正常系（緑の PR が最後まで squash-merge される）の実地確認が要求文に無い。NFR2.2 は「3 ジョブ成功が Actions 履歴で確認できる」までで、マージ完了までは確認しない。 | NFR2.1 か NFR2.2 に正常系の受入行を追加する: 「green な PR が 1 回、merge queue を通って squash-merge まで完走する（実地 1 回）」。Bolt B2 の実装時に併せて確認する。 |

### Validation Tool Results

| チェック | コマンド | 結果 | 解釈 |
|---|---|---|---|
| traceability センサー | `bun .claude/tools/aidlc-sensor-traceability.ts --stage nfr-requirements --output-path .../traceability.json` | `{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[],"findings_count":0}` | upstream_ids（NFR1〜5）と coverage 行が過不足なく一致。`reverse` フィールド省略はセンサー側で許容（`obj.reverse ?? []`）— 欠落ではない |
| required-sections センサー（security-requirements.md） | `bun .claude/tools/aidlc-sensor-required-sections.ts --stage nfr-requirements --output-path .../security-requirements.md` | `{"pass":true,"h2_count":5,...}` | 5 H2、フレームワーク既定の ≥2 を満たす |
| required-sections センサー（tech-stack-decisions.md） | 同上（対象ファイル差し替え） | `{"pass":true,"h2_count":3,...}` | 3 H2、既定を満たす |
| `gh api repos/amadeus-dlc/amadeus-ng/rulesets/21190453`（読取） | — | `merge_queue`（SQUASH/ALLGREEN）は active、`required_status_checks` ルールは無し、`bypass_actors` は空 | §1「実地の現状」の記述と完全一致（P1 前提の裏取り） |
| `.github/workflows/ci.yml` 実物突合 | `cat .github/workflows/ci.yml` | `merge_group` トリガ無し、`permissions` 未指定、toolchain は `dtolnay/rust-toolchain@stable`（floating）、`tools/lint` の CI ステップ無し | §1・NFR2.2・NFR2.3・NFR4.2・NFR4.4 の「現状」記述と一致 |
| `Cargo.toml` / `tools/lint/Cargo.toml` 実物突合 | `cat Cargo.toml`, `cat tools/lint/Cargo.toml` | `[workspace.lints.rust]` に `unsafe_code` 無し、`tools/lint` の `[lints.rust]` も `missing_docs` のみ | NFR4.3・tech-stack-decisions.md の「現状はクレート個別 attribute」記述と一致 |
| `rust-toolchain.toml` 存在確認 | `test -f rust-toolchain.toml` | 不在 | NFR4.2 の前提（新規作成が必要）と一致 |
| `modules/app/aidlc/src/main.rs` 実物確認 + 全クレート `forbid(unsafe_code)` grep | `grep -rn "forbid(unsafe_code)" modules/ tools/` | `main.rs` 以外の全 9 クレート（`tools/lint` 含む）に `#![forbid(unsafe_code)]` あり、`main.rs` のみ無し | §1「`main.rs` に漏れ」の主張が正確（過大でも過小でもない） |
| `Cargo.lock` / `tools/lint/Cargo.lock` パッケージ数 | `grep -c "^name = "` | ルート 74、`tools/lint` 5 | nfr-requirements-questions.md P4 の「74 パッケージ」「5 パッケージ」実測値と一致 |
| Dependabot 言及の横断検索 | `grep -rn "Dependabot" inception/practices-discovery/` | `contributions/aidlc-devsecops-agent.md:28` のみ（evidence.md の確定アクション・インタビュー Q4〜Q8 には無し） | Finding #1 の根拠。SHA ピン留めとは非対称な扱い |
| PBT `ProptestConfig` 実装状況 | `grep -rn "ProptestConfig" modules/` | ヒット無し | tech-stack-decisions.md §3「未決」の記載どおり未着手（虚偽記載なし、正直な未決扱い） |

### Summary

U10（packaging）の NFR 要求は、上流（FR9.1〜9.6・NFR2・NFR4・unit-of-work.md・practices-discovery 確定アクション）との対応が全数一致し、`gh api` の ruleset・`ci.yml`・`Cargo.toml`・`tools/lint/`・`main.rs` の実地確認ともすべて整合していた（虚偽・誇張・古い実測値は見つからなかった）。FR9.6（エラーハンドリング規則）が U9 の責務としてU10のスコープから正しく除外されている点、`dtolnay/rust-toolchain@master` への切替や `cargo audit --file` の使い分けなど技術選定も妥当。3 件の Minor 所見（Dependabot の扱いの非対称な記載、NFR4.2 合格基準の運用規範との混在、merge queue 正常系の受入未記載）はいずれも成果物の質を高める改善提案であり、実装を妨げるものではない。Critical / Major 所見は無く、developer が本書だけで Bolt B2 を実装できる水準にある。

**更新ファイル:** `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/nfr-requirements/security-requirements.md`（`## Review` セクションを末尾に追加）。他のファイルは編集していない。
