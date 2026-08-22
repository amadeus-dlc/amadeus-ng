# Requirements Analysis — 確認質問

Issue #7 と 00-policy §4 で目標・切替条件・クリティカルパス・スコープ外は明文化済み。
以下は**要求の境界が動く5点**のみ（codekb・team-practices・設計監査を読了のうえで残った不確定点）。

## Q1. 設計監査の修正束を本 intent の要求に含める範囲

Practices Discovery で CI 整備2件（tools/lint CI・PBT シード固定 = D束の一部）は stage-1 に含めると確定済み。残る束の扱いを確定したい:

- A束: canon 語彙の自己矛盾修正（use-case-rules / gateway-taxonomy の load 語彙、数行）
- B束: 仕様の canon 追従（11号ポート表・01号集約候補表・10/12号の整合、docs のみ）
- C束: コード修正（R1: PlanAction 移動 / R2: effective_plan_action 移設 ほか C17〜C33）

R1/R2 は裁定済み（DECIDED）で、B-1（監査台帳）・B-2 の設計が乗る土台に直接影響します。

- A. A+B束と、C束のうち R1/R2 履行分だけ含める（B-1 着手前の土台整備。残りの C束は後続 intent）
- B. A〜C束すべて含める（監査完済してから B-1 へ）
- C. どれも含めない（最短優先。監査対応は全部後続 intent）
- X. Other (please specify)

[Answer]: A. A+B束と、C束のうち R1/R2 履行分だけ含める（B-1 着手前の土台整備。残りの C束は後続 intent）

## Q2. 切替条件3「自プロジェクト開発で使うスコープのステージ一式」の受入解釈

00-policy §4 条件3 の「bugfix / feature 相当のスコープのステージ一式が揃っている」は、D6 互換により upstream `dist/claude/` のステージ資産を**そのまま読む**前提です。受入基準としてどう解釈しますか？

- A. バイナリが bugfix / feature スコープのグラフ解決・ステージ進行（next/report のゲート往復）を通せること — ステージ本文の実行は Claude Code（ハーネス）側の仕事なので、エンジンの導線が通れば条件3は満たす
- B. 実際に bugfix スコープの intent を1本、最初から最後まで回して初めて条件3を満たす（条件4・5 と合わせて受入）
- X. Other (please specify)

[Answer]: B. 実際に bugfix スコープの intent を1本、最初から最後まで回して初めて条件3を満たす（Q4 の DoD と同一の実地スモークで検収）

## Q3. 0b（実行時採取 + hash-canonical 受入表）の担当変更

Issue では 0b は「オーナー担当」（bun + upstream 導入が必要だったため）。しかし現在は AI-DLC 導入済みで bun がこのリポジトリで動きます。0b（CLI 実行出力ゴールデン・hash-canonical 受入表 = ADR 0001 の実ハッシュ出力）をワークフロー内の作業（Bolt）に取り込みますか？

- A. 取り込む（canon-json 実装の受入表と CLI ゴールデンを Bolt 化。オーナーは結果レビューのみ）
- B. オーナー担当のまま（ワークフローは依存として待つ）
- X. Other (please specify)

[Answer]: A. 取り込む（canon-json 実装の受入表と CLI ゴールデンを Bolt 化。オーナーは結果レビューのみ。※初回提示が説明不足だったため 0b の内容 — 本家ツールを bun で実行し実ハッシュ・CLI 実出力を正解データとして採取する作業 — を説明のうえ確定）

## Q4. stage-1 到達（Issue close）の Definition of Done

「amadeus-ng 自身をホストにこのリポジトリの開発が回る」を検収可能にする具体条件は？

- A. 本リポジトリで bugfix 相当の小 intent を1本、amadeus-ng バイナリをエンジンにして開始→ゲート承認→完了まで通す（実地スモーク）＋ doctor green ＋ CI green
- B. doctor green ＋ 条件1〜3・5 の個別テスト green で足りる（実地の1本通しは切替後の初仕事に回す）
- X. Other (please specify)

[Answer]: A. 本リポジトリで bugfix 相当の小 intent を1本、amadeus-ng バイナリをエンジンにして開始→ゲート承認→完了まで通す（実地スモーク）＋ doctor green ＋ CI green

## Q5. 性能 NFR の要否

upstream は bun 起動 ~20ms を謳います。Rust バイナリは通常これを大きく下回りますが、数値目標を立てますか？（エンジンは next/report のたびに起動されるワンショット CLI）

- A. 立てない（「体感で upstream と同等以上」の定性のみ。実測が明確に劣化したら課題化）
- B. 立てる（例: `next` 応答 p95 < 100ms @ 本リポジトリ規模の状態ファイル — 計測は 0b のゴールデン採取と同時）
- X. Other (please specify)

[Answer]: A. 立てない（「体感で upstream と同等以上」の定性のみ。実測が明確に劣化したら課題化）

## Consolidated Summary Confirmation

初回（2026-08-22T05:2x、Q1〜Q5）の確認は `Looks correct` で確定済み（原本は
`archive/2026-08-22-requirements-analysis/` に保存）。以下は改訂ラウンド（Q6）を加えた再確認。

- 監査修正束: A束（canon 語彙）+ B束（仕様追従）+ C束のうち R1/R2 履行分を本 intent に含める。残りの C束は後続 intent
- 切替条件3の受入 = DoD の実地スモークと同一: bugfix 相当の小 intent 1本を amadeus-ng バイナリで開始→ゲート→完了まで通す + doctor green + CI green
- 0b（本家実行による hash-canonical 受入表 + CLI 出力ゴールデン採取）はワークフローの Bolt に取り込む（オーナーはレビューのみ）
- 性能 NFR は数値目標を立てない（定性のみ）
- Practices Discovery で確定済みの stage-1 追加整備（branch protection・サプライチェーン4件・tools/lint CI・PBT シード固定）を要求に反映する
- 改訂ラウンド（Q6 = A）: FR1.1 を「監査シャードは投影、台帳本体は SQLite ジャーナル、合格 = 投影出力の逐語契約一致」へ、FR1.2 を「SQLite Tx + 楽観 version による audit-first、合格 = 改訂版 audit_lock.qnt の ITF 準拠」へ、FR1.3 を「WorkflowExecutionRepository（ES: store / find_by_id）、O2 close」へ改訂する
- 改訂ラウンド（Q6 = A）: NFR3 を「ジャーナルから集約を再構成、互換ファイルは投影で再生成」へ、NFR1 の逸脱台帳注記に ADR-003/007 の追加登録を反映、§7 の O1（ADR-002）/ O3（ADR-004）を解決済みとして close する
- 改訂ラウンド: 上記以外の FR/NFR/制約/前提/スコープ外と Q1〜Q5 の回答は変更しない

Does this all look correct before I generate the requirements artifact?

- Looks correct
- Request changes

[Answer]: Looks correct

---

## 改訂ラウンド（2026-08-22、units-generation からの後方ジャンプ）

units-generation の Q9/Q9a（オーナー回答「いやー改訂しないとまずくないか。」→「今すぐ戻って改訂する」）を受け、
domain-design の ADR-001/003/004/007（イベントソーシング採用・SQLite ストア・WorkflowExecution 集約ルート・
ロック機構退役）と矛盾する FR1 群ほかを改訂する。既存の Q1〜Q5 の回答と他の FR/NFR は変更しない。

## Q6. ADR に合わせた改訂の範囲

改訂候補:

- FR1.1 — 監査シャード（`<record>/audit/<host>-<clone>.md`）は ReadModelUpdater の投影（リードモデル）、
  台帳本体は SQLite ジャーナル。合格 = 投影出力が 0a 逐語契約（EVENT_HEADINGS 86 語・FIELD_ORDER）に一致
- FR1.2 — 「ロック区間と結合」→「SQLite Tx + 楽観 version による audit-first（ジャーナル先行）」。
  合格 = 改訂版 `audit_lock.qnt`（ジャーナル/スナップショット/version/チェックポイント協定）の ITF 準拠
- FR1.3 — `AuditLedgerRepository` → `WorkflowExecutionRepository`（ES: store / find_by_id）の設計・実装。
  O2（AuditLedger の位置づけ）は ADR-001/003 で「イベントログ」と裁定済みとして close
- NFR3 — 「監査台帳から再構成」→「ジャーナルから集約を再構成でき、互換ファイル（状態ファイル・監査シャード）は
  投影で再生成できる」
- NFR1 — 逸脱台帳の件数注記に ADR-003/007 の追加登録（SQLite ファイル・ロック dir 非生成）を反映
- §7 O1（next_decision 配置）/ O3（StateFile 所有）— ADR-002 / ADR-004 で解決済みとして close

- A. 上記すべてを改訂する（推奨）
- B. FR1.3 は削除し、ES Repository は FR2（report）の前提として扱う（他は A どおり）
- C. FR1 の 3 項目だけ改訂し、NFR3・NFR1 注記・O1〜O3 は触らない
- X. Other (please specify)

[Answer]: A
