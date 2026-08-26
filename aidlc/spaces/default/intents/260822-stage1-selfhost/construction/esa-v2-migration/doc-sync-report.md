# doc-sync-report — Bolt B6（ADR-010 / event-store-adapter-rs v2.0.0 乗り換え）後の設計文書同期

> 実施日 2026-08-27。ブランチ `bolt/b6-esa-v2-conformist`。
> 正とした順序: (1) ADR-010（2026-08-26/27 の追記まで）、(2) developer-report-1 §6（8 件）、
> (3) developer-report-2 §8（13 件）、(4) 実装の現物
> （`modules/core/use-case/src/orchestration/` / `modules/core/interface-adapter/src/orchestration/`、
> 本家 crate の実ソース `~/.cargo/registry/.../event-store-adapter-rs-2.0.0/src/{types,event_store_for_sqlite}.rs`）。
>
> **新しい設計判断はしていない。** 確定済みの裁定と実装の実態を反映し、失効箇所には
> 家内書式（`~~打ち消し~~ → 失効（日付・理由・参照）`）で注記した。監査証跡（過去のレビュー所見・
> 履歴・退役記録）は一行も削除していない。コード・`formal/**`・`.claude/**`・`coding-rules/**` は未変更。
> `git add` / `commit` / `push` はしていない。

## 1. 直したファイルと件数

| # | ファイル | 主な変更 |
|---|---|---|
| 1 | `docs/specs/01-domain-model.md` | 状態 16 → **17 属性**（`last_updated_at`）。「スナップショット / **チェックポイント**の更新はジャーナル追記と同一 Tx 内」のうち**チェックポイント側を失効**（`catchup` は独立遷移。協定モデルも当初からそう書いている） |
| 2 | `docs/specs/10-orchestration.md` | §2.1 状態 17 属性・`version` = 不透明トークン・メメントの「serde を知らない」失効（memento 経由で検査点は 1 か所）・封筒の `id` 化。§3 ポート表: `EventStoreImpl` / 3 テーブル / `within_write_transaction` / `InMemoryWorkflowExecutionRepository` を失効し `WorkflowExecutionRepositoryImpl<S>` へ。§ 実装順の in-memory Gateway |
| 3 | `docs/specs/11-workspace.md` | `LockIdentity` 行・ポート表・W13・ITF 再生先・§10 確定事項の 5 か所。**登録簿の直列化機構は「確定」から「未決（U7 で裁定）」へ差し戻し** |
| 4 | `docs/specs/deviations.md` | D4 行の SQLite 表構成（本家 2 表 + `amadeus_projection_checkpoint`）。**観測可能な差は増えていない**ことを明記（パス・git 管理外・ロック dir 非生成・互換ファイル内容は不変） |
| 5 | `.../inception/contract-design/contract-summary.md` | 契約一覧の C3 / C6 行、**C3 全面**（ローカル `EventStore` 削除・`usize` 復帰・`JournalReadError`・約束 ④ 失効 + ⑥ 追加）、**C5**（封筒 `id` 化・ワイヤの全面改訂・契約 JSON の射程）、**C6 全面**（本家 DDL 逐語 + 我々の 1 表）、§3 所有ルールと検証、§4 未解決に**未決 2 件を追加**、`## Review` 追記の `u64` 改訂を失効 |
| 6 | `.../u3-.../functional-design/functional-spec.md` | 部分失効バナー + §1 配置表・§2 ポート・§3.1 / §3.2 / §3.4 / §3.5・§4 ワイヤ・§5 ITF・§7 テスト・§8 申し送り |
| 7 | `.../u3-.../functional-design/entities.md` | 部分失効バナー + YAML 12 エントリ（`EventStore` / `EventStoreError`→`JournalReadError` / `CorruptCause` 6→4 / `EventStoreImpl` / 3 表 / ワイヤ 2 型 / InMemory 2 型 / `WorkflowExecutionState` 17 属性 / `WorkflowExecutionRepositoryImpl` / **`JournalReaderImpl` 新設**）、`relationships` 全面、§2 要約、`## Review` に B6 追記 |
| 8 | `.../u3-.../functional-design/rules.md` | 部分失効バナー + BR1.1 / BR1.2 / BR1.3 / BR1.4 / BR1.5 / BR2.1〜BR2.7 / BR3.2 / BR3.5 / BR5.2、`applies_to` 6 行、§2 要約表 10 行 |
| 9 | `.../u3-.../functional-design/pending-revision.md` | 末尾に「2026-08-27 追記」節。項目 4 / 5 / 6 / 7 / 8 / 9 後半 / 10 後半を失効、項目 1 / 2 / 3 は有効、**B6 で新たに生じた申し送り 3 件**を追加 |
| 10 | `.../u3-.../nfr-design/logical-components.md` | 部分失効バナー + コンポーネント表 7 行（`journal_reader_impl` を追加）、§2 境界（**NFR4.1 再検討**）、§3 障害ドメイン 2 行、§4 テスト配置 3 行 |
| 11 | `.../u3-.../nfr-design/security-design.md` | 部分失効バナー + §2 検査点（**3 段 → 1 段**、`Schema` 前段廃止）、§3（Tx は本家 / `actual` の読み直し / **NFR3.5 は未決**）、§4 障害表、§5 依存（`=2.0.0` 完全固定・serde/chrono・thiserror 推移依存）、§7 決定性・ITF、§9 要求対応表 4 行 |
| 12 | `.../u2-.../functional-design/rules.md` | BR2.1（封筒の `id` 化）、**BR5.2（serde は memento 経由へ全面改訂）**、要約表 2 行 |
| 13 | `.../u2-.../functional-design/entities.md` | `WorkflowExecution.version`（不透明トークン・`usize`）+ `last_updated_at` 新設、`WorkflowExecutionEvent` の封筒（`id` / `occurred_at` の型 / `is_created` 新設）、serde の制約 2 行、`WorkflowExecutionSnapshot`（17 属性）、§要約 2 行 |
| 14 | `.../u2-.../functional-design/functional-spec.md` | §概要の「serde なし」失効、集約 API 面（`with_version` 削除・本家 `Aggregate` の実装）、フロー手順 2（封筒）と 4（**`with_version(v + 1)` 失効**）、BR 一覧行 |

**14 ファイル**（`git diff --stat` 実測: 605 insertions / 223 deletions。監査シャードの自動追記を除く）。

## 2. developer-report §6 / §8 の消化状況

### developer-report-1 §6（8 件）

| # | 状態 | 反映先 |
|---|---|---|
| 1 | 済 | C3（`usize` 復帰）、U3 rules BR1.1、U3 functional-spec §2、`## Review` 追記の失効 |
| 2 | 済 | C5 封筒（`id` = `WorkflowExecutionEventId`）、10 号 §2.1、U2 rules BR2.1 / entities |
| 3 | 済 | C6（snapshot payload 17 属性）、01 号、10 号 §2.1、U3 functional-spec §4.2、U2 entities |
| 4 | 済 | U2 entities の `WorkflowExecutionEvent`（`id` / `is_created` を新設として記載） |
| 5 | 済（**再検討は未着手のまま明記**） | U3 nfr-design 2 ファイル、U2 functional-spec、U2 entities（`occurred_at` 型）。NFR4.1 の再検討そのものは本同期の範囲外 |
| 6 | 済 | U2 rules BR5.2 全面改訂、U2 entities / functional-spec、10 号 §2.1 メメント |
| 7 | 済 | U3 functional-spec §3.2 に「本家は指定番号を**含む**」と明記 |
| 8 | 済 | 10 号 §3・11 号 §3 の ADR-006 参照を ADR-010 へ。ES 拡張語彙 `store` の由来としての ADR-006 参照は**残した**（撤回されていない部分のため） |

### developer-report-2 §8（13 件）

| # | 状態 | 反映先 |
|---|---|---|
| 1 | 済 | C6 全面（本家 DDL 逐語 + `amadeus_projection_checkpoint`）、U3 entities の 3 表エントリ、deviations D4、10 号 §3 |
| 2 | 済 | C6 バナー、U3 rules BR2.1、U3 nfr-design §2「前段 open」、U3 entities `Schema` 変種 |
| 3 | 済 | C5 バナー、U3 functional-spec §4、U3 rules BR2.5、U3 entities `EventPayloadWire` |
| 4 | 済 | 同上（`CorruptCause` 6 → 4 分類を U3 entities / rules に明記） |
| 5 | 済（**正本への追記は未実施**） | C5 バナー ③、U3 functional-spec §4、U3 nfr-design §7 に「ストアの payload は契約 JSON ではない」と明記。**`coding-rules` 正本（BR1.7 の射程）は編集許可外**のため未着手 — §4 参照 |
| 6 | 済 | C6 バナー、U3 functional-spec §4、U3 entities `StateWire` |
| 7 | 済（**未決として記載**） | C6・U3 rules BR2.1・U3 functional-spec §3.5・U3 nfr-design §3 / §4・U3 entities `JournalReaderImpl`。すべて「U7 で再裁定」と明記 |
| 8 | 済（**未決として記載**） | C3/C6 未解決表、11 号 3 か所、10 号 §3、U3 rules BR2.4（`status: superseded`）、U3 functional-spec §3.4、U3 nfr-design §3（NFR3.5） |
| 9 | 済 | 10 号 §3・ポート表前の補足、11 号 §3 / §5、C3 約束 ④、U3 entities / rules BR2.7 |
| 10 | 済 | C3 コードブロック、U3 rules BR1.1、U3 entities、U3 functional-spec §2 |
| 11 | 済 | U3 entities（`JournalReadError` へ改称・3 変種 + 失効 2 変種、`CorruptCause` 4 分類）、U3 rules BR1.5 |
| 12 | 済 | C6、U3 entities `CheckpointRow`、U3 rules BR2.6（Clock を持たない） |
| 13 | 済 | U3 rules BR2.6、U3 nfr-design logical-components（`clock` は**現在利用者なし**）、U3 functional-spec §8 |

### ADR-010 追記（実装後の裁定 3 件）

- **(1) genesis の初期 version は Gateway が写しに 1 を載せる** → C3 約束 ⑥、U3 functional-spec §2 / §3.1、U3 rules BR1.3、U3 entities（`FIRST_STORED_VERSION`）に反映。
- **(2) `Conflict` の `actual` は競合時の読み直し** → C3 の `store` doc、U3 rules BR1.3、U3 entities `RepositoryError.Conflict`、U3 nfr-design §3 に反映。
- **(3) `busy_timeout` は設定不可・U7 で再裁定** → 上記 §8-7 のとおり、すべて「未決」として記載。

## 3. grep 結果と分類

指定の grep（`EventStoreImpl|InMemoryEventStore|with_version|within_write_transaction|user_version|StateWire|global_seq_nr INTEGER`）を
`docs/specs/` / `inception/contract-design/` / `u3-event-store-repository/` / `u2-.../functional-design/` に実行した結果、
**残存はすべて意図的**である。0 件化していないのは、失効注記そのものが「何が失効したか」を名指す必要があるためで、
退役語の物理削除（B5 の BR3.1 のような grep 0 件ゲート）は本同期の目的ではない。

分類は次の 4 群。**直し漏れは 0 件**、**編集許可外が 2 ファイル 4 件**である。

### (A) 失効注記そのものが退役語を名指している（意図的）

`docs/specs/10-orchestration.md`（1 行）、`docs/specs/11-workspace.md`（6 行）、
`contract-summary.md`（3 行）、`u3-.../functional-spec.md`・`entities.md`・`rules.md`・
`nfr-design/*.md`・`pending-revision.md`、`u2-.../functional-spec.md`・`entities.md` の
`~~打ち消し~~` / `【失効】` / `status: superseded` / 部分失効バナー内。

### (B) 失効マーク配下に温存した原文（監査証跡・意図的）

- `u3-.../functional-spec.md` §2 の `event_store()` / `event_store_mut()` 行（バレット全体が `~~` で囲まれている）、
  §3.1 手順 1〜5、§3.2 手順 1〜3、§3.4 / §3.5 の旧手順（いずれも直前に失効注記あり）。
- `u3-.../entities.md` の `EventStoreImpl` / `JournalRow` / `SnapshotRow` / `EventPayloadWire` / `StateWire` /
  `InMemoryEventStore` エントリ本体（各エントリ冒頭に `# 失効（…）` コメント + `status: superseded` +
  `description` 先頭の `【失効】`）。
- `u3-.../functional-design/pending-revision.md` の項目 4 / 5 / 6 / 7 / 8 / 9（末尾の
  「2026-08-27 追記」節が 1 件ずつ失効理由を書いている）。
- `u3-.../entities.md` の `## Review` Findings 表と `Validation Tool Results`
  （レビュー所見の原文。直後に 2026-08-27 追記で所見 1・2 の失効と 16 → 17 属性を注記）。

### (C) 既に正しい記述（B5 で処理済み・変更不要）

- `u2-.../functional-design/rules.md` BR5.3 の `logic` 欄 — 「`with_version` は B6 委任 1 で削除済み」と
  既に書かれており、ブリーフの「BR5.3 は改訂済み」と一致する。
- `docs/specs/01-domain-model.md` の ADR-006 参照（「集約は Repository を呼ばず `.await` も持たない」）—
  ADR-010 が撤回したのは「本家 crate に依存しない」であって、この分離原則ではない。

### (D) 編集許可外で残した実在の直し漏れ（**要対応**、4 件）

| ファイル | 行 | 内容 | なぜ残したか |
|---|---|---|---|
| `u3-.../nfr-requirements/security-requirements.md` | 32 | NFR3.2 の健全性検査に `PRAGMA user_version` 由来の記述 | ブリーフの編集許可は `functional-design/*.md` と `nfr-design/*.md` のみで、`nfr-requirements/` は含まれていない |
| 同上 | 35 | **NFR3.5「登録簿の直列化 — `within_write_transaction` は busy_timeout（5000ms）内で直列化」** — 要求そのものが実現不能になっている。最も重い残件 | 同上 |
| 同上 | 51 | DoS の緩和策としての `busy_timeout` 依存 | 同上 |
| `u3-.../nfr-design/traceability.json` | 17 | `NFR3.5` の `target` が `"within_write_transaction と Busy の扱い（security-design §3）"` | 許可は `nfr-design/*.md`（`.json` ではない）。かつ traceability センサーが読むファイルなので、無断編集は避けた |

`security-requirements.md` の NFR4.1（依存最小化）も chrono / serde 採用で再検討対象だが、
同じ理由で未編集。**この 4 件 + NFR4.1 は、次に `nfr-requirements` を触れる際にまとめて処理するのが妥当**。

## 4. 判断が要ると思った箇所（本同期では触っていない）

1. **`coding-rules` 正本への追記 3 件**（編集禁止指示のため未着手 — developer-report-2 §8-5 / §9-E）。
   - BR1.7 の射程: 「契約 JSON（canon-json）」はストアの payload を含まない。正本は
     `u1-canon-json-goldens/functional-design/rules.md` BR1.7 と `docs/adr/0001` 決定 5 で、
     どちらも編集許可外。**文書側には「射程外である」旨を C5 / U3 §4 / nfr-design §7 に書いた**ので、
     読み手が誤解する経路は塞いである。
   - `thiserror` が本家経由で推移依存に入った件の注記。
   - `IntentId::value()` と `as_str()` の並立（委任 1 §7 D、B6 でも未処理）。
2. **NFR4.1（依存最小化）の再検討そのもの**。ADR-010 が「再検討が要る」と書いており、
   自前 ISO 8601 整形を撤去して chrono を採った以上、要求の文面を書き換えるか
   「chrono / serde は本家 trait の境界要求として受容する」と明記するかの**裁定が要る**。
   本同期では「再検討が要る」と**問題提起の形で**書き、要求文面には手を入れていない。
3. **`security-design.md` §2 検査点 2 の縮退**（自分で気づいた点、報告のみ）。
   旧設計の「段 2 = ワイヤ → ドメイン値の parse-don't-validate」は、ドメイン型が直接 `Deserialize` を
   持つようになったことで**段ごと消えた**。集約全体の不変条件は `from_state()` が守るが、
   個々の Domain Primitive（`StageSlug` / `IntentId` 等）が serde 経路で parse 検証を保つかは
   型ごとの実装依存であり、**旧設計ほど網羅的ではない可能性がある**。文書には
   「申し送り」として記載したが、実装の実地確認と要否の裁定はしていない。
4. **`docs/specs/01-domain-model.md` §3.3 の代表不変条件の文面**。
   「スナップショット / チェックポイントの更新はジャーナル追記と同一 Tx 内に限られる」のうち
   チェックポイント側は、`journal_protocol.qnt` が当初から `catchup` を独立遷移として書いており
   実装（別接続・別 Tx）とも一致する。**仕様文の側が最初から不正確だった**と判断して訂正注記を入れたが、
   これが「新しい設計判断」に踏み込んでいないかは確認してほしい（モデルと実装の両方が
   独立遷移で一致しているので、事実の訂正の範囲だと考えている）。
5. **`entities.md` / `functional-spec.md` に残る `WorkflowExecutionSnapshot` の型名**（U2 側）。
   B5 で `WorkflowExecutionState` へ改名済みだが U2 の設計文書は旧名のままの箇所がある。
   B6 起因ではない B5 の同期漏れなので、**本同期のスコープ外として触っていない**
   （`WorkflowExecutionSnapshot` エントリの description には改名済みである旨だけ追記した）。

## 5. 未決として明記した項目（「解決済み」と書いていないことの確認）

| 項目 | 記載箇所 | 文言 |
|---|---|---|
| BR2.4 / 登録簿 `intents.json` の直列化 | C3/C6 未解決表、11 号 §2.2 / §3 / §6 W13 / §10、10 号 §3、U3 rules BR2.4（`status: superseded`）、U3 functional-spec §3.4、U3 nfr-design security-design §3 / §9（NFR3.5） | 「**口ごと削除され代替は未定 — U7 で裁定**」「**『解決済み』ではなく未決である**」 |
| `busy_timeout` | C6 未解決表、U3 rules BR2.1、U3 functional-spec §3.5、U3 nfr-design §3 / §4 / logical-components §3、U3 entities `JournalReaderImpl` | 「**単一プロセス前提の現状は受容し、U7 の並行モデルと併せて再裁定**」 |

11 号 §10 の「確定（2026-08-23、Bolt B5）」は**打ち消して「再び未決へ差し戻し」**と明記した
（確定済みと読める記述を残すと U7 が裁定を素通りするため）。

## 6. 保全した Published Language の固定トークン

`journal` / `snapshot` / `pkey` / `skey` / `aid` / `seq_nr` / `payload` / `occurred_at` /
`last_updated_at` / `version`（本家 DDL と trait の逐語）、`EventStore` / `Aggregate` / `Event` /
`AggregateId` / `set_version` / `is_created` / `type_name`（本家 trait の綴り）、
ID（`C3`〜`C7`・`BR*.*`・`NFR*.*`・`FR*.*`・`ADR-***`・`W13`・`J1`〜`J6`・`D6`）、
H2 見出し（`## Review` / `## Sources` 等）、`READY` / `NOT-READY` の判定語は一切変更していない。
本家 DDL は crate 実ソース（`event_store_for_sqlite.rs` の `CREATE_SCHEMA_SQL`）から逐語で転記した。
