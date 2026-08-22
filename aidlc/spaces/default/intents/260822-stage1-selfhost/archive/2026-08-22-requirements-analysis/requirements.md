# requirements — stage-1（セルフホスト切替）への最短経路

> Requirements Analysis（Inception 2.3）成果物。出典: Issue #7（intent 逐語 = `aidlc-state.md` **Project**）、
> `docs/specs/00-policy.md` §4（切替5条件の正本）、RE 成果物
> `aidlc/spaces/default/codekb/docs/business-overview.md` / `architecture.md` / `code-structure.md`、
> チーム実践 `../practices-discovery/team-practices.md`、確認質問
> `requirements-analysis-questions.md`（Q1〜Q5 回答済み・Looks correct 確認済み）。

## 1. Intent 分析

目標は機能追加ではなく**ブートストラップの段の切替**である: 現在は upstream AI-DLC（stage-0）を
ホストにこのリポジトリを開発しているが、**amadeus-ng 自身をホストにこのリポジトリの開発が回る**
状態（stage-1）に最短で到達する（`business-overview.md` の D8 ブートストラップ構図）。
「開発ワークフローを回す装置」自体が成果物のため、セルフホストが最良の受入テストになる。

検収の正本は 00-policy §4 の5条件。本 intent の Definition of Done（Q4・Q2 で確定）:

1. 本リポジトリで bugfix 相当の小 intent を1本、amadeus-ng バイナリをエンジンにして
   開始→ゲート承認→完了まで通す（実地スモーク。切替条件1・2・3 の統合受入を兼ねる）
2. `--doctor` 自己診断 green（条件4）
3. CI green（条件5 — 既達を維持）

## 2. 機能要求（FR）

各 FR は Issue #7 のクリティカルパス項目・インタビュー回答・チーム実践へ遡及する。
実装順序・Bolt 分割は delivery-planning が決める（ここでは順序を規定しない）。

### FR1 — 監査台帳と audit-first 結合【条件2 / Issue 項目2 残件 / スライス B-1】

- FR1.1 監査台帳の append（append-only・O_APPEND、シャード = `<record>/audit/<host>-<clone>.md`）と
  位置付き読取（シャード横断の順序規約 = timestamp ソート + バッファ位置 tiebreak）を実装する。
  合格 = 0a 採取済みの逐語契約（EVENT_HEADINGS 86 語・FIELD_ORDER）との一致テスト green。
- FR1.2 audit-first 遷移（B9: 監査行を書いてから状態を書く。状態はキャッシュ、真実源は監査）を
  ロック区間（既存 `WorkspaceLock`/`LockProtocol`）と結合する。合格 = `audit_lock.qnt` ITF 準拠維持。
- FR1.3 `AuditLedgerRepository`（集約名 + Repository 規約）を設計・実装する。B-1 冒頭で
  AuditLedger の位置づけ（peer 集約 vs イベントログ）を裁定する（→ §7 Open questions）。

### FR2 — report ユースケース【条件1 / Issue 項目3-B】

- FR2.1 `report` の遷移コミット（approve / reject / revise / skip / awaiting-approval / resumed）を
  ユースケースとして実装する。合格 = 0a 抽出済み契約マップとの一致 + ITF 準拠（engine_loop）維持。
- FR2.2 report_dispatch + B10 述語（ゲート受理の最小前提）+ verification モジュール最小面を実装する。

### FR3 — next ユースケース と Continue【条件1 / Issue 項目3-C】

- FR3.1 `next` の 21 分岐ラダーを実装する。合格 = 抽出済み契約マップの分岐網羅テスト green。
- FR3.2 load-steering 分割配信と `continue_token`（正準 JSON + ハッシュ — FR7 に依存）・`continue` 動詞を
  実装する。
- FR3.3 着手前に next_decision の層配置裁定を得る（→ §7 Open questions。domain-design で扱う）。

### FR4 — マルチコール CLI と文言カタログ配線【条件1 / Issue 項目4】

- FR4.1 マルチコールバイナリ `aidlc` のディスパッチャ（ROUTES 表、逸脱台帳 #1 のコマンド綴り写像）を
  実装する。合格 = 0b の CLI 実行出力ゴールデン（FR7）との突き合わせ green。
- FR4.2 `message-catalog` の逐語文言を CLI 出力面に配線する（LLM の分岐条件になる文言はバイト一致）。

### FR5 — 最小フック4本【条件1・2 / Issue 項目5】

- FR5.1 Stop forwarding loop フック。FR5.2 HUMAN_TURN 記録フック。FR5.3 state-transition guard。
  FR5.4 write-audit-log。いずれも upstream の観測可能契約（発火条件・出力・ブロック挙動）互換。
  合格 = 0b ゴールデンとの一致 + 実地スモークでの実働。

### FR6 — doctor サブセットとドッグフード切替【条件4 / Issue 項目6】

- FR6.1 `--doctor` サブセット（stage-1 で必要な検査項目）を実装する。合格 = 本リポジトリで green。
- FR6.2 DoD の実地スモーク（§1）を実施し、Issue #7 を close する。

### FR7 — canon-json 実装と 0b ゴールデン採取【FR3/FR4/FR5 の依存 / Q3 で取込確定】

- FR7.1 upstream ツールを bun で実行し **hash-canonical 受入表**（ADR 0001 — 実入力に対する実ハッシュ出力）を
  採取・コミットする。
- FR7.2 upstream CLI（next/report ほか）の**実行出力ゴールデン**（stdout JSON・状態ファイル差分・監査行）を
  採取・コミットする。
- FR7.3 `canon-json` クレートを実装する。合格 = FR7.1 受入表の全行一致。

### FR8 — 設計監査の土台整備【Q1 で範囲確定】

- FR8.1 A束: canon 語彙の自己矛盾修正（`coding-rules/use-case-rules.md:38` の `repository.load()` →
  `find()`、`gateway-taxonomy.md` §4 の「load / save」散文）。
- FR8.2 B束: 仕様の canon 追従（11号 §2.3/§3 ポート・供給面表、01号 §3 集約候補表、
  10号 §3「同上」、10/12号の PlanAction・CheckboxState 所有一意化、12号 §2.3/§5/§39 整合）。
- FR8.3 C束のうち裁定 R1: `PlanAction` の所有を workflow_definition へ移動（orchestration は re-export）。
- FR8.4 C束のうち裁定 R2: 有効プラン畳み込みを orchestration 側ドメインサービスへ移設し、
  `WorkflowDefinition` にはグリッド照会のみ残す。
- 残りの C束（C17〜C33 ほか）は本 intent に**含めない**（→ §6 スコープ外）。

### FR9 — CI・ガバナンス整備【practices-discovery Q4〜Q8 で確定】

- FR9.1 `main` の branch protection（required checks: check / quint / coverage）を設定する。
- FR9.2 サプライチェーン4件: `cargo audit` CI 追加（tools/lint の独立 Cargo.lock 含む）・
  `rust-toolchain.toml` 固定・`unsafe_code = "forbid"` の workspace lints 昇格・
  CI `permissions: contents: read` 明示。
- FR9.3 `tools/lint` への CI 3ステップ追加（fmt / clippy / 自己テスト — 監査 C27）。
- FR9.4 PBT シード固定によりカバレッジ相対ゲート許容を 0.5pp → 0.01 へ引き締める。
- FR9.5 カバレッジ除外を `scripts/coverage.sh` に追加する（composition root = `main.rs` 配線部のみ）。
- FR9.6 エラーハンドリング様式規則（手実装エラー enum・thiserror/anyhow 不使用）の文面を起草し、
  オーナー確認のうえ coding-rules 正本へ 1 ファイル追加する。

## 3. 非機能要求（NFR）

- NFR1 **upstream 互換（D6 範囲）**: 観測可能な契約 — ワークスペースレイアウト・監査イベント語彙 86 語・
  CLI 語彙・`AIDLC_*` 環境変数・LLM 分岐条件の逐語文言 — を維持する。仕様レベルの逸脱は
  `docs/specs/deviations.md` への台帳登録のみ許す（現行 3 件 + 予約 1 件）。
  合格 = ゴールデンパリティ・逐語一致テスト green。
- NFR2 **品質ゲート維持**: CI 3ジョブ green・カバレッジ絶対 90% 床（main.rs 配線部のみ除外可）・
  TDD（`team.md` Testing Posture: Methodology tdd / Ordering 確定文言）・テストピラミッド定性配分。
- NFR3 **監査完全性（B9）**: クラッシュ後は監査台帳から状態を再構成できる（状態ファイルはキャッシュ）。
  合格 = audit-first 原子性の ITF 準拠 + クラッシュ再構成テスト。
- NFR4 **セキュリティ/サプライチェーン**: `unsafe_code` forbid（workspace）、`cargo audit` clean、
  ツールチェーン固定、least-privilege CI（FR9 の受入と同一）。
- NFR5 **性能（非目標の明示）**: 数値目標は立てない（Q5）。定性基準 =「体感で upstream と同等以上」。
  明確な劣化が観測されたら課題化する。

## 4. 制約

- C1 upstream ピン `3c3146cf`（v2.6.40）。ステージ資産（33 ステージ・エージェント・プロトコル・
  コンパイル済みグラフ）は upstream `dist/claude/` を**そのまま**使い、ステージ類は書かない（D6 配当）。
- C2 クリーンアーキテクチャ（層 = クレート、依存内向き強制）と coding-rules 正本
  （`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`、`cargo lint` 強制）を遵守する。
- C3 進め方: PR は Bolt 単位・squash-merge・直列運用。GitHub Issue = intent 1:1。
  Walking Skeleton は作らない（skeleton: off — `team.md`）。
- C4 人間可読成果物・仕様は日本語正本（D7）。コード識別子・固定トークンは英語。

## 5. 前提（Assumptions）

- A1 upstream `dist/claude/` 資産は Claude Code ハーネス上でそのまま動作する（stage-0 で実証中 —
  本ワークフロー自体がその実働証拠）。
- A2 実地スモークは gated モードで行う（swarm / 自律 Bolt は不要 — Issue スコープ外宣言と整合）。
- A3 0b 採取は現リポジトリの bun + upstream ツールで再現可能（AI-DLC 導入済み）。

## 6. スコープ外（本 intent で扱わない）

Issue #7 明示分: swarm / Bolt 自律実行、センサー・プラグイン・他 6 ハーネス・配布一般化、
OTel 配線、インストーラ、12/13 号仕様の全文執筆。
本ステージで追加確定分: C束の残り（C17〜C33 ほか — 後続 intent）、macOS CI ジョブ・main への
push トリガー（practices Q7 不採択）、配布時 Deployment Pipeline / Execution の定義・SBOM・
provenance（配布 intent で扱う — `team.md` Deployment）。

## 7. Open questions（後続ステージへの引き継ぎ）

- O1 next_decision の層配置裁定（FR3.3 — domain-design で扱う。Issue 項目 3-C の前提裁定）
- O2 AuditLedger の位置づけ: peer 集約 vs WorkflowExecution のイベントログ（FR1.3 — B-1 冒頭裁定。
  設計監査 E束）
- O3 StateFile 所有の一本化（WorkflowExecution が集約ルート、StateFile は媒体 — B-2 設計時）
- O4 エラーハンドリング様式規則の文面確定（FR9.6 — オーナー確認）
- O5 codekb フィンガープリントの repoDir 衝突（リポジトリ名 = サブディレクトリ名）の upstream 報告要否

## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-08-22T05:24:42Z
**Iteration:** 1

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | FR1.3, FR2.2, FR3.2, FR3.3, FR4.2, FR6.2, FR7.1, FR7.2, FR8.1–8.4, FR9.1–9.6 | 一貫性に欠ける — 同じ文書内で FR1.1/FR1.2/FR2.1/FR3.1/FR4.1/FR5/FR6.1/FR7.3 は「合格 = …」で明示的な pass/fail 基準を持つが、上記の項目群にはそれが無い。inception ガードレール「each requirement must have a clear pass/fail criterion」に照らすと、これらは QA がテストを書けない状態のまま残る。特に FR3.3 は「次決定の層配置裁定を得る」という決定タスクであり、実装可能な要求というより前提条件の解決待ちに見える（§7 O1 への参照は適切だが、FR としての合格基準が無い） | 各項目に一行で良いので「合格 = …」を追記する。FR8/FR9 のように成果物が具体的な設定変更・ドキュメント修正の場合は「合格 = <ファイル>に<変更>が反映され、レビューで確認できる」程度で足りる。FR3.3 は要求ではなく前提解決タスクである旨を明示するか、O1 解決後に FR として具体化する運びを注記する |
| 2 | Minor | §7 O5 | 「codekb フィンガープリントの repoDir 衝突」という Open Question の出所が、本ステージが読める上流成果物（business-overview / architecture / code-structure / team-practices / Q&A）のいずれにも見当たらない（`fingerprint` という語は `reverse-engineering-timestamp.md` に1箇所あるのみで、repoDir 衝突には触れていない） | O5 の出所（RE 作業中の実地観察か、別セッションでの発見か）を一行付記する。出所不明のまま残すと、後続ステージが検証不能な前提として引き継いでしまう |
| 3 | Minor | §1 vs FR6.2 | 00-policy §4 の切替条件3（「自プロジェクト開発で使うスコープのステージ一式が揃っている」）は §1 の DoD 項目1（「条件1・2・3の統合受入を兼ねる」）で解釈は示されているが、対応する FR（FR6.2 — DoD の実地スモーク実施）自体には条件3のタグが付いていない（FR6 見出しは【条件4】のみ） | FR6.2 の行、または見出しに「条件3」を明示的に併記し、5条件と FR の対応表が本文だけで完結するようにする |

### Summary

出典の引用・Q1〜Q5 回答の反映・Issue #7 クリティカルパスとの対応・ID 規約はいずれも正確で、スコープ境界（§6）も明快。主な改善点は FR8/FR9 を中心とした一部項目に明示的な pass/fail 基準（「合格 = …」）が欠けている点で、これは QA のテスト作成を妨げる可能性があるため人間の承認前に一読を勧める。ブロッキング（Critical）な欠落は見当たらず、READY と判断する。
