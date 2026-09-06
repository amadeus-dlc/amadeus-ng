# security-design — U10 CI・品質管理（`u10-ci-governance`）

> 2026-09-06改訂。更新済みの品質・安全性要件を、CI・設定管理・検証の設計に具体化する。今回更新する成果物は本書とtraceability.json。GitHub設定・コード・品質閾値は変更しない。

## Sources

- [Q1] `nfr-design-questions.md` の2026-09-06確認要約（Looks correct）。
- [requirements] `../nfr-requirements/security-requirements.md` のNFR2.1〜2.5 / NFR4.1〜4.5。
- [technology] `../nfr-requirements/tech-stack-decisions.md`。
- [contracts] `../../../inception/contract-design/contract-summary.md`、`../../../inception/domain-design/components.md`。U10はpackagingであり、製品CLI・フックの外部契約C1/C2を所有しない。
- [local] リポジトリルートの `.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/`（2026-09-06読取）。
- [observed] `../ruleset-observed-20260906.json`。同日取得済みの設定記録であり、実働試験の代わりにはしない。
- [history] `../code-generation/superseding-decisions.md`。暫定0.05などの旧裁定は過去の記録として区別する。

## 1. 設計方針と境界

CIは検査結果を計算し、GitHubのrulesetは必須結果とキュー条件をマージ判断へ適用する。設定の存在、検査の実行、結果の受理を分けて検証する。未実行・取消・取得失敗を成功へ読み替えない。

構成はCI定義、レビュー結果の再評価、品質設定、ruleset管理手順に分ける。外部依存にはAction・再利用ワークフロー、Rust/Node/Bun/Quintの配布元、crates.io、RustSec advisory DBがある。外部障害をマージ条件から隔離するのはauditだけで、他の必須検査が外部取得に失敗すればマージを止める。

トークンは秘密情報として扱う。SHA固定は取得版を固定する手段であり、提供元やコードの無害性を保証しない。参照版の変更は差分レビューと検証を伴う変更提案で行う。新規クラウド資源・AWS Bedrock・製品の永続化機構は導入しない。

## 2. CIの構成と結果の受理

`ci.yml` はmain向けの変更提案、merge_group、workflow_dispatchで起動する。concurrencyはworkflow名とrefで分離し、同一組の古い実行を取り消す。取消の結果を合格として受理しない。

| ジョブ | 責務 | 必須結果との関係 | 失敗時の扱い |
|---|---|---|---|
| check | workspaceのfmt/clippy/cargo lint/test、tools/lintのmanifest-path指定fmt/clippy/test | checkとして直接必須、CI Successもsuccess必須 | 不合格としてマージを止める |
| quint | Node 22 / Quint 0.32.0でquint-gate.shを実行 | quintとして直接必須、CI Successもsuccess必須 | モデル検査失敗・取得失敗を合格にしない |
| coverage | coverage.shで絶対・条件付き相対ゲートを評価 | coverageとして直接必須、CI Successもsuccess必須 | 閾値未達・計測失敗を合格にしない |
| aidlc-distribution | Bun 1.3.13で配布同期・ローカル修正と回帰試験を検査 | CI Success経由で必須 | 同期差分や回帰失敗でCI Successを止める |
| review-thread-resolution | SHA固定の外部ワークフローで未解決スレッドを検査 | 変更提案でCI Successがsuccess必須 | 失敗・取消・未実行は合格にしない |
| ci-success（表示名CI Success） | 上記結果をイベント別条件で集約 | CI Successとして直接必須 | always()で起動して結果を検査し、合わない結果を拒否 |
| audit | workspaceとtools/lintのCargo.lockをcargo auditへ渡す | 直接必須にもCI Successにも含めない | 赤・未実行を可視化し、脆弱性対応と取得失敗の再実行を分ける |

### イベントごとの違い

| イベント | check/quint/coverage/aidlc-distribution | レビュー検査 | coverage比較 |
|---|---|---|---|
| pull_request | 全件success必須 | success必須 | 絶対90%とbaseに対する相対差 |
| merge_group | 全件success必須 | skippedを受理 | 絶対90%のみ |
| workflow_dispatch | 全件success必須 | skippedを受理 | 絶対90%のみ |

CI Successは `needs` の結果を検査する。基本4検査のskippedやcancelledは受理しない。レビュー検査をスキップできるのは、変更提案以外の2イベントである。auditの先行コマンドが失敗して後続のtools/lint検査が走らなかった場合、そのロックファイルの監査は未実行として扱い、再実行で確認する。

### レビュー結果の再評価

`review-thread-resolution.yml` はレビュー・コメントの作成/変更/削除等、15分間隔、手動実行を契機に、同じ外部ワークフローで `Check unresolved comments` の状態を再評価する。手動指定では対象番号を指定でき、無指定は外部ワークフローへ空の入力を渡す。

再評価するコミットステータスと、`ci.yml` の実行時に集約したCI Successは別の出力である。再評価だけで既に完了したCI Successも自動更新されるとは、このローカル定義だけから保証しない。スレッドの解決・再開後にどの結果が更新され、最新のマージ条件へ反映されるかを実働検証で確認する。

## 3. 権限・秘密情報・外部コード

`ci.yml` のworkflow既定はcontents: readであり、追加権限はreview-thread-resolutionに限定する。別ファイルの再評価ワークフローでは、workflowとrefreshジョブに同じレビュー用権限を明示する。

| 対象 | 宣言する権限 | 目的と境界 |
|---|---|---|
| ci.ymlの通常ジョブ | contents: read | ソース・依存取得と検査。既定をwriteに広げない |
| ci.ymlのreview-thread-resolution | contents: read、issues: read、pull-requests: read、checks: write、statuses: write | レビューの読取と検査・状態の反映。外部ワークフローへ与える権限として明示 |
| review-thread-resolution.ymlのrefresh | 同上 | レビュー状態の再評価。別ワークフローの権限であり「全workflowが読取専用」とは記述しない |

両レビュー呼出の参照版とci_refは、以下の同一SHAを使う。

`9cf0e9a8cd74c72de704763025003ed3b7608c65`

呼出先は `j5ik2o/ci/.github/workflows/review-thread-resolution.yml`。更新時は参照版とci_refの一致、権限差分、入力・出力の契約、解決/未解決/検査不能の結果を確認する。トークンをログ・保存JSON・成果物へ出力しない。

配布検証ジョブのcheckout/setup-bunもSHA固定され、同ジョブのcheckoutはpersist-credentials=false。その他にはタグ/ブランチ参照が残る。特定ジョブの設定を全ジョブへ一般化しない。全Actionの一括SHA固定とDependabot導入は既存裁定により見送る。

## 4. rulesetの管理と復旧

観測済みのruleset「main」は4コンテキスト（check/quint/coverage/CI Success）、strict=true、bypassなし。deletion・non_fast_forward・merge_queueを維持し、キューはSQUASH・ALLGREEN・同時1件とする。

管理手順の置き場は `scripts/governance/ruleset-required-checks.sh`。設計上の操作順は次のとおり。

1. 名前から対象を解決し、現在のJSONを取得してbefore.jsonに保存する。認証情報は保存しない。
2. 必須コンテキストの集合とstrictを比較する。一致すればPUTを実行しない。
3. 不一致なら、既存rulesからrequired_status_checksだけを置換し、他の規則・conditions・bypass_actorsを保持した送信用JSONを作る。dry-runでは予定JSONを確認する。
4. 権限を持つ担当者が変更し、再取得したafter.jsonで結果を確認する。既存スクリプトの自動検査はコンテキスト集合・strict・保護規則の存在を確認するため、キューの具体値やbypassの不変性は前後JSONの比較でも確認する。
5. 必須検査失敗時にマージを止める経路と、全成功時にキューを完走する経路を実働で確認し、対象版・実行結果を保存する。

変更には保存先を明示し、記録欠落を避ける。誤設定時は管理者が現在値とbefore.jsonを比較し、並行して行われた正当な変更を上書きしないよう復元対象を決める。GET結果にはPUT非対応のフィールドも含まれるため、before.jsonを無加工でPUTしない。復元後も再取得・差分確認・成功/失敗両経路の検証を行う。

今回は要件に合う設定が観測されているため、設計更新のためのGitHub書込は行わない。

## 5. 品質設定と再現性

Rust版と構成要素は `rust-toolchain.toml` を正本にする。channel=1.95.0、components=rustfmt/clippy/llvm-tools、profile=minimal。CIはtoolchain-inputs.shからchannel/componentsを導出して渡す。ローカルとCIの実際の版一致は別途ログで検証する。

unsafe_code=forbidはworkspace.lints.rustで定義し、各メンバーのlints.workspace=trueで継承する。tools/lintは独立クレートなので個別のlints.rustへ同じ禁止を宣言する。ルートに書いただけで全クレートへ適用済みとは判断しない。

カバレッジの正本はcoverage.shで、CI側も同じシードを宣言する。

| 項目 | 値・方式 | 検証 |
|---|---|---|
| 絶対床 | 90.0% | workspaceの計測値が床以上 |
| 相対許容差 | 0.01ポイント | head >= base - 0.01。base比較は変更提案時 |
| シード | PROPTEST_RNG_SEED=20260823 | CIとローカル・headとbaseで同じ値 |
| 明示除外 | modules/app/aidlc/src/main.rsの1ファイルのみ | 下記の式が計測へ渡り、他ファイルやクレートを除外しない |
| 再現性 | 同一コード・版・シードの2回測定 | 生のhead値と差を記録し、差0.00ポイントの受入目標への達否を判定 |

除外式は表の区切りと混同されないよう、表の外に記載する。

`(^|/)modules/app/aidlc/src/main\.rs$`

シード固定は必要条件であり、再現性の実証ではない。取得失敗・計測失敗・残る非決定性は未達として記録する。tools/lintはworkspace外で、90%床の対象には含めない。過去の暫定0.05・旧ロック試験由来の残差は履歴として保持し、現在の0.01と混同しない。

## 6. 論理コンポーネントと障害の影響範囲

| コンポーネント | 配置 | 障害の影響 | 手当て |
|---|---|---|---|
| CI検査とCI Success | ci.yml | 個別実行の不合格。共有設定の誤りは同じ定義を使う複数実行へ波及する | 原因の検査単位を識別し、設定修正後に対象版を再検証 |
| レビュー検査・再評価 | ci.yml、review-thread-resolution.yml、外部固定版 | 対象変更のマージ阻止。外部実装・共通設定の障害や誤検知は複数の変更へ波及し得る | 対象・コミット・入力と出力を照合し、結果の更新経路を確認 |
| audit | ci.yml | 監査失敗/未実行は可視化されるが単独ではマージを止めない | 脆弱性対応と外部取得失敗を区別し、両ロックファイルの結果を確認 |
| ruleset | GitHub | 誤設定は全対象マージの停止または誤許可を招く | 前後JSON、差分確認、必要時の復旧と両経路の実働検証 |
| ツールチェーン・lints | TOML、導出スクリプト | 主にRustを使うcheck/coverage/auditへ影響。全ジョブが同じ障害になるとは限らない | 正本と導出入力・継承先を確認 |
| カバレッジ | coverage.sh | coverage不合格からCI Success・マージ条件へ伝播 | 対象ファイル・閾値・比較条件・再現性を確認 |

共有資源はランナー・キャッシュだけではない。外部配布元、再利用ワークフロー、トークンの権限、リポジトリ設定も共有の依存・境界である。キャッシュがあることを取得成功や検査成功の根拠にしない。

## 7. 要求対応と検証計画

| 要求 | 設計箇所 | 検証方法 |
|---|---|---|
| NFR2.1 | §4 必須4コンテキストとキュー | 設定JSON、失敗時停止と全成功時完走 |
| NFR2.2 | §2 イベント別集約 | 各イベントの結果・取消・スキップ、レビュー再評価の反映 |
| NFR2.3 | §2 check | workspaceとtools/lintの各検査ログ |
| NFR2.4 | §5 同条件計測 | 生の2回測定値、差、head/baseの比較 |
| NFR2.5 | §5 除外と床 | 除外範囲と絶対ゲート |
| NFR4.1 | §2 audit | 両ロックファイルの実行・未実行・失敗の識別 |
| NFR4.2 | §5 正本と導出 | ローカル/CIのRust版一致 |
| NFR4.3 | §5 lints継承 | 全メンバーと独立クレートの宣言、unsafe不適合例の拒否 |
| NFR4.4 | §3 権限と外部コード | ジョブ別権限、固定版の一致、秘密情報の非出力 |
| NFR4.5 | §4 設定管理 | 前後JSON、変更対象・実行者・結果、復旧手順 |

設定確認にはverify-ci-governance.shを使う。その成功は設定の存在の証拠であり、全CI・カバレッジ2回測定・依存監査・キュー完走・レビュー再評価の実働成功を意味しない。今回の設計更新ではそれらを実施せず、後続の受入項目として残す。

上流要件のR-01（Markdown表2行の表示崩れ）は上流文書のレビュー所見として残る。本設計では正規表現を表の外へ置き、表示崩れを持ち込まない。上流所見を本設計の完了だけで解消扱いにしない。

## Assumptions & Open Questions

None.


## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T14:08:56Z
**Iteration:** 1
**Request Challenge:** review:8656c6b1637c9f1712c8de1943c19cf7

### Findings

advisory（承認判断の参考となる独立レビュー）として、現在の設計本文に新規所見はない。上流nfr-requirementsのR-01は別文書の未解決所見として保持し、この判定で解消扱いにしない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| - | - | - | No findings | No action required | Resolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| required-sections | PASS、H2=9、findings_count=0 | レビュー追記前のsecurity-design本文の構造を確認。 |
| upstream-coverage | PASS、未参照0 | 今回解決済みのsecurity-requirements・tech-stack-decisions・contract-summaryを指定し、security-design・traceabilityの成果物集合で検査した。 |
| traceability | PASS、gaps/orphans/invalid_targets等すべて0 | 対象UnitのNFR詳細要求と対応表に欠落・不正参照はない。 |
| 要求ID・設計節の独立照合 | PASS、10件 | 上流要求行、upstream_ids、coverageのID集合が一致し、各targetの設計節が本文に存在する。 |
| Bun.markdown.htmlによる表の描画確認 | PASS、本文6表 | HTMLへ描画した各表の全行がヘッダーと同じ列数（順に4/4/3/3/4/3列）。正規表現は表外にあり、上流R-01の表示崩れを持ち込んでいない。 |
| linter / type-check | 適用外 | 対象はMarkdown/JSON設計文書で、TS/JS等の実コード・対象スニペットはない。コード検査成功とは扱わない。 |
| 現行CI・設定との照合 | 一致 | CIの7ジョブ、3イベント別の集約条件、audit必須外、別ワークフローの再評価と権限、2呼出のSHA/ci_ref一致、4必須コンテキスト、strictとキュー設定を確認した。 |
| 品質設定・管理手順との照合 | 一致 | Rust 1.95.0と構成要素の導出、unsafe禁止の適用方法、シード20260823、0.01ポイント、90%床、main.rsのみの除外、rulesetスクリプトの比較・保存・送信項目と検査範囲を確認した。 |
| 上流境界との照合 | 一致 | U10はCI・設定管理のpackagingであり、製品外部契約C1/C2や製品の永続化責務を新たに所有していない。 |

### Summary

設計は10件の詳細要求を、具体的なCI定義・設定管理・復旧・受入方法へ対応付けている。レビュー状態の再評価と完了済みCI Successの更新を同一視せず、外部実装の未観測部分や実働未確認事項を区別しているためREADYとする。

本レビューではGitHub書込、全CI実行、依存監査、カバレッジ2回測定、キュー完走試験、外部再利用ワークフロー内部の検証を行っていない。それらは本文§7の後続検証として残る。
