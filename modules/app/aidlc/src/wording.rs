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

/// 分岐 4b — `--single` にステージが無い。
pub const SINGLE_REQUIRES_STAGE: &str = "--single requires --stage <slug>.";

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

#[cfg(test)]
mod tests {
    use super::*;

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
