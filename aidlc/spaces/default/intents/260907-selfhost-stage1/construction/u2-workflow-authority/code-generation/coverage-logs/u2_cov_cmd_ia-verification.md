# u2_cov_cmd_ia 検証報告 — `core-command-interface-adapter` のカバレッジ向上

担当 crate: `core-command-interface-adapter`（`modules/core/command/interface-adapter/`）。
プロダクトコードは変更していない（テスト専用の `#[cfg(test)] mod` 宣言 2 か所を除く。後述）。
ログは `coverage-logs/u2_cov_cmd_ia/01`〜`04`。

## 1. 着手前後の crate 行カバレッジ（crate 単体計測）

計測: `cargo llvm-cov --no-report -p core-command-interface-adapter --tests` → `cargo llvm-cov report --json --ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$'`。JSON の `files[].summary.lines` を集計（`scripts/coverage.sh` と同じ指標）。

| | 総行 | カバー | 行カバレッジ | 未カバー |
| --- | ---: | ---: | ---: | ---: |
| 着手前（`01-baseline-crate-coverage.log`） | 6,995 | 5,729 | 81.90% | 1,266 |
| 着地後（`03-after-crate-coverage.log`） | 6,995 | 6,789 | **97.06%** | 206 |

注記:
- 着地後の計測は専用 `CARGO_TARGET_DIR`（scratchpad 配下）で行った。共有 `target/` は他担当の `cargo llvm-cov clean` で profraw が消え、混在計測になるためである（実際に一度消えて再計測した）。着手前の計測は共有 `target/` で行ったが、テスト実行ログは 18 バイナリ全件 ok を記録している。
- 着手前の初回 2 試行はサンドボックス下で統合テストの実行ファイルが `SIGKILL` され失敗した（一過性。ログ先頭に注記、サンドボックス外で再計測）。
- 対象 28 ファイルの crate 単体基準の未カバー行は **977 → 128（86.9% 減）**。workspace 基準（brief の 576 行）は親の最終計測で確認する。crate 単体の未カバーは workspace 基準の上位集合（他 crate のテストが踏む行を含む）なので、crate 単体で減った行は workspace でも減る。

### 対象ファイル別

| ws未カバー(brief) | 着手前(crate) | 着地後(crate) | 総行 | ファイル（`src/orchestration/`） |
| ---: | ---: | ---: | ---: | --- |
| 89 | 152 | 2 | 152 | `session_audit_repository_impl.rs` |
| 68 | 68 | 29 | 164 | `plan_approval_runtime_repository_impl.rs` |
| 65 | 65 | 12 | 158 | `workflow_continuation_repository_impl.rs` |
| 52 | 52 | 2 | 106 | `dto/hook_health_event_dto.rs` |
| 40 | 46 | 4 | 113 | `dto/plan_approval_event_dto.rs` |
| 29 | 61 | 8 | 309 | `dto/intent_execution_event_dto.rs` |
| 25 | 25 | 3 | 149 | `artifact_audit_repository_impl.rs` |
| 25 | 25 | 3 | 143 | `hook_health_repository_impl.rs` |
| 22 | 59 | 6 | 137 | `dto/reported_dto.rs` |
| 18 | 30 | 1 | 306 | `dto/intent_execution_dto.rs` |
| 17 | 17 | 17 | 566 | `workflow_definition_repository_impl.rs` |
| 15 | 17 | 0 | 74 | `dto/learnings_captured_dto.rs` |
| 15 | 23 | 0 | 23 | `dto/jump_scope_dto.rs` |
| 14 | 14 | 14 | 882 | `compiled_definition_repository_impl.rs` |
| 12 | 21 | 0 | 21 | `dto/session_audit_event_dto.rs` |
| 10 | 10 | 10 | 449 | `intent_repository_impl.rs` |
| 8 | 8 | 2 | 185 | `dto/plan_approval_runtime_dto.rs` |
| 7 | 14 | 7 | 520 | `intent_execution_repository_impl.rs` |
| 7 | 47 | 0 | 47 | `dto/answer_recorded_dto.rs` |
| 5 | 30 | 3 | 78 | `dto/active_directive_dto.rs` |
| 5 | 52 | 0 | 52 | `dto/decision_recorded_dto.rs` |
| 5 | 5 | 1 | 49 | `dto/plan_approval_evidence_dto.rs` |
| 5 | 5 | 3 | 38 | `dto/hook_health_dto.rs` |
| 4 | 8 | 0 | 60 | `dto/jumped_dto.rs` |
| 4 | 53 | 0 | 53 | `dto/memory_journals_observed_dto.rs` |
| 4 | 5 | 1 | 34 | `dto/run_floor_dto.rs` |
| 3 | 33 | 0 | 154 | `dto/intent_dto.rs` |
| 3 | 32 | 0 | 32 | `dto/prompt_observed_dto.rs` |
| **576** | **977** | **128** | | |

## 2. 追加したテスト（40 件・すべて所有 crate 内）

TDD の red は「プロダクトコードを変えない」前提では踏めないため、各テストは「その行が担う契約」を先に書き、実行して green を確認し、`cargo llvm-cov` の未カバー行の消滅で「その行を実際に検証した」ことを確かめた（refactor は `patched` / `failure_vocabulary_contract` などの共通化）。`assert!(true)` 型、実装の写し、命中目的の空呼び出しは書いていない。

### Repository 実装（実 SQLite = `tempfile` + 本家 `EventStoreForMemory`）

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `src/orchestration/session_audit_repository_impl_tests.rs`（新規。impl 側に `#[cfg(test)] #[path] mod tests;` を追加） | 5 | NotFound を捏造しない／保存→再構成で記録・観測 ID が往復／同版二重保存は `Conflict{expected,actual}`／差分イベント再生／manifest 不一致・復号不能イベント・復号不能 snapshot・通番枯渇 snapshot・鍵不一致 snapshot を `Corrupt`（通番の有無つき）で拒否／他集約のイベント書込を拒否／新規作成に読取済み版を添える契約違反を失敗として返す／SQLite の媒体失敗（他接続の `BEGIN EXCLUSIVE` → `WouldBlock`、`journal` 表喪失 → `Io{Other}`、壊れた payload → `InvalidData`、いずれも `path` 付き）／存在しないディレクトリの `open` → `NotFound`／失敗語彙が memory・SQLite 両実体で同一（`read_error`・`io` の全分岐） |
| `src/orchestration/plan_approval_runtime_repository_impl_tests.rs`（新規。同上の宣言追加） | 4 | manifest/通番不一致・復号不能イベント・復号不能 snapshot・読めない payload・snapshot 喪失（履歴のみ）を `Corrupt` で拒否（NotFound に丸めない）／契約違反書込を `Corrupt`／`WouldBlock`・`journal` 喪失の伝播／`open` 失敗／`read_error` の `OtherError`・非 SQLite `IOError` の分類 |
| `src/orchestration/workflow_continuation_repository_impl_tests.rs`（既存に追記） | 5 | 上記 session audit と同じ破損 5 種・他集約書込・契約違反（memory/SQLite 両方）／SQLite 媒体失敗 3 種／`open` 失敗／失敗語彙の両実体一致 |
| `src/orchestration/hook_health_repository_impl_tests.rs`（追記） | 4 | 復号不能 snapshot・通番枯渇 snapshot・契約違反書込（両実体）／`open` 失敗／失敗語彙の両実体一致 |
| `src/orchestration/artifact_audit_repository_impl_tests.rs`（追記） | 4 | 同上 |
| `tests/intent_execution_repository_impl_test.rs`（追記） | 1 | `find_for_approval_origin` は実行 ID だけで実行を引き、別 space を名乗っても同じ実行を返し、存在しなければ `NotFound` |

### DTO（`src/orchestration/dto/decode_contract_tests.rs` 新規、`dto/tests.rs` から `#[path]` で読み込み）

17 件。方針は「正しい DTO を JSON に落としてから 1 か所だけ壊す」（`patched(json, "a.b.0.c", value)`）。フィールド名を手書きしないので綴りの変更に追随する。

| テスト | 検証した契約 |
| --- | --- |
| `every_execution_event_survives_the_write_read_round_trip` | 実行イベント 22 例（SingleStageRunStarted / PipelineLinkCompleted / TaskSynchronized / HealthChecked / MemoryJournalsObserved / DecisionRecorded×2 / PromptObserved / PlanAnswerLogged / AnswerRecorded×3 / DirectiveContextInvalidated×2 / Jumped×2 / Reported×7 / LearningsCaptured）が書き→読みで同値 |
| `a_corrupt_field_is_rejected_by_name_instead_of_being_rounded_to_success` | 41 通りの破損がフィールド名付き `Malformed` で拒否される（id / aggregate_id / stage / answer_id / disposition / summary / direction / scope.executes / steps / report_id / human_before / approval_observation_id / learnings.* / space / intent ほか。NoOp 3 変種と Error/RunStage/LoadSteering の各 stage を含む） |
| `invariant_breaking_rows_are_rejected_as_invariant_violations` | 根拠と警告の同時保持、遷移と手順列の不一致、計画承認根拠と要約ファイルの同時保持、単位名文法外を `InvariantViolation` |
| `the_run_floor_restores_every_known_boundary_and_rejects_unknown_ones` | 境界種別 4 種の復元と未知種別の拒否（`run_floor.kind`） |
| `interaction_identifiers_outside_their_grammar_are_rejected_by_name` | `plan_answer` / `approval_observations` / `interactions.consumed_human` の文法外拒否、列長不一致の `InvariantViolation` |
| `pending_and_summary_prompts_survive_the_snapshot_round_trip` | 保留質問・要約質問・最新応答・消費済み応答を含む snapshot の往復 |
| `rows_written_before_optional_columns_existed_are_read_with_their_defaults` | `interactions` / `practices_affirmed` / `memory_empty_reported` 欄が無い行は既定値で読む（未記録であって破損ではない） |
| `a_recorded_directive_survives_the_snapshot_round_trip` | `Error` / `LoadSteering` 指示の往復 |
| `every_plan_approval_event_survives_the_write_read_round_trip` | 共有承認イベント 10 変種（Created / ChallengeIssued / ResponseObserved / ResponsePrepared / AnswerRecorded / AnswerCompleted / AnswerAborted / GenerationCertified / GenerationRevoked / InvalidationResolved）の往復 |
| `a_requested_generation_survives_the_plan_approval_round_trip` | GenerationRequested の往復 |
| `plan_approval_events_reject_identifiers_and_choices_outside_the_closed_sets` | `event_id` / `runtime_id` / `choice` / `operation_id`（呼び出し箇所 6 か所）/ `session` の拒否と、`Request Changes` の受理 |
| `a_challenge_whose_stored_id_disagrees_with_its_evidence_is_rejected` | 保存された発行 ID が根拠から導いた ID と食い違えば `InvariantViolation` |
| `every_hook_health_event_survives_the_write_read_round_trip` | started / heartbeat / dropped / first-drop の往復 |
| `hook_health_events_missing_their_material_are_rejected` | 12 通り（target / hook / reason の欠落・文法外、未知 kind） |
| `a_hook_health_snapshot_round_trips_and_rejects_an_impossible_drop_summary` | drop 件数と最新理由が矛盾する snapshot の拒否 |
| `a_named_record_and_a_source_baseline_survive_the_intent_snapshot_round_trip` | 記録名・採取済みソース一覧の往復と文法外拒否 |
| `a_greenfield_adjusted_stage_must_have_been_folded_to_skip` | `greenfield_adjusted` かつ EXECUTE の行は `InvariantViolation`、SKIP なら調整済みとして復元 |

## 3. 検査結果

- `cargo test -p core-command-interface-adapter`: 339 passed / 0 failed（`02-target-tests.log`）。
- `cargo fmt -p core-command-interface-adapter --check`: 成功。`cargo fmt --all --check` は他担当が編集中の `modules/app/aidlc/src/validation_basis.rs` / `tests/refusal_paths_contract.rs` で失敗する（所有外。`04-fmt-clippy-lint.log` に注記）。
- `cargo clippy -p core-command-interface-adapter --all-targets -- -D warnings`: 成功。
- `cargo lint`: 成功。

## 4. dead code 候補（プロダクトコードは削除していない）

| 箇所 | 根拠 |
| --- | --- |
| `plan_approval_runtime_repository_impl.rs` L99 `foreign approval snapshot`、L132-135 `foreign approval event`、L147-150 `foreign approval write` | `PlanApprovalRuntimeId` は `Workspace` の単一変種（`modules/core/command/domain/src/orchestration/plan_approval_runtime_id.rs`）。`base.id() != id` / `event.aggregate_id() != id` は型上 false にしかならない |
| `plan_approval_runtime_repository_impl.rs` L33、`intent_repository_impl.rs` L139、`intent_execution_repository_impl.rs` L167、`workflow_definition_repository_impl.rs` L162（`open` の `_ => ErrorKind::Other`） | 本家 `EventStoreForSqlite::new` は `Connection::open` の失敗（`rusqlite::Error::SqliteFailure`）を `IOError` に写す（`event_store_for_sqlite.rs` `map_write_error`）ので、`IOError` 以外は生成されない |
| `plan_approval_runtime_repository_impl.rs` L186-189（`store` の `OtherError` 分岐） | 本家 SQLite backend は `OtherError` を rusqlite の型変換失敗（API 誤用）でしか返さない。実 SQLite では到達不能。テストダブルを禁じる本 brief の方針では未検証 |
| `intent_repository_impl.rs` L316-319、`intent_execution_repository_impl.rs` L407-410、`workflow_definition_repository_impl.rs` L396-399（差分ループ内 `checked_add` 失敗の `SequenceGap`） | 直前行の `base.seq_nr() + 1` が `usize::MAX` で先に panic するため到達不能（`hook_health` 等の同種の `checked_add` は snapshot の `seq_nr = usize::MAX` で到達でき、今回カバーした） |
| `session_audit_repository_impl.rs` / `workflow_continuation_repository_impl.rs` の Conflict 経路内 `map_err(|error| self.read_error(error))`（競合直後の再読取失敗） | 実 SQLite では競合検出と再読取が同一接続で連続するため、間で媒体を壊す手段がない |
| `dto/pipeline_record_dto.rs` L21/27/32/38、`dto/plan_answer_dto.rs` L31/35/44-45、`dto/plan_generation_dto.rs` L21/33-34、`dto/plan_receipt_dto.rs` L40、`dto/session_audit_record_dto.rs` L28/36、`dto/workflow_continuation_event_dto.rs` L80/87、`dto/dto_vocabulary.rs` L260/285/298 | 対象一覧外。`fold_left` / `map_err` の閉包行や `Option` 分岐で、上流のテストが踏む行（workspace 基準ではカバー済み） |
| `compiled_definition_repository_impl.rs` L1305/1357、`intent_repository_impl.rs` L536/569、`intent_execution_repository_impl.rs` L722、`workflow_definition_repository_impl.rs` L603/796 | `#[cfg(test)]` 内の `panic!` / `matches!` の失敗側。テストコードが計測に混入している行で、プロダクトの未カバーではない |

## 5. 残った未カバー行と理由

- 対象 28 ファイルの残り 128 行のうち、上記 dead code / 到達不能 / テストコード混入が約 60 行。残りは llvm-cov のジェネリック実体化（memory / SQLite の 2 実体）に伴う差分と、`workflow_definition_repository_impl.rs`・`compiled_definition_repository_impl.rs`・`intent_*_repository_impl.rs`（対象一覧では各 7〜17 行）で、この 4 ファイルは他担当の統合テストが踏む行が大半のため、workspace 計測では brief の値より小さく出る見込み。
- `dto/review_record_dto.rs`（30/30 未カバー、対象一覧外）は RMU 側のテストが踏むため手を付けていない。

## 6. 補足（作業上の注意）

- 所有 crate 外のファイルは編集していない。ただし作業途中で一度 `cargo fmt --all` を実行した（13:37 UTC+9）。同時刻に他担当が編集中だった `modules/app/aidlc/`・`modules/core/command/domain/tests/` のファイルが整形された可能性がある（以後は `cargo fmt -p core-command-interface-adapter` に限定）。
- 共有 `target/` に対して `cargo llvm-cov clean --workspace` を 2 回実行した。並行して計測中だった他担当の profraw を消した可能性がある（親の最終計測は `coverage.sh` が自前で clean するので影響しない）。
- `session_audit_repository_impl.rs` と `plan_approval_runtime_repository_impl.rs` の末尾に `#[cfg(test)] #[path = "..._tests.rs"] mod tests;`（3 行）を追加した。既存の `hook_health` / `artifact_audit` / `workflow_continuation` と同じ crate 規約で、テストビルド以外のバイトは変わらない。`dto/tests.rs` 末尾に `#[path = "decode_contract_tests.rs"] mod decode_contract_tests;` を追加した。
