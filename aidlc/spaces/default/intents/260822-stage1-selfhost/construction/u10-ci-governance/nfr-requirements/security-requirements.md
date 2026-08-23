# security-requirements — U10 CI ガバナンス（`u10-ci-governance`）

> NFR Requirements（Construction 3.2）成果物（Unit: U10、kind: packaging）。出典:
> `../../../inception/requirements-analysis/requirements.md`（FR9.1〜9.5、NFR2 品質ゲート維持、NFR4 セキュリティ /
> サプライチェーン）、`../../../inception/units-generation/unit-of-work.md`（U10 の責務・境界・合格）、
> `../../../inception/contract-design/contract-summary.md`（外部契約は C1 / C2 のみ — U10 は契約面を持たない）、
> `../../../inception/practices-discovery/evidence.md`（確定アクション 1〜4、実測）、`aidlc/spaces/default/codekb/docs/
> technology-stack.md`（GitHub Actions 1 本 3 ジョブ、cargo-llvm-cov、Quint 0.32.0、rust-toolchain.toml 不在）、
> `aidlc/spaces/default/memory/team.md`（Testing Posture / Code Style の確定事項）、確認事項 `nfr-requirements-questions.md`
> （前提 P1〜P8、Looks correct）、実地確認（2026-08-22 UTC: `gh api` の ruleset、`ci.yml`、`scripts/coverage.sh`、`tools/lint/`）。
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
- 実地の現状（2026-08-22 UTC）: `main` に **ruleset「main」（active）** — `deletion` / `non_fast_forward` / `merge_queue`（SQUASH、
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
**Date:** 2026-08-23T00:36:45Z
**Iteration:** 2(advisory, recovery, unit: u10-ci-governance)

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | `security-requirements.md` NFR2.4(§2) | NFR2.4 の合格基準はいまも「同一コードの 2 回計測で line coverage の差が 0.00pp」「`TOLERANCE=0.01` で相対ゲートが誤検知しない」と書かれているが、実装済みの `scripts/coverage.sh:45` は `TOLERANCE=0.05` であり、同じ Unit の `tech-stack-decisions.md` §1・§3 と `superseding-decisions.md` #1 は「PBT シード固定後も `fs_workspace_lock.rs:237` の並行テスト由来で ±1 行(0.0175pp)の揺れが残るため差 0.00pp は未達、U3 のロック退役後に 0.01 へ引き締める」と明記している。同一 Unit 内の 3 文書が食い違ったまま — この主成果物だけが暫定値(0.05)と未達の事実を反映していない。 | NFR2.4 の合格基準を「`TOLERANCE=0.05`(暫定)。2 回計測の差は PBT 由来の揺れは 0.00pp まで決定化済みだが、FS ロック並行テスト由来の残差(実測 0.0175pp)により全体では非 0。U3 のロック退役(ADR-007)後に 0.01 へ引き締める」へ書き換える。 |
| 2 | Major | `security-requirements.md` §1 / NFR2.1 / NFR4.4 / §3 STRIDE、`tech-stack-decisions.md` 全体 | `superseding-decisions.md` #9(2026-08-23T00:40Z、オーナー指示、"追記" — 同 #10 が明記するとおり凍結文書への反映対象は #1〜#8 のみで #9 は含まれない)以降、`.github/workflows/ci.yml` には `review-thread-resolution` ジョブ(j5ik2o/ci の外部再利用ワークフロー、SHA 固定呼び出し)と `ci-success` 集約ジョブが追加され、ruleset「main」の `required_status_checks` は **4 コンテキスト**(`check`/`quint`/`coverage`/`CI Success`)に拡張済み — `ruleset/2026-08-23-ci-success/after.json`(updated_at 2026-08-23T09:15:41+09:00)で確認。ところが本成果物の NFR2.1 は「3 コンテキスト」のまま、NFR4.4 は「`permissions: contents: read` を明示(ジョブ個別の昇格なし)」「3 ジョブ + audit が read 権限で成功」と書いている一方、実装の `review-thread-resolution` ジョブは `permissions: contents: read, checks: write, issues: read, pull-requests: read, statuses: write` という**ジョブ個別の権限昇格**を明示的に持つ(`.github/workflows/ci.yml` の該当ジョブ定義、`review-thread-resolution.yml` も同様)。NFR4.4 の「ジョブ個別の昇格なし」という最小権限の主張は現行実装と矛盾している。§1(信頼境界)・§3(STRIDE の Elevation of Privilege 行)・`tech-stack-decisions.md`(選定表)のいずれにも、この外部 SHA 固定ワークフロー呼び出しと書込権限を持つ新しいガバナンス層への言及が一切ない。 | (a) NFR2.1 を「4 コンテキスト(check/quint/coverage/CI Success)」に訂正。(b) NFR4.4 の「ジョブ個別の昇格なし」を「`review-thread-resolution` ジョブのみ `checks: write` / `statuses: write` / `issues: read` / `pull-requests: read` を個別付与(未解決レビュースレッドの検出に必要な最小権限)」に訂正。(c) §1 信頼境界と §3 STRIDE(Elevation of Privilege)に、外部再利用ワークフロー(SHA 固定 `j5ik2o/ci/.../review-thread-resolution.yml@9cf0e9a8...`)という新しい信頼境界・攻撃面を追記。(d) `tech-stack-decisions.md` §1 に選定行(review-thread-resolution ゲート採用・SHA 固定・不採用案)を追加。 |
| 3 | Minor | `security-requirements.md` NFR4.2(§2、合格基準列) | 合格基準が「ローカルと CI の `rustc --version` が同一。toolchain 更新は PR でのみ」のまま — 機械検証可能な基準(rustc バージョン一致)と運用規範(PR 経由での更新)が同じセルに混在している(iteration 1 の Minor 2 の再掲)。`nfr-design/security-design.md:133` はこの指摘を「要求文書側の記述粒度の指摘であり、nfr-requirements の管轄」として明示的に本ステージへ差し戻している — 本成果物側での対応がまだ空白のまま。 | 合格基準を機械検証可能な行(`rustc --version` 一致)と運用規範の行(toolchain 更新は PR 経由)に分けて記載する、または運用規範を「出典」列や別注記に移す。 |
| 4 | Minor | `security-requirements.md` NFR2.1(§2、合格基準列) | NFR2.1 の合格基準は「赤のまま merge queue に入れた PR がマージされない」という否定的経路のみを検証対象にしており、肯定的経路(全緑の PR が merge queue を完走してマージされる)への言及がない。`superseding-decisions.md` #3 は PR #25 が `merge_group` CI 緑で完走し squash-merge されたことを「NFR2.1 の正常系受入を満たす」と記録しているが、この事実は本成果物の合格基準列には反映されていない(iteration 1 の Minor 3 の再掲。`nfr-design/security-design.md` 側では正常系受入を明記済みだが、本成果物側は据え置き)。 | 合格基準に正常系(「全緑の PR が merge queue を経て squash-merge される」)を追記する。実績として PR #25(2026-08-22T23:44:17Z UTC 換算)を出典に添えられる。 |
| 5 | Minor | `security-requirements.md` §5 適用外 / §3 脅威の検討 | Dependabot(`github-actions` / `cargo`)を本 intent で見送った裁定(`practices-discovery/evidence.md`、`nfr-design/nfr-design-questions.md`)への言及が本成果物に無い。SHA ピン留め見送りは §3 Denial of Service 行と `tech-stack-decisions.md` §2 に明記されているが、Dependabot は非対称に欠落している(iteration 1 の Minor 1 の再掲)。`nfr-design/security-design.md` §7 では両者を対称に記載済みで実質的には解消しているが、本成果物自体は単体で読んだときにこの非対称さが残る。 | §3 または §5 に一行、「Dependabot(github-actions/cargo)の導入は SHA ピン留めと同様に本 intent では見送り、後続 intent で扱う」を追記する。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts --stage nfr-requirements` | `{"pass":true,"gaps":[],"orphans":[],"findings_count":0}` | traceability.json は upstream_ids(NFR1〜5)と coverage 配列が一致し、N/A の正当化も含め機械的な破綻なし。 |
| `aidlc-sensor-required-sections.ts`(security-requirements.md) | `{"pass":true,"h2_count":5}` | 必須 H2 見出し(範囲/要求/脅威/データ分類/適用外)は揃っている。 |
| `aidlc-sensor-required-sections.ts`(tech-stack-decisions.md) | `{"pass":true,"h2_count":3}` | 必須 H2 見出し(選定/依存差分/未決)は揃っている。 |
| `grep TOLERANCE scripts/coverage.sh` | `TOLERANCE=0.05` | Finding #1 の裏付け — 成果物の記載(0.01/0.00pp)と実装が不一致。 |
| `cat .github/workflows/ci.yml` | `review-thread-resolution` / `ci-success` ジョブと `permissions: checks: write, statuses: write, ...` を確認 | Finding #2 の裏付け — 成果物が記述しない新しいガバナンス層と権限昇格が実在する。 |
| `cat ruleset/2026-08-23-ci-success/after.json` | `required_status_checks` に 4 コンテキスト(check/quint/coverage/CI Success) | Finding #2 の裏付け — ruleset の実地状態は「3 コンテキスト」という記載と食い違う。 |
| `grep C1/C2 contract-summary.md` | C1・C2 は U7 の外部契約、U10 には契約面が無いという記載と整合 | §1 冒頭の契約面に関する記述は正しく、破綻なし。 |

### Summary

構造面(必須見出し・traceability 上流ID網羅)は健全で、iteration 1 で確認された FR9.1〜9.5/NFR2/NFR4 の要求分解自体に破綻はない。一方でこの回復レビューが本来の目的とした「実装後の実態との整合」については、2 件の Major な乖離を検出した — (1) NFR2.4 の合格基準がいまも達成済みでない `TOLERANCE=0.01`/差 0.00pp を掲げたままで、同じ Unit の tech-stack-decisions.md や superseding-decisions.md 自身がすでに「暫定 0.05」と認めている自己矛盾、(2) オーナー指示による最新の追加(review-thread-resolution ゲート・CI Success 集約・required checks の 4 コンテキスト化・ジョブ個別の権限昇格)が superseding-decisions.md #9 として記録済み・実装済み・GitHub 側にも反映済みであるにもかかわらず、本成果物の NFR2.1・NFR4.4・§1・§3 のいずれにも一切反映されていない(#9 は明示的に「#1〜#8 のみ反映」の対象外)。後者は特に NFR4.4 の「ジョブ個別の昇格なし」という最小権限の主張が現行実装と正面から矛盾しており、セキュリティ要求文書としての信頼性に関わる。Critical(実装・ランタイムを壊す欠陥)は無く、Major は 2 件、Minor は 3 件(いずれも iteration 1 由来の再掲・部分未解消)であり、advisory の目安(Critical 0 かつ Major ≤ 2)は満たすため Verdict は READY とするが、Major 2 件はいずれも「セキュリティ要求文書が現行の実地状態を誤って記述している」という同種の実害を持つため、承認ゲートでは本文修正(#1・#2)を先に済ませることを強く推奨する。
