#!/usr/bin/env bun
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { comparableObservation, type Binding } from "./corpus-normalization";
import { sealCorpus } from "./verify-corpus";
import { listFiles } from "./upstream-source";

/** 観測された生成値だけを役割別に登録する。任意のハッシュやUUID形を一括削除しない。 */
export function prepareCorpus(root: string) {
  const bindings: Binding[] = [];
  const counts = new Map<string, number>();
  const add = (kind: Binding["kind"], value: string, role: string) => {
    if (!value || bindings.some(entry => entry.value === value)) return;
    const count = (counts.get(role) ?? 0) + 1;
    counts.set(role, count);
    const fixed: Record<string, string> = { time: "<time>", duration: "**Duration ms**: <duration>", mtime: "**Artifact Mtime Ms**: <time>", "learned-day": "(learned <date>)" };
    bindings.push({ kind, value, replacement: fixed[role] ?? `<${role}:${count}>` });
  };
  const paths = ["observations.json", "supplemental/cases.json", "stage1/cases.json", "doctor/cases.json", "learnings/cases.json"];
  const texts: string[] = [];
  const observedTimes: string[] = [];
  for (const path of paths) {
    if (!existsSync(join(root, path))) continue;
    const document = JSON.parse(readFileSync(join(root, path), "utf8"));
    const observations = Array.isArray(document) ? document : document.observations;
    for (const observation of observations) {
      const env = observation.input.environment;
      add("path", env.AIDLC_PROJECT_DIR, "root");
      add("path", observation.input.argv[0].startsWith("/") ? observation.input.argv[0] : "", "tool");
      add("path", env.PATH, "tool");
      if (env.TMPDIR) add("path", env.TMPDIR, "tool");
      observedTimes.push(observation.capture_started_at, observation.capture_finished_at);
      try {
        const output = JSON.parse(observation.output.stdout);
        if (typeof output.continue_token === "string") add("token", output.continue_token, "token");
      } catch { /* 非JSONの公開出力には継続トークンがない。 */ }
      for (const match of observation.output.stdout.matchAll(/[A-Za-z0-9_-]{200,}/g)) {
        try {
          const token = JSON.parse(Buffer.from(match[0], "base64url").toString("utf8"));
          if (token.p?.v === 1 && typeof token.m === "string") add("token", match[0], "token");
        } catch { /* tokenの封筒と一致しない固定文字列は保持する。 */ }
      }
      for (const [file, data] of Object.entries(observation.changed_files as Record<string, string | null>)) {
        if (!data) continue;
        const text = Buffer.from(data, "base64").toString("utf8");
        if (file.endsWith("/intents.json")) {
          for (const match of text.matchAll(/"uuid":\s*"([a-f0-9-]{36})"/g)) add("identifier", match[1], "id");
        }
        if (file.includes("/audit/")) {
          add("clone", file.split("/").at(-1)!.replace(/\.md$/, ""), "clone");
          for (const match of text.matchAll(/^\*\*(?:Directive Epoch|Approval Fingerprint|Questions SHA-256|Prompt SHA-256)\*\*: ((?:sha256:)?[a-f0-9]{64})$/gm)) add("digest", match[1], "digest");
          for (const match of text.matchAll(/^\*\*Fire id\*\*: ([a-f0-9]{8})$/gm)) add("fire-id", match[1], "fire");
          for (const match of text.matchAll(/^\*\*Duration ms\*\*: \d+$/gm)) add("duration", match[0], "duration");
          for (const match of text.matchAll(/^\*\*Artifact Mtime Ms\*\*: \d+(?:\.\d+)?$/gm)) add("mtime", match[0], "mtime");
        }
        for (const match of file.matchAll(/\b(\d{6}-(?:golden|stage1|doctor|learnings))\b/g)) add("record-date", match[1], "record");
        if (file.endsWith("/memory/project.md")) for (const match of text.matchAll(/\(learned \d{4}-\d\d-\d\d\)/g)) add("learned-day", match[0], "learned-day");
      }
      for (const [file, data] of Object.entries((observation.fixture_changes ?? {}) as Record<string, string | null>)) {
        if (data && file.includes("/.aidlc-hooks-health/")) {
          for (const match of Buffer.from(data, "base64").toString("utf8").matchAll(/\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z/g)) add("time", match[0], "time");
        }
      }
    }
    texts.push(JSON.stringify(comparableObservation(document)));
  }
  // 生成時刻の範囲だけを許可し、入力資料中の固定日付は保持する。
  const dates = observedTimes.filter(Boolean).map(Date.parse);
  if (existsSync(join(root, "source.json"))) {
    const source = JSON.parse(readFileSync(join(root, "source.json"), "utf8"));
    for (const arg of source.capture_argv ?? []) if (typeof arg === "string" && arg.startsWith("/")) add("path", arg, "tool");
  }
  for (const path of listFiles(root).filter(path => /(?:provenance|case)\.json$/.test(path))) texts.push(readFileSync(path, "utf8"));
  const floor = Math.min(...dates) - 2000, ceiling = Math.max(...dates) + 2000;
  for (const text of [...texts, ...observedTimes.filter(Boolean)]) {
    for (const match of text.matchAll(/\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z/g)) {
      const at = Date.parse(match[0]);
      if (at >= floor && at <= ceiling) add("time", match[0], "time");
    }
  }
  writeFileSync(join(root, "comparison.json"), JSON.stringify({ version: 1, bindings }, null, 2) + "\n");
  sealCorpus(root);
}

if (import.meta.main) prepareCorpus(process.argv[2]);
