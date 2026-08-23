# code-summary — U9 正本・仕様の canon 追従（`u9-canon-docs`、Bolt B4）

> Code Generation（Construction 3.5）の結果（Unit: U9、kind: spec、Bolt: B4、規模 S）。出典: `code-generation-plan.md`（承認指紋
> sha256:819fec3a…）、`unit-test-instructions.md`、`developer-report-1.md`（委任 1: coding-rules / components.md / deviations.md）、
> `developer-report-2.md`（委任 2: 仕様 01 / 10 / 11 / 12 号）、`../functional-design/rules.md`（BR1.1〜BR5.2）と各 pending-revision。
> **コードは書いていない**（受入 1 / 1b で実測）。

## 1. 結果

- 改訂対象 10 ファイルすべてに BR1.1〜BR5.2（+ 計画で取り込んだ BR1.5 と pending-revision 項目）を適用した。コード（`modules` / `tools` / `scripts` /
  `.github` / `Cargo.*`）と `docs/specs/research/**` の diff はゼロ。
- 受入検査 1〜5（security-design §3）はすべて緑（§3）。sentinel 7 語は `docs/specs/*.md` で 0 件、`coding-rules/*.md` では履歴注記 2 行のみ。
- 委任 2 本は並行で完了（所有ファイル非重複、コミットはコンダクタ）。コンダクタ統合で 2 箇所を追加修正（§4）。

## 2. 作成・変更ファイル（`git diff --stat -- docs aidlc/spaces/default/knowledge aidlc/.../inception`、コミット前の実測）

| ファイル | 変更 | 適用した BR |
|---|---|---|
| `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md` | +1 / −1 | BR1.1 |
| `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md` | +13 / −8 | BR1.2 / BR1.3 / BR1.4 / BR1.5（§1b 一般形、WorkspaceLock 退役注記） |
| `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md` | **新規**（24 行） | BR4.1（FD Q1 = A の文面、裁定日 2026-08-23） |
| `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md` | +2 / −1（7 行 = 7 ファイル） | BR4.2 |
| `docs/specs/deviations.md` | +1（# 4） | BR3.4 |
| `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md` | +9 / −10 | BR3.5（WorkspaceModel → 値オブジェクト語彙、ReadModelUpdater に描画責務） |
| `docs/specs/01-domain-model.md` | +27 / −11 | BR2.2 / BR2.4 / BR3.1 / BR3.2 / BR3.6（§7.1 新設） |
| `docs/specs/10-orchestration.md` | +27 / −15 | BR2.3 / BR2.4 / BR3.1 / BR3.3（§2.1 ES 形、§3 ポート表、§8、§10 S2） |
| `docs/specs/11-workspace.md` | +33 / −24 | BR2.1 / BR3.2（§2.1 / §2.2 / §2.3 / §3 / §4 / §5 / §7 / §9 / §10） |
| `docs/specs/12-workflow-definition.md` | +28 / −29 | BR2.4 / BR2.5（5 箇所）/ BR3.1 / BR3.3 |

合計 9 変更 + 1 新規、+148 / −94（`developer-report-1.md` / `-2.md` の改訂一覧に節単位の対応と出典注記を記載）。

## 3. 受入検査の記録（`unit-test-instructions.md` §1、統合後の実測）

| # | 検査 | 結果 |
|---|---|---|
| 1 | `git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock` | 空（コード変更ゼロ） |
| 1b | `git diff --stat origin/main..HEAD -- docs/specs/research` | 空 |
| 2 | sentinel 7 語 grep（`coding-rules/*.md` + `docs/specs/*.md`） | `docs/specs/*.md` = 0 件。`coding-rules/gateway-taxonomy.md` に 2 件のみ — 4 行目「適用例」（ファイル冒頭の履歴 — 旧 PR の記述）と 96 行目「適用の帰結」旧→新移行表の**旧**列（`StateFileStore`）。いずれも履歴注記で規範ではない |
| 3 | README 行数 = ルールファイル数 | 7 = 7（一言・機械強制の一致は目視確認） |
| 4 | 表の列数（10 ファイル） | `tables ok` |
| 4b | 見出し重複 | なし |
| 5 | deviations # 4 | 1 行（理由欄: ADR-001 / 003 / 004 / 007） |
| 6 | CodeRabbit スレッド | PR 作成後に実施（§7） |

Red 基線（承認直後、`origin/main` 同等）: sentinel ヒット = gateway-taxonomy 3 / 10 号 3 / 11 号 2 / 12 号 5、README 6 = 6、deviations 最大 # 3
（診断の詳細は各 developer-report の「Red 基線」）。

## 4. 主要な判断（委任の設計質問とコンダクタ裁定）

| # | 論点 | 裁定 |
|---|---|---|
| 1 | メメントのアクセサ名（委任 2 質問 1）: U2 pending-revision 9 は型名だけの改名と読めたが、改名の目的は「ドメイン API から `snapshot` の語を除き ES スナップショット（C6）との混同を避ける」こと | 10 号 §2.1 の規範を `state()` / `from_state()` とし、U2 pending-revision 9 に追記（B5 の計画で確定、ゲートでオーナー確認） |
| 2 | 10 号 §10 S2 行が退役済み `withAuditLock` を規範として残す（委任 2 質問 2） | コンダクタが B4 内で改訂（SQLite 1 Tx + 投影チェックポイント、ADR-001 / 003 / 007、`audit_lock.qnt` は B5 で協定モデルへ） |
| 3 | 10 号 §6 I14 / 11 号 §6 W1〜W5 / 01 号 §3.3 代表不変条件が mkdir ロック前提（委任 2 質問 3） | **B5（U3）へ繰り延べ** — ADR-007 の `audit_lock.qnt` 改訂と同期して E4 定義名を差し替える（本 Unit では「改訂して存続」の注記のみ） |
| 4 | 11 号 §3 audit 5 動詞（CLI 語彙 = 逐語契約）と投影の責務の関係（委任 2 質問 4） | 逐語契約に触れない方針で保留 — U4 / U5 の設計で確定 |
| 5 | `intents.json` の直列化機構（委任 2 質問 5）、stage-0/1 併用期の相互排他（質問 6） | 11 号 §10 の未決事項として登録（U3 設計 / オーナー裁定待ち） |
| 6 | 委任 1 の判断 1〜4（deviations 行の U3 注記、旧→新表の `FsWorkspaceLock` 残置、同一節内の語彙同期、components.md の自己整合） | すべて受容（最小変更の範囲内、新しい規範の導入なし） |
| 7 | 11 号 §7-4 の ITF 項が「ロックサービスの純粋遷移関数」のまま（統合レビューで検出） | コンダクタが改訂（協定モデルへ改訂後の `audit_lock.qnt` を Repository 実装の遷移関数に再生 — B5） |

## 5. テスト

- プロダクションコード・テストコードの変更なし。既存スイート（`cargo test --workspace`）は `origin/main` の緑のまま（PR の CI で確認）。
- 「テスト」= §3 の受入検査（grep / diff / 行数 / 表整形スクリプト）。

## 6. 計画からの逸脱

- なし（Step 0〜8 のとおり）。コンダクタ統合で §4 の 2・7 を追加修正したが、いずれも BR3.2 / BR3.3 の範囲内の自己整合。

## 7. 申し送り

- **B5（U3）**: `WorkflowExecutionSnapshot` → `WorkflowExecutionState` + `state()` / `from_state()` 改名、`IntentId` UUIDv7 是正 + `IntentDirName`、
  `audit_lock.qnt` 協定モデル改訂に伴う 10 号 §6 I14 / 11 号 §6 W1〜W5 / 01 号 §3.3 代表不変条件の差し替え、deviations # 4 の SQLite パス確定、
  `intents.json` 直列化機構。
- **U4 / U5**: 11 号 §3 audit 5 動詞と投影の関係。
- **ステージゲート**: FD / NFR 要求 / NFR 設計の pending-revision（本計画に取り込んだ項目）は Request Changes で正本へ同期。
- 受入 6（CodeRabbit 全件対応）と merge queue は PR 作成後。

## 8. コミット（ブランチ `bolt/b4-u9-canon-docs`、`origin/main` 1c5cb28 起点）

- 記録コミット: 456caf3（計画・承認）。文書コミット: 本ファイル作成後に 1 コミット（squash 時のコミット名 = Bolt slug）。
