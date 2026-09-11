//! Claude SessionStartのadditionalContextを組み立てる純粋な描画部品。
use crate::{SessionContextNotices, SessionWorkflowFields};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};

/// 表示することが既に決まったSessionStart文面。権限や帰属は判断しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStartContext {
    text: String,
}
impl SessionStartContext {
    const fn new(text: String) -> Self {
        Self { text }
    }
    /// 実行中ワークフローの表示文面。
    #[must_use]
    pub fn workflow(
        fields: &SessionWorkflowFields,
        session: &str,
        notices: &SessionContextNotices,
    ) -> Self {
        let session = if session.is_empty() {
            "(unavailable)"
        } else {
            session
        };
        let recovery = if notices.recovery_present() {
            "NOTE: A compaction recovery breadcrumb exists at .aidlc-recovery.md — check if state was preserved correctly.\n"
        } else {
            ""
        };
        let drift = if notices.uncompiled_stages().is_empty() {
            String::new()
        } else {
            format!(
                "NOTE: {} stage file(s) on disk are not in the compiled stage graph and will NOT execute: {}. Run `bun .claude/tools/aidlc-graph.ts compile` to include them, then start a fresh workflow (an in-flight workflow keeps its original stage set).\n",
                notices.uncompiled_stages().len(),
                notices.uncompiled_stages().join(", ")
            )
        };
        Self::new(format!(
            "AIDLC WORKFLOW ACTIVE\n{}Scope: {}\nRuntime Session: {session}\nLifecycle Phase: {}\nCurrent Stage: {}\nStatus: {}\nActive Agent: {}\nLast Completed: {}\nNext Action: {}\n{}{recovery}{drift}{FORWARDING}",
            notices.rebind_offer(),
            fields.scope(),
            fields.phase(),
            fields.stage(),
            fields.status(),
            fields.agent(),
            fields.last(),
            fields.next(),
            notices.unit_line()
        ))
    }
    /// ワークフロー開始前の既知セッションを表示する。
    #[must_use]
    pub fn runtime(session: &str) -> Self {
        Self::new(format!(
            "AIDLC Runtime Session: {session}\nUse this exact value for any Plan Approval --session argument in this conversation."
        ))
    }
    /// 表示することが決まった再選択案内だけを返す。
    #[must_use]
    pub fn rebind_probe(session: &str, offer: &str) -> Self {
        Self::new(format!("AIDLC Runtime Session: {session}\n{offer}"))
    }
    /// 切替命令の選択を行わず、渡された名前と命令を再選択文へ写す。
    #[must_use]
    pub fn format_rebind_offer(was: &str, live: &str, instruction: &str) -> String {
        format!(
            "INTENT REBIND OFFER: This conversation is bound to {was}, but the shared cursor names {live}. Move the shared cursor back to {was}? [Y/n] - on Yes, {instruction}; on No, keep working {was} through this session binding. This changes only machine-local navigation.\n"
        )
    }
    /// additionalContextのJSON。末尾改行は呼出側が付ける。
    #[must_use]
    pub fn to_json_line(&self) -> String {
        let mut fields = ObjectMembers::new();
        fields.insert("additionalContext", JsonValue::String(self.text.clone()));
        serialize(
            &JsonValue::Object(fields),
            SerializationProfile::ContractCompact,
        )
    }
}

/// 固定本家2.7.1のSessionStart公開文言。
const FORWARDING: &str = "On BARE /aidlc re-entry, offer the user the standard resume options (Resume / Redo / Jump / Start Fresh). Explicit /aidlc --resume already selects Resume: do NOT offer the menu; forward --resume unchanged and continue directly. Check the active intent's aidlc-state.md for full context.\n\nFORWARDING-LOOP DISCIPLINE (non-negotiable — the engine owns ALL routing):\n- The engine binary (`aidlc-orchestrate.ts`) is the ONLY authority on the next move. You run it, you do EXACTLY what its one directive says, you commit with `report`, you repeat. You never re-derive routing yourself.\n- STEP 1 — YOUR VERY FIRST ACTION: take everything the user typed after `/aidlc` and append it to the first `next` call UNCHANGED. The flags ARE the user's intent; dropping them sends the workflow to the wrong place. `/aidlc --phase ideation` → you MUST run `next --phase ideation`, never bare `next`. `/aidlc --stage X` → `next --stage X`. `/aidlc` alone → `next`. Before running that first `next`, verify: if the user's message contained `--phase`/`--stage`/`--scope`/`--depth`/freeform text, it MUST appear on your `next` command — a bare `next` when the user gave arguments is a bug.\n- When a directive is `{kind:\"print\"}` whose message names a command to run (e.g. `aidlc-jump.ts execute ...`, a scope/config change, or `init`): that named command is your IMMEDIATE next tool call. Run THAT EXACT command FIRST. Do NOT run `next` again, do NOT read more files, do NOT plan a stage — until the named command has run. Re-running the engine before it is a protocol violation that silently skips the move.";
