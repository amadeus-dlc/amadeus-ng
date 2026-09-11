#!/usr/bin/env bun
/** U2の通常root pipelineの再開と完了拒否を固定本家から採取する。 */
import assert from "node:assert/strict";
import {cpSync,mkdirSync,mkdtempSync,readFileSync,writeFileSync,rmSync,utimesSync} from "node:fs";
import {tmpdir} from "node:os";
import {join,relative,dirname} from "node:path";
import {spawnSync} from "node:child_process";
import {UPSTREAM,verifySource} from "./upstream-source";
const [dist,destination]=process.argv.slice(2);assert(dist&&destination);verifySource(dist);
const parent=mkdtempSync(join(tmpdir(),"pipeline-progress-")),root=join(parent,"workspace");
const observations=[];
try{
 mkdirSync(root);cpSync(join(dist,".claude"),join(root,".claude"),{recursive:true});writeFileSync(join(root,"source.rs"),"fn main() {}\n");
 const env={PATH:process.env.PATH,HOME:join(parent,"home")};
 const run=(tool:string,args:string[])=>spawnSync(process.execPath,[join(root,".claude/tools",tool),...args],{cwd:root,env,encoding:"utf8"});
 const created=run("aidlc-utility.ts",["intent-create","--scope","bugfix","--label","progress","--arguments","Fix pipeline progress"]);assert.equal(created.status,0,created.stderr);
 const intents=join(root,"aidlc/spaces/default/intents"),record=join(intents,readFileSync(join(intents,"active-intent"),"utf8").trim()),handoff=join(record,"inception/reverse-engineering/developer-scan.md");
 mkdirSync(dirname(handoff),{recursive:true});writeFileSync(handoff,"handoff\n");utimesSync(handoff,4102444800,4102444800);
 const next=(id:string)=>{let result=run("aidlc-orchestrate.ts",["next","--resume"]);let value=JSON.parse(result.stdout);for(let i=0;i<20&&value.kind==="load-steering";i++){result=run("aidlc-orchestrate.ts",["continue",value.continue_token]);value=JSON.parse(result.stdout);}assert.equal(value.kind,"run-stage",result.stdout);observations.push({id,kind:"next",pipeline:value.pipeline});};
 const report=(id:string,result:string,single=false)=>{const args=["report","--stage","reverse-engineering","--result",result,"--user-input",result==="rejected"?"Request Changes":"Approve"];if(result==="rejected")args.push("--reason","test revision");if(single)args.push("--single");const observed=run("aidlc-orchestrate.ts",args);observations.push({id,kind:"report",args,exit:observed.status,stdout:observed.stdout,stderr:observed.stderr});};
 const link=(agent:string)=>{const observed=run("aidlc-log.ts",["link","--stage","reverse-engineering","--link",agent,"--artifact",relative(root,handoff)]);assert.equal(observed.status,0,observed.stderr);};
 next("empty");report("await-empty","awaiting-approval");report("approve-empty","approved");
 {const args=["report","--result","approved","--user-input","Approve"];const r=run("aidlc-orchestrate.ts",args);observations.push({id:"approve-empty-implicit",kind:"report",args,exit:r.status,stdout:r.stdout,stderr:r.stderr});}report("single-empty","completed",true);
 link("aidlc-developer-agent");next("developer");report("await-one","awaiting-approval");
 link("aidlc-architect-agent");next("complete");
 writeFileSync(handoff,"changed\n");utimesSync(handoff,4102444800,4102444800);next("expired");report("await-expired","awaiting-approval");report("valid-rejection","rejected");report("await-revising","awaiting-approval");report("approve-revising","approved");report("revise-revising","revised");
}finally{rmSync(parent,{recursive:true,force:true});}
mkdirSync(dirname(destination),{recursive:true});writeFileSync(destination,JSON.stringify({source:UPSTREAM,capture_command:[process.execPath,import.meta.path,dist,destination],observations},null,2)+"\n");
