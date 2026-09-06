# security-requirements — U10 CI・品質管理（`u10-ci-governance`）

> 2026-09-06改訂。CI設定と承認済み方針へ要件を整合させる。今回の改訂は文書3点に限定し、GitHub設定・品質閾値・プロダクトコードは変更しない。

## Sources

- [Q1] `nfr-requirements-questions.md` の2026-09-06確認要約（Looks correct）。
- [requirements] `../../../inception/requirements-analysis/requirements.md` のFR9.1〜9.5、NFR2、NFR4。
- [contracts] `../../../inception/contract-design/contract-summary.md` と `../../../inception/units-generation/unit-of-work.md`。U10はpackagingで、製品の外部契約を所有しない。
- [local] リポジトリルートの `.github/workflows/ci.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/toolchain-inputs.sh`、`scripts/governance/ruleset-required-checks.sh`（2026-09-06読取）。
- [observed] `../ruleset-observed-20260906.json`。同日取得済みのGitHub設定を示す観測記録であり、将来の状態を保証するものではない。
- [history] `../code-generation/superseding-decisions.md` の過去の裁定。暫定許容差0.05などの旧値は現行値と区別する。

## 1. 範囲と信頼境界

対象はCI、品質検査、依存検査、マージ条件の管理である。FR9.6のエラー様式規則はU9の責務であり、ここでは変更しない。

信頼境界は、GitHub Actionsの実行環境とトークン、外部Action・再利用ワークフローの取得先、crates.io・RustSec advisory DB・Node/Quintの配布元、管理者権限で変更するrulesetに分かれる。SHA固定は特定版への固定であり、そのコード自体の安全性を証明するものではない。

観測済みのruleset「main」（ID 21190453）はactiveで、必須チェックは `check` / `quint` / `coverage` / `CI Success` の4つ、strict有効、bypassなし。削除・force push防止、マージキューのSQUASH・ALLGREEN・同時1件が設定されている。設定の存在と、成功・失敗両経路の実働確認は別の証拠として扱う。

`ci.yml` は `pull_request` / `merge_group` / `workflow_dispatch` で起動する。`CI Success` は基本3チェックと `aidlc-distribution`、`review-thread-resolution` の結果を集約する。`audit` は集約対象・必須チェックともに含めない。

## 2. 要求

| ID | 要求 | 測定可能な合格基準 | 出典 |
|---|---|---|---|
| NFR2.1 | 必須チェック4つをrulesetで強制し、既存のマージキュー・保護規則・bypassなしを維持する | 設定JSONで4コンテキストの集合とstrict=trueを確認する。必須検査失敗時にマージされない経路と、全成功時にキューを完走しsquash-mergeされる経路の両方について、対象変更・実行URL・結果を保存する | FR9.1, NFR2, Q1, observed |
| NFR2.2 | キュー用検査を実行し、CI Successが依存検査の失敗・取消・不正なスキップを成功へ読み替えない | check/quint/coverage/aidlc-distributionはすべてsuccess必須。変更提案ではreview-thread-resolutionもsuccess必須、merge_group/workflow_dispatchでは同検査のskippedを受理する。イベントごとの実行結果を確認する。coverageは変更提案時に絶対・相対ゲート、他2イベントでは絶対ゲートを実行する | NFR2, Q1, local |
| NFR2.3 | workspaceと独立クレートtools/lintを品質検査の対象にする | checkの実行ログでworkspaceのfmt/clippy/cargo lint/testと、tools/lintのmanifest-path指定によるfmt/clippy/testが成功する。テスト件数は実行時の結果を記録し、過去の31本に固定しない | FR9.3, NFR2 |
| NFR2.4 | シード20260823をCIとローカルで統一し、カバレッジ相対差の許容を0.01ポイントに維持する | 同一コード・ツールチェーン・シードで2回測定し、生のhead値と差を記録する。差0.00ポイントの再現性を受入目標とし、未達なら未達のまま原因を記録する。相対ゲートはhead >= base - 0.01で判定する。固定シードの存在だけで再現性達成とはしない | FR9.4, NFR2, Q1 |
| NFR2.5 | main.rsの配線ファイルだけを明示除外し、残るworkspace計測対象のカバレッジ90%以上を維持する | 除外式が `(^|/)modules/app/aidlc/src/main\.rs$` のみで、クレート全体の除外がないことを確認する。計測結果が90%以上でabsolute gate成功。tools/lintはworkspace外であり、この90%床の対象と誤記しない | FR9.5, NFR2 |
| NFR4.1 | workspaceとtools/lintの両Cargo.lockをcargo auditの対象とし、結果を可視化する | 両方の実行・結果を識別できるログを残す。脆弱性検出・DB取得失敗・未実行を成功と扱わない。先行ステップ失敗で後者がskippedなら両方成功とは扱わず、必要な再実行で確認する。auditは既存裁定によりadvisoryであり、単独の赤はrulesetによるマージ阻止を保証しない | FR9.2, NFR4, Q1 |
| NFR4.2 | Rust 1.95.0、rustfmt/clippy/llvm-tools、minimalをrust-toolchain.tomlで一元管理する | CI入力がtoolchain-inputs.shで同ファイルから導出され、ローカルとCIのrustcが指定版1.95.0に一致することをログで確認する | FR9.2, NFR4 |
| NFR4.3 | workspaceメンバーとtools/lintでunsafe_code=forbidを適用する | 全workspaceメンバーのlints継承とtools/lintの個別宣言を確認する。両範囲のclippyが成功し、適用検証ではunsafeを含む不適合例が拒否される | FR9.2, NFR4 |
| NFR4.4 | workflow既定をcontents: readとし、レビュー検査に必要な個別権限だけを付与する | review-thread-resolutionにcontents: read、checks: write、statuses: write、issues: read、pull-requests: readがあることを確認する。他ジョブの追加書込権限がないこと、外部呼出先とci_refが同じSHAで固定されること、トークンを出力しないことを設定・実行ログの検査対象にする | FR9.2, NFR4, Q1 |
| NFR4.5 | rulesetの変更内容と実行主体を追跡可能にする | ruleset-required-checks.shの手順、変更時の前後JSONと結果を保存する。既存規則・4コンテキスト・strict・bypassの維持を確認する。現在値と要求が同じ場合は変更不要として記録する | NFR4, Q1 |

### 運用規範

ツールチェーン・シード・依存・CI・Action参照版の更新は、レビュー対象の変更提案を経て行う。これはNFR4.2の版一致という測定基準とは別の運用規範である。ruleset変更は権限を持つ担当者が実行し、今回の要件改訂では実行しない。脆弱性検出時は依存更新を検討し、外部DB取得失敗時は原因と再実行結果を記録する。

### 現時点の確認と未検証事項

設定の読取と保存済みruleset JSONから、4コンテキスト、権限、シード、閾値、ツールチェーンの宣言を確認した。今回の要件改訂ではカバレッジ2回測定、cargo audit、全CI実行、キューの成功・失敗試験は実行していない。これらは後続の検証項目であり、達成済みとは記録しない。

旧レビューが扱った許容差0.05・残差0.0175ポイントは過去の実測と暫定裁定である。現行の `scripts/coverage.sh` は0.01であり、今回の要件は確認済み要約に従いこの値を維持する。

## 3. 脅威の検討（STRIDE、ガバナンス面）

| 区分 | 脅威 | 対応と限界 |
|---|---|---|
| Spoofing（なりすまし） | トークンや管理者権限の悪用 | GitHubの認証下でもトークンは秘密情報。個別権限と利用先を限定する（NFR4.4/4.5） |
| Tampering（改竄） | 不合格コードのマージ、依存・外部Actionの改竄 | 必須チェック・既存保護・lockファイル・依存検査を組み合わせる。auditは署名検証の代替ではない |
| Repudiation（否認） | 誰がどのrulesetを変更したか不明 | NFR4.5の前後JSON・実行結果・実行主体を記録する |
| Information Disclosure（情報漏洩） | トークンがログや外部コードに渡る | トークンをログ出力しない。外部再利用ワークフローと実行権限を明記する。公開ログでも秘密情報がないと無条件に断定しない |
| Denial of Service（利用不能） | 外部配布元障害、検査未実行、キュー停滞 | イベント別の実行経路を検証し、失敗を可視化する。auditだけは必須外という既存裁定を維持する |
| Elevation of Privilege（権限昇格） | checks/statusesへの書込権限を持つ外部ワークフローの悪用 | レビュー検査のみに個別権限を与え、呼出先とci_refをSHA固定する。固定版更新時も変更内容をレビューする |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| 公開CIログ・カバレッジ結果 | Public | 公開を前提に秘密情報の出力を防ぐ。検査結果の未実行・失敗を区別する |
| ruleset観測JSON・前後JSON | Internal（運用記録） | 内容を確認して記録に保存する。認証トークンや認証ヘッダーを混ぜない |
| GITHUB_TOKENなどの認証情報 | Secret | ジョブごとの権限を限定し、ログ・成果物へ出力しない |

## 5. 適用外と繰り延べ

- NFR1: U10は製品のupstream互換面を変更しないため直接の派生要件はない。
- NFR3: 製品の永続化・投影を持たないため対象外。ガバナンス変更の記録はNFR4.5で扱う。
- NFR5: U10固有のCI実行時間の数値目標は設けない。製品CLIの性能劣化測定要求を取り消すものではない。
- Dependabot（github-actions/cargo）の導入は既存裁定により見送り、後続の検討事項とする。
- 全ActionのSHA固定は未採用。現状では配布検証ジョブのcheckout/setup-bunとレビュー用外部ワークフローが固定され、他にはタグ・ブランチ参照が残る。全件固定済みとも全件未固定とも記載しない。

## Assumptions & Open Questions

None.


## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T13:55:52Z
**Iteration:** 1
**Request Challenge:** review:c23f88e6478d662f8377718fce748442

### Findings

本レビューはadvisory（承認判断の参考となる独立レビュー）。確認済みの2026-09-06要約と現行設定を基準とし、過去の暫定値を現在の要求として扱わない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |
| R-01 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/nfr-requirements/security-requirements.md > §2 NFR2.5行、および aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u10-ci-governance/nfr-requirements/tech-stack-decisions.md > §1 カバレッジ行 | 除外式の縦棒がMarkdown表の区切りとして解釈されるため、両行ともヘッダー4列に対して5列になる。合格基準・出典や理由の表示がずれる。式そのものはcoverage.shと一致している。 | 両方の表内で正規表現の縦棒をMarkdown用にエスケープするか、正確な式を表の外へ移して参照する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| required-sections | PASS、security-requirements: H2=7、tech-stack-decisions: H2=5 | レビュー追記前の2成果物で必要な文書構造を確認した。 |
| upstream-coverage | PASS、未参照0 | 今回解決済みの入力requirements・contract-summaryと、指定成果物security-requirements・tech-stack-decisions・traceabilityを指定して検査した。静的定義の全入力を指定した初回はfunctional-spec・rules・technology-stackを未参照としたが、今回のUnitへ配送された入力集合とは異なるため、その結果を欠落所見にはしない。 |
| traceability | PASS、gaps/orphans/invalid_targets等すべて0 | NFR1〜5の網羅とN/A理由があり、NFR2/NFR4から計10件の派生要求へ対応する。 |
| NFR派生IDの行存在確認 | PASS | traceability.jsonの全OK targetがsecurity-requirementsの要求行として存在する。 |
| Markdown表の列数確認 | FAIL、2行 | R-01の2行のみ4列に対して5列。 |
| linter | 対象外、直接起動はno-eslint-config（終了127） | 今回はMarkdown/JSON文書で、TS/JSコードの成果物・対象スニペットはない。ESLintによる検証成功とは扱わない。 |
| type-check | 対象外、直接起動はno-tsconfig-found（終了1） | 今回はMarkdown/JSON文書で、TS/TSXコードの成果物・対象スニペットはない。TypeScript検査成功とは扱わない。 |
| doctor | 46 passed / 0 failed | 検査設定の確認に使用。未初期化submodule・runtime-graph未生成等のadvisoryは、本要件の設定照合結果とは分ける。 |
| 現行設定・観測JSONとの照合 | 一致 | 4必須コンテキスト、strict、bypassなし、SQUASH/ALLGREEN/同時1件、イベント別CI Success、audit必須外、ジョブ別権限、固定シード、0.01、90%床、main.rsのみ除外、Rust版とAction参照範囲を確認した。 |
| 上流契約の責務照合 | 一致 | U10はpackagingでFR9.1〜9.5・NFR2/NFR4を担当し、製品外部契約C1/C2は所有しない。FR9.6のU9帰属を変更していない。 |

### Summary

必須チェックとキュー正常系・異常系、外部ワークフローの権限境界、依存検査の限界、現行の品質閾値を検証可能な要求として記述できている。残る所見は表の表示崩れ1件であり、カバレッジ再測定・依存監査・実際のキュー完走等が未実施である点も明示されているためREADYとする。今回はGitHub書込、全CI実行、カバレッジ測定は行っていない。
