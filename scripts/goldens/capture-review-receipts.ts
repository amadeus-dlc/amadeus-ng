#!/usr/bin/env bun
/** 固定本家の通常stage-levelレビュー受領。変更は一時workspaceだけに限定する。 */
import assert from "node:assert/strict";
import { cpSync,mkdirSync,mkdtempSync,readFileSync,readdirSync,rmSync,writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname,join } from "node:path";
import { spawnSync } from "node:child_process";
import { UPSTREAM,verifySource } from "./upstream-source";
const [dist,destination]=process.argv.slice(2);assert(dist&&destination);verifySource(dist);
const corpus=JSON.parse(readFileSync(new URL("../../tests/golden/upstream-a277af21/stage1/cases.json",import.meta.url),"utf8"));
const fixture=corpus.observations.find((c:any)=>c.id==="review/request");assert(fixture);
const parent=mkdtempSync(join(tmpdir(),"aidlc-review-receipts-")),root=join(parent,"workspace");mkdirSync(root);
try {
 cpSync(join(dist,".claude"),join(root,".claude"),{recursive:true});
 const capturedRoot=fixture.input.environment.AIDLC_PROJECT_DIR;
 for(const [path,encoded]of Object.entries(fixture.initial_files)){
  if(typeof encoded!=="string"||path.startsWith(".claude/"))continue;
  const target=join(root,path);mkdirSync(dirname(target),{recursive:true});
  writeFileSync(target,Buffer.from(encoded,"base64").toString("utf8").replaceAll(capturedRoot,root));
 }
 const intents=join(root,"aidlc/spaces/default/intents");const record=join(intents,readFileSync(join(intents,"active-intent"),"utf8").trim());
 const audit=()=>readdirSync(join(record,"audit")).sort().map(p=>readFileSync(join(record,"audit",p),"utf8")).join("");
 const normalize=(text:string)=>text.replaceAll(root,"<ROOT>").replaceAll(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g,"<TS>");
 const base=["review","--stage","requirements-analysis","--reviewer","aidlc-product-lead-agent","--iteration","1"];
 const observations=[];
 for(const [id,args]of [["request",base],["retry-first",[...base,"--retry-pending"]],["retry-second",[...base,"--retry-pending"]]] as const){
  const before=audit();const beforeState=readFileSync(join(record,"aidlc-state.md"),"utf8");
  const result=spawnSync(process.execPath,[join(root,".claude/tools/aidlc-log.ts"),...args],{cwd:root,env:{PATH:process.env.PATH,HOME:join(parent,"home"),AIDLC_PROJECT_DIR:root,CLAUDE_PROJECT_DIR:root},encoding:"utf8"});
  observations.push({id,args,exit:result.status,stdout:normalize(result.stdout),stderr:normalize(result.stderr),audit:normalize(audit().slice(before.length)),state_changed:readFileSync(join(record,"aidlc-state.md"),"utf8")!==beforeState});
  assert.equal(result.status,id==="retry-second"?1:0,`${id}: ${result.stderr}`);
 }
 mkdirSync(dirname(destination),{recursive:true});writeFileSync(destination,JSON.stringify({source:UPSTREAM,capture_command:[process.execPath,import.meta.path,dist,destination],tool_versions:{bun:Bun.version},fixture:"upstream-a277af21/stage1/cases.json#review/request",normalization:"workspace path and ISO timestamp only",observations},null,2)+"\n");
}finally{rmSync(parent,{recursive:true,force:true});}
