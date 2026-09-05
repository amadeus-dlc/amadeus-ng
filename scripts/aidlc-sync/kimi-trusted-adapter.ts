import { existsSync, readFileSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, join, relative, sep } from "node:path";

/**
 * ユーザー所有の信頼リストに cwd の実パスがある場合だけ、プロジェクトのフックへイベントを転送する。
 * 未登録・cwd 欠落・フック不在なら何も実行せず0を返し、転送した場合は子プロセスの終了コードを返す。
 * 元の入力を標準入力へ渡し、標準出力・標準エラーは継承する。外部を指すフックのリンクや不正な JSON は例外にする。
 * このディスパッチャと registryPath は、実行対象のリポジトリ外にある利用者所有のファイルとして配置する。
 */
export async function forward(raw: string, target: string, registryPath: string): Promise<number> {
  const payload = JSON.parse(raw);
  if (typeof payload.cwd !== "string" || !isAbsolute(payload.cwd) || !existsSync(payload.cwd)) return 0;
  const project = realpathSync(payload.cwd);
  if (!existsSync(registryPath)) return 0;
  const trusted: unknown = JSON.parse(readFileSync(registryPath, "utf8"));
  if (!Array.isArray(trusted) || !trusted.includes(project)) return 0;
  const adapter = join(project, ".kimi-code/hooks/aidlc-kimi-adapter.ts");
  if (!existsSync(adapter)) return 0;
  const path = relative(project, realpathSync(adapter));
  if (path.split(sep)[0] === ".." || isAbsolute(path)) throw new Error("信頼済みプロジェクトの外を指すフックは実行できません");
  const child = Bun.spawn([process.execPath, adapter, target], {
    cwd: project, stdin: "pipe", stdout: "inherit", stderr: "inherit",
  });
  child.stdin.write(raw);
  child.stdin.end();
  return await child.exited;
}

if (import.meta.main) {
  try {
    process.exitCode = await forward(await Bun.stdin.text(), process.argv[2] ?? "", join(dirname(import.meta.path), "trusted-projects.json"));
  } catch (error) {
    console.error(`AI-DLC trusted hook: ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  }
}
