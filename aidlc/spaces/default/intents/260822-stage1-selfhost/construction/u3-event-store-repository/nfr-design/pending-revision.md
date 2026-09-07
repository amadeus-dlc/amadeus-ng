# pending-revision — U3 nfr-design（2026-09-07 再走レビューの所見）

> 2026-09-07 再走（Modify）の advisory レビュー（aidlc-architecture-reviewer-agent、iteration 1、**READY**、Critical 0 / Major 1 / Minor 1 /
> Info 4）の所見 R-01〜R-06 を、メイン（コンダクタ）が現行コードで**全件再実測して有効と確認**したうえで、確定文面として記録する。
> 成果物は終端受領で凍結中（per-unit・advisory のため承認ゲートには載らない）。次に `security-design.md` / `logical-components.md` を
> 開く機会（code-generation の計画、または後方ジャンプ）に本ファイルの文面をそのまま適用する。番号はレビュー節の R-ID と対応。

## 1. R-01（Major）— 同一プロセス内の兄弟 Repository 接続（security-design §3 / §4、NFR4.7）

**再実測**: `modules/app/aidlc/src/runtime.rs:306-308` は `IntentExecutionRepositoryImpl::open(&store)` / `IntentRepositoryImpl::open(&store)` /
`WorkflowDefinitionRepositoryImpl::open(&store)` を同じ `StorePath` に対して開く（同型の 3 本組 `:817-819` / `:1589-1591`、2 本組
`:402-403` / `:461-462`）。`intent_repository_impl.rs:126-136` のとおり兄弟 Repository も `IntentSqliteStore::new(path)` で独立の本家ストア
（= 別の rusqlite 接続）を持つ。所見は有効。

**確定文面（§3 の「直列化の実体は CAS」項の (i) を差し替え）**:
> (i) 合成ルート（U7）は 1 コマンドで同一 `StorePath` に**兄弟 Repository の接続を 2〜3 本**開く（`runtime.rs:306-308` の 3 本組 =
> IntentExecution / Intent / WorkflowDefinition、同型 `:817-819` / `:1589-1591`、2 本組 `:402-403` / `:461-462`。各 `*SqliteStore::new` が
> 独立の rusqlite 接続を持つ）。SQLite の書込ロックはファイル単位であり、`busy_timeout` を設定できないという前提は同一プロセス内の
> 兄弟接続にもそのまま掛かる。現行で競合しない根拠は、ユースケースが await を逐次に並べ（`current_thread` ランタイム、`spawn` /
> `spawn_blocking` なし — tech-stack-decisions §1）、本家の Tx が `persist_*` 1 呼出に閉じるため、兄弟接続の Tx が重なる時点が無いこと
> である。この前提（**Tx をまたいで兄弟 Repository を呼ばない・並行タスクを起こさない**）は `reopened()` の複数ハンドルと併せて U7 の
> 並行モデル裁定に明示的に載せる。`IntentExecutionRepositoryImpl` の `open` だけを数えた「1 コマンド 1 ハンドル」という旧文は撤回する。

**確定文面（§4 の Busy 行）**: 「待たずに中断し再実行を促す（文言は U7）。並行モデルは U7 の裁定」→ 「別プロセス、**または同一プロセス内で
兄弟接続の Tx が重なった場合**に `WouldBlock`。待たずに中断し再実行を促す（文言は U7）。並行モデル（複数プロセス・`reopened()`・兄弟接続）は
U7 の裁定」。

**確定文面（logical-components §5 第 1 項）**: 「`IntentExecutionRepositoryImpl::open` をコマンド単位で 1 回開く（現行 8 か所）」→
「コマンド単位で兄弟 Repository の `open` を同じ `StorePath` に 2〜3 本並べる（`IntentExecutionRepositoryImpl::open` は 8 か所）」。
U7 裁定 (1) に「兄弟接続の Tx 非重複の前提」を追記。

## 2. R-02（Minor）— `kinds_codec` の利用者（logical-components §1）

**再実測**: `grep -rn kinds_codec modules/` で command アダプタ内の利用者は `workflow_definition_dto.rs:92`、
`compiled_definition_repository_impl.rs:325`、`:753` の 3 か所（RMU 側は同形の別モジュール）。本文の「`WorkflowDefinitionDto` だけ」は
`dto/` 配下に限った grep から書いた限定主張で、反証された。所見は有効。

**確定文面**: 「利用者は `WorkflowDefinitionDto`（兄弟 Repository の DTO、実測 `workflow_definition_dto.rs:92`）で U3 の DTO は使わない」→
「利用者は 3 か所（`workflow_definition_dto.rs:92`、`compiled_definition_repository_impl.rs:325` / `:753`）でいずれも兄弟 Repository の所有。
U3 の DTO は使わない — 同じ `dto/` に同居する共有機構として記す」。

## 3. R-03（Info）— `DtoDecodeError::Malformed` のフィールド名（security-design §2 層 (2)）

**再実測**: `dto/dto_decode_error.rs:10-15` は `Malformed { field: &'static str, found: String }`。所見は有効。
**確定文面**: `Malformed { field, value }` → `Malformed { field, found }`（本文 1 か所と例示コードの注釈）。

## 4. R-04（Info）— `IntentExecution::new` の引用範囲（security-design §2 層 (2)）

**再実測**: `pub fn new(` は `:290`、本体の終端 `Ok(execution)` → `}` は `:336-337`。`:345` は `replay` の doc 内。所見は有効。
**確定文面**: `intent_execution.rs:290-345` → `intent_execution.rs:290-337`。

## 5. R-05（Info）— adapter 行の依存列挙（logical-components §1）

**再実測**: `interface-adapter/Cargo.toml` の `[dependencies]` に `core-command-use-case` / `core-command-domain` / `core-infrastructure` が
実在。表の domain 行は内部依存を挙げており数え方が不揃い。所見は有効。
**確定文面**: `intent_execution_repository_impl` 行の依存列を「`core-command-use-case` / `core-command-domain` / `core-infrastructure`、
`event-store-adapter-rs`（`sqlite`）、`rusqlite`、`serde` / `serde_json`、`chrono`。dev: `tempfile`、`tokio`」へ。

## 6. R-06（Info）— 上流 `requirements.md:133-135` の失効が traceability から見えない

**再実測**: `ls formal/orchestration/` = engine_loop / journal_protocol / stop_hook。`audit_lock` は `modules/` / `formal/` で 0 件。
上流 NFR3 の合格基準（`audit_lock.qnt` の ITF 準拠、集約名 `WorkflowExecution`）は失効している。nfr-requirements pending-revision 項目
R-03 と同一の事象。所見は有効（設計変更は不要）。
**扱い**: U3 の書込範囲外。Unit 完了報告でオーナーに「上流 `requirements.md` NFR3 の改訂要否（後方ジャンプ / 人間裁定）」を提示する。
`traceability.json` の NFR3.x は設計としては OK のまま据え置く（上流の失効は上流側の修正で解消する事象であり、U3 の GAP ではない）。
