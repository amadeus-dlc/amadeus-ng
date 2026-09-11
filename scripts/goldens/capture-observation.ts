import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { digest, UPSTREAM } from "./upstream-source";

const manifestBytes = readFileSync(new URL("./upstream-manifest.sha256", import.meta.url), "utf8");
if (digest(manifestBytes) !== UPSTREAM.manifest) throw new Error("固定配布の参照マニフェストが不一致");
const distributed = new Map(manifestBytes.trimEnd().split("\n").map(line => [line.slice(66), line.slice(0, 64)]));

/** 配布コードは固定元のマニフェストが前提。生成/変更された作業ファイルは全文保存する。 */
export function snapshot(root: string): Record<string, string | null> {
  const result: Record<string, string | null> = {};
  const usesDistribution = existsSync(join(root, ".claude/tools"));
  const seen = new Set<string>();
  function visit(dir: string) {
    for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      if (entry.name === ".git") continue;
      // 採取用 HOME の下に bun / OS が作るキャッシュは観測でない（Linux の bun は
      // `~/.bun/install/cache` に `.pile` を置き、macOS は `~/Library/Caches` に置く）。
      if (relative(root, dir).startsWith("aidlc/.capture-home") && ["Library", ".cache", ".bun"].includes(entry.name)) continue;
      const path = join(dir, entry.name);
      if (entry.isDirectory()) visit(path);
      else if (entry.isFile()) {
        const name = relative(root, path), bytes = readFileSync(path);
        seen.add(name);
        if (!usesDistribution || distributed.get(name) !== digest(bytes)) result[name] = bytes.toString("base64");
      }
      else throw new Error(`採取対象に通常ファイル以外がある: ${path}`);
    }
  }
  if (existsSync(root)) visit(root);
  if (usesDistribution) for (const path of distributed.keys()) if (!seen.has(path)) result[path] = null;
  return result;
}

export function isolatedEnvironment(root: string, overrides: Record<string, string> = {}): Record<string, string> {
  return {
    PATH: `${dirname(process.execPath)}:/usr/bin:/bin:/usr/sbin:/sbin`, HOME: join(root, "aidlc/.capture-home"),
    LANG: "C.UTF-8", LC_ALL: "C.UTF-8", TZ: "UTC",
    CLAUDE_PROJECT_DIR: root, AIDLC_PROJECT_DIR: root,
    ...overrides,
  };
}

export function captureObservation(options: {
  root: string; argv: string[]; stdin?: string; environment?: Record<string, string>;
}) {
  const environment = isolatedEnvironment(options.root, options.environment);
  const initial_files = snapshot(options.root);
  const capture_started_at = new Date().toISOString();
  const result = spawnSync(options.argv[0], options.argv.slice(1), {
    cwd: options.root, env: environment, input: options.stdin ?? "", maxBuffer: 32 * 1024 * 1024,
  });
  const after = snapshot(options.root);
  const changed_files = Object.fromEntries([...new Set([...Object.keys(initial_files), ...Object.keys(after)])]
    .filter(path => initial_files[path] !== after[path]).map(path => [path, after[path] ?? null]));
  return {
    capture_started_at, capture_finished_at: new Date().toISOString(),
    input: { argv: options.argv, stdin: options.stdin ?? "", environment }, initial_files, changed_files,
    output: {
      exit_code: result.status ?? null, signal: result.signal ?? null, error: result.error?.message ?? null,
      stdout: (result.stdout ?? Buffer.alloc(0)).toString("utf8"),
      stderr: (result.stderr ?? Buffer.alloc(0)).toString("utf8"),
      stdout_base64: (result.stdout ?? Buffer.alloc(0)).toString("base64"),
      stderr_base64: (result.stderr ?? Buffer.alloc(0)).toString("base64"),
    },
  };
}
