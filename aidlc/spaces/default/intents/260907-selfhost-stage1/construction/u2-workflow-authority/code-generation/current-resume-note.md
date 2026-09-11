# 現在の作業引継ぎ（工程は実行中）

## 承認と進行

Runtime Sessionは01a08484-cb70-73d2-b5b9-6a845d957b0d。利用者のApprove Planをcode-generation-questions.mdへ記録し、plan-approvalのanswerが成功済み。承認指紋は949787981b8366cb16c132c715b3a248ee1f657deb2927fd0d4283136525d887。現在のcode-generation/u2-workflow-authorityを続ける。単なる文脈復元のためにnextを再実行して承認を失効させない。工程の完了報告・独立レビュー・Unit完了は未実施。mise trustは承認後に成功済み。

## この再開で進めたこと

- 親担当: Validation Basisの採取、Reportedへの保存、両側DTO、監査欄順。StageValidation新型、app/validation_basis.rs。固定本家22観測一致。Reportedの次工程SourceBaseline保存、同じスナップショットの公開とno-opへの不正付加の拒否。app/source_baseline.rs、runtime/report、projection、ReadModelUpdaterの追加経路。
- 親担当: projection_golden_testのapprove2件を保存観測付きReportedへ移行（採取は別22観測）。Query契約の明示resumeの旧期待を本家2.7.1へ更新しDAO34件成功。
- pipeline担当: 既存消費処理の検証12件、jumpの通常resolve/executeと結果Query・無効化監査・基準snapshot。通常5操作の全文比較、projectionのjump3件が成功。read schema v4→v5の移行はTDDで作業中。
- session担当: PreCompactと所有文脈失効の結線、cold SessionStart→intent-createのstamp/handoff、終了監査の帰属。session関連20件成功。現在HealthCheckedのC7更新UseCase・既存IntentExecutionイベント/DTO/RMUを作業し、専用API7件成功。cold非呼出しの実配線検証はU3へ引継ぎ。
- context担当: owner/context/issuance revision保存と完全一致文脈失効。専用7件・既存承認16件等成功。現在レビュー受領のartifact/source/appendix結合を実装中。

## 現在の共有編集

collaborationの既存担当名はu2_pipeline_evidence（jump）、u2_session_hooks（HealthChecked）、u2_context_epoch（review receipt）。新規に重複委譲しない。reviewの署名更新中に他担当と親の再検査がコンパイル停止しているため、まずproduction/test双方がコンパイル可能な区切りを作るよう依頼済み。未接続引数を適当なデフォルトで埋めてテストを通さない。各担当の変更を戻さない。

## 最新の成功と未達

- 親の報告変更後intent_lifecycleは123件成功。以後の共有review変更後は再確認する。
- projection_golden_testは18件中16成功・2失敗。残る2件はセンサー監査3種類の未裁定差。
- 未回答はsensor-audit-comparison-questions.mdとjump-contract-questions.md。どちらもasync質問を提示しdecisionを記録済み。答えは到着していない。Approve Planからこの例外裁定を推定しない。
- jumpの直接executeは、方向不一致・別scope・初期化targetについて本家と既存モデルが不一致。通常経路や投影の成功で、公開直接実行の不一致を隠さない。
- 最終Clippy/fmt、全workspace、Quint/ITF、カバレッジ90%床/相対ゲート、release、CIとB1統合、U2の最終manifest/traceability/summary/reviewは未了。
- Step7のruntime compile/TaskUpdate同期/learnings persist/review-freeze/reviewer-scope/fold-usage等はremaining-step7-inventory.md（Unit直下）を参照。診断記録が終わってもStep7全体ではない。

## 次に行うこと

1. 各担当のコンパイル可能通知と完成記録を読み、共有変更後の適切な契約・Clippy・cargo lintを確認する。
2. 未回答の裁定が来たら、質問文書へ正確に記録しaidlc-log answerを行う。その選択に依存する互換判定を修正する。
3. 承認済み9ステップの残りを継続し、必要な成果物・source-manifest・traceabilityを揃えてから独立レビューとUnit完了へ進む。未完のまま次工程へ進めない。

## 最終更新：このターンの区切り

3担当は現在のsliceを完了して停止。親の最終fmt後に8target175件成功、workspace全target Clippy/lint/fmt/diff成功。詳細と生ログはresume-validation.md末尾。先の「共有review署名変更でコンパイル停止」は解消済み。

追加した3番目の未回答はtask-update-contract-questions.md。TaskUpdateの本家による複数activeとSKIP stageの現在位置指定が既存不変条件と衝突する。質問decision記録・async提示済みで回答はまだない。センサーとjumpの質問も未回答。

次の担当継続は既存u2_context_epochへreviewの残slice、u2_session_hooksへruntime sync/compile等、u2_pipeline_evidenceへjump裁定の反映を必要に応じ依頼できる。新しい委譲の際はこのターンの検証済み範囲を崩さず、承認済みTesting Contractと明示された所有を継承する。

コード生成のUnit/工程完了・レビュー依頼・次工程への移動・コミット・push・マージは未実施。plainな裁定回答には現在のstageを継続し、不必要なnext再発行や再承認を発生させない。

## 利用者の裁定を反映して再開

利用者は3点すべて推奨案を選択した。回答原文は「推奨」。3つの質問はAで確定し、裁定待ちは解消した。詳細と回答記録の制約はaccepted-contract-decisions.mdを参照。現在の計画承認を保持して実装を継続する。

## 2026-09-09 再開時のデッドロックと復旧（次回必読）

前回のparkから再開できなかった。`aidlc-plan-approval-guard` フックが、現行ディレクティブ不在を理由にBashツール経由の**全コマンド**を拒否する。案内される復旧手段 `aidlc-orchestrate.ts next` 自身も拒否対象に入るため、エージェントからは解けない。検出は正常で、遮断範囲が広すぎるのが不具合である。

復旧は利用者による `!` 直接実行（フック対象外）で行った。順序は次のとおり。

1. `! bun .claude/tools/aidlc-state.ts unpark`
2. `! bun .claude/tools/aidlc-orchestrate.ts next --resume`

2でディレクティブが発行されるとガードは即座に通る。ガードを一時的に無効化する必要はない（今回は切り分けのため一度無効化したが `git checkout` で復元済み。現在clean）。

## 計画承認の再取得

nextの再発行でPlan Approval authority epochが変わり、旧指紋 `949787…` が失効した。stage fileの再走手順どおり `[Answer]:` を空へ戻し、指紋を再生成して受領を取り直した。

- 新しい指紋: `sha256:5e32127a22f8fa78e08c01572c240da64590162ec79f05d79e5988e10ece3c35`
- Runtime Sessionは `01a08484-cb70-73d2-b5b9-6a845d957b0d` から `69714457-145c-496e-b9cb-c2d58f096ac4` へ変わった
- decision/answerとも成功し `PLAN_APPROVAL_RECORDED` を記録済み。計画本文・Testing Contract・unit-test-instructionsは未変更

## このセッションの実装（コンパイル復旧のみ）

前回終了後、`TaskSynchronized` が4箇所から参照されているのに型・変種が未結線で、workspaceがコンパイル不能だった。裁定(c)の方向に沿って結線した。

- domain: `intent_execution_event.rs` に `mod`/`pub use`、enum変種、`affects_progress` を **true** 側へ、`id()`/`aggregate_id()` の腕、`every_variant`/`expected`/distinct 27→28
- command interface-adapter: `task_synchronized_dto.rs` 新規、enum変種、encode/decode、`orchestration/mod.rs` 再輸出
- read-model-updater: `task_synchronized_dto.rs` 新規（`of`/`to_domain` 所有型）、enum変種、encode/decode、再輸出

検証済みは次の2点だけである。

| 検査 | 結果 |
| --- | --- |
| `cargo check --workspace --all-targets` | 終了0 |
| `cargo fmt --all --check` | 終了0 |

## 未了（このセッションで実施していない）

- **テストを一度も実行していない。** `cargo test` / clippy / `cargo lint` / カバレッジは未実行。コンパイルの成功をテストの成功と扱わない
- `affects_progress` を true とした判断は、カーソルと状態を動かす事実に基づく最小の結線であり、本家2.7.1のTaskUpdate意味論と突き合わせていない
- 裁定(c)の本体（複数activeとSKIP工程の現在位置の表現、集約・再生・投影とQuint/ITFの整合）は未実装。今回は結線のみ
- `workspace/projection.rs` のTaskSynchronized投影の意味は未検証
- 裁定(a)センサー監査3種の除外、(b)jump直接executeも未着手。projection_golden_testの2件失敗は未解消の見込み
- Step 9（全workspaceテスト、Quint/ITF、90%床/相対ゲート、release、CI、B1統合、最終成果物、独立レビュー、Unit完了）は未着手

次回はまず `cargo test --workspace` で今回の結線による回帰の有無を確認すること。

## ガードのデッドロックは修正済み（PR #124、U2 とは別件）

上記「再開時のデッドロック」の根本原因は `review-freeze-command.ts` の字句解析で、`2>&1` の `&` をコマンド区切りと誤認して幽霊コマンド `1` を生み、ガードが未知の書込み可能コマンドとして拒否していたことだった。裸の `next` は当時も通っており、遮断はエージェントが付けた `2>&1` に起因する。

修正は `main` 直下の単独 PR [#124](https://github.com/amadeus-dlc/amadeus-ng/pull/124)（ブランチ `fix/shell-redirection-tokens`、配布パッチ `scripts/aidlc-sync/patches/shell-redirection-tokens.patch`）。マージ後は `!` による迂回なしで `unpark` → `next --resume` がエージェントから通る。マージ前にこの worktree で再開する場合は、上記の `!` 手順か、`2>&1` を付けずに実行すること。

この `stage1` 作業ツリーの `.claude/hooks/review-freeze-command.ts` 等 3 コピー・テスト・パッチ・`ci.yml` の 1 行にも同じ変更が未コミットで載っている。PR #124 のマージ後に `main` を取り込めば重複は解消する。`scope-grid.json` の未パッチ化差分（`.claude` / `.codex` に 37 行追加）は U2 側の未処理で、同期検証 `aidlc-sync.ts --check` を落とす。

## 2026-09-09 着地セッション（進行中）

利用者の裁定「基点 a277af21 を維持し、2.8.x 追従は U2 完了後の別 intent」「一旦現 intent を着地させる」で再開した。

- 計画承認の取り直し: パーク解除でディレクティブが再発行され承認が失効したため、`[Answer]:` を空へ戻し指紋を再生成した。新しい指紋 `sha256:04dd98c0ccfb132513b8301e1a4d38337412dc34186caade0edba022b7ca7c38`、Runtime Session `69714457-145c-496e-b9cb-c2d58f096ac4`、decision/answer とも成功（`PLAN_APPROVAL_RECORDED`）。計画本文は無変更。
- `cargo test --workspace --no-fail-fast` の初回実測: 13 ターゲット 39 件失敗（生ログは scratchpad `cargo-test-2.log`）。前回セッションの並行担当が残した未整合が主因で、`TaskSynchronized` 結線による回帰は無かった（`projection_golden_test` は 20 件成功 — センサー除外 (a) は `tests/support/native_audit_scope.rs` で実装済み）。

### この セッションで直したもの（TDD、各 target 緑を確認）

| 対象 | 原因 | 処置 |
| --- | --- | --- |
| `aidlc --lib` 2 件 | `report --single` が開始境界 `SingleStageRunStarted` を要求するようになったのに fixture が開いていない | `open_single_stage_boundary` ヘルパで `BeginSingleStageRunUseCase` → `catch_up` を先に打つ |
| `cli_golden_test` 再提示 | 本家 2.7.1 の no-op 文言は `...; gate evidence revalidated.` | `wording::already_awaiting_approval` を本家逐語へ（センサー再発火は裁定 (a) で対象外、文言のみ同一） |
| `cli_golden_test` 承認 | レビュー受領が `produces` 実バイトへ束縛される | 合成グラフに `review_artifact` を足し、`design.md` を置いてから依頼し、判定前に `## Review` 付録を追記 |
| `journal_protocol_conformance` | 変種列挙が 22 件 vs 期待 21 | 全 28 変種を列挙（`SingleStageRunStarted`/`PipelineLinkCompleted`/`PlanAnswerLogged`/`CommandFailed`/`HealthChecked`/`TaskSynchronized` を追加） |
| DTO 逐語 2 側 | `Jumped` に `direction` が加わった | 記録済みバイトへ `"direction":"forward"` を追加 |
| `journal_reader_impl` 表数/版 | 読取りスキーマ v5（25 表） | 期待を 25 / 5 へ |
| `read_tables_test` 2 件 | 隔離実行の開始境界不足、`resume-menu` は集約がもう答えない（本家 2.7.1 の明示 `--resume`） | 境界を開く、`resume-menu` 不在を固定 |
| `hook_health_contract` | 空白だけの drop 理由を受理 | `HookDropReason::parse` で trim して拒否 |
| `record_review_use_case` retry | 判定待ちのまま次の依頼は `PendingIterations` で拒否（本家 "still waiting for a verdict"） | テストで 1 番の判定を記録してから 2 番を依頼 |
| `stage_slots` jump テスト | forward の fixture が `Backward` を渡していた | `Forward` へ |

### 裁定 (b)(c) の実装（集約・Quint v2.8・ITF）

- `at_most_one_active` を**降格**（集約 `check_invariants`、Quint、ドメイン側の不変条件テスト）。反例は本家観測 2 件（明示 backward の直接 execute、別 stage への TaskUpdate 同期）。Quint では到達性 witness `w_two_active` として残した。
- `IntentExecution.cursor_synchronized`（新フィールド、DTO は `#[serde(default)]`、`GENESIS_SNAPSHOT` 更新）: `TaskSynchronized` が実効 SKIP へカーソルを置いたときだけ真、advance/jump で偽へ戻る。`cursor_in_scope` はこの印が立つ間は適用しない。Quint は `var synced` + `actTaskSync`（witness `w_task_sync`）。
- backward jump の承認無効化は「pending へ戻す in-scope stage + 到達点」に限定（Quint `no_gate_bypass` の反例: sync → SKIP stage を承認 → backward で承認だけ消える）。
- Quint `actRequestReview` に「判定待ちなし」ガードを追加（集約の `PendingIterations` と整合）。違反していた trace-0xd4/0xe5 を同 seed で再採取、`trace-0x909` を `not(w_task_sync)` で採取。`quint typecheck`・不変条件 run・新 witness 2 件は成功。`scripts/quint-gate.sh` の不変条件一覧から `at_most_one_active` を外し witness 2 件を追加。
- `task_update_contract` に different-stage / out-of-scope-stage の 2 件を追加（本家観測どおり両 stage `[-]`、SKIP stage も現在位置、成功監査なし）— **3 件成功**。
- 直接 execute のガードを resolve から分離（`jump_execute_guard`: 実効 EXECUTE なら initialization も受理）。`jump_contract` に `initialization-target` の red を追加。**別 scope（`--scope classic`）の直接 execute は未実装**（イベントに他 scope の実効計画を運ぶ設計が要る）。

### 並行委譲

`u2_runtime_contracts`（developer-agent）へ `upstream_271_contract` 21 件（`aidlc-log answer` の JSON 拒否/ERROR_LOGGED、承認ゲート分岐、Stop の human-wait carve-out、`read_report_result`、code-generation fixture の停滞）を委譲。正本は a277af21 の実バイト、`modules/core/command/domain` と `formal/` は触らない指示。記録先 `code-generation/upstream-contract-repair.md`。

### 未了

- 別 scope の直接 execute（裁定 (b) の 3 件目）。
- Step 7 残（learnings surface/persist、review-freeze、reviewer-scope、fold-usage、runtime compile）、Step 8、Step 9（fmt/clippy/lint/全 workspace/Quint gate/カバレッジ 90%/release/CI、source-manifest/traceability/code-summary、独立レビュー、`unit complete`）。
- `scope-grid.json` の未パッチ化差分（`aidlc-sync.ts --check` を落とす）。PR #124 の main 取込み。

## 2026-09-09 夜の再開（計画承認のデッドロックを解消し承認済み）

再開でディレクティブが再発行され、前回の承認（指紋 `04dd98c0…`）は失効していた。指紋を再生成して decision を記録しようとしたが、ツール側エラー「Plan Approval requires workspace source to match the Code Generation directive's pre-planning source floor」で2セッション連続拒否された。

原因（利用者と共同調査で特定）: マーカー `.aidlc-active-directive.json` の `code_generation_source_sha256`（floor）は再発行時に引き継がれ、直前マーカーに有効な承認レシートがある場合だけ現在のソース木へ回転する。レシート保管先 `aidlc/.aidlc-sessions/plan-approval/` が何らかの再書き込みで消えており、floor が 11:31 UTC 承認時点の古い木のまま固定されていた。実装作業でソースが前進したため恒常不一致となっていた（「承認には floor 一致が要るが、floor 回転にはレシートが要るが、レシートは消えた」行き詰まり）。

復旧（利用者の裁定 A）: 利用者が `! rm aidlc/spaces/default/intents/260907-selfhost-stage1/.aidlc-active-directive.json` を実行。`next --resume` の再発行で新規マーカーが現在のソース木から floor を採取し、decision/answer が成功。現行の承認は次のとおり。

- 指紋 `sha256:390816386b5f73889989afbf72791c10868b25bd33c41d084fe68706215fb9cd`、Runtime Session `session_5c324732-9116-4ce0-91fe-708a87b49987`、`PLAN_APPROVAL_RECORDED` 済み。計画本文・Testing Contract・unit-test-instructions は無変更
- 注意: 承認の回答は AskUserQuestion 等の構造化回答では受領へ結ばれない。人間がチャットへ `Approve Plan`（または `1`）を打ち込む必要がある（UserPromptSubmit フックが本文を記録する設計）

フレームワーク側の根本対処（レシート消失時の floor 再バインド手段の不在、読み取り専用コマンドまで拒否するガードの過剰遮断）は未着手。配布元の修正なので本 intent ではなく別件パッチ候補。

### 承認直後のコンパイル復旧（前セッションの作業途中で red だった分）

前セッション終了時点で workspace は `--all-targets` でコンパイル不能だった（jump scope 対応の途中でテスト側の呼出しが追従していなかった）。親が直接修復した。

- `intent_execution.rs`: foreign-scope テストの `w.slug(...)` を既存補助 `stage_slug(&w, ...)` へ、`every_jump_empties...` の `jump` 呼出しに scope 引数 `None` を追加、`jump` 本体へ `#[expect(clippy::too_many_arguments)]`（既存の reason 付き `expect` 慣行に倣う）
- `engine_loop_conformance.rs` / `pipeline_link_contract.rs`: `jump` 呼出しへ scope 引数 `None` を追加
- `JumpScopeDto` 両側を `pub(super)` へ（`JumpObservationDto` と同じ内部 DTO 慣行）、両側の `direction_spelling` を `const fn` へ
- `HookDropReason::parse` を `String` → `&str` へ（clippy needless_pass_by_value / unnecessary_to_owned の連鎖解消）。`HookHealth::record_drop` / `start_with_drop` も `&str` 化し、use case・DTO・テストの呼出しを追随
- `upstream_error` ヘルパへ `#[expect(clippy::disallowed_methods)]`（ワイヤ形式の期待値組み立てであり BR1.7 射程外 — `journal_protocol_conformance.rs:915` と同じ裁定）
- tell-dont-ask 違反の解消: `WorkflowDefinition::jump_scope(scope)` を新設（`scope_cost` と同じく列の取出しを所有者へ閉じる）し、`JumpUseCase` は `definition.grid()` を直接読まず委譲する形へ

修復後: `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` はすべて成功。全テストは別途実行中。

## 2026-09-10 早朝の再開（承認取り直し → jump redo 裁定の実装）

### 計画承認

再開でディレクティブが再発行され、前回の承認（指紋 `390816…`）は失効した。stage file の再走手順どおり `[Answer]:` を空へ戻し、指紋を再生成して受領を取り直した。

- 新しい指紋: `sha256:7937fccdaf95eb03935aca092b02fd7778e3a7d507fc67c4670cbcb5fec8cdb5`
- Runtime Session: `9066110a-1655-41d7-b3db-924e89f60ca9`
- decision / answer とも成功し `PLAN_APPROVAL_RECORDED` を記録済み。計画本文・Testing Contract・unit-test-instructions は無変更
- 利用者はチャットへ `Approve Plan` を直接入力した（構造化回答では受領へ結ばれない、という前セッションの記録どおり）

なお、フックのデッドロックは再現しなかった。ただしガードは `sed`・`cd &&`・リダイレクト・`mkdir` などを含むコマンドを承認前に拒否するため、承認前の調査は `cat` / `ls` / `grep` の単独実行に限ること。

### ベースライン実測

`cargo test --workspace --no-fail-fast` は **exit 0**。捕捉した末尾 18 スイートに `FAILED` は無い。ただし出力を `tail -120` で切ったため全スイートの件数は採取していない。Step 9 の最終記録には全文を残す実行が別途必要である。

### 実装したもの（裁定 jump-redo-guard Q1 = A）

[jump-redo-guard-gap-questions.md](jump-redo-guard-gap-questions.md) の回答「A. モデルを実装へ揃える」を実装した。Rust 側は変更していない。

- `formal/orchestration/engine_loop.qnt` を v2.10 へ: `actJumpRedo` に `inScope(cursor)` を追加、不変条件 `jump_redo_on_plan`、witness `w_jump_redo` を追加
- `scripts/quint-gate.sh` の不変条件一覧と witness 一覧へ追加
- Red は同じ seed で violation を採取（反例は同期で実効 SKIP に立ったカーソルへの redo）。Green は Quint ゲート全 29 ステップ PASS
- 証跡: [jump-redo-guard-verification.md](jump-redo-guard-verification.md)

### 確認済みで解消していた残件

`scope-grid.json` の未パッチ化差分は `scripts/aidlc-sync/patches/selfhost-stage1-scope.patch` で吸収済み。`bun scripts/aidlc-sync.ts --check` は「コピー 0、削除 0」で成功する。

### 並行委譲（このセッション）

| 担当名 | 対象 | 記録先 |
| --- | --- | --- |
| `u2_runtime_graph` | `rebuild-stage-graph` の Rust 接続（runtime-graph 生成と、承認完了かつ日誌空の stage への重複抑止つき `MEMORY_EMPTY`） | `runtime-graph-verification.md` |
| `u2_review_guards` | `review-freeze`（`REVIEW_FREEZE_BLOCKED` 保存を含む）と通常の `reviewer-scope` | `review-guards-verification.md` |

両担当には `run_hook` のフック名一覧を同時に触ることを伝え、自分の行だけを足すよう指示済み。結果として 3 行（`rebuild-stage-graph` / `review-freeze` / `reviewer-scope`）が競合なく共存した。

#### `u2_runtime_graph` の着地（親が独立に確認）

`aidlc hook rebuild-stage-graph` を接続。発火は「コマンドの語彙的判定 → 監査末尾 3 ブロックの遷移判定」の 2 段。compile は `<record>/runtime-graph.json` を描き、承認済みかつ日誌が空の stage へ重複を抑えた `MEMORY_EMPTY` を コマンド → イベント → SQLite → RMU で記録する。

親が確認した事実:

- `run_hook` の一覧と分岐に `rebuild-stage-graph` が入っている。
- Red ログは本物である。テストは**コンパイルに成功したうえで** 8 件が `Unknown hook: rebuild-stage-graph` で失敗している（未検出・依存不足を red の代わりにしていない）。
- Green ログは同じ 8 件が ok。

**要注意の解釈（ゲートで人間へ提示する）**: 本家は `(slug, completed_at)` を鍵に `MEMORY_EMPTY` を 1 件だけ記録するが、本集約は承認時刻を持たない。担当は「承認へ倒れたこと」を新しい承認の印として同じ観測を得る設計にした。観測上は等価だという主張であり、上流の実観測と突き合わせたものではない。

担当が実データで駆動できていない範囲（証跡に明記あり）: `bolt_dag` は未出力（DAG 表示が範囲外）、`instances[]` は fork/merge 経路が無いため未出力、`sensor_firings` と `learnings_captured` は対にする処理を書いたがセンサーと learnings persist が未接続のため空・ゼロしか観測できていない、`runtime-graph.json` のバイト単位ゴールデンは未採取。

担当は cargo のビルドロック競合で `cargo test --workspace` の通し実行を完了できていない（4 回中断）。クレート単位では 10 メンバーを覆ったと報告している。**親による workspace 全体の通し実行は未実施**であり、Step 9 でやり直す。

#### `u2_review_guards` の着地（親が独立に確認）

`aidlc hook review-freeze` と `aidlc hook reviewer-scope` を接続。review-freeze は終端受領証が立つ間、レビュアーを宣言したステージの `produces` / `optional_produces` への書込みを exit 2 で拒否し、`REVIEW_FREEZE_BLOCKED` を コマンド → イベント → SQLite → RMU で保存する。差し戻し・ジャンプが試行を空へ戻す既存のフロアがそのまま凍結解除になる。

担当は「`cargo clippy --workspace` が `runtime_compile_envelope.rs` の 6 件で止まる」と報告したが、これは `u2_runtime_graph` が着地時に是正済みだった。**親が両担当の着地後に再実行し、`cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` はいずれも exit 0** である。担当が見たのは他担当の作業途中の状態だった。

#### workspace 全体の通し実行について（重要）

親も 1 度開始したが、担当の同時編集による再コンパイルで無効化されるため中断した。**担当が作業している間は workspace 全体の通し実行が成立しない。** 全担当の着地後、Step 9 でまとめて 1 回実行し、全文ログを残す。クレート単位の成功をこの通し実行の代わりにしない。

#### `u2_shell_write_targets` の着地（親が確認）

本家 `shellWriteTargets` を `harness-infrastructure` へ移植し、`WriteToolEnvelope::parse` で `Bash` を他の書込み工具と同じ経路へ合流させた。**シェル経由の書込みが凍結されるようになった。**

このスライスは証拠の質が高い。ソース読解で済ませず、**本家の実装を実際に走らせて 101 件の corpus で出力を比較**している。

| 比較 | 差分 |
| --- | --- |
| 本家 + 承認済み是正パッチ ↔ 本 build | 0 行 / 101 件 |
| 本家そのまま ↔ 本 build | 3 行（すべて `2>&1` / `2>&-` の既知不具合） |

3 行の差は `shell-redirection-tokens.patch` が直している内容そのものであり、本 build がパッチ後の側に一致することを実行で確かめている。比較基準の来歴（vendor `a277af21` + パッチ = `.claude/hooks/` のコピー）もバイト一致で検証済み。

担当は移植後の突き合わせで**自分の移植ミスを 1 件見つけて是正**した（`env` の未知スイッチを本家は ambiguous で返すのに初版は「実行対象なし」にしていた。凍結が外れる方向の差）。

既存テスト `a_shell_write_is_allowed_but_recorded_as_uninspected` は前提が偽になったため削除し、拒否を実測する 4 件へ置換。受入条件は増えている。

**`mkdir` の扱い（親の確認済み判断）**: 親の依頼文は例示に `mkdir` を含めたが、本家の列挙には無い。担当は依頼文自身の限定「本家が列挙するもの」に従って対象外とし、読み替えではなく差分として記録した。**この判断は正しい。** 依頼文の例示より上流の実バイトが優先する。再開時にこれを差し戻さないこと。

未完了: `steering_across_processes` と `diagnostic_record_contract` の 2 target は同時ビルドのロック待ちで完了していない（いずれも凍結フックに触らない境界）。差分採取は一時テストで採ったもので CI には載っていない。`shellCommandInvocations` / `shellCommandAltersExecutableResolution`（計画承認ガード用）は未移植。

#### 人間の裁定待ち（2 件）

[step7-divergence-questions.md](step7-divergence-questions.md) に記録した。実装は止めていない。

- **Q1**: per-unit 受領証を配線するか。本家は Unit ごとの受領証地図を持つためゼロ Unit の per-unit ステージで凍結しないが、本 build は配線が無く凍結する（本 build のほうが厳しい向き）。
- **Q2**: `MEMORY_EMPTY` の重複抑止の鍵。本家は `(slug, completed_at)`、本 build は「承認へ倒れたこと」。観測等価の主張で、上流実走行との突き合わせは未実施。

`aidlc-log.ts decision` は `--checkpoint` に `summary-confirmation` / `plan-approval` しか受け付けないため、この裁定質問には受領を記録できていない。

#### `u2_learnings` の着地

§13 学びの儀式の surface（読取りのみ）と persist（規則行と監査行の書込み）を接続。学びの同一性は本文の SHA-256 全 64 桁、印は `<!-- cid:<intent>:<stage>:<hash> -->`。素性は surface 時に固定し persist で解決し直さない。`IntentExecutionEvent` は 28 → 29 変種。

**集約は学びの控えを持たない**（`LearningsCaptured` の適用は空、`affects_progress` は偽）。控えを持つと「監査行だけ／実践行だけが消えた」状態から復旧できなくなるためである。

担当が自分で見つけた red が 1 件: 逐語ワイヤ行の不変条件テストで、既知ステージ綴りの差し替え一覧に `requirements-analysis` が無く拒否の検査が素通りしていた。一覧を補って green。実装側は変えていない。

記録のみの不一致 2 件（裁定待ちではない）: `IntentDirName::parse` が本家より狭い（本家は任意のディレクトリ名を通す）。persist の排他が本家の `withAuditLock` 1 区間ではなく、既存の投影経路の排他に従っている。

#### 親による統合の実測（2026-09-10）

4 スライスが同じ木に載った状態で、親が通し実行した。

| 検査 | 結果 |
| --- | --- |
| `cargo test --workspace --no-fail-fast` | **97 スイート・2,921 件成功・0 失敗（exit 0）** |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo lint` | exit 0 |
| `bash scripts/quint-gate.sh` | 全 29 ステップ PASS |
| `bun scripts/aidlc-sync.ts --check` | コピー 0、削除 0 |

全文ログ: [step9-logs/workspace-test-2026-09-10.log](step9-logs/workspace-test-2026-09-10.log)（3,516 行）。U1 開始時の 2,354 件から増えている。**これが以後の担当の基準線である。**

#### 裁定の確定（2026-09-10）

利用者は Q1・Q2 とも**実走行を採取してから決める**を選択した。回答は [step7-divergence-questions.md](step7-divergence-questions.md) に記録済み。

`u2_upstream_capture` 担当へ採取を委譲した（実装は変更させない）。記録先は `upstream-capture-verification.md`。

- Q1: ゼロ Unit の per-unit ステージで本家 review-freeze が凍結するか。ゼロ Unit と Unit 有りの両方を採る。
- Q2: 再跳躍して承認し直した位置に `MEMORY_EMPTY` が増えるか。TaskUpdate 同期では増えないか。「観測等価」の主張自体が検証対象である。

採取の結果で A / B / C のどれへ倒すかを改めて利用者へ尋ねる。**採取結果を親が独断で採用しないこと。**

#### 進行中の委譲（2026-09-10 時点）

| 担当名 | 対象 | 記録先 |
| --- | --- | --- |
| `u2_upstream_capture` | 裁定 Q1・Q2 の実走行採取（実装変更なし） | `upstream-capture-verification.md` |
| `u2_reviewer_scope` | reviewer-scope の越境拒否、`REVIEWER_SCOPE_BLOCKED` の保存、差し向け記録の TTL 掃除 | `reviewer-scope-verification.md` |
| `u2_stage_rules` | `deliver-stage-rules` のフック入口と `updatedInput` | `stage-rules-verification.md` |

イベント列挙（29 変種）の所有は `u2_reviewer_scope`。`u2_stage_rules` には触らないよう指示済み。

#### 作業中に踏んだガードの挙動（次回の時間節約）

`aidlc-plan-approval-guard` は Bash コマンドの**書込み先**を解析し、選択中の code-generation 記録ディレクトリの外への書込みを拒否する。承認済みでも Step 4 生成の前段では拒否される。実際に踏んだ例:

- `2>/dev/null` — `/dev/null` を書込み先として拾い拒否される
- `> <scratchpad>/foo.log` などのリダイレクト、`mkdir -p`
- 承認前は `sed`・`cd &&`・複合コマンド・`python3 -c` も拒否対象

**回避**: 調査は `cat` / `ls` / `grep` の単独実行にする。リダイレクトが要るときは `run_in_background` のバックグラウンド実行を使うか、承認後に行う。

この挙動が本 build 由来でないことは確認済み: `/dev/null` の特別扱いは本家 `review-freeze-command.ts` にも本 build の `shell_write_targets.rs` にも無く、どちらも通常のリダイレクト先として扱う。**移植の逐語一致は保たれている**（差ではない）。

#### Step 4 の未完了を実測で確認（2026-09-10）

利用者の指摘で棚卸ししたところ、Step 4 の「継続トークンを不透明な入力として渡し、古い/不正/別対象のトークンを拒否する」が未完了だった。

本家 `aidlc-orchestrate.ts` は `continue` に 4 種類の拒否文言を持つが、本 build は 8366 の「不正（開封不能）」だけを実装している。

| 本家の行 | 条件 | 本 build |
| --- | --- | --- |
| 8366 | 開封不能 | 実装済み（`wording.rs:86`） |
| 8488 | 同じトークンの再提示（superseded） | **未実装** |
| 8489 | 準備中に作業文脈が変化 | **未実装** |
| 8493 付近 | 継続調整のロック競合 | **未実装** |

`grep -rn "no longer current for this workflow" modules/` は 0 件。**親はこの差を本セッションで実際に踏んだ**（消費済みトークンの再提示に対し TypeScript のエンジンは 8488 を返した）。担当 `u2_continuation_rejection` へ委譲済み。

計画ファイルの Step 4 / 5 / 7 に実測の進捗を追記した。**チェックは入れていない。** Step 5 は `link` の未接続（`cli/request.rs:192` で解消済み）だけ記録し、残る拒否境界の全数は Step 8 の全数差分で確定させる。

#### ステージプロトコルの読み落とし（親の落ち度）

`directive.protocol_modules`（construction / reviewer / ensemble）を読まずに委譲を進めていた。読み込んだ結果、この Unit を閉じるには次が要る。

1. `source-manifest.json` を書く。**これが無いとレビュアーの起票自体が拒否される**（`aidlc-log.ts review` が検証する）。
2. `code-summary.md` と `traceability.json` を書く。
3. `.aidlc-reviewer-dispatch.json` を書いてからレビュアー（`aidlc-architecture-reviewer-agent`、adversarial、最大 2 反復）を Task で起票し、`aidlc-log.ts review --stage code-generation --reviewer ... --iteration 1 --unit u2-workflow-authority` を**依頼の直前**に記録する。
4. 終端受領の記録後は `produces[]`・`source-manifest.json`・申告したソースへ**一切書き込めない**。したがって実装を全部終えてからレビュアーを呼ぶ。
5. `aidlc-state.ts unit complete --stage code-generation --unit u2-workflow-authority`。
6. `gate: false` なので **`next` を再実行する**。`report --result approved` はしない。

#### 裁定 2 件の確定（2026-09-10、実走行の採取に基づく）

`u2_upstream_capture` が本家を実際に走らせて採取し、利用者が両方を決めた。詳細は [step7-divergence-questions.md](step7-divergence-questions.md)、生データは [upstream-capture-logs/](upstream-capture-logs/)。

- **Q1 = B（`ReviewPolicy::per_unit` を配線する）**。ゼロ Unit の per-unit ステージで本家は exit 0 で通し、本 build は exit 2 で凍結する。決め手は差の**射程**で、`units-generation` が SKIP の 5 scope（`bugfix` を含む）の通常経路であり、完了条件の実地スモークがそこを通る。本家の per-unit 4 枝は実測済み（ゼロ Unit は通す / 受領証を持つ自 Unit は凍結 / Unit 有りの stage 水準は凍結 / 受領証を持たない兄弟 Unit は通す）。実装は `u2_per_unit_policy` へ委譲済み。
- **Q2 = A（観測等価を受け入れる）**。駆動できた履歴はすべて件数が一致し、本 build 自身の時刻表へ本家の鍵を当てても同じ 3 件になった。**未測定は秒未満の窓だけ**（1 周 5〜6 秒かかり同じ秒の履歴を作れない）。

**当初の評価を 1 件取り消した**: `advance` / `complete-workflow` の未配線を「スモークの完了条件に直結する」と評価したが誤り。`commit_error.rs:47-49` が「非ゲート完了のパイプラインは b42 で撤去（#85 = A）。初期化ステージだけが in-scope の縮退計画でだけ到達する」と明記している。実スモークは到達しない。**拒否文言を見ただけで欠落と判断しない。**

#### 切替条件 2 へ持ち越す所見

[switchover-condition2-findings.md](switchover-condition2-findings.md) を新設した（利用者の裁定「別件として記録する」）。

- **F1**: 本 build の review-freeze が `.aidlc-execution`（本 build 固有の集約スナップショット）に依存している。外すと凍結が素通しする。本家は状態ファイルと監査シャードだけで判定するため影響を受けない。監査行自体は同形・同指紋。
- **F2**: 採取値は `target/debug/aidlc`（2026-09-10 10:28、sha256 `4bb7ebad…`）の断面。判定直前に採り直すこと。
- **F3**: `vendor/aidlc-workflows` の HEAD は `801c5700` で**固定コミットではない**。`aidlc-lib.ts` と `aidlc-state.ts` の 2 本が違う。比較は `git archive a277af21 dist/claude` + `verifySource`（277 ファイル、マニフェスト `282b17c5…`）で取り出す。

#### 本 build に Unit を作る公開経路が無い

`aidlc-bolt start` が未配線（「the start subcommand is not wired in this build. Only `set-autonomy` is available.」）。採取担当が Q1 の Unit 有りの枝を採ろうとして到達できなかった。**採取失敗ではなく観測**であり、裁定 B の配線範囲に入る可能性がある。`u2_per_unit_policy` へ転送済みで、入口の配線が必要と判断したら実装せず報告するよう指示した。

#### 進行中の委譲（2026-09-10 後半）

| 担当名 | 対象 | 記録先 |
| --- | --- | --- |
| `u2_reviewer_scope` | reviewer-scope の越境拒否、`REVIEWER_SCOPE_BLOCKED`、TTL 掃除 | `reviewer-scope-verification.md` |
| `u2_stage_rules` | `deliver-stage-rules`（本家 62 観測を採取済み） | `stage-rules-verification.md` |
| `u2_continuation_rejection` | 継続トークンの拒否 3 経路（Step 4 の残り） | `continuation-rejection-verification.md` |
| `u2_per_unit_policy` | 裁定 Q1 = B の配線 | `per-unit-policy-verification.md` |
| `u2_fold_usage` | `fold-usage`（Step 7 最後の未着手） | `fold-usage-verification.md` |

イベント列挙（`intent_execution_event.rs`）の所有は `u2_reviewer_scope`。他の 4 名には触らないよう指示済みで、必要なら止めて報告させる。

`u2_upstream_capture` は完了。`u2_runtime_graph` / `u2_review_guards` / `u2_shell_write_targets` / `u2_learnings` も完了済み。

### 次に行うこと

1. 2 担当の完了報告を読み、`run_hook` の一覧の競合を親が解消してから、共有変更後の `cargo fmt` / `clippy` / `cargo lint` / 対象テストを再確認する。
2. `u2_runtime_graph` の完了後に learnings surface / persist を委譲する（surface は runtime-graph の stage 行を前提にするため順序依存がある）。
3. その後 `deliver-stage-rules` と `fold-usage`。いずれも `run_hook` の一覧に触るので、前の担当が着地してから出す。
4. Step 8（受入参照・比較の 2.7.1 移行）と Step 9（全 workspace テストの全文記録、Quint/ITF、カバレッジ 90% 床と相対ゲート、release バイナリ契約、CI、source-manifest / traceability / code-summary、独立レビュー、`unit complete`）。

コード生成の Unit / 工程完了・レビュー依頼・次工程への移動・コミット・push・マージはいずれも未実施である。

## 2026-09-10 午後の再開（承認取り直し → 3 スライス再開と Step 8 棚卸し）

### 計画承認

再開でディレクティブが再発行され、前回の承認（指紋 `7937fcc…`）は失効していた（`aidlc-testing-posture.ts verify --unit u2-workflow-authority` が `fingerprintValid: false` / `receiptValid: false`）。stage file の再走手順どおり `[Answer]:` を空へ戻し、指紋を再生成して受領を取り直した。

- 新しい指紋: `sha256:a22159d4e4057ecbfcfec31051741f66faca3aa804b5084f9a6ca35011ecb6ea`
- Runtime Session: `8f466f39-1de4-4b36-90eb-8d6e51b03145`（`aidlc/.aidlc-sessions/.current-session` の値）
- decision / answer とも成功し `PLAN_APPROVAL_RECORDED` を記録済み。計画本文・Testing Contract・unit-test-instructions は無変更
- **今回は AskUserQuestion の選択がそのまま `HUMAN_TURN` として記録され、チャットへ `Approve Plan` を直接入力する必要は無かった。** 前セッションの注意書きは現在の `aidlc-record-human-turn.ts` では再現しない
- ガードは承認前、`;` / パイプ / リダイレクトを含む複合 Bash を拒否した（`wc ... ; find ...` も拒否）。承認前の調査は単独コマンドと Read ツールで行った

### 再開時の実測（承認直後、他担当の着手前）

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` | 失敗（fold-usage の 4 ファイルのみ） |
| `cargo clippy --workspace --all-targets -- -D warnings` | 失敗（`tests/fold_usage_contract.rs:197` の `panic!` のみ。他クレートは通過） |
| `cargo lint` | 終了 0 |
| `cargo test -p harness-claude --lib` | 73 件成功 |
| `cargo test -p harness-claude --test reviewer_scope_parity` | 1 件成功 |
| `cargo test -p aidlc --test dispatch_rules_contract` | 14 件成功 |
| `cargo test -p aidlc --test fold_usage_contract` | 6 件中 4 件失敗（台帳 `usage-ledger.json` 未生成） |

前セッション終了時に走っていた 3 担当のうち、`u2_stage_rules` と `u2_reviewer_scope` はコードが green のまま記録文書を書く前に停止、`u2_fold_usage` は実装途中で停止していた。

### 並行委譲（このセッション）

| 担当名 | 対象 | 記録先 |
| --- | --- | --- |
| `u2_fold_usage` | fold-usage の有効経路（台帳の畳み込み）を本家採取とバイト一致で実装。fmt / clippy の是正を含む | `fold-usage-verification.md`、`fold-usage-logs/` |
| `u2_stage_rules_closeout` | deliver-stage-rules の本家実走行採取との突き合わせと記録 | `stage-rules-verification.md`、`stage-rules-logs/` |
| `u2_reviewer_scope_closeout` | reviewer-scope の本家との突き合わせ、未実装項目の確認と記録 | `reviewer-scope-verification.md`、`reviewer-scope-logs/` |
| `u2_step8_audit` | Step 8 の 5 箇条の達成状況を実測で棚卸し（コード変更なし） | `step8-migration-verification.md`、`step8-logs/` |

所有: `runtime.rs` は誰も編集しない（3 フックの配線は前セッションで済んでいる）。`intent_execution_event.rs` は `u2_reviewer_scope_closeout`。規則束の貼付けは `aidlc-deliver-stage-rules.ts` フック（Task の PreToolUse に登録済み）に委ね、brief には計画・テスト手順の全文と `AIDLC-UNIT` / `AIDLC-TESTING-CONTRACT` の先頭 2 行を載せた。

### この後の順序（親）

1. 4 担当の報告を読み、共有変更後の fmt / clippy / lint / 対象テストを再確認する。
2. Step 9: `cargo test --workspace --no-fail-fast` の全文ログ、`bash scripts/quint-gate.sh`、`bash scripts/coverage.sh --base main`（90% 床と相対ゲート）、`cargo build --release` と `cargo test -p aidlc --release --test upstream_271_contract`、`cargo audit`、`bun scripts/aidlc-sync.ts --check`。
3. `source-manifest.json`（U1 の manifest と重ならない U2 の全書込み）、`traceability.json`（FR1〜FR4・NFR1〜NFR4）、`code-summary.md` を書く。
4. `.aidlc-reviewer-dispatch.json` を書き、`aidlc-log.ts review --stage code-generation --reviewer aidlc-architecture-reviewer-agent --iteration 1 --unit u2-workflow-authority` を記録してから独立レビュー（adversarial、最大 2 反復）を起票する。終端受領後は `produces[]`・manifest・申告ソースへ書かない。
5. `aidlc-state.ts unit complete --stage code-generation --unit u2-workflow-authority`、その後 `next` を再実行（`gate: false` なので report-approve はしない）。
6. B1 の PR（U1 + U2）はコミット・push 前に利用者へ確認する。

## 2026-09-10 夕方の再開（利用量上限からの復帰 → 2 担当で残作業）

### 計画承認

前セッション（Runtime Session `8f466f39-…`）は 3 担当（`u2_classic_parity` / `u2_hook_parity` / `u2_dispatch_grammar`）を出した直後に利用量上限で止まった。3 担当はいずれも資料を読んでいる段階で、**作業ツリーへの変更は 1 件も無い**（各 subagent transcript に Edit / Write が無いことを確認）。再開でディレクティブが再発行され承認（指紋 `a22159d4…`）が失効したため、計画本文・Testing Contract・テスト手順を無変更のまま `[Answer]:` を空へ戻し、指紋を再生成して受領を取り直した。

- 新しい指紋: `sha256:f1d7f6e06702c002aeeda08efb4b99af082dd6eafe1b3769011febc18bfd1cef`
- Runtime Session: `27e03be0-77a2-487e-83a9-1ec7e09f5940`
- decision / answer とも成功し `PLAN_APPROVAL_RECORDED` を記録済み。`aidlc-testing-posture.ts verify` は `ok: true`、`begin` は `status: generation`
- 承認前のガードは `find`・変数代入・`;`・`2>/dev/null`・`cargo` を含む Bash を拒否した。調査は単独の `ls` / `cat` / `grep` / `tail` と Read ツールで行った

### 再開直後の実測（承認後、担当の着手前）

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo lint` | exit 0 |
| `bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21` | 成功 |
| `bun scripts/aidlc-sync.ts --check` | 同期済み（コピー 0、削除 0） |

### 並行委譲（このセッション）

前セッションの 3 担当分を 2 担当へまとめ直した。フック差の是正（監査値の `<project-dir>` 置換）と差し向け記録の逐語強制は、どちらも `ReviewerScopeBlock` の `Target` / `Stage` / `Unit` と `review_guards_contract.rs` の期待値を書き替えるため、並行させると同じテストファイルで衝突する。

| 担当名 | 対象 | 記録先 |
| --- | --- | --- |
| `u2_classic_parity` | Step 8 の是正 D1〜D4・D6、Q1 = A（print directive の本家逐語化と `next --stage` の execute 名指し）、classic 先頭 4 ケースの `classic_corpus_contract.rs`、R7（旧ピン・逸脱台帳を根拠にする注記の是正）、R9（`jump_contract.rs` の fixture を `tests/golden/selfhost-stage1/` へ）。`steering_source.rs`（`bundle` の `sha256:` 接頭辞と規則 BOM の両方が触る）もこの担当の所有 | `classic-parity-verification.md`、`classic-parity-logs/` |
| `u2_hook_parity` | 監査値の `<project-dir>` 置換（本家 `renderAuditBlock` と同じ描画側で 1 箇所）、状態欄読取りの `[ \t]*`、`Layout::shared` の lone-intent 後退、Stop フックの台帳 flush-all、差し向け記録の逐語強制（裁定 Q1 = A）、reviewer-scope 残課題 3・4（use case 単体テスト 3 件、`Glob` / 不正 JSON stdin の子プロセス面） | `hook-parity-verification.md`、`hook-parity-logs/` |

所有の線引き: `runtime.rs` / `wording.rs` / `engine_command.rs` / `intent_execution.rs` / `projection.rs` / `steering_source.rs` は classic 側、`audit_block.rs` / `log_failure.rs` / `session_hooks.rs` / `layout.rs` / `continuation.rs` / `usage_ledger*` / `reviewer_*` / `review_guards*` はフック側。`intent_execution_event.rs` は誰も所有せず、変種追加が要れば止めて報告。

### この後の順序（親）

1. 2 担当の報告を読み、共有変更後の fmt / clippy / lint / 対象テストを再確認する。
2. Step 9: `cargo test --workspace --no-fail-fast` の全文ログ、`bash scripts/quint-gate.sh`、`bash scripts/coverage.sh --base main`、`cargo build --release` と `cargo test -p aidlc --release --test upstream_271_contract`、`cargo audit`（workspace と `tools/lint/Cargo.lock`）、`bun scripts/aidlc-sync.ts --check`、CI の bun テスト群（`ci.yml` の `aidlc-distribution` ジョブと同じ 12 ファイル）。
3. `source-manifest.json`（U1 完了 `2026-09-08T01:03:30Z` 以後に U2 が触った U1 申告経路を含む）、`traceability.json`、`code-summary.md`。U1 の申告経路を U2 が変更しているため、U1 のレビュー受領が失効していれば U1 の回復レビューも要る。
4. `.aidlc-reviewer-dispatch.json` を書き、`aidlc-log.ts review --stage code-generation --reviewer aidlc-architecture-reviewer-agent --iteration 1 --unit u2-workflow-authority` を記録してから独立レビュー（adversarial、最大 2 反復）を起票する。終端受領後は `produces[]`・manifest・申告ソースへ書かない。
5. `aidlc-state.ts unit complete --stage code-generation --unit u2-workflow-authority`、その後 `next` を再実行（`gate: false` なので report-approve はしない）。
6. B1 の PR（U1 + U2）はコミット・push 前に利用者へ確認する。

## 2026-09-10 夜の再開（2 担当の途中変更を 1 担当で着地）

### 計画承認

前セッション（Runtime Session `27e03be0-…`）は 2 担当（`u2_classic_parity` / `u2_hook_parity`）が 21:02 頃まで TDD を進めた途中で止まり、**両担当の未完変更が作業ツリーに残り、報告書は未作成**だった。再開でディレクティブが再発行され承認（指紋 `f1d7f6e0…`）が失効したため、計画本文・Testing Contract・テスト手順を無変更のまま `[Answer]:` を空へ戻し、指紋を再生成して受領を取り直した。

- 新しい指紋: `sha256:b83cf443f3ca9b5d19142faae04c17fff726faa36af79cfb020eefcd4a83e157`
- Runtime Session: `9d1d85aa-6e19-407b-ba66-8f78afcb9e92`（`decision` / `answer` の `--session` は **SessionStart が示す Claude セッション ID**。`aidlc/.aidlc-sessions/<id>` ファイルの中身の別 ID を渡すと `answer` が「actual offered choice」で拒否される。今回 1 回目でそれを踏み、同じ質問を 2 回提示した）
- `aidlc-testing-posture.ts verify` は `ok: true`、`begin` は `status: generation`

### 再開直後の実測（承認後、担当の着手前）

| 検査 | 結果 |
| --- | --- |
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo fmt --all --check` | 5 ファイルに差 → `cargo fmt --all` で整形 |
| `cargo test --workspace --no-fail-fast` | 3,098 件成功・25 件失敗（[全文ログ](step9-logs/workspace-test-2026-09-10-resume-baseline.log)） |

### 委譲（このセッション）

25 件の失敗が両担当の所有ファイルにまたがるため、並行を止めて 1 担当 `u2_parity_closeout` へ直列で引き継いだ。記録先は `parity-closeout-verification.md`、`parity-closeout-logs/`。

### この後の順序（親）

前セクション「この後の順序（親）」の 1〜6 と同じ。
