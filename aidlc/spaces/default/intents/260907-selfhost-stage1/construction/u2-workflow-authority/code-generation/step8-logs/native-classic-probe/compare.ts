import { readFileSync, existsSync, realpathSync } from "node:fs";
const [logs, corpus, root, clone, host] = process.argv.slice(2);
const roots = [root, realpathSync(root)].filter((v, i, a) => a.indexOf(v) === i);
function norm(text: string): string {
  let out = text;
  for (const r of roots.sort((a, b) => b.length - a.length)) out = out.split(r).join("<ROOT>");
  if (clone) out = out.split(clone).join("<CLONE>");
  if (host) out = out.split(host).join("<CLONE>");
  out = out.replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g, "<TS>");
  out = out.replace(/intents\/\d{6}-/g, "intents/<TS>-");
  out = out.replace(/\d{6}-golden/g, "<TS>-golden");
  return out;
}
function window(a: string, b: string): string {
  let i = 0; while (i < a.length && i < b.length && a[i] === b[i]) i++;
  const s = Math.max(0, i - 60);
  return `byte ${i}\n      corpus: ${JSON.stringify(a.slice(s, i + 120))}\n      native: ${JSON.stringify(b.slice(s, i + 120))}`;
}
const pairs: [string, string][] = [
  ["next-no-active-intent", "cli/next/no-active-intent/stdout.json"],
  ["next-start", "cli/next/start/stdout.json"],
  ["continue-load-steering", "cli/continue/load-steering/stdout.json"],
  ["next-stage-jump-print", "cli/next/stage-jump-print/stdout.json"],
  ["jump-resolve", "cli/jump/resolve-forward/stdout.json"],
];
for (const [name, rel] of pairs) {
  const nativePath = `${logs}/${name}.stdout`;
  const exit = existsSync(`${logs}/${name}.exit`) ? readFileSync(`${logs}/${name}.exit`, "utf8").trim() : "?";
  const corpusExit = readFileSync(`${corpus}/${rel.replace(/stdout\.json$/, "exit")}`, "utf8").trim();
  console.log(`=== ${name}  (corpus ${rel})  exit corpus=${corpusExit} native=${exit}`);
  const c = norm(readFileSync(`${corpus}/${rel}`, "utf8").replace(/\n$/, ""));
  const n = norm(readFileSync(nativePath, "utf8").replace(/\n$/, ""));
  if (c === n) { console.log("  IDENTICAL after normalization"); continue; }
  let cj: any, nj: any;
  try { cj = JSON.parse(c); nj = JSON.parse(n); } catch (e) { console.log("  not both JSON; raw diff:\n  " + window(c, n)); continue; }
  const keys = [...new Set([...Object.keys(cj), ...Object.keys(nj)])];
  console.log(`  corpus keys: ${Object.keys(cj).join(",")}`);
  console.log(`  native keys: ${Object.keys(nj).join(",")}`);
  for (const k of keys) {
    const a = JSON.stringify(cj[k]), b = JSON.stringify(nj[k]);
    if (a === b) { console.log(`  = ${k}`); continue; }
    if (!(k in cj)) { console.log(`  + ${k} (native only): ${b?.slice(0, 160)}`); continue; }
    if (!(k in nj)) { console.log(`  - ${k} (corpus only): ${a?.slice(0, 160)}`); continue; }
    if (k === "continue_token") { console.log(`  ~ ${k} differs (token is per-run; lengths corpus=${a.length} native=${b.length})`); continue; }
    console.log(`  ! ${k} differs: ${window(a, b)}`);
  }
}
