import { afterEach, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { configure } from "./aidlc-kimi-hooks";
import { forward } from "./aidlc-sync/kimi-trusted-adapter";

const project = resolve(import.meta.dir, "..");
const scratch: string[] = [];
afterEach(() => { for (const path of scratch.splice(0)) rmSync(path, { recursive: true, force: true }); });
function temp() { const path = realpathSync(mkdtempSync(join(tmpdir(), "aidlc-harness-"))); scratch.push(path); return path; }
function put(root: string, path: string, data: string) { mkdirSync(dirname(join(root, path)), { recursive: true }); writeFileSync(join(root, path), data); }

test("ハーネス名と配置先が不一致なら runner 生成を拒否する", () => {
  const script = `import {renderRunner} from ${JSON.stringify(join(project, ".codex/tools/aidlc-runner-gen.ts"))}; console.log(renderRunner("classic", "test"));`;
  const result = Bun.spawnSync([process.execPath, "-e", script], { env: { ...process.env, AIDLC_HARNESS_DIR: ".codex", AIDLC_HARNESS_NAME: "kimi" } });
  expect(result.exitCode).not.toBe(0);
  expect(result.stderr.toString()).toContain("Harness mismatch");
});

test("Kimi の runner は /skill:aidlc を使う", () => {
  const script = `import {renderRunner} from ${JSON.stringify(join(project, ".kimi-code/tools/aidlc-runner-gen.ts"))}; console.log(renderRunner("classic", "test"));`;
  const result = Bun.spawnSync([process.execPath, "-e", script], { env: { ...process.env, AIDLC_HARNESS_DIR: ".kimi-code", AIDLC_HARNESS_NAME: "kimi" } });
  expect(result.exitCode).toBe(0);
  expect(result.stdout.toString()).toContain("/skill:aidlc");
});

test("Kimi の設定がファイルでなくても doctor は診断を返す", () => {
  const root = temp(), home = temp();
  mkdirSync(join(home, "config.toml"));
  put(root, ".kimi-code/hooks.snippet.toml", '[[hooks]]\nevent = "Stop"\ncommand = "bun .kimi-code/hooks/aidlc-kimi-adapter.ts continue-workflow"\n');
  const result = Bun.spawnSync([process.execPath, join(project, ".kimi-code/tools/aidlc-utility.ts"), "doctor", "--project-dir", root], {
    env: { ...process.env, KIMI_CODE_HOME: home, AIDLC_HARNESS_DIR: ".kimi-code", AIDLC_HARNESS_NAME: "kimi" },
  });
  expect(result.exitCode).toBe(1);
  expect(result.stdout.toString()).toContain("Kimi hook configuration could not be read");
  expect(result.stdout.toString()).toContain("AI-DLC Health Check");
});

test("信頼登録前にはリポジトリのフックを実行せず、登録後だけ payload を転送する", async () => {
  const root = temp(), home = temp(), marker = join(root, "executed.txt"), registry = join(home, "trusted-projects.json");
  put(root, ".kimi-code/hooks/aidlc-kimi-adapter.ts", `await Bun.write(${JSON.stringify(marker)}, await Bun.stdin.text()); process.exit(2);`);
  const raw = JSON.stringify({ cwd: root, session_id: "test" });
  writeFileSync(registry, "[]");
  expect(await forward(raw, "test", registry)).toBe(0);
  expect(existsSync(marker)).toBe(false);
  writeFileSync(registry, JSON.stringify([root]));
  expect(await forward(raw, "test", registry)).toBe(2);
  expect(readFileSync(marker, "utf8")).toBe(raw);
});

test("Kimi 登録は既存設定を保持し、二重登録せず、確認だけでも検証できる", () => {
  const root = temp(), home = temp();
  put(root, "scripts/aidlc-sync/kimi-trusted-adapter.ts", readFileSync(join(project, "scripts/aidlc-sync/kimi-trusted-adapter.ts"), "utf8"));
  put(root, ".kimi-code/hooks.snippet.toml", readFileSync(join(project, ".kimi-code/hooks.snippet.toml"), "utf8"));
  const original = 'default_model = "my-model"\n[[hooks]]\nevent = "Stop"\ncommand = "echo existing"\n';
  put(home, "config.toml", original);
  configure(root, home, true);
  const first = readFileSync(join(home, "config.toml"), "utf8");
  configure(root, home, true);
  expect(readFileSync(join(home, "config.toml"), "utf8")).toBe(first);
  expect(first).toContain(original.trim());
  expect(first).not.toContain("bun .kimi-code/hooks/");
  expect(() => configure(root, home, false)).not.toThrow();
  const hooks = (Bun.TOML.parse(first) as { hooks: unknown[] }).hooks;
  expect(hooks.length).toBe(16);
});

test("信頼済みプロジェクトから外部ファイルへのシンボリックリンクを拒否する", async () => {
  const root = temp(), outside = temp(), registry = join(root, "trusted-projects.json");
  put(outside, "hook.ts", 'throw new Error("must not execute");');
  mkdirSync(join(root, ".kimi-code/hooks"), { recursive: true });
  symlinkSync(join(outside, "hook.ts"), join(root, ".kimi-code/hooks/aidlc-kimi-adapter.ts"));
  writeFileSync(registry, JSON.stringify([root]));
  await expect(forward(JSON.stringify({ cwd: root }), "test", registry)).rejects.toThrow("プロジェクトの外");
});
