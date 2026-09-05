import { afterEach, describe, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { applyPlan, MANIFEST, planSync, validateProviders, type Inventory } from "./aidlc-sync";

const scratch: string[] = [];
afterEach(() => { for (const path of scratch.splice(0)) rmSync(path, { recursive: true, force: true }); });
function fixture() {
  const root = mkdtempSync(join(tmpdir(), "aidlc-sync-test-"));
  scratch.push(root);
  const project = join(root, "project"), stage = join(root, "stage");
  mkdirSync(project); mkdirSync(stage);
  return { project, stage };
}
function put(root: string, path: string, content: string) {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), content);
}
const empty = (): Inventory => ({ format: 1, files: {}, preserved: {} });
function install(project: string, stage: string) {
  const plan = planSync(project, stage, empty());
  applyPlan(project, stage, plan);
  return plan.inventory;
}

describe("配布物の入れ替え", () => {
  test("保持設定や個人設定で Bedrock が有効なら更新前に拒否する", () => {
    const { project, stage } = fixture();
    put(stage, ".claude/settings.json", JSON.stringify({ env: { CLAUDE_CODE_USE_BEDROCK: "0" } }));
    expect(() => validateProviders(project, stage)).not.toThrow();
    put(project, ".claude/settings.local.json", JSON.stringify({ env: { CLAUDE_CODE_USE_BEDROCK: "1" } }));
    expect(() => validateProviders(project, stage)).toThrow("AWS Bedrock");
    rmSync(join(project, ".claude/settings.local.json"));
    put(project, ".codex/config.toml", 'model_provider = "amazon-bedrock"\n');
    expect(() => validateProviders(project, stage)).toThrow("Bedrock");
  });
  test("旧配布物を削除し、設定・独自ファイル・作業記録を保持する", () => {
    const { project, stage } = fixture();
    put(stage, ".claude/tools/old.ts", "old");
    put(stage, ".claude/settings.json", "upstream");
    put(stage, ".kimi-code/tools/new.ts", "kimi");
    const previous = install(project, stage);
    put(project, ".claude/settings.json", "my settings");
    put(project, ".claude/hooks/statusline-combined.sh", "my hook");
    put(project, ".codex/hooks/aidlc-local.test.ts", "my test");
    put(project, "aidlc/spaces/default/intents/work/aidlc-state.md", "running");
    put(project, "AGENTS.md", "my rules");
    rmSync(join(stage, ".claude/tools/old.ts"));
    put(stage, ".claude/tools/new.ts", "new");
    put(stage, ".claude/settings.json", "upstream changed");
    const plan = planSync(project, stage, previous);
    expect(plan.remove).toEqual([".claude/tools/old.ts"]);
    expect(plan.review).toEqual([".claude/settings.json"]);
    applyPlan(project, stage, plan);
    expect(existsSync(join(project, ".claude/tools/old.ts"))).toBe(false);
    for (const [path, value] of [
      [".claude/tools/new.ts", "new"], [".claude/settings.json", "my settings"],
      [".claude/hooks/statusline-combined.sh", "my hook"], [".codex/hooks/aidlc-local.test.ts", "my test"],
      ["aidlc/spaces/default/intents/work/aidlc-state.md", "running"], ["AGENTS.md", "my rules"],
    ]) expect(readFileSync(join(project, path), "utf8")).toBe(value);
    expect(planSync(project, stage, plan.inventory).changed).toBe(false);
  });

  test("管理対象のローカル変更を削除・上書きする前に拒否する", () => {
    const { project, stage } = fixture();
    put(stage, ".codex/tools/old.ts", "old");
    const previous = install(project, stage);
    put(project, ".codex/tools/old.ts", "local edit");
    rmSync(join(stage, ".codex/tools/old.ts"));
    expect(() => planSync(project, stage, previous)).toThrow("ローカル変更");
    expect(readFileSync(join(project, ".codex/tools/old.ts"), "utf8")).toBe("local edit");
  });

  test("新しい配布ファイルが管理外ファイルと衝突したら拒否する", () => {
    const { project, stage } = fixture();
    put(stage, ".agents/skills/custom/SKILL.md", "distribution");
    put(project, ".agents/skills/custom/SKILL.md", "user skill");
    expect(() => planSync(project, stage, empty())).toThrow("衝突");
  });

  test("コピー途中の障害で旧ファイル・台帳を復元し、新規ファイルを残さない", () => {
    const { project, stage } = fixture();
    put(stage, ".claude/tools/old.ts", "old");
    const previous = install(project, stage);
    const oldManifest = readFileSync(join(project, MANIFEST), "utf8");
    rmSync(join(stage, ".claude/tools/old.ts"));
    put(stage, ".claude/tools/new.ts", "new");
    const plan = planSync(project, stage, previous);
    expect(() => applyPlan(project, stage, plan, () => { throw new Error("disk full"); })).toThrow("復元しました");
    expect(readFileSync(join(project, ".claude/tools/old.ts"), "utf8")).toBe("old");
    expect(readFileSync(join(project, MANIFEST), "utf8")).toBe(oldManifest);
    expect(existsSync(join(project, ".claude/tools/new.ts"))).toBe(false);
  });

  test("削除台帳の相対パスによる範囲外アクセスを拒否する", () => {
    const { project, stage } = fixture();
    const previous = empty();
    previous.files[".claude/../../outside"] = { sha256: "a".repeat(64), executable: false };
    expect(() => planSync(project, stage, previous)).toThrow("管理対象外");
  });

  test("導入先のシンボリックリンクを経由して書き込まない", () => {
    const { project, stage } = fixture();
    put(stage, ".claude/tools/main.ts", "distribution");
    symlinkSync(stage, join(project, ".claude"));
    expect(() => planSync(project, stage, empty())).toThrow("シンボリックリンク");
  });

  test("欠落した管理ファイルを復元し、実行権限も配布物に合わせる", () => {
    const { project, stage } = fixture();
    put(stage, ".claude/hooks/run.sh", "#!/bin/sh\n");
    chmodSync(join(stage, ".claude/hooks/run.sh"), 0o755);
    const previous = install(project, stage);
    rmSync(join(project, ".claude/hooks/run.sh"));
    const plan = planSync(project, stage, previous);
    expect(plan.install).toEqual([".claude/hooks/run.sh"]);
    applyPlan(project, stage, plan);
    expect(planSync(project, stage, plan.inventory).changed).toBe(false);
  });

  test("配布元で消えた保持設定も削除しない", () => {
    const { project, stage } = fixture();
    put(stage, ".codex/config.toml", "config");
    const previous = install(project, stage);
    rmSync(join(stage, ".codex/config.toml"));
    const plan = planSync(project, stage, previous);
    expect(plan.review).toEqual([".codex/config.toml"]);
    applyPlan(project, stage, plan);
    expect(readFileSync(join(project, ".codex/config.toml"), "utf8")).toBe("config");
  });
});
