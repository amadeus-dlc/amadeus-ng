#!/usr/bin/env bun
/** 固定本家の通常root jumpを一時workspaceで採取する。 */
import assert from "node:assert/strict";
import {cpSync,mkdirSync,mkdtempSync,readFileSync,writeFileSync,rmSync,readdirSync} from "node:fs";
import {tmpdir} from "node:os";
import {join,dirname,relative} from "node:path";
import {spawnSync} from "node:child_process";
import {UPSTREAM,verifySource} from "./upstream-source";
const [dist,destination]=process.argv.slice(2); assert(dist&&destination);verifySource(dist);
const parent=mkdtempSync(join(tmpdir(),"jump-normal-")),root=join(parent,"workspace");
const observations=[];
try{
 mkdirSync(root);cpSync(join(dist,".claude"),join(root,".claude"),{recursive:true});writeFileSync(join(root,"source.rs"),"fn main() {}\n");
 const env={PATH:process.env.PATH,HOME:join(parent,"home")};
 const run=(tool:string,args:string[])=>spawnSync(process.execPath,[join(root,".claude/tools",tool),...args],{cwd:root,env,encoding:"utf8"});
 const created=run("aidlc-utility.ts",["intent-create","--scope","bugfix","--label","jump","--arguments","Fix jump"]);assert.equal(created.status,0,created.stderr);
 const intents=join(root,"aidlc/spaces/default/intents"),record=join(intents,readFileSync(join(intents,"active-intent"),"utf8").trim());
 const audit=()=>readdirSync(join(record,"audit")).sort().map(name=>readFileSync(join(record,"audit",name),"utf8")).join("");
 const normalise=(text:string)=>text.replaceAll(relative(root,record),"<RECORD>").replaceAll(root,"<ROOT>").replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g,"<TS>");
 for(const [id,args] of [
 ["resolve",["resolve","--stage","code-generation"]],
 ["redo",["execute","--target","reverse-engineering","--direction","redo"]],
 ["forward",["execute","--target","code-generation","--direction","forward"]],
 ["backward",["execute","--target","requirements-analysis","--direction","backward"]],
 ["missing-target",["execute","--direction","forward"]]
 ] as const){
  if(id==="backward"){const file=join(record,"construction/code-generation/code-summary.md");mkdirSync(dirname(file),{recursive:true});writeFileSync(file,"# Implementation\n\n## Review\nREADY\n");}
  const before=audit();const result=run("aidlc-jump.ts",[...args]);const appended=audit().slice(before.length);
  observations.push({id,args,exit:result.status,stdout:normalise(result.stdout),stderr:normalise(result.stderr),audit:normalise(appended)});
 }
}finally{rmSync(parent,{recursive:true,force:true});}
mkdirSync(dirname(destination),{recursive:true});writeFileSync(destination,JSON.stringify({source:UPSTREAM,capture_command:[process.execPath,import.meta.path,dist,destination],observations},null,2)+"\n");
