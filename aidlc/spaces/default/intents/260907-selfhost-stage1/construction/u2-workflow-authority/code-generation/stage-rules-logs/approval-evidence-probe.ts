#!/usr/bin/env bun
// `aidlc-plan-approval-guard.ts` がどの述語で落ちているかを読取りだけで観測する。
// 何も書かない。判定に使う関数を本家 lib からそのまま呼ぶ。

const lib = await import("../../../../../../../../../.claude/tools/aidlc-testing-posture.ts");
const projectDir = process.cwd();
const target = { unit: "u2-workflow-authority" };

const approval = lib.evaluateCodeGenerationApproval(projectDir, target);
process.stdout.write(`evaluateCodeGenerationApproval:\n${JSON.stringify(approval, null, 2)}\n`);

try {
  const authority = lib.resolveCodeGenerationAuthority(projectDir, target);
  process.stdout.write(`resolveCodeGenerationAuthority:\n${JSON.stringify(authority, null, 2)}\n`);
} catch (error) {
  process.stdout.write(`resolveCodeGenerationAuthority threw: ${String(error)}\n`);
}
