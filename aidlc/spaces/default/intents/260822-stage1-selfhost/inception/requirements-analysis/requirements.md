# requirements — stage-1（セルフホスト切替）への最短経路

> Requirements Analysis（Inception 2.3）成果物。出典: Issue #7（intent 逐語 = `aidlc-state.md` **Project**）、
> `docs/specs/00-policy.md` §4（切替5条件の正本）、RE 成果物
> `aidlc/spaces/default/codekb/docs/business-overview.md` / `architecture.md` / `code-structure.md`、
> チーム実践 `../practices-discovery/team-practices.md`、確認質問
> `requirements-analysis-questions.md`（Q1〜Q5 回答済み・Looks correct 確認済み）。
> **改訂 2026-08-22（Q6）**: domain-design の ADR-001/003/004/007（イベントソーシング採用・SQLite ストア・
> WorkflowExecution 集約ルート・ロック機構退役）に合わせ FR1.1〜1.3・FR3.3 注記・NFR1 注記・NFR3・§7 O1〜O3 を
> 改訂した（units-generation からの後方ジャンプ。改訂前の原本は
> `../../archive/2026-08-22-requirements-analysis/requirements.md`）。ゲートのレビュー所見（Major 1 / Minor 1）を
> 受け FR8.1（gateway-taxonomy 正本修正 2 点の同梱）と FR3.3（合格基準の具体化）を追補。それ以外は初版のまま。

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

### FR1 — 監査台帳（イベントジャーナル）と audit-first 結合【条件2 / Issue 項目2 残件 / スライス B-1】

> 2026-08-22 改訂（ADR-001/003/004/007 整合）: 台帳の真実源は SQLite ジャーナル、upstream 互換の監査シャードは
> ReadModelUpdater の投影（リードモデル）。旧文言（mkdir ロック区間との結合・`AuditLedgerRepository`）は廃止。

- FR1.1 監査シャード（`<record>/audit/<host>-<clone>.md`、append-only）を ReadModelUpdater の投影として生成し、
  位置付き読取（シャード横断の順序規約 = timestamp ソート + バッファ位置 tiebreak）を実装する。台帳本体は
  SQLite ジャーナル（ADR-003）。合格 = 投影出力が 0a 採取済みの逐語契約（EVENT_HEADINGS 86 語・FIELD_ORDER）
  と一致するテスト green。
- FR1.2 audit-first 遷移（B9: ジャーナルへイベントを書いてから投影を描く。投影はキャッシュ、真実源は
  ジャーナル）を SQLite Tx + 楽観 version（条件付き書込 — ADR-007。mkdir ロック機構 `WorkspaceLock` /
  `LockProtocol` は退役）と結合する。合格 = 改訂版 `audit_lock.qnt`（ジャーナル / スナップショット / version /
  チェックポイント協定: version 競合拒否・チェックポイント単調性・投影冪等性）の ITF 準拠。
- FR1.3 `WorkflowExecutionRepository`（集約名 + Repository 規約。ES 形 store / find_by_id — ADR-004/006）を
  設計・実装する。AuditLedger は peer 集約ではなく WorkflowExecution のイベントログ（ADR-001/003 で裁定済み、
  旧 O2 は close）であり、旧称 `AuditLedgerRepository` は採用しない。合格 = store → find_by_id の
  ラウンドトリップ（最新スナップショット + seq_nr 以降の replay）テスト green。

### FR2 — report ユースケース【条件1 / Issue 項目3-B】

- FR2.1 `report` の遷移コミット（approve / reject / revise / skip / awaiting-approval / resumed）を
  ユースケースとして実装する。合格 = 0a 抽出済み契約マップとの一致 + ITF 準拠（engine_loop）維持。
- FR2.2 report_dispatch + B10 述語（ゲート受理の最小前提）+ verification モジュール最小面を実装する。（B10 述語の射程はレシートの**鮮度のみ** — オーナー裁定 2026-09-02、#51 = A。凍結検査は後続 intent）

### FR3 — next ユースケース と Continue【条件1 / Issue 項目3-C】

- FR3.1 `next` の 21 分岐ラダーを実装する。合格 = 抽出済み契約マップの分岐網羅テスト green。
- FR3.2 load-steering 分割配信と `continue_token`（正準 JSON + ハッシュ — FR7 に依存）・`continue` 動詞を
  実装する。
- FR3.3 着手前に next_decision の層配置裁定を得る（→ §7 O1。domain-design の ADR-002 で裁定済み:
  `WorkflowExecution` のクエリメソッド）。合格 = `next_decision` が `WorkflowExecution` の `&self` クエリメソッド
  （`(&self, &WorkflowDefinition, ...) → NextDecision`）として実装され、ユースケース層（`core-use-case`）に
  21 分岐ラダーの判断ロジックが存在しないことをコードレビューで確認（`cargo lint` の候補ルール化は後続）。

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
  `find()`、`gateway-taxonomy.md` §4 の「load / save」散文）。加えて ADR-006 が同修正に同梱すると指示した 2 点:
  `gateway-taxonomy.md` §2b の許容動詞一覧に **ES Repository の拡張語彙 `store`**（event-store-adapter-rs 同形。
  §2b はステートソーシング Repository の規則であり、ES Repository の動詞は本家ライブラリの語彙に従う旨を注記）
  を追加し、同 §3 の実例表から旧称 `AuditLedgerRepository` を除去する（AuditLedger はイベントログ —
  ADR-001/003、FR1.3）。合格 = 上記 4 点が正本に反映され、`coding-rules/README.md` の一覧と矛盾しないことを
  レビューで確認。
- FR8.2 B束: 仕様の canon 追従（11号 §2.3/§3 ポート・供給面表、01号 §3 集約候補表、
  10号 §3「同上」、10/12号の PlanAction・CheckboxState 所有一意化、12号 §2.3/§5/§39 整合）。
- FR8.3 C束のうち裁定 R1: `PlanAction` の所有を workflow_definition へ移動する。完全移動とし、
  `orchestration` からの再輸出は置かず、呼出側の参照パスを同一 Bolt で一斉修正する（2026-08-22 の再エクスポート
  禁止裁定 — `coding-rules/module-visibility.md` 追補 / ADR-005 改訂。承認後の文言訂正、監査台帳に記録）。
  合格 = `orchestration` に `PlanAction` の定義・再輸出が無く、全参照が `workflow_definition::PlanAction` を指し
  CI 3 ジョブ green。
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
  `docs/specs/deviations.md` への台帳登録のみ許す（現行 3 件 + 予約 1 件。ADR-003/007 により
  「SQLite ファイルの追加・ロック dir 非生成・upstream 互換ファイルはリードモデルとして維持」の登録を追加する）。
  合格 = ゴールデンパリティ・逐語一致テスト green。
- NFR2 **品質ゲート維持**: CI 3ジョブ green・カバレッジ絶対 90% 床（main.rs 配線部のみ除外可）・
  TDD（`team.md` Testing Posture: Methodology tdd / Ordering 確定文言）・テストピラミッド定性配分。
- NFR3 **監査完全性（B9）**: クラッシュ後は SQLite ジャーナルから集約 `WorkflowExecution` を再構成でき、
  upstream 互換ファイル（状態ファイル・監査シャード）は投影で冪等に再生成できる（ADR-003/004 — 状態ファイルは
  リードモデル）。合格 = 改訂版 `audit_lock.qnt` の ITF 準拠 + クラッシュ再構成（ジャーナル → 集約 → 投影）テスト。
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

- O1 （解決済み・close）next_decision の層配置 — ADR-002: `WorkflowExecution` のクエリメソッド（FR3.3）
- O2 （解決済み・close）AuditLedger の位置づけ — ADR-001/003: WorkflowExecution のイベントログ（FR1.3）
- O3 （解決済み・close）StateFile 所有 — ADR-004: `WorkflowExecution` が集約ルート、状態ファイルはリードモデル
- O4 エラーハンドリング様式規則の文面確定（FR9.6 — オーナー確認）
- O5 codekb フィンガープリントの repoDir 衝突（リポジトリ名 = サブディレクトリ名）の upstream 報告要否

## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-08-22T09:11:20Z
**Iteration:** 1（ゲート差し戻し後の再レビュー・advisory）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Major | FR8.1 | 追補文が「同 §3 の実例表から旧称 `AuditLedgerRepository` を除去する」と述べているが、`gateway-taxonomy.md` を実測すると `AuditLedgerRepository` は §3（「ポート造語（Store / Reader / Writer / Source / Provider）は禁止」— この節に該当語は登場しない）ではなく、**§2「Repository 名 = 集約名 + Repository」の実例リスト**（`AuditLedger` → `AuditLedgerRepository` の行）に存在する。ADR-006 Consequences（decisions.md:111）自体は節番号を指定していないため、この誤りは追補時に FR8.1 側で新たに持ち込まれたもの。この誤り自体は次段の実装・検証を妨げない（`AuditLedgerRepository` は正本内で一意な識別子であり grep で特定できる）が、「§3」という誤った節番号のまま合格基準の一部として正本に残ると、レビュー担当者が §3 を見て「無い」と誤判定するリスクがある。 | FR8.1 の節番号を「§3」→「§2」に修正する（または節番号を明示せず「実例リストから旧称 `AuditLedgerRepository` を除去する」とし、番号依存を避ける）。 |
| 2 | Info | FR3.3 | 合格基準は「`next_decision` が `WorkflowExecution` の `&self` クエリメソッドとして実装され、ユースケース層に 21 分岐ラダーの判断ロジックが存在しないことをコードレビューで確認」に具体化され、検証可能になった。前回指摘（Minor 1）は解消済みと判断する。 | 対応不要。 |
| 3 | Info | FR8.1 | ADR-006 Consequences が指示した「§2b の許容動詞一覧への ES 拡張語彙 `store` の注記追加」は、追補文が節番号・追加内容ともに `gateway-taxonomy.md` の実際の構成（§2b「Repository のメソッド語彙」に許容動詞一覧が存在）と一致しており、正確。前回指摘（Major 1）のうちこの半分は解消済み。 | 対応不要。 |

### Summary

前回指摘 2 件（Major 1・Minor 1）のうち、FR3.3 の合格基準具体化（旧 Minor）は完全に解消された。FR8.1 の ADR-006 由来の追補（旧 Major）は、§2b への `store` 注記追加は正確だが、`AuditLedgerRepository` 除去の節番号を「§3」と誤記しており（実際は §2 の実例リスト）、新たな Major 所見として計上する。ただし識別子自体は一意で実装・検証を妨げるものではなく、他の FR/NFR との新規矛盾は確認されなかったため、advisory 判定としては READY とする。承認前に節番号の一字修正を検討されたい。所見 3 は前回どおりスコープ外の情報所見として据え置き（今回追加なし）。
