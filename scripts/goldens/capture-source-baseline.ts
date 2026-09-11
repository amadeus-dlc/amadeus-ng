#!/usr/bin/env bun
/** U2開始基準の追加観測。固定本家の実コードからTSV全体を採取する。 */
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync, symlinkSync, chmodSync, cpSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";
import { UPSTREAM, verifySource } from "./upstream-source";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination); verifySource(dist);
const upstream = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-lib.ts")).href);
type Case = { id: string; files: Record<string, string>; outside?: Record<string, string>; executable?: string[]; symlinks?: Record<string, string>; git_repositories?: string[] };
const cases: Case[] = [
 { id:"empty", files:{} },
 { id:"regular-and-exclusions", files:{"src/lib.rs":"fn main() {}\n", "README.md":"# Demo\n", "target/output":"ignored", "aidlc/note.md":"ignored", ".claude/tools/data/harness.json":'{"name":"claude"}\n', ".claude/tool.ts":"ignored"} },
 { id:"escaping-and-utf16-order", files:{"a\tb\nc\rd\\e":"odd", "\u{10000}.txt":"astral", "\ue000.txt":"bmp", "run.sh":"#!/bin/sh\n"}, executable:["run.sh"] },
 { id:"internal-and-dangling-links", files:{"src/file.rs":"linked\n"}, symlinks:{"alias":"src", "broken":"missing", "src/cycle":".."} },
 { id:"external-links", files:{"local":"here"}, outside:{"code.rs":"external\n", "readme":"text\n", "binary":"\0not source"}, symlinks:{"external":"../outside"} },
 { id:"registered-conditional", files:{".aidlc-source-paths.json":'{"version":1,"paths":["target/source","dist/exact.bin"]}\n', "target/source/file.bin":"\0source", "target/ignored":"ignored", "dist/exact.bin":"payload", "dist/ignored":"ignored"} },
 { id:"nested-cache-boundary", files:{"package/aidlc/spaces/default/intents/record/.aidlc-sensors/cache":"ignored", "package/aidlc/spaces/default/knowledge/.aidlc-sensors/source":"included"} },
 { id:"registered-harness-without-manifest", files:{".aidlc-source-paths.json":'{"version":1,"paths":[".codex/source.rs"]}',".codex/source.rs":"not registrable"} },
 { id:"malformed-registry", files:{".aidlc-source-paths.json":"invalid"} },
 { id:"registered-missing", files:{".aidlc-source-paths.json":'{"version":1,"paths":["target/missing"]}'} },
 { id:"embedded-git", files:{"nested/file.rs":"tracked\n"}, git_repositories:["nested"] },
 { id:"embedded-unborn", files:{"nested/.git/HEAD":"ref: refs/heads/main\n", "nested/file":"present"} },
];
const observations=[];
for(const input of cases) {
 const parent=mkdtempSync(join(tmpdir(),"aidlc-source-baseline-")), root=join(parent,"workspace"); mkdirSync(root);
 try {
  for(const [base, files] of [[root,input.files],[join(parent,"outside"), input.outside??{}]] as const)
   for(const [path,body] of Object.entries(files)) {mkdirSync(dirname(join(base,path)),{recursive:true});writeFileSync(join(base,path),body);}
  for(const path of input.executable??[]) chmodSync(join(root,path),0o755);
  for(const [path,target] of Object.entries(input.symlinks??{})) {mkdirSync(dirname(join(root,path)),{recursive:true});symlinkSync(target,join(root,path));}
  for(const repo of input.git_repositories??[]) {
   const env={PATH:process.env.PATH,HOME:parent,GIT_CONFIG_NOSYSTEM:"1",GIT_CONFIG_GLOBAL:"/dev/null",GIT_AUTHOR_NAME:"Baseline",GIT_AUTHOR_EMAIL:"baseline@example.invalid",GIT_COMMITTER_NAME:"Baseline",GIT_COMMITTER_EMAIL:"baseline@example.invalid",GIT_AUTHOR_DATE:"2026-09-09T00:00:00Z",GIT_COMMITTER_DATE:"2026-09-09T00:00:00Z"};
   for(const args of [["init","--initial-branch=main"],["add","."],["-c","commit.gpgsign=false","commit","-m","baseline"]]) {
    const result=spawnSync("git",["-C",join(root,repo),...args],{env}); assert.equal(result.status,0,result.stderr.toString());
   }
  }
  const state=upstream.workspaceSourceState(root);
  const listing=state===null?null:upstream.serializeSourceListing(state.listing);
  observations.push({...input,listing, fingerprint: listing===null?null:`sha256:${upstream.sourceListingSha256(listing)}`});
 } finally {rmSync(parent,{recursive:true,force:true});}
}
// 開始CLIも固定本家の配布束をそのまま使い、公開監査欄とTSVを採る。
const parent=mkdtempSync(join(tmpdir(),"aidlc-source-baseline-start-"));
let start_observation;
try {
 const root=join(parent,"workspace");mkdirSync(root);cpSync(join(dist,".claude"),join(root,".claude"),{recursive:true});
 const files={"source.rs":"fn main() {}\n"};for(const [path,body] of Object.entries(files))writeFileSync(join(root,path),body);
 const args=["intent-create","--scope","bugfix","--label","baseline","--arguments","Fix source baseline"];
 const result=spawnSync(process.execPath,[join(root,".claude/tools/aidlc-utility.ts"),...args],{cwd:root,env:{PATH:process.env.PATH,HOME:join(parent,"home")},encoding:"utf8"});
 assert.equal(result.status,0,result.stderr);
 const intents=join(root,"aidlc/spaces/default/intents");const record=join(intents,readFileSync(join(intents,"active-intent"),"utf8").trim());
 const audit=readdirSync(join(record,"audit")).map(file=>readFileSync(join(record,"audit",file),"utf8")).join("");
 const field=/\*\*Source Baseline\*\*: ([^\n]+)/.exec(audit)?.[1];assert(field);
 const snapshots=join(record,".aidlc-source-review/code-generation");const name=readdirSync(snapshots).find(file=>file.startsWith("baseline-"));assert(name);
 start_observation={files,args,exit:result.status,stderr:result.stderr,source_baseline:field,snapshot:name,listing:readFileSync(join(snapshots,name),"utf8")};
} finally {rmSync(parent,{recursive:true,force:true});}
mkdirSync(dirname(destination),{recursive:true});
writeFileSync(destination,JSON.stringify({source:UPSTREAM,capture_command:[process.execPath,import.meta.path,dist,destination],tool_versions:{bun:Bun.version},normalization:"none",observations,start_observation},null,2)+"\n");
