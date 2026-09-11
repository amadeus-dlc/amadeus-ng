import { afterAll, describe, expect, test } from "bun:test";
import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const projects: string[] = [];
afterAll(() => { for (const project of projects) rmSync(project, { recursive: true, force: true }); });

// park 直後の再開と同じ「code-generation でディレクティブ不在」の状態でガードを起動する。
// 承認記録もディレクティブも置かないので、書込み可能と分類された Bash はすべて拒否される。
function guardProject(harness: string) {
  const project = mkdtempSync(join(tmpdir(), "aidlc-shell-redirection-"));
  projects.push(project);
  cpSync(resolve(harness, "tools"), join(project, harness, "tools"), { recursive: true });
  cpSync(resolve(harness, "hooks"), join(project, harness, "hooks"), { recursive: true });
  const record = join(project, "aidlc/spaces/default/intents");
  mkdirSync(record, { recursive: true });
  writeFileSync(join(record, "aidlc-state.md"), "# AI-DLC State Tracking\n\n## Project Information\n- **Project**: redirection regression\n- **Scope**: poc\n\n## Current Status\n- **Lifecycle Phase**: CONSTRUCTION\n- **Current Stage**: code-generation\n");
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith("AIDLC_") && !key.startsWith("AWS_AIDLC_")));
  env.CLAUDE_PROJECT_DIR = project;
  const bash = (command: string) => Bun.spawnSync([process.execPath, join(project, harness, "hooks/aidlc-plan-approval-guard.ts")], {
    cwd: project, env, stdout: "pipe", stderr: "pipe",
    stdin: Buffer.from(JSON.stringify({ hook_event_name: "PreToolUse", cwd: project, tool_name: "Bash", tool_input: { command } })),
  });
  return { project, bash, tool: join(project, harness, "tools/aidlc-utility.ts") };
}

for (const harness of [".claude", ".codex", ".kimi-code"]) {
  const { shellCommandInvocations, shellWriteTargets } = await import(`../${harness}/hooks/review-freeze-command.ts`);
  const cwd = "/work";
  describe(`${harness}: リダイレクト演算子の字句解析`, () => {
    test("2>&1 は区切りでも引数でもない", () => {
      expect(shellCommandInvocations("bun .claude/tools/aidlc-utility.ts status 2>&1"))
        .toEqual([{ name: "bun", args: [".claude/tools/aidlc-utility.ts", "status"] }]);
      expect(shellWriteTargets("bun .claude/tools/aidlc-utility.ts status 2>&1", cwd)).toEqual([]);
    });
    test("記述子の複製と閉鎖は語を生まない", () => {
      for (const command of ["cmd >&2", "cmd 1>&2", "cmd 2>&-", "cmd <&0", "cmd 2>&1 1>&2"]) {
        expect(shellCommandInvocations(command), command).toEqual([{ name: "cmd", args: [] }]);
        expect(shellWriteTargets(command, cwd), command).toEqual([]);
      }
    });
    test("パイプ前の 2>&1 は次のコマンドを壊さない", () => {
      expect(shellCommandInvocations("ls -la dir 2>&1 | head -20"))
        .toEqual([{ name: "ls", args: ["-la", "dir"] }, { name: "head", args: ["-20"] }]);
    });
    test("記述子番号付きのファイル出力は書込み対象を保つ", () => {
      expect(shellCommandInvocations("cmd 2>err.log")).toEqual([{ name: "cmd", args: ["err.log"] }]);
      expect(shellWriteTargets("cmd 2>err.log", cwd)).toEqual([resolve(cwd, "err.log")]);
      expect(shellWriteTargets("cmd >out.log 2>&1", cwd)).toEqual([resolve(cwd, "out.log")]);
      expect(shellCommandInvocations("cmd 12>x")).toEqual([{ name: "cmd", args: ["x"] }]);
    });
    test("&> と &>> は両出力のファイル書込みであり区切りではない", () => {
      for (const command of ["cmd &>out.log", "cmd &>>out.log"]) {
        expect(shellCommandInvocations(command), command).toEqual([{ name: "cmd", args: ["out.log"] }]);
        expect(shellWriteTargets(command, cwd), command).toEqual([resolve(cwd, "out.log")]);
      }
    });
    test("空白で区切られた数値は引数のまま", () => {
      expect(shellCommandInvocations("echo 2 > out")).toEqual([{ name: "echo", args: ["2", "out"] }]);
      expect(shellCommandInvocations("ls dir2>out")).toEqual([{ name: "ls", args: ["dir2", "out"] }]);
    });
    test("本来の区切りは従来どおり分割する", () => {
      for (const command of ["a && b", "a & b", "a || b", "a | b", "a; b", "a\nb"]) {
        expect(shellCommandInvocations(command).map(({ name }) => name), command).toEqual(["a", "b"]);
      }
      expect(shellCommandInvocations("a 2>&1 && b >&2")).toEqual([{ name: "a", args: [] }, { name: "b", args: [] }]);
    });
    test("引用符とエスケープの中は解釈しない", () => {
      expect(shellCommandInvocations("echo '2>&1' \"a&b\" \\& x"))
        .toEqual([{ name: "echo", args: ["2>&1", "a&b", "&", "x"] }]);
    });
  });

  describe(`${harness}: 計画承認ガードと標準エラーの複製`, () => {
    test("ディレクティブ不在でも 2>&1 付きの読取り専用ツール呼出しを許可する", () => {
      const fixture = guardProject(harness);
      for (const command of [
        `bun ${fixture.tool} status`,
        `bun ${fixture.tool} status 2>&1`,
        `bun ${fixture.tool} status 2>&1 | head -20`,
        "ls -la . 2>&1",
      ]) {
        const result = fixture.bash(command);
        expect(result.exitCode, `${command}\n${result.stderr.toString()}`).toBe(0);
      }
    }, 30000);
    test("実際のファイル書込みは引き続き拒否する", () => {
      const fixture = guardProject(harness);
      for (const command of [
        `bun ${fixture.tool} status > out.log`,
        `bun ${fixture.tool} status 2>err.log`,
        `bun ${fixture.tool} status &>out.log`,
        "bun scripts/other.ts 2>&1",
      ]) {
        const result = fixture.bash(command);
        expect(result.exitCode, command).toBe(2);
        expect(result.stderr.toString(), command).toContain("Plan Approval");
      }
    }, 30000);
  });
}
