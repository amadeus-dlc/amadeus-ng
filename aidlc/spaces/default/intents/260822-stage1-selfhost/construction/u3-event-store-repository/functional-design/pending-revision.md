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
10. **ファクトリ命名**: ~~`StorePath::for_space` → `StorePath::of`（複数の値を集約 = `of`）~~
    → **撤回**（命名監査 F8、2026-08-24）。`of` は「与えた値を包む」に読めるが実体は固定
    レイアウトの導出であり、`for_space` のほうが「space のためのパス」と言えている。
    正本 `factory-naming.md` 原則 1（正確な語が表の動詞に勝つ）に反していた。`for_space` のまま。
    あわせて
    `InMemoryWorkflowExecutionRepository::{new(), with_store(store)}` → `new(store)` + `Default`
    （コンストラクタ相当は `new` に統一。SQLite 実装 `WorkflowExecutionRepositoryImpl::new(store)` と同形）。
    正本 `coding-rules/factory-naming.md`（オーナー裁定 2026-08-24）。本文同期済み。

---

## 追記 2026-08-27 — Bolt B6（ADR-010 / event-store-adapter-rs v2.0.0 へ乗り換え）による失効

上記 1〜10 は B5 時点の申し送りであり、**B6 で自前ストアを全削除したため次の項目は前提ごと失効した**
（履歴として残す）:

- **項目 4**（スナップショット payload の `version` を新 version = `event.seq_nr` で保存 /
  `with_version(new_version).state()`）→ **失効**。`with_version` は削除され、version は
  **ストアが採番する不透明トークン**になった（BR5.3 / ADR-010 追記 (1)）。`version = seq_nr` という
  等式そのものが否定されている。
- **項目 5**（`open_with_busy_timeout(path, clock, timeout)` の公開）→ **`EventStoreImpl` 側は失効**。
  本家の接続には `busy_timeout` を設定できない。同名のコンストラクタは
  `JournalReaderImpl::open_with_busy_timeout` として**我々の別接続にのみ**残っている。
  本家接続の待ち時間は**未決であり U7 で裁定**する。
- **項目 6**（`UPDATE snapshot` の SET に `schema_version` を含める）→ **失効**。SQL も
  `schema_version` 列も本家スキーマには無い。なお「競合時の `actual` は読んでよい」という判断は
  形を変えて生きている — 本家は整形済み文字列しか返さないので、競合時のみ
  `get_latest_snapshot_by_id` を 1 回読み直して `actual` を作る（ADR-010 追記 (2)）。
- **項目 7**（`SqliteEventStore` → `EventStoreImpl` の改名）→ **対象ごと消滅**。`EventStoreImpl` は
  ファイルごと削除した。技術接頭辞を型名に出さないという規則（gateway-taxonomy §5）自体は
  `WorkflowExecutionRepositoryImpl<S>` に受け継がれている（格納形式は型引数 `S` の選択であって型名には出ない）。
- **項目 8**（`within_write_transaction` の閉包引数が `rusqlite::Transaction` を公開面に出す点を
  U7 で再確認 / `persist_event(event, version)` の version 検査）→ **口ごと削除**。U7 へ持ち越すのは
  「公開面の是非」ではなく**登録簿の直列化機構そのもの**になった（ADR-010 は「登録簿を SQLite の
  テーブルへ移し RMU の投影対象にする」を筋と書いている。**未決**）。
- **項目 9** の後半（C3 の数値パラメータ `usize` → `u64` へ改訂）→ **撤回**。本家の `usize` に戻した
  （借り物の契約を我々のドメイン型に合わせて書き換えていたこと自体が
  `coding-rules/upstream-contracts.md` 違反だった）。`store(&mut self, …)` への改訂は**不変**。
- **項目 10** の後半（`InMemoryWorkflowExecutionRepository::new(store)` + `Default`）→ **対象ごと消滅**。
  テストダブル型は削除され、`WorkflowExecutionRepositoryImpl::in_memory()` が本家 memory
  バックエンドを内包する。`StorePath::for_space` は**不変**。

**引き続き有効**: 項目 1（`## Review` の扱い）、項目 2（`IntentDirName` の正規表現）、
項目 3（`GateApproved.phase_boundary` のワイヤ形 — ドメイン型の serde 表現として今も有効）。

**2026-08-27 追記**: 項目 3 の裁定（`{from_phase: string, to_phase: string} | null` の入れ子形）が
`functional-spec.md` §4.1 のワイヤ表に未反映のまま `phase_boundary: string | null` と書かれていた
食い違いを解消した（同ファイル §4.1 を入れ子形に訂正 — 新しい設計判断ではなく本項目の裁定への追従。
実装 `PhaseBoundary { from_phase, to_phase }`（`#[derive(Serialize, Deserialize)]` の既定表現）とも
一致することを確認済み）。

### B6 で新たに生じた申し送り（正本への追記候補）

1. **BR1.7 の射程**: 「契約 JSON（canon-json）」はストアの payload を含まない — 本家が
   `serde_json::to_vec` で書くため。射程を `coding-rules` 正本に一行足したい。
2. **`thiserror` が推移依存に入った**（本家経由）。我々が直接使わない方針は不変だが、
   「推移依存として存在する」ことを正本に注記したい。
3. **`IntentId::value()` と `as_str()` の並立**（委任 1 §7 D、B6 でも未処理）。

---

## 追記 2026-09-07 — unit-major 再走（Keep）の再レビュー READY と新規所見

- **経緯**: 4 成果物（entities / rules / functional-spec / traceability、2026-09-05 #112 是正版）を **Keep**（本文無変更、オーナー会話回答）で再採用し、
  独立レビュー iteration 1（advisory、challenge `review:30b70b9a25be173ec55f85af6cdb7893`）は **READY**（Critical 0 / Major 1 / Minor 4）。
  前回（2026-09-05 NOT-READY）の ID 1〜3 / R-04〜R-09 は**全件 Resolved**（レビュアーが現行 HEAD `f2b6b6a9` で実測）。
  旧 `## Review` 節はレビュー要求の付録規則（付録は 1 節のみ）により `functional-spec.md` 末尾から切り離し、`review-history-20260905.md` へ退避した。
- **上記「引き続き有効」項目 1〜3 の現状**: 項目 1（`## Review` の扱い）→ 2026-09-07 の READY 節に置き換わり履歴は退避ファイルへ（**解消**）。
  項目 2（BR4.2 の正規表現）→ 本文・実装テスト `intent_dir_name.rs:165-168` と一致、R-09 Resolved（**解消**）。
  項目 3（§4.1 の入れ子形）→ 2026-08-27 追記どおり反映済み（**解消**）。有効な旧項目は残っていない。

### 新規所見（次の U3 FD 書込機会で適用 — per-unit・gate false のため承認ゲートには載らない）

11. **R-11（Major）派生ビューの欠落**: `functional-spec.md` に、ステージ定義が求める ER 図（`entities.md` の YAML から派生する mermaid `erDiagram`）と
    規則要約（`rules.md` からの派生表）が無い（本文の mermaid 0 件、required-sections センサーは雛形未配備のため検出しない）。
    適用文面: §2 の直後に「### 2a. エンティティ関係（派生ビュー — 正本は entities.md）」として `IntentExecution` / `IntentExecutionEvent` /
    `IntentExecutionId` / `SnapshotEnvelope` / `EventEnvelope` / `IntentExecutionRepositoryImpl<S>` / `SnapshotStrategy` の関係を `erDiagram` で描き
    テキスト代替を併記、§5 の直後に「### 5a. 規則要約（派生ビュー — 正本は rules.md）」として BR1.1〜BR5.2（23 本）の 1 行要約表を置く。
12. **R-10（Minor）components.md の未参照**: upstream-coverage センサーが `unreferenced: ["components"]` で FAIL（既定重大度 advisory）。
    この Unit で `components.md` に触れているのは質問票だけで、呼称も退役名 `PersistenceGateways`（現行名 `CommandGateways`）。
    適用文面: functional-spec §1 の所有表の直後に「上流の論理コンポーネント対応: `../../../inception/domain-design/components.md` の `CommandGateways`
    （Repository 実装と本家ストアの接続）および `CoreInfrastructure`」の 1 行を足す。質問票の呼称は AI 記述（人間の逐語回答ではない）のため
    「（現行名 `CommandGateways`）」の括弧注記を添えてよい。
13. **R-12（Minor）§7 の Definition 側件数の鮮度**: 「共通契約 14 件と実装固有 12 件の計 26 件」は 2026-09-05 時点の値。#110（`a1ddb37d`）で
    契約マクロにテスト 2 名が増え、現行 HEAD の実測は `workflow_definition_repository_contract` 18 件 + `workflow_definition_repository_impl_test`
    12 件 = **30 件**（コンダクタ再実測 2026-09-07、`cargo test --locked -p core-command-interface-adapter --test …`）。U3 側の「22 + 23 = 45 件」は一致。
    適用文面: 「計 26 件（2026-09-05 時点。2026-09-07 再実測は契約 18 + 実装 12 = 30 件）」。
14. **R-13（Minor）traceability.json の BR5.1 / BR5.2 二重登録**: coverage で FR1 の target に BR5.1、NFR3 の target に BR5.2 を置きながら、
    reverse では両者を「要求 ID を持たない」N/A にしている。適用文面: reverse の BR5.1 を `{"status": "OK", "target": "FR1"}`、BR5.2 を
    `{"status": "OK", "target": "NFR3"}` に改め、横断方針の説明文は story-map 備考へ移す（教訓「OK target は単一 ID」）。
15. **R-14（Minor）§7 にクラッシュ再構成の検証行が無い**: unit-of-work の U3 合格条件 3 つのうち「クラッシュ後の再構成」に対応する行が §7 の表に無い。
    `modules/app/aidlc/tests/crash_reconstruction_test.rs` に実在（**5 件** — コンダクタ再実測 2026-09-07 `cargo test --locked -p aidlc --test crash_reconstruction_test` = 5 passed。レビュアー R-14 の「11 件」は誤り）。適用文面: §7 表に
    「| クラッシュ再構成 | app/aidlc/tests/crash_reconstruction_test.rs | 途中停止後の journal / snapshot からの再構成（5 件） |」を追加。
