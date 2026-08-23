<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T11:56:32Z — [u1-canon-json-goldens] 質問ゼロで前提 3 点（セキュリティ設計・論理コンポーネント・障害ドメイン）の確認のみ; kind = library で performance/scalability/reliability/observability の要求が無く、成果物は security-design / logical-components / traceability。NFR 要求レビューの Minor 1（深さ上限 128 の互換影響）は P1 に『実測深さの棚卸し』として引き取った

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->

### U1 レビュー結果（2026-08-22）

- aidlc-architecture-reviewer-agent、iteration 1、advisory: **READY**（Critical 0 / Major 0 / Minor 2）。
- Minor 1: `inception/domain-design/components.md` の CanonJson `external_dependencies: []` が
  sha2 / serde / serde_json の実依存と食い違う — code-generation 計画の棚卸し項目に追加し、実体化後に
  components.md を更新する（繰り延べ）。
- Minor 2: security-design.md §4 の serde_json 直接呼び出し禁止の記述が「直列化関数」に限定され、
  BR1.7 の「契約経路の `to_value`」への言及が抜けている — レビュー凍結後のため本文は触らず、
  code-generation 計画に禁止範囲（直列化関数 + 契約経路の to_value）を明記して引き取る（繰り延べ）。
- 凍結（review-freeze）後に成果物は変更していない。
- 2026-08-22T17:25:00Z — [Interpretations] U10（packaging）の nfr-design: produces は security-design + traceability の 2 つ（logical-components は produces 外）— 論理コンポーネント（CI ジョブ・スクリプト・ruleset の境界と障害ドメイン）は security-design 内の節に置く。`audit` ジョブを required checks に含めない判断（外部 advisory DB の一時障害で全マージが止まるのを避ける）を前提 P1 として人間確認へ
- 2026-08-22T17:28:00Z — [Interpretations] proptest 1.11.0 のソース（config.rs:40 `PROPTEST_RNG_SEED`、`RngSeed::Fixed(u64)`）で環境変数によるシード固定が実在することを確認 — NFR2.4 の決定化は `scripts/coverage.sh` と CI で `PROPTEST_RNG_SEED` を固定値に設定するだけで足り、テストコードの変更は不要（security-design §4 の第一候補 (a) が成立）
- 2026-08-22T17:31:00Z — [Open questions] U10 nfr-design レビュー READY（Minor 2）: (1) カバレッジ除外 regex の表記が tech-stack-decisions（`^` アンカー + 相対パス基準）と設計で逐語不一致 → code-generation 計画で正本を 1 つに確定（`modules/app/aidlc/src/main\.rs$`、cargo llvm-cov の相対パス基準） (2) ruleset 冪等スクリプトは規則タイプの有無だけでなく required コンテキスト集合の一致で収束判定する → 計画に反映

## 2026-08-23T00:30Z — U10 nfr-design-questions.md を確認済みバイトへ復元
- PR #25 レビュー指摘の引き取り（ecb2307）で人間確認済みの質問ファイルを書き換えてしまい、エンジンが「確認後に変更」を検出してステージ完了を拒否した。
- 人間が確認したバイト（0f3a151）へ復元。訂正内容（FR9.6 は U9 の責務 / 日付 UTC 表記）は `u10-ci-governance/code-generation/superseding-decisions.md`（#6 ほか）と本日誌が正本。
- 学習候補: 人間確認済みの questions ファイルは訂正対象にせず、訂正は superseding-decisions / 日誌へ書く。
- 2026-08-23T01:50:00Z — [Interpretations] [u2-domain-es-core] kind = library のため produces は security-design / logical-components / traceability。U2 固有の NFR 設計質問は無し — 検査点 3 か所（decide / apply_event / from_snapshot）+ next_decision の definition_id 検査、core-domain 内のモジュール分割（orchestration / workflow_definition、private mod + ファサード pub use）、テスト配置を前提 P1〜P4 として人間確認へ
- 2026-08-23T01:55:00Z — [Open questions] [u10-ci-governance] 回復レビュー（iteration 2）NOT-READY、Major 3 / Minor 1: review-thread-resolution / ci-success ジョブ・required checks 4 コンテキスト・ジョブ個別権限（checks/statuses: write）・外部再利用ワークフロー（SHA 固定）の信頼境界が security-design に未反映（オーナー指示 #9 は凍結後の追加）。オーナー指示「修正してレビューは是正して」に従い本文を実態に同期し、再レビューはステージゲートの Request Changes 経路で行う（回復枠は消費済み）
- 2026-08-23T02:05:00Z — [Deviations] [u10-ci-governance] 回復レビュー所見の本文同期を試みたが review-freeze フック（終端受領の凍結）が produces への書込を拒否 — 設計どおりゲートの Request Changes 経路で是正する。編集案は `u10-ci-governance/nfr-design/pending-revision.md`（nfr-requirements 分も同名ファイル）に保存し、ゲート差し戻し直後に適用してレビュアーを再実行する
- 2026-08-23T02:20:00Z — [Open questions] [u2-domain-es-core] レビュー iteration 1 READY（Major 2 / Minor 2）: (1) C4 の NotFound は GraphReadError に存在しない新変種 — B3 範囲に追加 (2) ADR-008 Decision (3) が start にも id 検査を要求し BR2.6 と矛盾 → ADR-008 を訂正（start は記録のみ） (3) logical-components の「既存」行: checkbox は workspace コンテキスト、Status は inline (4) NotStale は stale_report のもの。本文は凍結中のためゲートの Request Changes 経路で適用（pending-revision.md）
