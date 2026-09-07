# pending-revision — contract-design（ステージゲート / 契約改訂で処理）

1. C3 `EventStore<AID, A, E>` の数値パラメータ `usize` → **`u64`**（実ドメイン型 `seq_nr` / `version` = u64、Bolt B3 実装）。C3 は「Rust trait が正本」と宣言しており、
   U3（Bolt B5）がユースケース層に置く trait が u64 で確定する。U5 / U6 はその trait を実装・消費するだけで別定義を持たない（型不一致は起きない）。
   contract-summary.md の C3 本文を次の契約改訂機会に u64 へ同期する（U3 FD BR1.1、nfr-design レビュー所見 3）。
2. C5 観測契約 / `IntentExecutionEvent.Recomposed`（contract-summary :433-436 の `payload: { skipped: [slug], added: [slug], stages_in_scope }` と `projects_to: { audit: [RECOMPOSED] }`）に、監査行 `Stages skipped` / `Stages added` の**列挙順 = 計画（intent の stages）の文書順**を明記する。現行コード（b51 #118、`modules/core/read-model-updater/src/workspace/projection.rs` の `in_document_order`、テスト `the_recomposed_spelling_follows_the_document_order_not_the_alphabet`）は文書順で描き、ドメインの `StageSlugSet`（BTreeSet）の辞書順にしない。上流 `.claude/knowledge/aidlc-shared/audit-format.md:93` も順序に沈黙しているため、契約側で固定しないと投影の再実装で辞書順に戻る余地がある。文面案: `# stages_skipped / stages_added の列挙順は intent.stages の文書順（b51 2026-09-06）` を C5 の RECOMPOSED 行と §4 の Recomposed エントリに付す（U4 functional-design 再走 2026-09-07、gap-measurement G-1、rules.md BR2.1）。
