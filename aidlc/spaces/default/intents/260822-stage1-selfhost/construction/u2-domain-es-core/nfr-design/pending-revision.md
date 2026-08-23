# pending-revision — U2 nfr-design（ステージゲートの Request Changes で適用する改訂案）

> レビュー（iteration 1、2026-08-23、READY: Major 2 / Minor 2）の所見を是正する編集案。終端の受領が凍結している（review-freeze フック）
> ため、nfr-design ステージゲートで人間が Request Changes を選んだ直後に適用し、レビュアーを再実行する。本ファイルは produces ではない。
> ADR-008 Decision (3)（所見 2 の上流側）は inception 成果物のため先に訂正済み（`start` は記録のみ、検査は `next_decision`）。

1. `logical-components.md` §2「Bolt B3 の範囲拡張」に追記: 「`GraphReadError::NotFound { expected: WorkflowDefinitionId, actual: WorkflowDefinitionId }`
   （新変種）を `core-use-case` に追加し、`find_by_id` が要求 id とハーネス定義 id の不一致を fatal として返す（C4 の `NotFound`）。`InMemory…` /
   `…Impl` の両方で実装」。§4 受入手順に「`find_by_id` の id 不一致 → `NotFound` のテスト（Impl / InMemory）」を追加。
2. `logical-components.md` §1「既存」行を実体に合わせる: 「`orchestration/{autonomy_mode, jump_direction}.rs` は変更なし。`CheckboxState` は
   `workspace/checkbox.rs`（別コンテキスト `workspace` 所有、`use crate::workspace::CheckboxState`）で変更なし。`Status` は現在
   `workflow_execution.rs` にインライン定義 — B3 で private mod `status.rs` に切り出してファサードから `pub use`（module-visibility）」。
3. `security-design.md` §2 の `decide` 行から `NotStale` を除き、第 5 行「`stale_report`（クエリ）: `accepts_commands`（BR1.0）と staleness
   （stage < cursor ∧ Completed）— `Err(CommandError::{NotRunning, NotStale})`」を追加。§1 の「3 か所 + next_decision」に「+ stale_report の
   ガード」を注記。
4. `traceability.json` は変更なし（target の節番号は不変）。
5. （PR #27 CodeRabbit 再掲）`logical-components.md` §2 に C4 `NotFound { expected, actual }` / `HarnessIdentity { path, cause }` の契約行を追加し、Impl と
   InMemory の双方で同じ契約を検証する旨を明記（項目 1 と同じ — 実装済みの内容を設計へ写す）。
6. （PR #27 CodeRabbit 再掲）`security-design.md` §2 の `decide` 行から `NotStale` を除き `stale_report` の検査行を追加（項目 3 と同じ）。
   `nfr-design-questions.md` は人間確認済みバイト（エンジンが凍結）のため変更せず、P1 の「3 か所 + next_decision」の注記はここで補う。
