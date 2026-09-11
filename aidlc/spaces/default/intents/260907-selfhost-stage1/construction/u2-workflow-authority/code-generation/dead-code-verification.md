# dead code 候補の裁定と削除の検証（U2 Step 9、担当 `u2_dead_code`）

2026-09-11。裁定は [coverage-round2-questions.md](coverage-round2-questions.md) Q3 = A（「型・不変条件により到達不能」と根拠づけられたものは削除。`${PWD}` 単独分岐は本家と突き合わせてから決める）。5 担当の報告書（`coverage-logs/u2_cov_*-verification.md`）の候補を 1 件ずつ再確認し、振る舞いを変えない refactor として削除した。ログは [dead-code-logs/](dead-code-logs/)。他担当が編集中の `compiled_definition_repository_impl.rs` / `pipeline_link_error.rs` には触れていない。

## 判定の基準

- **削除**: 到達不能が「型」（変種数・`Option`/`Result` の構造）または「同一関数・同一構造体内の不変条件」で言え、かつ `unwrap` / `expect` / `indexing_slicing` / `panic` / `unreachable`（いずれも `Cargo.toml` で deny）を使わずに分岐そのものを消せるもの。
- **残置**: (a) 関数の型（`Result` / 任意の `String` / 任意の `Path`）では到達可能で、呼出し順序という外側の前提でだけ到達しないもの、(b) 本家（ピン `a277af21`）に同じ分岐があるもの、(c) 削除に新 API（無謬コンストラクタ・反復子）の追加や lint 設定の変更が要るもの、(d) 将来の変種追加で到達しうる網羅 `match` の腕や単一変種ゆえの比較。

## 検証結果（テスト・静的検査）

| 検査 | ログ | 結果 |
| --- | --- | --- |
| 削除前 `cargo test --workspace` | [01-before-cargo-test-workspace.log](dead-code-logs/01-before-cargo-test-workspace.log) | 3514 passed / 0 failed |
| 削除後 `cargo clippy --workspace --all-targets -- -D warnings` | [02-after-clippy.log](dead-code-logs/02-after-clippy.log) | exit 0 |
| 削除後 `cargo fmt --all --check` | [03-after-fmt.log](dead-code-logs/03-after-fmt.log) | exit 1 — 差分は他担当 `u2_keywords_display` が編集中の `modules/core/command/domain/tests/rejection_material_contract.rs` のみ |
| 削除後 `rustfmt --check`（本作業の変更 12 ファイル） | [03b-after-fmt-own-files.log](dead-code-logs/03b-after-fmt-own-files.log) | exit 0 |
| 削除後 `cargo lint` | [04-after-cargo-lint.log](dead-code-logs/04-after-cargo-lint.log) | exit 0 |
| 削除後 `cargo test --workspace` | [05-after-cargo-test-workspace.log](dead-code-logs/05-after-cargo-test-workspace.log) | 3517 passed / 0 failed（増分は他担当が並行追加したテスト。本作業は `wording::resume_menu` の単体テスト 1 件を関数と一緒に除去） |

## `${PWD}` 単独分岐の本家突き合わせ（`shell_write_targets.rs:485`）

- 本家: `vendor/aidlc-workflows` の固定コミット `a277af21`、`dist/claude/.claude/hooks/review-freeze-command.ts` `normalizeShellTarget`（582-595 行）。
  ```ts
  let cleaned = target.replace(/^of=/, "").replace(/^[,:[\]{}()]+|[,:[\]{}()]+$/g, "");
  if (cleaned === "$PWD" || cleaned === bracedPwd) { cleaned = cwd; }
  else if (cleaned.startsWith("$PWD/")) { ... }
  else if (cleaned.startsWith(`${bracedPwd}/`)) { ... }
  if (cleaned.length === 0 || /[$`*?]/.test(cleaned)) return "";
  ```
- 入力 `${PWD}` 単独: 本家は先に末尾の `}` を剥がして `${PWD` になり、`cleaned === "${PWD}"` は偽、`$` を含むので `""`（宛先なし）。本 build の `normalize` も `trim_matches` で同じ順に剥がし `${PWD` になり、`BRACED_PWD` と一致せず `$` を含むので `None`（宛先なし）。**両出力は一致**（宛先なし）。`${PWD}/x` は末尾が `x` なので剥がれず、両者とも `cwd/x` に解決される（既存テストが固定）。
- 判定: 本家でも本 build でも到達しない分岐であり、観測が一致するので Q3 の指示どおり削除した（`|| trimmed == BRACED_PWD` と未使用になる `const BRACED_PWD` を除去、根拠をコメントに残した）。上流不一致ではない。

## 候補ごとの裁定

### harness（`u2_cov_harness_query` 8 件）

| 候補 | 裁定 | 削除行数（概算） | 理由 |
| --- | --- | ---: | --- |
| `harness/infrastructure/src/shell_write_targets.rs:485` `${PWD}` 単独 | 削除 | 2 | 上記の本家突き合わせで観測一致 |
| `harness/infrastructure/src/reviewer_scope_segments.rs:29,73` `let Some(&c) = characters.get(index) else { break }` | 削除 | 6 | `while index < len` と `get(index)` は同値。`while let Some(&character) = characters.get(index)` に書き換え、else 腕を消した |
| `harness/infrastructure/src/shell_text.rs:135` `expression(...)?` の失敗経路 | 残置 | 0 | `Regex::new` の型は `Result`。消すには `expect`（deny）か `ShellParseError::Pattern` 変種の廃止が要る（理由 (c)） |
| `harness/claude/src/delegated_lifecycle.rs:32` `segments.at(index)` の `None` | 残置 | 0 | `Collection` は設計上反復子を持たず `at` は `Option` を返す。ループ本体に `return`/`continue` が複数あり `fold_left` へ移せない。消すには FCC への反復子追加が要る（理由 (c)） |
| `harness/claude/src/dispatch_rules_envelope.rs:233,245,249,257` `apply` の失敗経路 | 残置 | 0 | `apply(input: &mut ObjectMembers, slot)` の型では `items`/`stages` 不在は表現可能で、到達不能は呼出し元 `brief_slots` との組合せの不変条件（理由 (a)）。消すには `BriefSlot` の再設計が要る |
| `harness/claude/src/runtime_compile_envelope.rs:229` `words.first()` の `None` | 削除 | 5 | 唯一の呼出し元が 2 要素の配列を渡す。先頭語を別引数 `first: &str` で受ける署名に変え、空列を型で作れなくした |
| `core/infrastructure/src/append_only.rs:51-54` `bytes.get(written..)` の `None` | 削除 | 7 | `while let Some(remaining) = bytes.get(written..).filter(|rest| !rest.is_empty())` に書き換え（`written < len` と同値）、`InvalidInput` の else 腕を消した |
| `core/infrastructure/src/secret_file.rs:187` `file_name()` 無しの既定 `"secret"` | 残置 | 0 | `PathBuf` の型では `..`/根が表現可能で、到達不能は `read()` が先に走るという呼出し順序の前提（理由 (a)） |

### RMU（`u2_cov_rmu` 6 箇所）

| 候補 | 裁定 | 削除行数 | 理由 |
| --- | --- | ---: | --- |
| `read_tables/execution_row.rs:59,61,63` `ContinuationWait::Question/Conversation/Resume` | 残置 | 0 | ドメイン enum の網羅 `match` の腕。変種が存在する限り消せない（理由 (d)、報告書も同旨） |
| `orchestration/workflow_continuation_read_model_updater.rs:202-204` `WorkflowContinuation::new` の失敗 | 残置 | 0 | `new` の型は `Result`（理由 (c)） |
| `orchestration/hook_health_reader.rs:201-209` `i64::try_from(usize)` 失敗 | 残置 | 0 | 型は `Result`。64bit 環境依存の前提で消すには `expect` が要る（理由 (c)） |
| `orchestration/plan_approval_files.rs:66` 受領ファイル名の成分検査 | 残置 | 0 | `file.name()` は `&str` で型では任意。ファイルシステム境界の入力検査（construction 規則「境界で入力を検証する」）でもある（理由 (a)） |
| `read_tables/jump_result_row.rs:43-44,48-51,118-119` `MissingGenesis` | 残置 | 0 | `replay_executions` が先に拒むという呼出し順序の前提。`remove`/`resolve_jump_target`/`at` の型は `Option`/`Result`（理由 (a)(c)） |
| `workspace/projection.rs:1246` 後方跳躍で観測 `None` の既定腕 | 削除 | 3 | `if direction == Backward && observation().is_some()` + `map_or_else` を `if let Some(observation) = jumped.observation().filter(|_| direction == Backward)` に書き換え、既定腕を消した |

### command interface-adapter（`u2_cov_cmd_ia` §4）

| 候補 | 裁定 | 削除行数 | 理由 |
| --- | --- | ---: | --- |
| `plan_approval_runtime_repository_impl.rs` L99/L132-135/L147-150 `foreign approval …`（`PlanApprovalRuntimeId` 単一変種） | 残置 | 0 | 今日は型上 false だが、変種追加で即座に到達する集約横断の不変条件検査であり、姉妹リポジトリ（`intent_*`/`workflow_definition_*`）と同型の防御（brief 手順 1 の「将来の変種追加で到達しうる」＝理由 (d)）。削除は人間の裁定事項として残す |
| `open` の `_ => ErrorKind::Other`（4 ファイル） | 残置 | 0 | 到達不能の根拠は外部クレート `EventStoreForSqlite::new` の実装挙動であり型ではない（理由 (a)）。`EventStoreWriteError` は他変種を持つ |
| `plan_approval_runtime_repository_impl.rs` L186-189 `OtherError` 分岐 | 残置 | 0 | 外部クレートの enum に変種が存在し、網羅 `match` の腕（理由 (d)） |
| 差分ループ内 `checked_add` 失敗（3 ファイル） | 残置 | 0 | 「直前の `+ 1` が先に panic」は debug ビルドの挙動。release（`overflow-checks` 既定 off）では wrap するため型・不変条件で到達不能とは言えない |
| Conflict 経路内の再読取 `map_err` | 残置 | 0 | I/O の `Result`（理由 (c)） |
| DTO の閉包行・`Option` 分岐 | 対象外 | 0 | 報告書自身が「workspace 基準ではカバー済み」としており dead code ではない |
| `#[cfg(test)]` 内の `panic!`/`matches!` 失敗側 | 対象外 | 0 | テストコード |

### domain / use-case（`u2_cov_domain_uc`）

| 候補 | 裁定 | 削除行数 | 理由 |
| --- | --- | ---: | --- |
| `intent_execution.rs:1336-1340` `plan_fingerprint` の `embedded.ok_or_else` | 削除 | 5 | ガードを `let Some(contract) = embedded.as_ref().filter(|c| c.is_current(&posture)) else { … }` に書き換え（条件は `is_none_or(!is_current)` の否定と同値、拒否文言は不変）。`ok_or_else` を消した |
| `intent_execution.rs:2021-2022` `recompose` の「Execute でも Skip でもない」拒否 | 削除 | 4 | `effective_plan` は `slots.at(stage)` で、範囲内（`stage < stage_count() == slots.len()`）では必ず `Some`、`PlanAction` は `Execute`/`Skip` の 2 変種。範囲外は直前で拒否済みなので `toward_execute = flips.divide(&toward_skip)` とし、空集合の検査を消した |
| `plan_approval_runtime.rs:233-242` `record_answer` の `ok_or_else` 2 箇所 | 削除 | 10（書換後 +12） | `valid` 判定と `Some` の取り出しを `and_then`/`filter`/`map` で 1 度にまとめ、`let Some((offered, response)) = matched else { Err(同文言) }` にした。判定条件・拒否文言は同じ |
| `plan_approval_runtime.rs:518-519` `resolve_invalidation` の `UnknownInvalidation` | 削除 | 3 | 私有関数で唯一の呼出し元 `resolve_for_publication`（427 行）が `invalidations` の存在を先に検査する。検査を消し、戻り値を `PlanApprovalEvent` に変えた（`unnecessary_wraps` 対応）。呼出し元で `Ok(...)` に包む |
| `testing_posture/classification.rs:87-88` 構造化 Methodology の `components` 追加 | 残置 | 0 | 本家 `aidlc-testing-posture.ts:524` `if (methodology !== "custom") components.add(methodology)` と同じ分岐（理由 (b)） |
| `testing_posture/classification.rs:129-131` `default_ordering("custom")` | 残置 | 0 | 本家 `defaultOrdering` の `custom` 腕（同 248 行）と同じ（理由 (b)）。`match` の網羅腕でもある |
| `testing_posture/classification.rs:116-118` 正規表現コンパイル失敗 | 残置 | 0 | `Regex::new` の型は `Result`（理由 (c)） |
| `use-case/src/orchestration/record_single_stage_run_use_case.rs:152-160` `map_err` | 残置 | 0 | 集約の `record_single_stage_run` の型は `Result<_, SingleStageRunRefusal>` で、UseCase 側の写しは型が要求する変換。`require_pipeline_single` とは別メソッドの拒否であり、消すには `unwrap` が要る（理由 (a)(c)） |
| `summary_questions.rs:125-127` `missing required H2 section` | 残置 | 0 | `confirmed_lines(lines, visible)` の型では summary 無しの入力を作れる。到達不能は呼出し元 `parse` の順序の前提であり、本家 `aidlc-lib.ts:6872` にも同じ文言の検査がある（理由 (a)(b)） |

### app（`u2_cov_app` §4）

| 候補 | 裁定 | 削除行数 | 理由 |
| --- | --- | ---: | --- |
| `turn.rs:334-338` 分岐 6 `resume-menu` の Ask、`wording::resume_menu`（+ 単体テスト 1 件） | 削除 | 10 + 8（+ テスト 9） | `NextDecision::ResumeMenu` を構築する箇所は 0 件（`grep -rn ResumeMenu modules/` は `match` 腕・変種定義・テストのみ）。RMU の `read_tables_test.rs:916` が「集約は `resume-menu` を答えない」を固定している。分岐の場所に理由コメントを残した。`NextDecision::ResumeMenu` / `AskKind::ResumeMenu` の変種自体は 3 crate にまたがる契約なので消していない（後述） |
| `runtime.rs:1250-1259` `layout_for_artifact_only` の `record_dir().is_some()` 枝 | 削除 | 10 | `Layout::state_file()` は同じ構造体の `record_dir.as_ref().map(...)` なので、`state_file().is_some()` で返った後に `record_dir()` が `Some` になることはない（同一構造体内の不変条件） |
| `runtime.rs:898-942` `committed_directive` の `invalid()` 腕 | 残置 | 0 | 投影 `report_result` の `steps`/`no_op_reason` は読み面の `String` で、型では閉集合外を表現できる。読み面の入力検査（理由 (a)） |
| `runtime/pipeline_link.rs:47` `record_dir()` の `None` | 削除 | 3 | 直前の `state_file()` 存在検査と同じ `record_dir` から導くので、`let Some(record) = layout.record_dir().filter(|_| state_file().is_some_and(exists)) else { 同じ拒否文言 }` に 1 本化。文言 `"Cannot record reverse-engineering developer link: active intent record is unavailable."` は本 build の他所・テスト・ゴールデンから参照されていない（`grep -rn` 0 件） |
| `runtime/review_guards.rs:52` `record_dir()` の `None` | 削除 | 3 | `audit_dir()` も `record_dir` から導く。`let Some(record) = layout.record_dir().filter(|_| audit_dir().is_some_and(has_shard)) else { silent }` に 1 本化（どちらも `silent` なので観測不変） |
| `runtime/pipeline_link.rs:133` `relative` の成分検査 | 残置 | 0 | `observe(project, path, relative: &str)` の型では任意の綴りを受け、`current()` からも呼ばれる。ファイルシステム境界の入力検査（理由 (a)） |
| `runtime/session_start.rs:68`、`runtime/session_hooks.rs:39,145,219` 固定リテラルの `AuditFieldKey::parse` 失敗 | 残置 | 0 | `parse` の型は `Result` で無謬コンストラクタが無い。消すには `AuditFieldKey` への新 API 追加が要る（理由 (c)）。RMU 側の `key(key::DETAILS)?` も同型 |
| `turn.rs:713,725` `scope_row`/`stock_examples` の失敗 | 残置 | 0 | 別クエリの SQLite I/O 失敗経路で、型は `Result`。「直前の同表クエリが成功した」は実行時の前提（理由 (a)(c)） |
| `cli/request.rs`、`runtime.rs:3718…`、`presenter.rs` のテストヘルパ `panic!` 腕 | 対象外 | 0 | テストコード |

## 集計

- 削除（プロダクトコード）: 13 箇所、除去した到達不能行は概算 71 行（書換で置いた行を差し引いた純減は約 45 行）。加えてテスト 9 行（`wording::resume_menu` の単体テスト）。
- 残置: 24 箇所（理由は各表）。
- 変更ファイル（12）: `modules/harness/infrastructure/src/shell_write_targets.rs`、`modules/harness/infrastructure/src/reviewer_scope_segments.rs`、`modules/harness/claude/src/runtime_compile_envelope.rs`、`modules/core/infrastructure/src/append_only.rs`、`modules/core/read-model-updater/src/workspace/projection.rs`、`modules/core/command/domain/src/orchestration/intent_execution.rs`、`modules/core/command/domain/src/orchestration/plan_approval_runtime.rs`、`modules/app/aidlc/src/turn.rs`、`modules/app/aidlc/src/wording.rs`、`modules/app/aidlc/src/runtime.rs`、`modules/app/aidlc/src/runtime/pipeline_link.rs`、`modules/app/aidlc/src/runtime/review_guards.rs`。

## 親へ渡す裁定事項（実装は変えていない）

1. `NextDecision::ResumeMenu`（domain）/ `AskKind::ResumeMenu`（query use-case）の変種は構築箇所 0 件のまま残る。変種の廃止は `engine_signal.rs`・`spelling.rs`・`next_answer_row.rs`・`presenter.rs`・query `directive.rs` にまたがる契約変更で、本家の分岐表との対応（分岐 6）をどう記録するかを含めて人間の裁定が要る。
2. `PlanApprovalRuntimeId` 単一変種ゆえの `foreign approval …` 3 検査（command interface-adapter）は、型上は今日到達不能だが変種追加で到達する防御であり、削除するかは裁定を求める。
3. 固定リテラルの `AuditFieldKey::parse` 失敗腕（app 4 箇所、RMU の `key(...)?` も同型）は、無謬コンストラクタを `AuditFieldKey` に足せば構造的に消せる。公開 API 追加なので別件とした。
