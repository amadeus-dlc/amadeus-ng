# Step 8（旧受入参照・比較項目の 2.7.1 移行）の達成状況の棚卸し

2026-09-10、担当 `u2_step8_audit`。承認済み実装計画 Step 8 の 5 箇条を、現在の作業ツリー（branch `stage1`、HEAD `1dc727e0` に未コミットの変更を載せた状態）のコードとテストの実行結果で棚卸しした。製品コード・テスト・コーパス・正規化設定は 1 バイトも変えていない。実行ログは [step8-logs/](step8-logs/) にある。散っていた 4 本の移行記録（[cli-golden-migration.md](cli-golden-migration.md)、[infrastructure-golden-migration.md](infrastructure-golden-migration.md)、[distribution-field-migration.md](distribution-field-migration.md)、[rmu-golden-migration.md](rmu-golden-migration.md)）は 2026-09-09 時点の記録であり、本書はそれらを現在のソースで検証し直した結果である。

「テストが green」と「2.7.1 の全観測面と一致」は別物として扱う。下表の「満たした」は、対応する 2.7.1 の採取データと当該テストが実際に突き合わせている面についてだけ言う。

## 結論: Step 8 の 5 箇条

| # | 箇条（原文要約） | 判定 | 根拠の要約 |
| --- | --- | --- | --- |
| 1 | U1 の一覧と全数差分を使い、既存 CLI・配布 JSON・hash・状態/監査投影・補完の比較を 2.7.1 へ揃える | **一部** | 旧コーパスを読んでいた 6 テストファイルはすべて `tests/golden/upstream-a277af21/` を読む（§2）。配布 JSON はバイト一致、hash は 32 行 × 5 面、監査描画は 106 ブロック、補完は 3 ケースを 2.7.1 で検査（§3）。しかし CLI 28 ケースのうち native の出力を突き合わせるのは 8 ケースで、うち全文比較は 2、キー集合 2、slug 置換 5（重複含む）。状態 19 件のうち投影で突き合わせるのは 14、監査 18 件のうち 9（§3.2–3.3）。`bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21` は **失敗**（§6、R1） |
| 2 | キー集合だけの一致、固定 slug の読み替え、既知欠落を許す条件、駆動できないままの必要ケースを見直し、同じ前提で全観測面を比較する | **一部** | 既知欠落（`narration` / `conductor_persona`）を許す例外は削除済み（`cli_golden_test.rs:7`、`:258-261` で persona 全文一致）。キー集合比較は `next/start`（`:214-227`）と `continue/load-steering`（`:233-274`）に残る。slug 置換は report 5 ケース（`:295-305`、`:348-427`）に残る。同じ前提（33 ノードの配布グラフ）で全観測面を比較する CLI テストは classic scope には無く、bugfix scope では `upstream_271_contract.rs` が 2.7.1 実観測（`stage1/cases.json`、`selfhost-stage1/*.json`）と全文一致させている。駆動できないケースは 12 件（§4） |
| 3 | 不正 UTF-8 と ID 対応を保つ比較を Rust 側にも適用する。固定文言・監査語彙・識別子を正規化で消さない | **一部（概ね満たす）** | 不正 UTF-8: `an_invalid_utf8_stdin_still_records_an_anonymous_human_turn`（`upstream_271_contract.rs:3463-3560`）が stdin/stdout/stderr/追記監査を base64 のバイトで比較、`state_guard_ignores_invalid_utf8_stdin_without_initialization`（`claude_hook_contract.rs:313-331`）。ID 対応: `normalization.json` から `<SESSION>` の一律規則 2 本が消え、`normalization_replaces_only_declared_environment_values`（`golden_corpus_read.rs:323-370`）が未対応の session/token を保持することを検査。TS 側 `readBindings` は「識別子対応を潰す正規化」を拒否（`corpus-normalization.ts:29-32`）。Rust 側に `comparison.json` の役割対応を読む比較器は無く、各テストが 1:1 の置換（記録名、日付、slug）で済ませている（§5） |
| 4 | 旧 2.6.40 の値を現行互換の根拠として残さない。用途を区別し、一括置換で移行しない | **一部** | 旧コーパスを実行時に読む Rust は 0 件。残る `3c3146cf` 参照は §1 の表のとおり分類でき、入力リテラル 1 行と採取ガード 3 行は用途が明記されている。一方で「旧ピンが正本」と書く注記（`wording.rs:543-545`、`compiled_definition_repository_impl.rs:35,729`、RMU `wording.rs:6,10,21`）、削除済み「逸脱台帳」を根拠にする注記（`wording.rs:687-690` ほか 8 行）、2.7.1 コーパス内に残った旧ピンの説明文（`cli/set-autonomy/state-field-absent/case.json:6`）、旧コーパス本体 2 ディレクトリ（git 追跡 323 ファイル）と `coding-rules/README.md:5,25` の「正本は `upstream-3c3146cf`」記述が残る。legacy-differences の「README.md → U2 が整理」は未着手 |
| 5 | 差の意味が承認済み契約で決まらない場合は入力・両出力・該当ソースを提示して裁定を求める | **満たした（裁定済み 3 件）＋ 追加候補あり** | センサー監査 3 種の除外 = A（`tests/support/native_audit_scope.rs` で実装）、jump 直接 execute = A（[jump-verification.md](jump-verification.md)、[jump-foreign-scope-verification.md](jump-foreign-scope-verification.md)）、TaskUpdate 同期 = A（[accepted-contract-decisions.md](accepted-contract-decisions.md)）。本棚卸しで新たに裁定・是正が要る項目を §7 に列挙した（R1〜R10） |

## 1. 旧参照の残存（`rg -n 'upstream-3c3146cf|supplemental-3c3146cf|3c3146cf|2\.6\.40|v2\.6\.40' modules tests scripts tools formal`）

旧コーパス本体（`tests/golden/upstream-3c3146cf/`、`tests/golden/supplemental-3c3146cf/` のデータ行）を除いたヒットを 1 行ずつ分類した。分類は次の 3 つ。**A** = 誤正規化を防ぐ入力リテラル・採取ガード（用途が明記されている）、**B** = 歴史的注記（実装当時の根拠・本家行番号。旧コーパスを読まない）、**C** = 旧版・削除済み記録を現行の根拠にしている（未達）。

| ファイル:行 | 内容 | 分類 | 所見 |
| --- | --- | --- | --- |
| `modules/core/infrastructure/tests/golden_corpus_read.rs:432-433` | `normalization_preserves_stable_record_names_and_upstream_pins` の入力 `intents/example-1234abcd upstream-3c3146cf build-host-deadbeef` | A | 固定文字列を正規化しないことを検査する入力。用途はテスト名と U1 一覧に明記。移行済み |
| `scripts/goldens/capture-cli.ts:314`、`capture-supplemental.ts:112` | 旧採取コーパスを上書きしない assert | A | 採取スクリプトの保護。用途明記 |
| `scripts/goldens/capture-source.test.ts:66` | 来歴コミットの取り違えを拒否するテストの入力 | A | 拒否テストの陰性入力として旧 SHA を使う。用途明記 |
| `modules/core/command/domain/src/orchestration/mod.rs:3`、`workspace/mod.rs:3` | 「現行受入の基準は固定本家 2.7.1。旧ピン 3c3146cf の個別引用は当時の実装根拠」 | B | 用途区別を明記した見出し（cli-golden-migration の「用途を訂正」の実体） |
| `modules/core/read-model-updater/src/workspace/projection.rs:129-131` | `Mode` キーの由来。「旧 2.6.40 採取では成功経路を採れなかった…この古い制限を現行 2.7.1 の挙動や互換性の証明として使わない」 | B | 用途区別を明記 |
| `modules/core/command/domain/src/orchestration/intent_execution.rs:945,1075,1730,2083,2133,2250` | upstream 関数名と行番号（`emitJumpDirective`、`handleReport`、`verifyReviewerPrecondition`、`handleSetAutonomy`、`handleSingleReport`、`handleReview`） | B | 当時の実装根拠。2.7.1 の行番号へは更新されていない。挙動は 2.7.1 コーパス（jump-normal.json、stage1/cases.json 等）で別途検査 |
| `modules/core/command/domain/src/orchestration/gate_decision.rs:11`、`workflow_definition/review_policy.rs:100`、`orchestration/review_verdict.rs:25`、`modules/core/command/use-case/src/orchestration/review_log_kind.rs:6` | 同上（`computeGate`、budget 導出、verdict 大小無視、`--verdict` 有無） | B | 同上 |
| `modules/core/read-model-updater/src/orchestration/read_model_updater.rs:387`、`workspace/projection.rs:103,105,113,1678,1871` | 書込順・監査欄の出典（`aidlc-state.ts:3705-3723`、`aidlc-log.ts:916-919` ほか） | B | 同上。`PRACTICES_AFFIRMED` / `REVIEW_REQUESTED` の欄順は 2.7.1 の `audit_block_golden_test`（106 ブロック）で描画一致 |
| `modules/app/aidlc/src/runtime.rs:1680,1939,2189`、`cli/request.rs:112,124` | `handleReview` / `handlePracticesPromote` / `handleSetAutonomy` の段順、`aidlc-bolt` / `aidlc-state` の動詞集合 | B | 同上 |
| `modules/core/command/domain/src/workspace/audit_events.rs:361` | 「research golden-3c3146cf-audit §1 の語形の非一様性」 | B/C | 削除済み research 文書を引用。検査内容（`Stage Completion` 等の見出し）は 2.7.1 の 106 ブロック描画一致で担保されるので挙動は正、注記の根拠だけが失効 |
| `modules/app/aidlc/src/wording.rs:453` | `stage_graph_not_readable`「ピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8565-8570` @3c3146cf)」 | C | 2.7.1 で再確認した記録が無い。この文言を含む 2.7.1 採取は無い（cli 28 ケースに該当なし） |
| `modules/app/aidlc/src/wording.rs:543-545` | 「`report` の逐語 — 逐語はピン `3c3146cf` の … が正本」 | C | 旧ピンを**正本**と明記。report 5 ケースの逐語は 2.7.1 の `cli/report/*/stdout.json` と slug 置換付きで一致しており挙動は正だが、注記が 2.7.1 と矛盾 |
| `modules/app/aidlc/src/wording.rs:687-690` | `resume_redo` の命令綴りは「逸脱台帳 #1 の写像」、`cli_golden_test.rs` の「駆動できないケース」を参照 | C | 逸脱台帳は削除済み `docs/specs/`（2026-09-07）の記録。`cli_golden_test.rs` に「駆動できない」の節はもう無い（grep 0 件）。綴り差（`aidlc-jump execute` vs 本家 `bun .claude/tools/aidlc-jump.ts execute`）は §7 R3 |
| `modules/app/aidlc/src/cli/mod.rs:23`、`runtime.rs:1945,2272`、`wording.rs:1261`、`tests/intent_lifecycle.rs:5729`、`modules/core/query/use-case/src/orchestration/{read_only_verb.rs:4,engine_command.rs:8}`、`modules/core/command/domain/src/workflow_definition/workflow_definition.rs:24` | 「逸脱台帳 #1 / #2」を根拠にする注記（計 9 行） | C | 同上。特に `engine_command.rs:1-8` は「self-host 正準のマルチコール形」を逸脱台帳 #1 で正当化するが、同ファイル `:87-104` の `ReadOnlyUtility` / `NounTokens` は 2.7.1 観測（`selfhost-stage1/next-input.json`）に合わせて `bun .claude/tools/aidlc-utility.ts …` を出しており、注記と実装が食い違う（§7 R3） |
| `modules/core/command/interface-adapter/src/orchestration/compiled_definition_repository_impl.rs:35` | 「ピン留め `3c3146cf` の配布実バイト 33 ノードが全数取り込めることは `golden_parity_test` が固定した」 | C | `golden_parity_test.rs:91` は既に `upstream-a277af21/data` を読む。注記が古い |
| 同 `:729` | 「contract-pretty のバイト契約 (12 §10 / golden-3c3146cf-graph-dist §1-§2)」 | C | 削除済み research 文書と旧ピンを根拠に記述。実体は 2.7.1 の `storing_the_ingested_bundle_reproduces_the_dist_bytes` で 91,041 バイト一致 |
| `modules/core/read-model-updater/src/workspace/wording.rs:6,10,21` | 「各関数の出典（upstream file:line @ 3c3146cf）」「ピン留めソース 3c3146cf で逐語確認済み（2026-08-22 の採取で 4/4 バイト一致）」 | C | 採取状態の根拠が旧採取。当該文言 `Field not found in state file` は 2.7.1 の `cli/set-autonomy/state-field-absent` でも版差なし（legacy-differences で完全一致側）なので挙動は正。注記の根拠だけが 2.7.1 で未更新 |
| `formal/orchestration/stop_hook.qnt:6-7`、`engine_loop.qnt:4-5,38` | モデルの抽象元「upstream v2 @ 3c3146cf の 07-hooks.md / 02-orchestration-engine.md」、`handleSetAutonomy` 行番号 | B | モデルの由来注記。engine_loop は v2.10（2026-09-10）で 2.7.1 の jump 契約へ追従済み。stop_hook の 2.7.1 追従有無は本棚卸しの範囲外（Step 9 の Quint ゲートで扱う） |
| `tests/golden/upstream-a277af21/cli/set-autonomy/state-field-absent/case.json:6`（出所 `scripts/goldens/capture-cli.ts:548`） | 「ピン 3c3146cf の intent-create が起こす状態ファイルには … 行が無いため …」 | C | **2.7.1 コーパス内**のケース説明に旧ピン名が残る。`provenance.commit` は a277af21 で正しい。修正には再採取または再封印（`corpus-manifest.json`）が要る（§7 R6） |
| `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md:5,25` | 「仕様の正本はゴールデン `tests/golden/upstream-3c3146cf/`（ピン 3c3146cf = v2.6.40）」 | C | 検索範囲外のパスだが、規則の正本が旧コーパスを正本と宣言している。2026-09-08 の同ファイル改訂（作業ツリーの diff）でもこの 2 行は据え置き（§7 R5） |
| `tests/golden/upstream-3c3146cf/`（git 追跡 320 ファイル）、`tests/golden/supplemental-3c3146cf/`（同 3 ファイル） | 旧コーパス本体と README | C | Rust / TS のどの検査も読まない。legacy-differences の「README.md: 旧版の説明資料のみ。受入参照移行時に U2 が整理」は未着手。新コーパス `upstream-a277af21/` には README が無い（依頼書の「README.md」は現存しない）（§7 R5） |

## 2. U1 一覧（legacy-consumer-inventory.md）の 11 行の現在

| U1 の行 | 現在のファイル:行 | 状態 |
| --- | --- | --- |
| `cli_golden_test.rs:49` 旧 CLI を読む | `modules/app/aidlc/tests/cli_golden_test.rs:22-26`（`upstream-a277af21/cli`）、`:29-39` で採取元 SHA を検査 | 移行済み。比較方式は §3.1 |
| 同 `:235` / `:280` run-stage の persona/narration、park の narration 欠落を期待 | `:233-274`（persona 全文一致、欠落例外なし）、`:281-292`（park は公開 JSON 全文バイト一致、`presenter.rs` に narration 実装済み） | 移行済み。欠落を固定する期待は無い |
| 同 `:19` / `:351` stage-jump-print 等を駆動できない旨 | `:1-12` の見出しは「非ゲート報告、フェーズをまたぐ報告、次工程の全配布入力の再現は別途必要」とだけ書き、stage-jump-print / after-approval の記述は消えている | 記述は更新されたが、当該ケースは依然として駆動されていない（§4） |
| `golden_parity_test.rs:90` 配布 JSON のパース・件数・往復 | `modules/core/command/interface-adapter/tests/golden_parity_test.rs:89-92`（`upstream-a277af21/data`）、件数 `:53-86`（bugfix 9 / refactor 10 は 2.7.1 実測）、`:456-494` で graph/grid のバイト一致 | 移行済み。distribution-field-migration の `review_artifact` / センサー 3 欄の保持も現行ソースで確認 |
| `support/mod.rs:228` コーパス root と正規化設定 | `modules/core/infrastructure/tests/support/mod.rs:228-233`（`upstream-a277af21`） | 移行済み |
| `golden_corpus_read.rs:155` 採取ファイル・来歴・必須範囲・正規化の固定点 | `golden_corpus_read.rs:229-275`（来歴 SHA a277af21）、`:278-304`（BR2.4 の verb / report / hook）、`:373-426`（固定点） | 移行済み |
| 同 `:493` `../supplemental-3c3146cf/cases.json` | `:494-548`（`supplemental/cases.json`、SHA a277af21、`synthetic-preconditions`、multi-part 240 規則・set-autonomy gated・transcript carve-out） | 移行済み。ただし検査対象はコーパスの内容であり native の同経路ではない（§4 #12） |
| `golden_hash_canonical.rs:22` / `:216` | `golden_hash_canonical.rs:22-25`、`:216-249`（ケース単位の採取元も a277af21） | 移行済み。旧新の cases は provenance の commit 以外同一（`jq` で確認）、32 行 × 5 面を全行比較 |
| `audit_block_golden_test.rs:35` / `:123` 70 ブロック | `audit_block_golden_test.rs:36`、`:170`（106 ブロック）、`:177-205`（27 イベント語） | 移行済み。件数は増えて固定 |
| `projection_golden_test.rs:84` / `:239` | `projection_golden_test.rs:87-89`、`:118-124`（33 ノード / classic 列）、`:245-269`（ハンク断片へ投影） | 移行済み。18 ケース + native_audit_scope 2 件 = 20 件成功。断片外の行を補う `context` は 5 ケースに残る（`:374-383,:407-422,:468-472,:499-512`） |
| 同 `:268` / `:518` genesis は監査のみ、scaffold missing 許容、completed-ungated は参照なし | `:275-293`（`assert_audit_only`、`scaffold missing` 許容）、`:553-563`、`:386-388`（completed-ungated は b42 で撤去、#85 = A） | 変更なし（意図どおり）。状態全文の検収は classic では無し、bugfix では `bugfix_initial_state_matches_the_captured_271_bytes` |

「旧ピンを根拠とする本体コメント」（U1 一覧後半）は §1 の B/C に分類した。

## 3. 全数差分（legacy-differences.json）の対応表

`legacy-differences.json` は `old` / `new` / `classification` / 両 SHA だけを持ち、差の中身は `legacy-differences.patch` にある。下表の「差の内容」は patch から読み取った。

### 3.1 公開標準出力/標準エラー/終了値の版差（8 件、U2 の適合対象）

| 差分ファイル | 2.6.40 → 2.7.1 の差 | 現在の検査（ファイル:行、テスト名） | 比較方式 | 判定 |
| --- | --- | --- | --- | --- |
| `cli/continue/load-steering/stdout.json` | run-stage の `bundle` 等 | `cli_golden_test.rs:233-274` `the_terminal_run_stage_keys_and_persona_match_the_recorded_case` | 合成 3 ノードグラフ。本家の必須キーが欠けないこと + 追加キーが reviewer 宣言由来の 3 つだけ + `conductor_persona` 全文一致 | 一部（キー集合）。同型の全文一致は bugfix/brownfield で `upstream_271_contract.rs:2955-2962` `the_first_reverse_engineering_directive_matches_fixed_upstream_bytes`（`selfhost-stage1/bugfix-first-next.json`、2.7.1 採取） |
| `cli/next/after-approval/stdout.json` | `bundle` ハッシュ | 消費者なし | — | 未駆動（§4 #3） |
| `cli/next/no-active-intent/stdout.json` | 「26 of 33 stages, 23 approval gates」→「25 of 33 stages, 22 approval gates」 | 消費者なし。native の文言生成は `wording.rs:278-294` にあるが、名指すコマンドは `engine_command.rs:185-199` の `aidlc-utility intent-create --scope …`（本家は `bun .claude/tools/aidlc-utility.ts intent-create --scope classic`） | — | 未駆動（§4 #4、§7 R3） |
| `cli/next/start/stdout.json` | `bundle` ハッシュ | `cli_golden_test.rs:214-227` `the_load_steering_keys_match_the_recorded_case` | キー集合のみ | 一部 |
| `cli/report/awaiting-approval-repeat/stdout.json` | 末尾に `; gate evidence revalidated.` | `cli_golden_test.rs:358-367` `reopening_an_awaiting_gate_matches_the_revalidated_reply_after_slug_substitution`（`wording.rs:875`） | 1 行全文、slug 置換 | 一部。本家はこの再報告でセンサーを再発火（同ケース `audit.md` に `SENSOR_FIRED` / `SENSOR_FAILED`）。裁定 A で対象外。文言だけを揃えたことは [current-resume-note.md](current-resume-note.md) の表にも明記 |
| `hooks/state-transition-guard/deny-delegated-lifecycle/stderr` | 文言全面改訂 | `claude_hook_contract.rs:165-216` `state_transition_guard_matches_the_four_fixed_upstream_boundaries` | stdout / stderr / exit をバイト比較、初期化なしも検査 | 満たした |
| `hooks/state-transition-guard/deny-direct-state-transition/stderr` | 同上 | 同上 | 同上 | 満たした |
| `hooks/stop-forwarding-loop/block-pending-directive/stdout` | reason が「`rules_content` を適用せよ」から「step-two の continue コマンドを保持せよ（トークン付き）」へ | 直接の消費者なし。同型の bugfix 観測は `upstream_271_contract.rs:90-160` `stop_blocks_pending_work_once_then_releases_without_workflow_progress`（`selfhost-stage1/stop-values.json` の `run-reverse-engineering` と stdout バイト一致） | classic ケース本体は未駆動 | 一部 |

### 3.2 監査の版差（18 件、U2 の適合対象）

描画: `audit_block_golden_test.rs:123-208` は 42 ファイル 106 ブロックすべてを読み取り→再描画→バイト一致で検査する（SENSOR_* 36 ブロックを含む）。これは「フィールドが与えられれば同じバイトを描ける」の検査であり、「事象から同じフィールドを作る」の検査ではない。投影: `projection_golden_test.rs` は手で組んだ 1 イベントを投影し、`native_audit_scope::expected_native_audit`（`tests/support/native_audit_scope.rs`）で SENSOR_FIRED / SENSOR_PASSED / SENSOR_FAILED のブロックだけを期待から外して比較する（裁定 A）。CLI: classic scope の CLI 駆動で監査ファイルを比較するテストは無い。

| 差分ファイル | 差の内容 | 投影検査 | CLI 駆動 | 判定 |
| --- | --- | --- | --- | --- |
| `cli/intent-create/classic-scope/audit.md` | `WORKFLOW_STARTED` に `Source Baseline` | `the_genesis_draws_all_sixteen_initialization_rows`（監査のみ、`scaffold missing` 許容） | なし（bugfix は `bugfix_initial_state_matches_the_captured_271_bytes` が状態全文、監査は未比較） | 一部 |
| `cli/jump/execute-backward/audit.md` | `Skip Kind`、`Source Baseline`、`Changed Upstream Artifacts` / `Invalidated Downstream Artifacts` / `Invalidated Downstream Reviews` | `jumping_backward_resets_the_downstream_and_hands_the_phase_row_back`（両面、`jump_observation()` を渡す） | bugfix は `jump_contract.rs:382` `normal_jump_stdout_stderr_and_audit_match_fixed_upstream`（`selfhost-stage1/jump-normal.json`） | 満たした（投影） |
| `cli/jump/execute-forward-across-phases/audit.md` | `Skip Kind: jump`、`Source Baseline` | `jumping_forward_across_a_phase_verifies_the_one_it_leaves`（両面） | 同上 | 満たした（投影） |
| `cli/jump/execute-forward-to-conditional/audit.md` | 同上 | なし | なし | 未駆動（§4 #5） |
| `cli/jump/execute-forward/audit.md` | 同上 | `jumping_forward_skips_the_source_and_opens_the_target`（両面、context 付き） | 同上 | 満たした（投影） |
| `cli/recompose/rejected-starved-input/audit.md` | `ERROR_LOGGED` の `Command` 欄が `<ROOT>` → `<project-dir>`（本家がパスを伏せる） | なし（監査のみの差） | native は `runtime/log_failure.rs:81-105` で `<project-dir>` へ伏せる。この経路の比較は `selfhost-stage1/log-failure.json`（`intent_lifecycle.rs:669`） | 一部 |
| `cli/report/approved-across-phases/audit.md` | SENSOR_*、`Validation Basis` | `approving_the_last_stage_of_a_phase_counts_the_checkboxes_not_the_plan`（センサー除外、context 付き。`Validation Basis` は採取済みの値を `Reported` に渡す） | Validation Basis の採取は `validation_basis.rs` と `selfhost-stage1/validation-basis.json` 22 観測（[validation-basis-verification.md](validation-basis-verification.md)） | 一部（センサー除外は裁定 A） |
| `cli/report/approved/audit.md` | SENSOR_*、`Validation Basis`、`Source Baseline` | `approving_a_gate_completes_the_stage_and_starts_the_next_one`（同上） | 同上 | 一部 |
| `cli/report/awaiting-approval-repeat/audit.md` | SENSOR_* のみ | なし | なし | 対象外（裁定 A） |
| `cli/report/awaiting-approval/audit.md` | SENSOR_* | `opening_a_gate_writes_the_awaiting_approval_row_and_moves_the_checkbox`（除外後は両面一致） | なし | 一部 |
| `cli/report/completed-ungated/audit.md` | `Validation Basis` | なし（`projection_golden_test.rs:386-388`） | 到達不能（`commit_error.rs` `UnwiredTransition`: advance / complete-workflow は b42 で撤去、#85 = A） | 意図的な到達不能（§4 #1） |
| `cli/report/revised/audit.md` | SENSOR_* | `revising_a_stage_re_enters_the_gate_with_the_verbatim_details` | なし | 一部 |
| `cli/set-autonomy/state-field-absent/audit.md` | `ERROR_LOGGED` の `Command` 欄 `<project-dir>` | なし | `intent_lifecycle.rs:5544-5790` は文言のみ（コーパス未参照） | 一部（§4 #8） |
| `cli/skip/skipped/audit.md` | `Skip Kind: conditional-runtime` | `skipping_a_stage_moves_on_without_touching_the_completed_count`（両面） | なし | 満たした（投影） |
| `hooks/write-audit-log/{artifact-created, artifact-updated-by-edit, artifact-updated-by-overwrite, trusts-the-settings-matcher}/audit.md` | `File` 欄 `<ROOT>` → `<project-dir>` | なし（描画は 106 ブロックに含む） | `claude_hook_contract.rs:401-543` は `contains("**Event**: ARTIFACT_CREATED")` 等の部分一致。コーパスの監査バイトとは未比較 | 一部（§4 #11） |

### 3.3 状態の版差（19 件、U2 の適合対象）

多くの `state.diff` の版差は、状態ファイル冒頭に `- **Project Description Source**: project-description.json` が加わったことによるハンク行番号の +1 だけである（内容は同じ）。投影テストはハンク断片へ投影するので、この差は検査の成否に影響しない。

| 差分ファイル | 差の内容 | 投影検査 | 判定 |
| --- | --- | --- | --- |
| `cli/intent-create/classic-scope/state-full.md` | `Project Description Source` 行の追加、`Project Root` が `<ROOT>` → `.` | なし。genesis 投影は空の本文へ当てて `scaffold missing` を許容（骨格は合成ルートの責務） | 一部。同じ骨格の全文一致は bugfix で `bugfix_initial_state_matches_the_captured_271_bytes`（`upstream_271_contract.rs:2044-2156`。`Project Root: .`、`Project Description Source` を含む） |
| `cli/intent-create/classic-scope/state.diff` | 同上 | 同上 | 一部 |
| `cli/jump/execute-backward/state.diff` | 行番号のみ | `jumping_backward_…` | 満たした（投影） |
| `cli/jump/execute-forward-across-phases/state.diff` | 行番号のみ | `jumping_forward_across_a_phase_…` | 満たした（投影） |
| `cli/jump/execute-forward-to-conditional/state.diff` | 行番号のみ | なし | 未駆動 |
| `cli/jump/execute-forward/state.diff` | 行番号のみ | `jumping_forward_skips_…`（context 3 行） | 満たした（投影） |
| `cli/park/park/state.diff` | 行番号のみ | `parking_writes_the_marker_…`。CLI stdout は `cli_golden_test.rs:281-292` でバイト一致 | 満たした |
| `cli/practices-promote/affirm/state.diff` | 行番号のみ | なし。`intent_lifecycle.rs:4960-5050` は `contains` | 一部（§4 #7） |
| `cli/recompose/add-restores-conditional/state.diff` | 行番号のみ | `recomposing_back_into_scope_drops_the_annotated_entry_whole`（context 3 行） | 満たした（投影） |
| `cli/recompose/skip-one/state.diff` | 行番号のみ | `recomposing_moves_a_stage_between_the_two_plan_lists` | 満たした（投影） |
| `cli/recompose/skip-two-appends-in-graph-order/state.diff` | 行番号のみ | `recomposing_keeps_existing_skip_entries_where_they_are` | 満たした（投影） |
| `cli/report/approved-across-phases/state.diff` | 行番号のみ | `approving_the_last_stage_of_a_phase_…`（context 1 行） | 満たした（投影） |
| `cli/report/approved/state.diff` | 行番号のみ | `approving_a_gate_…`（context 3 行） | 満たした（投影） |
| `cli/report/awaiting-approval/state.diff` | 行番号のみ | `opening_a_gate_…` | 満たした（投影） |
| `cli/report/completed-ungated/state.diff` | 行番号のみ | なし | 意図的な到達不能 |
| `cli/report/rejected/state.diff` | 行番号のみ | `rejecting_a_gate_writes_two_rows_…` | 満たした（投影） |
| `cli/report/revised/state.diff` | 行番号のみ | `revising_a_stage_…` | 満たした（投影） |
| `cli/skip/skipped/state.diff` | 行番号のみ | `skipping_a_stage_…` | 満たした（投影） |
| `cli/unpark/unpark/state.diff` | 行番号のみ | `unparking_removes_both_marker_lines` | 満たした（投影） |

「満たした（投影）」は、手で組んだイベントから RMU が同じ行を描くことの検収であり、classic scope のコマンド → イベント → SQLite → RMU → ファイルの経路を同じ前提で比較したものではない。その経路の全文比較は bugfix scope（`stage1/cases.json`、`selfhost-stage1/*.json`）で `upstream_271_contract.rs` / `jump_contract.rs` / `pipeline_link_contract.rs` / `session_hooks_contract.rs` 等が行っている。

### 3.4 入力表現 3 件・hash 1 件・UUID/token 1 件・配布 JSON 2 件・README 1 件

| 差分ファイル | 差の内容 | 現在の検査 | 判定 |
| --- | --- | --- | --- |
| `hooks/stop-forwarding-loop/{block-pending-directive, no-workflow-ignored, reentrant-ignored}/stdin.json` | `session_id` が `<SESSION>` → 固定 UUID `11111111-2222-4333-8444-555555555555`（固定識別子の保持） | `no-workflow-ignored` は `claude_hook_contract.rs:219-264` が corpus の stdin バイトをそのまま流し stdout/stderr をバイト比較。`block-pending-directive` / `reentrant-ignored` の stdin は未使用（`upstream_271_contract.rs:573-593` は inline の `{"stop_hook_active":true}`） | 一部 |
| `hash-canonical/cases.json` | provenance の `upstream_commit` のみ（32 ケースの id / input / construct / expected は旧新同一。`jq` で確認） | `golden_hash_canonical.rs` 7 テスト（5 面 × 32 行、入力クラス 11、来歴） | 満たした |
| `normalization.json` | 説明文の更新、`<SESSION>` の regex 規則 2 本（UUID、200 文字以上の token）を削除 | `golden_corpus_read.rs:307-320`（4 種プレースホルダの閉集合）、`:323-370`（未対応 ID の保持）、`:373-426`（固定点）、`:429-444`（旧ピン文字列と記録名の保持）。TS `corpus-normalization.ts:8-36` `readBindings`（同値へ潰す規則を拒否） | 満たした。残置: `ALLOWED_PLACEHOLDERS`（`:151`）と `families.*.applies` に `<SESSION>` が残る（規則が無いので無害だが、規則が復活しても検出しない） |
| `data/scope-grid.json`、`data/stage-graph.json` | 2.7.1 の配布実バイト（`review_artifact`、センサー参照の `fire_on` / `default_severity` / `category`、bugfix 9 / refactor 10） | `golden_parity_test.rs` 11 テスト（`storing_the_ingested_bundle_reproduces_the_dist_bytes` でバイト一致） | 満たした |
| `README.md`（旧コーパス） | 「旧版の説明資料のみ。受入参照移行時に U2 が整理」 | 未着手。旧コーパスと README は残置、新コーパスに README なし | 未達（§7 R5） |

## 4. 駆動できないままの必要ケース

「到達不能（設計）」は根拠コメントがあり裁定済みのもの。「未駆動」はテストが無いだけで、実装の有無・一致は本棚卸しでは確定していない（[step7-divergence-questions.md](step7-divergence-questions.md) の「拒否文言を見ただけで欠落と判断しない」に従い、実行して確かめるまで欠落と断じない）。両出力を採るための手順は [step8-logs/native-classic-probe/](step8-logs/native-classic-probe/) に置いたが、委任エージェントは `aidlc-state-transition-guard` に実行を拒否されるため親が実行する必要がある。

| # | ケース | 現状 | 分類 | 根拠 / 必要なこと |
| --- | --- | --- | --- | --- |
| 1 | `cli/report/completed-ungated`（stdout / audit / state） | 参照テストなし | 到達不能（設計、裁定済み） | `modules/core/command/use-case/src/orchestration/commit_error.rs` `UnwiredTransition` の doc「advance / complete-workflow の 2 段が該当 — 非ゲート完了のパイプラインは b42 で撤去した（#85 = A）。初期化ステージだけが in-scope の縮退計画でだけ到達する」、`projection_golden_test.rs:386-388`。2.7.1 の同ケースは `Validation Basis` を監査に乗せるので、切替条件 2 の判定材料として残す |
| 2 | `cli/next/stage-jump-print` | 参照テストなし | 上流仕様との不一致の疑い（裁定対象、§7 R3） | 本家 stdout（コーパス逐語）: `{"kind":"print","message":"Run \`bun .claude/tools/aidlc-jump.ts execute --target contract-design --direction forward --scope classic\` to perform the jump, then re-run \`next\` to continue from the jump target."}`。native: `to perform the jump` を出すソースは無い（grep 0 件）。`next --stage` の解決は `EngineCommand::ResolveJump` → `aidlc-jump resolve --stage <slug>`（`engine_command.rs:106-108`）であり、本家の「execute を direction / scope 付きで名指す」形と綴り・意味の両方が違う可能性がある。native の実出力は未採取 |
| 3 | `cli/next/after-approval` | 参照テストなし | 未駆動（裁定不要） | 承認後の `next` が requirements-analysis の規則束を配る。合成グラフでは `bundle` が一致しないので、配布グラフと採取時の memory 層で前提を再現するテストが要る |
| 4 | `cli/next/no-active-intent` | 参照テストなし | 未駆動 + 綴り差の疑い（§7 R3） | 本家: `Run \`bun .claude/tools/aidlc-utility.ts intent-create --scope classic\` to start the workflow (25 of 33 stages, 22 approval gates, 5 stages repeat per unit of work in Construction), …`。native の文言は `wording.rs:278-294`、名指すコマンドは `engine_command.rs:185-199` `aidlc-utility intent-create --scope …`。件数（25 / 22 / 5）の一致は未確認 |
| 5 | `cli/jump/execute-forward-to-conditional` | 投影・CLI とも参照なし | 未駆動（裁定不要） | conditional（reverse-engineering を含む区間）を跨ぐ前方ジャンプの `Skip Kind` / `Source Baseline`。bugfix の forward は `jump-normal.json` で一致 |
| 6 | `cli/jump/resolve-forward` | 参照テストなし（classic） | 未駆動 | bugfix の `resolve` は `jump_contract.rs:294` と `jump-normal.json` |
| 7 | `cli/practices-promote/affirm`（stdout / state / audit） | コーパス未参照 | 未駆動 | `intent_lifecycle.rs:4960-5050` は inline 文字列の `contains`。2.7.1 の `state.diff` / `audit.md`（`PRACTICES_AFFIRMED`）とは未比較 |
| 8 | `cli/set-autonomy/state-field-absent`（stdout.txt / audit） | コーパス未参照 | 未駆動 | `intent_lifecycle.rs:5544-5790` は inline 文言。2.7.1 でも本ケースは exit 1（状態ファイルに `Construction Autonomy Mode` 行が無い）で、コーパスの stdout は空、`ERROR_LOGGED` を監査へ書く |
| 9 | `cli/report/approved-across-phases`（CLI 面） | 投影のみ | 未駆動（CLI） | フェーズ境界越えの CLI 駆動は `intent_lifecycle.rs:3851-3853` の inline 期待（`PHASE_COMPLETED`、`Phase boundary: inception → end`）。33 ノードの配布グラフでの境界越えは未比較 |
| 10 | `hooks/stop-forwarding-loop/{block-pending-directive, reentrant-ignored}` | コーパス未参照 | 未駆動（同型を別観測で検査） | bugfix の同型は `stop-values.json` / `stop-publication.json` で stdout バイト一致。classic のトークン付き reason は未比較 |
| 11 | `hooks/write-audit-log/*`（5 ケース）、`hooks/record-human-turn/active-workflow` の監査バイト | フック駆動の `contains` 検査のみ | 一部 | 描画は 106 ブロック一致。`File` / `Session` 欄を含む 1 ブロックのバイト一致をフック駆動で見るテストは無い |
| 12 | `supplemental/cases.json` の 3 ケース（`cli/continue/multi-part`、`cli/set-autonomy/gated`、`hooks/stop-forwarding-loop/transcript-carve-out`） | `golden_corpus_read.rs:494-548` はコーパスの内容だけを検査 | 一部 | native 側: transcript は `stop_transcript_contract.rs`（87 観測一致、[stop-transcript-verification.md](stop-transcript-verification.md)）、set-autonomy gated は `intent_lifecycle.rs:5544-`（b50）。multi-part（`parts > 1`）の native 配送を 2.7.1 観測と比較するテストは本棚卸しで見つけられなかった（[continuation-rejection-verification.md](continuation-rejection-verification.md) の範囲は拒否 3 経路） |

補足: `modules/app/aidlc/tests/jump_contract.rs:14` は受入フィクスチャを `tests/golden/` ではなく intent 記録内の `jump-logs/questions-observations.json` から `include_str!` している。動作はするが、コーパスの封印（`corpus-manifest.json`）の外にあり、記録の整理で消えると壊れる（§7 R9）。

## 5. 不正 UTF-8 と ID 対応

- 不正 UTF-8: `tests/golden/selfhost-stage1/human-invalid-utf8.json`（2.7.1、byte `FF` を stdin に渡す合成入力）の消費者は `upstream_271_contract.rs:3463-3560` `an_invalid_utf8_stdin_still_records_an_anonymous_human_turn`。stdin を base64 から復号したバイトで渡し、exit / stdout / stderr / 追記監査を base64 のバイトで比較する（`Timestamp` 行だけ置換）。`claude_hook_contract.rs:313-331` は `[0xff]` を state-transition-guard に渡して無視・無初期化を検査。`golden_corpus_read.rs` に不正 UTF-8 の検査は無い（依頼書の「該当テスト」は存在しない）。コーパス側は `verify-corpus.ts:10-14` `readJson` が JSON の不正 UTF-8 を拒否し、`corpus-normalization.ts:38-45` `comparableFile` が不正 UTF-8 を文字列置換の対象にせず生バイトで比較する。Rust 側にこの `binary_bytes` 相当の読み手は無い（`observations.json` / `stage1/cases.json` の `initial_files` を Rust が読むのは base64 復号のみ）。
- ID 対応: `normalization.json` に UUID / token を一律に潰す規則は無い（`jq` で旧新の規則を並べて確認。旧版の `<SESSION>` 2 規則が除去）。`comparison.json` は `identifier` 13 / `token` 13 / `digest` 5 / `fire-id` 22 を役割番号（`<id:n>` / `<token:n>` …）へ 1:1 対応させ、`readBindings` が同じ置換先の重複を拒否する。Rust 側でこの `comparison.json` を読む比較器は無い（`rg 'comparison\.json'` は 0 件）。Rust の各テストは、記録名（`upstream_271_contract.rs:2116-2154` で採取時の記録名を実測名へ 1:1 置換）、日付（`:223-228`）、`Start Date` と `Last Updated` の同値対応（`:2109-2120`、同じ値が 2 回現れることを assert）、slug（`cli_golden_test.rs:303-305`）を個別に対応させており、識別子を一律に消してはいない。継続トークンは比較対象の run-stage 出力に含まれないので対応付けの対象になっていない。

## 6. テスト実行結果（2026-09-10、`rustc 1.95.0`、ログ [step8-logs/](step8-logs/)）

| コマンド | 結果 | 件数 | ログ |
| --- | --- | --- | --- |
| `cargo test -p aidlc --test cli_golden_test --test upstream_271_contract` | exit 0 | `cli_golden_test` 10 成功 / 0 失敗、`upstream_271_contract` 75 成功 / 0 失敗（336.61s） | `app-cli-golden-and-271.log` |
| `cargo test -p core-command-interface-adapter --test golden_parity_test` | exit 0 | 11 成功 / 0 失敗 | `interface-adapter-golden-parity.log` |
| `cargo test -p core-infrastructure --test golden_hash_canonical --test golden_corpus_read` | exit 0 | `golden_corpus_read` 14 成功、`golden_hash_canonical` 7 成功、失敗 0 | `infrastructure-golden.log` |
| `cargo test -p core-read-model-updater --test audit_block_golden_test --test projection_golden_test` | exit 0 | `audit_block_golden_test` 1 成功、`projection_golden_test` 20 成功、失敗 0 | `rmu-golden.log` |
| `bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21` | **exit 1** | 「採取ファイルの欠落または追加」: `reviewer-scope/cases.json`（2026-09-10 12:58 に追加）が `corpus-manifest.json`（2026-09-08 09:28 封印）に無い | `verify-corpus.log` |

件数は 2026-09-10 12:07 の workspace 通し実行（[step9-logs/workspace-test-2026-09-10.log](step9-logs/workspace-test-2026-09-10.log)）と同じ。テストバイナリのハッシュは変わっており（他担当の並行編集）、本実行は現在の断面に対するものである。コンパイル失敗は起きなかった。

## 7. 親と人間の裁定・是正が要る項目

| ID | 項目 | 入力・両出力・該当ソース | 種別 |
| --- | --- | --- | --- |
| R1 | **コーパスの封印が古く、CI が落ちる** | `.github/workflows/ci.yml:45` は `bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21` を実行する。現状は exit 1（§6）。`reviewer-scope/cases.json`（`u2_reviewer_scope` が採取。`source.commit` は a277af21）が `corpus-manifest.json` に無い。是正は `scripts/goldens/prepare-corpus.ts:82` と同じ `sealCorpus(root)` による再封印。コーパスの所有者（親）が行う | 是正（親） |
| R2 | 新コーパス `tests/golden/upstream-a277af21/` と `tests/golden/selfhost-stage1/` が **git 未追跡**（`git status` で `??`、ignore 対象ではない）。旧コーパス 2 本は追跡中 | B1 の PR に含めないと CI の verify-corpus と全 golden テストが CI で失敗する | 是正（親） |
| R3 | **print directive が名指すコマンドの綴り**（旧「逸脱台帳 #1」） | 入力: 空 workspace で `next --scope classic`、および `next --stage contract-design`。本家出力: `cli/next/no-active-intent/stdout.json`（`bun .claude/tools/aidlc-utility.ts intent-create --scope classic`）、`cli/next/stage-jump-print/stdout.json`（`bun .claude/tools/aidlc-jump.ts execute --target contract-design --direction forward --scope classic`）。native: `engine_command.rs:87-104` の `ReadOnlyUtility` / `NounTokens` は 2.7.1 観測（`selfhost-stage1/next-input.json`、`next_input_contract.rs:10-58` でバイト一致）に合わせて `bun .claude/tools/aidlc-utility.ts …` を出すが、`:105-108`（`aidlc-state unpark`、`aidlc-jump resolve --stage`）、`:185-199`（`aidlc-utility intent-create`）、`:120-155`（`aidlc-utility scope-change` / `config-change`、`aidlc-composer detect`、`aidlc-orchestrate report`）、`wording.rs:692-695`（`aidlc-jump execute … --direction redo`）はマルチコール形のまま。同じ 1 点で綴りを切り替える設計（`engine_command.rs:1-8`）なのに 2 形が混在し、根拠は削除済みの逸脱台帳。裁定: 2.7.1 コーパスの `bun .claude/tools/<tool>.ts` 形へ全面的に揃えるか、マルチコール形を承認済みの観測差として記録するか。加えて `next --stage` が本家の「execute を名指す」意味へ揃っているかは実出力を採ってから判断する（[native-classic-probe/](step8-logs/native-classic-probe/)） | 裁定（人間） |
| R4 | `cli/report/awaiting-approval-repeat` の本家はセンサー再発火を伴う | 本家 `audit.md` に `SENSOR_FIRED` × 3、`SENSOR_FAILED` × 3（`required-sections` / `upstream-coverage`）。native は文言だけ一致（`wording.rs:875`）。裁定 A（センサー 3 種は対象外）で既に対象外だが、「gate evidence revalidated」を名乗る出力がセンサー以外の根拠再検証（成果物・レビュー）を行っているかは本棚卸しでは確認していない | 確認（親） |
| R5 | 旧コーパス `tests/golden/upstream-3c3146cf/`（320 ファイル）、`supplemental-3c3146cf/`（3 ファイル）の扱いと、`coding-rules/README.md:5,25` の「正本は `upstream-3c3146cf`」記述 | どの検査も読まない。`memory/project.md` の Mandated「旧 2.6.40 の値を現行互換の根拠として残さない」に照らし、削除して README を a277af21 へ書き換えるか、履歴として残すかを決める。`legacy-differences.json` の「README.md → U2 が整理」はこの裁定待ち | 裁定（人間） |
| R6 | 2.7.1 コーパス内の旧ピン文言 | `tests/golden/upstream-a277af21/cli/set-autonomy/state-field-absent/case.json:6` と出所 `scripts/goldens/capture-cli.ts:548` の description「ピン 3c3146cf の intent-create が…」。内容は 2.7.1 でも真だが説明が旧ピンを名指す。直すなら description の修正と再封印（または再採取）が要る | 裁定（軽微） |
| R7 | 「旧ピンが正本」「逸脱台帳」を根拠にする注記の更新 | §1 の C 行（`wording.rs:453,543-545,687-690`、`compiled_definition_repository_impl.rs:35,729`、RMU `wording.rs:6,10,21`、`audit_events.rs:361`、逸脱台帳 9 行）。製品挙動の変更ではなく注記の是正。`memory/project.md` の Forbidden「削除済みの過去記録を現存する根拠として扱わない」に触れる | 是正（親判断） |
| R8 | 未駆動ケースのテスト追加（§4 #3〜#11） | 裁定は不要。配布グラフ 33 ノードと採取時の memory 層で前提を再現する CLI テスト（classic）が無い。TDD 契約に従い Red から追加する | 実装（U2 の残作業） |
| R9 | `jump_contract.rs:14` が intent 記録内のファイルを `include_str!` | `aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u2-workflow-authority/code-generation/jump-logs/questions-observations.json`。受入フィクスチャを `tests/golden/`（封印対象）へ移すかを決める | 是正（親判断） |
| R10 | `<SESSION>` プレースホルダの残置 | `golden_corpus_read.rs:151` `ALLOWED_PLACEHOLDERS`、`normalization.json` の `families.cli.applies` / `families.hooks.applies`。規則が無いので無害。一律規則が戻ったときに検出する検査が無いことだけ記す | 記録のみ |

## Sources

- 承認済み計画: [code-generation-plan.md](code-generation-plan.md) Step 8（原文 5 箇条）、[unit-test-instructions.md](unit-test-instructions.md)。
- U1: [legacy-consumer-inventory.md](../../u1-upstream-acceptance/code-generation/legacy-consumer-inventory.md)、[legacy-differences.json](../../u1-upstream-acceptance/code-generation/legacy-differences.json)（`counts`、`files[]` の `old` / `new` / `classification`）、[legacy-differences.patch](../../u1-upstream-acceptance/code-generation/legacy-differences.patch)（各ハンク）。
- U2 既存記録: [cli-golden-migration.md](cli-golden-migration.md)、[infrastructure-golden-migration.md](infrastructure-golden-migration.md)、[distribution-field-migration.md](distribution-field-migration.md)、[rmu-golden-migration.md](rmu-golden-migration.md)、[accepted-contract-decisions.md](accepted-contract-decisions.md)、[sensor-audit-comparison-questions.md](sensor-audit-comparison-questions.md)、[jump-contract-questions.md](jump-contract-questions.md)、[task-update-contract-questions.md](task-update-contract-questions.md)、[validation-basis-gap.md](validation-basis-gap.md)、[validation-basis-verification.md](validation-basis-verification.md)、[report-source-baseline-verification.md](report-source-baseline-verification.md)、[jump-verification.md](jump-verification.md)、[jump-foreign-scope-verification.md](jump-foreign-scope-verification.md)、[current-resume-note.md](current-resume-note.md)、[progress-status.md](progress-status.md)、[step7-divergence-questions.md](step7-divergence-questions.md)、[switchover-condition2-findings.md](switchover-condition2-findings.md)、[step9-logs/workspace-test-2026-09-10.log](step9-logs/workspace-test-2026-09-10.log)。
- 現在のテスト: `modules/app/aidlc/tests/{cli_golden_test,upstream_271_contract,claude_hook_contract,next_input_contract,jump_contract,intent_lifecycle}.rs`、`modules/core/command/interface-adapter/tests/golden_parity_test.rs`、`modules/core/infrastructure/tests/{golden_hash_canonical,golden_corpus_read}.rs`、`tests/support/mod.rs`、`modules/core/read-model-updater/tests/{audit_block_golden_test,projection_golden_test}.rs`、`tests/support/native_audit_scope.rs`。
- 製品コード（注記の確認）: `modules/app/aidlc/src/{wording,runtime,cli/request,cli/mod,presenter}.rs`、`modules/app/aidlc/src/runtime/log_failure.rs`、`modules/core/query/use-case/src/orchestration/engine_command.rs`、`modules/core/command/use-case/src/orchestration/commit_error.rs`、`modules/core/command/domain/src/orchestration/{mod,intent_execution,gate_decision,review_verdict}.rs`、`modules/core/command/domain/src/workspace/{mod,audit_events}.rs`、`modules/core/command/domain/src/workflow_definition/review_policy.rs`、`modules/core/command/interface-adapter/src/orchestration/compiled_definition_repository_impl.rs`、`modules/core/read-model-updater/src/workspace/{wording,projection,initial_state}.rs`、`modules/core/read-model-updater/src/orchestration/read_model_updater.rs`、`formal/orchestration/{stop_hook,engine_loop}.qnt`。
- コーパス: `tests/golden/upstream-a277af21/{source.json,corpus-manifest.json,comparison.json,normalization.json,observations.json}`、同 `cli/{provenance,cases-missing}.json` と各 `case.json` / `stdout.json` / `argv`、`hooks/cases-missing.json`、`supplemental/{cases,provenance}.json`、`stage1/cases.json`、`hash-canonical/cases.json`、`data/`；`tests/golden/upstream-3c3146cf/{README.md,cli/cases-missing.json,hash-canonical/cases.json,normalization.json}`；`tests/golden/selfhost-stage1/{bugfix-first-next,human-invalid-utf8,stop-values,next-input}.json`；`scripts/goldens/{normalization.json,corpus-normalization.ts,verify-corpus.ts,prepare-corpus.ts,capture-cli.ts,capture-supplemental.ts,capture-source.test.ts,upstream-source.ts}`。
- CI: `.github/workflows/ci.yml:39-45`。
- 規則: `aidlc/spaces/default/memory/{org,team,project}.md`、`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md`。
- 実行ログ: [step8-logs/summary-2026-09-10.txt](step8-logs/summary-2026-09-10.txt) と同ディレクトリの各ログ、[step8-logs/native-classic-probe/README.md](step8-logs/native-classic-probe/README.md)（未実行の手順）。

## Assumptions & Open Questions

- [assumption] `legacy-differences.json` の分類「U2 の適合対象」の各件は、その差を 2.7.1 の期待で検査する native 側テストの有無で「満たした / 一部 / 未駆動」を判定した。投影テスト（手で組んだイベント）による一致を「満たした（投影）」と明示し、CLI 経路の一致とは区別した。
- [assumption] 「駆動できない」の判定は、当該ケース ID をコーパスから読むテストの有無（`rg`）で行った。同じ経路を別の 2.7.1 観測（bugfix scope）で検査している場合は「一部」とし、実装欠落とは断じていない。
- [assumption] §7 R3 の native 側出力は、ソース（`engine_command.rs`、`wording.rs`）から読み取った綴りであり、実行して採ったものではない。実出力は親が [native-classic-probe/native-probe.sh](step8-logs/native-classic-probe/native-probe.sh) を実行して確定させる。
- 未解決: `cli/continue/multi-part`（`parts > 1`）の native 配送を 2.7.1 の補完観測と比較するテストの所在。本棚卸しでは見つけられなかった（§4 #12）。
- 未解決: `formal/orchestration/stop_hook.qnt` のモデルが 2.7.1 の stop フック観測（`stop-values.json` 等）と整合しているかは、Step 9 の Quint ゲートの結果（[progress-status.md](progress-status.md) では 29 ステップ成功）以上には確認していない。
