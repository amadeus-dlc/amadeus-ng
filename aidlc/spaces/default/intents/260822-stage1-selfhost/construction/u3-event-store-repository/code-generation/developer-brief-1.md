# developer-brief-1 — 委任 1: ロック系の退役 + U2 是正（U3 / Bolt B5）

Conversation language: 日本語（コメント・rustdoc・報告はすべて日本語。識別子・固定トークンは英語）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（Bolt B5）の委任 1 を担当する。リポジトリルートは `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、
ブランチ `bolt/b5-u3-event-store-repository`（チェックアウト済み）。**コーディング規則の正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README + 7 ルール）を
最初に読む**（field-visibility / module-visibility / gateway-taxonomy / use-case-rules / domain-equality / tell-dont-ask / error-handling）。

所有ファイル（書いてよい範囲）: `modules/core/{domain,use-case,interface-adapter}/src/**`、`modules/core/domain/tests/**`、`modules/core/interface-adapter/tests/**`、
`modules/infra-io/src/**`、`tools/lint/**`（`target/` 除く）、`formal/workspace/**`、`tests/conformance/fixtures/audit_lock/**`、`scripts/quint-gate.sh`、`scripts/coverage.sh`、
`modules/core/interface-adapter/Cargo.toml`（`md5` の除去のみ）、`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/{tell-dont-ask,README,gateway-taxonomy}.md`、
報告 `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/developer-report-1.md`（新規・あなただけが書く）。

触らないもの: 計画 `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`、`docs/specs/**`、`Cargo.toml`（workspace root）、`.github/**`。
`git add` / `git commit` はしない（コンダクタが 2 コミットに区切る — 「退役」完了時点と「是正」完了時点でそれぞれ報告を更新し、最終応答で知らせる）。`.claude/` 配下の
ツールは実行しない。

## 先に読むもの（順に）

1. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/code-generation-plan.md`（§1、§5.1 Step 1〜2、§7）
2. `.../u3-event-store-repository/functional-design/rules.md`（BR3.1 / BR3.2 / BR4.1 / BR4.2 / BR4.3）と `entities.md`（IntentId / IntentDirName / WorkflowExecutionState /
   RetiredLockMachinery）、`functional-spec.md`（§1 配置、§6 退役チェックリスト）
3. `.../u3-event-store-repository/nfr-design/security-design.md` §6（退役の安全手順）
4. `.../construction/u3-event-store-repository/code-generation/unit-test-instructions.md`
5. 既存コード: `modules/core/domain/src/orchestration/{intent_id.rs,workflow_execution_snapshot.rs,snapshot_error.rs,workflow_execution.rs,mod.rs}`、
   `modules/core/domain/src/workspace/{mod.rs,lock_protocol.rs,lock_identity.rs}`、`modules/core/use-case/src/{lib.rs,workspace/**}`、
   `modules/core/interface-adapter/src/{lib.rs,process_probe.rs,workspace/**}`、`modules/infra-io/src/{lib.rs,process_probe.rs}`、`tools/lint/src/{check.rs,main.rs}`、
   `scripts/quint-gate.sh`、`scripts/coverage.sh`（冒頭コメントと `TOLERANCE`）、`tests/conformance/fixtures/audit_lock/`、`formal/workspace/audit_lock.qnt`

## 作業 A — 退役（計画 Step 1、報告 §A。完了時点で報告を保存し、作業 B へ）

1. 削除: use-case `src/workspace/`（mod ごと、`lib.rs` の `pub mod workspace` / `pub use` も）、adapter `src/workspace/fs_workspace_lock.rs` と `src/process_probe.rs`
   （`src/workspace/mod.rs` / `src/lib.rs` の宣言・`pub use` を整理。`src/workspace/state_file_io.rs` は**維持**）、domain `src/workspace/lock_protocol.rs` /
   `lock_identity.rs`（`mod.rs` の `mod` と `pub use`: `LockProtocol` / `LockIdentity` / `reap_eligible` / `LockError`）、infra-io `src/process_probe.rs`（`lib.rs` の
   `pub use process_alive` 等）、tests `modules/core/interface-adapter/tests/fs_workspace_lock_test.rs`、`modules/core/domain/tests/audit_lock_conformance.rs`、
   `formal/workspace/audit_lock.qnt`（`formal/workspace/` が空になれば dir ごと）、`tests/conformance/fixtures/audit_lock/`（dir ごと）。
2. `tools/lint`: ルール `reap-decision-locality`（`RULE_REAP_DECISION_LOCALITY` / `REAP_HELP` / `reap_rule` フィールド / `push_reap` / `mentions_reap_state` / 関連分岐）と
   その赤例テスト・`main.rs` の登録・使用法文言を削除。`checkbox-vocabulary` / `no-public-fields` は維持。`cargo test --manifest-path tools/lint/Cargo.toml` 緑、
   `cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings` 緑。
3. adapter `Cargo.toml` から `md5` を除去（他に `md5` 使用が無いことを grep で確認）。
4. `scripts/quint-gate.sh`: `AUDIT_LOCK` 変数と、typecheck ループの該当モデル・`invariants run: audit_lock`・witness ループ（w_threshold_reap … w_recovery_after_mid_txn_crash）
   を除去。冒頭コメントの「(engine_loop / stop_hook / audit_lock)」も更新（journal_protocol の追加は委任 4 が行う — ここでは増やさない）。
5. `scripts/coverage.sh`: `TOLERANCE=0.05` → `0.01`、直前のコメント 2 行を「U3 のロック退役（ADR-007、Bolt B5）でジッタ源（並行ロックテスト）が消えたため 0.01 に
   引き締めた（team.md Testing Posture）」に更新。
6. coding-rules: `tell-dont-ask.md` の `reap_eligible` を例示する箇所に「（退役済み — ADR-007 / Bolt B5。以後は履歴としての例）」を注記し、規範の文は
   `checkbox-vocabulary` の例で成立するように調整。`README.md` の tell-dont-ask 行の機械強制を「`cargo lint`（checkbox-vocabulary）」に。`gateway-taxonomy.md` §1 の
   「本リポジトリでは `core_interface_adapter::{clock, process_probe}`」→ `{clock}`、『適用の帰結』表の `FsWorkspaceLock` 言及は履歴（旧列）なので残してよい。
7. 検査: `cargo build --workspace`、`cargo test --workspace` 緑、
   `grep -rnE 'WorkspaceLock|FsWorkspaceLock|LockProtocol|LockIdentity|reap_eligible|OwnerStamp|AcquireBudget|LockGuard|LockError|process_alive|ProcessProbe|audit_lock|reap-decision-locality' modules tools scripts formal .github Cargo.toml` = 0 件
   （`tools/lint/target` は除外）、`grep -rn 'aidlc-lock' modules tools scripts` = 0 件、`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`。
   結果を報告 §A に（削除ファイル一覧、grep 結果、テスト数）。

## 作業 B — U2 是正（計画 Step 2、報告 §B）

1. **Red**: `modules/core/domain/src/orchestration/intent_id.rs` のテストを UUIDv7 形式に書き換え（受理: `01a02785-1bd8-76eb-aeea-5aa303ebd5b6`；拒否: 大文字・
   version 4（13 文字目が `4`）・variant 不正（17 文字目が `c` 等）・長さ 35 / 37・空・ハイフン位置違い・kebab 文字列）。`IntentIdError` は `Empty` / `Length { actual }` /
   `Format { position }` / `Version { found }` / `Variant { found }`（材料のみ、手実装 Display / Error — error-handling.md）。
   `modules/core/domain/src/workspace/intent_dir_name.rs`（新規）のテスト: 受理 `260822-stage1-selfhost` / `260822-a` / `260822-stage1-selfhost-2`；拒否 先頭 6 桁なし・
   大文字・連続ハイフン・末尾ハイフン・65 字以上・空。`IntentDirNameError`（材料のみ）。失敗出力を報告に記録。
2. **Green**: 実装（標準ライブラリのみ、正規表現クレートを足さない。添字アクセスは `as_bytes().get(i)` / イテレータで — `indexing_slicing` を生まない）。
   `workspace/mod.rs` の `pub use IntentDirName` / `IntentDirNameError`。
3. **Refactor（改名）**: `workflow_execution_snapshot.rs` → `workflow_execution_state.rs`、`WorkflowExecutionSnapshot` → `WorkflowExecutionState`、
   `WorkflowExecutionSnapshotBuilder` → `WorkflowExecutionStateBuilder`、`snapshot_error.rs` → `state_error.rs`、`SnapshotError` → `StateError`、
   `WorkflowExecution::snapshot()` → `state()`、`from_snapshot()` → `from_state()`、rustdoc / コメントの「スナップショット」表現は「状態の写し（memento）」に
   （C6 の `snapshot` テーブルの話はアダプタ層の用語なのでドメインには出さない）。`orchestration/mod.rs` の `pub use` を更新し旧名を残さない（alias 禁止）。
   既存テスト・`modules/core/domain/tests/engine_loop_conformance.rs` の IntentId リテラル（`itf-engine-loop` / `260822-stage1-selfhost` / `u2`）を UUIDv7 に置換。
4. 検査: `grep -rn 'Snapshot\|snapshot' modules/core/domain/src/orchestration` = 0 件、`cargo test --workspace` 緑、`cargo clippy --workspace --all-targets -- -D warnings`、
   `cargo fmt --all --check`。

## 作法（厳守）

- TDD（Red → Green → Refactor、失敗出力を報告に）。プロダクトコードに `unwrap` / `expect` / `panic!` / 添字アクセスを書かない。フィールドは private + アクセサ、mod は
  private + ファサード `pub use`。エラーは手実装 enum（材料のみ）。
- 後方互換の残置（型エイリアス・deprecated・feature flag・旧 mod の空殻）を作らない。
- 設計に無い判断が要ったら推測せず、報告の「設計質問」に書いて該当箇所を保留（他の作業は進める）。

## 報告（`developer-report-1.md`）

見出し: 「§A 退役（削除一覧・grep・テスト数・コマンド出力）」「§B U2 是正（Red の失敗出力・Green・改名の差分概要・grep・テスト数）」「設計質問」「未了」。
最終応答は報告の要約（日本語、10 行以内）。