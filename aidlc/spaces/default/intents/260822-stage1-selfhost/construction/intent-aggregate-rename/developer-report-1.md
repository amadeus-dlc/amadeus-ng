# B12 開発者報告 1 — `Intent` 構造体 + `IntentExecution` 集約への分割

対象ブリーフ: [`brief-1.md`](brief-1.md)（改訂 10 まで反映）
ブランチ: `bolt/b12-intent-aggregate-rename`（origin/main 基準、**push なし**）
検証の `CARGO_TARGET_DIR`: `target-delegate`（`cargo lint` のみ `target-delegate-lint`）

---

## 0. 結論（先に）

**受入基準はすべて緑**である（改訂 10 反映後に全ゲートを再実行した実測）。
`cargo test --workspace` は **838 passed / 0 failed**（origin/main は 774 なので **+64**）。
`tests/` の差分は 0 で、投影ゴールデン 19 本は無改変のまま全緑である。

本 Bolt は途中で改訂 2〜10 の裁定を受けて到達点が変わった。最終形は**単純改名ではなく分割**で
ある:

| | 役割 |
|---|---|
| **`Intent`**（新設・静的な集約） | 静的な intent — 識別子・定義のピン・依頼・解決済み計画・走査結果。Always Valid、変異メソッドなし。**集約なので genesis `create` は `(Intent, IntentEvent)` の対を返す**（改訂 8） |
| **`IntentExecution`**（集約） | 1 回の実行 — `id: IntentExecutionId` と `intent_id: IntentId`、および実行時状態だけ |
| **`IntentExecutionId`**（新設） | 実行自身の識別子（1 intent : n 実行なので intent の識別子を借りられない） |

計画が要る判断は `&Intent` を**引数で受け取り**、入口で `intent.id() == self.intent_id` と
計画長の一致を照合する（`coding-rules/aggregate-references.md`）。

固定フィクスチャに旧名や `"intent_id"` のバイトが埋まっている箇所は**発見されなかった**（§3）。
したがって「止めて報告」の条件には該当しなかった。

判断待ちだった `EVENT_MANIFEST` の値も、2026-08-30 の裁定を受けて
`"intent-execution-event/1"` へ揃えた（§5 (a)）。これで本 Bolt に旧語彙の化石は残っていない。

改訂 8 で **`Intent` も集約**と確定し（「集約ではない不変構造体」は撤回）、`IntentEvent::Created`
の新設と対返しファクトリ `Intent::create` を実装した（§5 (k)）。これに伴い §5 (i) の
「`Intent::new` は変更不要」裁定は**上書きされた**。

改訂 10 で **`CommitVerdictUseCase` をリポジトリ保持形へ戻した**（§5 (m)）—
`execute` の引数は集約 ID と値オブジェクトだけになり、計画はユースケースが自分で引く。

改訂 9 で **ドメインから永続化知識を全撤去**した（§5 (l)）。`core-command-domain` に serde と
event-store-adapter-rs が**無い**ことが規則の機械強制であり、行のバイトを決めるのは書く側
（command interface-adapter）と読む側（RMU）の永続化 DTO になった。**ワイヤ形式のバイトは
1 文字も変わっていない**（12 変種 + スナップショットを逐語で固定、両側で同一リテラル）。

---

## 1. 到達点（改訂 10 まで反映・実測）

| 旧 | 新 |
|---|---|
| `WorkflowExecution`（集約） | `IntentExecution`（実行時状態だけに縮小） |
| `WorkflowDefinition::new` | `from_artifacts`（再構成）+ 新設 `define`（genesis・対を返す。改訂 7） |
| （無し） | **`WorkflowDefinitionEvent::Defined`**（新設・改訂 7） |
| （無し） | **`Intent`**（新設・静的側の集約） |
| `Intent::new` | `from_material`（再構成・serde 復号もここを通る）+ 新設 `create`（genesis・対を返す。改訂 8） |
| （無し） | **`IntentEvent::Created`**（新設・改訂 8） |
| （無し） | **`IntentExecutionId`**（新設・実行の識別子。`AggregateId` はこちらが実装） |
| `WorkflowExecutionEvent` | `IntentExecutionEvent`（変種名は不変） |
| `WorkflowExecutionState` | `IntentExecutionSnapshot`（クレート内私有、16 属性 → **12 属性**） |
| `WorkflowExecutionStateBuilder` | `IntentExecutionSnapshotBuilder`（クレート内私有 + `#[cfg(test)]`） |
| `WorkflowExecutionRepository` | `IntentExecutionRepository`（`find_by_id(&IntentExecutionId)`） |
| `WorkflowExecutionRepositoryImpl` | `IntentExecutionRepositoryImpl` |
| `InMemoryWorkflowExecutionRepository` | `InMemoryIntentExecutionRepository` |
| `RehydratedWorkflowExecution` | `RehydratedIntentExecution` |
| `AGGREGATE_TYPE_NAME = "WorkflowExecution"` | `"IntentExecution"`（`IntentExecutionId` が持つ） |
| `StartError` | **`IntentError` へ統合して削除**（別名は残さない） |
| `StateError` | `SnapshotError`（ファイルも `snapshot_error.rs` へ。オーナー裁定 2026-08-29） |
| `WorkflowExecution::state()` / `from_state()` | `IntentExecution::snapshot()` / `from_snapshot()`（同裁定） |
| `RepositoryError::NotFound { intent_id }` | `{ execution_id: IntentExecutionId }` |
| `JournalEntry::intent_id()`（RMU 読取行） | `execution_id()`（ジャーナルの集約キーは実行識別子） |

### 集約の縮小（改訂 3 の中心）

保持する: `id` / `intent_id` / `overlay` / `checkbox` / `approved` / `revision_count` /
`cursor` / `status` / `parked_at` / `autonomy` / `seq_nr` / `last_updated_at`。

**保持しない**（Intent 側へ）: `definition_id` / `definition_revision` / `stages`。
base の `plan_action` と `conditional` はもともと `stages` から導いていたので同時に消えた。
実効プランは従来どおり `overlay` が持つ（recompose が上書きする列であり、実行時状態である）。

### genesis の新形

```rust
IntentExecution::start(id: IntentExecutionId, intent: Intent, occurred_at) -> (IntentExecution, IntentExecutionEvent)
```

`Result` が消えた。受け取る `Intent` が Always Valid なので、ここに失敗経路が無いためである。
計画の解決（スコープ検査・表示属性の単一行検査）は補助コンストラクタ `Intent::resolve` が担う。

改訂 8 以降、その `Intent` 自身も集約なので genesis は対を返す。`start` に渡すのは**対の左**で
ある:

```rust
Intent::create(id, definition_id, definition_revision, start_request, stages, scan)
    -> Result<(Intent, IntentEvent), IntentError>
Intent::resolve(id, &definition, start_request, scan)
    -> Result<(Intent, IntentEvent), IntentError>   // 補助。create へ委譲
Intent::from_material(..) -> Result<Intent, IntentError>  // 再構成。イベントを作らない
```

### ファイル改名（すべて `git mv`）

| 旧 | 新 |
|---|---|
| `domain/src/orchestration/workflow_execution.rs` | `intent_execution.rs` |
| `domain/src/orchestration/workflow_execution_event.rs` | `intent_execution_event.rs` |
| `domain/src/orchestration/workflow_execution_state.rs` | `intent_execution_snapshot.rs` |
| `domain/proptest-regressions/orchestration/workflow_execution.txt` | `intent_execution.txt` |
| `use-case/src/orchestration/workflow_execution_repository.rs` | `intent_execution_repository.rs` |
| `use-case/src/orchestration/rehydrated_workflow_execution.rs` | `rehydrated_intent_execution.rs` |
| `interface-adapter/src/orchestration/workflow_execution_repository_impl.rs` | `intent_execution_repository_impl.rs` |
| `interface-adapter/tests/workflow_execution_repository_contract.rs` | `intent_execution_repository_contract.rs` |
| `interface-adapter/tests/workflow_execution_repository_impl_test.rs` | `intent_execution_repository_impl_test.rs` |

新設ファイル: `domain/src/orchestration/intent.rs` / `intent_event.rs`（改訂 8） /
`intent_execution_id.rs` / `uuid_v7.rs`、`domain/src/workflow_definition/workflow_definition_event.rs`
（改訂 7）。
削除ファイル: `domain/src/orchestration/start_error.rs`。

---

## 2. 外形不変の証明（受入基準）

| 観点 | 実測 |
|---|---|
| `tests/golden/**` | `git status --short -- tests/` が **0 件**（読むだけ・1 バイトも変えていない） |
| 投影ゴールデン | `projection_golden_test.rs` の 19 本が**無改変で全緑**（フィクスチャの `Started` 組み立てだけ新形に追随） |
| 監査語彙 | `WORKFLOW_*` に触れていない（RMU の投影核は無改変） |
| `EVENT_MANIFEST` | `"intent-execution-event/1"` へ改めた（§5 (a)）。ゴールデンにこの文字列は **0 件**（`grep -rc "workflow-execution-event" tests/` が全ファイル 0）で、外形契約ではない |

RMU 側を実測したところ、投影核（`projection.rs`）は `JournalEntry` の集約識別子を**使っていない**
（使用は journal reader とテストだけ）。ジャーナルの集約キーが実行識別子に変わっても、
投影出力のバイトには影響しない。

---

## 3. 固定フィクスチャの確認（「止めて報告」条件）

```
grep -rn "WorkflowExecution|\"intent_id\"" tests/ modules/core/command/domain/tests/ \
    modules/core/command/interface-adapter/tests/   →  0 件
```

ITF トレース（`formal/**` の出力）・ゴールデン・逐語アサートのいずれにも旧名は埋まっていな
かった。したがって独断でバイトを書き換えた箇所は無い。

---

## 4. 受入基準の実行結果（すべて実測）

| # | 基準 | 結果 |
|---|---|---|
| 1 | `cargo fmt --all --check` | **緑**（exit 0） |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | **緑**（exit 0） |
| 3 | `cargo lint` | **緑**（exit 0） |
| 4 | `cargo test --workspace`（退行 0） | **緑**。**838 passed / 0 failed**（origin/main 774 → **+64**） |
| 5 | `scripts/quint-gate.sh` | **緑**（exit 0） |
| 6 | `scripts/coverage.sh --base origin/main` | **緑**。head **98.61855%**。絶対 `[PASS] >= 90.0%`、相対 `[PASS] head >= base (98.52600%) - 0.01` |
| 7 | プロダクトコードに `unwrap` / `expect` 0 件 | **緑**（clippy の `unwrap_used` / `expect_used` deny で機械強制） |
| 8 | 外形不変 | **緑**（§2） |
| 9 | `grep -rn "WorkflowExecution" modules/ --include='*.rs'` | **緑**。**0 件** |
| 10 | `git log --follow` でファイル履歴が追える | **緑**（改名はすべて `git mv`） |
| 11 | `Intent` が変異メソッドを持たない | **緑**。`grep -c "&mut self" intent.rs` = **0**（改訂 8 で集約と確定したが、変異は現状も無い） |
| 12 | `IntentExecutionId` に `IntentId` と同等の形式検査テスト | **緑**（15 本） |
| 13 | genesis 新形のテスト | **緑**（`start_records_the_definition_identity_and_the_resolved_plan` ほか） |
| 14 | `Started` payload に intent が載るラウンドトリップ | **緑**（`the_started_payload_round_trips_the_intent_through_serde`） |
| 15 | 集約状態に Intent 由来の静的フィールドが残っていない | **緑**（`the_snapshot_carries_no_static_material_from_the_intent` が写しの JSON を逐語で検査） |
| 16 | `&Intent` ガード（id 不一致・長さ不一致）のテスト | **緑**（`a_command_refuses_an_intent_that_belongs_to_another_intent` ほか 5 本） |
| 17 | 全 `&mut self` コマンドがイベントを返す（`coding-rules/aggregate-commands.md`） | **緑**。集約の `pub fn` かつ `&mut self` は **12 本**あり、うち 11 本（`complete_stage` / `open_gate` / `approve_gate` / `reject_gate` / `revise_stage` / `skip_stage` / `jump` / `park` / `unpark` / `recompose` / `switch_autonomy`）はすべて `Result<IntentExecutionEvent, CommandError>` を返す。残る 1 本は規則が明示的に除外する fold の `apply_event`（`Result<(), ApplyError>`）。イベントを返さない遷移メソッドは **0 本** |
| 18 | genesis が (集約, 誕生イベント) の対を返す / 再構成経路がイベントを生成しない | **緑**。`IntentExecution::start` は `(IntentExecution, IntentExecutionEvent)`、`WorkflowDefinition::define` は `(WorkflowDefinition, WorkflowDefinitionEvent)`、`Intent::create` / `Intent::resolve` は `Result<(Intent, IntentEvent), IntentError>`（改訂 8）。再構成の `IntentExecution::from_snapshot` / `apply_event` / `WorkflowDefinition::from_artifacts` / `Intent::from_material` はいずれもイベント型を戻り値に持たない（型で保証） |
| 19 | `EVENT_MANIFEST` の綴りが集約名に揃っている（2026-08-30 裁定） | **緑**。値は `"intent-execution-event/1"`。旧綴りのコード出現は **0 件**（`grep -rn "workflow-execution-event" modules/ tools/ --include='*.rs'` は doc 注記の 1 行のみ — 改名の経緯を記録した意図的な残置）。ゴールデン側は改名前から **0 件** |
| 20 | ドメインが永続化知識を持たない（改訂 9） | **緑**。`grep -rn "Serialize\|Deserialize\|event_store_adapter" modules/core/command/domain/src` は **0 件**（残るのは docs.rs の URL 1 行のみ）。`Cargo.toml` の `[dependencies]` は `chrono` だけで、serde も event-store-adapter-rs も無い — **違反はビルドで落ちる**（規則の機械強制） |
| 21 | ワイヤ形式のバイト不変（改訂 9） | **緑**。12 変種 + スナップショットの JSON を**改訂 9 直前に実測した出力そのもの**で逐語固定。書く側と読む側が**同一のリテラル**を独立に持つので、片側の綴りが動けばどちらかが落ちる。横断の証明は `journal_protocol_conformance` / `crash_reconstruction_test` / 投影ゴールデン 19 本 / 移設後の `upstream_event_store_conformance` が担う |
| 22 | 互換残置ゼロ | **緑**。`#[deprecated]` **0 件**、`pub use ... as` **0 件**、旧名 `Intent::new` / `from_state()` / `WorkflowExecution` / `IntentMaterial` の grep いずれも **0 件**（doc コメントを含む実測） |
| 23 | `execute` の引数に集約が無い（ID + VO のみ・改訂 10） | **緑**。`execute(&mut self, execution_id: &IntentExecutionId, stage: Option<&StageSlug>, transition: ReportedTransition, occurred_at: DateTime<Utc>)` — 集約 ID 1 つと値オブジェクト 3 つだけで、`&Intent` は消えた（`coding-rules/use-case-rules.md` §2b）。計画はユースケースが保持する `IntentRepository` から内部で引く |

### テストの増減の内訳（実測）

テスト関数は **755 → 813**（+58。集計は `git grep -c -E "^\s*#\[([a-z_:]+::)?test\]|^\s*#\[tokio::test" <ref> -- modules` の総和で、
origin/main と HEAD を同じ式で測った）。分割フェーズで 14 本が消え、以後の各改訂で増えた。消えた 14 本の行き先:

- 3 本（`an_empty_stage_list_is_refused` ほか）→ `Intent::new` の不変条件テストへ移設（intent.rs）
- 5 本（旧 memento モジュール）→ 新しいスナップショットテスト 4 本へ置換
- 4 本（`start_error.rs`）→ `IntentError` のテストへ移設（Display は 5 変種すべてを逐語で固定）
- 1 本（`from_state_rejects_a_broken_invariant`）→ `..._runtime_invariant` へ改名
- 1 本（`workflow_execution_conforms_to_every_committed_engine_loop_trace`）→ `intent_conforms_...` へ改名

増えた 52 本の内訳は、分割フェーズの 38 本に加えて、改訂 7（`WorkflowDefinition` の規則適合）
が 5 本、改訂 8（`Intent` の規則適合）が 9 本である。改訂 8 の 9 本は
`intent_event.rs` 3 本（材料・serde 往復・値等価）、`intent.rs` 5 本（対を返す genesis・
`resolve` も対・再構成が無イベント・両経路で同一の不変条件・復号が再構成検査を通る）、
`intent_execution.rs` 1 本（`start` は `create` の対の左を受け取る）である。

**カバレッジが失われた箇所は無い**（相対ゲートも base を上回っている）。

---

## 5. 判断が要った点

### (a) `EVENT_MANIFEST` の値も `intent-execution-event/1` へ揃えた（裁定済み）

当初は据え置いて判断を仰いだ。据え置きの根拠は「doc 自身が『変えると既存行が読めなくなる』と
逐語固定を宣言している」ことだったが、**この根拠は本リポジトリの実態に当たらない**という裁定を
受けた（2026-08-30）。裁定の根拠:

1. **ゴールデンにこの文字列は焼かれていない。** `grep -rc "workflow-execution-event" tests/` は
   全ファイル 0 件（実測）。外形契約ではない。
2. **配布済みデータが存在しない。** ジャーナルはクローンごとの使い捨てランタイム（gitignore 済み）
   であり、「既存行が読めなくなる」対価がそもそも発生しない。
   `coding-rules/no-backward-compatibility.md`（未配布のため互換の対価が無い）がそのまま適用される。
3. **改名 Bolt に旧語彙の化石を 1 つ残すほうが、将来の混乱コストが高い。**

TDD で進めた。まず逐語固定テストの期待値だけを新綴りへ書き換えて red を確認し
（`assertion left == right failed / left: "workflow-execution-event/1" / right:
"intent-execution-event/1"`）、そのうえで定数値を変えて green にした。

直した箇所は**定数 1 + 参照 5**（旧綴りのコード出現は改名前 6 件・改名後 0 件、実測）:

| # | ファイル | 直した内容 |
|---|---|---|
| 1 | `core/command/domain/src/orchestration/event_manifest.rs:24` | 定数値そのもの |
| 2 | `core/command/domain/src/orchestration/event_manifest.rs:33` | 逐語固定テストの期待値 |
| 3 | `core/command/interface-adapter/tests/intent_execution_repository_impl_test.rs:36` | `const MANIFEST`（アダプタが書く綴りの写し） |
| 4 | `core/command/domain/tests/upstream_event_store_conformance.rs:42` | `const MANIFEST`（本家封筒に載せる綴りの写し） |
| 5 | `core/read-model-updater/src/orchestration/journal_reader_impl.rs:1335` | 異種 manifest 拒否テストの「版だけ違う」ケース（`…-event/2`） |
| 6 | `app/aidlc/tests/crash_reconstruction_test.rs:147` | 生 SQL の `INSERT INTO journal(… manifest) VALUES (…)` |

doc の注記も裁定どおり言い直した（`event_manifest.rs:19-23`）——
「変えると既存行が読めなくなる」→「綴りは集約名に合わせて `workflow-execution-event/1` から
改めた。**未配布期の改名は `no-backward-compatibility.md` による**（ジャーナルは使い捨て
ランタイムで配布済みの行が無く、旧綴りを温存する対価が発生しない）。**配布後は同じ改名が
破壊的変更になる**ので、そのときは版を上げるか移行を用意すること」。テスト側のコメントも
「逐語で固定して、意図しない揺れを落とす」へ改めた（固定の目的は残し、失効した理由づけを外した）。

受入基準 9（`WorkflowExecution` の grep 0 件）とは元より無関係である（小文字・ハイフン綴りなので
部分一致しない）。改名後も基準 9 は 0 件のまま。

### (b) 再生に要る `&Intent` の入手経路（A 案 — オーナー確定）

再構成は「最新スナップショット + 以降のイベント再生」で行うが、写しから計画が消えたため、
`apply_event` が要求する `&Intent` の出所が無くなった。ブリーフはここを決めていなかったので、
検出時点で裁定を求めつつ、**ブリーフ自身の制約から選択肢が 1 つに絞れる**ことを示して A で
着手した:

- ポートに `&Intent` を足す案（B）は、改訂 2 が `find_by_id(&IntentExecutionId)` と署名を
  確定させているので不可。
- 写しに `stages` を残す案（C）は、改訂 3 が追加した受入基準に反するので不可。
- イベントが stage を index で運ぶ案（D）は、`Started` 以外の payload 構造の変更になるので不可。

裁定は改訂 4（A+）→ 改訂 5（B）→ **改訂 6（A 確定）** と動いたが、**実装の巻き戻しは
発生していない** — 改訂 6 が確定させた形は着手時の A そのものだからである。確定形:

- ポート署名は `find_by_id(&IntentExecutionId)`。
- 再生材料の `Intent` は `IntentExecutionRepositoryImpl` が**自ストリーム先頭の `Started`**
  （seq_nr 1・genesis 専用）から内部復元する。自集約のストリームを読むのは Repository の
  本業であり責務境界の違反ではない（`gateway-taxonomy.md`「署名は自集約の ID だけを取る」）。
- 復元した `Intent` は**外へ返さない** — `RehydratedIntentExecution` は実行と版だけを持つ。
- `CommitVerdictUseCase::execute(&IntentExecutionId, intent: &Intent, ...)` — Controller が
  読んで渡す（改訂 3 の 4 / 改訂 5 の 3）。

先頭が別変種・別 manifest・0 件のジャーナルは `Corrupt` で止める（テスト
`a_journal_whose_first_row_is_not_a_genesis_is_corrupt`）。読取が 1 回増える代償はある。

### (c) `StartError` を `IntentError` へ統合して削除した

`IntentExecution::start` が失敗しなくなった（Always Valid な Intent を受け取るだけ）ため、
`StartError` という名前の持ち主が消えた。5 変種すべてを `IntentError` へ寄せ、`start_error.rs`
は削除した（`no-backward-compatibility.md` どおり別名も再エクスポートも残さない）。
**Display の文言は 1 文字も変えていない** — 利用者に見える文言を改名の巻き添えにしないためで
ある。

### (d) `Intent` は `StartRequest` を丸ごと保持する

scope / request / depth / test_strategy をバラさず、既存の値オブジェクトのまま持つ。
`Intent` の一部として直列化するため `StartRequest` に serde を導出した（4 値に不変条件は無く、
復号が検査点を迂回する余地も無い）。`depth` / `test_strategy` は改訂 2 の列挙には無いが、
`Started` の payload に既にあり U4 の `Scope Configuration` 描画材料なので静的側に含めた。

### (e) 取り違えガードは**入口 1 か所**に置いた

改訂 3 は「受け取り時にガードする」と定めるが、索引ヘルパ（`entry` / `resolve`）にも置くと
不一致が `InvalidTarget` に化けて原因を取り違える。そこで:

- コマンドは `guard_running_for(intent)` を入口に置き、`CommandError::IntentMismatch` で拒否
- `apply_event` は入口で `ApplyError::IntentMismatch`
- `next_decision` / `jump_resolve` は先頭で照合
- 公開クエリ `gated` だけは `entry` 経由で `None`（範囲外と同じ扱い）

`resolve` の内側にも一度ガードを置いたが、呼出経路が既に照合済みで**到達しない枝**になったため
外した（`&self` を使わなくなったので関連関数へ変えた）。

### (f) `from_snapshot` の検査点が弱くなった

写しに計画が載らなくなったので、復号時に検査できるのは**実行時の不変条件だけ**である
（長さ・通番・カーソル・park・active の数）。計画を要する `no_gate_bypass` は `&Intent` が渡る
コマンド・適用の側へ移した。`security-design §2`「検査点は 1 か所」の文言は、改訂 3 の分割に
より「実行時の検査点は `from_snapshot`、計画を要する検査点は `&Intent` を受け取る面」の 2 か所に
なる。**設計文書側の是正が要る**（§6）。

### (g) `JournalEntry` の識別子は実行識別子へ

ジャーナル行の集約キーは実行識別子になったので、`JournalEntry::intent_id()` は
`execution_id()` へ改めた。投影核はこの値を使っていないので、外形には影響しない（§2）。

### (h) 写しの語彙を `snapshot` へ揃えた（オーナー裁定）

`state()` は戻り型が `IntentExecutionSnapshot` である以上「表現の露出」に読めるため、ES の
第一級語彙 `snapshot()` へ改めた。対になる `from_state()` は `from_snapshot()`、失敗型
`StateError` は `SnapshotError`（ファイルも `snapshot_error.rs`）へ揃えた。後方互換の別名は
置いていない（`no-backward-compatibility.md`）。

これは §6 で裁定を仰いでいた「`StateError` を `SnapshotError` へ戻すか」への回答でもある —
`snapshot()` / `from_snapshot()` と揃うのが自然なので、B5 の改名（Snapshot → State）は本 Bolt で
巻き戻った形になる。`entities.md` の該当記述（「エラーは `StateError`（旧 SnapshotError）」）は
失効するので §6 に申し送る。

### (i) ~~`Intent::new` がイベントを返さないこと（変更不要）~~ → **改訂 8 で上書き**

**この裁定は失効した。** 2026-08-29 には「`Intent` は独立した集約ではなく、その値は `Started`
に焼き込まれる静的構成なので現行設計どおり変更不要」と確定していたが、オーナー自身のその後の
原則裁定（「`IntentRepository` は必ず `Intent` を I/O する」「集約のファクトリは (インスタンス,
イベント) の対を返す。無ければリポジトリで永続化できない」）により**上書きされた**。

**`Intent` は集約である**（2026-08-30 改訂 8）— 静的で変異が現状無いだけで、
`WorkflowDefinition` と同じ類型である。対応は (k) に記す。旧裁定の論拠だった「`Started` との
二重化」は、`Started` が**実行の**歴史、`Created` が**intent の**歴史という別の集約の別の
事実なので、二重化には当たらない。

---

### (j) `WorkflowDefinition` を集約規則へ適合させた（改訂 7）

`WorkflowDefinition` は集約と裁定済みだが、ファクトリ `new` が素の Self だけを返し、イベント
語彙も無かった — `aggregate-commands.md`（ファクトリは (集約, 誕生イベント) の対が必須）に
**現に非適合**だったので本 Bolt で直した。

- `WorkflowDefinitionEvent::Defined { id, revision }` を新設。**内容フルは焼かない** — 実ファイル
  （`stage-graph.json` / `scope-grid.json` / `scopes/*.md`）がこの集約のリードモデルであり内容の
  正本だからである。運ぶのは「どの系譜のどの内容版が確立されたか」という事実だけで、内容の
  変更は将来の差分イベント（`ScopeComposed` 等）が運ぶ。
- genesis ファクトリ `WorkflowDefinition::define(...) -> (WorkflowDefinition, WorkflowDefinitionEvent)`。
- 実ファイルからの読取は genesis ではなく**再構成**なので、旧 `new` の役割を
  `from_artifacts(...)` へ改めた（イベントを生成しない）。構造体リテラルはここ 1 か所だけで、
  `define` はここへ委譲する（`factory-naming.md`）。`WorkflowDefinitionRepositoryImpl` と
  インメモリ実装、テストの計 12 箇所を追随させた。
- **ジャーナル・永続化への接続はしていない**（ブリーフどおり）。イベントを `store` する先は
  後続 intent の課題であり、ここでは型と形だけを規則へ適合させた。

---

### (k) `Intent` を集約規則へ適合させた（改訂 8 — (i) の裁定を上書き）

`WorkflowDefinition` と同じ非適合が `Intent` にもあった。TDD で進めた（red は**コンパイル
エラー 9 件**が `Intent::create` / `Intent::from_material` / `IntentEvent` / `Created` の不在と
`resolve` の戻り型不一致をそれぞれ名指しした状態、実測）。

| 変更 | 内容 |
|---|---|
| `IntentEvent::Created(Created { intent })` 新設 | 材料は**作られた時点の intent 丸ごと** = 全属性。定義側の `Defined` が内容を焼かないのと対照的だが理由は明快で、定義には実ファイルというリードモデルが別にあるのに対し、**intent の属性は intent 自身にしか無い** |
| genesis `Intent::create(..) -> Result<(Intent, IntentEvent), IntentError>` | 対を返す。動詞 `create` は upstream の `intent-create` そのもの（`factory-naming.md` のドメイン語優先） |
| 補助 `Intent::resolve(..)` も対を返す形へ | 定義から計画を解決する経路も genesis である。`create` へ委譲するので検査点は 1 か所のまま |
| 再構成 `Intent::from_material(..) -> Result<Intent, IntentError>`（旧 `new`） | **イベントを作らない**（戻り値の型に `IntentEvent` が現れないことが保証）。構造体リテラルはここ 1 か所だけ |
| `#[serde(try_from = "IntentMaterial")]` | **復号も再構成経路を通す**。素の derive では Always Valid 検査を素通りし、壊れた歴史を読み戻した瞬間に不変条件が破れていた（改訂 8 の「検査は両経路で同一」）。**直列化側は derive のままなので書き出すバイトは変わらない** |
| `IntentExecution::start` が受け取る intent | **`create` の対の左**。`Started` が intent を丸ごと運ぶ現行形は維持（BR2.2 自己完結）。イベントに集約の写しが載るのは歴史の記録であり `aggregate-references.md` 違反ではない |

**ジャーナル接続はしていない**（改訂 8 の 5）。`Created` を `store` する `IntentRepository` は
U7（intent-create の実装）の課題である。

呼出側の移行は、`WorkflowDefinition` の 12 呼出を `from_artifacts` へ寄せたのと同じ整理にした
——**フィクスチャは既定で再構成コンストラクタ**（`Intent::new` の 29 箇所を機械置換）とし、
**genesis を演じる 3 箇所だけ `create` の対の左を渡す形**にした: ITF 準拠テスト
（`engine_loop_conformance.rs` — モデルトレース再生の起点）、ユースケース土台
（`use-case/src/orchestration/test_support.rs` — 本番の呼出側に最も近い）、および改訂 8 の 4 を
逐語で固定する新規テスト `an_execution_starts_from_the_left_of_the_intent_create_pair`。

副次として、orchestration ファサードの失効記述 3 件も是正した（`state()` / `from_state()` →
`snapshot()` / `from_snapshot()`、既に存在しない `start_from_plan_unchecked` のコマンド表行、
`Intent` を「Domain Primitive」に分類していた `pub use` の見出し）。

---

### (l) ドメインから永続化知識を全撤去した（改訂 9）

オーナー裁定「ドメインに永続化知識を含めるな。集約はどんな永続化知識からも中立」。
正典は `coding-rules/domain-persistence-neutrality.md`。

#### 撤去した側

| 対象 | 措置 |
|---|---|
| `IntentMaterial`（改訂 8 で入れた serde 復号の中間表現） | 削除 |
| serde の derive / 属性 / import | `core-command-domain` の **28 ファイル**から撤去 |
| `event_manifest.rs` | 削除（ジャーナル列の値は永続化語彙） |
| `IntentId` / `IntentExecutionId` の `AggregateId` 実装 | 撤去（aid 列の組み方はストアの語彙）。`TryFrom<String>` も `#[serde(try_from)]` 専用だったので畳んだ |
| 集約の serde plumbing（`into` / `try_from` の変換 impl） | 撤去。永続化境界は `snapshot()` / `from_snapshot()` の memento だけになった |
| `Cargo.toml` の `serde` / `event-store-adapter-rs` | 撤去。**この Cargo.toml が規則の機械強制**である |

`chrono` は残した（時刻の**値**は永続化知識ではない — 規則の対象外条項）。dev-dependency の
`serde_json` も残した（ITF 準拠テストが Quint トレースを `Value` として読む手段で derive は
使わない。機械強制の射程が `[dependencies]` であることは本セッション側の `2ab271b2` で明確化
されている）。

#### 行き先

| 層 | 持ち物 |
|---|---|
| **command interface-adapter**（書く側） | `orchestration/wire/` — イベント全 12 変種・スナップショット・Intent とその部品の DTO、閉集合の綴りの正本、`AggregateId` を満たすストア鍵 `AggregateKey`、`EVENT_MANIFEST` |
| **RMU**（読む側） | `orchestration/wire/` — **自前の**復号 DTO と綴りの正本、自前の `EVENT_MANIFEST`。スナップショット DTO とストア鍵は持たない（journal 表しか読まず、本家ストアに触れるのはテストだけ） |

書きは「domain の公開アクセサ → DTO → serde」、読みは「serde → DTO → domain の検査付き
再構成コンストラクタ」。**Always Valid の担保は落ちていない** — 担保の場所がドメインの serde
属性からこの層の変換関数へ移っただけで、復号は必ず `Intent::from_material` /
`IntentExecution::from_snapshot` を通る。

**綴りはドメインの `as_str` / `parse` を流用していない。** 同じ値でも面ごとに綴りが違うためで、
実例として `PhaseId` はジャーナル上 `"Ideation"`、`stage-graph.json` 上は `"ideation"` である。
流用すると片方の綴りを変えた瞬間にもう片方のバイトが壊れる。

#### バイト不変の証明

着手前に**現行のワイヤ形式を実測して採取**し、それを恒久テストの期待値リテラルにした
（12 変種 + スナップショット）。**書く側と読む側が同一のリテラルを独立に持つ**ので、
どちらかの綴りが動けばどちらかが落ちる — これが「型を共有せずにワイヤ形式の一致を保つ」ための
単体側の歯止めである。横断側は `journal_protocol_conformance`（アダプタが書き RMU が読む）、
`crash_reconstruction_test`、投影ゴールデン 19 本、移設後の `upstream_event_store_conformance`
が担う。実際、RMU を DTO へ切り替えた時点でも RMU の投影テストは**ドメイン serde が書いた行を
読んで**全緑であり、その逆も緑だった（切替の各段でこの交差を通している）。

#### 判断が要った 3 点

1. **スナップショット型の公開**（ブリーフ補足どおり）。`IntentExecutionSnapshot` を `pub` にし、
   読取アクセサ 12 本と検査付き構築（`IntentExecutionSnapshotBuilder`）を公開した。フィールドは
   クレート内に閉じたままである。ビルダーの主コンストラクタは「既定できない 3 点（実行 id・
   intent id・実効プラン）」を取る形へ改めた — 旧 `new(id, &Intent)` は birth 既定値を intent から
   導いていたが、**復号する側は Intent を持たない**ためである。`overlay` は構築引数になったので
   セッターは廃止した（入口を二重に持たない）。
2. **DTO 型の可視性**。`pub` にした。理由は、自クレートと `app` の適合テストが生の行を作るのに
   ストアの型引数を名指しする必要があり、`pub` な `IntentExecutionRepositoryImpl<S>` の
   `where` 節がクレート私有型を指すと `private_interfaces` で `-D warnings` に落ちるためである。
   見えるのは**アダプタの利用者**だけで、ドメインとユースケースは依存の向き（層 = クレート）に
   より参照できない。あわせてストアの型別名 `IntentExecutionSqliteStore` /
   `IntentExecutionMemoryStore` を公開し、テストが三つ組を綴らずに済むようにした。
3. **`upstream_event_store_conformance.rs` の移設**（判断を仰いだうえで実施）。domain の
   dev-dependency からも `event-store-adapter-rs` を落とす必要があるため、interface-adapter へ
   `git mv` して DTO 経由に書き換えた。検証の主語は「ドメイン型が本家契約を満たす」から
   「**この層の DTO が本家契約を満たし、往復でドメインへ戻る**」へ変わったが、往復の両端で
   ドメイン値の一致を見るので確認している性質は落ちていない。

#### 副次的な改善（報告事項）

**復号失敗の分類が精密になった。** 改訂 9 以前は serde の `try_from` 失敗が本家の
`DeserializationError` に畳まれるため `UndecodablePayload` にしかならなかったが、DTO を挟んだ
ことで「読めない」と「読めたが不変条件を破る」を分離できるようになった。これは
`CorruptCause::InvariantViolation` の doc（「`from_snapshot` の `Err`」）がもともと意図していた
分類である。既存テスト 1 本の期待値をこの分類へ更新した（`a_snapshot_that_breaks_an_aggregate_invariant_is_refused_by_the_decoder`）。

---

### (m) `CommitVerdictUseCase` をリポジトリ保持形へ戻した（改訂 10）

オーナー裁定「ユースケースはリポジトリの参照を保持し、`execute` 内部で利用する。リポジトリを
外で使うな」。改訂 5 の 3 / 改訂 6 で維持していた「Controller が `&Intent` を読んで渡す」は
**I8（読取専用ユースケース `Next` 専用のパターン）の誤適用**であり、撤回された。

| 変更 | 内容 |
|---|---|
| `IntentRepository` ポート新設 | 改訂 5 の「U7 で新設」を**前倒し**。当面 `find_by_id(&IntentId) -> Result<Intent, IntentRepositoryError>` のみ。`store` は intent-create を実装する U7 で足す（使われない口を先に開けない） |
| `IntentRepositoryError` 新設 | 実行側の `RepositoryError` と**別の面の別の型**。**`Conflict` を持たない** — 読取専用ポートに CAS は無く、楽観 version の競合は構成不能だからである（`corrupt_cause.rs` と同じ「実際に起きうる変種だけを各面が持つ」考え方） |
| `CommitVerdictUseCase<E, I>` | ポート 2 本（`execution_repository` / `intent_repository`）を保持する |
| `execute` から `&Intent` を除去 | 引数は**集約 ID 1 つ + 値オブジェクト 3 つ**だけになった |
| 内部フロー | ① 実行を再構成 → ② `aggregate.intent_id()` を読む → ③ `intent_repository.find_by_id` → ④ `&intent` を集約コマンドへ → ⑤ `store`。取り違えのガードは従来どおり集約側で発火する（ここでは構成上一致する） |
| `Conflict` 再試行 | attempt 全体をやり直すので**計画も引き直す**。`Intent` は不変なので再取得は無害である |
| 結線 | `interface-adapter` に結線テスト用の `InMemoryIntentRepository` を置いた。**実物の実装は U7** — intent の完全な材料が永続化されているのは今のところ各実行の `Started` だけで、読み先（intent 自身のジャーナル）の設計ごと U7 の課題である |

テストはユースケース内 fake（`InMemoryIntentRepository`）で、取得回数を観測点にした
（「1 試行につき 1 回・再試行で 2 回」）。計画が引けない場合が `RepositoryError` ではなく
`IntentRepositoryError` のまま伝播することも固定している。

#### `stage` 引数は残した（確認済み・回答待ち）

ブリーフの確定形は 3 引数（`id` / `transition` / `occurred_at`）と書かれているが、現行の
`execute` は `&Intent` のほかに **`stage: Option<&StageSlug>`** を取っており、これは
`ReportedTransition` に**入っていない**（実測: 5 変種が持つのは `artifacts` / `user_input` /
`feedback` / `reason` だけ）。`stage` は 2 か所で効いている:

1. **BR1.9 の通過済み再報告** — `is_stale_re_report` は「報告がカーソル以外を名指しした」ことを
   前提にした冪等 no-op である。
2. **`Conflict` 再試行の対象名指し** — `None` のまま再試行すると対象が「そのときのカーソル」へ
   再解決され、競合相手が先に承認していた場合に**報告されていない次ステージへ `Forward` を打つ**
   （現行 doc が明示している既知の穴）。

`StageSlug` は値オブジェクトなので §2b の一般則（「集約 ID や値オブジェクトを渡すのは OK」）に
そのまま適合する。したがって確定形の 3 引数は「`&Intent` を落とす」ことを示した略記と読み、
**`stage` は残した**。team-lead へ確認を送ってあり、`stage` も落とす裁定であれば BR1.9 と
再試行の名指しをどう保つかの設計が別途要るので、そこは独断しない。

---

## 6. 申し送り

1. **`security-design §2` の「検査点は `from_snapshot` の 1 か所」** — §5 (f)。分割後の実態に
   合わせた是正が要る。`aidlc/**` は所有ファイル外なので触っていない。
2. **`entities.md` の `WorkflowExecutionState` 節**（U3 functional-design）— 型名・属性数
   （17 → 12）・Builder 名・エラー名（`StateError` → `SnapshotError` で B5 の改名が巻き戻った）が
   すべて失効した。同じく所有ファイル外。
3. ~~**`EVENT_MANIFEST` の値**~~ — **解決済み**（2026-08-30 裁定、§5 (a)）。値は
   `intent-execution-event/1` へ改めた。なお `docs/specs/10-orchestration.md:44` と
   inception 期の成果物（`domain-design/decisions.md:435`、`contract-design/contract-summary.md`
   の 271 / 291 / 374 行）に旧綴りが残っている。いずれも所有ファイル外なので触っていない。
4. **`IntentRepository` は U7（intent-create 実装時）で新設**（改訂 5 の 5・改訂 8 の 5）。
   B12 ではポート定義も作っていない — テストは `Intent` を直接構築している。改訂 8 で
   `IntentEvent::Created` と対返しファクトリまでは揃ったので、U7 は
   `store(&created, &intent, ..)` の形をそのまま組める。
5. **「この intent の現在の実行」の解決**と**「同一 intent の生きた実行は同時に 1 つ」**の
   不変条件は、改訂 3 のとおり本 Bolt では決めていない（U6 / U7 の設計点）。`intents.json` には
   フィールドを足していない。
6. **合成ルート（U7）が `Intent` をどこから読むか**も未決。`CommitVerdictUseCase` は
   `&Intent` を受け取るだけで取得手段を持たない。Repository は再生用に `Started` から自前で
   復元するので、両者が同じ intent を指すことは id 照合ガードが担保する。
7. **`WorkflowDefinitionEvent` / `IntentEvent` の永続化先**は未接続（改訂 7 の 5・改訂 8 の 5）。
   定義の変異取込と intent-create が要件化した時点で、ジャーナル・Repository の書込経路を
   設計する。`EVENT_MANIFEST` は実行イベント専用の綴りなので、intent / 定義のジャーナルを
   起こす際は**別の manifest 値**が要る（型判別子は集約ごとに分ける）。
8. **将来「実行ごとに計画を再解決する」要件が出たら再裁定**（改訂 5 の意味論注記）。現在は
   `Intent` が不変なので「`Started` の写し」と「現在の Intent」は常に一致するが、同一 intent で
   実行ごとに `stages` が変わる要件が現れたら、`stages` は Intent ではなく**実行の開始材料**へ
   移す再設計が要る。
9. **`formal/orchestration/journal_protocol.qnt` の対応表コメント**が古い（Rust 名を参照する
   コメントが 5 行）。モデル本体は Rust 型名を参照しないので Quint ゲートは緑のまま。
10. **`docs/**` と `coding-rules/**` の旧名**はメインセッションの担当（ブリーフどおり触って
   いない）。
11. **メインセッション側の未コミット変更**が作業ツリーに残っている（`brief-1.md`、
    `coding-rules/`、`docs/`、`components.md`）。自分の所有ファイルではないのでコミットして
    いない。

---

### 改訂 9 由来の申し送り

12. **RMU のテストが書くスナップショット行の payload は `null` である。** RMU は journal 表しか
    読まないのでスナップショットの DTO を持たず、本家ストアの `A` 型引数を `serde_json::Value`
    で満たしている。RMU の主張には影響しないが、RMU のテストだけを見て「スナップショット行の
    形」を判断してはいけない — その正本は書く側の `WireSnapshot` である。
13. **`docs/specs` と inception 期の成果物**に「ドメイン層に serde が入る」旨の記述が残っている
    可能性がある（`decisions.md` の serde 受容は本セッション側で失効注記済みと承知しているが、
    他の箇所は未確認）。所有ファイル外なので触っていない。
14. ~~**改訂 10 は未着手**~~ — **実装済み**（§5 (m)）。残る課題は 2 つ: (a) `IntentRepository`
    の**実物の実装**（intent 自身のジャーナルの導入ごと U7）、(b) `execute` の `stage` 引数を
    残した判断の追認（§5 (m) の末尾、team-lead へ確認済み・回答待ち）。
15. **合成ルート（U7）はポートを 2 本注入する。** `CommitVerdictUseCase::new` の署名が
    `(execution_repository, intent_repository)` になったので、U7 の結線はこの形になる。
    アダプタ側にあるのは結線テスト用のインメモリ実装だけである。

## 7. コミット

`origin/main..HEAD` は **20 本**（本報告の反映コミットを含む）。うち委任側が 17 本、
本セッション側（正本・記録の整備）が `ecf268f3` / `7384aeed` の 2 本である。いずれも
`b12: ` 接頭辞、`git add` は明示パス、**push なし**。

| # | コミット | 内容 |
|---|---|---|
| 1 | `e27cfd86` | 集約 `WorkflowExecution` を `Intent` へ改名する（一族・ファイル名・`type_name`） |
| 2 | `40c90b94` | 集約のフィールド `intent_id` とアクセサを `id` / `id()` へ改める |
| 3 | `75ad3239` | 委任ブリーフと開発者報告を記録する |
| 4 | `033f8988` | 写しのビルダーを `IntentSnapshotBuilder` へ改め、クレート内私有に絞る |
| 5 | `156b5111` | 再訂正の反映を開発者報告へ記録する |
| 6 | `72a7de06` | 集約一族を `IntentExecution` へ改名する（改訂 2 の受け皿） |
| 7 | `2d71d357` | 実行の識別子 `IntentExecutionId` を新設する |
| 8 | `637916f3` | 静的な intent を表す `Intent` を新設する |
| 9 | `3dda7cbd` | 集約を `IntentExecution` へ縮小し、計画は `&Intent` で受け取る |
| 10 | `affb01cc` | 分割を開発者報告へ書き直す |
| 11 | `9cb09791` | 写しの語彙を `snapshot` へ揃え、確定した引数順と規則参照を反映する |
| 12 | `81edabee` | 改訂 6 確定・snapshot 語彙・規則参照を開発者報告へ反映する |
| 13 | `df749101` | `WorkflowDefinition` を集約規則（ファクトリは対を返す）へ適合させる（改訂 7） |
| 14 | `c6f50825` | 改訂 7 を開発者報告へ反映する |
| 15 | `ecf268f3` | （本セッション）集約規則 3 本の正典化・呼称の統一・設計記録と仕様の失効注記 |
| 16 | `fd2ee4d9` | ジャーナルの型判別子を `intent-execution-event/1` へ揃える（§5 (a) 裁定） |
| 17 | `76810545` | `EVENT_MANIFEST` 改名の裁定と実測を開発者報告へ反映する |
| 18 | `7384aeed` | （本セッション）manifest 改名の正本追従（5 箇所）と改訂 8 の裁定記録 |
| 19 | `9565cf64` | `Intent` を集約規則へ適合させる（`IntentEvent` 新設・`create` は対を返す。改訂 8） |
| 20 | （本コミット） | 改訂 8 を開発者報告へ反映する（自身の SHA は確定前なので記さない） |

改名フェーズ（1〜5）→ 分割フェーズ（6〜10）→ 語彙・規則適合フェーズ（11〜20）の 3 段である。
改訂 2 が来た時点で**巻き戻しは不要だった** — 集約側の改名がそのまま `IntentExecution` への
機械改名で流用できたためである。`3dda7cbd` だけは 1 コミットが大きいが、集約の縮小・ポート・
アダプタ・RMU・app が同時に動かないとビルドが通らないため分割できなかった。
