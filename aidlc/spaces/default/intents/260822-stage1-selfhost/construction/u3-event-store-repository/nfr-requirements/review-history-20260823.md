# review-history-20260823 — security-requirements.md 旧 Review 節の退避

> 2026-08-23T09:08:14Z の独立レビュー（advisory、iteration 1、READY: Major 2 / Minor 1）を、2026-09-07 の再走（Modify）で
> `security-requirements.md` を書き直す際に末尾から切り離して保存したもの。各所見の現行での扱いは本文の「Review 履歴」節を参照
> （#1 TOLERANCE 0.01 = 解消、#2 `indexing_slicing` / `panic` deny = 解消、#3 `audit` advisory 注記 = NFR4.1 に反映）。
> git 上の原本は `5ae731d8`（#29）の同ファイル末尾。

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T09:08:14Z
**Iteration:** 1（advisory, unit: u3-event-store-repository）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | security-requirements.md NFR2.3 / tech-stack-decisions.md §2 | `scripts/coverage.sh` の相対ゲート許容誤差 `TOLERANCE` は現在 0.05 に設定されており、そのインラインコメント（同スクリプト冒頭）は「U3 のロック退役（ADR-007）でジッタ源が消えたら 0.01 へ引き締める（NFR2.4）」と明記している（実測: `TOLERANCE=0.05` — ジッタ源は `fs_workspace_lock.rs:237` の並行テスト）。この 0.05→0.01 のタイトニングは team.md Testing Posture が「stage-1 スコープでシード固定により 0.01 へ引き締める」と確約した項目でもあり、まさに本 Unit（ロック退役の実行主体）の Bolt B5 で条件が満たされる。しかし NFR2.3 の合格基準は「`scripts/coverage.sh` が PASS（絶対 + PR 相対）」のみで、TOLERANCE の値そのものには触れていない。0.05 のまま実装しても gate は形式的に PASS してしまうため、このタイトニング作業が実装時に見落とされるリスクがある。 | NFR2.3（または新設の NFR2.x）に「ロック退役完了後、`scripts/coverage.sh` の `TOLERANCE` を 0.05 → 0.01 へ引き締め、該当コメントを更新する」ことを明示的な受け入れ基準として追加する。 |
| 2 | Major | security-requirements.md NFR4.3 | 要求は「範囲・減算（`seq_nr − 1`）・索引は事前検査し…`unwrap` / `expect` / `panic!` / `indexing_slicing` を生まない」ことを求め、合格基準を「clippy deny + レビュー」としている。しかし実測（`Cargo.toml` `[workspace.lints.clippy]`、45 行の全リストを確認）では `unwrap_used` / `expect_used` は deny 済みだが、`indexing_slicing` と `panic`（`clippy::panic`）はいずれも設定されていない（`grep -n "indexing_slicing\|panic" Cargo.toml` = 0 件）。`coding-rules/README.md` 自身が「オーナーの指摘は可能な限り機械的な強制へ落とし込む（型→既存 lint→`cargo lint` の優先順）」と明言しているにもかかわらず、この 2 パターンは「レビュー」という人力チェックのみに依存しており、BR1.3 の `event.seq_nr() − 1` のような u64 減算アンダーフロー（`[profile]` に `overflow-checks` の明示設定も無く実測、release ビルドでは黙って wrap しうる）や配列アクセスでの実際の panic を防ぐ機械的保証が要求の記述と一致しない。 | `indexing_slicing = "deny"` と `panic = "deny"`（clippy）を `[workspace.lints.clippy]` に追加することを本 Bolt（B5）のスコープに明記するか、機械強制を意図的に見送るならその理由を NFR4.3 に明記する。 |
| 3 | Minor | tech-stack-decisions.md §2 | `cargo audit` を CI `audit` ジョブの required status check 化していない点（`.github/workflows/ci.yml` の `audit` ジョブは `ci-success` の `needs` に含まれず advisory 扱い — コメントで意図的と明記）は本 Unit の設計ではなく既存の CI 設計判断であり問題ではないが、NFR4.1 の合格基準「`cargo audit` が CI で緑」は「緑でなくてもマージ可能」という現状の運用（advisory）と字面上ややズレる。 | NFR4.1 に「`audit` ジョブは advisory（required 化なし、`ci-success` からも除外済み）」という既存運用を一行注記し、誤読を防ぐ。 |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts --stage nfr-requirements` | PASS（`{"pass":true,"gaps":[],"orphans":[],...}`） | traceability.json の upstream_ids（NFR1〜5）と coverage が過不足なく対応 |
| `aidlc-sensor-required-sections.ts`（security-requirements.md） | PASS（h2_count=5） | §1〜§5 の H2 見出し検出、登録簿既定（≥2）を満たす |
| `aidlc-sensor-required-sections.ts`（tech-stack-decisions.md） | PASS（h2_count=3） | §1〜§3 の H2 見出し検出 |
| `aidlc-sensor-upstream-coverage.ts`（`--consumes functional-spec,rules,requirements,contract-summary`） | PASS（unreferenced=[]） | 4 つの上流成果物すべてが security-requirements.md 本文で参照されている |
| `linter` / `type-check` | 該当なし | 本ステージの成果物に TS/JS・TSX コードスニペットが無いため対象外（Rust/SQL のみ） |
| 実ファイル突合（`Cargo.toml` `[workspace.lints.clippy]`、`.gitignore`、`.github/workflows/ci.yml`、`scripts/coverage.sh`、`scripts/quint-gate.sh`、`rust-toolchain.toml`） | 概ね整合、2 件の乖離を Findings #1/#2 に記録 | `unsafe_code = "forbid"` / `permissions: contents: read` / `cargo audit` ジョブ / `.gitignore` の `.aidlc-*` パターンは既に存在し NFR4.1〜4.2・NFR1.1 の前提と一致（新規追加不要）。TOLERANCE と indexing_slicing/panic lint の 2 点のみ乖離 |

### Summary

要求は上流（requirements.md NFR1〜5、functional-design の BR1.x〜BR5.x、C3/C6、ADR-006/007）と広く整合し、traceability・required-sections・upstream-coverage の 3 センサーは全て PASS、STRIDE・データ分類・信頼境界の記述も本 Unit の規模（ローカル単一ユーザ・認証なし）に対して妥当である。一方で、実ファイル突合により 2 件の Major を検出した: (1) `scripts/coverage.sh` の TOLERANCE タイトニング（0.05→0.01）という、コード内コメントが名指しで本 Unit に紐付けている既存 TODO が要求に反映されていない、(2) NFR4.3 が「clippy deny」で防げると主張する `indexing_slicing` / `panic!` が実際にはワークスペース lint に存在しない。いずれも Critical ではなく（ビルド・CI を壊さず、実装完了の見落としリスクに留まる）、Major 2 件で advisory の READY 閾値（Critical 0 かつ Major ≤ 2）内に収まるため、Verdict は READY とする。ただし Bolt B5 の実装ゲートで上記 2 点への対応（または明示的な見送り理由の記載）を確認することを強く推奨する。
