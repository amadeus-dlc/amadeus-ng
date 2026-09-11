# u2_cov2_app 検証報告 — `aidlc` crate（`modules/app/aidlc/`）のカバレッジ向上（2 回目）

担当: `u2_cov2_app`。所有 crate は `aidlc` のみ。**プロダクトコードは変更していない**（テスト追加のみ。`src` 側の変更は `#[cfg(test)]` モジュール 3 か所への追記だけ）。

## 1. 計測条件

- コマンド: `CARGO_TARGET_DIR=target/u2_cov2_app cargo llvm-cov --no-report -p aidlc --tests --no-fail-fast` → `cargo llvm-cov report --json --ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$'`、`PROPTEST_RNG_SEED=20260823`。担当専用の target ディレクトリを使い、共有 target には `clean` を掛けていない。閾値・除外・seed は `scripts/coverage.sh` と同じで変更していない。
- 数値は llvm-cov の JSON summary（`files[].summary.lines`）。1 回目の報告どおり summary は**関数（クロージャを含む）ごと**に行を数えるので、`report --text` の count=0 行だけでは残りが見えない。今回は JSON の `functions[].regions` から関数単位で未実行行を復元するスクリプト（scratch 内 `zfn.py`。crate ハッシュの違うインスタンス化を名前で統合し、行ごとに最大 count を取る）で対象行を特定した。復元値は summary と数行の差で一致する（着手前 600 / 633）。
- `#[cfg(test)]` 内のテストコードも分母に入る。**テスト内の「失敗時だけ評価される行」（複数行に割った `assert!` の文言引数、到達しない `match` 腕）も未カバーに数えられる**ので、`src` 内テストは 1 行 `assert!` と Debug 表示の包含検査に揃えた（`validation_basis.rs` で 1 度踏んで直した）。`tests/` 配下のファイルは分母に入らない。

## 2. 着手前後の行カバレッジ

| 対象 | 着手前 | 着地後 |
| --- | --- | --- |
| `aidlc` crate 全体（`main.rs` 除外） | 16,205 / 16,838 = **96.24 %**（未カバー 633） | 16,389 / 16,910 = **96.92 %**（未カバー 521） |
| 対象ファイル（[targets2-app.md](targets2-app.md)）の未カバー合計 | 633 | **521**（−112、17.7 % 減） |

**目標（300 行以下）には届いていない。** テストは crate 全体で 928 件成功・0 失敗（[02-after-measure.log](u2_cov2_app/02-after-measure.log)、llvm-cov 計測下の `--no-fail-fast`）。`cargo fmt -p aidlc -- --check`・`cargo clippy -p aidlc --all-targets -- -D warnings`・`cargo lint` はいずれも成功（[04](u2_cov2_app/04-fmt.log) / [05](u2_cov2_app/05-clippy.log) / [03](u2_cov2_app/03-lint.log)）。新規テスト 40 件だけの実行は [06-new-tests.log](u2_cov2_app/06-new-tests.log)。

届かなかった理由は §5 に書く。要点は、残り 521 行のうち約 7 割が「`IntentExecutionRepositoryImpl::open` などが**成功した後**に個別ユースケースだけを失敗させないと呼ばれない 1 行クロージャ」か「到達不能の防御腕」であり、**プロダクトコードに手を入れず、公開 API も足さない**という制約の下では踏めないことである。

### ファイル別（着手前 → 着地後）

| ファイル | 前 | 後 | ファイル | 前 | 後 |
| --- | ---: | ---: | --- | ---: | ---: |
| `runtime.rs` | 152 | 126 | `runtime/session_hooks.rs` | 14 | 10 |
| `runtime/continuation.rs` | 60 | 53 | `runtime/runtime_graph.rs` | 9 | 6 |
| `runtime/plan_approval.rs` | 68 | 48 | `summary_questions_input.rs` | 6 | 4 |
| `turn.rs` | 56 | 47 | `validation_basis.rs` | 6 | 4 |
| `runtime/pipeline_link.rs` | 32 | 31 | `runtime/task_sync.rs` | 4 | 2 |
| `runtime/learnings.rs` | 25 | 17 | `runtime/log_failure.rs` | 2 | 1 |
| `runtime/testing_posture.rs` | 23 | 17 | `lexical_path.rs` | 4 | 0 |
| `source_baseline.rs` | 22 | 17 | `session_processes.rs` | 17 | 17 |
| `runtime/jump.rs` | 19 | 15 | `source_fingerprint.rs` | 8 | 8 |
| `runtime/review_documents.rs` | 20 | 13 | `runtime/review_guards.rs` | 18 | 17 |

（上記以外の 26 ファイルは 1〜7 行で変化なし。）

## 3. 追加したテスト（合計 40 件）

### 3.1 `tests/refusal_paths_contract.rs` への追記（34 件、末尾の「2 回目（u2_cov2_app）」節）

1 回目の `Workspace`（配布定義付き一時ワークスペース、`aidlc::runtime::run` の同一プロセス呼出し、stdin を要するフックの子プロセス起動）を再利用し、失敗注入の道具を足した: `spawn`（起動名を `bin/<argv0>` に複製して環境変数付きで子プロセス起動）、`hook_with_directory_stdin`（stdin をディレクトリにする）、`corrupt_store`（空間ストアを SQLite でないバイト列にする）、`break_cursor`、`hook_drops`、`runtime_db` / `store_db`（`rusqlite` で引き金や列改名を仕込む）。

| 領域 | 件数 | 検証した契約 |
| --- | ---: | --- |
| `write-audit-log` | 3 | 台帳がファイル／シャード無し／存在しない成果物／追記を拒む引き金（`conflict`）／壊れたストアの各原因を drop に残して通す。状態ファイル無しではカーソルが要り、無い・ディレクトリ・文法外名は space 段の drop に残す。stdin が読めないフックは沈黙。 |
| `record-human-turn` | 4 | 共有承認ロックがディレクトリでも人間ターンの監査は空間ストアへ残す。壊れたカーソル・記録無し・カーソル無し・引き金で拒まれた観測・壊れたストアでは何も書かず通す。 |
| `aidlc-log decision` | 1 | `--rationale` が監査へ同伴する。引き金で拒まれた追記は `Audit emission failed: repository: …` で断る。 |
| `aidlc-jump` | 1 | `execute --target` の文法外は `Unknown stage`、`resolve --stage` の未知も `Unknown stage`、壊れたストアは `io: InvalidData at <store>`。 |
| ソース基準の予算 | 1 | `.git` のある根で `AIDLC_TEST_SOURCE_MAX_{DIRECTORIES,ENTRIES,SYMLINKS}` を超えると、鋳造・jump・report の基準は失敗ではなく **`unbindable`** になり snapshot を書かない（1 回の jump は監査に基準を 2 度描く）。予算内では snapshot を残す。 |
| `aidlc-testing-posture` | 1 | `--unit "Bad Unit!"` の逐語、壊れたカーソルでの `render`/`fingerprint`/`begin` の拒否、壊れたストアの `io: InvalidData`。 |
| `aidlc-learnings` | 2 | 選択ファイルがディレクトリ／`stage_slug` 無し／文法外／`selections` 無しの拒否、`.aidlc-learnings.lock` がディレクトリなら `persist failed`、memory 層が読めなければ投影復元が `PermissionDenied` で先に断る。`runtime-graph.json` が `{}`／ディレクトリ、状態ファイルがディレクトリ（投影復元の `publication conflict`）。 |
| summary-confirmation | 1 | `--questions-file` 無しの逐語、質問ファイルがディレクトリ。 |
| 共有承認ストアの保留行 | 2 | 投影が描き直した直後に壊す引き金で、`kind` = response／answer／未知、`space` 無し／文法外、`execution_id` 無し／文法外、`operation_id` 文法外の 8 欠陥を**それぞれの逐語**で断る（読み面を推測で埋めない）。`.aidlc-runtime.lock` がディレクトリなら `Is a directory`、別ハンドルが握っていれば 5 秒で `exclusive file lock timed out`。 |
| `continue-workflow`（Stop） | 6 | ロックがディレクトリ（resume 待ち読取と停止要求保存の両方）・別プロセスが握る（1 秒）・空間ストアが壊れた・状態ファイルがディレクトリ・文法外 space で `next` が終了 1・runtime ストアが壊れた、のいずれも終了 0 で沈黙し、可能なら drop に原因を残す。共有 resume 待ちの印があってもカーソルが壊れていれば通す。drop の保存自体が拒まれても止めない。 |
| `validate-state`（PreCompact） | 2 | breadcrumb がディレクトリなら終了 1、監査追記の引き金失敗と壊れたストアは drop、clone id 無しでは当該シャードに何も書かない。 |
| `sync-workflow-state` | 1 | 壊れた初期化印は稼働記録の拒否をそのまま返す、カーソル無しは沈黙。 |
| 投影前提の破壊 | 4 | `.aidlc-clone-id` がディレクトリなら decision／set-autonomy／review／learnings persist／report がすべて `clone id: …` で断る。report は `stage-graph.json` 無し・非配列・slug 無し・文法外 slug を基準採取で断る。`aidlc/spaces/default` がファイルなら `next`・鋳造とも `cannot create the record directory`。壊れたストアは report／set-autonomy が `journal: io: InvalidData` で断る。 |
| plan-approval の前提 | 1 | decision／answer の `--unit` 文法外、計画文書の無い記録、壊れたカーソルを同じ逐語で断る。 |
| 壊れた読み面 | 2 | `read_intent`／`read_scope_change`／`read_pipeline_progress`／`read_steering_plan`／`read_run_stage`／`read_next_answer`／`read_execution` の列を改名すると `next` は `Read model not readable at <store>: …` で断り、戻せば通る。`read_definition_stage` の行を消せば `stage metadata unavailable`。jump resolve（`read_next_jump`／`read_next_jump_phase`）と testing-posture render（`read_testing_contract`）も同じく断る。 |
| pipeline link / review-freeze / rebuild-stage-graph | 3 | 権限の無い handoff は `handoff file must be a regular file … (Permission denied…)`。文法外 space の review-freeze は判断を下さず通す。rebuild-stage-graph は壊れたカーソル・壊れたストアを drop に残し、既存の `runtime-graph.json` を変えない。 |

### 3.2 `tests/upstream_271_contract.rs` への追記（2 件、末尾の「u2_cov2_app」節）

既存の `workspace_with_plan_questions()`（code-generation 段まで進めた記録）を再利用。

| テスト | 検証した契約 |
| --- | --- |
| `a_plan_decision_carries_its_rationale_and_begin_is_refused_before_approval` | 提示前・提示だけの `begin` は `Plan Approval is not explicitly answered Approve Plan`、`--rationale` は監査へ同伴、文字を持たない `--session "!!!"` は decision／answer とも `Plan Approval challenge requires a nonblank session`。 |
| `a_plan_answer_needs_a_readable_questions_file_at_the_supplied_path` | 質問文書がディレクトリなら `steering read: IsADirectory at …` で断り、受領証を出さない。 |

### 3.3 `src` 内の単体テスト（4 件）

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `lexical_path.rs` | 1 | `.` は落とし `..` は直前を戻す字句解決（ファイルを開かない）。 |
| `runtime/review_documents.rs` | 2 | 権限の無い成果物・途中成分がファイル・壊れた／無い定義グラフは束ねない。`workspace_requires` の段でソース木が読めなければ `unbindable` として束ねる。 |
| `validation_basis.rs` | 1 | 定義グラフ無し／非配列は警告になり、状態の無い配置は何も返さない。 |

## 4. dead code 候補（今回新たに根拠を得たもの。プロダクトは変更していない）

| 箇所 | 根拠 |
| --- | --- |
| `runtime/pipeline_link.rs:104, 218, 219` の `repo` 分岐・クロージャ | `pipeline_history.rs:200-205` は非空の `repo` を**常に** `UnregisteredRepo` で拒むので、`repo` 付きの `Duplicate`／`OutOfOrder`／成功出力（`"repo"` 欄）に到達しない。 |
| `runtime/plan_approval.rs:52` | 直前の `prepare_shared_store`（`:611-613`）が同じ条件（Ready かつストア無し）を先に拒む。 |
| `runtime.rs:316`（`records_exist_without_cursor` の `record_dir().is_some()` 枝） | 唯一の呼出し `:335` が `record_dir().is_none() &&` で守っている。 |
| `runtime.rs:2611, 2613, 2738, 2740, 2959, 2968`（`definition_id` / `compiled_definition_id` の失敗写像） | `harness_name` は定数 `"claude"` で、両 `parse` は空・制御文字以外を受理する。 |
| `runtime.rs:1123` | `:1043-1061` の閉集合検査で未知フック名は先に断られ、`record-human-turn` 以外は上で分岐済み。 |
| `runtime/plan_approval.rs:139, 313, 404, 574`（`invalid_active_space` 写像） | `PlanApprovalAccess::open` に至る前に `store_path` / `observe_hook_health` が同じ文法検査で断る（`an_invalid_active_space_is_refused_before_any_store_is_touched`）。 |
| `runtime/learnings.rs:423-424, 428, 438` | `persist` は `catch_up_before_reading(&pinned)`（`:271`）を先に通すので、壊れたカーソル・開けないストアはそこで断られ `store()` に届かない（`learnings_persist_refuses_a_record_whose_cursor_is_lost_or_broken` の観測と一致）。 |
| `runtime/continuation.rs:147` | `next` が `ask` を返す状態は無い（1 回目報告の `resume-menu` 廃止）。 |
| `runtime/continuation.rs:232, 297, 361, 430, 478-480` | いずれも `record_dir()` が `None` の枝だが、`consider` は `state_file()`（record から導く）が `Some` のときしか進まない。 |
| `runtime/review_guards.rs:63, 148, 183` | `health_target` は `SpaceName::parse` が失敗したときだけ `None` だが、その前に `store_path` 等で同じ文法検査に落ちるか、freeze では record が無い。 |
| `runtime/session_hooks.rs:39, 145, 165, 219`、`runtime/session_start.rs:68` | `AuditFieldKey::parse` の引数は固定リテラル（1 回目報告のとおり）。 |
| `runtime/pipeline_link.rs:134, 175-178` | `relative` は `record.join(...)` を剥いだ綴りで `Normal` 以外を持たず、FIFO 等は `:154` の `is_file` で先に断られる。 |

## 5. 残った未カバー行（521）と理由

内訳（行番号は着地後。復元スクリプトの出力）:

1. **成功したストア open の後に個別ユースケースだけを失敗させないと呼ばれない 1 行クロージャ（約 250 行）** — `runtime.rs` の `publish_directive`（250-283）、`bind_state_text`（304-309）、`mint_intent`（2535-2670）、`prepare_definition_for_first_read`（2738-2757）、`log_answer` の runtime 投影（1424-1441, 1522-1561）、`plan_approval.rs` の `origin_layout`／`recover_*`／`record_*`／`publish`（142-419）、`continuation.rs` の `store_request`／`recover_publication`（400-469）、`jump.rs`（59-88, 103-210）、`testing_posture.rs`（29-178）、`pipeline_link.rs`（139-203, 266-280）、`turn.rs`（870-998）。空間ストアを壊す・引き金で追記を拒む・読み面の列を改名する、の 3 手で踏めるものは今回踏んだ。残りは「同じストアの隣の open は成功し、この 1 呼出しだけが失敗する」状況が要り、テスト容易化の API を足さない制約では作れない。
2. **§4 の dead code 候補（約 45 行）**。
3. **construction／unit 段まで進めた記録が要る経路（約 15 行）** — `continuation.rs:249-266`（`observe_questions` の phase／unit 検査）、`plan_approval.rs:336-348`（応答準備の `NoPendingChallenge` 以外の失敗）、`turn.rs:881-895, 956-960`（pipeline の実行 id 欠落・支援エージェント欠落）。1 回目「Next Steps」の候補だが、`workspace_at_code_generation()` は 33 回の report/approve を子プロセスで回す重い fixture で、unit 付き run-stage を出すにはさらに units-generation 相当の成果物が要る。今回は plan-approval の周辺（rationale／blank session／begin 前）だけを追加した。
4. **プラットフォーム条件（13 行）** — `session_processes.rs:23-32, 85`（Linux の `/proc`、`win32`）。macOS で計測したため未実行。
5. **競合・レース（約 12 行）** — `review_documents.rs:85-121`、`pipeline_link.rs:173-190`、`source_fingerprint.rs:93`、`source_baseline.rs:277, 281, 312, 316`（読取中の差替え検知）、`ledger_lock.rs:42-45`。決定的に再現できない。
6. **固定閾値の予算（約 10 行）** — `source_fingerprint.rs:42, 47, 83`（環境変数を読まない 100,000／250,000 の定数）、`source_baseline.rs:346`（25 万ファイル・4 GiB）。
7. **テストヘルパの `panic!` 腕（8 行）** — `cli/request.rs`、`runtime.rs:3709, 4197, 4223`、`presenter.rs`（`#[cfg(test)]` 内）。
8. **その他（約 30 行）** — `runtime.rs:571-574`（投影成功後に読み面が開けない）、`:549, 2029, 2245`（`catch_up_before_reading` が先に断るので「cannot open the event store」に届かない）、`:1893, 2060, 2083, 2137-2138, 2282, 2366, 2411`、`session_processes.rs:61, 120, 127, 194, 203`、`usage_ledger.rs` の Number 変種、`directive_drawing.rs`、`dispatch_rules.rs:265-271`（memory 外の規則パス。配布グラフには無い）。

## 6. 所見（プロダクト側。裁定を要するものは実装を変えていない）

- **`source_baseline::read` は採取の失敗を握り潰す**（`source_baseline.rs:27-32`）。`.git` の無い根では失敗が**空の一覧**（= 「ソース無し」）に、`.git` のある根では `unbindable` になる。予算超過・読取失敗が「ソースが無いワークスペース」と区別されないのは上流仕様との突合せが要る点として記す（今回は現行の振る舞いを契約として固定した）。
- **`observe_questions` の phase 判定**は状態ファイルの `Lifecycle Phase` を小文字化して固定 5 種と比べる（`continuation.rs:234-250`）。投影が書く値なので実運用では常に一致するが、手書き状態では沈黙する。
- 引き金（`RAISE(ABORT)`）で拒まれた追記は、リポジトリ層で `conflict: expected N, actual N` に写像される（`Audit emission failed: repository: conflict…`）。原因の文言（`injected …`）は失われる。楽観ロックの競合と保存層の失敗が同じ文言になる点は `error-handling.md`（境界での文言変換）の観点で確認対象になり得る。
- `learnings persist` で memory ファイルが読めない場合、`observe`（`:347`）より先に投影復元（`catch_up`）が `publication io: PermissionDenied` で断る。`learnings_persist_failed` の文言は届かない。
- 1 回目報告の所見（配布 scope の `keywords:` ブロック列）は `u2_keywords_display` が対応済みと聞いており、今回は触れていない。

## 7. ログ

`coverage-logs/u2_cov2_app/`: [01-before-measure.log](u2_cov2_app/01-before-measure.log)（着手前の crate 計測）、[02-after-measure.log](u2_cov2_app/02-after-measure.log)（着地後の crate 計測 = crate 全テスト 928 件成功、末尾に前後の summary と対象ファイル別の前後）、[03-lint.log](u2_cov2_app/03-lint.log)、[04-fmt.log](u2_cov2_app/04-fmt.log)、[05-clippy.log](u2_cov2_app/05-clippy.log)、[06-new-tests.log](u2_cov2_app/06-new-tests.log)（新規 40 件だけの実行）。
