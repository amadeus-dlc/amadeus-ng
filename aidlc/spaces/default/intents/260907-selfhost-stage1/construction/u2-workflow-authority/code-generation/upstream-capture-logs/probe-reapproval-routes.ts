#!/usr/bin/env bun
/**
 * 探索: 承認済み → 再跳躍 → もう一度承認済み へ持って行ける経路が両実装にあるか。
 * 対象は初期化段 `state-init`（本家は report --result completed の advance で戻せる）。
 * 一時 workspace のみを触る。
 *
 *   bun probe-reapproval-routes.ts <dist> <rust-binary>
 */
import { copyFileSync, cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const [dist, rustBin] = process.argv.slice(2);

function build(impl: "upstream" | "rust") {
  const parent = mkdtempSync(join(tmpdir(), `aidlc-route-${impl}-`));
  const root = join(parent, "workspace");
  mkdirSync(root);
  cpSync(join(dist, ".claude"), join(root, ".claude"), { recursive: true });
  writeFileSync(join(root, "source.rs"), "fn main() {}\n");
  const env = { PATH: process.env.PATH!, HOME: join(parent, "home") };
  const bin = join(parent, "bin");
  mkdirSync(bin);
  for (const n of ["aidlc", "aidlc-utility", "aidlc-jump", "aidlc-orchestrate", "aidlc-state", "aidlc-log"]) {
    copyFileSync(rustBin, join(bin, n));
  }
  // stdin は必ず閉じる（引数なしの `next` が入力待ちで固まらないように）。
  // 20 秒で打ち切り、固まった経路も観測として残す。
  const opts = { cwd: root, env, encoding: "utf8" as const, input: "", timeout: 20_000 };
  const tool = (name: string, args: string[]) =>
    impl === "upstream"
      ? spawnSync(process.execPath, [join(root, ".claude/tools", `${name}.ts`), ...args], opts)
      : spawnSync(join(bin, name), args, opts);
  const created = tool("aidlc-utility", ["intent-create", "--scope", "bugfix", "--label", "route", "--arguments", "probe"]);
  if (created.status !== 0) throw new Error(`${impl} intent-create failed: ${created.stderr}`);
  const intents = join(root, "aidlc/spaces/default/intents");
  const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());
  return { impl, root, parent, record, tool };
}

const show = (impl: string, id: string, r: any) => {
  const out = (r.stdout || "").trim().replace(/\s+/g, " ").slice(0, 200);
  const err = (r.stderr || "").trim().replace(/\s+/g, " ").slice(0, 200);
  console.log(`  [${impl}] ${id.padEnd(44)} exit=${r.status}`);
  if (out) console.log(`      out: ${out}`);
  if (err) console.log(`      err: ${err}`);
};

for (const impl of ["upstream", "rust"] as const) {
  console.log(`\n===================== ${impl} =====================`);
  const h = build(impl);
  try {
    const cb = () => {
      const state = readFileSync(join(h.record, "aidlc-state.md"), "utf8");
      return (state.split("\n").find((l) => l.includes("state-init —")) ?? "?").trim();
    };
    show(impl, "next bugfix", h.tool("aidlc-orchestrate", ["next", "bugfix"]));
    console.log(`      state-init: ${cb()}`);
    show(impl, "jump redo state-init", h.tool("aidlc-jump", ["execute", "--target", "state-init", "--direction", "redo"]));
    console.log(`      state-init: ${cb()}`);

    // 再承認の候補経路を順に試す。checkbox が [x] へ戻った時点で成功。
    const routes: [string, string, string[]][] = [
      ["orchestrate", "next（引数なし）", ["next"]],
      ["orchestrate", "report --result completed", ["report", "--stage", "state-init", "--result", "completed"]],
      ["orchestrate", "report --result approved", ["report", "--stage", "state-init", "--result", "approved"]],
      ["orchestrate", "report --result awaiting-approval", ["report", "--stage", "state-init", "--result", "awaiting-approval"]],
      ["aidlc-state", "aidlc-state approve", ["approve", "--stage", "state-init"]],
    ];
    for (const [face, label, args] of routes) {
      const toolName = face === "aidlc-state" ? "aidlc-state" : "aidlc-orchestrate";
      show(impl, label, h.tool(toolName, args));
      const line = cb();
      console.log(`      state-init: ${line}`);
      if (line.startsWith("- [x]")) { console.log(`      ==> ${label} で承認済みへ戻った`); break; }
    }
  } catch (e) {
    console.log("  ERROR:", e instanceof Error ? e.message : String(e));
  } finally {
    rmSync(h.parent, { recursive: true, force: true });
  }
}
