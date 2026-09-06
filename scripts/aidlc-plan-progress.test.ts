import { afterAll, describe, expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const projects: string[] = [];
afterAll(() => { for (const project of projects) rmSync(project, { recursive: true, force: true }); });

// 承認チャレンジと人間回答は使い捨てプロジェクト内でのみ生成する。
async function approvedProject(harness: string) {
  const project = mkdtempSync(join(tmpdir(), "aidlc-plan-progress-"));
  projects.push(project);
  cpSync(resolve(harness, "tools"), join(project, harness, "tools"), { recursive: true });
  cpSync(resolve(harness, "hooks"), join(project, harness, "hooks"), { recursive: true });
  const record = join(project, "aidlc/spaces/default/intents");
  mkdirSync(record, { recursive: true });
  const state = "# AI-DLC State Tracking\n\n## Project Information\n- **Project**: progress regression\n- **Scope**: poc\n\n## Current Status\n- **Lifecycle Phase**: CONSTRUCTION\n- **Current Stage**: code-generation\n";
  writeFileSync(join(record, "aidlc-state.md"), state);
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith("AIDLC_") && !key.startsWith("AWS_AIDLC_")));
  env.CLAUDE_PROJECT_DIR = project;
  const run = (args: string[], input?: object) => Bun.spawnSync(args, {
    cwd: project, env, stdout: "pipe", stderr: "pipe",
    ...(input ? { stdin: Buffer.from(JSON.stringify(input)) } : {}),
  });
  const guard = (path: string) => run([process.execPath, join(project, harness, "hooks/aidlc-plan-approval-guard.ts")], {
    hook_event_name: "PreToolUse", cwd: project, tool_name: "Write", tool_input: { file_path: path, content: "test" },
  });
  const remove = (paths: string[]) => run([process.execPath, join(project, harness, "hooks/aidlc-plan-approval-guard.ts")], {
    hook_event_name: "PreToolUse", cwd: project, tool_name: "Bash",
    tool_input: { command: `rm -- ${paths.map(path => `'${path.replaceAll("'", "'\\''")}'`).join(" ")}` },
  });
  for (const args of [["init", "-q"], ["config", "user.email", "tests@example.com"], ["config", "user.name", "Tests"], ["add", "-A"], ["commit", "-qm", "baseline"]]) {
    expect(run(["git", ...args]).exitCode).toBe(0);
  }
  const lib = await import(`../${harness}/tools/aidlc-lib.ts`);
  const posture = await import(`../${harness}/tools/aidlc-testing-posture.ts`);
  const audit = await import(`../${harness}/tools/aidlc-audit.ts`);
  const unit = "progress-unit";
  lib.writeActiveDirectiveMarker(project, { kind: "run-stage", stage: "code-generation", unit, state_sha256: createHash("sha256").update(state).digest("hex") });
  const authority = posture.resolveCodeGenerationAuthority(project, { unit });
  const contract = posture.resolveTestingPosture(project);
  const dir = posture.codeGenerationRecordDir(project, unit);
  mkdirSync(dir, { recursive: true });
  const plan = `# Plan\n\n${posture.renderTestingContract(contract)}\n## Steps\n\n- [ ] Step 1. Implement\n`;
  const instructions = "# Tests\n\n## Command\n\nbun test unit.test.ts\n";
  const planPath = join(dir, "code-generation-plan.md");
  writeFileSync(planPath, plan);
  writeFileSync(join(dir, "unit-test-instructions.md"), instructions);
  const questions = join(dir, "code-generation-questions.md");
  writeFileSync(questions, `## Plan Approval\n[Approval Fingerprint]: ${posture.approvalFingerprint(plan, instructions, contract.contract_sha256, authority)}\n- Approve Plan\n- Request Changes\n[Answer]:\n`);
  expect(guard(join(record, ".aidlc-reviewer-dispatch.json")).exitCode).toBe(2);
  const session = "progress-regression";
  audit.appendAuditEntry("SESSION_STARTED", { Source: "startup", Session: session }, project);
  const identity = ["--stage", "code-generation", "--checkpoint", "plan-approval", "--questions-file", questions, "--session", session, "--unit", unit];
  const log = join(project, harness, "tools/aidlc-log.ts");
  const decision = run([process.execPath, log, "decision", ...identity, "--decision", "Approve this exact Code Generation plan?", "--options", "Approve Plan,Request Changes"]);
  expect(decision.exitCode, decision.stdout.toString() + decision.stderr.toString()).toBe(0);
  const human = run([process.execPath, join(project, harness, "hooks/aidlc-record-human-turn.ts")], { hook_event_name: "UserPromptSubmit", session_id: session, prompt: "Approve Plan" });
  expect(human.exitCode, human.stderr.toString()).toBe(0);
  writeFileSync(questions, readFileSync(questions, "utf8").replace("[Answer]:", "[Answer]: Approve Plan"));
  const answer = run([process.execPath, log, "answer", ...identity, "--details", "Approve Plan"]);
  expect(answer.exitCode, answer.stdout.toString() + answer.stderr.toString()).toBe(0);
  posture.beginCodeGeneration(project, { unit });
  const reissue = () => lib.writeActiveDirectiveMarker(project, { kind: "run-stage", stage: "code-generation", unit, state_sha256: createHash("sha256").update(state).digest("hex") });
  return { project, record, planPath, plan, guard, remove, reissue };
}

const authority = {
  targetId: "unit:u1-canon-json-goldens",
  intentId: "progress-regression",
  directiveEpoch: `sha256:${"1".repeat(64)}`,
  runFloor: "STAGE_STARTED:2026-09-06T00:00:00Z#1",
  sourceFloor: "2".repeat(40),
};

for (const harness of [".claude", ".codex", ".kimi-code"]) {
  const { approvalFingerprint } = await import(`../${harness}/tools/aidlc-testing-posture.ts`);
  const fingerprint = (plan: string) => approvalFingerprint(plan, "unit tests", "sha256:contract", authority);
  describe(`${harness}: 計画承認と作業進捗`, () => {
    const plan = "# Plan\n\n- [ ] Step 1. Inspect\n- [ ] Step 2. Verify\n";
    test("完了チェックだけでは承認を失効させない", () => {
      expect(fingerprint(plan.replace("[ ] Step 1", "[x] Step 1"))).toBe(fingerprint(plan));
    });
    test("作業内容や追加ステップの変更は再承認が必要", () => {
      expect(fingerprint(plan.replace("Inspect", "Publish"))).not.toBe(fingerprint(plan));
      expect(fingerprint(`${plan}- [ ] Step 3. Publish\n`)).not.toBe(fingerprint(plan));
    });
    test("コード例のチェック表記は承認内容として保持する", () => {
      const sample = `${plan}\n\x60\x60\x60text\n- [ ] expected output\n\x60\x60\x60\n`;
      expect(fingerprint(sample.replace("[ ] expected", "[x] expected"))).not.toBe(fingerprint(sample));
    });
    test("改行を保持し、列挙形式や大文字の完了マークも扱う", () => {
      for (const prefix of ["-", "+", "*", "1.", "2)"]) {
        const original = `# Plan\r\n\r\n${prefix} [ ] Step\r\n`;
        expect(fingerprint(original.replace("[ ]", "[X]"))).toBe(fingerprint(original));
      }
    });
    test("引用・インデントコード・HTML内の表記は除外しない", () => {
      for (const sample of ["> - [ ] quoted", "    - [ ] code", "<!--\n- [ ] hidden\n-->", "<pre>\n- [ ] code\n</pre>", "~~~\n- [ ] code\n~~~"]) {
        const original = `${plan}\n${sample}\n`;
        expect(fingerprint(original.replaceAll("[ ]", "[x]"))).not.toBe(fingerprint(original));
      }
    });
    test("承認後のチェック更新でも次の編集とレビュー準備を許可する", async () => {
      const fixture = await approvedProject(harness);
      writeFileSync(fixture.planPath, fixture.plan.replace("[ ] Step 1", "[x] Step 1"));
      for (const path of [join(fixture.project, "src/example.ts"), join(fixture.record, ".aidlc-reviewer-dispatch.json")]) {
        const result = fixture.guard(path);
        expect(result.exitCode, result.stderr.toString()).toBe(0);
      }
      writeFileSync(fixture.planPath, `${fixture.plan}\n## Review\n\n**Verdict:** READY\n`);
      const cleanup = fixture.guard(join(fixture.record, ".aidlc-reviewer-dispatch.json"));
      expect(cleanup.exitCode, cleanup.stderr.toString()).toBe(0);
      expect(fixture.remove([join(fixture.record, ".aidlc-reviewer-dispatch.json")]).exitCode).toBe(0);
      expect(fixture.remove([join(fixture.record, ".aidlc-reviewer-dispatch.json"), join(fixture.project, "src/example.ts")]).exitCode).toBe(2);
      expect(fixture.guard(join(fixture.project, "src/example.ts")).exitCode).toBe(2);
      expect(fixture.guard(join(fixture.record, ".aidlc-reviewer-dispatch.json.bak")).exitCode).toBe(2);
      expect(fixture.guard(join(fixture.record, "other/.aidlc-reviewer-dispatch.json")).exitCode).toBe(2);
      const linked = join(fixture.record, ".aidlc-reviewer-dispatch.json");
      symlinkSync(join(fixture.project, "outside.json"), linked);
      expect(fixture.guard(linked).exitCode).toBe(2);
      rmSync(linked);
      writeFileSync(fixture.planPath, fixture.plan.replace("Implement", "Publish without tests"));
      expect(fixture.guard(join(fixture.project, "src/example.ts")).exitCode).toBe(2);
      fixture.reissue();
      expect(fixture.guard(join(fixture.record, ".aidlc-reviewer-dispatch.json")).exitCode).toBe(2);
    }, 30000);
  });
}
