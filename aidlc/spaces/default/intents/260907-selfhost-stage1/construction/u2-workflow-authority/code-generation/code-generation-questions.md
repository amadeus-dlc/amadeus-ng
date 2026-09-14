# U2の実装計画の確認

## Plan Approval

[Question]: Approve this exact Code Generation plan?

[code-generation-plan.md](code-generation-plan.md)と、その中のTesting Contract、および[unit-test-instructions.md](unit-test-instructions.md)を承認対象とする。

**2026-09-13 改訂**: Step 1–8（報告結果の CQS 是正、CLI・承認受領・Claude フック・補助更新の Rust 接続、受入検査の 2.7.1 移行）は前試行で実装・検証済み（B1 コミット済み）。本改訂は、bugfix 実地スモークが踏むが未配線の 8 サブコマンドを Step 9 として TDD で追加する。対象は `aidlc-state lookup`（4 動詞）と `aidlc-utility` の `project-description` / `scope-table` / `stage-table` / `codekb-scope-diff` / `codekb-snapshot` / `codekb-path` / `codekb-publish`。4 群（stage/scope 読取・project-description・codekb 読取・codekb 書込）に分け、各群を Red→Green→Refactor で実装し、`required_surface_contract.rs` を同期する。B1 統合の確認は Step 10 へ繰り下げた。実地スモークと native doctor の達成を、この単位の実装完了で代用しない。

[Approval Fingerprint]: sha256:3477fec043bf80eea543e8667d9573a701faab71afaa6fcc088700a133c8498d

- Approve Plan — 計画とテスト境界を承認してU2を実装する。
- Request Changes — 計画またはテスト手順を修正する。

[Answer]: Approve Plan
