#!/usr/bin/env bun
import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync, realpathSync, renameSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { randomUUID } from "node:crypto";

type Hook = { event: string; command: string; matcher?: string; timeout?: number };
const begin = "# BEGIN amadeus-ng AI-DLC hooks";
const end = "# END amadeus-ng AI-DLC hooks";
const quote = (value: string) => `'${value.replaceAll("'", `'"'"'`)}'`;

export function configure(projectPath: string, home: string, apply: boolean): void {
  const project = realpathSync(projectPath);
  const directory = join(home, "aidlc");
  const config = join(home, "config.toml");
  const wrapper = join(directory, "aidlc-kimi-adapter.ts");
  const registry = join(directory, "trusted-projects.json");
  const source = readFileSync(join(project, "scripts/aidlc-sync/kimi-trusted-adapter.ts"), "utf8");
  const snippet = Bun.TOML.parse(readFileSync(join(project, ".kimi-code/hooks.snippet.toml"), "utf8")) as { hooks: Hook[] };
  const original = existsSync(config) ? readFileSync(config, "utf8") : "";
  // 旧来の、このリポジトリが登録したブロックも置換する。その他の設定は逐語保持。
  let kept = original;
  const start = original.indexOf(begin), finish = original.indexOf(end);
  if (start >= 0 || finish >= 0) {
    if (start < 0 || finish < start || original.indexOf(begin, start + begin.length) >= 0) throw new Error("フック設定の管理マーカーが不正です");
    kept = original.slice(0, start) + original.slice(finish + end.length);
  }
  const old = Bun.TOML.parse(kept) as { hooks?: Hook[] };
  if (old.hooks?.some(h => h.command.includes("aidlc-kimi-adapter.ts"))) {
    throw new Error("管理ブロック外に AI-DLC フックがあります。重複する旧登録を確認・除去してください");
  }
  const hooks = snippet.hooks.map(h => {
    const target = h.command.match(/aidlc-kimi-adapter\.ts\s+([a-z-]+)$/)?.[1];
    if (!target) throw new Error("スニペットのフック対象を読めません");
    return { ...h, command: `${quote(process.execPath)} ${quote(wrapper)} ${target}` };
  });
  const block = hooks.map(h => "[[hooks]]\n" + Object.entries(h).map(([k, v]) => `${k} = ${JSON.stringify(v)}`).join("\n")).join("\n\n");
  const updated = `${kept.trimEnd()}\n\n${begin}\n${block}\n${end}\n`;
  Bun.TOML.parse(updated);
  const registered: unknown = existsSync(registry) ? JSON.parse(readFileSync(registry, "utf8")) : [];
  if (!Array.isArray(registered) || registered.some(p => typeof p !== "string")) throw new Error("信頼リストの形式が不正です");
  const projects = [...new Set([...registered, project])].sort();
  if (!apply) {
    if (!registered.includes(project) || original !== updated || !existsSync(wrapper) || readFileSync(wrapper, "utf8") !== source) {
      throw new Error("Kimi フックが未同期です。bun scripts/aidlc-kimi-hooks.ts --trust を実行してください");
    }
    return;
  }
  const backup = join(project, ".aidlc-sync/backups", `kimi-hooks-${randomUUID()}`);
  mkdirSync(backup, { recursive: true, mode: 0o700 });
  for (const [path, name] of [[config, "config.toml"], [wrapper, "aidlc-kimi-adapter.ts"], [registry, "trusted-projects.json"]]) {
    if (existsSync(path)) { copyFileSync(path, join(backup, name)); chmodSync(join(backup, name), 0o600); }
  }
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  for (const [path, content] of [[wrapper, source], [registry, JSON.stringify(projects, null, 2) + "\n"], [config, updated]]) {
    const temporary = `${path}.${randomUUID()}.tmp`;
    writeFileSync(temporary, content, { mode: 0o600 });
    renameSync(temporary, path);
  }
  console.log(`Kimi の信頼済みプロジェクトを登録しました: ${project}\n退避: ${backup}`);
}

if (import.meta.main) {
  try {
    const arg = process.argv[2];
    if (process.argv.length !== 3 || !["--trust", "--check"].includes(arg)) throw new Error("使用法: bun scripts/aidlc-kimi-hooks.ts --trust | --check");
    configure(resolve(import.meta.dir, ".."), process.env.KIMI_CODE_HOME ?? join(homedir(), ".kimi-code"), arg === "--trust");
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
