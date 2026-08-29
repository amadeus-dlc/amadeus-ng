# B12 開発者報告 1 — `Intent` 構造体 + `IntentExecution` 集約への分割

対象ブリーフ: [`brief-1.md`](brief-1.md)（改訂 3 まで反映）
ブランチ: `bolt/b12-intent-aggregate-rename`（origin/main 基準、**push なし**）
検証の `CARGO_TARGET_DIR`: `target-delegate`（`cargo lint` のみ `target-delegate-lint`）

---

## 0. 結論（先に）

**受入基準はすべて緑**である（改訂 3 反映後に全ゲートを再実行した実測）。
`cargo test --workspace` は **798 passed / 0 failed**（origin/main は 774 なので **+24**）。
`tests/` の差分は 0 で、投影ゴールデンは無改変のまま全緑である。

本 Bolt は途中で 3 回の裁定を受けて到達点が変わった。最終形は**単純改名ではなく分割**である:

| | 役割 |
|---|---|
| **`Intent`**（新設・不変構造体） | 静的な intent — 識別子・定義のピン・依頼・解決済み計画・走査結果。Always Valid、変異メソッドなし |
| **`IntentExecution`**（集約） | 1 回の実行 — `id: IntentExecutionId` と `intent_id: IntentId`、および実行時状態だけ |
| **`IntentExecutionId`**（新設） | 実行自身の識別子（1 intent : n 実行なので intent の識別子を借りられない） |

計画が要る判断は `&Intent` を**引数で受け取り**、入口で `intent.id() == self.intent_id` と
計画長の一致を照合する（`coding-rules/aggregate-references.md`）。

固定フィクスチャに旧名や `"intent_id"` のバイトが埋まっている箇所は**発見されなかった**（§3）。
したがって「止めて報告」の条件には該当しなかった。

---

## 1. 到達点（改訂 3 後の対応表・実測）

| 旧 | 新 |
|---|---|
| `WorkflowExecution`（集約） | `IntentExecution`（実行時状態だけに縮小） |
| （無し） | **`Intent`**（新設・静的側の不変構造体） |
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

新設ファイル: `domain/src/orchestration/intent.rs` / `intent_execution_id.rs` / `uuid_v7.rs`。
削除ファイル: `domain/src/orchestration/start_error.rs`。

---

## 2. 外形不変の証明（受入基準）

| 観点 | 実測 |
|---|---|
| `tests/golden/**` | `git status --short -- tests/` が **0 件**（読むだけ・1 バイトも変えていない） |
| 投影ゴールデン | `projection_golden_test.rs` の 19 本が**無改変で全緑**（フィクスチャの `Started` 組み立てだけ新形に追随） |
| 監査語彙 | `WORKFLOW_*` に触れていない（RMU の投影核は無改変） |
| `EVENT_MANIFEST` | `"workflow-execution-event/1"` を据え置き（§5 (a)） |

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
| 4 | `cargo test --workspace`（退行 0） | **緑**。**798 passed / 0 failed**（origin/main 774 → **+24**） |
| 5 | `scripts/quint-gate.sh` | **緑**（exit 0） |
| 6 | `scripts/coverage.sh --base origin/main` | **緑**。head **98.61797%**。絶対 `[PASS] >= 90.0%`、相対 `[PASS] head >= base (98.52600%) - 0.01` |
| 7 | プロダクトコードに `unwrap` / `expect` 0 件 | **緑**（clippy の `unwrap_used` / `expect_used` deny で機械強制） |
| 8 | 外形不変 | **緑**（§2） |
| 9 | `grep -rn "WorkflowExecution" modules/ --include='*.rs'` | **緑**。**0 件** |
| 10 | `git log --follow` でファイル履歴が追える | **緑**（改名はすべて `git mv`） |
| 11 | `Intent` が変異メソッドを持たない | **緑**。`grep -c "&mut self" intent.rs` = **0** |
| 12 | `IntentExecutionId` に `IntentId` と同等の形式検査テスト | **緑**（15 本） |
| 13 | genesis 新形のテスト | **緑**（`start_records_the_definition_identity_and_the_resolved_plan` ほか） |
| 14 | `Started` payload に intent が載るラウンドトリップ | **緑**（`the_started_payload_round_trips_the_intent_through_serde`） |
| 15 | 集約状態に Intent 由来の静的フィールドが残っていない | **緑**（`the_snapshot_carries_no_static_material_from_the_intent` が写しの JSON を逐語で検査） |
| 16 | `&Intent` ガード（id 不一致・長さ不一致）のテスト | **緑**（`a_command_refuses_an_intent_that_belongs_to_another_intent` ほか 5 本） |

### テストの増減の内訳（実測）

テスト関数は **755 → 779**（+24）。**14 本が消え、38 本が増えた**。消えた 14 本の行き先:

- 3 本（`an_empty_stage_list_is_refused` ほか）→ `Intent::new` の不変条件テストへ移設（intent.rs）
- 5 本（旧 memento モジュール）→ 新しいスナップショットテスト 4 本へ置換
- 4 本（`start_error.rs`）→ `IntentError` のテストへ移設（Display は 5 変種すべてを逐語で固定）
- 1 本（`from_state_rejects_a_broken_invariant`）→ `..._runtime_invariant` へ改名
- 1 本（`workflow_execution_conforms_to_every_committed_engine_loop_trace`）→ `intent_conforms_...` へ改名

**カバレッジが失われた箇所は無い**（相対ゲートも base を上回っている）。

---

## 5. 判断が要った点

### (a) `EVENT_MANIFEST` の値は据え置いた

`EVENT_MANIFEST = "workflow-execution-event/1"` は改名すれば `intent-execution-event/1` 相当に
なる綴りである。据え置いた理由は 2 つ:

1. **改名一族の表に無い。** 表は文字列値の改名を 1 つだけ（`AGGREGATE_TYPE_NAME`）明示している。
2. **doc 自身が逐語固定を宣言している。** 定数のテストに「綴りは行に書かれて残る値である —
   変えると既存行が読めなくなるので逐語で固定する」と書かれており、値の変更は既存ジャーナル行を
   `Corrupt` にする破壊的変更である。

受入基準 9 とは衝突しない（小文字・ハイフンなので `WorkflowExecution` に部分一致しない）。
**判断を仰ぎたい**: 値も揃えるなら別 Bolt での実施を推奨する（永続化形式の変更である）。

### (b) 再生に要る `&Intent` の入手経路（選択肢 A を採った）

再構成は「最新スナップショット + 以降のイベント再生」で行うが、写しから計画が消えたため、
`apply_event` が要求する `&Intent` の出所が無くなった。ブリーフはここを決めていない。

**ブリーフ自身の制約から選択肢は 1 つに絞れた**ので、質問を出したうえで A で進めた:

- ポートに `&Intent` を足す案は、改訂 2 が `find_by_id(&IntentExecutionId)` と署名を確定させて
  いるので不可。
- 写しに `stages` を残す案は、改訂 3 が追加した受入基準に反するので不可。
- イベントが stage を index で運ぶ案は、`Started` 以外の payload 構造の変更になるので不可。

採った形（A）: `IntentExecutionRepositoryImpl` が**ジャーナル先頭の `Started` から**その時点の
intent を復元して再生に使う。`Started` は genesis 専用なので必ず 1 件目にある。先頭が別変種・
別 manifest・0 件のジャーナルは `Corrupt` で止める（テスト
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

### (f) `from_state` の検査点が弱くなった

写しに計画が載らなくなったので、復号時に検査できるのは**実行時の不変条件だけ**である
（長さ・通番・カーソル・park・active の数）。計画を要する `no_gate_bypass` は `&Intent` が渡る
コマンド・適用の側へ移した。`security-design §2`「検査点は 1 か所」の文言は、改訂 3 の分割に
より「実行時の検査点は `from_state`、計画を要する検査点は `&Intent` を受け取る面」の 2 か所に
なる。**設計文書側の是正が要る**（§6）。

### (g) `JournalEntry` の識別子は実行識別子へ

ジャーナル行の集約キーは実行識別子になったので、`JournalEntry::intent_id()` は
`execution_id()` へ改めた。投影核はこの値を使っていないので、外形には影響しない（§2）。

---

## 6. 申し送り

1. **`security-design §2` の「検査点は `from_state` の 1 か所」** — §5 (f)。分割後の実態に
   合わせた是正が要る。`aidlc/**` は所有ファイル外なので触っていない。
2. **`entities.md` の `WorkflowExecutionState` 節**（U3 functional-design）— 型名・属性数
   （17 → 12）・Builder 名・エラー名がすべて失効した。同じく所有ファイル外。
3. **`StateError` を `SnapshotError` へ戻すか** — `SnapshotError` はこの型自身の旧名であり
   （B5 で改名、`entities.md:126` に「旧名の再エクスポート・型エイリアスは残さない」と記録）、
   戻すのは過去の裁定の巻き戻しになる。据え置いて裁定を仰ぐ。
4. **`EVENT_MANIFEST` の値** — §5 (a)。
5. **「この intent の現在の実行」の解決**と**「同一 intent の生きた実行は同時に 1 つ」**の
   不変条件は、改訂 3 のとおり本 Bolt では決めていない（U6 / U7 の設計点）。`intents.json` には
   フィールドを足していない。
6. **合成ルート（U7）が `Intent` をどこから読むか**も未決。`CommitVerdictUseCase` は
   `&Intent` を受け取るだけで取得手段を持たない。Repository は再生用に `Started` から自前で
   復元するので、両者が同じ intent を指すことは id 照合ガードが担保する。
7. **`formal/orchestration/journal_protocol.qnt` の対応表コメント**が古い（Rust 名を参照する
   コメントが 5 行）。モデル本体は Rust 型名を参照しないので Quint ゲートは緑のまま。
8. **`docs/**` と `coding-rules/**` の旧名**はメインセッションの担当（ブリーフどおり触って
   いない）。
9. **メインセッション側の未コミット変更**が作業ツリーに残っている（`brief-1.md`、
   `coding-rules/` の 7 ファイル）。自分の所有ファイルではないのでコミットしていない。

---

## 7. コミット

意味単位で 6 本（いずれも `b12: ` 接頭辞、`git add` は明示パス、**push なし**）。

| コミット | 内容 |
|---|---|
| `e27cfd8` | 集約 `WorkflowExecution` を `Intent` へ改名する（一族・ファイル名・`type_name`） |
| `40c90b9` | 集約のフィールド `intent_id` とアクセサを `id` / `id()` へ改める |
| `75ad323` | 委任ブリーフと開発者報告を記録する |
| `033f898` | 写しのビルダーを `IntentSnapshotBuilder` へ改め、クレート内私有に絞る |
| `156b511` | 再訂正の反映を開発者報告へ記録する |
| `72a7de0` | 集約一族を `IntentExecution` へ改名する（改訂 2 の受け皿） |
| `2d71d35` | 実行の識別子 `IntentExecutionId` を新設する |
| `637916f` | 静的な intent を表す不変構造体 `Intent` を新設する |
| `3dda7cb` | 集約を `IntentExecution` へ縮小し、計画は `&Intent` で受け取る |

前半 5 本は改名フェーズ、後半 4 本が分割フェーズである。改訂 2 が来た時点で**巻き戻しは
不要だった** — 集約側の改名がそのまま `IntentExecution` への機械改名で流用できたためである。
`3dda7cb` だけは 1 コミットが大きいが、集約の縮小・ポート・アダプタ・RMU・app が
同時に動かないとビルドが通らないため分割できなかった。
