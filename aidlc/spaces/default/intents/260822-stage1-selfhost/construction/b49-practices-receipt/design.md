# b49 設計 — practices-discovery の受領証（`PRACTICES_AFFIRMED`）を `IntentExecution` のイベントで、`aidlc-state practices-promote` 動詞、approve の段 12 ガード（2026-09-05）

対象: #7 キュー 5 の残り（段 12 — b48 の裁定で分割）。前段: [`b48-review-receipts/design.md`](../b48-review-receipts/design.md)。
ピン: upstream `3c3146cf`（`core/tools/aidlc-state.ts` `handlePracticesPromote` `:3477-3770`、`handlePracticesEvent` `:3407-3475`、`aidlc-orchestrate.ts` `hasFreshPracticesAffirmationReceipt` `:4755-4826` と `handleReport` の段 12 `:5768-5782`、`aidlc-lib.ts` `extractMarkdownSection` `:10105` / `appendUnderHeading` `:10150` / `replaceSection` `:10175` / `setOrInsertField` `:6599`、`audit-format.md` `:220`）。

## 0. 裁定（オーナー、着手前に質問 2026-09-05）

| 問い | 裁定 |
| --- | --- |
| team.md / project.md への書き込みを誰が担うか | **A: RMU が投影する**。合成ルート（CLI）がドラフト 2 本と contributions を読んで材料として渡し、集約 `IntentExecution` が受理して昇格内容（5 節の本文・印付き規則行・承認者）をイベント `PracticesAffirmed` に載せる。RMU が状態ファイルと同じ要領で team.md / project.md・監査行・タイムスタンプを描く |
| 昇格に失敗したときの監査行 `PRACTICES_OVERRIDE` | **A: 描かない**。拒否は stderr + exit 1 だけ（b48 の `ERROR_LOGGED` 非描画と同じ扱い、逸脱台帳） |
| 兄弟動詞 `practices-event`（PRACTICES_DISCOVERED / OVERRIDE / SECTION_EMPTY） | **A: 射程外**。新しい面 `aidlc-state` は `practices-promote` だけを配線し、他の動詞は not-wired 拒否 |

用語（初見向け）: **昇格（promote）** = Practices Discovery で人間が承認した「チームの決めごと」のドラフト 2 本（`team-practices.md` / `discovered-rules.md`）を、メモリ層の正本 `team.md`（5 節を置換）と `project.md`（`## Mandated` / `## Forbidden` に `(affirmed YYYY-MM-DD)` 印の規則を追記）へ書き写すこと。**受領証** = 昇格が成功した事実の記録（監査行 `PRACTICES_AFFIRMED` と状態ファイルの `Practices Affirmed Timestamp` の対）。**試行** = そのステージの直近の開始・差し戻し・ジャンプ以降の区間（b48 と同じ）。**段 12** = 「practices-discovery の承認は現在の試行の受領証を要する」という report のガード。

## 1. 原則からの導出

- **コマンド側 = 集約と判断**。「昇格した」は実行の事実なので `IntentExecution` のイベント。鮮度（試行の区切り）は b48 と同じく集約の状態遷移が決めるので、段 12 の判断も集約のガード（`approve_gate`）に閉じる。upstream は状態ファイルのタイムスタンプと監査行の 2 部受領証を突き合わせるが、我々では集約の状態が正本であり、状態ファイル・監査行はその投影である（偽造の余地は upstream より狭い）。
- **昇格の内容の計算は純粋関数**（ドラフト 2 本 × 正本 2 本 × 日付 → 置換する節・追記する印付き行）。これはワークスペースの Markdown 機構（仕様 11）であって業務判断ではないので、domain の `workspace` モジュールに値オブジェクト `PracticesPromotion` として置き、合成ルートがファイルを読んで組み、集約は受け取った値をイベントに載せる（材料は引数、判断は集約）。
- **RMU は 4 面を描く**: team.md / project.md（新しい面 — 状態ファイルと同じ「置換」面）、状態ファイル（`Practices Affirmed Timestamp` / `Last Updated`）、監査行 `PRACTICES_AFFIRMED`。読み面 `read_*` には消費者が無いので列を足さない。
- **upstream の書込順（project.md → team.md → 監査 + 状態）を投影の永続化順に写す**。at-least-once の再実行は置換・重複除去で冪等。
- **`aidlc-state practices-promote` は新しい面 `Face::State`**。合成ルートは構文段（フラグ・ファイルの存在と形・contributions の identity marker）と逐語だけを持つ。
- **繰延（記録して進む）**: `--target-dir`（テスト用の書込先差替え — not-wired 拒否）、`practices-event` 3 動詞、失敗時の `PRACTICES_OVERRIDE`、approve 時の revision backstop（`unrecordedRevisionSinceGateOpen` — 成果物フック由来、CP5）、`aidlc-state` の他 21 動詞。

## 2. ドメイン（`modules/core/command/domain`）

### 2.1 `workflow_definition::PRACTICES_DISCOVERY_SLUG`（定数）

`"practices-discovery"`。upstream は 3 ツールとも slug をリテラルで持つ（`findStageBySlug("practices-discovery")` / `slug === "practices-discovery"`）。定義集約のクエリにはしない — 定義は「どのステージが practices か」を宣言しないからである。

### 2.2 `workspace::MarkdownSections`（純粋関数、新規 `markdown_sections.rs`）

upstream `aidlc-lib.ts` の 3 関数の写し（`## ` 見出しの行頭一致・末尾空白許容・`### ` は一致しない・最初の一致が勝つ・fenced code block 内の見出しは無視（extract のみ））:

- `extract_section(content, heading) -> Option<String>`（`extractMarkdownSection` — 見出し不在は `None`。upstream の `""` は「不在」と「空節」を畳むが、呼出側の分岐は `=== ""` なので `None` と空文字を同じに扱えばよい）。
- `replace_section(content, heading, body) -> Result<String, HeadingNotFound>`。
- `append_under_heading(content, heading, text) -> Result<String, HeadingNotFound>`。

RMU の `state_writers::with_field_or_insert`（既存 `HeadingNotFound` を持つ）はこの `append_under_heading` へ寄せる（同じ意味論なので重複実装を残さない）。

### 2.3 `workspace::PracticesPromotion`（値オブジェクト、新規 `practices_promotion.rs`）

`{ sections: Vec<PromotedSection>, mandated: Vec<String>, forbidden: Vec<String> }`（`PromotedSection { heading: String, body: String }` は同ファイルか 1 型 1 ファイル。`heading` は `## ` を除いた名前 `Way of Working` 等）。

- `PracticesPromotion::plan(team_practices_draft, discovered_rules_draft, team_md, project_md, today: NaiveDate) -> Result<PracticesPromotion, PromotionPlanError>`（upstream Step 4a / 4b の写し）:
  - 5 節 `Way of Working` / `Walking Skeleton` / `Testing Posture` / `Deployment` / `Code Style` を順に見て、ドラフトに節が在れば（`extract_section` が `Some` かつ空でない）その本文を採る。**正本 team.md にその見出しが無ければ `Err(TeamHeadingMissing(heading))`**（upstream `replaceSection failed on team.md for "<heading>"`）。
  - 規則はドラフトの `## Mandated` / `## Forbidden` 節を行ごとに trim し、空行・`<!--` 始まり・`#` 始まりを捨てる。各行を `<rule> (affirmed <today>)` に印し、**正本 project.md に同じ印付き行（trim 一致）が既に在れば除く**（重複除去）。同じ実行内の重複も 1 つにする。正本に `## Mandated` / `## Forbidden` が無ければ `Err(ProjectHeadingMissing(heading))`（規則が 1 つも無い見出しは検査しない — upstream も append を呼ばないので throw しない）。
  - 節も規則も空の昇格は**受理する**（upstream は `Sections Written: `（空）と 0 / 0 で `PRACTICES_AFFIRMED` を emit する）。
- クエリ: `sections_written() -> Vec<&str>`（見出し名の列）、`mandated_appended() / forbidden_appended() -> u32`。

### 2.4 `IntentExecution` の状態・コマンド・適用

- 状態に `practices_affirmed: Vec<bool>`（計画と同じ長さ。genesis は全て false。真になるのは practices ステージの添字だけだが、フロアを `approved` / `review_attempts` と同じ形で扱うため計画長で持つ）。スナップショット DTO・完全コンストラクタ（長さ検査）・再構成に載せる。
- クエリ `practices_stage(&self) -> Option<StageIndex>`（`stage_keys` から `PRACTICES_DISCOVERY_SLUG` の位置）、`practices_affirmed(&self, stage) -> Option<bool>`。
- **フロア（適用側で false に戻す）**: b48 のレビュー試行と**同じ 4 か所** — `advance_from` で立った次ステージ（forward / skipped）、`GateRejected(s)` の s、`Jumped` は全ステージ。`StageRevised` はフロアでは**ない**（upstream の FLOOR は `STAGE_STARTED` / `GATE_REJECTED` / `STAGE_REVISING` だが、`STAGE_REVISING` を emit するのは `reject`（`GATE_REJECTED` と対）と approve 時の revision backstop だけで、`revise` は `STAGE_AWAITING_APPROVAL` を emit する — `:3001` / `:2986` / `:2760`。我々の `GateRejected` がその対を 1 イベントで表す）。
- **`affirm_practices(&mut self, intent: &Intent, promotion: &PracticesPromotion, affirming_user: &str, occurred_at) -> Result<IntentExecutionEvent, CommandError>`**:
  1. 取り違え → `IntentMismatch`。
  2. 計画に practices ステージが無い → `UnknownStage("practices-discovery")`（b48 の変種を再利用）。
  3. 本流の状態（Running / Parked / Completed / autonomous）は**見ない** — upstream の `practices-promote` に status ガードは無い（`aidlc-log review` と同じ受理集合）。
  - 受理 → `PracticesAffirmed`。適用: `practices_affirmed[stage] = true`（再昇格は上書き — upstream も何度でも emit する）。
- **`approve_gate`（段 12）**: `require_checkbox` の後・`require_review_receipt`（段 11）の**前**に `require_practices_receipt(stage)` — `stage == practices_stage() && !practices_affirmed[stage]` → `PracticesReceiptMissing(StageIndex)`。順序は upstream どおり: orchestrate の段 12（`:5772`）が `aidlc-state approve`（`verifyReviewerPrecondition`）を spawn する前に効く。
- イベント 1 変種（新規 `intent_execution_event/practices_affirmed.rs`、`id` + `aggregate_id`）: `PracticesAffirmed { stage: StageSlug, affirming_user: String, sections: Vec<PromotedSection>, mandated: Vec<String>, forbidden: Vec<String> }`（`mandated` / `forbidden` は**印付きの行**）。C5 のイベント表を 16 変種へ。
- `CommandError` 新変種 1: `PracticesReceiptMissing(StageIndex)`（Display は材料だけ）。

## 3. Quint（`formal/orchestration/engine_loop.qnt`、v2.6）

状態変数: `practicesStage: int`（init で `STAGES.union(Set(-1)).oneOf()` から選び全アクションで凍結。`-1` = 計画に無い。stage 0 は非ゲートなので選ばない）、`affirmed: bool`、スナップショット `prevAffirmed`。

抽象化の対応（ヘッダに表を書く）: `practicesStage` ↔ `IntentExecution::practices_stage()`、`affirmed` ↔ `practices_affirmed[practicesStage]`（他のステージは常に false なので 1 bit に射影）。

アクション:
- `actPromotePractices`: `practicesStage != -1`；`affirmed' = true`、他は全変数フレーム、`lastAction' = "promote_practices"`。
- `actReportForward` に**ガード** `(s == practicesStage) implies affirmed` を足す。フロア: `nxt == practicesStage` なら `affirmed' = false`（`actReportSkipped` も同じ）；`actReject` は `cursor == practicesStage` なら false；`actJumpForward` / `actJumpBackward` / `actJumpRedo` は false；`actRevise` は触らない；他はフレーム。
- `snapshot` に `prevAffirmed`、`step` に 1 アクション。

不変条件 3 本（各 1 つの mutation で検出力を証明し §9 に記録する）:

| 名前 | 内容 | 検出する mutation |
| --- | --- | --- |
| `approve_requires_practices_receipt` | `(lastAction == "report_forward" and prevCursor == practicesStage) implies prevAffirmed` | `actReportForward` の受領証ガードを外す |
| `practices_receipt_floor` | 差し戻し（cursor == practicesStage）→ `not(affirmed)`、ジャンプ 3 種 → `not(affirmed)`、forward / skipped でカーソルが practicesStage へ動いた → `not(affirmed)` | `actReject` のリセットを外す |
| `promote_frame` | `lastAction == "promote_practices"` は本流の 8 変数と `stanceRecorded`・レビュー 3 変数を動かさない | `actPromotePractices` で `cursor' = cursor + 1` |

witness 2 本（負形式）: `w_practices_affirmed = lastAction == "promote_practices"`、`w_approved_practices = lastAction == "report_forward" and prevCursor == practicesStage`（ガードの空虚成立を防ぐ）。`scripts/quint-gate.sh` の invariants 列と witness 列に追加。

ITF 準拠（`engine_loop_conformance.rs`）: `parse_state` に 2 変数、合成計画は索引 `practicesStage` のステージ slug を **`practices-discovery`** にする（集約は slug で見つけるため。`-1` なら従来どおり `stage-<n>`）、`assert_projection` で `practices_affirmed(stage)` を `s == practicesStage and affirmed` と突き合わせる。駆動 `promote_practices` → `affirm_practices(intent, &PracticesPromotion::default(), "r", at)`；`report_forward` は変更なし（受領証ガードはモデルが通しているので通る）。フィクスチャは 13 本すべて採り直し（状態変数が増える）、`trace-0x808`（`not(w_approved_practices)`）を追加。採取コマンドは §9 に記録（b48 §9 の 13 本 + 1 本）。

## 4. ユースケース（`modules/core/command/use-case`）

- 新規 `PromotePracticesUseCase<E, I>`（`aidlc-state practices-promote`）: 入力 `PracticesPromotionRequest { promotion: PracticesPromotion, affirming_user: String }`。定型 3 手（find execution → find intent → `affirm_practices` → store）+ 楽観競合 1 回再試行（`RecordReviewUseCase` と同型）。定義は引かない（practices ステージは計画の slug で見つかる）。CQS: 成功は `Ok(())`（stdout の材料は合成ルートが全部持っている — 発生時刻は自分が渡した `occurred_at`、件数は `promotion`）。`PromotePracticesError { Repository, IntentRepository, Command(CommandError) }`。
- `CommitVerdictUseCase` は変更なし（段 12 は `approve_gate` の中で効く）。`CommitError::Transition { error: PracticesReceiptMissing, .. }` がそのまま上がる。
- `test_support` に「索引 1 が `practices-discovery` の計画」を組むヘルパ（`genesis_with_practices(stage_count)`）。

## 5. RMU（`modules/core/read-model-updater`）

- `ReadModel` に **メモリ面**を足す: `memory: Option<MemoryFaces { team: String, project: String, dirty: bool }>`。`ReadModel::new(state)` は `None`、`with_memory(team, project)` で載せる。`project_one` の `PracticesAffirmed` 腕は `memory` が `None` なら `ProjectionError::MemoryFilesMissing`（fail-closed — 動詞が存在を確かめた後に消された場合だけ到達）。
- `PracticesAffirmed` の投影（順序は upstream Step 4〜7）:
  1. team.md: `sections` を順に `replace_section(team, "## <heading>", body)`（見出し不在 → `ProjectionError::MemoryHeadingMissing { file, heading }`）。
  2. project.md: `mandated` / `forbidden` の各行を、trim 一致で既に在る行を除いて `append_under_heading("## Mandated" / "## Forbidden", line + "\n")`（at-least-once の再実行で重複しない）。
  3. 状態ファイル: `## Project Information` の `Practices Affirmed Timestamp` を `set_or_insert`（`with_field_or_insert`）で発生時刻へ、`Last Updated` も発生時刻へ（upstream `:3740-3750`）。
  4. 監査 1 行 `## Practices Affirmed`: `Affirming User` / `Sections Written`（`", "` 結合、空なら空値 — 空値の欄も描く: `**Sections Written**: ` + LF）/ `Mandated Rules Appended` / `Forbidden Rules Appended`（upstream `:3733-3738` の構築順）。`mod key` に 4 鍵。
- `ProjectionTargets` に `team_md` / `project_md`（`memory_dir` から組む）。`catch_up`: 未投影の実行イベントを描く前に両ファイルが**両方在れば**読んで `with_memory`（片方だけ在るのは `None` 扱い — 動詞側の存在検査が正本）。書く順は **project.md → team.md → 状態ファイル → 監査シャード**（`dirty` のときだけメモリ面を書く。`write_atomic` で置換）。`CatchUpError` に `MemoryFileRead` / `MemoryFileWrite`。
- `read_execution` に列は足さない（消費者なし）。`read_tables/spelling.rs::jump_refusal` に `practices-receipt-missing`。
- イベント DTO 1 変種を RMU 側と command 側の両集合に（`sections: [{heading, body}]`、`mandated: [..]`、`forbidden: [..]`）。command 側スナップショット DTO に `practices_affirmed: [bool]`（欄不在は全 false）。

## 6. app（`modules/app/aidlc`）

- `cli/face.rs`: `Face::State`（`aidlc-state`）。`cli/request.rs`: `(Face::State, Some("practices-promote"))` → `Request::StatePracticesPromote(PromoteArgs)`；既知の他 23 動詞（`get, set, set-skeleton-stance, set-construction-iteration, checkbox, count, advance, finalize, complete-workflow, gate-start, approve, reject, revise, skip, resume, acknowledge-compaction, reuse-artifact, lookup, practices-event, fork, merge, park, unpark`）→ `Request::StateNotWired { verb }`（own wording、b48 の `LogNotWired` と同型: `Cannot run aidlc-state <verb>: the <verb> subcommand is not wired in this build. Only \`practices-promote\` is available.`）；未知 → stderr `Unknown subcommand: <sub>. Valid: get, set, set-skeleton-stance, set-construction-iteration, checkbox, count, advance, finalize, complete-workflow, gate-start, approve, reject, revise, skip, resume, acknowledge-compaction, reuse-artifact, lookup, practices-event, practices-promote, fork, merge, park, unpark`（`:630` 逐語。sub が無ければ `undefined`）。
- `cli/promote_args.rs`: upstream `:3512-3519` の写し — `--x` の次のトークンを値に取る（真偽フラグ無し、末尾の孤立 `--x` は捨てる）。`--team-practices` / `--discovered-rules` / `--affirming-user` / `--target-dir`。`--project-dir` は `Invocation` が剥がす。
- `runtime::practices_promote`（順序は `handlePracticesPromote` どおり、拒否はすべて stderr + exit 1 = `Completion::refused`）:
  1. `--team-practices` か `--discovered-rules` が無い → `Usage: aidlc-state.ts practices-promote --team-practices <path> --discovered-rules <path> [--affirming-user <name>] [--target-dir <path>]`（`:3522` 逐語）。
  2. `--target-dir` → not-wired 拒否（own wording: `Cannot redirect the promotion: --target-dir is not wired in this build. The affirmed practices are written to the active space's memory directory.`）。
  3. 実行カーソル不在 → own wording `Cannot resolve the active intent for practices promotion.`（upstream は暗黙に状態ファイルを読んで倒れる）；読めない・壊れているは `unreadable_execution_cursor`。
  4. Step 1: practices ステージが定義に無い → `practices-promote failed: practices-discovery is absent from the compiled stage graph`（クエリ側 `read_definition_stage` を slug で 1 引当。support agents もここから読む）。ドラフト 2 本の親ディレクトリが違う → `practices-promote failed: team-practices and discovered-rules drafts must share one stage directory`。contributions: `<draftDir>/contributions/<agent>.md` の 1 行目が `**Collaborator:** <agent>` と一致しなければ集めて `practices-promote failed: ensemble evidence is incomplete: <agent> (no contribution file); <agent> (missing identity-marker first line)`（`; ` 結合、support agents の宣言順）。
  5. Step 2: `practices-promote failed: team-practices draft not found: <path>` / `… discovered-rules draft not found: <path>` / `… could not read drafts: <detail>`。
  6. Step 3: `practices-promote failed: team.md not found at <path>` / `… project.md not found at <path>` / `… could not read targets: <detail>`（path は `<memory_dir>/team.md` 等の実パス）。
  7. Step 4: `PracticesPromotion::plan(…, today = 発生時刻の UTC 日付)`。`TeamHeadingMissing` → `practices-promote failed: replaceSection failed on team.md for "## <heading>": replaceSection: heading not found: ## <heading>`；`ProjectHeadingMissing` → `practices-promote failed: appendUnderHeading failed on Mandated: appendUnderHeading: heading not found: ## Mandated`（Forbidden も同形）。
  8. ユースケース → 拒否は `practices-promote failed: <材料>`（`UnknownStage` は Step 1 で先に止まるので実質到達しない）→ `catch_up`（失敗は `orchestrate_failure`）→ stdout JSON 1 行（canon-json ContractCompact）: `{"emitted":"PRACTICES_AFFIRMED","sections_written":["Way of Working",…],"mandated_appended":<n>,"forbidden_appended":<n>,"affirmed_at":"<YYYY-MM-DDTHH:MM:SSZ>","team_md":"<path>","project_guardrails":"<path>"}`（`:3765-3773` の鍵順。`affirmed_at` は渡した発生時刻の秒精度 ISO）。
- `report` の段 12: `CommitError::Transition { error: PracticesReceiptMissing(_), stage, .. }` → **`error` directive**（stdout、exit 0 — upstream は orchestrate 自身の `errorDirective` であり `aidlc-state approve` の包み文ではない）: `Cannot approve "practices-discovery" before practices-promote succeeds. Run aidlc-state.ts practices-promote after the human approves; it records Practices Affirmed Timestamp and a fresh PRACTICES_AFFIRMED receipt for this stage attempt, then report --result approved --user-input "<exact choice>".`（`:5777-5781` 逐語）。`runtime.rs` の 13 段表の「11 completion-evidence / 12 practices 受領証」行を更新（12 は集約へ）。
- `journal_protocol_conformance.rs` / `intent_lifecycle.rs` の網羅 match を 16 変種へ。

## 7. テスト（TDD、層ごとに red → green）

- ドメイン: `MarkdownSections` 3 関数（見出し末尾空白・`###` 非一致・最初の一致・fence 内無視・EOF 節・不在）；`PracticesPromotion::plan` 表（節の有無・空節・規則の trim / コメント / `#` 行・重複除去（正本に既在 / 同一ドラフト内）・見出し不在 2 形・空の昇格）；`affirm_practices` の拒否 2 形と受理（本流の状態に依らず）；フロア（forward / skipped / reject / jump 3 種で false、revise では残る、`SingleStageRunCommitted` / stance / park / autonomy は触らない）；`approve_gate` の段 12（practices ステージだけ要求、受領証で通る、段 11 より先に効く — reviewer 宣言 + 受領証なしで `PracticesReceiptMissing` が先）；再構成で `practices_affirmed` が復元される；完全コンストラクタの長さ検査。
- Quint: ゲート全緑、mutation 3 件、ITF 14 本の準拠（アクション網羅 23）。
- ユースケース: 3 手・競合再試行・拒否の伝播。
- RMU: 4 面（team.md 置換・project.md 追記と重複除去・状態ファイル 2 欄・監査行のフィールド順と空値）、`memory` 不在の fail-closed、見出し不在、`catch_up` の書込順と `dirty` 判定（メモリ面を触らないイベントでは書かない — mtime / 内容不変で固定）。
- interface-adapter: DTO 往復 1 変種 + スナップショット `practices_affirmed`。
- app: パーサ、逐語カタログ、`Workspace` ハーネスで end-to-end — 合成グラフに `practices-discovery`（support agents 3 本）を持たせ、ドラフト 2 本 + contributions 3 本 + memory の team.md / project.md を書いて: 昇格 → JSON 1 行 → team.md の 5 節が置換され project.md に印付き行が並び、状態ファイルに `Practices Affirmed Timestamp` が入り、監査行が upstream のフィールド順で 1 行 → `report --result approved` が `done`；受領証なしの承認は `error` directive の逐語；差し戻し後は積み直しが要る；再昇格は重複しない；拒否 10 形の逐語（usage / target-dir / 未鋳造 / 定義に無い / dir 不一致 / contributions 2 形 / draft 不在 / target 不在 / 見出し不在）；未知動詞と not-wired 動詞。
- ゴールデン `cli/report/approved`（practices-discovery = reviewer なし）は**影響を受ける** — 承認前に昇格を積む形へ（b48 で advisory 受領証を積んだのと同じ）。
- カバレッジ相対ゲート（base ≧ 99.12%）を割らない。

## 8. 仕様・記録

- `docs/specs/10-orchestration.md`: B10 行に段 12 の裁定 3 点；§3 ユースケースに `PromotePractices`；§6 に I19「practices-discovery の承認は現在の試行の昇格受領証を要する。試行の区切りは I18 と同じ」E4 = `engine_loop::{approve_requires_practices_receipt, practices_receipt_floor, promote_frame}`；§9 に v2.6；§10 に b49 の実装ノート（段 12 配線、メモリ面の投影）。
- `docs/specs/11-workspace.md`: メモリ層 2 ファイルが RMU の投影面になったことを追記（人が編集する正本でもある — 投影は節置換・行追記だけで他を触らない）。
- `docs/specs/deviations.md` #6: `PRACTICES_OVERRIDE` 非描画、`--target-dir` / `practices-event` / 他 21 動詞の not-wired、revision backstop（CP5）、`Cannot resolve the active intent for practices promotion.` の own wording。
- `handoff-b49.md`、Issue #7 キュー 5 の本文（b49 完了 = キュー 5 完了）。

## 9. 検証記録（2026-09-05 実測、実装は Opus サブエージェント、統合レビューは Fable 5）

- **ドメイン**: `IntentExecutionEvent` を 16 変種へ（`PracticesAffirmed { stage, affirming_user, sections, mandated, forbidden }`、`id` + `aggregate_id` を持つ）。
  `IntentExecution` に `practices_affirmed: Vec<bool>`（計画長、完全コンストラクタが長さを検査）。クエリ `practices_stage()`（slug `PRACTICES_DISCOVERY_SLUG` の位置）/ `practices_affirmed(stage)`。
  コマンド `affirm_practices(intent, &PracticesPromotion, affirming_user, at)`（取り違え → 計画に practices が無い、の 2 ガードだけ。**本流の状態は見ない**。再昇格は上書き）。
  `approve_gate` の段 12 = `require_practices_receipt`（`require_checkbox` の**後**・`require_review_receipt` の**前**）。
  フロア: `reset_attempt` をレビュー会計と共用へ改め（前進 / 読み飛ばしで立った次ステージ・`GateRejected` のステージ）、`Jumped` は全ステージ。`StageRevised` はフロアではない。
  `CommandError` 新変種 1（`PracticesReceiptMissing(StageIndex)`、Display は材料だけ）。
  `workspace` に純関数 3 本（`extract_section` / `replace_section` / `append_under_heading` — upstream `aidlc-lib.ts` の写し）と `HeadingNotFound`、値オブジェクト `PracticesPromotion` / `PromotedSection` / `PromotionPlanError`。
  `workflow_definition::PRACTICES_DISCOVERY_SLUG`。
- **DTO**: 1 変種の永続化 DTO（command 側 / RMU 側の両集合。`sections: [{heading, body}]` / `mandated` / `forbidden`）、スナップショット行に `practices_affirmed: [bool]`（欄不在は全ステージ false で読む）。ワイヤ形式 16 変種を両側のゴールデンコーパスで固定。
- **ユースケース**: 新規 `PromotePracticesUseCase<E, I>`（find → find intent → コマンド → store、`Conflict` 1 回再試行。**定義は引かない**）、入力 VO `PracticesPromotionRequest`、封筒 `PromotePracticesError`。成功は `Ok(())`（CQS — stdout の材料は合成ルートが全部持っている）。
- **RMU**: `ReadModel` にメモリ面（`MemoryFaces { team, project, dirty }` / `with_memory` / `memory` / `replace_memory`）。`PracticesAffirmed` の投影は 4 面（team.md の 5 節置換 → project.md の印付き行追記（trim 一致で重複除去）→ 状態ファイルの `Practices Affirmed Timestamp`（`## Project Information` に挿入）と `Last Updated` → 監査行 `PRACTICES_AFFIRMED` の 4 欄）。`ProjectionTargets` は memory ディレクトリを受け取って `team_md` / `project_md` を導く。`catch_up` は 2 本とも在るときだけ載せ、**dirty のときだけ** project.md → team.md の順で書く（状態ファイル → 監査シャードはその後）。`CatchUpError::{MemoryFileRead, MemoryFileWrite}` / `ProjectionError::{MemoryFilesMissing, MemoryHeadingMissing}`。`read_tables/spelling.rs::jump_refusal` に `practices-receipt-missing`。
- **クエリ側**: `read_definition_stage` を slug で 1 引当するポート `DefinitionStageDao` / View `DefinitionStageView`（`stage_slug` / `support_agents`）/ 実装 `DefinitionStageDaoImpl` / ダブル `InMemoryDefinitionStageDao` / ユースケース `FindDefinitionStageUseCase`。**行が引けないこと自体が「グラフに無い」の答え**である。
- **app**: `Face::State`（`aidlc-state`）。`cli/promote_args.rs` は upstream `:3512-3519` の写し（真偽フラグ無し・末尾の孤立フラグは捨てる）。`practices-promote` → `PromotePracticesUseCase`、他の 24 動詞 → not-wired 拒否（own wording）、未知 → `:630` の逐語。`runtime::practices_promote` の順序: usage → `--target-dir` 未配線 → 実行カーソル → 投影の追いつき → Step 1（定義の行・ドラフト dir・contributions）→ Step 2 / 3（ドラフトと正本の読取）→ Step 4（`PracticesPromotion::plan`）→ 記録 → `catch_up` → stdout JSON 1 行（`ContractCompact`、鍵順は upstream `:3760-3768`）。`report` の段 12 は **orchestrate 自身の `error` directive**（stdout・exit 0）。
- **Quint v2.6**: 状態変数 2 本（`practicesStage` は init で選んで凍結、`affirmed`）+ スナップショット 1 本、アクション 1 本（`actPromotePractices`）、`actReportForward` の段 12 ガード、フロア（forward / skipped の新カーソル、reject のカーソル、jump 3 種）、不変条件 3 本（19 本へ）、witness 2 本（9 本へ）。
  **mutation 検査 3/3（対照の無変異は `[ok]`）**: `actReportForward` の段 12 ガードを外す → `approve_requires_practices_receipt`；`actReject` の昇格受領証リセットを外す → `practices_receipt_floor`；`actPromotePractices` で `cursor' = cursor + 1` → `promote_frame`。
  ITF 準拠: `parse_state` に 2 変数、合成計画は索引 `practicesStage` の slug を `practices-discovery` にする（`slug_at`）、`assert_projection` で `practices_affirmed(stage)` を `s == practicesStage and affirmed` と突き合わせ、駆動 `promote_practices` → `affirm_practices(intent, &PracticesPromotion::default(), "r", at)`。フィクスチャ 14 本、アクション網羅 23。
- **テスト**: 新規 **93 本**（`#[test]` / `#[tokio::test]` の増分。`cargo test --workspace` の実測は b48 の 1,974 本 → 2,067 本）。
  ドメインの `MarkdownSections` 13 本 / `PracticesPromotion::plan` 8 本 / 集約の段 12・フロア・再構成 11 本、
  ユースケース 6 本、DTO 往復（両側）と欄不在の読み、RMU の 4 面投影 7 本と `catch_up` の書込順・dirty・媒体失敗 5 本、
  クエリ側 DAO の契約 2 実装 + failing / empty、app の `Workspace` ハーネス end-to-end 9 本（一巡 → 4 面 → 承認、
  受領証なしの拒否逐語、差し戻し後の積み直し、再昇格の非重複、拒否 10 形、未知 / not-wired 動詞、媒体失敗 5 形）。
- `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `cargo test --workspace` — **全緑（49 スイート、2,067 本）**。`tools/lint` 自己テスト 69 本も緑。`cargo doc --workspace --no-deps` warning 0
- `scripts/quint-gate.sh` — **全 PASS（25 ステップ。engine_loop の不変条件 19 本、witness 9 本）**
- `scripts/coverage.sh --base origin/main` — 絶対 **99.1309% ≥ 90.0% PASS**、相対 **99.1309% ≥ 99.1247 − 0.01 PASS**

### ITF フィクスチャの採取コマンド（`M=formal/orchestration/engine_loop.qnt`、`D=tests/conformance/fixtures/engine_loop`）

b48 §9 の 13 本は**同じコマンド**で再採取し、新規 1 本（`trace-0x808`）を足した。

```
for s in 0xa1 0xb2 0xc3 0xd4 0xe5 0xf6 0x202; do quint run $M --seed $s --max-samples 1 --max-steps 40 --out-itf $D/trace-$s.itf.json; done
quint run $M --seed 0x101 --max-samples 2000 --max-steps 40 --invariant 'not(lastAction == "report_revised")' --out-itf $D/trace-0x101.itf.json
quint run $M --seed 0x303 --max-samples 2000 --max-steps 40 --invariant 'not(w_repark)'             --out-itf $D/trace-0x303.itf.json
quint run $M --seed 0x404 --max-samples 2000 --max-steps 40 --invariant 'not(w_single_run)'         --out-itf $D/trace-0x404.itf.json
quint run $M --seed 0x505 --max-samples 2000 --max-steps 40 --invariant 'not(w_stance_recorded)'    --out-itf $D/trace-0x505.itf.json
quint run $M --seed 0x606 --max-samples 2000 --max-steps 40 --invariant 'not(w_approved_reviewed)'  --out-itf $D/trace-0x606.itf.json
quint run $M --seed 0x707 --max-samples 2000 --max-steps 40 --invariant 'not(w_retry_review)'       --out-itf $D/trace-0x707.itf.json
quint run $M --seed 0x808 --max-samples 2000 --max-steps 40 --invariant 'not(w_approved_practices)' --out-itf $D/trace-0x808.itf.json
```

状態数: 0x101 = 10、0x202 = 41、0x303 = 3、0x404 = 2、0x505 = 16、0x606 = 38、0x707 = 19、0x808 = 24、0xa1 / 0xb2 / 0xc3 / 0xd4 / 0xe5 / 0xf6 = 41（状態変数が増えたぶん、同じ seed でも b48 とは経路が変わっている）。

### 設計との差分（実装で受け入れたもの）

1. **`aidlc-state` の認識動詞は 23 ではなく 24** — 設計 §6 の一覧に `unit` を足した。upstream の switch（`:530-627`）は `unit` を受理する（`handleUnit` へ落ちる）ので、こちらで「未知」に落とすと upstream が知っている動詞を知らないと言うことになる。未知動詞の逐語（`:630` の `Valid:` 一覧）は upstream 自身が `unit` を載せていないので**そのまま**である。
2. **ゴールデン `cli/report/approved` は影響を受けなかった** — 設計 §7 は「承認前に昇格を積む形へ」としていたが、`cli_golden_test.rs` の合成グラフのステージ slug は `domain-design` であり `practices-discovery` ではない（`RECORDED_SLUG` は**採取済み stdout の文字列置換にだけ**使う名前で、駆動するグラフの slug ではない）。段 12 は practices-discovery だけに効くので、ゴールデンの駆動は 1 行も変えていない。
3. **`with_field_or_insert` の挿入位置が変わった（是正）** — 設計 §2.2 のとおり `append_under_heading` へ寄せた結果、挿入は「節末尾の空行より**前**」から「次の `## ` 見出しの**直前**」へ移った。後者が upstream（`setOrInsertField` → `appendUnderHeading`）であり、ゴールデン `cli/park/park/state.diff` の実バイトでもある。`HeadingNotFound` は RMU 側の型を捨ててドメインの型に一本化した（同じ意味論の型を並立させない — `no-backward-compatibility.md`）。`park_marker` は別実装のまま残す（**2 行を対で置き直す**書き手であり、「1 フィールドの置換 or 挿入」の口では表せない — 理由の doc も書き替えた）。
4. **`ProjectionTargets::new` は memory ディレクトリ 1 本を受け取る** — `team_md` / `project_md` を個別に受け取ると、別 space の 2 本を取り合わせた束を構成できてしまう（この型がそもそも「片方だけ差し替わった取り合わせを作れなくする」ために在る）。
5. **`CatchUpError::MemoryFileWrite` は材料を詰め直す** — 書き手は設計どおり `write_atomic`（tmp+rename + W_OK バリア）だが、あちらの `ReadOnlyTarget` 文言は**状態ファイルを名指す**ので、そのまま運ぶと診断が嘘になる。`{ path, detail }` に写し替えた。
6. **クエリ側に 1 表ぶんの口を新設** — 設計 §6 の「既存 DAO/View に無ければ最小の DAO メソッドと View を足す」に従い、`DefinitionStageDao` / `DefinitionStageView` / `DefinitionStageDaoImpl` / `InMemoryDefinitionStageDao` / `FindDefinitionStageUseCase` を足した。View が運ぶ列は `stage_slug` / `support_agents` の 2 つだけである（使わない 30 列を載せると「この行の写しは何のためにあるのか」が読めなくなる）。契約テストは既存の 12 ポートと同じ形で 2 実装に走らせる。
7. **`directive_drawing::strings` を `pub(crate)` へ** — `read_definition_stage.support_agents` も `read_run_stage.support_agents` と同じ 1 行 JSON 配列なので、開き方を 2 か所に書かない。
8. **段 12 の逐語は `commit_refusal` の腕の順序で効く** — `PracticesReceiptMissing` と `ReviewReceiptMissing` はどちらも `CommitError::Transition` なので、前者の腕を**先**に置く。段 12 は orchestrate 自身の `error` directive（包み文なし）、段 11 は `aidlc-state approve` の包み文、という違いがそのまま腕の形に出る。
9. **`MemoryFileRead` の観測には参照入力の分離が要る** — 取得ループは投影面（`ProjectionTargets`）と参照入力（`SteeringSource`）を別の引数で受け取るが、合成ルートは両方を同じ memory ディレクトリへ向ける。同じ向き先のままだと `catch_up_steering` が先に同じファイルで倒れるので、投影面の読取失敗だけを孤立させるテストは参照入力を別ディレクトリへ向けて組む（ループの契約は 2 入力が別であること）。

### 統合レビュー（Fable 5、2026-09-05）

- 差分 64 ファイル + 新規 20 ファイルを全読。設計 §2〜§8 との対応を確認し、上の差分 9 点をすべて受け入れた（いずれも upstream 準拠か規律の帰結）。
- ゲートを再計測: fmt / clippy / `cargo lint` / `cargo test --workspace`（49 スイート 2,067 本）/ `tools/lint` 69 本 / `cargo doc` warning 0 / Quint ゲート 25 ステップ PASS。
- ITF フィクスチャ 14 本を上の採取コマンドで再採取し、`#meta` を除いてバイト一致を確認。mutation 3 件を再実行し 3/3 で `[violation]`（対照は `[ok]`）。
- 直した点: `docs/specs/deviations.md` #6 の行が表と空行で分断されていたので表の 5 行目の直後へ移した（Markdown の表は空行で終わる）。§9 のテスト数の起点を実測（b48 の 1,974 本）に訂正。

