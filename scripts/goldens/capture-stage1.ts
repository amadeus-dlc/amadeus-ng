#!/usr/bin/env bun
/** 本家の公開CLI/フックに合成入力を与える。実地スモークの証拠には使わない。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { captureObservation } from "./capture-observation";
import { UPSTREAM, verifySource } from "./upstream-source";

export function captureStage1(dist: string) {
  verifySource(dist);
  const root = mkdtempSync(join(tmpdir(), "aidlc-stage1-"));
  const observations: Array<ReturnType<typeof captureObservation> & { id: string }> = [];
  const write = (path: string, text: string) => { mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, text); };
  const run = (id: string, file: string, args: string[] = [], stdin = "", environment: Record<string, string> = {}) => {
    const observation = captureObservation({ root, argv: [process.execPath, join(root, ".claude", file), ...args], stdin, environment });
    observations.push({ id, ...observation });
    assert.equal(observation.output.error, null);
    assert.equal(observation.output.signal, null);
    return observation.output;
  };
  try {
    cpSync(dist, root, { recursive: true });
    write(join(root, "src/base.ts"), "export const base = 1;\n");
    const init = run("intent-create/bugfix", "tools/aidlc-utility.ts", ["intent-create", "--scope", "bugfix", "--label", "stage1", "--arguments", "Fix one small behavior"]);
    assert.equal(init.exit_code, 0, init.stderr);
    const intents = join(root, "aidlc/spaces/default/intents");
    const record = join(intents, readdirSync(intents, { withFileTypes: true }).find(e => e.isDirectory())!.name);
    const state = join(record, "aidlc-state.md");
    write(state, readFileSync(state, "utf8").replace(/^- \*\*Current Stage\*\*:.*$/m, "- **Current Stage**: code-generation")
      .replace(/^- \[[ xSR?-]\] code-generation(\s+—\s+)EXECUTE$/m, "- [-] code-generation$1EXECUTE"));
    write(join(root, ".gitignore"), ".capture-home/\n");
    const git = captureObservation({ root, argv: ["git", "init", "-q"] });
    assert.equal(git.output.exit_code, 0, git.output.stderr);
    let directive = JSON.parse(run("next/code-generation", "tools/aidlc-orchestrate.ts", ["next"]).stdout);
    let part = 0;
    while (directive.kind === "load-steering") {
      assert(part++ < 20);
      directive = JSON.parse(run(`continue/code-generation-${part}`, "tools/aidlc-orchestrate.ts", ["continue", directive.continue_token]).stdout);
    }
    assert.equal(directive.kind, "run-stage", JSON.stringify(directive));
    const dir = join(record, "construction/code-generation");
    const contract = run("testing-posture/render", "tools/aidlc-testing-posture.ts", ["render"]);
    assert.equal(contract.exit_code, 0, contract.stderr);
    write(join(dir, "code-generation-plan.md"), `# Plan\n\n${contract.stdout}\n## Steps\n\n- [ ] Implement\n`);
    write(join(dir, "unit-test-instructions.md"), "# Instructions\n\n## Command\n\n`bun test src/base.test.ts`\n");
    const fingerprint = run("testing-posture/fingerprint", "tools/aidlc-testing-posture.ts", ["fingerprint", "--stage-level"]);
    assert.equal(fingerprint.exit_code, 0, fingerprint.stderr);
    const questions = join(dir, "code-generation-questions.md");
    const fp = fingerprint.stdout.trim();
    write(questions, `## Plan Approval\n[Approval Fingerprint]: ${fp}\nA. Approve Plan\nB. Request Changes\n[Answer]:\n`);
    const identity = ["--stage", "code-generation", "--checkpoint", "plan-approval", "--questions-file", relative(root, questions), "--session", "stage1-session", "--stage-level"];
    run("session/start", "hooks/aidlc-session-start.ts", [], JSON.stringify({ hook_event_name: "SessionStart", session_id: "stage1-session", source: "startup", cwd: root }));
    const decision = run("plan/decision", "tools/aidlc-log.ts", ["decision", ...identity, "--decision", "Approve this exact Code Generation plan?", "--options", "Approve Plan,Request Changes"]);
    assert.equal(decision.exit_code, 0, decision.stderr);
    const before = run("plan/before-answer", "tools/aidlc-testing-posture.ts", ["begin", "--stage-level"]);
    assert.notEqual(before.exit_code, 0, before.stdout);
    const writeInput = JSON.stringify({ hook_event_name: "PreToolUse", session_id: "stage1-session", cwd: root, tool_name: "Write", tool_input: { file_path: join(root, "src/next.ts"), content: "export const next = 2;" } });
    const hookBefore = run("plan-hook/before-answer", "hooks/aidlc-plan-approval-guard.ts", [], writeInput);
    assert.equal(hookBefore.exit_code, 2);
    run("plan/human-answer", "hooks/aidlc-record-human-turn.ts", [], JSON.stringify({ hook_event_name: "UserPromptSubmit", session_id: "stage1-session", prompt: "Approve Plan", cwd: root }));
    write(questions, readFileSync(questions, "utf8").replace(/\[Answer\]:\s*$/, "[Answer]: Approve Plan\n"));
    const wrongSession = run("plan/wrong-session", "tools/aidlc-log.ts", ["answer", ...identity.map(value => value === "stage1-session" ? "other-session" : value), "--details", "Approve Plan"]);
    assert.equal(wrongSession.exit_code, 1);
    const answer = run("plan/answer", "tools/aidlc-log.ts", ["answer", ...identity, "--details", "Approve Plan"]);
    assert.equal(answer.exit_code, 0, answer.stderr);
    const approved = run("plan/approved-begin", "tools/aidlc-testing-posture.ts", ["begin", "--stage-level"]);
    assert.equal(approved.exit_code, 0, approved.stderr);
    const hookApproved = run("plan-hook/approved-write", "hooks/aidlc-plan-approval-guard.ts", [], writeInput);
    assert.equal(hookApproved.exit_code, 0, hookApproved.stderr);
    const contractHash = JSON.parse(contract.stdout.match(/```json\s*([\s\S]*?)```/)![1]).contract_sha256;
    const wrongTarget = run("plan-hook/wrong-target", "hooks/aidlc-plan-approval-guard.ts", [], JSON.stringify({ hook_event_name: "PreToolUse", session_id: "stage1-session", cwd: root, tool_name: "Task", tool_input: { subagent_type: "aidlc-developer-agent", prompt: `AIDLC-UNIT: another-unit\nAIDLC-TESTING-CONTRACT: ${contractHash}\nImplement` } }));
    assert.equal(wrongTarget.exit_code, 2);
    const planPath = join(dir, "code-generation-plan.md");
    const originalPlan = readFileSync(planPath, "utf8");
    write(planPath, originalPlan + "\n承認後の追加\n");
    const changed = run("plan/changed-content", "tools/aidlc-testing-posture.ts", ["begin", "--stage-level"]);
    assert.equal(changed.exit_code, 1);
    write(planPath, originalPlan);
    run("next/reissued-directive", "tools/aidlc-orchestrate.ts", ["next"]);
    const stale = run("plan/stale-receipt", "tools/aidlc-testing-posture.ts", ["begin", "--stage-level"]);
    assert.equal(stale.exit_code, 1);
    write(state, readFileSync(state, "utf8").replace(/^- \*\*Current Stage\*\*:.*$/m, "- **Current Stage**: requirements-analysis")
      .replace(/^- \[[ xSR?-]\] requirements-analysis(\s+—\s+)EXECUTE$/m, "- [-] requirements-analysis$1EXECUTE"));
    let requirementsDirective = JSON.parse(run("next/requirements", "tools/aidlc-orchestrate.ts", ["next"]).stdout);
    let reqPart = 0;
    while (requirementsDirective.kind === "load-steering") {
      assert(reqPart++ < 20);
      requirementsDirective = JSON.parse(run(`continue/requirements-${reqPart}`, "tools/aidlc-orchestrate.ts", ["continue", requirementsDirective.continue_token]).stdout);
    }
    const requirementsDir = join(record, "inception/requirements-analysis");
    const summaryQuestions = join(requirementsDir, "requirements-analysis-questions.md");
    write(summaryQuestions, "# Questions\n\n## Q1\n修正対象は何か。\nA. 小さな不具合\nX. Other (please specify)\n[Answer]: A\n\n## Consolidated Summary Confirmation\nLooks correct / Request changes\n[Answer]:\n");
    const summaryIdentity = ["--stage", "requirements-analysis", "--checkpoint", "summary-confirmation", "--questions-file", relative(root, summaryQuestions)];
    run("question/decision", "tools/aidlc-log.ts", ["decision", "--stage", "requirements-analysis", "--decision", "修正対象は何か。", "--options", "小さな不具合,Other"]);
    run("question/human-answer", "hooks/aidlc-record-human-turn.ts", [], JSON.stringify({ hook_event_name: "UserPromptSubmit", session_id: "stage1-session", prompt: "A", cwd: root }));
    const questionAnswer = run("question/answer", "tools/aidlc-log.ts", ["answer", "--stage", "requirements-analysis", "--details", "A"]);
    assert.equal(questionAnswer.exit_code, 0, questionAnswer.stderr);
    const summaryDecision = run("summary/decision", "tools/aidlc-log.ts", ["decision", ...summaryIdentity, "--decision", "Does this all look correct before I generate the artifact?", "--options", "Looks correct,Request changes"]);
    assert.equal(summaryDecision.exit_code, 0, summaryDecision.stderr);
    run("summary/human-answer", "hooks/aidlc-record-human-turn.ts", [], JSON.stringify({ hook_event_name: "UserPromptSubmit", session_id: "stage1-session", prompt: "Looks correct", cwd: root }));
    write(summaryQuestions, readFileSync(summaryQuestions, "utf8").replace(/\[Answer\]:\s*$/, "[Answer]: Looks correct\n"));
    const summaryAnswer = run("summary/answer", "tools/aidlc-log.ts", ["answer", ...summaryIdentity, "--details", "Looks correct"]);
    assert.equal(summaryAnswer.exit_code, 0, summaryAnswer.stderr);
    const requirements = join(requirementsDir, "requirements.md");
    write(requirements, "# Requirements\n\n## Functional Requirements\nFR1: 小さな不具合を修正する。\n\n## Sources\n[Q1] 承認済み回答。\n\n## Assumptions & Open Questions\nNone.\n");
    run("artifact/saved-after-summary", "hooks/aidlc-write-audit-log.ts", [], JSON.stringify({ hook_event_name: "PostToolUse", session_id: "stage1-session", cwd: root, tool_name: "Write", tool_input: { file_path: requirements, content: readFileSync(requirements, "utf8") }, tool_response: { success: true } }));
    const reviewIdentity = ["--stage", "requirements-analysis", "--reviewer", "aidlc-product-lead-agent", "--iteration", "1"];
    const reviewRequest = run("review/request", "tools/aidlc-log.ts", ["review", ...reviewIdentity]);
    assert.equal(reviewRequest.exit_code, 0, reviewRequest.stderr);
    write(requirements, readFileSync(requirements, "utf8") + "\n## Review\n\n**Reviewer:** aidlc-product-lead-agent\n**Verdict:** READY\n**Iteration:** 1\n\n### Findings\nNone.\n");
    const reviewCompleted = run("review/completed", "tools/aidlc-log.ts", ["review", ...reviewIdentity, "--verdict", "READY"]);
    assert.equal(reviewCompleted.exit_code, 0, reviewCompleted.stderr);
    for (const [id, tool, args] of [
      ["review-brief/summary", "aidlc-review-brief.ts", ["summary", "--stage", "requirements-analysis", "--questions-file", relative(root, summaryQuestions)]],
      ["review-brief/review", "aidlc-review-brief.ts", ["review", "--stage", "requirements-analysis", "--why", "first"]],
      ["review-brief/context", "aidlc-review-brief.ts", ["context", "--stage", "requirements-analysis"]],
      ["project-description/read", "aidlc-utility.ts", ["project-description"]],
    ] as const) {
      const result = run(id, `tools/${tool}`, [...args]);
      assert.equal(result.exit_code, 0, result.stderr);
    }
    const frozen = run("review-hook/frozen-write", "hooks/aidlc-review-freeze.ts", [], JSON.stringify({ hook_event_name: "PreToolUse", session_id: "stage1-session", cwd: root, tool_name: "Write", tool_input: { file_path: requirements, content: "changed" } }));
    assert.equal(frozen.exit_code, 2, frozen.stderr);
    const scan = join(record, "inception/reverse-engineering/developer-scan.md");
    write(scan, "# Synthetic scan\n\n## Evidence\n合成された引継ぎ入力。実解析の成果ではない。\n");
    const developerLink = run("link/developer", "tools/aidlc-log.ts", ["link", "--stage", "reverse-engineering", "--link", "aidlc-developer-agent", "--artifact", relative(root, scan)]);
    assert.equal(developerLink.exit_code, 0, developerLink.stderr);
    const architectLink = run("link/architect", "tools/aidlc-log.ts", ["link", "--stage", "reverse-engineering", "--link", "aidlc-architect-agent"]);
    assert.equal(architectLink.exit_code, 0, architectLink.stderr);
    const awaiting = run("report/awaiting-approval", "tools/aidlc-orchestrate.ts", ["report", "--stage", "requirements-analysis", "--result", "awaiting-approval"]);
    assert.notEqual(JSON.parse(awaiting.stdout).kind, "error", awaiting.stdout);
    run("gate/human-approve", "hooks/aidlc-record-human-turn.ts", [], JSON.stringify({ hook_event_name: "UserPromptSubmit", session_id: "stage1-session", prompt: "Approve", cwd: root }));
    const approvedReport = run("report/approved", "tools/aidlc-orchestrate.ts", ["report", "--stage", "requirements-analysis", "--result", "approved", "--user-input", "Approve"]);
    assert.notEqual(JSON.parse(approvedReport.stdout).kind, "error", approvedReport.stdout);
    for (const [name, toolName, toolInput, event] of [
      ["deliver-stage-rules", "Task", { subagent_type: "aidlc-product-agent", prompt: "Inspect this synthetic fixture." }, "PreToolUse"],
      ["reviewer-scope", "Read", { file_path: requirements }, "PreToolUse"],
      ["sync-workflow-state", "TaskUpdate", { status: "in_progress", activeForm: "Working [code-generation]" }, "PostToolUse"],
      ["rebuild-stage-graph", "Bash", { command: "bun .claude/tools/aidlc-orchestrate.ts report --stage requirements-analysis --result approved --user-input Approve" }, "PostToolUse"],
    ] as const) {
      const result = run(`hook/${name}`, `hooks/aidlc-${name}.ts`, [], JSON.stringify({ hook_event_name: event, session_id: "stage1-session", cwd: root, tool_name: toolName, tool_input: toolInput }));
      assert.equal(result.exit_code, 0, result.stderr);
    }
    const subagent = run("hook/log-subagent", "hooks/aidlc-log-subagent.ts", [], JSON.stringify({ hook_event_name: "SubagentStop", session_id: "stage1-session", cwd: root, agent_type: "aidlc-developer-agent", agent_id: "fixture-agent", last_assistant_message: "Synthetic completion" }));
    assert.equal(subagent.exit_code, 0);
    const compact = run("hook/validate-state", "hooks/aidlc-validate-state.ts", [], JSON.stringify({ hook_event_name: "PreCompact", session_id: "stage1-session", cwd: root, trigger: "manual" }));
    assert.equal(compact.exit_code, 0);
    const folded = run("hook/fold-usage-disabled", "hooks/aidlc-fold-usage.ts", [], JSON.stringify({ hook_event_name: "Stop", session_id: "stage1-session", cwd: root }), { AIDLC_DISABLE_USAGE_TRACKING: "1" });
    assert.equal(folded.exit_code, 0);
    const ended = run("hook/session-end", "hooks/aidlc-session-end.ts", [], JSON.stringify({ hook_event_name: "SessionEnd", session_id: "stage1-session", cwd: root, reason: "exit" }));
    assert.equal(ended.exit_code, 0);
    return { source: UPSTREAM, fixture_kind: "synthetic-preconditions", protection: "enabled", observations };
  } finally { rmSync(root, { recursive: true, force: true }); }
}

if (import.meta.main) {
  const [dist, out] = process.argv.slice(2);
  assert(dist && out, "Usage: bun capture-stage1.ts <verified-dist> <new-output-dir>");
  const corpus = captureStage1(dist);
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, "cases.json"), JSON.stringify(corpus, null, 2) + "\n");
}
