# developer-report-7 — 委任 7: カバレッジの回復（相対ゲート）（U3 / Bolt B5）

> 出典: `developer-brief-7.md`、`coverage-gaps-b5.md`、`code-generation-plan.md` §5.2〜§5.3、`unit-test-instructions.md`。
> 対象ブランチ `bolt/b5-u3-event-store-repository`（コミットはしていない — 作業ツリーに残してある）。

## 1. 結論

- **相対ゲート回復**: `bash scripts/coverage.sh --base origin/main` が `[PASS] relative gate` / `[PASS] absolute gate` の両方を出した。
  head **98.42%** ≥ base **97.39%**（前は head 96.81% < base 97.40% で赤）。
- **新規カバー行数**: `coverage-gaps-b5.md` が挙げた 10 ファイル 161 行のうち **149 行**をカバーした（残り 12 行、内訳は §4）。目標 +70 行を大きく超えている。
- **追加テスト**: 41 本（`cargo test --workspace` 623 → **664**、全緑）。
- **プロダクトコードの変更は 0 行**。挙動不変の最小リファクタも行っていない（到達不能と判断した分岐は無理に通さず §4 に列挙した）。

## 2. 追加テスト一覧

`+n` は追加したテスト関数の本数。「カバーした対象行」は `coverage-gaps-b5.md` の行番号（プロダクトコードの行番号は本委任で動いていない — 追記はすべて各ファイルのテストモジュール末尾に置いた）。

### 2.1 新規ファイル

| ファイル | +n | カバーした対象行 |
| --- | --- | --- |
| `modules/core/interface-adapter/tests/in_memory_workflow_execution_repository_test.rs` | 6 | `memory/workflow_execution_repository.rs` 32-34, 36, 39, 44-46, 58-60, 101-105, 114-121（**24 行 = 全部**） |

`InMemoryWorkflowExecutionRepository` には実装固有のテストがまったく無く、契約テスト（`workflow_execution_repository_contract.rs`）が通る経路だけが踏まれていた。SQLite 側の `workflow_execution_repository_impl_test.rs` と**同名・同趣旨**のテストを 6 本置き、片方だけ通る経路を残さないようにした（BR2.7）。スナップショットとジャーナルをずらす手段は、SQLite が生 SQL なのに対し in-memory は `EventStore::persist_event`（ジャーナルだけの追記）で作る — 実装に破壊用フックを開けない方針（BR2.8）は維持した。

- `a_store_without_any_row_reports_not_found`（`new()` の疎通も兼ねる）
- `a_journal_without_a_snapshot_is_corrupt_not_missing`（`MissingSnapshot`）
- `the_version_after_a_replay_is_the_sequence_of_the_last_applied_event`（replay ループ + `with_version`）
- `a_gap_in_the_replayed_journal_is_corrupt`（`apply_cause` の `SequenceGap` 腕）
- `a_replayed_event_naming_a_stage_outside_the_plan_is_corrupt`（`apply_cause` の `UnknownStage` 腕）
- `the_repository_hands_out_a_reader_over_the_same_store`（`event_store()`）

### 2.2 既存の統合テストへの追記（`tests/**`）

| ファイル | +n | カバーした対象行 |
| --- | --- | --- |
| `tests/event_store_impl_test.rs` | 7 | `event_store_impl.rs` 355, 388, 423-426, 428, 492-495, 516, 550-554, 572-573（**19 行**） |
| `tests/workflow_definition_repository_impl_test.rs` | 3 | `workflow_definition_repository_impl.rs` 488-491, 498-499, 613-618, 620, 628-630, 632, 634-636, 638, 645-647, 649, 661-666, 668, 674-676, 678, 685-687, 689（**40 行**） |
| `tests/workflow_execution_repository_impl_test.rs` | 2 | `workflow_execution_repository_impl.rs` 45, 61-63（**4 行**） |

`tests/event_store_impl_test.rs`（+7）:

- `an_append_only_write_starts_from_version_zero_and_refuses_the_same_sequence_twice` — 写しが無い集約の現在 version = 0（355）と、版が一致したままの `UNIQUE` 違反（492-495）。
- `a_journal_insert_that_fails_for_another_reason_is_reported_as_io` — `DROP TABLE journal` 後の追記。競合以外の SQL 失敗を `Conflict` に化けさせず `Io` で運ぶこと（388）。
- `a_genesis_write_against_an_existing_snapshot_conflicts` — 期待 version 0 なのに写しが既にある（`INSERT INTO snapshot` の主キー違反、550・552-554）。
- `a_snapshot_insert_that_fails_for_another_reason_is_reported_as_io` — `DROP TABLE snapshot` 後の genesis 書込（551）。
- `an_update_that_matches_no_snapshot_row_conflicts` — `UPDATE ... WHERE version = expected` の影響 0 行（572-573）。
- `a_version_beyond_the_json_exact_limit_is_refused_instead_of_being_rounded` — 2^53 超の version は `StateWire::encode` で弾く（516）。
- `a_snapshot_row_that_breaks_an_aggregate_invariant_is_corrupt` — 復号は通るが `from_state` が落ちる行（`seq_nr = 0`、423-426・428）。

`tests/workflow_definition_repository_impl_test.rs`（+3）:

- `every_closed_set_field_is_reported_as_malformed_with_the_key_that_caused_it` — 表駆動で 7 キー（`number` / `execution` / `mode` / `consumes[].conditional_on` / `requires_stage[]` / `rules_in_context[].scope` / `review_class`）の未知値を 1 本にまとめた。既存テストが押さえている `slug` / `phase` は重複させていない。
- `a_scopes_path_that_is_not_a_directory_is_reported_instead_of_being_treated_as_empty` — `scopes_dir` がファイル（`read_dir` が `NotFound` 以外で失敗、488-491）。
- `an_identity_entry_that_cannot_be_read_as_a_file_is_reported_with_its_path` — `aidlc-*.md` という名の**ディレクトリ**（`read_to_string` が失敗、498-499）。

`tests/workflow_execution_repository_impl_test.rs`（+2、および `Fixture::store()` ヘルパ 1 本）:

- `a_replayed_event_naming_a_stage_outside_the_plan_is_corrupt`（`apply_cause` の `UnknownStage` 腕、45）
- `the_repository_hands_out_a_reader_over_the_same_store`（`event_store()`、61-63）

### 2.3 `src/**` のインラインテストモジュールへの追記

`pub(super)` / 私有関数・`tables` のような内部状態にしか触れない経路は統合テストから届かないため、既存の `#[cfg(test)] mod tests` の末尾へ追記した（`wire/mod.rs` だけはテストモジュール自体が無かったのでファイル末尾に新設した）。プロダクトコードは 1 行も触っていない。

| ファイル | +n | カバーした対象行 |
| --- | --- | --- |
| `src/orchestration/wire/mod.rs` | 8 | 93, 126, 143, 171, 188, 196, 204, 214, 263, 320, 357（**11 行 = 全部**） |
| `src/orchestration/memory/in_memory_event_store.rs` | 5 | 76-82, 119-122, 124, 162-165, 196, 209, 212（**19 行 = 全部**） |
| `src/orchestration/wire/state_wire.rs` | 3 | 96, 101, 154, 158, 162（**5 行**） |
| `src/orchestration/event_store_impl.rs` | 2 | 113, 223-227（**6 行**） |
| `src/orchestration/workflow_definition_repository_impl.rs` | 2 | 107-108, 110, 119-124, 129-131, 785, 788（**14 行**） |
| `src/orchestration/wire/event_wire.rs` | 1 | 286（**1 行**） |
| `modules/core/use-case/src/orchestration/event_store.rs` | 1 | 95-97（**3 行**） |
| `modules/core/use-case/src/orchestration/projection_name.rs` | 1 | 78-80（**3 行**） |

主なものだけ補足する。

- `wire/mod.rs`（新設 8 本）: `WireObject` の読取口が「オブジェクトでない値」「`string \| null` に別の型」「配列でない値」「要素の型違い（texts / slugs / bools / u32s）」を拒否すること、`parse_checkbox` が 1 文字ちょうどを要求すること、`parse_direction` が 3 語の閉集合であること、`StageEntryWire::to_entry` が `conditional` に真偽値を要求すること。parse-don't-validate の検査点 1 / 2 に対応する。
- `in_memory_event_store.rs`（5 本）: 集約不変条件が破れた写しの行（`corrupt` 自由関数 + `decode_snapshot`）、同一通番の 2 度目の追記、genesis 書込 × 既存写し、古い版からの書込、2^53 超の version。後ろ 4 本は §2.2 の SQLite 側と 1:1 に対応させ、**両実装が同じ観測を返すこと**を並べて固定した。
- `event_wire.rs`（1 本）: `payload` 側の `type` が閉集合の外（列は閉集合の中）。既存の `an_event_type_column_that_disagrees_with_the_payload_tag_is_rejected`（両方閉集合内で食い違う → `UndecodablePayload`）とは**原因が違う**（→ `UnknownEventType`）ことを明示した。
- `event_store_impl.rs`（2 本）: `OperationInterrupted` の写像（`Other` へ畳まない）と `Debug` の描画（接続・時計を出さない）。
- `workflow_definition_repository_impl.rs`（2 本）: ADR-008 で足した 2 変種（`NotFound` / `HarnessIdentity`）の診断文言、frontmatter の空行・`#` コメント・`:` 無し行の読み飛ばし。

## 3. 検査結果（実測）

```
$ cargo fmt --all --check
（出力なし、exit 0）

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.97s
（警告 0、exit 0）

$ cargo lint
（出力なし、exit 0）

$ PROPTEST_RNG_SEED=20260823 cargo test --workspace
total passed: 664   failed: no      （委任 6 時点 623 → +41、全緑）
```

`clippy::indexing_slicing` / `clippy::panic` に触れる追記は無かったため、新しい `#![allow(...)]` は 1 つも足していない（既存の file 単位 allow をそのまま使っている）。`redundant_clone` に 1 か所引っ掛かったので、`fixture` を後段でも参照する形へテスト側を書き直して解消した。

## 4. coverage.sh の出力（before / after）

### before（委任 7 着手前、コミット 989b2ae）

```
==> head の line coverage を計測中 (/Users/j5ik2o/orca/workspaces/amadeus-ng/docs)
head line coverage: 96.80997341644513%
[PASS] absolute gate: head (96.80997341644513%) >= threshold (90.0%)
==> base (origin/main) を一時 worktree にチェックアウト中
base (origin/main) line coverage: 97.39995207284927%
[FAIL] relative gate: head (96.80997341644513%) < base (97.39995207284927%) - tolerance (0.01)
```

### after（本委任の作業ツリー）

```
==> head の line coverage を計測中 (/Users/j5ik2o/orca/workspaces/amadeus-ng/docs)
head line coverage: 98.42091176732983%
[PASS] absolute gate: head (98.42091176732983%) >= threshold (90.0%)
==> base (origin/main) を一時 worktree にチェックアウト中
base (origin/main) line coverage: 97.38797028516655%
[PASS] relative gate: head (98.42091176732983%) >= base (97.38797028516655%) - tolerance (0.01)
exit=0
```

- 絶対ゲート: 96.81% → **98.42%**（床 90.0% に対し +8.42pt）
- 相対ゲート: head − base = **+1.03pt**（許容誤差 0.01 に対し十分な余裕）

### 到達不能として残した行（12 行）

`cargo llvm-cov report --lcov` を after の計測データから起こして突合した実測値。

| ファイル | 残り | 理由 |
| --- | --- | --- |
| `workflow_definition_repository_impl.rs` | 563 | `serialize_grid` の `if let Some(column) = grid.column(scope)` の else 側。`scope_names()` は同じ `columns` マップの鍵集合を返すので、`column(scope)` が `None` になる scope は構造上存在しない。 |
| 同上 | 596-597, 599-602 | `compute_revision` の 2 つの `map_err`。`to_value` の入力 `RevisionInput` は String / Option\<String\> / Vec / bool / `serde_json::Value` だけで構成され失敗経路が無く、`hash_canonical(..).rendered()` は常に `sha256:<hex64>` なので `DefinitionRevision::parse` も必ず通る。 |
| 同上 | 749 | `scope_file_paths` の `path.file_name().and_then(\|n\| n.to_str())` が `None` になる continue。`read_dir` は `..` を返さないので `file_name()` は必ず `Some`、`to_str()` が `None` になるのは非 UTF-8 のファイル名だけである。**開発機（macOS / APFS）は非 UTF-8 のファイル名の作成自体を拒否する**ため、この行を通すテストはローカルで書けない（Linux なら `OsStrExt::from_bytes` で作れるが、macOS で落ちるテストになるので入れなかった。§6 に申し送り）。 |
| `wire/event_wire.rs` | 425 | `match tag` の `_` 腕。直前に `EVENT_TYPES.contains(&tag)` を通しているので構造上到達しない（実装側にも同趣旨のコメントがある）。 |
| `wire/event_wire.rs` | 510 | **テストヘルパ自身**の分岐（`cause()` の `other => panic!`）。実装が正しい限り到達しない。 |
| `wire/state_wire.rs` | 257, 273 | 同じくテストヘルパ自身の分岐（`cause()` の `panic!` と `let ... else { panic! }`）。 |

`event_store_impl.rs` / `memory/in_memory_event_store.rs` / `memory/workflow_execution_repository.rs` / `workflow_execution_repository_impl.rs` / `wire/mod.rs` / `use-case/{event_store,projection_name}.rs` は**未カバー 0 行**になった。

## 5. 設計上気づいた点・疑問

1. **相対ゲートの許容誤差 0.01pp は、base 側の実測ゆらぎより狭い可能性がある。** 同じ `origin/main`（db6c0a1）を同じ `PROPTEST_RNG_SEED=20260823` で 3 回計測した実測値は 97.39995207 / 97.38797029 / 97.38797029 で、最大差 **0.012pp** が出た（2 回は完全一致）。PBT のシードは固定済みなので、残るゆらぎ源は PBT ではない — `busy_timeout` 超過や FS 待ちのようなタイミング依存テストが、実行のたびに違う分岐へ落ちている可能性が高い。今回は head − base = +1.03pt なので影響しないが、head と base が拮抗した Bolt では偽陽性の赤を出しうる。ゆらぎ源の特定（どのテストの行カバレッジが揺れるか）を後続 intent の課題として申し送る。本委任のスコープ（`scripts/**` は所有外）では触っていない。
2. **`cargo llvm-cov` は `src/**` のインライン `#[cfg(test)] mod tests` を計測対象に含む。** `coverage-gaps-b5.md` が `use-case/event_store.rs` 95-97（`FakeStore` の中）や `state_wire.rs` 257 / 273（テストヘルパ）を未カバー行として挙げていたのはそのためである。したがって「インラインテストを増やす」こと自体が分母も増やす。今回はすべて 100% 実行される追記なので比率は上がったが、テストヘルパに未実行の分岐（`panic!` する else 腕など）を作ると**カバレッジを下げる**副作用がある。`scripts/coverage.sh` の除外方針（composition root のみ）を今後見直すなら、テストヘルパの扱いは論点になる。
3. **`scope_file_paths` は名前だけを見てディレクトリも候補に入れる。** `aidlc-x.md` という名のディレクトリがあると `read_to_string` が失敗し、`GraphReadError::ScopeFile` で**致命**になる（今回テストで固定した）。有効スコープの権威（F7）が欠けたまま進まないという意味では正しいが、upstream 側の態度は未確認である（`load_scopes` の TODO(spec: 12 §11) と同じ性質の穴）。仕様側で裏を取るかどうかはコンダクタ判断に委ねる。
4. **`persist_event` の genesis 経路（写しが無い集約に版 0 で追記）は、今回まで両実装ともテストが無かった。** ポート契約としては定義済み（`event_store.rs` の rustdoc）だが、Repository がこの経路を使わない（毎回スナップショットも更新する — ADR-001）ため落ちていた。今回 in-memory / SQLite / `FakeStore` の 3 実装すべてで固定したので、U7 で `persist_event` を実際に使う判断をしても観測は揃っている。

## 6. 未了事項

- **`workflow_definition_repository_impl.rs:749`（非 UTF-8 ファイル名）は未カバーのまま。** macOS では再現テストが書けないため見送った。CI（ubuntu）でだけ走る `#[cfg(target_os = "linux")]` テストを足す案はあるが、「ローカルで実行されないテスト」を増やす是非はオーナー裁定が要ると判断して入れていない。
- **`git add` / `git commit` / `git push` は実行していない。** 変更はすべて作業ツリーに残してある（`modules/` 配下 11 ファイル変更 + 新規テスト 1 ファイル、および本報告 md）。
- 本委任では**プロダクトコードを 1 行も変更していない**ため、挙動不変リファクタの報告事項は無い。
