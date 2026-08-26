# developer-report-2 — 委任 2: ポート / 値 / エラー、ワイヤ、InMemory、契約テスト（U3 / Bolt B5）

> 開発エージェント（aidlc-developer-agent）の作業報告。ブランチ `bolt/b5-u3-event-store-repository`。
> 出典: `developer-brief-2.md`、`code-generation-plan.md` §5.2 Step 3〜5、`../functional-design/{rules,entities,functional-spec}.md`
> （BR1.1〜BR1.5 / BR2.5 / BR2.7 / BR2.8）、`../nfr-design/security-design.md` §2 / §3、`../nfr-requirements/security-requirements.md`
> （NFR2.2 / NFR3.x / NFR4.3 / NFR4.5）、`../../../inception/contract-design/contract-summary.md` C3、
> `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README + 7 ルール、着手前に全読）。

## 基線（着手時、委任 1 完了後の HEAD）

- `cargo test --workspace`: **448 passed / 0 failed**
- `cargo clippy --workspace --all-targets -- -D warnings`: 緑、`cargo fmt --all --check`: 差分なし

## §A Red の失敗出力（レイヤーごとに 3 回）

### A-1. Step 3（Data model / use-case）— 値 2 型・エラー 2 型 + CorruptCause・ポート 3 本

テストと `mod.rs` の `pub use` だけを先に置いた状態での `cargo test -p core-use-case`（抜粋、計 8 件の E0432）:

```
error[E0432]: unresolved import `event_store::EventStore`
  --> modules/core/use-case/src/orchestration/mod.rs:21:9
   |
21 | pub use event_store::EventStore;
   |         ^^^^^^^^^^^^^^^^^^^^^^^ no `EventStore` in `orchestration::event_store`

error[E0432]: unresolved import `journal_reader::JournalReader`
error[E0432]: unresolved import `workflow_execution_repository::WorkflowExecutionRepository`
error[E0432]: unresolved import `global_seq_nr::GlobalSeqNr`
error[E0432]: unresolved import `projection_name::ProjectionName`
error[E0432]: unresolved imports `event_store_error::CorruptCause`, `event_store_error::EventStoreError`
error[E0432]: unresolved import `projection_name::ProjectionNameError`
error[E0432]: unresolved import `repository_error::RepositoryError`
error: could not compile `core-use-case` (lib) due to 8 previous errors
```

### A-2. Step 4（Business logic / ワイヤ）

```
error[E0432]: unresolved import `event_wire::EventPayloadWire`
  --> modules/core/interface-adapter/src/orchestration/wire/mod.rs:14:16
error[E0432]: unresolved import `state_wire::StateWire`
error[E0433]: cannot find type `EventPayloadWire` in this scope   (× 6)
error[E0433]: cannot find type `StateWire` in this scope          (× 4)
error: could not compile `core-interface-adapter` (lib test) due to 12 previous errors
```

Green 直前にもう 1 度赤を踏んでいる（実装を書いた後、**テスト側の欠陥 2 件**が露見した — 下の「判断」§B-6 参照）:

```
---- state_wire::tests::the_encoded_object_lists_the_sixteen_attributes_in_the_declared_order ----
assertion `left == right` failed   left: 25  right: 16      （キー数の数え方が入れ子オブジェクトを拾っていた）

---- state_wire::tests::an_arbitrary_state_survives_the_round_trip ----
called `Result::unwrap()` on an `Err` value:
  Corrupt { aggregate_id: "01a02785-…", seq_nr: None, cause: InvariantViolation }
                                                （生成器が 2^53 超の seq_nr / version を作り、書込側の防波堤が発火）
```

### A-3. Step 5（API / 契約テスト）

```
error[E0432]: unresolved imports `core_interface_adapter::orchestration::InMemoryEventStore`,
              `core_interface_adapter::orchestration::InMemoryWorkflowExecutionRepository`
  --> modules/core/interface-adapter/tests/workflow_execution_repository_contract.rs:13:5
error: could not compile `core-interface-adapter` (test "workflow_execution_repository_contract")
```

InMemory 実装の投入後、契約テスト 12 本のうち 1 本が赤（テスト側の前提誤り — seed 後のカーソルはゲート付きステージ）:

```
---- concurrent_rehydration_conflicts ----
panicked at tests/support/contract.rs:125: 索引 2 は非ゲート: InvalidTarget(StageIndex(2))
```

`complete_stage` → `open_gate` に直して緑（実装は変更していない）。

## §B 実装概要

### B-1. use-case 層（Step 3）— `modules/core/use-case/src/orchestration/`

| ファイル | 公開面 |
|---|---|
| `global_seq_nr.rs` | `GlobalSeqNr`（`ZERO` 定数、`new` / `value`、`From<u64>`、`Display`、`Ord`） |
| `projection_name.rs` | `ProjectionName::parse`（kebab `^[a-z][a-z0-9-]*$`、1〜64 字）/ `as_str`、`ProjectionNameError { Empty, Length, Format }` |
| `event_store_error.rs` | `EventStoreError`（`Conflict` / `Io` / `Corrupt` / `Schema` / `CheckpointRegression`）、`CorruptCause`（6 変種）。手実装 `Display` + `std::error::Error` |
| `repository_error.rs` | `RepositoryError`（`NotFound` / `Conflict` / `Io` / `Corrupt`）、`RepositoryError::from_event_store(error, &IntentId)` |
| `event_store.rs` | `trait EventStore<AID, A, E>`（4 メソッド、`async fn`、数値は u64） |
| `journal_reader.rs` | `trait JournalReader`（`events_after` / `checkpoint` / `advance_checkpoint`） |
| `workflow_execution_repository.rs` | `trait WorkflowExecutionRepository`（`find_by_id` / `store`、いずれも `&self`） |

trait はすべて AFIT（`async fn`）・`dyn` なし・`Send` / `Sync` 境界なし。`# Errors` 節つき。`mod.rs` の `pub use` に旧名は無い。
依存追加は `modules/core/use-case/Cargo.toml` の **dev-dependency `tokio` のみ**（プロダクト依存は増やしていない — NFR4.1）。

### B-2. アダプタ層のワイヤ（Step 4）— `src/orchestration/wire/`

- `wire/mod.rs`: ファサード（`pub(crate) use`）＋ **2 つのワイヤが共有する部品**（`SCHEMA_VERSION = 1`、正準 JSON 符号化 `to_canonical_json`、
  唯一の読取口 `parse_json`、未知フィールドを拒否する `WireObject` リーダ、固定トークンの写像、`StageEntryWire`）。
  共有部品を `mod.rs` に置いたのは、ブリーフの所有ファイルが 3 本に限られており、かつ**同じ値がイベントと状態で別綴りになる**のを防ぐには
  写像を 1 か所に持つ必要があるため。
- `wire/event_wire.rs`: `EventPayloadWire`（serde の internally tagged enum、12 変種）。`encode` / `decode` / `event_type` / `SCHEMA_VERSION`。
  JSON は `{"type":"<変種名>", …材料}`、封筒 4 列は含めない。
- `wire/state_wire.rs`: `StateWire`（16 属性、宣言順がそのままキー順）。`encode` / `decode` / `SCHEMA_VERSION`。

固定トークンは upstream 綴り（`EXECUTE` / `SKIP`、PhaseId 5 語、CheckboxState の 6 マーク、`autonomous` / `gated`）。
復号は parse-don't-validate（`IntentId` / `StageSlug` / `PlanAction` / `PhaseId` / `WorkflowDefinitionId` / `DefinitionRevision` は
ドメインの `parse` をそのまま通す）。公開面は `pub(crate)` まで。

### B-3. InMemory（Step 5）— `src/orchestration/memory/`

- `in_memory_event_store.rs`: `InMemoryEventStore` — `EventStore<IntentId, WorkflowExecution, WorkflowExecutionEvent>` と
  `JournalReader` の実装。内部は C6 と同じ 3 表（`journal` / `snapshot` / `checkpoint`）＋ global 通番の採番器で、
  **ペイロードは SQLite 実装と同じワイヤ（正準 JSON）で保持**する。
- `workflow_execution_repository.rs`: `InMemoryWorkflowExecutionRepository { store: RefCell<InMemoryEventStore> }` —
  `find_by_id` = スナップショット → `from_state` → `with_version(行の version)` → 差分 replay → `with_version(最後の seq_nr)`、
  `store` = 前提検査（BR1.3 の 4 条件）→ `persist_event_and_snapshot`。
- `orchestration/mod.rs` のファサードに `InMemoryEventStore` / `InMemoryWorkflowExecutionRepository` を追加。

### B-4. 契約テスト（Step 5）— `tests/`

- `tests/support/mod.rs`: `StoreFixture` trait（`open()` = 同じストアを指す新しい Repository、`reader()` = 同じストアの `JournalReader`）と
  集約の組み立てヘルパ。
- `tests/support/contract.rs`: 契約 12 本のジェネリック関数（`round_trip` / `not_found` / `genesis_expects_version_zero` /
  `genesis_twice_conflicts` / `concurrent_rehydration_conflicts` / `sequence_gap_is_refused` / `mismatched_identity_is_refused` /
  `journal_reads_every_event_in_global_order` / `journal_reads_only_the_difference` / `unregistered_checkpoint_is_zero` /
  `checkpoint_advances_and_repeats_are_noops` / `checkpoint_regression_is_refused`）。
- `tests/workflow_execution_repository_contract.rs`: in-memory 実装を上の関数群へ流し込むだけ（`macro_rules!` で 12 本を宣言）。
  **委任 3 は同じ関数群に SQLite 実装の fixture を差すだけで済む。**

## §C 判断（設計に無い/裁量のある選択）

1. **`EventStoreError` → `RepositoryError` の写像は `From` ではなく `RepositoryError::from_event_store(error, &IntentId)`**。
   理由: `EventStoreError::Corrupt` の `aggregate_id` は生 `String`、`RepositoryError::Corrupt` は `IntentId` で、
   全域の `From` が書けない（`Schema` も集約識別子を持たない）。呼出文脈の識別子を第 2 引数で受け、
   行が名乗る識別子が `IntentId` として妥当ならそれを、妥当でなければ文脈の識別子を使う。
   `Schema` → `Corrupt(SchemaVersion)`、`CheckpointRegression` → `Corrupt(InvariantViolation)`（**投影名の材料は落ちる** —
   `RepositoryError::Corrupt` に置き場が無く、entities.md の 4 変種を増やさない方を採った。Repository の 2 メソッドからは到達しない経路）。
2. **InMemory はワイヤ経由で行を持つ**（設計の「`BTreeMap<…, WorkflowExecutionEvent>` 相当」からの具体化）。
   ドメイン値をそのまま持つと、共有した契約テストがワイヤ経路を 1 バイトも通らず、「in-memory では通るが SQLite では落ちる」経路を
   見逃す。行の形も規則も SQLite と同形にした。副次的に `wire` が実コードから使われるので `dead_code` も出ない。
3. **`InMemoryEventStore` は共有ハンドル**（内部 `Rc<RefCell<Tables>>`、`Clone` は同じ 3 表を指す）。
   Repository のフィールドは設計どおり `RefCell<InMemoryEventStore>` のまま。契約テストの「新しいインスタンスで `find_by_id`」を
   実装によらず書くために `StoreFixture::open()` がハンドルを複製する（SQLite なら同じファイルを開き直すことに対応）。
4. **借用は `await` をまたがない**（設計 functional-spec §2 の約束）。`clippy::await_holding_refcell_ref` が deny 相当で発火したため、
   Repository は `self.store.borrow().clone()` でハンドルを複製してから `await` する。**委任 3 への申し送り**は §E-1。
5. **`persist_event(event, version)` は「ジャーナル追記のみ」＋ `version` を楽観前提として検査**する。
   BR2.3 は「(1) のみ」と書くが、`version` を無視すると引数が意味を失う（本家 event-store-adapter-rs では
   スナップショットを更新しない追記の楽観前提）。スナップショット行は一切書かない点は BR2.3 どおり。
6. **JSON の整数値域に防波堤を置いた**。canon-json は 2^53 超の整数を JS と同じく f64 経由で書く（バイト一致のため）ので、
   そのまま書くと `seq_nr` / `version` の往復が静かに壊れる。`StateWire::encode` は 2^53 超を
   `Corrupt(InvariantViolation)` で拒否する（丸めて書かない）。境界値 2^53 ちょうどの往復と、超過の拒否を各 1 本テストで固定。
   PBT の生成器も同じ値域に合わせた（実運用の `seq_nr` は永続化済みイベント数なので上限には遠く届かない）。
7. **`phase_boundary` は入れ子オブジェクト**（`{"from_phase": …, "to_phase": …}`）。functional-spec §4.1 の表は
   `string | null` だが、ドメインの `PhaseBoundary` は 2 つの `PhaseId` の組で、1 本の文字列に畳むには区切り記号の発明が要る。
   両半分をそれぞれ `PhaseId::parse` に通す形を採った（§E-2 に設計質問として起票）。
8. **正準 JSON のプロファイルは `ContractCompact`**（空白なし・宣言順）。`HashCanonical`（再帰ソート）は canon-json の
   rustdoc がハッシュ入力用と明記しているため使わない。同じ入力が常に同じバイト列になることは PBT で固定。
9. **`Status` / `AutonomyMode` / `JumpDirection` はアダプタ側に厳密な写像を置いた**（ドメインに `parse` / `as_str` が無いため）。
   特に `AutonomyMode::read_state` は状態ファイル読取用の **fail-closed リーダ**で未知値を gated に畳むので、
   破損検出の境界では使わない（使うと改竄が検出できない — NFR3.2 / NFR4.4）。この差は専用テスト 1 本で固定した。
10. **破損（`MissingSnapshot` / `UndecodablePayload` / `SchemaVersion` / `UnknownEventType`）は契約テストから外し、実装固有テストに置いた**
    （ブリーフの選択肢どおり）。公開の破壊フック（`corrupt_for_test` 等）は作らず、`in_memory_event_store.rs` の
    インライン `#[cfg(test)]` から内部の行を直接壊す（同一モジュールなので private フィールドに届く）。委任 3 は同じ 4 本を
    直接 SQL で書けばよい。なお「ジャーナル行はあるがスナップショットが無い」状態は `persist_event` で自然に作れる。
11. **`proptest` を adapter の dev-dependency に追加**した（ブリーフは「dev-dependency は tokio のみ」と書くが、Step 4 が PBT を要求するため）。
    workspace 依存に既存の版があり、プロダクト依存は増えていない。workspace `Cargo.toml` への追加は `tokio` の 1 行のみ。
12. **`#[allow(async_fn_in_trait, reason = …)]` を 3 つの trait に付けた**。付けないと rustc の
    `async_fn_in_trait`（warn-by-default）が「auto trait 境界を指定できない」と警告し、CI の `-D warnings` で落ちる（実測で確認）。
    これは「`Send` 境界を要求しない」という設計（C3 / Q3 = A）そのものへの注意喚起なので、理由つきで抑止した。
    ブリーフが禁じた `unused_async` の握りつぶしではない（`unused_async` は 1 件も発火していない）。
13. **前提検査の失敗が持つ `aggregate_id` はイベント側**（`event.intent_id()`）にした。書けなかった行の識別子だから。

## §D 検査結果

| 検査 | 結果 |
|---|---|
| `cargo test -p core-use-case` | **48 passed / 0 failed** |
| `cargo test -p core-interface-adapter` | lib **70** / append_only 1 / golden 9 / definition-repo 27 / **契約 12** = 119 passed / 0 failed |
| `cargo test --workspace` | **549 passed / 0 failed**（基線 448 → +101） |
| `cargo clippy --workspace --all-targets -- -D warnings` | 緑 |
| `cargo fmt --all --check` | 差分なし |
| `cargo lint`（`tools/lint`） | 違反なし |
| PBT の決定性 | `PROPTEST_RNG_SEED=20260823` で 2 回連続同結果。`proptest-regressions/` は残していない（生成された 1 件は、上の A-2 のテスト側欠陥に由来するもので、修正後は再生成されない） |
| プロダクトコードの禁止構文 | `panic!` / `unwrap` / `expect` / `todo!` / 添字アクセス を新規 12 ファイルの `#[cfg(test)]` 前で走査 — **0 件** |

`git add` / `git commit` は行っていない（コンダクタの担当）。`.claude/` のツールは実行していない。
所有ファイル外（`modules/core/domain/**`、`sqlite_event_store.rs` 等、`docs/**`、`formal/**`、計画・検査手順・質問票）は触っていない。

## §E 設計質問

1. **【委任 3 に影響・要裁定】`RefCell<SqliteEventStore>` は `clippy::await_holding_refcell_ref` と両立しない。**
   `EventStore` のメソッドが `async fn` である以上、`&self` の Repository から `&mut` のストアを呼ぶには
   「借用を持ったまま `await`」になる。in-memory は共有ハンドルなので借用を複製で閉じられたが、
   `SqliteEventStore` は `Connection` を所有し `Clone` できないため同じ手が使えない。取り得る道は
   (a) `SqliteEventStore` に同期の内部メソッドを持たせ、`await` を挟まない借用区間で呼ぶ（ポートを経由しなくなる）、
   (b) `Connection` 自体を `RefCell` に入れて `EventStore` の実装を `&self` 化する（trait の `&mut self` 署名は C3 の定義）、
   (c) 当該箇所に理由つき `#[allow]` を置く、のいずれか。設計（entities.md / functional-spec §2）の
   「借用は await をまたがない」は (a) か (b) を前提にしているように読めるので、どちらを採るかを裁定いただきたい。
2. **`phase_boundary` のワイヤ形**（§C-7）。表の `string | null` を入れ子オブジェクトに具体化した。
   ワイヤは新設のストア内部形式で upstream 互換面ではないため往復忠実さを優先したが、表の記述を改訂するか確認いただきたい。
3. **`CheckpointRegression` → `Corrupt(InvariantViolation)` で投影名が落ちる**（§C-1）。
   Repository の 2 メソッドからは到達しない経路なので実害は無いと判断したが、材料を残すなら
   `RepositoryError` に変種か材料を足す必要がある（entities.md の改訂）。
4. **スナップショット payload の `version` は書込前の値**になる（集約は遷移で version を動かさないため）。
   列の `version` が正で、`find_by_id` が `with_version(列の値)` で載せ替えるので再構成は正しい。
   payload 側も新 version に揃えるべきなら `aggregate.clone().with_version(new).state()` に変えるが、
   BR2.3 の「payload = ?（適用後の集約）」の逐語からは現状が素直と判断した。

## §F 未了（本委任の範囲外・後続へ）

- SQLite ストア本体・`StorePath`・`WorkflowExecutionRepositoryImpl`・`schema.rs`（委任 3）。
  委任 3 は `tests/support/contract.rs` の 12 関数に fixture を差すだけで契約テストを共有できる。
- Quint `journal_protocol.qnt` と ITF 準拠（委任 4）。ITF の再生先は本委任の `InMemoryEventStore` を使える。
- 仕様・正本の同期（委任 5）、`indexing_slicing` / `panic` の lint 昇格（委任 6）。
- C3 の `usize` → `u64` 改訂の申し送り（所有者 U5 / U6）は本委任では触っていない。
