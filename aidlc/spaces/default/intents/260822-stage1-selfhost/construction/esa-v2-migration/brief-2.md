# brief-2 — 委任 2: ストア差し替えと自前実装の削除（乗り換え本体）

Conversation language: 日本語。AI-DLC ステージ外（オーナー裁定）。規律は同じ: TDD・検査全通過・報告ファイル。

## 目的

ADR-010 の乗り換え第 2 段（最終段）。永続化を本家 `EventStoreForSqlite` に切り替え、
自前ストア一式（約 2,400 行）を削除し、横断読取と checkpoint を自前実装する。

## 先に読むもの

1. ADR-010（`inception/domain-design/decisions.md` — 特に「横断読取はライブラリ責務外・
   自前実装」の 2026-08-26 追記）と `developer-report-1.md`（委任 1 の到達点と設計質問）
2. **オーナー裁定 2 件（2026-08-27、日誌に記録済み）**:
   - (A) serde 復号は **memento 経由** — `#[serde(into = "WorkflowExecutionState",
     try_from = "WorkflowExecutionState")]` で `from_state()` の検査点を通す
   - (B) **version はストアが採番する不透明トークン。seq_nr と混ぜない** —
     BR5.3 改訂済み（U2 rules.md）。`version == seq_nr − 1` 系の前提検査は全廃
3. 本家 v2.0.0 の **SQLite user-account example**（`examples/`）— 利用作法の正
4. `coding-rules/` — upstream-contracts / cqrs-boundaries / gateway-taxonomy §1c / README 優先順

## やること

1. **(A) memento 化**: `WorkflowExecutionState` に serde derive を足し、`WorkflowExecution` の
   serde を into/try_from 経由へ。改竄 JSON が **Err で拒否される**テストを書く（委任 1 の
   probe の恒久化）。
2. **`sqlite` feature 有効化**と Repository の差し替え: `WorkflowExecutionRepositoryImpl` が
   `EventStoreForSqlite<IntentId, WorkflowExecution, WorkflowExecutionEvent>` を直接所有。
   利用作法（シリアライザ設定・genesis / update の呼び分け・version の受け方）は
   **本家 example を実測して従う**。
3. **(B) version 結合の除去**: `check_preconditions` から version を読む検査を全廃。
   seq_nr の連続性検査（ドメインの関心）は残す。
4. **横断読取の自前実装**（新型 `JournalReaderImpl` — gateway-taxonomy §1c の永続化基盤ポート）:
   - 同一 DB ファイルへ**自前の別接続**を開く（本家ストアとは独立。読取専用）
   - カーソル = 本家 `journal` の rowid（追記専用 + 書込直列化で単調 — ADR-010 の根拠）。
     `GlobalSeqNr(u64)` に包む。payload の復号は本家と同じシリアライザ形式で
   - **checkpoint は自前表**（名前は本家と衝突しないもの。例 `amadeus_projection_checkpoint`）。
     単調・後退拒否・冪等 advance（journal_protocol モデルの契約と同型）
   - **スキーマガードテスト**: `sqlite_master` から本家 `journal` の DDL を読み、ピン留めした
     期待値と比較。ずれたら「本家スキーマが変わった。=2.0.0 固定を見直せ」と明示的に落ちる
5. **削除**: `event_store_impl.rs` / `schema.rs` / ローカル `EventStore` trait /
   旧 `JournalReader` 実装。エラー型は「我々の口が返す分」だけ残し、本家エラーからの写像を
   定義（`error-handling.md` — 我々が書くエラー型は手実装 enum のまま）。
   **後方互換の残骸を残さない**（旧名・エイリアス・二重口の全廃）。
6. **InMemory 側**: 契約テスト（両実装に同じ約束）を保つ。本家 memory バックエンドへの
   置き換えで自前 `InMemoryEventStore` も消せるなら消す（削減が増える）。ただし
   `journal_protocol` の ITF 準拠テストが InMemory 実装に依存している — **Quint モデルの
   意味論（version_equals_journal 等）に手を入れる必要が出たら、改変せず止めて報告**
   （モデルの改訂はオーナー/コンダクタ裁定）。
7. テスト・ワイヤ・呼出側の追随。**Published Language 面（ISO 8601 文字列・ITF fixture・
   Quint）は逐語維持**。一括置換で文字列リテラルを壊さない。

## 触ってはいけないもの

`docs/**`・設計文書（食い違いは列挙のみ）。`.claude/**` / `scripts/**` / `formal/**`（Quint モデル改変禁止 — 上記 6）/ `.coderabbit.yaml`。`git add/commit/push` 禁止。

## 検査（全部通す）

fmt / clippy -D warnings / cargo lint / PROPTEST_RNG_SEED=20260823 cargo test --workspace
（689 全緑を維持。削除に伴うテスト減は「削除対象のテストのみ」であることを報告で証明）/
quint-gate / cargo audit（両 lock）/
`PROPTEST_RNG_SEED=20260823 bash scripts/coverage.sh --base origin/main`（絶対 90% 床 + 相対ゲート）

## 報告

`construction/esa-v2-migration/developer-report-2.md`。削除行数の実測、テスト数の増減内訳、
version 結合を除去した全箇所、スキーマガードの中身、食い違い列挙、設計質問、未了。
最終応答 10 行以内（ファイルが正）。
