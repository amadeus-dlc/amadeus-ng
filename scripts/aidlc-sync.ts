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

/** 指定ディレクトリで Git を実行し、文字コード変換せず標準出力を返す。失敗時は標準エラーで例外にする。 */
function gitBytes(cwd: string, args: string[]): Buffer {
  const result = Bun.spawnSync(["git", ...args], { cwd, stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) throw new Error(result.stderr.toString().trim());
  return result.stdout;
}

/** Git のテキスト出力を UTF-8 として返す。バイナリの取得には gitBytes を使う。 */
function git(cwd: string, args: string[]): string {
  return gitBytes(cwd, args).toString();
}

/** 配布ファイルの内容比較に使う SHA-256 を16進数で返す。 */
function digest(data: Uint8Array): string {
  return createHash("sha256").update(data).digest("hex");
}

/** 通常ファイルの内容ハッシュと実行権限の有無を、更新台帳用に取得する。 */
function stamp(path: string): Stamp {
  return { sha256: digest(readFileSync(path)), executable: (lstatSync(path).mode & 0o111) !== 0 };
}

/** 両方のファイルが存在し、内容と実行権限が一致する場合だけ true を返す。 */
function same(a: Stamp | undefined, b: Stamp | undefined): boolean {
  return !!a && !!b && a.sha256 === b.sha256 && a.executable === b.executable;
}

/** プロジェクト相対パスが、このスクリプトで同期するハーネス配下か判定する。 */
function managed(path: string): boolean {
  return ROOTS.some(([, root]) => path.startsWith(`${root}/`));
}

/** 管理対象外のパス、空のパス成分、親参照、バックスラッシュを拒否する。 */
function validatePath(path: string): void {
  if (path.includes("\\") || path.split("/").some(p => !p || p === "." || p === "..") || !managed(path)) {
    throw new Error(`管理対象外のパス: ${path}`);
  }
}

/**
 * 既存の各パス成分にシンボリックリンクがないことを確認し、連結したパスを返す。
 * 未作成の成分は許容する。入力は呼出側で検証済みの相対パスに限り、範囲検証は validatePath が担う。
 */
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

/** prefix 配下の通常ファイルを相対パスで列挙する。存在しないディレクトリは空配列、リンク等は例外にする。 */
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

/** コピー先の親を作成し、ファイル内容と POSIX の許可ビットをコピーする。 */
function copy(from: string, to: string): void {
  mkdirSync(dirname(to), { recursive: true });
  copyFileSync(from, to);
  chmodSync(to, lstatSync(from).mode & 0o777);
}

/** ファイル一覧をソートし、パスから内容ハッシュ・実行権限への対応表を作る。paths はその場でソートする。 */
function inspect(root: string, paths: string[]): Record<string, Stamp> {
  return Object.fromEntries(paths.sort().map(path => [path, stamp(join(root, path))]));
}

/** 旧台帳の形式、管理パス、ハッシュ、権限フラグを検証し、不正なら更新計画の作成前に拒否する。 */
function validateInventory(inventory: Inventory): void {
  if (inventory.format !== 1 || !inventory.files || !inventory.preserved) throw new Error("更新台帳の形式が不正です");
  for (const [path, entry] of Object.entries({ ...inventory.files, ...inventory.preserved })) {
    validatePath(path);
    if (!/^[a-f0-9]{64}$/.test(entry.sha256) || typeof entry.executable !== "boolean") {
      throw new Error(`更新台帳のハッシュが不正です: ${path}`);
    }
  }
}

/**
 * 導入後に使われる設定が Bedrock を有効にしていないか検証する。
 * 既存のプロジェクト設定と Claude の個人設定を優先し、未配置の設定だけ stage から読む。
 * 設定の読込・解析に失敗した場合も例外とし、更新を進めない。
 */
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

/**
 * 現在の導入物、パッチ適用済み配布物、旧台帳から、書込みを行わず更新計画を作る。
 * 保持設定の変更は review に分け、管理外ファイルとの衝突や未登録のローカル変更は例外にする。
 * 戻り値の inventory は適用成功後に保存する新しい台帳である。
 */
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

/** 初回移行用に、旧配布コミットの追跡ファイルから台帳を復元する。配布物がない版やリンクは拒否する。 */
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

/**
 * 検証済みの計画を、退避→管理ファイルの削除→コピー→台帳更新の順で適用する。
 * 同期処理の導入先への書込みを担当し、成功時は退避先を返す。保持設定は未配置のものだけをコピーする。
 * 適用中の例外では元のファイルと台帳の復元を試みる。強制終了・復元自体の失敗では退避からの手動復旧が必要。
 * @param copyFile ファイルコピー処理。テストでコピー障害を再現するために差し替えられる。
 */
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

/**
 * CLI 引数を解釈し、固定された submodule の配布物を準備・検証して同期する。
 * 既定は差分表示のみ。--check は差分があれば終了コード1、--apply は排他ロック下で計画を適用する。
 * 一時配布物と取得済みロックは、成功・失敗にかかわらず解放する。
 */
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
