#!/usr/bin/env bun
/** 文字列番号の承認済み是正と、未修正本家の観測を分けて記録する。 */
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { resolve,join,dirname } from "node:path";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import { UPSTREAM,verifySource } from "./upstream-source";
const [dist,destination]=process.argv.slice(2);
assert(dist && destination);verifySource(dist);
const root=resolve(import.meta.dir,"../..");const temporary=mkdtempSync(join(tmpdir(),"aidlc-numeric-delta-"));const output=join(temporary,"observed.json");
try{
 const result=Bun.spawnSync(["cargo","test","-p","aidlc","--test","upstream_271_contract","numeric_human_envelopes_preserve_only_string_number_choices","--","--nocapture"],{cwd:root,env:{...process.env,AIDLC_NUMERIC_HUMAN_CAPTURE:output,AIDLC_PLAN_DECISION_SOURCE:resolve(dist),AIDLC_PLAN_DECISION_BUN:process.execPath},stdout:"inherit",stderr:"inherit"});
 assert.equal(result.exitCode,0);
 const observations=JSON.parse(readFileSync(output,"utf8"));
 assert.equal(observations[0].native.response_base64,null);assert.equal(observations[0].upstream.response_base64,null);
 assert.equal(typeof observations[1].native.response_base64,"string");assert.equal(observations[1].upstream.response_base64,null);
 mkdirSync(dirname(destination),{recursive:true});writeFileSync(destination,JSON.stringify({source:UPSTREAM,capture_command:process.argv,native_binary_sha256:createHash("sha256").update(readFileSync(join(root,"target/debug/aidlc"))).digest("hex"),approved_delta:"文字列の番号をJSON数値として捨てず、元の応答として保持する。JSON数値値そのものは受理しない。両者を本家未修正との一致とは呼ばない。",normalization:[],observations},null,2)+"\n");
}finally{rmSync(temporary,{recursive:true,force:true});}
