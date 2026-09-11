#!/usr/bin/env bun
/** U2 log linkの固定本家観測。一時workspace以外の監査は更新しない。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import { spawnSync } from "node:child_process";
import { UPSTREAM, verifySource } from "./upstream-source";
const [dist,destination]=process.argv.slice(2);assert(dist&&destination);verifySource(dist);
const stage="reverse-engineering", developer="aidlc-developer-agent", architect="aidlc-architect-agent";
const base=["link","--stage",stage];
type Step={id:string,args:string[],file?:"fresh"|"old"|"changed"|"rewritten"|"directory"|"symlink"};
type Case={id:string,cold?:boolean,steps:Step[]};
const cases:Case[]=[
 {id:"syntax",cold:true,steps:[
  {id:"missing-stage",args:["link"]}, {id:"missing-link",args:base},
  {id:"flag-end",args:["link","--stage"]},{id:"flag-next",args:["link","--stage","--link",developer]},
  {id:"selector",args:[...base,"--link",developer,"--intent","other"]},
  {id:"cold",args:[...base,"--link",developer]}]},
 {id:"guards",steps:[
  {id:"not-pipeline",args:["link","--stage","code-generation","--link",developer]},
  {id:"unknown-stage",args:["link","--stage","BOGUS","--link",developer]},
  {id:"unknown-link",args:[...base,"--link","not-declared"]},
  {id:"repo",args:[...base,"--link",developer,"--repo","example"]},
  {id:"order",args:[...base,"--link",architect]},
  {id:"artifact-required",args:[...base,"--link",developer]},
  {id:"artifact-path",args:[...base,"--link",developer,"--artifact","wrong.md"]},
  {id:"artifact-missing",args:[...base,"--link",developer,"--artifact","<HANDOFF>"]},
  {id:"artifact-old",file:"old",args:[...base,"--link",developer,"--artifact","<HANDOFF>"]},
  {id:"single-no-attempt",file:"fresh",args:[...base,"--link",developer,"--artifact","<HANDOFF>","--single"]}]},
 {id:"chain",steps:[
  {id:"developer",file:"fresh",args:[...base,"--link",developer,"--artifact","<HANDOFF>"]},
  {id:"duplicate-developer",args:[...base,"--link",developer,"--artifact","<HANDOFF>"]},
  {id:"architect",args:[...base,"--link",architect]},
  {id:"duplicate-architect",args:[...base,"--link",architect]},
  {id:"changed-architect",file:"changed",args:[...base,"--link",architect]},
  {id:"not-rewritten",args:[...base,"--link",developer,"--artifact","<HANDOFF>"]},
  {id:"rewritten-developer",file:"rewritten",args:[...base,"--link",developer,"--artifact","<HANDOFF>"]},
  {id:"rewritten-architect",args:[...base,"--link",architect]}]},
 {id:"directory",steps:[{id:"directory",file:"directory",args:[...base,"--link",developer,"--artifact","<HANDOFF>"]}]},
 {id:"symlink",steps:[{id:"symlink",file:"symlink",args:[...base,"--link",developer,"--artifact","<HANDOFF>"]}]},
 {id:"blank-flags",steps:[{id:"blank-repo-selector",file:"fresh",args:[...base,"--link",developer,"--artifact","<HANDOFF>","--repo","","--intent","","--space","","--ignored","value","--stage-level"]}]},
];
const observations=[];
for(const scenario of cases){
 const parent=mkdtempSync(join(tmpdir(),"aidlc-link-")),root=join(parent,"workspace");mkdirSync(root);cpSync(join(dist,".claude"),join(root,".claude"),{recursive:true});
 const env={PATH:process.env.PATH,HOME:join(parent,"home")};
 try{
  let record="";
  if(!scenario.cold){writeFileSync(join(root,"source.rs"),"fn main() {}\n");const result=spawnSync(process.execPath,[join(root,".claude/tools/aidlc-utility.ts"),"intent-create","--scope","bugfix","--label","link","--arguments","Fix pipeline link"],{cwd:root,env,encoding:"utf8"});assert.equal(result.status,0,result.stderr);const intents=join(root,"aidlc/spaces/default/intents");record=join(intents,readFileSync(join(intents,"active-intent"),"utf8").trim());}
  const state=()=>record?readFileSync(join(record,"aidlc-state.md"),"utf8"):"";
  const handoff=join(record,"inception/reverse-engineering/developer-scan.md");
  const audit=()=>record?readdirSync(join(record,"audit")).sort().map(p=>readFileSync(join(record,"audit",p),"utf8")).join(""):"";
  const normalize=(s:string)=>s.replaceAll(record?relative(root,record):"<unused>","<RECORD>").replaceAll(root,"<ROOT>").replaceAll(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g,"<TS>");
  const steps=[];
  for(const step of scenario.steps){
   if(step.file){mkdirSync(join(record,"inception/reverse-engineering"),{recursive:true});rmSync(handoff,{force:true,recursive:true});if(step.file==="directory")mkdirSync(handoff);else if(step.file==="symlink"){writeFileSync(join(root,"real.md"),"handoff\n");symlinkSync(join(root,"real.md"),handoff);}else{writeFileSync(handoff,step.file==="changed"?"changed handoff\n":"handoff\n");const seconds=step.file==="old"?946684800:step.file==="rewritten"?4102444801:4102444800;utimesSync(handoff,seconds,seconds);}}
   const before=audit(), beforeState=state();const args=step.args.map(a=>a==="<HANDOFF>"?relative(root,handoff):a);
   const result=spawnSync(process.execPath,[join(root,".claude/tools/aidlc-log.ts"),...args],{cwd:root,env,encoding:"utf8"});
   steps.push({...step,args:step.args,exit:result.status,stdout:normalize(result.stdout),stderr:normalize(result.stderr),audit:normalize(audit().slice(before.length)),state_changed:state()!==beforeState});
  }
  observations.push({id:scenario.id,cold:scenario.cold??false,steps});
 }finally{rmSync(parent,{recursive:true,force:true});}
}
mkdirSync(join(destination,".."),{recursive:true});writeFileSync(destination,JSON.stringify({source:UPSTREAM,capture_command:[process.execPath,import.meta.path,dist,destination],normalization:"selected record and workspace paths; audit ISO timestamps only; handoff mtime is fixed input",observations},null,2)+"\n");
