#!/usr/bin/env bun
import { createHash, randomUUID } from "node:crypto";
import { chmodSync, copyFileSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

// 配布元のコミットは gitlink が正本。ここでは自動的な版選択や fetch をしない。
export const ROOTS = [
  ["claude", ".claude"], ["codex", ".codex"],
  ["codex", ".agents"], ["kimi", ".kimi-code"],
] as const;
export const PRESERVE = new Set([
  ".claude/settings.json", ".claude/CLAUDE.md",
  ".codex/config.toml", ".codex/hooks.json", ".codex/rules/default.rules",
  ".claude/rules/aidlc.md",
  ".kimi-code/mcp.json", ".kimi-code/rules/aidlc.md",
]);
export const MANIFEST = "scripts/aidlc-sync/installed.json";
type Stamp = { sha256: string; executable: boolean };
export type Inventory = { format: 1; files: Record<string, Stamp>; preserved: Record<string, Stamp> };
export type Plan = {
  inventory: Inventory; install: string[]; remove: string[];
  review: string[]; changed: boolean;
};

function gitBytes(cwd: string, args: string[]): Buffer {
  const result = Bun.spawnSync(["git", ...args], { cwd, stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) throw new Error(result.stderr.toString().trim());
  return result.stdout;
}

function git(cwd: string, args: string[]): string {
  return gitBytes(cwd, args).toString();
}

function digest(data: Uint8Array): string {
  return createHash("sha256").update(data).digest("hex");
}

function stamp(path: string): Stamp {
  return { sha256: digest(readFileSync(path)), executable: (lstatSync(path).mode & 0o111) !== 0 };
}

function same(a: Stamp | undefined, b: Stamp | undefined): boolean {
  return !!a && !!b && a.sha256 === b.sha256 && a.executable === b.executable;
}

function managed(path: string): boolean {
  return ROOTS.some(([, root]) => path.startsWith(`${root}/`));
}

// 配布物・旧台帳に不正なパスがあっても作業ツリーの外へ出ない。
function validatePath(path: string): void {
  if (path.includes("\\") || path.split("/").some(p => !p || p === "." || p === "..") || !managed(path)) {
    throw new Error(`管理対象外のパス: ${path}`);
  }
}

export function safePath(root: string, path: string): string {
  let current = root;
  for (const part of path.split("/")) {
    current = join(current, part);
    try {
      if (lstatSync(current).isSymbolicLink()) throw new Error(`シンボリックリンクは更新できません: ${current}`);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
  }
  return current;
}

function walk(root: string, prefix: string): string[] {
  const path = safePath(root, prefix);
  if (!existsSync(path)) return [];
  return readdirSync(path, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name)).flatMap(entry => {
    const name = `${prefix}/${entry.name}`;
    if (entry.isSymbolicLink()) throw new Error(`配布物内のシンボリックリンク: ${name}`);
    if (entry.isDirectory()) return walk(root, name);
    if (!entry.isFile()) throw new Error(`通常ファイルではありません: ${name}`);
    return [name];
  });
}

function copy(from: string, to: string): void {
  mkdirSync(dirname(to), { recursive: true });
  copyFileSync(from, to);
  chmodSync(to, lstatSync(from).mode & 0o777);
}

function inspect(root: string, paths: string[]): Record<string, Stamp> {
  return Object.fromEntries(paths.sort().map(path => [path, stamp(join(root, path))]));
}

function validateInventory(inventory: Inventory): void {
  if (inventory.format !== 1 || !inventory.files || !inventory.preserved) throw new Error("更新台帳の形式が不正です");
  for (const [path, entry] of Object.entries({ ...inventory.files, ...inventory.preserved })) {
    validatePath(path);
    if (!/^[a-f0-9]{64}$/.test(entry.sha256) || typeof entry.executable !== "boolean") {
      throw new Error(`更新台帳のハッシュが不正です: ${path}`);
    }
  }
}

export function validateProviders(project: string, stage: string): void {
  for (const path of [".claude/settings.json", ".claude/settings.local.json", ".codex/config.toml"]) {
    const existing = safePath(project, path);
    const target = existsSync(existing) ? existing : join(stage, path);
    if (!existsSync(target)) continue;
    const content = readFileSync(target, "utf8");
    if (path.endsWith(".json")) {
      const settings = JSON.parse(content);
      const env = settings.env ?? {};
      if (["1", "true"].includes(String(env.CLAUDE_CODE_USE_BEDROCK).toLowerCase()) ||
          /(?:global\.|us\.|eu\.|apac\.)?anthropic\.claude-/.test(JSON.stringify([settings.model, ...Object.values(env)]))) {
        throw new Error(`このプロジェクトは AWS Bedrock を使用しません。設定を見直してください: ${path}`);
      }
    } else {
      const config = Bun.TOML.parse(content) as { model_provider?: string; model?: string };
      if (/bedrock/i.test(config.model_provider ?? "") || /^openai\.gpt-/.test(config.model ?? "")) {
        throw new Error(`Bedrock 向け Codex 設定を使用できません: ${path}`);
      }
    }
  }
}

export function planSync(project: string, stage: string, previous: Inventory): Plan {
  validateInventory(previous);
  const paths = ROOTS.flatMap(([, root]) => walk(stage, root));
  const files = inspect(stage, paths.filter(path => !PRESERVE.has(path)));
  const preserved = inspect(stage, paths.filter(path => PRESERVE.has(path)));
  const inventory: Inventory = { format: 1, files, preserved };
  const install: string[] = [], remove: string[] = [], review: string[] = [];
  const conflicts: string[] = [];
  for (const path of new Set([...Object.keys(previous.files), ...paths])) {
    validatePath(path);
    const destination = safePath(project, path);
    if (existsSync(destination) && !lstatSync(destination).isFile()) throw new Error(`ファイル以外との衝突: ${path}`);
    const current = existsSync(destination) ? stamp(destination) : undefined;
    if (PRESERVE.has(path)) {
      if (!current && preserved[path]) install.push(path);
      if (current && !same(previous.preserved[path], preserved[path])) review.push(path);
      continue;
    }
    if (current && !same(current, previous.files[path]) && !same(current, files[path])) {
      conflicts.push(path);
      continue;
    }
    if (files[path] && !same(current, files[path])) install.push(path);
    if (!files[path] && current) remove.push(path);
  }
  for (const path of Object.keys(previous.preserved)) {
    if (!preserved[path] && !review.includes(path)) review.push(path);
  }
  if (conflicts.length) throw new Error(`ローカル変更または管理外ファイルとの衝突。設定保持・パッチ化してから再実行してください:\n${conflicts.join("\n")}`);
  const changed = !!(install.length || remove.length || JSON.stringify(previous) !== JSON.stringify(inventory));
  return { inventory, install, remove, review, changed };
}

// 初回移行だけ、旧配布元のファイル一覧と内容から所有権を確定する。
function adopt(source: string, revision: string): Inventory {
  const files: Record<string, Stamp> = {}, preserved: Record<string, Stamp> = {};
  const commit = git(source, ["rev-parse", "--verify", `${revision}^{commit}`]).trim();
  for (const [harness, root] of ROOTS) {
    const prefix = `dist/${harness}/`;
    for (const line of git(source, ["ls-tree", "-r", "-z", commit, "--", `${prefix}${root}/`]).split("\0").filter(Boolean)) {
      const [meta, fullPath] = line.split("\t");
      const [mode, type, object] = meta.split(" ");
      const path = fullPath.slice(prefix.length);
      validatePath(path);
      if (type !== "blob" || !["100644", "100755"].includes(mode)) throw new Error(`旧配布物の形式が不正: ${path}`);
      const entry = { sha256: digest(gitBytes(source, ["cat-file", "blob", object])), executable: mode === "100755" };
      (PRESERVE.has(path) ? preserved : files)[path] = entry;
    }
  }
  if (!Object.keys(files).length) throw new Error("指定コミットには旧配布物がありません");
  return { format: 1, files, preserved };
}

// 作業ツリーへの書込みはこの関数だけ。先に退避し、削除→コピー→台帳の順で確定する。
export function applyPlan(project: string, stage: string, plan: Plan, copyFile = copy): string {
  const backup = safePath(project, `.aidlc-sync/backups/${randomUUID()}`);
  const targets = [...new Set([...plan.remove, ...Object.keys(plan.inventory.files), ...plan.install, MANIFEST])];
  const existing: string[] = [];
  mkdirSync(backup, { recursive: true });
  for (const path of targets) {
    const destination = safePath(project, path);
    if (existsSync(destination)) {
      copy(destination, join(backup, "files", path));
      existing.push(path);
    }
  }
  writeFileSync(join(backup, "restore.json"), JSON.stringify({ targets, existing }, null, 2) + "\n");
  try {
    for (const path of [...plan.remove, ...Object.keys(plan.inventory.files)]) {
      rmSync(safePath(project, path), { force: true });
    }
    for (const path of new Set([...Object.keys(plan.inventory.files), ...plan.install])) {
      copyFile(join(stage, path), safePath(project, path));
    }
    const manifest = safePath(project, MANIFEST);
    mkdirSync(dirname(manifest), { recursive: true });
    safePath(project, `${MANIFEST}.tmp`);
    writeFileSync(`${manifest}.tmp`, JSON.stringify(plan.inventory, null, 2) + "\n");
    renameSync(`${manifest}.tmp`, manifest);
  } catch (error) {
    // 失敗時は元のファイルと台帳を復元。強制終了時にも退避を残す。
    for (const path of targets) rmSync(safePath(project, path), { force: true });
    for (const path of existing) copy(join(backup, "files", path), safePath(project, path));
    rmSync(join(project, `${MANIFEST}.tmp`), { force: true });
    throw new Error(`更新に失敗したため復元しました。退避: ${backup}`, { cause: error });
  }
  return backup;
}

function run(): void {
  const args = process.argv.slice(2);
  if (args.includes("--help")) {
    console.log("bun scripts/aidlc-sync.ts [--check | --apply] [--adopt-from <commit>] [--accept-preserved]\n既定は差分表示のみ。--check は差分があれば終了コード1。--accept-preserved は保持設定の配布元差分を確認済みとして記録します。");
    return;
  }
  let mode = "plan", revision: string | undefined, acceptPreserved = false;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === "--apply" || arg === "--check") {
      if (mode !== "plan") throw new Error("--apply と --check は同時指定できません");
      mode = arg.slice(2);
    } else if (arg === "--adopt-from") {
      revision = args[++i];
      if (!revision || !/^[a-f0-9]{7,40}$/.test(revision)) throw new Error("--adopt-from にはコミットのハッシュが必要です");
    } else if (arg === "--accept-preserved") acceptPreserved = true;
    else throw new Error(`不明な引数: ${arg}`);
  }
  const project = resolve(import.meta.dir, "..");
  const source = join(project, "vendor/aidlc-workflows");
  if (!existsSync(join(source, ".git"))) throw new Error("git submodule update --init --recursive を実行してください");
  if (git(source, ["status", "--porcelain"]).trim()) throw new Error("配布元 submodule に未コミットの変更があります");
  const commit = git(source, ["rev-parse", "HEAD"]).trim();
  const sourceVersion = readFileSync(join(source, "core/tools/aidlc-version.ts"), "utf8");
  const version = sourceVersion.match(/AIDLC_VERSION = "([^"]+)"/)?.[1];
  if (!version) throw new Error("配布元のバージョンを読めません");
  const manifest = safePath(project, MANIFEST);
  if (revision && existsSync(manifest)) throw new Error("--adopt-from は更新台帳がない初回移行専用です");
  const previous: Inventory = existsSync(manifest) ? JSON.parse(readFileSync(manifest, "utf8"))
    : revision ? adopt(source, revision) : { format: 1, files: {}, preserved: {} };
  const stage = mkdtempSync(join(tmpdir(), "aidlc-sync-"));
  const lock = safePath(project, ".aidlc-sync/lock");
  let locked = false;
  try {
    // apply は計画作成前から排他する。台帳はロック取得後に読み直す。
    if (mode === "apply") {
      mkdirSync(dirname(lock), { recursive: true });
      mkdirSync(lock);
      locked = true;
      if (existsSync(manifest) && readFileSync(manifest, "utf8") !== JSON.stringify(previous, null, 2) + "\n") {
        throw new Error("更新台帳が変わりました。再実行してください");
      }
    }
    for (const [harness, root] of ROOTS) {
      const dist = join(source, "dist", harness);
      // ignore 対象のローカルファイルを混ぜず、submodule の追跡ファイルだけをコピーする。
      const prefix = `dist/${harness}/`;
      const paths = git(source, ["ls-files", "-z", "--", `${prefix}${root}/`])
        .split("\0").filter(Boolean).map(path => path.slice(prefix.length));
      if (!paths.length) throw new Error(`配布物がありません: dist/${harness}/${root}`);
      if (root !== ".agents" && readFileSync(join(dist, root, "tools/aidlc-version.ts"), "utf8") !== sourceVersion) {
        throw new Error(`生成元とバージョンが不一致: ${harness}`);
      }
      for (const path of paths) {
        validatePath(path);
        copy(safePath(dist, path), join(stage, path));
      }
    }
    const patches = join(project, "scripts/aidlc-sync/patches");
    for (const name of readdirSync(patches).filter(name => name.endsWith(".patch")).sort()) {
      git(stage, ["apply", "--check", join(patches, name)]);
      git(stage, ["apply", join(patches, name)]);
    }
    validateProviders(project, stage);
    const plan = planSync(project, stage, previous);
    console.log(`配布元: vendor/aidlc-workflows @ ${commit}\nバージョン: ${version}`);
    for (const path of plan.install) console.log(`COPY   ${path}`);
    for (const path of plan.remove) console.log(`DELETE ${path}`);
    for (const path of plan.review) console.log(`REVIEW ${path}（既存内容を保持）`);
    console.log(`コピー ${plan.install.length}、削除 ${plan.remove.length}、保持設定の確認 ${plan.review.length}`);
    if (!plan.changed) { console.log("同期済みです。"); return; }
    if (mode === "check") { process.exitCode = 1; return; }
    if (mode !== "apply") return;
    if (plan.review.length && !acceptPreserved) throw new Error("保持設定の配布元差分を確認し、必要なマージ後に --accept-preserved を付けて実行してください");
    if (git(source, ["rev-parse", "HEAD"]).trim() !== commit || git(source, ["status", "--porcelain"]).trim()) {
      throw new Error("計画中に配布元が変更されました。再実行してください");
    }
    const backup = applyPlan(project, stage, plan);
    console.log(`更新完了。退避: ${backup}`);
  } finally {
    rmSync(stage, { recursive: true, force: true });
    if (locked) rmSync(lock, { recursive: true });
  }
}

if (import.meta.main) {
  try { run(); } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    if (error instanceof Error && error.cause) console.error(String(error.cause));
    process.exitCode = 1;
  }
}
