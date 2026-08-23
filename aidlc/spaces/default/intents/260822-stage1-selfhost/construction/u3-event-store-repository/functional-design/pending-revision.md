# pending-revision — U3 functional-design（ステージゲートで処理）

> レビュー iteration 1（NOT-READY: Critical 1 / Major 2）の所見 3 件は**本文に反映済み**（rules.md BR1.1 / BR1.2 / BR1.3 / BR2.3、entities.md EventStore /
> WorkflowExecutionRepositoryImpl、functional-spec.md §2 / §3.1 / §3.2 — 2026-08-23、コミット 49964cf / 966aaac）。レビュー予算 1 のため再レビューは不可で、
> entities.md 末尾の `## Review` は履歴として NOT-READY のまま残る（nfr-design レビュー所見 2）。

1. ゲートで: `entities.md` の `## Review` に「3 件とも本文で解消済み（iteration 1 後の是正、再レビューは予算外）」の注記を追記するか、Request Changes で
   レビュアーを再実行して iteration 2 の受領を得る（どちらかをオーナー裁定）。
2. BR4.2 の正規表現を `^[0-9]{6}-[a-z0-9]+(?:-[a-z0-9]+)*$`（連続ハイフン拒否 — kebab の家内規約、11 号 §2.2）に是正（委任 1 設計質問 1 — 実装は拒否側）。
3. functional-spec §4.1 `GateApproved.phase_boundary` のワイヤ形を `{from_phase: string, to_phase: string} | null`（入れ子）に改訂（委任 2 の具体化 — PhaseBoundary は
   PhaseId の組で、文字列 1 本に畳むには区切り記号の発明が要る）。
4. BR2.3 / functional-spec §3.1: スナップショット payload の `version` も新 version（= event.seq_nr）で保存する（`with_version(new_version).state()` を符号化）— 列と
   payload の一致、J3 の単純化（委任 2 設計質問 4 の裁定）。
5. entities.md `SqliteEventStore` / functional-spec §2: 公開面に `open_with_busy_timeout(path, clock, timeout)` を追加（`open` は 5000ms に委譲 — Busy 超過の観測テスト用、委任 3 設計質問 1）。
6. BR2.3 / functional-spec §3.1 手順 4: `UPDATE snapshot` の SET に `schema_version` を含める（将来のワイヤ版上げでの静かな破損経路を塞ぐ — 委任 3 設計質問 2）。
   競合時の `actual` は Tx 内で読んでよい（rollback 前後で同値）。
7. **型名**: `SqliteEventStore` → `EventStoreImpl`（gateway-taxonomy §5「技術接頭辞は使わない — 格納形式は実装の内部詳細」、ADR-003 の「Repository → EventStoreImpl(sqlite client)」
   の語）。entities / functional-spec / logical-components / 10 号 §3 / 11 号 §3 / components.md の表記を同期（委任 3 設計質問 3 — コンダクタ裁定、B5 統合で改名）。
8. `within_write_transaction` の閉包引数が `rusqlite::Transaction` を公開面に出す点は U7 の設計で再確認（委任 3 設計質問 4）。`persist_event(event, version)` は version を
   楽観前提として検査（両実装同義 — 委任 2 §C-5 / 委任 3 設計質問 5）を BR2.3 に追記。
9. **内部可変性の撤回**: `WorkflowExecutionRepository::store` を `&mut self` に、`WorkflowExecutionRepositoryImpl` / `EventStoreImpl` / InMemory 両型から `RefCell` /
   `Rc<RefCell<_>>` / 手書き `Clone` を除去（オーナー裁定 2026-08-23、正本 `coding-rules/interior-mutability.md` / `command-query-separation.md` を新設）。委任 8 で実装
   是正済み・本文同期済み。共有契約 C3（`inception/contract-design/contract-summary.md`）の `store(&self, …)` と数値パラメータ `usize` は、オーナー裁定（2026-08-23）
   により `&mut self` / `u64` へ改訂済み。C3 の所有者は U5/U6 だが、U3 の実装が正であることを本改訂で確定した。code-generation レビュー Major 所見 1 はこれで解消。
