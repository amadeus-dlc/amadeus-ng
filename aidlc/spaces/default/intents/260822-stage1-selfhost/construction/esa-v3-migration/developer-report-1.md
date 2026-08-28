# B7 委任報告書 1 — event-store-adapter-rs v3.0.0（EventEnvelope API）への乗り換え

Conversation language: 日本語
ブランチ: `bolt/b7-esa-v3-event-envelope`
コミット: `b30a294`（乗り換え本体）+ 本報告書

## 1. 変更概要

本家 v3.0.0 は `Event` / `Aggregate` trait を廃し、輸送のメタデータ（集約識別子・通番・
発生時刻・型判別子）を `EventEnvelope` / `SnapshotEnvelope` が運ぶ形になった。したがって
ドメイン型が実装する本家 trait は `AggregateId`（`IntentId`）**だけ**になり、ドメイン
イベントと集約状態は素の serde 型（本家の語で payload）になる。封筒を組むのはアダプタ層の
Repository である。

署名が 3 クレートを横断して一斉に変わるため、乗り換えは 1 コミットにまとめた
（`coding-rules/no-backward-compatibility.md`「改名や署名変更は呼出側ごと一斉に直す」）。

### 削除した型

| 型 | 理由 |
| --- | --- |
| `WorkflowExecutionEvent`（旧・封筒 struct） | 封筒は本家 `EventEnvelope` が担う。同名を 12 変種 enum が引き継いだ |
| `WorkflowExecutionEventId` | 識別子は封筒の `(aggregate_id, seq_nr)`。ファイルごと削除（106 行） |
| `WorkflowExecutionEventPayload`（enum 名） | `WorkflowExecutionEvent` へ改名。「Payload」は輸送の語でありドメインの語ではない（ubiquitous-language） |

### 削除したフィールド・メソッド

| 所在 | 削除したもの |
| --- | --- |
| 旧封筒 struct | `id` / `schema_version` / `occurred_at` の 3 フィールドと `SCHEMA_VERSION` 定数・`schema_version()` / `payload()` アクセサ |
| `WorkflowExecution`（集約） | `version` フィールド、`impl Aggregate`（`id` / `seq_nr` / `version` / `set_version` / `last_updated_at`） |
| `WorkflowExecutionState`（memento） | `version` フィールド・アクセサ・ビルダーメソッド（17 → **16 属性**） |
| `WorkflowExecutionRepositoryImpl` | `check_preconditions`（検査対象が構成不能になった）、`FIRST_STORED_VERSION` 定数 |
| `JournalReaderImpl` | `decode_event` の payload 内メタ照合（#500）と `schema_version` 検査（#466） |

### 新設した型

| 型 | 所在 | 役割 |
| --- | --- | --- |
| `JournalEntry` | `core-use-case`（`journal_entry.rs:28`） | 横断読取が返す 1 行（global_seq / intent_id / seq_nr / occurred_at / event） |
| `RehydratedWorkflowExecution` | `core-use-case`（`rehydrated_workflow_execution.rs:26`） | 再水和した集約 + ストア採番 version |
| `EVENT_MANIFEST` 定数 | `core-interface-adapter`（`event_manifest.rs:16`） | `workflow-execution-event/1`。Repository が書き JournalReaderImpl が照合する |

### 行数増減

`45c323c`（B6 マージ）から本コミットまで、`modules/**` の実測:

```
24 files changed, 1410 insertions(+), 1136 deletions(-)   → 純増 +274 行
```

プロダクトコードだけを見ると、削除（旧封筒 + `WorkflowExecutionEventId` + `version` 経路 +
二重照合）と新設（`JournalEntry` + `RehydratedWorkflowExecution` + `EVENT_MANIFEST` +
封筒組み立て）がほぼ相殺し、増分の大半は**テストと doc コメント**である。

## 2. 固定裁定 1〜9 の実施箇所

### 裁定 1 — `=3.0.0` ピン（`sqlite` feature）

- `Cargo.toml:113` — `event-store-adapter-rs = "=3.0.0"`（委任者が af747ef で先行コミット済み）
- `modules/core/interface-adapter/Cargo.toml:28` — `features = ["sqlite"]`（現状維持）
- MSRV 1.94.1 ≤ 固定ツールチェーン 1.95.0。`cargo update -p event-store-adapter-rs` が
  2.0.0 → 3.0.0 に解決し、他 5 依存は据え置き。
- 実測: crates.io の 3.0.0 と参照コピー（scratchpad）の `event_envelope.rs` / `types.rs` は
  `diff` でバイト一致することを確認済み。
- `core-use-case` からは本家依存を**削除**した（`modules/core/use-case/Cargo.toml`）。
  ポートが本家型を出さなくなったため（裁定 6）。

### 裁定 2 — ドメインイベントの payload 純化

- `workflow_execution_event.rs:36` — `pub enum WorkflowExecutionEvent`（旧
  `WorkflowExecutionEventPayload` の改名。12 変種は不変）
- `workflow_execution_event.rs:1-27` — 旧封筒 struct と `impl Event` を削除した旨を
  モジュール doc に記載
- `workflow_execution_event_id.rs` — ファイル削除
- `orchestration/mod.rs:94-100` — ファサードの `pub use` を更新
- serde（`Serialize` / `Deserialize`）は維持。回帰テスト
  `the_serialized_event_carries_no_transport_metadata`（`workflow_execution_event.rs:613`）が
  payload JSON に `seq_nr` / `occurred_at` / `schema_version` / `aggregate_id` / `manifest`
  が現れないことを綴りで固定する。

### 裁定 3 — seq_nr / 連続性検証はドメイン責務のまま

- `workflow_execution.rs:98-101` — 集約は `seq_nr: usize` / `last_updated_at` を維持
  （`version` のみ削除）
- `workflow_execution.rs:773` — `apply_event(&mut self, seq_nr: usize, occurred_at:
  DateTime<Utc>, event: &WorkflowExecutionEvent)` へ署名変更。`SequenceGap` /
  `SequenceExhausted` / `check_invariants` は現行どおり
- `workflow_execution.rs:460` — `commit` は適用後にイベントを返すだけ。封筒は作らない
- `workflow_execution.rs:237` / `:246` / `:254` — 旧 `Aggregate` trait の口を inherent
  アクセサとして持ち直した（`intent_id()` / `seq_nr()` / `last_updated_at()`）。
  `id()` → `intent_id()` の改名は、外部 trait の縛りが消えたのでユビキタス言語へ戻した
  もの（memento の `intent_id()` と綴りが揃う）
- `workflow_execution_repository_impl.rs:158` — Repository が
  `EventEnvelope::new(aggregate.intent_id().clone(), aggregate.seq_nr(),
  *aggregate.last_updated_at(), event).with_manifest(EVENT_MANIFEST)` を組む（`:168`）

### 裁定 4 — version を集約と memento から削除（2026-08-29 改訂版に準拠）

- `workflow_execution.rs:98` — 集約のフィールド列から `version` が消えた
- `workflow_execution_state.rs:53-54` — memento は 16 属性（`seq_nr` / `last_updated_at`）
- `rehydrated_workflow_execution.rs:26` — 再水和レコード（private フィールド + アクセサ
  `aggregate()` / `version()` / `into_aggregate()`）
- `workflow_execution_repository.rs:50` — `find_by_id` の戻り値が
  `RehydratedWorkflowExecution`
- `workflow_execution_repository.rs:67` — `store(&mut self, event, aggregate,
  expected_version: usize)`
- `workflow_execution_repository.rs:78` — `const UNPERSISTED_VERSION: usize = 0`
  （genesis が提示する版。呼出側に裸の `0` を書かせない）
- `workflow_execution_repository_impl.rs:306-321` — 新規作成・更新とも
  `persist_event_and_snapshot(envelope, aggregate.clone(), expected_version)`。
  分岐は本家 v3 が封筒の `seq_nr == 1` から導出する
- `genesis の set_version(1) ハック`は消滅（`FIRST_STORED_VERSION` 定数ごと削除）

**改訂の経緯**: 初稿の「更新は `persist_event(envelope, snapshot.version())`」は実装不能
だった。詳細は §5。

### 裁定 5 — manifest 定数

- `event_manifest.rs:16` — `pub(super) const EVENT_MANIFEST: &str =
  "workflow-execution-event/1"`
- 置き場所はコンテキスト直下の中立モジュール。書くのは Repository（コマンド側）、照合するのは
  JournalReaderImpl（クエリ側）なので、どちらか一方に置くと片方が相手を知ることになる
- `workflow_execution_repository_impl.rs:168` — 書込側
- `journal_reader_impl.rs:336` — 不一致・欠落を `Corrupt(UndecodablePayload)` で拒否
- 到達テスト `a_row_whose_manifest_is_not_ours_is_corrupt`
  （`journal_reader_impl.rs:1330`）が `""` / `"workflow-execution-event/2"` /
  `"some-other-type/1"` の 3 通りを踏む
- payload 内メタ照合（#500）は payload からメタが消えたので二重化ごと消滅した

### 裁定 6 — `JournalReader` ポートの戻り値

- `journal_entry.rs:28` — `JournalEntry`（global_seq / intent_id / seq_nr / occurred_at /
  event。private フィールド + アクセサ）
- `journal_reader.rs:37-40` — `events_after(&self, after) -> Result<Vec<JournalEntry>, _>`
- `journal_reader.rs:14-16` — 本家 `EventEnvelope` をポートから出さない理由（U4 で RMU
  クレートへ所有が移る — ADR-009 2026-08-28 追記）を trait doc に明記
- `journal_reader_impl.rs:329` — `IntentId::parse(&row.aggregate_id)`。失敗は
  `Corrupt(InvariantViolation)`。到達テスト
  `a_row_whose_aggregate_id_is_not_an_intent_id_is_corrupt`（`:1252`）
- `core-use-case` の `Cargo.toml` から本家依存を削除（ポートが本家型に触れなくなった）

### 裁定 7 — 不変（rowid カーソル / checkpoint 表 / アンカー照合 / busy_timeout / CREATE なし）

いずれも**手を触れていない**。スキーマガードと SELECT だけを v3 へ張り替えた。

- `journal_reader_impl.rs:74` — `amadeus_projection_checkpoint`（anchor_aid /
  anchor_seq_nr 列を含む）は現行のまま
- `journal_reader_impl.rs:248` / `:254` — アンカー照合（B6 で導入）は現行のまま
- `journal_reader_impl.rs:195` — `Connection::open_with_flags`（`SQLITE_OPEN_CREATE`
  なし — #511）は現行のまま
- `journal_reader_impl.rs:203` — `busy_timeout` は現行のまま
- `journal_reader_impl.rs:85` — `SELECT rowid, aid, seq_nr, payload, occurred_at,
  manifest FROM journal WHERE rowid > ?1 ORDER BY rowid`（occurred_at / manifest を追加）
- `journal_reader_impl.rs:502` — スキーマガードのピンを v3 DDL へ張り替え
  （`manifest TEXT NOT NULL DEFAULT ''` 追加）。索引 DDL
  `CREATE UNIQUE INDEX journal_aid_seq_nr_idx ON journal (aid, seq_nr)` は v2 と同一
- `journal_reader_impl.rs:359` — `occurred_at` 列（epoch ナノ秒）→ `DateTime<Utc>` の
  変換。往復は `the_journal_reads_every_event_in_global_order`（`journal_reader_impl_test.rs`）と
  `a_sound_row_becomes_a_journal_entry_with_every_material` が固定する

### 裁定 8 — v3 の新契約への追従

- `EventStoreWriteError::ContractViolation` の match 腕を追加
  （`workflow_execution_repository_impl.rs:212`）。ストレージ障害ではなく我々が封筒と
  `expected_version` を組み違えたときにしか出ないので `Corrupt(UndecodablePayload)` へ写す。
  到達テストは 2 本 — 単体（`a_write_failure_is_mapped_by_its_kind`）と契約
  （`a_genesis_with_a_non_zero_version_is_a_contract_violation`、memory / sqlite 両方）
- 不在集約への更新が一律 `OptimisticLockError` になる点は
  `RepositoryError::Conflict` 写像（`:207`）が現行どおり吸収する
- `with_keep_snapshot_count` は**呼んでいない**（保持ポリシー未使用）。`Result` 化の影響なし

### 裁定 9 — Quint モデルは変更しない

- `formal/**` は 1 バイトも触っていない（`git diff --stat -- formal/` が空）
- ITF 準拠テストは新シグネチャへ追従のみ:
  - `engine_loop_conformance.rs` — `started.seq_nr()` → `agg.seq_nr()`（本家 trait の
    import 削除のみ）
  - `journal_protocol_conformance.rs` — `Writer` が `version` を持つ形へ。モデルの
    `loadedVersion` は**再水和レコードの版**へそのまま射影される
- `bash scripts/quint-gate.sh` 緑（§3-5）

## 3. 受入基準の実行ログ

### 1. `cargo fmt --all --check`

```
$ cargo fmt --all --check
(exit 0)
```

**PASS**（出力なし = 差分なし）

### 2. `cargo clippy --workspace --all-targets -- -D warnings`

```
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
(exit 0)
```

**PASS**

### 3. `cargo lint`

```
$ cargo lint
(exit 0)
```

**PASS**（違反 0 件 = 出力なし）

### 4. `cargo test --workspace`

```
suites=31 passed=621 failed=0
```

主要スイートの末尾:

```
test result: ok. 259 passed; 0 failed; ... (core-domain lib)
test result: ok.   5 passed; 0 failed; ... (upstream_event_store_conformance)
test result: ok.   1 passed; 0 failed; ... (engine_loop_conformance)
test result: ok.  46 passed; 0 failed; ... (core-use-case lib)
test result: ok.  83 passed; 0 failed; ... (core-interface-adapter lib)
test result: ok.  20 passed; 0 failed; ... (workflow_execution_repository_contract)
test result: ok.  14 passed; 0 failed; ... (workflow_execution_repository_impl_test)
test result: ok.  13 passed; 0 failed; ... (journal_reader_impl_test)
test result: ok.   5 passed; 0 failed; ... (crash_reconstruction_test)
test result: ok.   1 passed; 0 failed; ... (journal_protocol_conformance)
```

**PASS**（全 621 テスト緑）

### 5. `bash scripts/quint-gate.sh`

```
==> summary
  [PASS] typecheck formal/orchestration/engine_loop.qnt
  [PASS] typecheck formal/orchestration/stop_hook.qnt
  [PASS] typecheck formal/orchestration/journal_protocol.qnt
  [PASS] invariants run: engine_loop
  [PASS] invariants run: stop_hook
  [PASS] invariants run: journal_protocol
  [PASS] witness w_block / w_cap_release_interactive / w_parked_auto_block / w_seed2 / w_sig_reset
  [PASS] witness w_conflict / w_crash_then_catchup / w_interleaved_writers / w_idempotent_catchup
  [PASS] quint test --match 'r_.*' (formal/orchestration/stop_hook.qnt)

[PASS] quint gate: all steps green
```

**PASS**（モデル無改変のまま緑）

### 6. `bash scripts/coverage.sh --base origin/main`

```
==> head の line coverage を計測中 (/Users/j5ik2o/orca/workspaces/amadeus-ng/docs)
head line coverage: 98.42058209993047%
[PASS] absolute gate: head (98.42058209993047%) >= threshold (90.0%)
==> base の line coverage を計測中 (...)
base (origin/main) line coverage: 98.39753160147308%
[PASS] relative gate: head (98.42058209993047%) >= base (98.39753160147308%) - tolerance (0.01)
```

**両方 PASS**。新設エラー枝（manifest 不一致 / aid parse 失敗 / `ContractViolation` /
負の rowid）にはコミット前に到達テストを書いたので、後追いの回復コミットは発生していない。
カバレッジは base より **+0.023pp** 上がった。

### 7. `unwrap` / `expect` と `#[allow]`

- プロダクトコードに `unwrap()` / `expect(` は**なし**（`modules/core/*/src` を機械的に
  走査して 0 件。加えて workspace lints の `unwrap_used` / `expect_used` deny が受入基準 2 で
  機械強制されている）
- 本 Bolt で追加した `#[allow]` は 3 箇所、いずれも `reason = "..."` 付き:
  - `workflow_execution_event.rs:601` / `:617`（`clippy::disallowed_methods` — serde 境界の
    往復確認・payload 綴りの検査）
  - `journal_reader_impl.rs:1215`（同 — 本家シリアライザと同形式のフィクスチャ生成）
- テストファイル先頭の `#![allow(clippy::unwrap_used, ...)]` は既存様式のまま（直上に理由
  コメントあり）。新規追加はなし

## 4. 判断に迷って独自解釈した点

### (a) 裁定 4 初稿の矛盾 — 止めて裁定を仰いだ【解決済み】

初稿の「更新は `persist_event(envelope, snapshot.version())`」は、`store` の引数に version が
無いため **store 内で `get_latest_snapshot_by_id` を読み直す**形にしかならず、常に最新版を
提示することになって楽観ロックが成立しない。SQLite では journal の `(aid, seq_nr)` 一意制約が
偶然救うが、**memory バックエンドには一意制約が無く黙って二重書込になる**
（`event_store_for_memory.rs` の `update_event_and_snapshot` は `entry.events.push` のみ）。

さらに Quint モデル `journal_protocol.qnt` の実測で 2 点を確認した:

1. `var loadedVersion: int -> int` があり `store_ok` のガードが
   `loadedVersion.get(w) == snapVersion` — モデルは**書き手が版を握る**前提で競合を定義して
   いる。裁定 9（モデル無改変）と両立するのは持ち回り形だけである
2. 不変条件 `snapshot_tracks_journal`（snapSeq == journalLen）がある。v3 の `persist_event`
   は snapshot 行の `seq_nr` 列を進めない（SQLite は `UPDATE snapshot SET version = ?,
   last_updated_at = ? WHERE ...` のみ、memory も `latest.seq_nr()` 据え置き）ため、
   更新に `persist_event` を使うとこの不変条件が破れる

委任者へ 2 通報告し、裁定 4 が改訂された（選択肢 A + 更新も `persist_event_and_snapshot`）。
**改訂版どおりに実装しており、独自解釈は残っていない。**

### (b) `expected_version` の newtype 化は見送り（委任者確定）

改訂ブリーフが「newtype（`StoreVersion` 等）を推奨」と書いたが、同日追記で「usize のまま
（不透明トークンの旨を doc 明記。newtype 化は U5/U6 実装時の境界強化候補として報告書に記録）」
と確定した。**U5/U6 の申し送り**として記録する: `seq_nr` と `expected_version` はどちらも
`usize` で、引数順を取り違えても型では止まらない。ユースケース本体を書くときに
`StoreVersion` newtype（`RepositoryError::Conflict` の材料も含む）を検討すること。

### (c) `Corrupt` の原因分類 — aid parse 失敗を `InvariantViolation` にした

裁定 6 は「`IntentId` へ parse、失敗は Corrupt」とだけ書き、`CorruptCause` を指定していない。
`UndecodablePayload` と `InvariantViolation` のどちらかだが、**`InvariantViolation` を選んだ**。
理由は 2 つ:

- `UndecodablePayload` の doc は「行の**ペイロード**をドメイン型へ復号できない」であり、
  aid は payload ではなく列である。裁定 5 が manifest 検査に `UndecodablePayload` を割り当てて
  いるので、この語を payload の話に保つほうが分類が濁らない
- 同じファイルの既存変換（`to_u64` の負値、`usize::try_from(row.seq_nr)` の負値）が
  すべて `InvariantViolation` を返している。「列の値をドメインへ運べない」という同類である

`CorruptCause` の doc は変更していない。

### (d) `find_by_id` のリプレイ開始位置は集約自身の `seq_nr` から採った

本家移行ガイド §3 は `snapshot.seq_nr()`（スナップショット行の列）を開始点にする。我々は
**集約自身の `seq_nr`**（`workflow_execution_repository_impl.rs:287`）を使った。裁定 3 が
「seq_nr はドメイン責務」と定めており、列はストア側の写しだからである。両者は書込経路で
必ず一致し、万一食い違えば `apply_event` が `SequenceGap` で止める（防御が 1 枚増える）。
既存テスト `a_replay_does_not_move_the_version_the_store_assigned` の巻き戻しヘルパは、
v3 が seq_nr 列を実値で持つようになったため `payload` と `seq_nr` を一緒に戻すよう更新した
（`workflow_execution_repository_impl_test.rs` の `rewind_snapshot_to_genesis`）。

### (e) 構成不能になったテスト 2 本を差し替えた

`check_preconditions`（イベントと集約が同じ集約を指すか / 通番が一致するか）は、イベントが
その 2 つを持たなくなったことで**検査対象が構成不能**になった（B6 で `seq_nr = 0` に対して
行ったのと同じ、実行時検査 → 型強制の置換）。契約テスト
`a_sequence_that_disagrees_with_the_aggregate_is_refused` /
`mismatched_identity_is_refused` は削除し、代わりに:

- `the_envelope_takes_every_transport_material_from_the_applied_aggregate`（封筒の材料が
  すべて集約由来であることを固定）
- `a_second_aggregate_gets_its_own_envelope_identity`
- `a_write_from_the_rehydrated_version_succeeds`（`Conflict` の裏面）
- `a_genesis_with_a_non_zero_version_is_a_contract_violation`

を置いた。契約テストの本数は 10 → 10（memory / sqlite 各 10 = 20 テスト）で変わらない。

### (f) 仕様書との語彙ドリフト（未解消 — 委任者判断が要る）

`docs/specs/**` は変更禁止のため触っていないが、次の 2 点でコードと仕様の語がずれた:

1. **C6 の memento が 17 属性 → 16 属性**（`version` 列の除去）。コード側 doc は 16 に
   直したが、`docs/specs` の C6 記述は 17 のままのはずである
2. **C5 の `schema_version` 予約フィールド**が消え、後継が journal の `manifest` 列に
   移った。C5 の宣言は payload 内メタを前提にしている可能性がある

いずれも裁定 2 / 4 の必然的な帰結であり、実装側で読み替えた箇所はない。仕様同期は
**別途 doc 同期タスク**として扱ってほしい。

### (g) `docs/specs` の BR 番号参照はそのまま残した

doc コメントに残る `BR1.3` / `BR5.3` / `C5` / `C6` 等の参照は、意味が変わっていない限り
書き換えていない。version 経路のように意味が変わった箇所だけ、doc の説明文を実態へ合わせた
（例: `workflow_execution_repository_impl.rs:19-35` の「楽観 version は不透明なトークンである
(BR5.3)」節）。

## 5. 委任者への確認事項

- **仕様同期**（§4-(f)）: `docs/specs` の C5 / C6 記述をどの Bolt で追随させるか
- **newtype 化**（§4-(b)）: `StoreVersion` の導入時期（U5/U6 の境界強化候補として記録済み）
- 本家 `event-store-adapter-rs` リポジトリへの接触は一切していない（issue / PR / コメントとも）
