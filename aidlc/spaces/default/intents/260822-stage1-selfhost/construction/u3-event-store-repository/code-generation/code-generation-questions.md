# code-generation-questions — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> Code Generation（Construction 3.5）の質問票（Unit: U3、kind: library）。**改訂履歴**: 初版 2026-08-23（Bolt B5、Q1 = A lint 昇格、
> P1〜P4、Plan Approval 2 回 — 指紋 `38d7646c…` → `04a8a9e1…`）→ **再走 2026-09-07（本版、Modify）**。旧版は
> `code-generation-questions-history-2026-08-23.md` に全文保存した。出典: `code-generation-plan.md`（2026-09-07）、`unit-test-instructions.md`
> （同）、`../functional-design/*.md`、`../nfr-requirements/*.md`、`../nfr-design/*.md`（いずれも 2026-09-07 再走 READY と各 `pending-revision.md`）。

## 以前の質問（2026-08-23 の記録 — 履歴）

Q1（`clippy::indexing_slicing` / `clippy::panic` の workspace lint 昇格）は **A（昇格する）** で確定し、B5 で実施済み（実測 `Cargo.toml:43-44`）。
前提 P1〜P4（委任 5 本・メメント改名・rusqlite / tokio の固定版・Quint DoD）は B5 で消化済みの履歴であり、本再走の前提ではない。

## 再走の前提（2026-09-07 — 確認事項）

- P5. **ワークスペース変更は Quint 凡例コメント 5 行だけ**: 再走した設計 3 段（機能設計 2026-09-05 是正 / NFR 要求 / NFR 設計、いずれも
  2026-09-07 READY）は現行コードの実測で書かれ、振る舞いに関わる不一致は見つかっていない。計画準備の照合で見つかった唯一の不一致は
  `formal/orchestration/journal_protocol.qnt` の凡例コメント（モデル型 ↔ Rust 対応表）が B12 以前の旧名 `WorkflowExecution` /
  `WorkflowExecutionRepository` を 5 行（`:10` / `:11` / `:15` / `:22` / `:23`）で使っていることで、これを現行名 `IntentExecution` /
  `IntentExecutionRepository` へ追従させる（BR5.1、functional-spec `:137`）。状態機械本体（var / action / 不変条件 / witness）・プロダクトコード・
  テスト・依存・scripts・CI は変更しない。それ以外は Unit 限定コマンドと受入の再実測、設計との照合表、`code-summary.md` / `traceability.json` /
  `source-manifest.json` の現行化（U10 の再走と同じ型）。
- P6. **新たな不一致が見つかったとき**: Red テスト案を先に報告し、計画変更（承認の取り直し）を受けてから Green にする。本計画を根拠に凡例 5 行以外の
  コードを直さない。
- P7. **凍結文書は触らない**: FD / NFR 要求 / NFR 設計の `pending-revision.md`（FD 11〜15 / NFR 4〜10 / ND 1〜6）はステージゲートの Request Changes
  経路で折り戻す。上流 `requirements.md:133-135`（NFR3 の `audit_lock.qnt` / `WorkflowExecution`）の改訂要否はオーナー裁定待ちとして code-summary
  §7 に載せる。
- P8. **委任と PR**: 委任 1 回（aidlc-developer-agent、Opus）、ブリーフは `developer-brief-9.md`、報告は `developer-report-11.md`。本 Bolt は
  ワークスペース差分（凡例 5 行）を持つので、Unit 完了後に Bolt 単位の PR（slug `b52-u3-event-store-repository`、直列運用、squash-merge）を開き、
  収束ルールで畳む。

## Plan Approval

2026-09-07 の対象: `code-generation-plan.md` の Step 1〜7 と、その Testing Contract、および `unit-test-instructions.md`。

ワークスペースの変更は `formal/orchestration/journal_protocol.qnt` の凡例コメント 5 行（旧名 `WorkflowExecution` / `WorkflowExecutionRepository` →
現行名 `IntentExecution` / `IntentExecutionRepository`）だけで、状態機械本体・プロダクトコード・テスト・依存・scripts・CI は変更しない。
Unit 限定コマンド 11 本と受入（fmt / clippy / `cargo lint` / `tools/lint` 自己テスト、quint-gate、coverage 2 回、`cargo audit` 2 件、退役 grep、
旧名 grep、`cargo test --workspace` 1 回）を実測し、設計 3 段の主張（検査点の関数・行番号・テスト名、コンポーネント配置と依存、件数）を現行コードで
照合して記録する。`code-summary.md` を現行の事実で書き直し（旧版は履歴として保存済み）、`traceability.json` の 46 件を実在ファイルのパス単体へ
対応付け、`source-manifest.json`（`writes` は上記 1 ファイル）を作る。新たな不一致が判明した場合は Red テスト案を先に返して計画を改訂する。

計画準備時（2026-09-07、`origin/main` = `f2b6b6a9` と同一コード）の実測は契約 22 / 実装固有 23 / 本家適合 10 / クラッシュ再構成 5 が PASS、退役
grep 0 件、旧名 grep は上記 5 行のみ。これは全 CI 実行・カバレッジ再測定・依存監査の成功を意味しない。

[Approval Fingerprint]: sha256:1b534890e8a9d47c4722b120d633d96680ff61fc23710d935604e08303468074

- Approve Plan — この計画で実コード生成（検証と記録の現行化）に進む
- Request Changes — 計画・テスト手順を修正する

[Answer]: Approve Plan
