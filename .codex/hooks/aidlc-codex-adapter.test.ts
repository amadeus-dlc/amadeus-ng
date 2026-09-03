import { describe, expect, test } from "bun:test";
import { randomUUID } from "node:crypto";
import { resolve } from "node:path";
import { normalizePreToolUseUpdatedInput } from "./aidlc-codex-adapter.ts";

const projectDir = resolve(import.meta.dir, "../..");
const adapterPath = resolve(import.meta.dir, "aidlc-codex-adapter.ts");

async function runAdapter(target: string, toolName: string, toolInput: object) {
  const subprocess = Bun.spawn(["bun", adapterPath, target], {
    cwd: projectDir,
    stdin: "pipe",
    stdout: "pipe",
    stderr: "pipe",
  });
  subprocess.stdin.write(
    JSON.stringify({
      hook_event_name: "PreToolUse",
      session_id: "01a067db-02fd-7de3-9d72-fde50be2e2ac",
      turn_id: randomUUID(),
      tool_use_id: randomUUID(),
      cwd: projectDir,
      tool_name: toolName,
      tool_input: toolInput,
    }),
  );
  subprocess.stdin.end();

  const [exitCode, stdout, stderr] = await Promise.all([
    subprocess.exited,
    new Response(subprocess.stdout).text(),
    new Response(subprocess.stderr).text(),
  ]);
  expect(stderr).toBe("");
  expect(exitCode).toBe(0);
  return JSON.parse(stdout) as {
    hookSpecificOutput: {
      hookEventName: string;
      permissionDecision?: string;
      updatedInput?: Record<string, unknown>;
    };
  };
}

describe("Codex PreToolUse input rewrites", () => {
  test("bind-bash-session explicitly allows its updated input", async () => {
    const output = await runAdapter("bind-bash-session", "Bash", { command: "true" });

    expect(output.hookSpecificOutput).toMatchObject({
      hookEventName: "PreToolUse",
      permissionDecision: "allow",
    });
    expect(output.hookSpecificOutput.updatedInput?.command).toContain(
      "AIDLC_SESSION_OVERRIDE_SOURCE='payload'; true",
    );
  });

  test("normalizes shared rule-delivery rewrites into the Codex envelope", () => {
    const output = JSON.parse(
      normalizePreToolUseUpdatedInput(
        JSON.stringify({
          hookSpecificOutput: {
            hookEventName: "PreToolUse",
            updatedInput: { message: "brief with active-stage rules" },
          },
        }),
      ),
    );

    expect(output).toEqual({
      hookSpecificOutput: {
        hookEventName: "PreToolUse",
        permissionDecision: "allow",
        updatedInput: { message: "brief with active-stage rules" },
      },
    });
  });
});
