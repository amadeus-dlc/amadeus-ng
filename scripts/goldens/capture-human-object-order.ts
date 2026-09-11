#!/usr/bin/env bun
/** 固定本家の抽出関数をそのまま実行し、JSONオブジェクトの選択順を記録する。 */
import assert from "node:assert/strict";
import { readFileSync,writeFileSync,mkdirSync,mkdtempSync,rmSync } from "node:fs";
import { join,dirname } from "node:path";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import { UPSTREAM,verifySource } from "./upstream-source";
const [dist,destination]=process.argv.slice(2);assert(dist && destination);verifySource(dist);
const file=join(dist,".claude/hooks/aidlc-record-human-turn.ts");const source=readFileSync(file,"utf8");
const start=source.indexOf("function extractResponseText(");const end=source.indexOf("export async function run(",start);assert(start>=0 && end>start);
const body=source.slice(start,end);
const raw=[
 '{"session_id":"s","prompt":{"2":"Approve Plan","1":"Request Changes"}}',
 '{"session_id":"s","prompt":{"4294967295":"Approve Plan","4294967294":"Request Changes"}}',
 '{"session_id":"s","prompt":{"01":"Approve Plan","1":"Request Changes"}}',
 '{"session_id":"s","prompt":{"-1":"Approve Plan","1":"Request Changes"}}',
 '{"session_id":"s","prompt":{"b":"Approve Plan","a":"Request Changes"}}',
 '{"session_id":"s","prompt":{"1":"Request Changes","answer":"Approve Plan"}}',
 '{"session_id":"s","prompt":{"foo":{"2":"Approve Plan","1":"Request Changes"}}}',
 '{"session_id":"s","prompt":{"2":"Request Changes","1":""}}',
 '{"session_id":"s","prompt":{"4294967295":"Approve Plan","a":"Request Changes"}}',
];
const temporary=mkdtempSync(join(tmpdir(),"aidlc-human-object-"));
try{
 const script=join(temporary,"extract.ts");writeFileSync(script,body+`\nconst raw=${JSON.stringify(raw)}; console.log(JSON.stringify(raw.map((stdin,index)=>({id:String(index+1),stdin,response:extractResponseText(JSON.parse(stdin).prompt)}))));\n`);
 const result=Bun.spawnSync([process.execPath,script],{stdout:"pipe",stderr:"pipe"});assert.equal(result.exitCode,0,result.stderr.toString());
 const observations=JSON.parse(result.stdout.toString());assert.equal(observations[0].response,"Request Changes");
 mkdirSync(dirname(destination),{recursive:true});writeFileSync(destination,JSON.stringify({source:UPSTREAM,source_file:".claude/hooks/aidlc-record-human-turn.ts",extracted_body_sha256:createHash("sha256").update(body).digest("hex"),capture_method:"関数本文を変更せずTypeScriptとして実行。JSONの原入力文字列のキー順も保持する。番号文字列の承認済み差分は別コーパス。",capture_command:process.argv,normalization:[],observations},null,2)+"\n");
}finally{rmSync(temporary,{recursive:true,force:true});}
