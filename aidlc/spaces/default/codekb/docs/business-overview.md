# business-overview — amadeus-ng のビジネスドメインと目的

> リバースエンジニアリング成果物（2026-08-22 実施、`c4d8d95` 時点）。一次情報は開発者スキャン結果・`docs/specs/00-policy.md`・`docs/specs/deviations.md`。

## ビジネスドメインと目的

amadeus-ng は、AWS の **aidlc-workflows**（AI-DLC: AI-Driven Development Life Cycle のワークフローエンジン。TypeScript + bun 実装）を **Rust で再実装**するプロジェクトである。upstream は [awslabs/aidlc-workflows](https://github.com/awslabs/aidlc-workflows) のブランチ `v2`、コミット `3c3146cf`（v2.6.40）にピン留めされ、as-built 仕様 28 ファイル（約 21,000 行）が `docs/upstream/specs/` に凍結参照されている。

再実装の動機（`00-policy.md` D1）は、ユーザー環境のランタイム前提（bun）を排し、プリビルドバイナリで配布することにある。ドメインは「AI エージェントによる開発ワークフローのオーケストレーション」そのもの — ステージグラフの解決、エンジンループ（next/report）、ゲート承認、ワークスペース状態・監査台帳・ロックの管理 — であり、upstream と**観測可能な契約レベルで互換**（ワークスペースレイアウト、監査イベント語彙 86 語、CLI 語彙、`AIDLC_*` 環境変数、LLM の分岐条件になる逐語文言）を維持する方針（D6）。

## 現行 intent と stage-1 切替条件

現在のアクティブ intent は `260822-stage1-selfhost`（GitHub Issue #7「stage-1（セルフホスト切替）への最短経路」）。開発戦略はコンパイラと同じブートストラップ構図（D8）で、現在は **stage-0**（upstream の AI-DLC ワークフローをホストに amadeus-ng を開発）にあり、**stage-1** で自分自身をホストに切り替える。

切替条件の正本は `docs/specs/00-policy.md` §4 の 5 条件（センサー・プラグインは含めない）:

1. エンジン `next` / `report` が Claude Code 上でゲート込みで動く
2. 状態・監査・ロックが upstream 互換で機能する（D6 の範囲）
3. 自プロジェクト開発で使うスコープ（bugfix / feature 相当）のステージ一式が揃っている
4. `--doctor` 自己診断が green
5. smoke + unit 相当の CI が green

## 主要機能 — 実装済みと計画

**実装済み（フェーズ A / スライス 1 相当、inside-out の内側 3 層）**:

- **ドメイン層**（`core-domain`）: 3 つの境界づけられたコンテキスト — orchestration（集約 `WorkflowExecution` = エンジンループの純粋ステップ関数）、workflow_definition（読取モデル集約 `WorkflowDefinition` + ステージグラフ/スコープグリッド）、workspace（`LockProtocol`、`reap_eligible` 述語、状態ファイル純関数群）
- **ポート**（`core-use-case`）: `WorkflowDefinitionRepository`（読取専用）・`WorkspaceLock` の trait 2 本
- **Gateway**（`core-interface-adapter`）: 定義読取 Repository 実装、mkdir ベースのファイルシステムロック `FsWorkspaceLock`
- **共有 Published Language**: 監査イベント語彙（86 イベント / 22 カテゴリ）、ディレクティブ種別（10 種）、upstream 逐語文言カタログ（7 形、バイト一致確認済み）
- **形式検証**: Quint モデル 3 本（engine_loop / stop_hook / audit_lock、mutation テストで検査力証明済み）+ ITF 準拠テスト + ゴールデンパリティテスト

**計画済み・未着手**: ユースケース本体、composition root、CLI 面（バイナリ `aidlc` は現状スタブ）、正準 JSON シリアライザ（`canon-json` スタブ）、監査台帳 I/O、`WorkflowExecutionRepository`（B-2）、ハーネス配線（`harness-claude` スタブ）。

## 互換ポリシーと逸脱管理

互換判定の原則（2026-08-22 オーナー裁定）は「仕様か実装か」— 観測可能な契約（オンディスク形式、監査行、逐語文言、CLI 面の振る舞い、原子性保証）は踏襲必須、内部機構（プロセス構成、ライブラリ、アルゴリズム）は仕様を守る限り自由。仕様レベルの逸脱は `docs/specs/deviations.md` に一元管理され、現在 **3 件**（#1 コマンド綴り写像 — bun 不在による設計変更、#2 upstream 既知バグ M12 の修正、#3 `AIDLC_LOG` 環境変数の拡張）+ 予約 1 件（インストーラ追加）が登録済み。

## 品質戦略との結びつき

「開発ワークフローを回す装置」自体が成果物のため、自分の開発に使うこと（セルフホスト）が最良の受入テストと位置づけられている。仕様（specs 6 本 + ADR 5 本 + research 15 本）と形式モデルが実装に先行し、upstream 互換は逐語・バイト一致（ゴールデン入力: `tests/golden/upstream-3c3146cf/`）で管理される。切替後は「ホスト = 直近安定タグ、ターゲット = 開発版」の 2 版運用で self-hosting tax に備える。
