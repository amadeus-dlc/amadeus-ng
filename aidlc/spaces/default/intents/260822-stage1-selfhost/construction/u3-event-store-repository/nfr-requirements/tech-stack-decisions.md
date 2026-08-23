# tech-stack-decisions — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> NFR Requirements（Construction 3.2）成果物（Unit: U3、Bolt: B5）。出典: `aidlc/spaces/default/codekb/docs/technology-stack.md`（既存依存の実測）、`../functional-design/
> functional-design-questions.md`（Q3 = A rusqlite、Q4 = A Quint）、ADR-006（tokio、ローカル trait）、`security-requirements.md`（NFR4.1）、team.md（サプライチェーン裁定）。

## 1. 選定

| 項目 | 決定 | 根拠 |
|---|---|---|
| SQLite ドライバ | `rusqlite`（`features = ["bundled"]`、1 系の最新固定版を B5 着手時に `cargo add` で確定） | 成熟・同期 API・最小依存（`libsqlite3-sys` のみ）。同梱ビルドでホストの SQLite 版差を排除（Q3 = A） |
| async ランタイム | `tokio`（`rt`, `macros`、current_thread。composition root の `#[tokio::main(flavor = "current_thread")]` は U7） | ADR-006。本 Unit では trait の `async fn` と `#[tokio::test]` に使う |
| 同期呼出の形 | Repository / EventStore の `async fn` 内で rusqlite を同期呼出（`spawn_blocking` なし、`Send` 不要） | ワンショット CLI、ms 単位のブロッキング（Q3 = A）。内部可変性は `RefCell`（FD BR1.1） |
| 正準 JSON | 既存 `canon-json`（U1） | payload のバイト決定性（FD BR2.5） |
| ワイヤ | `serde` / `serde_json`（既存、adapter のみ） | ドメインは serde 非依存（ADR-004） |
| 形式検証 | Quint 0.32.0（既存）— 新モデル `formal/orchestration/journal_protocol.qnt`、ITF fixture を `tests/conformance/fixtures/journal_protocol/` | Q4 = A、ADR 0003 |
| テスト | `proptest`（既存）、`tempfile`（既存 dev）— 一時 DB ファイル | PBT / 契約テスト |
| 退役 | `md5`（ロック dir 名専用）を adapter から除去 | ADR-007 |

## 2. 依存の差分（予定）

| クレート | 変更 | 用途 |
|---|---|---|
| workspace `Cargo.toml` | `rusqlite = { version = "<固定>", features = ["bundled"] }`、`tokio = { version = "<固定>", features = ["rt", "macros"] }` を `[workspace.dependencies]` へ | 一元管理 |
| `core-use-case` | 依存追加なし（`async fn` は言語機能）。dev に `tokio`（テスト用）のみ可 | ポート定義 |
| `core-interface-adapter` | `rusqlite`、`tokio`（dev: `#[tokio::test]`）を追加、`md5` を除去 | ストア実装 |
| `core-domain` | 変更なし | 是正 2 型 + 改名は標準ライブラリのみ |
| `tools/lint` | 依存変更なし（ルール削除のみ） | — |

`cargo audit` は CI `audit` ジョブで新依存を含めて検査する（NFR4.1）。

## 3. 未決（後続で確定）

- `rusqlite` / `tokio` の固定版は B5 着手時点の最新安定版を採り、code-summary に記録する。
- `PRAGMA` の追加（`temp_store` 等）は非目標（NFR5）。必要になった時点で課題化。
