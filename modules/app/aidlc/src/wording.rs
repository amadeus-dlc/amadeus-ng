//! 逐語文言 — **出す側が組む**（`coding-rules/error-handling.md`）。
//!
//! ドメインもポートも材料しか運ばない。利用者が読む文字列を組み立てるのはここだけである。
//!
//! # 逐語は 1 バイトも変えない
//!
//! ここの文字列は upstream の観測可能な契約（Published Language）であり、綴りが変われば
//! 互換が壊れる（`coding-rules/ubiquitous-language.md`「外に出る値は逐語で維持する」）。
//! 各定数の doc に upstream の出典を `ファイル:行` で書いてあるので、疑わしいときは
//! そちらを正とすること。

/// 未知サブコマンド（upstream `aidlc-orchestrate.ts:6155`）。
///
/// 引数が無いときの `(none)` まで含めて逐語である。
#[must_use]
pub fn unknown_orchestrate_subcommand(given: Option<&str>) -> String {
    format!(
        "Unknown subcommand: {}. Valid: next, continue, report, park",
        given.unwrap_or("(none)")
    )
}

/// 上限超過の emit 拒否（upstream `aidlc-orchestrate.ts:266`）。
///
/// upstream は定数 `DIRECTIVE_MAX_BYTES` を埋め込むので、こちらも同じ値を描く。
#[must_use]
pub fn refusing_oversize_directive(cap: usize) -> String {
    format!("aidlc-orchestrate: refusing to emit a directive larger than {cap} bytes")
}

/// `aidlc/active-space` の値が空間名として成立しない。
///
/// upstream の `activeSpace()` は値を検証せず**そのままパス片として使う**
/// （`aidlc-lib.ts:1300-1308`）ので、対応する逐語は存在しない。我々のストアは空間名を
/// 型で受けるため、通せない値は既定へ落とさずここで止める — 落とすと record と
/// イベントが別々の空間へ散る。
#[must_use]
pub fn invalid_active_space(raw: &str) -> String {
    format!(
        "The active space \"{raw}\" is not a valid space name. Fix aidlc/active-space (or remove it to use the default space), then run the command again."
    )
}

/// `--review` に閉集合外の値が来た（upstream `aidlc-utility.ts:159` 逐語）。
///
/// 接頭辞を付けないのは、これが `aidlc-utility` 面の拒否だからである（`aidlc-orchestrate:`
/// と名乗ると出所を偽る）。upstream は同じ文言を `{"error": …}` に包んで stderr へ出し
/// exit 1 する。包み方をここで変えないのは、stderr のエンベロープ形式が本文言だけの問題では
/// なく自己防衛拒否の全面に関わるためである（横断の是正は別 Bolt）。
#[must_use]
pub fn unknown_review_class(raw: &str) -> String {
    format!("Unknown review class: \"{raw}\". Valid: adversarial, advisory, none.")
}

/// 未捕捉の失敗（upstream `aidlc-orchestrate.ts:6167`）。
#[must_use]
pub fn orchestrate_failure(detail: &str) -> String {
    format!("aidlc-orchestrate: {detail}")
}

/// intent は着地したが最初の実行の永続化に失敗した — 部分失敗の診断と復旧手順
/// （issue #77 の先行改善、オーナー裁定 2026-09-01）。
///
/// upstream に対応する逐語は無い（upstream は単一ロック + ファイル操作でこの失敗形が
/// 存在しない）。我々の ES 分割（2 集約 = 2 ストリーム、集約間トランザクション無し）に
/// 固有の診断である。孤児は無害に残り、恒久の検出・修復は doctor が担う。
#[must_use]
pub fn orphaned_intent(orphan: &str, detail: &str) -> String {
    // 断定するのは検証済みの事実だけ — 状態ファイルが書かれていないこと (骨格の書込は
    // ユースケース成功後にしか走らない) と、実行の書込が失敗として報告されたこと。
    // 実行行の存否そのものはポート契約が Err ⇒ 未永続化を約束しないので断定しない
    // (存否の確認と修復は doctor の仕事 — issue #77、PR #87 CodeRabbit 指摘の反映)。
    format!(
        "aidlc-orchestrate: {detail}\n\
         Intent {orphan} was minted, but storing its first execution failed - the \
         intent is left behind without a started workflow (no state file was written). \
         Re-run intent-create to mint a fresh intent; the leftover intent is inert. \
         Detection and repair of leftovers is tracked by the doctor command (issue #77)."
    )
}

/// 継続トークンが検証できない（upstream `aidlc-orchestrate.ts:5999`）。
///
/// トークンの不正・鍵の不在・引数の個数違いを**区別しない** — fail-closed の指示は
/// どの原因でも同じ「fresh `next` からやり直せ」だからである（I12）。
pub const INVALID_CONTINUATION_TOKEN: &str = "Invalid steering continuation token: this stage's rules cannot be loaded from where they left off. Run a fresh `next` to restart delivery from part 1.";

/// 鍵ファイルが壊れている（upstream `aidlc-orchestrate.ts:2323`）。
#[must_use]
pub fn corrupt_key_file(path: &str) -> String {
    format!(
        "The local key file at \"{path}\" is corrupt, so this stage's rules cannot be loaded safely. \
         Delete that file and run a fresh `next`; a replacement is created automatically."
    )
}

/// 鍵ファイルが読めない（upstream `aidlc-orchestrate.ts:2331`）。
#[must_use]
pub fn unreadable_key_file(path: &str, cause: &str) -> String {
    format!(
        "Cannot read the local key file at \"{path}\", so this stage's rules cannot be loaded ({cause})."
    )
}

/// 鍵ファイルが作れない（upstream `aidlc-orchestrate.ts:2350`）。
#[must_use]
pub fn uncreatable_key_file(path: &str, cause: &str) -> String {
    format!(
        "Cannot create the local key file at \"{path}\", so this stage's rules cannot be loaded \
         ({cause}). Fix the directory permissions, then run a fresh `next`."
    )
}

/// 受理されない `--result`（upstream `aidlc-orchestrate.ts:5528`）。
#[must_use]
pub fn unknown_result(given: &str) -> String {
    format!(
        "Unknown --result \"{given}\". accepted outcomes: {}.",
        core_command_domain::orchestration::ACCEPTED_RESULTS.join(", ")
    )
}

/// 遷移が拒否された（upstream `aidlc-state.ts` 由来の拒否をエンジンが中継する形）。
#[must_use]
pub fn transition_rejected(detail: &str) -> String {
    format!("Transition rejected: {detail}")
}

/// 実行カーソル `<record>/.aidlc-execution` が在るのに読めない。
///
/// **upstream に対応する逐語は無い。** upstream は実行の識別子をどこにも持たない
/// （リードモデルにも欄が無い）ので、この失敗そのものが upstream には存在しない。
/// 我々はそれを record に据えるため、「不在」と「壊れている」を分けて答える必要がある
/// ——不在（まだ鋳造していない）は `No workflow execution to report against.` で、
/// 壊れているのがこちらである。原因（分類とパス）は
/// [`crate::execution_cursor::ExecutionCursorError`] の `Display` が運ぶ材料をそのまま置く。
#[must_use]
pub fn unreadable_execution_cursor(cause: &str) -> String {
    format!(
        "The execution cursor cannot be read ({cause}). Fix that file, or remove it and mint \
         a fresh intent with `aidlc-utility intent-create`."
    )
}

// ---------------------------------------------------------------------------
// `next` の逐語 — 21 分岐ラダーが出す文言 (契約マップ
// `docs/specs/research/orchestration-next-ladder.md` §1 が正本)。
//
// b44 でクエリ側 (`NextUseCase::wording`) からここへ移した。**行の `kind` に従って描くのは
// 出す側の仕事**であり、クエリ側は綴り (`decision_kind` 等) を運ぶだけである
// (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。
// ---------------------------------------------------------------------------

/// `--review` の併用ガード (前置)。
pub const REVIEW_COMBINATION: &str = "Cannot combine --review with read-only, workspace, compose, single-stage, jump, or resume modes. Apply /aidlc --review <class> first, then run the other command.";

/// 分岐 2 — `--stage` と `--phase` の併用。
pub const STAGE_AND_PHASE: &str = "Cannot use --stage and --phase together. Use one or the other.";

/// 分岐 4c の併用ガード。
pub const COMPOSE_WITH_JUMP: &str = "Cannot combine compose with --stage/--phase. Compose re-shapes the plan; jump moves the cursor. Run them separately.";

/// 分岐 7 の init ジャンプガード (`INIT_JUMP_ERROR`)。
pub const INIT_JUMP: &str = "Cannot jump to initialization stages. The Initialization phase runs automatically when you start a workflow (describe what to build, e.g. /aidlc \"build the auth service\").";

/// 分岐 9b — state もカーソルも無い。
pub const NO_STATE: &str = "No workflow state found (no active intent). Start one by describing what to build (/aidlc \"build the auth service\") or by naming a scope (/aidlc --scope <scope>).";

/// 分岐 4a — 記述が空 (upstream `aidlc-orchestrate.ts:2980-2982` 逐語)。
pub const NEW_INTENT_BLANK: &str =
    "`next --new-intent` requires a nonblank new-work description after the confirmed scope.";

/// 分岐 4b — `--single` にステージが無い (upstream `:3014-3016` 逐語)。
pub const SINGLE_REQUIRES_STAGE: &str =
    "--single requires --stage <slug>. A stage-runner runs exactly one named stage.";

/// 分岐 4b — `--single` と `--phase` の併用 (upstream `:3008-3010` 逐語)。
pub const SINGLE_WITH_PHASE: &str =
    "Cannot use --single with --phase. --single runs one stage; pass --stage <slug>.";

/// 分岐 4b / 段 2 — initialization ステージの隔離実行 (upstream `SINGLE_INIT_ERROR` `:4440-4441` 逐語)。
///
/// `next --single` と `report --single` の**両方**が同じ定数を使う (upstream も同じ 1 定数)。
pub const SINGLE_INIT: &str = "Cannot run an initialization stage with --single. Initialization is bootstrap (it creates the intent + state); it runs automatically when you start a workflow (describe what to build, e.g. /aidlc \"build the auth service\").";

/// 分岐 4b — その scope では読み飛ばされるステージ (upstream `:4463-4465` 逐語)。
#[must_use]
pub fn stage_skipped_for_scope(stage: &str, scope: &str) -> String {
    format!(
        "Stage \"{stage}\" is skipped for scope \"{scope}\". Choose a different stage or change scope."
    )
}

/// `--label` の畳み方を conductor へ伝える一文 (upstream `:889-890` 逐語)。
///
/// 先頭の空白は upstream のまま — 直前の文へ連結される位置にある。
const LABEL_HINT: &str = " Replace `--label` with a 2-3 word kebab essence of the description (e.g. \"simple calc\"), which becomes the readable folder name for this piece of work.";

/// 完了した intent の `done` reason に必ず続く新規作業のヒント (upstream `:855-859` 逐語)。
///
/// 先頭の空白は upstream のまま — reason 本文へ連結される位置にある。
const NEW_WORK_HINT: &str = " If this input is genuinely NEW, unrelated work (not a follow-up to the completed intent), don't stop here: offer to start a second intent, and on the human's yes run `next --new-intent --scope <scope> \"<text>\"` (see the SKILL's new-work offer, never auto-birth).";

/// 分岐 2.5 — park している。
#[must_use]
pub fn parked(stage: &str) -> String {
    format!("Workflow parked at \"{stage}\". Resume with /aidlc --resume.")
}

/// `park` の失敗の中継形（upstream `aidlc-orchestrate.ts:8252`）。
///
/// upstream の `handlePark` は `aidlc-state.ts park` を spawn し、非ゼロ終了なら
/// その stderr／stdout を `Cannot park the workflow: <detail>` に**そのまま**包んで
/// error directive で返す。材料はこちらではユースケースの失敗の `Display` である。
#[must_use]
pub fn park_refused(detail: &str) -> String {
    format!("Cannot park the workflow: {detail}")
}

/// park の拒否 1 — autonomous な構築ラン（upstream `aidlc-state.ts:1712-1714`）。
///
/// 無人の autonomous ランには再開する人間が居ないので、そもそも止めてはならない
/// （issue #365 のガード）。ハイフンは upstream のまま ASCII の `-` である。
pub const PARK_REFUSED_AUTONOMOUS: &str = "Refusing to park: Construction Autonomy Mode is autonomous. An unattended autonomous run has no human to resume it and must keep moving - do not park it.";

/// park の拒否 2 — 完了済み（upstream `aidlc-state.ts:1742`）。
pub const PARK_NOTHING_TO_PARK: &str = "Workflow is already Completed - nothing to park.";

/// park の拒否 3 — 実行がまだ鋳造されていない。
///
/// upstream に対応する逐語は無い（あちらは状態ファイル不在時に `readStateFile` の失敗文を
/// 中継する）。`report` の同型の拒否と綴りを揃えてある。
pub const PARK_WITHOUT_EXECUTION: &str = "No workflow execution to park. Run `next` first.";

/// 分岐 2.6 — park 中の `--resume`。
#[must_use]
pub fn unpark_then_resume(spelled: &str) -> String {
    format!(
        "This workflow is parked. Run `{spelled}` to clear the park marker, then re-run `next --resume` to continue."
    )
}

/// 分岐 3b / 解決不能 — 未知 scope。
#[must_use]
pub fn unknown_scope(scope: &str, valid: &[String]) -> String {
    format!(
        "Unknown scope \"{scope}\". Valid scopes: {}.",
        valid.join(", ")
    )
}

/// 分岐 4 — 環境変数の既定 scope が未知。
#[must_use]
pub fn invalid_env_scope(value: &str, valid: &[String]) -> String {
    format!(
        "Invalid AWS_AIDLC_DEFAULT_SCOPE \"{value}\". Valid scopes: {}.",
        valid.join(", ")
    )
}

/// 誕生 print の本文 (upstream `createPrintDirective` `:900-910` 逐語)。
///
/// `new_intent` が真なら「別 intent なのでこのセッションを畳め」の 4 文が続き、偽なら
/// 「そのまま `next` を再実行せよ」の継続形になる。分岐するのは尾部だけで、コマンドの
/// 名指し・コスト節・ラベル助言は共通である。
#[must_use]
pub fn birth_print(spelled: &str, cost: &str, has_description: bool, new_intent: bool) -> String {
    let label_hint = if has_description { LABEL_HINT } else { "" };
    if new_intent {
        format!(
            "Run `{spelled}` to start the new intent{cost}.{label_hint} Then STOP, do NOT re-run `next` in this session. \
This is a NEW, unrelated intent, and the current session still carries the previous intent's context. \
Tell the user to start a fresh session using this harness's reset or restart flow, then invoke its AI-DLC entry skill to begin the new intent with a clean slate. \
Nothing is lost: the intent is saved on disk and resumes on the next `next`."
        )
    } else {
        format!(
            "Run `{spelled}` to start the workflow{cost}, then re-run `next` to continue.{label_hint}"
        )
    }
}

/// コスト節 (upstream `costClause` `:669-676` 逐語)。括弧は呼出側が付ける。
///
/// 4 つの数はいずれも `read_definition_scope` の列であり、ここでは並べるだけである
/// (集約が数え、RMU が行に書いた)。
#[must_use]
pub fn cost_clause(total: u32, execute: u32, gates: u32, per_unit_stages: u32) -> String {
    let per_unit = match per_unit_stages {
        0 => String::new(),
        1 => ", 1 stage repeats per unit of work in Construction".to_string(),
        count => format!(", {count} stages repeat per unit of work in Construction"),
    };
    format!("{execute} of {total} stages, {gates} approval gates{per_unit}")
}

/// 未知ステージ (upstream `:4441` / `:4605` / `:5281` 逐語 — 一覧への案内まで含む)。
#[must_use]
pub fn unknown_stage(stage: &str) -> String {
    format!("Unknown stage \"{stage}\". Run /aidlc --help for the full list.")
}

/// 未知フェーズ (upstream `:4578` 逐語)。
///
/// フェーズ語彙の並びは upstream `PHASES` の宣言順である (`aidlc-lib.ts:130-136`、
/// 辞書順ではない)。
#[must_use]
pub fn unknown_phase(phase: &str) -> String {
    format!(
        "Unknown phase \"{phase}\". Valid phases: initialization, ideation, inception, construction, operation."
    )
}

/// 分岐 7 — そのフェーズに in-scope のステージが無い。
#[must_use]
pub fn no_stage_in_phase(phase: &str) -> String {
    format!("No in-scope stage found for phase \"{phase}\".")
}

/// 分岐 7 — jump の解決命令 (state あり)。
#[must_use]
pub fn resolve_jump(spelled: &str) -> String {
    format!("Run `{spelled}`.")
}

/// 分岐 5 — scope 変更の名指し (upstream `:3056` 逐語)。
#[must_use]
pub fn scope_change(spelled: &str) -> String {
    format!("Run `{spelled}` to change scope, then print its output verbatim and stop.")
}

/// 分岐 5 — 設定変更の名指し (upstream `:3072` 逐語)。
#[must_use]
pub fn config_change(spelled: &str) -> String {
    format!("Run `{spelled}` to update the configuration, then print its output verbatim and stop.")
}

/// 分岐 4c — composer ディスパッチの名指し。
#[must_use]
pub fn dispatch_composer(spelled: &str) -> String {
    format!("Dispatch the composer: run `{spelled}`.")
}

/// 分岐 8 — キーワードが当たった scope の確認 (upstream `:3150-3152` 逐語)。
///
/// コスト節の区切りは誕生 print の括弧ではなく ` - ` である (upstream `:3148`)。
#[must_use]
pub fn scope_confirm(scope: &str, intent: &str, cost: &str) -> String {
    format!(
        "This looks like \"{scope}\" work, so I'd run the \"{scope}\" plan for: \"{intent}\"{cost}. \
Say go ahead, name a different plan, or say \"compose\" and I'll tailor one to this task."
    )
}

/// 分岐 8 — どの既製 scope も当たらないときの compose 提案 (upstream `:3168-3171` 逐語)。
#[must_use]
pub fn compose_offer(intent: &str, examples: &str) -> String {
    format!(
        "None of the ready-made plans is an obvious fit for: \"{intent}\". \
I can work out a plan tailored to this task (recommended: reply \"compose\"), \
or you can pick one directly (e.g. {examples}; see /aidlc --help for the full list)."
    )
}

/// 分岐 1 (upstream `:2717` 逐語)。
#[must_use]
pub fn read_only(spelled: &str) -> String {
    format!(
        "Run `{spelled}`, print its output verbatim, then stop. \
This is a read-only utility, NOT workflow work: do NOT run `next` and do NOT advance, resume, or run any workflow stage."
    )
}

/// 分岐 1b/1c/1d — 名詞トークンの終端コマンド (upstream `:2764` 逐語)。
///
/// 読み取り専用ユーティリティとは 1 語だけ違う (`read-only` ではなく `terminal`)。
#[must_use]
pub fn terminal_utility(spelled: &str) -> String {
    format!(
        "Run `{spelled}`, print its output verbatim, then stop. \
This is a terminal utility, NOT workflow work: do NOT run `next` and do NOT advance, resume, or run any workflow stage."
    )
}

/// 分岐 6 — 再開メニュー。
#[must_use]
pub fn resume_menu(stage: &str) -> String {
    format!(
        "An existing workflow was found (currently at \"{stage}\"). How would you like to proceed? Resume from last checkpoint, redo the current stage, jump to a stage, or start fresh."
    )
}

/// 分岐 9c — 稼働中の自由記述。
pub const NEW_WORK_ROUTING: &str =
    "Does this continue the active work, start separate new work, or re-shape the plan?";

/// 分岐 7b — カーソルの無い記録が見つかった。
pub const INTENT_PICK: &str = "Existing intent records were found without an active cursor. Which intent should become active?";

/// 分岐 10 手順 3 (回復可能な plan/cursor 不整合。upstream `:3294-3297` 逐語)。
#[must_use]
pub fn recover_skip(stage: &str, spelled: &str) -> String {
    format!(
        "Stage \"{stage}\" is SKIP in the approved workflow plan but is still the active cursor. \
Do not run this stage. Run `{spelled}` to recover the stale pointer, then re-run `next` to continue."
    )
}

/// 分岐 10 手順 3 (回復経路のない plan/cursor 不整合)。
///
/// `checkbox` は行の綴り (`read_next_answer.checkbox` — `pending` / `in-progress` /
/// `awaiting-approval` / `revising` / `completed` / `skipped`) をそのまま埋める。
#[must_use]
pub fn inconsistent_skip(stage: &str, checkbox: &str) -> String {
    format!(
        "Stage \"{stage}\" is SKIP in the approved workflow plan but its active cursor state is \"{checkbox}\". Refusing to emit run-stage; repair the inconsistent state before continuing."
    )
}

/// 分岐 10 手順 5 (upstream `:3332-3348` — reason + `NEW_WORK_HINT`)。
#[must_use]
pub fn workflow_complete(stage: &str, scope: &str) -> String {
    format!(
        "Workflow complete — no in-scope stage remains after {stage} (scope: {scope}).{NEW_WORK_HINT}"
    )
}

/// `stage-graph.json` が読めないときの逐語文言 (12 §4 #1)。
///
/// ピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8565-8570` @3c3146cf)。
#[must_use]
pub fn stage_graph_not_readable(path: &str, cause: &str) -> String {
    format!(
        "Stage graph not readable at {path}: {cause}. Reinstall the framework or re-run setup to restore the data file."
    )
}

/// 構造化リードモデルを引けない (upstream に対応する逐語は無い — 診断文言)。
///
/// upstream はリードモデルを持たないので写す逐語が存在しない。したがって材料
/// (どのファイルが、なぜ) だけを並べ、回復手順を添える
/// (`coding-rules/error-handling.md`「材料だけを運ぶ」)。
#[must_use]
pub fn read_model_unreadable(path: &str, cause: &str) -> String {
    format!(
        "Read model not readable at {path}: {cause}. Start a workflow (intent-create) to build it, then run `next` again."
    )
}

// ---------------------------------------------------------------------------
// `continue` の逐語 — fail-closed の完全列挙 (02 §4.4)。
// ---------------------------------------------------------------------------

/// state-aware トークンの `h` 不一致。
pub const STATE_MOVED_ON: &str = "The saved position moved on: the workflow state changed while this stage's rules were being loaded. Run a fresh `next` to restart delivery from part 1.";

/// bundle / directive / route の束縛不一致 (stale)。
pub const STALE_CONTINUATION: &str = "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1.";

/// 存在しない部の要求。
pub const PART_NOT_EXIST: &str = "This request asks for a part of the stage rules that does not exist. Run a fresh `next` to restart delivery from part 1.";

// ---------------------------------------------------------------------------
// `report` の逐語 — 13 段ガードが出す文言 (契約マップ
// `docs/specs/research/orchestration-report-guards.md` §1 が正本、逐語はピン `3c3146cf` の
// `aidlc-orchestrate.ts handleReport` / `handleResumeReport` / `aidlc-lib.ts`)。
// ---------------------------------------------------------------------------

/// 段 1 — 版が読めない状態ファイル (upstream `aidlc-lib.ts:10628-10634` 逐語)。
pub const INCOMPATIBLE_STATE_UNPARSEABLE: &str = "Incompatible workflow state: the State Version field is missing, empty, or unparseable in aidlc-state.md, so this state cannot be matched to the current v8 stage graph and cannot be advanced safely. Archive your workspace ('mv aidlc aidlc.archive') and start a fresh workflow (describe what to build), or finish this workflow on the prior shell. Run `/aidlc --doctor` for the full diagnosis.";

/// 段 1 — この build より新しい版 (upstream `aidlc-lib.ts:10647-10652` 逐語)。
#[must_use]
pub fn incompatible_state_future(version: &str) -> String {
    format!(
        "Incompatible workflow state: State Version {version} is newer than the current v8 stage \
graph this build understands, so it cannot be advanced safely. Upgrade the framework to a build \
that ships state schema v{version} (or newer), or finish this workflow on the shell that produced \
it. Run `/aidlc --doctor` for the full diagnosis."
    )
}

/// 段 1 — この build より古い版 (upstream `aidlc-lib.ts:10658-10666` 逐語)。
#[must_use]
pub fn incompatible_state_past(version: &str) -> String {
    format!(
        "Incompatible workflow state: State Version {version} predates the current v8 stage graph. \
v8 renamed the Inception `application-design` stage to `domain-design` and inserted \
`contract-design`, so this state's stage rows no longer match the graph and cannot be advanced \
safely. Archive your workspace ('mv aidlc aidlc.v{version}-archive') and start a fresh workflow \
(describe what to build), or finish this workflow on the prior shell. Run `/aidlc --doctor` for \
the full diagnosis."
    )
}

/// 段 2 — `--single` に `--result` が無い (upstream `:5266-5270` 逐語)。
#[must_use]
pub fn single_requires_result() -> String {
    format!(
        "report --single requires --result <outcome>. Accepted: {} (the verdict for the single \
stage just run).",
        core_command_domain::orchestration::FORWARD_RESULTS.join(", ")
    )
}

/// 段 2 — `--single` に前進以外の `--result` が来た (upstream `:5274-5277` 逐語)。
#[must_use]
pub fn single_unknown_result(given: &str) -> String {
    format!(
        "Unknown --result \"{given}\". report commits forward outcomes only; accepted: {}.",
        core_command_domain::orchestration::FORWARD_RESULTS.join(", ")
    )
}

/// 段 2 — `--single` に `--stage` が無い (upstream `:5283-5286` 逐語)。
///
/// `next --single` 側の同名ガード ([`SINGLE_REQUIRES_STAGE`]) とは**別の文言**である。
pub const SINGLE_REPORT_REQUIRES_STAGE: &str = "report --single must not advance the main workflow. Pass --stage <slug> to commit the single stage's synthetic-id pair; --single never writes the main workflow's Current Stage.";

/// 段 2 — 隔離実行の対を記録できた (upstream `:5355-5359` 逐語)。
#[must_use]
pub fn single_run_committed(stage: &str) -> String {
    format!(
        "Single-stage run of \"{stage}\" committed under synthetic workflow \
\"single-stage:{stage}\". The main workflow's Current Stage is untouched."
    )
}

/// 段 2 — 対を記録できなかった (upstream `:5346-5349` 逐語)。
///
/// upstream は spawn の stderr / stdout を `detail` に載せ、空なら `"."` で閉じる。
/// こちらの材料はユースケースの失敗の `Display` 連鎖である。
#[must_use]
pub fn single_pair_failed(stage: &str, detail: &str) -> String {
    let detail = detail.trim();
    if detail.is_empty() {
        format!("Failed to record single-stage lifecycle pair for \"{stage}\".")
    } else {
        format!("Failed to record single-stage lifecycle pair for \"{stage}\": {detail}")
    }
}

/// 段 2 — 実行カーソルが無い (upstream に対応する逐語は無い — あちらは監査へ直接追記する)。
///
/// 隔離実行の対も**その intent の記録の中で起きた事実**なので、鋳造前のワークスペースには
/// 書けない (オーナー裁定 2026-09-04 = B)。中継形に材料として載せる。
pub const SINGLE_WITHOUT_EXECUTION: &str = "no active intent record";

/// 段 3 — `--skeleton-stance` の値が閉集合の外 (upstream `:4948-4950` 逐語)。
#[must_use]
pub fn unknown_skeleton_stance(given: &str) -> String {
    format!(
        "Unknown --skeleton-stance \"{given}\". Accepted: on, off, scope-dependent (the \
walking-skeleton stance classified from the team's ## Walking Skeleton prose)."
    )
}

/// 段 3 — 状態ファイルが無い (upstream `:4959` 逐語。ダッシュは U+2014)。
pub const SKELETON_STANCE_WITHOUT_STATE: &str = "No active intent workflow state found (aidlc-state.md is absent) — nothing to record a skeleton stance for.";

/// 段 3 — stance を記録できた (upstream `:5004-5006` 逐語)。
#[must_use]
pub fn skeleton_stance_recorded(stance: &str, stage: &str) -> String {
    format!(
        "Recorded walking-skeleton stance \"{stance}\" for \"{stage}\". \
Re-run `next` to continue — the gate is now determined."
    )
}

/// 段 3 — 現在地が skeleton-gate ステージでない (upstream `:4985-4986` 逐語。ダッシュは U+2014)。
#[must_use]
pub fn not_the_skeleton_gate(stage: &str, scope: &str) -> String {
    format!(
        "Current stage \"{stage}\" is not the skeleton-gate stage for scope \"{scope}\" — \
a skeleton stance is only reported for the first Construction Bolt's gate."
    )
}

/// 段 3 — 記録に失敗した (upstream `:4998-5000` 逐語)。
#[must_use]
pub fn skeleton_stance_failed(stage: &str, detail: &str) -> String {
    let detail = detail.trim();
    if detail.is_empty() {
        format!("Failed to record skeleton stance for \"{stage}\".")
    } else {
        format!("Failed to record skeleton stance for \"{stage}\": {detail}")
    }
}

/// 段 4 — 再開の報告に `--stage` が付いた (upstream `:5388-5389` 逐語)。
pub const RESUME_TAKES_NO_STAGE: &str =
    "A resume-choice report is not a stage transition; omit --stage.";

/// 段 4 — `--user-input` が無い (upstream `:5394-5395` 逐語)。
pub const RESUME_REQUIRES_USER_INPUT: &str =
    "report --result resumed requires --user-input with the human's resume choice.";

/// 段 4 — 状態ファイルが無い (upstream `:5403` 逐語。ダッシュは**ASCII の** `-`)。
pub const RESUME_WITHOUT_STATE: &str =
    "No active intent workflow state found (aidlc-state.md is absent) - nothing to resume.";

/// 段 4 — `Current Stage` が読めない (upstream `:5410` 逐語。ダッシュは ASCII の `-`)。
pub const RESUME_WITHOUT_CURRENT_STAGE: &str =
    "State file has no Current Stage field - cannot resume from the last checkpoint.";

/// 段 4 の選択肢 2 — やり直し (upstream `:5428` 逐語)。
///
/// 命令の綴りは**逸脱台帳 #1 の写像**である。upstream は
/// `bun <harness>/tools/aidlc-jump.ts execute --target …` を名指すが、こちらはマルチコールの
/// 正準形 `aidlc-jump` を名指す（`next/stage-jump-print` と同じ扱い —
/// `cli_golden_test.rs` の「駆動できないケース」を参照）。
#[must_use]
pub fn resume_redo(stage: &str, scope: &str) -> String {
    format!(
        "Redo accepted at \"{stage}\". Run `aidlc-jump execute --target {stage} --direction redo \
--scope {scope}` to reset the current stage, then re-run `next` to start it over."
    )
}

/// 段 4 の選択肢 3 — ジャンプ (upstream `:5434` 逐語)。
pub const RESUME_JUMP: &str = "Jump accepted. Ask the human which stage to jump to, then re-run `next --stage <slug>`; the direction and the target are worked out and checked for you.";

/// 段 4 の選択肢 4 — 新規開始 (upstream `:5440` 逐語)。
pub const RESUME_START_FRESH: &str = "Start-fresh accepted. Confirm the new work's scope and description with the human, then run `next --new-intent --scope <scope> \"<description>\"` — the existing workflow stays in place and the new intent starts alongside it.";

/// 段 4 の選択肢 1 — チェックポイントからの再開 (upstream `:5450` 逐語)。
#[must_use]
pub fn resume_from_checkpoint(stage: &str) -> String {
    format!(
        "Resume choice accepted at \"{stage}\". Re-run `next` to continue from the last checkpoint."
    )
}

/// 段 4 — どの選択肢にも当たらない (upstream `:5455` 逐語)。
///
/// 埋めるのは**正規化前の生値**である（upstream も `flags.userInput` をそのまま埋める）。
#[must_use]
pub fn unrecognized_resume_choice(given: &str) -> String {
    format!(
        "Unrecognized resume choice \"{given}\". Accepted choices: 1/resume from last checkpoint, \
2/redo the current stage, 3/jump to a stage, or 4/start fresh."
    )
}

/// 段 5 — `--result` が無い (upstream `:5529-5531` 逐語)。
#[must_use]
pub fn report_requires_result() -> String {
    format!(
        "report requires --result <outcome>. Accepted: {} (the verdict for the stage just acted on).",
        core_command_domain::orchestration::ACCEPTED_RESULTS.join(", ")
    )
}

/// 段 6 — 実行がまだ鋳造されていない (upstream `:5551` 逐語。ダッシュは U+2014)。
pub const REPORT_WITHOUT_STATE: &str = "No active intent workflow state found (aidlc-state.md is absent) — nothing to report a transition for.";

/// 段 7〜8 — 名指しされたステージが解決できない (upstream `:5588` 逐語。ダッシュは U+2014)。
///
/// upstream は「グラフに無い」(`:5588`) と「状態ファイルに行が無い」(`:5596`) を分けるが、
/// こちらの計画は 1 つ（intent が持つ解決済み計画）なので同じ文言に落ちる。slug の文法から
/// 外れた `--stage` もここで断る。
#[must_use]
pub fn reported_stage_not_in_graph(given: &str) -> String {
    format!(
        "Internal: reported stage \"{given}\" is not in the compiled graph — cannot commit its transition."
    )
}

/// 段 9 — `skipped` に明示の `--stage` が無い (upstream `:5609` 逐語)。
pub const SKIP_REQUIRES_EXPLICIT_STAGE: &str =
    "report --result skipped requires an explicit nonblank --stage <slug>.";

/// 段 9 — CONDITIONAL でも実効 SKIP でもない (upstream `:5616` 逐語)。
#[must_use]
pub fn skip_not_conditional(stage: &str, execution: &str) -> String {
    format!(
        "Stage \"{stage}\" is execution: {execution}; only a CONDITIONAL stage can report skipped."
    )
}

/// 段 9 — `--reason` が空 (upstream `:5623` 逐語)。
pub const SKIP_REQUIRES_REASON: &str =
    "report --result skipped requires a nonblank --reason <text>.";

/// 段 9 — カーソル以外を名指しした (upstream `:5629-5630` 逐語)。
#[must_use]
pub fn skip_must_name_cursor(stage: &str, current: &str) -> String {
    format!(
        "Cannot skip stage \"{stage}\": Current Stage is \"{current}\". \
A skip report must name the active stage exactly."
    )
}

/// 段 9 — checkbox が受理集合の外 (upstream `:5640` 逐語)。
#[must_use]
pub fn skip_precondition(stage: &str, state: &str) -> String {
    format!(
        "Stage \"{stage}\" is {state}; only an active, revising, or interrupted skipped stage can be routed as skipped."
    )
}

/// 段 10 — 非ゲートステージが gate 系を名乗った (upstream `:5677` 逐語)。
#[must_use]
pub fn ungated_stage(stage: &str, result: &str) -> String {
    format!("Stage \"{stage}\" is an ungated initialization stage; it cannot report {result}.")
}

/// 段 10 — `awaiting-approval` の前提違反 (upstream `:5706` 逐語)。
#[must_use]
pub fn gate_open_precondition(stage: &str, state: &str) -> String {
    format!("Stage \"{stage}\" is {state}; only an in-progress stage can open a gate.")
}

/// 段 10 — `rejected` の前提違反 (upstream `:5717` 逐語)。
#[must_use]
pub fn gate_reject_precondition(stage: &str, state: &str) -> String {
    format!(
        "Stage \"{stage}\" is {state}; only an active or awaiting-approval stage can be rejected."
    )
}

/// 段 10 — `revised` の前提違反 (upstream `:5732` 逐語)。
#[must_use]
pub fn gate_revise_precondition(stage: &str, state: &str) -> String {
    format!("Stage \"{stage}\" is {state}; only a revising stage can re-enter its gate.")
}

/// 段 10 — `rejected` にフィードバックが無い (upstream `:5724` 逐語)。
#[must_use]
pub fn reject_requires_feedback(stage: &str) -> String {
    format!(
        "report --result rejected for \"{stage}\" requires nonblank --user-input or --reason feedback."
    )
}

/// 段 13 — 人間の選択が無い (upstream `:5794` 逐語)。
#[must_use]
pub fn human_presence_required(result: &str, stage: &str) -> String {
    format!(
        "report --result {result} for \"{stage}\" requires --user-input with the human's exact approval choice."
    )
}

/// forward 表 — `[S]` / `[R]` は前進の完了ではない (upstream `:5815` 逐語)。
#[must_use]
pub fn forward_commits_completions_only(stage: &str, state: &str) -> String {
    format!("Stage \"{stage}\" is {state}; report commits forward completions only.")
}

/// forward 表 — `[ ]` はまだ走っていない (upstream `:5823` 逐語)。
#[must_use]
pub fn still_pending(stage: &str) -> String {
    format!("Stage \"{stage}\" is still pending. Run the stage before reporting it complete.")
}

/// forward 表 — ゲート未開放の `[-]` は明示 `--stage` を要する (upstream `:5868-5870` 逐語)。
#[must_use]
pub fn in_progress_requires_explicit_stage(stage: &str) -> String {
    format!(
        "Stage \"{stage}\" is still in-progress. To approve a gated stage that has not entered \
awaiting-approval, report the acted directive explicitly with --stage \"{stage}\" so the engine \
cannot mistake a freshly advanced Current Stage for the completed one."
    )
}

/// 遷移をコミットしない結末が集約まで届いた（upstream に対応する逐語は無い）。
///
/// 合成ルートは段 4 で `resume` / `resumed` を振り分けるので、通常は到達しない。
pub const RESUME_IS_ROUTED: &str = "Resume is routed, not committed. Run a fresh `next --resume`.";

/// 成功 — gate 系 3 語 (upstream `:5748-5750` 逐語)。
#[must_use]
pub fn recorded_result(result: &str, stage: &str) -> String {
    format!("Recorded {result} for \"{stage}\".")
}

/// 成功 — ルーティングされた読み飛ばし (upstream `:5662-5664` 逐語)。
#[must_use]
pub fn committed_skip(stage: &str, scope: &str) -> String {
    format!(
        "Committed skip for \"{stage}\" (scope: {scope}). State routed forward; run next to continue."
    )
}

/// 成功 — 前進 (upstream `:5923-5925` 逐語)。`subs` は段の綴りを ` + ` で継いだもの。
#[must_use]
pub fn committed_transition(subs: &str, stage: &str, scope: &str) -> String {
    format!(
        "Committed {subs} for \"{stage}\" (scope: {scope}). State advanced; run next to continue."
    )
}

/// no-op — 既に開いているゲート (upstream `:5701` 逐語)。
#[must_use]
pub fn already_awaiting_approval(stage: &str) -> String {
    format!("Stage \"{stage}\" is already awaiting approval.")
}

/// no-op — カーソルが先へ移った通過済みステージ (upstream `:5855-5856` 逐語)。
#[must_use]
pub fn already_completed_moved_on(stage: &str, current: &str, scope: &str) -> String {
    format!(
        "Stage \"{stage}\" is already completed and the workflow has moved on to \"{current}\" \
(scope: {scope}); idempotent re-report, no transition needed."
    )
}

/// no-op — 完了済みワークフロー (upstream `:5834` 逐語 + `NEW_WORK_HINT`)。
#[must_use]
pub fn workflow_already_completed(stage: &str, scope: &str) -> String {
    format!(
        "Workflow is already completed at \"{stage}\" (scope: {scope}); no transition was needed.{NEW_WORK_HINT}"
    )
}

/// 遷移が集約に拒否された (upstream `:5903-5904` 逐語 — 中継形)。
///
/// upstream は spawn 先の非ゼロ終了の出力をそのまま挟み、出力が空なら `.` で閉じる。
#[must_use]
pub fn transition_rejected_by(sub: &str, stage: &str, detail: &str) -> String {
    let tail = if detail.is_empty() {
        ".".to_string()
    } else {
        format!(": {detail}")
    };
    format!("Transition rejected by aidlc-state.ts {sub} for \"{stage}\"{tail}")
}

/// 判断が名指しした段に対応する集約コマンドが**この build に無い**。
///
/// upstream に対応する逐語は無い — あちらは `advance` / `complete-workflow` を持っている。
/// こちらは非ゲート完了のパイプラインを b42 で撤去した（#85 = A）ので、初期化ステージだけが
/// in-scope の縮退計画でだけこの断りが出る。b47 の未配線 2 形と同じ言い回しに揃えてある。
#[must_use]
pub fn transition_not_wired(sub: &str, stage: &str) -> String {
    format!("Cannot commit {sub} for \"{stage}\": the {sub} transition is not wired in this build.")
}

// ---------------------------------------------------------------------------
// `aidlc-log`（対話イベントの記録面 — b48 / B10）
// ---------------------------------------------------------------------------

/// 記録面の未知サブコマンド（upstream `aidlc-log.ts:1206` 逐語）。
#[must_use]
pub fn unknown_log_subcommand(given: Option<&str>) -> String {
    format!(
        "Unknown subcommand: {}. Valid: decision, answer, link, review",
        given.unwrap_or("undefined")
    )
}

/// 認識はするが**この build に無い**記録動詞（own wording）。
///
/// upstream に対応する逐語は無い — あちらは 4 動詞すべてを持つ。b46 が導入した
/// 「not wired in this build」の言い回しに揃えてある（[`transition_not_wired`] と同型）。
#[must_use]
pub fn log_verb_not_wired(verb: &str) -> String {
    format!(
        "Cannot record a {verb} event: the aidlc-log {verb} verb is not wired in this build. \
Only `review` is available."
    )
}

/// 値が必要なフラグに値が無い（upstream `parseFlags` `:106` 逐語）。
#[must_use]
pub fn flag_expects_a_value(flag: &str) -> String {
    format!("{flag} expects a value, got end of arguments.")
}

/// 値が必要なフラグの次がまたフラグだった（同 `:110` 逐語）。
#[must_use]
pub fn flag_expects_a_value_got_flag(flag: &str, value: &str) -> String {
    format!("{flag} expects a value, got another flag: \"{value}\". Did you forget the value?")
}

/// `--stage` が無い（upstream `handleReview` `:902` 逐語）。
pub const REVIEW_REQUIRES_STAGE: &str = "Missing --stage <slug>";

/// `--reviewer` が無い（同 `:903` 逐語）。
pub const REVIEW_REQUIRES_REVIEWER: &str = "Missing --reviewer <agent>";

/// `--intent` / `--space` セレクタは受け付けない（同 `:906` 逐語）。
pub const REVIEW_TAKES_NO_SELECTORS: &str = "The review command does not accept --intent/--space selectors. Switch to the target workspace first.";

/// アクティブな intent が解決できない（同 `:914` 逐語）。
pub const REVIEW_WITHOUT_INTENT: &str = "Cannot resolve the active intent for review logging.";

/// per-unit の受領証は**この build に無い**（own wording）。
///
/// upstream の `--unit` は per-unit ステージの受領証を 1 unit ごとに数えるが、unit の
/// ライフサイクル自体が本 build には無い（slice 2）。b46 の「not wired in this build」に揃える。
pub const REVIEW_UNIT_NOT_WIRED: &str = "Cannot record a per-unit review: the --unit receipt is not wired in this build. Record the stage-level review instead (omit --unit).";

/// 隔離実行の受領証は**この build に無い**（own wording）。
///
/// upstream の `--single` は受領証を疑似ワークフローへ閉じ込めるが、そのためには試行の
/// 区切りを `Workflow` ごとに分ける会計が要る（slice 2）。
pub const REVIEW_SINGLE_NOT_WIRED: &str = "Cannot record a single-stage review: the --single receipt is not wired in this build. An isolated run records no review receipt.";

/// 依頼形に `--iteration` が無い／正整数でない（upstream `:985` 逐語）。
pub const REVIEW_REQUEST_REQUIRES_ITERATION: &str =
    "REVIEW_REQUESTED requires --iteration <positive integer>.";

/// `--retry-pending` と `--verdict` の併用（同 `:1122` 逐語）。
pub const REVIEW_RETRY_WITH_VERDICT: &str = "--retry-pending cannot be combined with --verdict.";

/// 判定形に `--iteration` が無い／正整数でない（同 `:1125` 逐語）。
pub const REVIEW_COMPLETED_REQUIRES_ITERATION: &str =
    "REVIEW_COMPLETED requires --iteration <positive integer>.";

/// `--verdict` が閉集合の外（同 `:1131-1133` 逐語 — 一覧は `VALID_VERDICTS` の挿入順）。
#[must_use]
pub fn unknown_review_verdict(given: &str) -> String {
    format!("Unknown --verdict \"{given}\". Accepted: READY, NOT-READY.")
}

/// レビュアー宣言が無いステージ（同 `:928` 逐語）。
#[must_use]
pub fn stage_has_no_declared_reviewer(stage: &str) -> String {
    format!("Cannot record review: stage \"{stage}\" has no declared reviewer.")
}

/// `--reviewer` が宣言と食い違う（同 `:931-934` 逐語）。
#[must_use]
pub fn reviewer_does_not_match(stage: &str, given: &str, declared: &str) -> String {
    format!(
        "Cannot record review for \"{stage}\": reviewer \"{given}\" does not match the \
declared reviewer \"{declared}\"."
    )
}

/// 依頼がレビュー予算を超えた（upstream `reviewBudgetMessage` `:833-843` 逐語 — 2 形）。
#[must_use]
pub fn review_budget_exceeded(stage: &str, ordinal: u32, budget: u32) -> String {
    let tail = if budget == 1 {
        "This review runs as a single advisory pass - do not re-invoke the reviewer; \
quote its findings at the approval gate for the human to triage."
    } else {
        "The review loop is exhausted - present the gate with the unresolved findings \
for the human's decision instead of another review pass."
    };
    format!(
        "Refusing REVIEW_REQUESTED for \"{stage}\": review request {ordinal} exceeds \
this stage's review budget ({budget}). {tail}"
    )
}

/// 依頼の通し番号が順序と合わない（upstream `:1098-1100` 逐語）。
#[must_use]
pub fn review_out_of_sequence(stage: &str, iteration: u32, expected: u32) -> String {
    format!(
        "Refusing REVIEW_REQUESTED for \"{stage}\": iteration {iteration} is out of sequence; \
expected {expected} from the current audit attempt."
    )
}

/// 判定形に対応する依頼が無い（upstream `:1142-1144` 逐語）。
#[must_use]
pub fn review_completed_without_request(stage: &str, iteration: u32) -> String {
    format!(
        "Refusing REVIEW_COMPLETED for \"{stage}\": no unmatched REVIEW_REQUESTED \
iteration {iteration} exists in the current audit attempt."
    )
}

/// retry 形に対応する依頼が無い（upstream `:1054-1055` 逐語）。
#[must_use]
pub fn review_retry_without_request(stage: &str, iteration: u32) -> String {
    format!(
        "Refusing review retry for \"{stage}\": no unmatched REVIEW_REQUESTED \
iteration {iteration} exists in the current audit attempt."
    )
}

/// 段 11 — レビュアーを宣言したステージに終端受領証が無い
/// （upstream `reviewerPreconditionError` `aidlc-state.ts:2026-2037` 逐語）。
///
/// この文言は `aidlc-state.ts approve` の stderr であり、`report` からは
/// [`transition_rejected_by`] の包み文の中に現れる（b46 の既存形）。
#[must_use]
pub fn reviewer_precondition(stage: &str, reviewer: &str) -> String {
    format!(
        "Refusing to complete \"{stage}\": it declares a reviewer ({reviewer}) but no fresh \
REVIEW_COMPLETED is recorded for it. Invoke the reviewer (stage-protocol-reviewer.md §12a) and \
record the verdict with `aidlc-log.ts review --stage {stage} --reviewer {reviewer} --verdict \
<READY|NOT-READY>` before completing. Terminal ordering: apply any fixes FIRST, then run the \
reviewer, record the receipt, and stop editing produces[] artifacts - a later write to one \
invalidates the receipt and re-opens this refusal. Do not apply suggestions riding on a READY \
verdict; surface them at the gate instead."
    )
}

/// 受領証の記録に失敗した（own wording — 中継形）。
///
/// upstream は `emitError` が `ERROR_LOGGED` 行を描いてから stderr に出すが、本 build は
/// その行を描かない（既存の拒否と同じ扱い、逸脱台帳）。
#[must_use]
pub fn review_log_failed(stage: &str, detail: &str) -> String {
    let detail = detail.trim();
    if detail.is_empty() {
        format!("Failed to record the review receipt for \"{stage}\".")
    } else {
        format!("Failed to record the review receipt for \"{stage}\": {detail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 材料が空なら文を「.」で閉じ、在れば「: 材料」で続ける（upstream の三項）。
    #[test]
    fn the_two_b47_failure_lines_close_with_a_period_when_there_is_no_material() {
        assert_eq!(
            single_pair_failed("contract-design", "  "),
            "Failed to record single-stage lifecycle pair for \"contract-design\"."
        );
        assert_eq!(
            single_pair_failed("contract-design", "disk full"),
            "Failed to record single-stage lifecycle pair for \"contract-design\": disk full"
        );
        assert_eq!(
            skeleton_stance_failed("functional-design", ""),
            "Failed to record skeleton stance for \"functional-design\"."
        );
        assert_eq!(
            skeleton_stance_failed("functional-design", "disk full"),
            "Failed to record skeleton stance for \"functional-design\": disk full"
        );
    }

    // ---- b48: レビュー受領証の逐語 ----

    /// 記録面の未知動詞は upstream の `undefined` まで含めて逐語である（`:1206`）。
    #[test]
    fn the_unknown_log_subcommand_line_names_the_given_verb_or_undefined() {
        assert_eq!(
            unknown_log_subcommand(Some("frobnicate")),
            "Unknown subcommand: frobnicate. Valid: decision, answer, link, review"
        );
        assert_eq!(
            unknown_log_subcommand(None),
            "Unknown subcommand: undefined. Valid: decision, answer, link, review"
        );
    }

    /// 予算超過は 2 形 — budget 1 は advisory の言い回し、それ以外は反駁ループの言い回し。
    #[test]
    fn the_budget_refusal_switches_its_tail_at_a_budget_of_one() {
        let advisory = review_budget_exceeded("domain-design", 2, 1);
        assert!(
            advisory.starts_with(
                "Refusing REVIEW_REQUESTED for \"domain-design\": review request 2 exceeds \
this stage's review budget (1). "
            ),
            "{advisory}"
        );
        assert!(
            advisory.ends_with(
                "This review runs as a single advisory pass - do not re-invoke the reviewer; \
quote its findings at the approval gate for the human to triage."
            ),
            "{advisory}"
        );

        let adversarial = review_budget_exceeded("domain-design", 3, 2);
        assert!(
            adversarial.ends_with(
                "The review loop is exhausted - present the gate with the unresolved findings \
for the human's decision instead of another review pass."
            ),
            "{adversarial}"
        );
    }

    /// `NoPendingReview` は動詞で言い回しが分かれる（判定形 / 呼び直し形）。
    #[test]
    fn the_unmatched_request_refusal_differs_between_the_verdict_and_the_retry_form() {
        assert_eq!(
            review_completed_without_request("domain-design", 1),
            "Refusing REVIEW_COMPLETED for \"domain-design\": no unmatched REVIEW_REQUESTED \
iteration 1 exists in the current audit attempt."
        );
        assert_eq!(
            review_retry_without_request("domain-design", 1),
            "Refusing review retry for \"domain-design\": no unmatched REVIEW_REQUESTED \
iteration 1 exists in the current audit attempt."
        );
    }

    /// 順序違反は要求値と期待値の両方を名指しする。
    #[test]
    fn the_out_of_sequence_refusal_names_both_ordinals() {
        assert_eq!(
            review_out_of_sequence("domain-design", 3, 1),
            "Refusing REVIEW_REQUESTED for \"domain-design\": iteration 3 is out of sequence; \
expected 1 from the current audit attempt."
        );
    }

    /// 宣言不一致は「与えられた名前」と「宣言された名前」を並べる。
    #[test]
    fn the_reviewer_mismatch_names_both_the_given_and_the_declared_agent() {
        assert_eq!(
            reviewer_does_not_match("domain-design", "me", "aidlc-quality-agent"),
            "Cannot record review for \"domain-design\": reviewer \"me\" does not match the \
declared reviewer \"aidlc-quality-agent\"."
        );
    }

    /// 段 11 の逐語は `aidlc-state.ts approve` の stderr そのものである（`:2026-2037`）。
    #[test]
    fn the_reviewer_precondition_is_the_upstream_state_refusal_verbatim() {
        let message = reviewer_precondition("domain-design", "aidlc-quality-agent");
        assert!(
            message.starts_with(
                "Refusing to complete \"domain-design\": it declares a reviewer \
(aidlc-quality-agent) but no fresh REVIEW_COMPLETED is recorded for it."
            ),
            "{message}"
        );
        assert!(
            message.contains(
                "record the verdict with `aidlc-log.ts review --stage domain-design --reviewer \
aidlc-quality-agent --verdict <READY|NOT-READY>` before completing."
            ),
            "{message}"
        );
        assert!(
            message.ends_with(
                "Do not apply suggestions riding on a READY verdict; surface them at the gate \
instead."
            ),
            "{message}"
        );
    }

    /// 記録の失敗は材料が空なら「.」で閉じる（b47 の 2 形と同じ作法）。
    #[test]
    fn the_review_log_failure_closes_with_a_period_when_there_is_no_material() {
        assert_eq!(
            review_log_failed("domain-design", "  "),
            "Failed to record the review receipt for \"domain-design\"."
        );
        assert_eq!(
            review_log_failed("domain-design", "disk full"),
            "Failed to record the review receipt for \"domain-design\": disk full"
        );
    }

    /// フラグ文法の 2 形は upstream の `parseFlags` そのままである。
    #[test]
    fn the_flag_grammar_refusals_are_verbatim() {
        assert_eq!(
            flag_expects_a_value("--stage"),
            "--stage expects a value, got end of arguments."
        );
        assert_eq!(
            flag_expects_a_value_got_flag("--stage", "--reviewer"),
            "--stage expects a value, got another flag: \"--reviewer\". Did you forget the value?"
        );
    }

    /// 引数が無いときは `(none)` を描く（upstream の `?? "(none)"`）。
    #[test]
    fn the_unknown_subcommand_line_names_the_given_verb_or_none() {
        assert_eq!(
            unknown_orchestrate_subcommand(Some("frobnicate")),
            "Unknown subcommand: frobnicate. Valid: next, continue, report, park"
        );
        assert_eq!(
            unknown_orchestrate_subcommand(None),
            "Unknown subcommand: (none). Valid: next, continue, report, park"
        );
    }

    #[test]
    fn the_oversize_refusal_names_the_cap_in_bytes() {
        assert_eq!(
            refusing_oversize_directive(28 * 1024),
            "aidlc-orchestrate: refusing to emit a directive larger than 28672 bytes"
        );
    }

    #[test]
    fn the_failure_line_is_prefixed_with_the_tool_name() {
        assert_eq!(
            orchestrate_failure("missing graph"),
            "aidlc-orchestrate: missing graph"
        );
    }

    /// 鍵の 3 形は path を二重引用符で囲む（upstream の `"${path}"`）。
    #[test]
    fn the_key_file_wordings_quote_the_path() {
        assert!(
            corrupt_key_file("/tmp/k").starts_with("The local key file at \"/tmp/k\" is corrupt")
        );
        assert!(unreadable_key_file("/tmp/k", "EACCES").contains("\"/tmp/k\""));
        assert!(uncreatable_key_file("/tmp/k", "EACCES").contains("\"/tmp/k\""));
    }

    /// 遷移拒否は理由を前置きの後ろへそのまま運ぶ。
    #[test]
    fn a_rejected_transition_carries_its_detail() {
        assert_eq!(
            transition_rejected("stage 3 is not the cursor"),
            "Transition rejected: stage 3 is not the cursor"
        );
    }

    /// 空間名の拒否は値を二重引用符で囲み、直し方を名指しする。
    #[test]
    fn the_invalid_active_space_wording_names_the_cursor_file() {
        let message = invalid_active_space("../escape");
        assert!(
            message.starts_with("The active space \"../escape\""),
            "{message}"
        );
        assert!(message.contains("aidlc/active-space"), "{message}");
    }

    /// 壊れた実行カーソルの文言は、原因（材料）と次の一手の両方を運ぶ。
    #[test]
    fn the_unreadable_execution_cursor_wording_carries_its_cause_and_the_recovery() {
        let message =
            unreadable_execution_cursor("malformed execution cursor at /w/record/.aidlc-execution");
        assert!(
            message.starts_with("The execution cursor cannot be read ("),
            "{message}"
        );
        assert!(
            message.contains("malformed execution cursor at /w/record/.aidlc-execution"),
            "{message}"
        );
        assert!(message.contains("aidlc-utility intent-create"), "{message}");
    }

    /// 閉集合外の `--review` は upstream の逐語で拒む。
    #[test]
    fn the_unknown_review_class_wording_is_verbatim() {
        assert_eq!(
            unknown_review_class("strict"),
            "Unknown review class: \"strict\". Valid: adversarial, advisory, none."
        );
    }

    #[test]
    fn the_key_file_wordings_end_with_their_recovery_instruction() {
        assert!(corrupt_key_file("/tmp/k").ends_with("a replacement is created automatically."));
        assert!(unreadable_key_file("/tmp/k", "EACCES").ends_with("(EACCES)."));
        assert!(
            uncreatable_key_file("/tmp/k", "EACCES")
                .ends_with("Fix the directory permissions, then run a fresh `next`.")
        );
    }

    /// park の文言はステージ名を名乗り、再開の綴りを添える。
    #[test]
    fn the_parked_wording_names_the_stage_and_the_resume_spelling() {
        assert_eq!(
            parked("domain-design"),
            "Workflow parked at \"domain-design\". Resume with /aidlc --resume."
        );
    }

    /// park の失敗は中継形に包まれ、upstream 逐語 2 形をそのまま運ぶ。
    #[test]
    fn the_park_refusals_are_relayed_verbatim_inside_the_wrapper() {
        assert_eq!(
            park_refused(PARK_REFUSED_AUTONOMOUS),
            "Cannot park the workflow: Refusing to park: Construction Autonomy Mode is autonomous. \
An unattended autonomous run has no human to resume it and must keep moving - do not park it."
        );
        assert_eq!(
            park_refused(PARK_NOTHING_TO_PARK),
            "Cannot park the workflow: Workflow is already Completed - nothing to park."
        );
        assert_eq!(
            park_refused(PARK_WITHOUT_EXECUTION),
            "Cannot park the workflow: No workflow execution to park. Run `next` first."
        );
        // 材料が何であれ包み方は 1 つである (upstream は spawn の出力をそのまま挟む)。
        assert_eq!(
            park_refused("repository: conflict"),
            "Cannot park the workflow: repository: conflict"
        );
    }

    /// park 中の `--resume` は「先に park を外せ」と綴りごと言う。
    #[test]
    fn the_unpark_wording_names_the_command_before_the_retry() {
        assert_eq!(
            unpark_then_resume("aidlc-orchestrate unpark"),
            "This workflow is parked. Run `aidlc-orchestrate unpark` to clear the park marker, \
then re-run `next --resume` to continue."
        );
    }

    /// 未知 scope の拒否は有効 scope を綴り順のまま並べる。
    #[test]
    fn the_unknown_scope_wording_lists_the_valid_scopes_in_the_given_order() {
        assert_eq!(
            unknown_scope("nope", &["classic".to_string(), "express".to_string()]),
            "Unknown scope \"nope\". Valid scopes: classic, express."
        );
    }

    /// 環境変数の既定 scope が未知なら、変数名まで名乗って拒否する。
    #[test]
    fn the_invalid_env_scope_wording_names_the_environment_variable() {
        assert_eq!(
            invalid_env_scope("nope", &["classic".to_string()]),
            "Invalid AWS_AIDLC_DEFAULT_SCOPE \"nope\". Valid scopes: classic."
        );
    }

    /// per-unit が 0 のときコスト節に反復の節は付かない。
    #[test]
    fn the_cost_clause_omits_the_per_unit_phrase_when_no_stage_repeats() {
        assert_eq!(cost_clause(12, 9, 4, 0), "9 of 12 stages, 4 approval gates");
    }

    /// 反復が 1 段なら単数形、2 段以上なら複数形になる。
    #[test]
    fn the_cost_clause_switches_between_the_singular_and_plural_per_unit_phrase() {
        assert_eq!(
            cost_clause(12, 9, 4, 1),
            "9 of 12 stages, 4 approval gates, 1 stage repeats per unit of work in Construction"
        );
        assert_eq!(
            cost_clause(12, 9, 4, 3),
            "9 of 12 stages, 4 approval gates, 3 stages repeat per unit of work in Construction"
        );
    }

    /// フェーズに in-scope のステージが無いときは、そのフェーズ名を名乗る。
    #[test]
    fn the_empty_phase_wording_names_the_phase() {
        assert_eq!(
            no_stage_in_phase("operation"),
            "No in-scope stage found for phase \"operation\"."
        );
    }

    /// scope 確認はコスト節を ` - ` で継ぐ（誕生 print の括弧ではない）。
    #[test]
    fn the_scope_confirm_wording_appends_the_cost_clause_with_a_dash() {
        assert_eq!(
            scope_confirm(
                "bugfix",
                "fix the login crash",
                " - 4 of 9 stages, 1 approval gates"
            ),
            "This looks like \"bugfix\" work, so I'd run the \"bugfix\" plan for: \
\"fix the login crash\" - 4 of 9 stages, 1 approval gates. \
Say go ahead, name a different plan, or say \"compose\" and I'll tailor one to this task."
        );
    }

    /// compose 提案は本文と既製 scope の例を並べる。
    #[test]
    fn the_compose_offer_wording_carries_the_text_and_the_stock_examples() {
        assert_eq!(
            compose_offer("do something odd", "\"express\", \"classic\""),
            "None of the ready-made plans is an obvious fit for: \"do something odd\". \
I can work out a plan tailored to this task (recommended: reply \"compose\"), \
or you can pick one directly (e.g. \"express\", \"classic\"; see /aidlc --help for the full list)."
        );
    }

    /// 読み取り専用ユーティリティは「ワークフローではない」と明示する。
    #[test]
    fn the_read_only_wording_forbids_advancing_the_workflow() {
        assert_eq!(
            read_only("aidlc-utility status"),
            "Run `aidlc-utility status`, print its output verbatim, then stop. \
This is a read-only utility, NOT workflow work: do NOT run `next` and do NOT advance, resume, or run any workflow stage."
        );
    }

    /// 終端ユーティリティは読み取り専用と 1 語だけ違う（`terminal`）。
    #[test]
    fn the_terminal_utility_wording_differs_from_the_read_only_one_by_a_single_word() {
        let terminal = terminal_utility("aidlc-utility intent list");
        assert!(
            terminal.contains("This is a terminal utility, NOT workflow work"),
            "{terminal}"
        );
        assert_eq!(
            terminal.replace("a terminal utility", "a read-only utility"),
            read_only("aidlc-utility intent list")
        );
    }

    /// 回復可能な SKIP 不整合は、走らせるなと言い回復の綴りを名乗る。
    #[test]
    fn the_recover_skip_wording_names_the_stage_and_the_recovery_command() {
        assert_eq!(
            recover_skip(
                "contract-design",
                "aidlc-orchestrate report --result skipped"
            ),
            "Stage \"contract-design\" is SKIP in the approved workflow plan but is still the active cursor. \
Do not run this stage. Run `aidlc-orchestrate report --result skipped` to recover the stale pointer, \
then re-run `next` to continue."
        );
    }

    /// 回復経路の無い SKIP 不整合は、カーソルの綴りをそのまま埋めて拒否する。
    #[test]
    fn the_inconsistent_skip_wording_quotes_the_checkbox_spelling() {
        assert_eq!(
            inconsistent_skip("contract-design", "in-progress"),
            "Stage \"contract-design\" is SKIP in the approved workflow plan but its active cursor state is \"in-progress\". Refusing to emit run-stage; repair the inconsistent state before continuing."
        );
    }

    /// stage-graph が読めないときは、所在と原因の両方を材料として置く。
    #[test]
    fn the_stage_graph_wording_carries_both_the_path_and_the_cause() {
        assert_eq!(
            stage_graph_not_readable("/w/.claude/tools/data/stage-graph.json", "not projected"),
            "Stage graph not readable at /w/.claude/tools/data/stage-graph.json: not projected. \
Reinstall the framework or re-run setup to restore the data file."
        );
    }

    /// リードモデルが引けないときも材料（所在と分類）だけを運ぶ。
    #[test]
    fn the_read_model_wording_carries_both_the_path_and_the_cause() {
        assert_eq!(
            read_model_unreadable("/w/aidlc/spaces/default/read-model.sqlite3", "unreadable"),
            "Read model not readable at /w/aidlc/spaces/default/read-model.sqlite3: unreadable. \
Start a workflow (intent-create) to build it, then run `next` again."
        );
    }
}
