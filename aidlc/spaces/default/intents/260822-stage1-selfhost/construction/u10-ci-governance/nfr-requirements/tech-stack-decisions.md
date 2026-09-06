# tech-stack-decisions — U10 CI・品質管理（`u10-ci-governance`）

> 2026-09-06改訂。既存設定と確認済み要約を記述し、過去の導入予定・暫定値を現行値へ整合する。

## Sources

- [Q1] `nfr-requirements-questions.md` の2026-09-06確認要約（Looks correct）。
- [requirements] `security-requirements.md` のNFR2.1〜2.5 / NFR4.1〜4.5、`../../../inception/requirements-analysis/requirements.md` のFR9.1〜9.5。
- [contracts] `../../../inception/contract-design/contract-summary.md`。U10は製品の外部契約を所有しない。
- [local] `.github/workflows/ci.yml`、`scripts/coverage.sh`、`rust-toolchain.toml`、`Cargo.toml`、`tools/lint/Cargo.toml`、`scripts/governance/`（リポジトリルート基準、2026-09-06読取）。
- [observed] `../ruleset-observed-20260906.json`、[history] `../code-generation/superseding-decisions.md`。

## 1. 選定

| 領域 | 現行の選定 | 理由・境界 | 不採用案・注意点 |
|---|---|---|---|
| マージの機械強制 | ruleset「main」、必須check/quint/coverage/CI Success、strict=true、bypassなし | 既存のSQUASH/ALLGREEN/同時1件のキューと削除・force push防止を維持する（NFR2.1） | classic branch protectionを重ねて二重管理しない |
| イベント別検査 | pull_request/merge_group/workflow_dispatch。concurrencyはworkflow名とrefで分離 | 変更提案とキューの検査が衝突しない。coverageの相対比較は変更提案時、他は絶対ゲート（NFR2.2） | キュー無効化や、全イベントに同じbase refを仮定する案は採らない |
| CI Success | aidlc-distribution/check/quint/coverageをsuccess必須とする。review-thread-resolutionは変更提案でsuccess、他イベントでskippedを受理 | 必須4コンテキストを増やさず配布物の同期・回帰とレビュー検査を集約する（NFR2.2） | auditは集約しない。失敗・取消を成功へ読み替えない |
| 外部レビュー検査 | `j5ik2o/ci/.github/workflows/review-thread-resolution.yml@9cf0e9a8cd74c72de704763025003ed3b7608c65`。ci_refも同じSHA | 未解決スレッドを検査する既存方針。外部コードと追加権限の信頼境界を明示する（NFR4.4） | 浮動参照や「全ジョブ読取専用」という説明を採らない |
| Rust | 1.95.0、rustfmt/clippy/llvm-tools、profile=minimal | rust-toolchain.tomlを正本とし、toolchain-inputs.shがchannel/componentsをCIへ渡す（NFR4.2） | stableという浮動版を指定しない。dtolnay/rust-toolchain@master自体はSHA未固定 |
| 依存検査 | auditジョブでcargo auditをworkspaceとtools/lintの両Cargo.lockへ実行 | 対象を明記し結果を可視化。advisoryとして必須チェック外に置く裁定を維持（NFR4.1） | 全依存監査成功・マージ阻止を、ジョブの存在だけで主張しない |
| unsafe禁止 | workspace.lints.rustとtools/lintの個別lints.rustでforbid | 独立クレートを含め適用する（NFR4.3） | クレート個別attributeだけに依存しない |
| 権限 | workflow既定contents: read。レビュー検査のみchecks/statuses: writeとissues/pull-requests: readを追加 | トークンを秘密情報として扱い、追加権限を限定（NFR4.4） | workflow全体をwriteへ広げない |
| tools/lintの品質 | manifest-path指定のfmt/clippy/testをcheckに含める | workspace外の独立クレートを明示検査（NFR2.3） | workspace検査や90%床に自動で含まれるとは扱わない |
| カバレッジ | 絶対90%、相対許容0.01ポイント、除外は `(^|/)modules/app/aidlc/src/main\.rs$` のみ | 配線ファイルだけを除き、その他のworkspace計測対象を維持（NFR2.5） | クレート単位除外は採らない |
| 性質検証の乱数 | PROPTEST_RNG_SEED=20260823をCIとcoverage.shで統一 | ランダム経路を固定し計測再現性を検証する（NFR2.4） | 過去の暫定0.05を現行値としない。シード固定だけで差0.00達成とはしない |
| 形式検証・配布検証 | Node 22/Quint 0.32.0でquint-gate.sh、Bun 1.3.13で配布同期・回帰試験 | 既存のCI検査範囲を維持する | 新たなクラウド資源やAWS Bedrockは導入しない |

## 2. 依存と変更の範囲

今回変更するのは要件・技術選定・対応表の3成果物のみ。Rust依存、lockファイル、CI設定、rulesetを新規作成・更新する作業ではない。

Action参照版の固定状況は次のとおりである。

- aidlc-distributionのcheckoutは `11d5960a326750d5838078e36cf38b85af677262`、setup-bunは `0c5077e51419868618aeaa5fe8019c62421857d6` に固定されている。checkoutはpersist-credentials=false。
- 外部レビュー検査は表のSHAへ固定されている。
- 他ジョブにはactions/checkout@v4、actions/setup-node@v4、Swatinem/rust-cache@v2、taiki-e/install-action@v2、dtolnay/rust-toolchain@masterが残る。全件SHA固定は本intentでは採用していない。
- Dependabot（github-actions/cargo）の導入は既存裁定で見送り。手動の変更提案による依存・参照版更新を維持する。

`scripts/governance/ruleset-required-checks.sh` は既存設定を保持しながら4コンテキストの集合とstrictを確認・補正する手順である。実際の変更時には前後JSONと結果を保存する。今回の要件更新ではこの書込処理を実行しない。

## 3. 確定事項と後続の検証

シード20260823、相対許容0.01ポイント、main.rsのみ除外、イベント別coverage、ruleset管理スクリプトの配置は確定済みであり、未決の技術選定として再掲しない。

残るのは受入の実測である。カバレッジ2回測定の差、CI上のRust版一致、両Cargo.lockの依存監査、マージキュー成功・失敗の実働を検証し、対象版と結果を保存する。今回読んだ設定・過去の実績を、現在版に対する新しい試験結果として扱わない。暫定0.05と旧ロック試験由来の残差は履歴に残し、今回承認された0.01への要件を覆さない。

## Assumptions & Open Questions

None.
