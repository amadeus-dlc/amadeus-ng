# code-generation-questions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Code Generation（Construction 3.5）の質問票（Unit: U9、kind: spec）。出典: `code-generation-plan.md`、`unit-test-instructions.md`、`../functional-design/*.md`
> （`gap-measurement-20260907.md` 含む）、`../nfr-requirements/*.md`、`../nfr-design/*.md`。
>
> **質問なし。** ブランチ / PR / 記録コミットの運用は確定済み（PR は 1 本直列、squash-merge、レビューボット全件対応、`git add -A`）。前提 P1〜P3（2026-08-23、
> B4）は当時の確認として保持し、再走の前提 P4〜P8 を確認のうえ、計画承認（Plan Approval）を求める。

## 前提（確認事項）

- P1. 本 Unit はコードを書かない（`modules` / `tools` / `scripts` / `.github` / `Cargo.*` / `docs/specs/research` の diff ゼロ）。TDD の赤→緑は
  「受入検査（sentinel grep / diff / README 行数 / 表整形）を先に走らせて赤を記録し、改訂で緑にする」と読み替える。
- P2. 機能設計・NFR 要求・NFR 設計の pending-revision（BR2.5 の 12 号 5 箇所、BR1.5 §1b 再構成、BR5.1 の grep 範囲と sentinel 7 語、diff スコープ
  Cargo.* まで、StageGraphReader の扱い）は**計画に取り込んで**実施する（正本の本文はステージゲートの Request Changes で同期）。
- P3. 委任は 2 本（委任 1 = coding-rules / components.md / deviations.md、委任 2 = 仕様 01 / 10 / 11 / 12 号）、所有ファイルが重ならないため並行。
  モデルは両方 Opus。開発エージェントは計画・検査手順・本質問票を書き換えず、`developer-report-<n>.md` に報告する。仕様の規範名は U2 の承認済み
  改名（`WorkflowExecutionSnapshot` → `WorkflowExecutionState`、B5 で改名）を採り、現行コード名を括弧で注記する。

## 再走時の前提（2026-09-07 — functional-design 差し戻し後の U9 再走、Modify）

P1〜P3 のうち P2（B4 で実施済み）と P3（委任の境界・規範名 — `WorkflowExecutionState` は型ごと消滅）は本再走では失効し、以下に置き換わる。P1 は有効。

- P4. **基準はコード**: 「正しい姿」= `gap-measurement-20260907.md` §4 の台帳（1 件ずつコードで実否確認済み）と §4.8 の裁き。記録を転記しない。台帳の基準
  `02cacea2` と現行 `origin/main`（`e8ca4a5f`）の差は Step 0 の基線再実測で吸収し、差があれば code-summary とブリーフ補遺に書く。
- P5. **改訂対象**: coding-rules 20 行 9 ファイル（改訂 12 + 履歴マーカー 8）、仕様 4 号（deviations は触らない）、共有契約 3 本（components 全面 /
  contract-summary 節単位 / unit-of-work 注記のみ）、`decisions.md` ADR-010 :476-478 の失効注記。計画時実測で `unit-of-work.md:64` の ADR-010 注記
  （「集約の外へ」— B13 で再失効）にも同形の追記 1 行を加える（設計 P4 の 4 行 + 1、本文は書き換えない）。コード・`formal/`・`research/` は触らない。
- P6. **委譲**: 派遣 A（coding-rules 9 ファイル + 10 号 + 01 号、`developer-brief-3.md` / `developer-report-3.md`）と派遣 B（11 号 + 12 号 + 共有契約 3 本 +
  decisions.md、`developer-brief-4.md` / `developer-report-4.md`）、所有ファイル非重複で並行、両方 Opus。ブリーフには gap-measurement §2 の表を 3 列丸ごと、
  §4 台帳、security-design §2 全文、訂正 2 件（RMU 呼出に「同期」を使わない / ADR-010 は打消し線 + 失効注記）、報告の根拠列、禁止事項（push / PR /
  GitHub 書込 / `AIDLC_*` でのフック回避 / 書込スコープ外 / `git commit`）を書く。番号 1 / 2 は B4 の履歴として残す。
- P7. **受入検査は 10 項目**（`unit-test-instructions.md`）: コード diff 空 / sentinel 10 語 0 件（基線 44）/ README 22 = 22 + 索引ずれ 2 点解消 / 表整形 /
  deviations diff 空 / `## Review` 節 3 本のバイト同一 + decisions.md は追記のみ / 4 号の予定表記 / 実測表との突合 / 用語 / レビュー（CodeRabbit 全件 +
  ステージレビュー READY + CI 7 ジョブ）。(1)〜(9) を PR 前に、(10) を PR 後に実測して PR 本文へ貼る。
- P8. **ブランチ / PR**: 現行の `stage1-selfhost` で作業し、`main` へ 1 本の PR（squash、コミット名 = `u9-canon-docs`）。記録 + 文書を `git add -A` で回収して
  コミット。収束条件を最新 head で再実測してから merge queue へ（AI 裁定可 — オーナー包括承認 2026-08-29）。

## Plan Approval

`code-generation-plan.md`（埋め込みの Testing Contract を含む）と `unit-test-instructions.md` を確認し、文書改訂に進んでよいか。

[Approval Fingerprint]: sha256:e4d9ca1076803db31aebccd0b7fd330e5f4982415b6762d8bbada2d2f3fcb673

- Approve Plan — 計画どおり文書改訂に進む
- Request Changes — 計画を修正する

[Answer]: Approve Plan
