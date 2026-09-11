#!/usr/bin/env bun
/** 固定2.7.1の状態遷移ガードのシェル入力境界を採取する。 */
import assert from "node:assert/strict";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const [dist,destination]=process.argv.slice(2);
assert(dist && destination,"固定distと保存先が必要");
verifySource(dist);
const guard=await import(pathToFileURL(join(resolve(dist),".claude/hooks/aidlc-state-transition-guard.ts")).href);
const state="bun .claude/tools/aidlc-state.ts approve stage";
const next="bun .claude/tools/aidlc-orchestrate.ts next";
const commands:[string,string][]=[
 ["plain",state],["read-only","bun .claude/tools/aidlc-state.ts show"],
 ["quoted-path",'bun ".claude/tools/aidlc-state.ts" approve stage'],
 ["single-quoted-path","bun '.claude/tools/aidlc-state.ts' approve stage"],
 ["bun-run","bun run .claude/tools/aidlc-state.ts approve stage"],
 ["absolute-bun","/tools/bun.exe .claude/tools/aidlc-state.ts approve stage"],
 ["assignment",`MODE="safe" ${state}`], ["env",`env -i MODE=safe ${state}`],
 ["command-prefix",`command ${state}`],["exec-prefix",`exec ${state}`],
 ["after-and",`cd project && ${state}`],["after-pipe",`echo text | ${state}`],
 ["after-newline",`echo text\n${state}`],["subshell",`(${state})`],
 ["block",`{ ${state}; }`],["echo-prose",`echo ${state}`],
 ["single-quoted-separator",`echo 'text; ${state}'`],
 ["double-quoted-separator",`echo "text; ${state}"`],
 ["multiline-string",`printf '%s' "text\n${state}\n"`],
 ["literal-heredoc",`cat <<'EOF'\n${state}\nEOF`],
 ["unquoted-heredoc",`cat <<EOF\n${state}\nEOF`],
 ["tab-heredoc",`cat <<-EOF\n\t${state}\n\tEOF`],
 ["function-definition",`later() { ${state}; }`],
 ["function-keyword",`function later { ${state}; }`],
 ["emoji-before-function",`echo '🚀'; later() { ${state}; }`],
 ["double-quoted-substitution",`echo "$( ${state} )"`],
 ["single-quoted-substitution",`echo '$( ${state} )'`],
 ["unquoted-heredoc-substitution",`cat <<EOF\n$( ${state} )\nEOF`],
 ["quoted-heredoc-substitution",`cat <<'EOF'\n$( ${state} )\nEOF`],
 ["newline-after-function",`later() { echo hi; }\n${state}`],
 ["allowed-specialized","bun .claude/tools/aidlc-state.ts practices-promote --stage practices-discovery"],
 ["orchestrate-next",next], ["dispatcher-native","aidlc next"],
 ["dispatcher-ts","bun .claude/tools/aidlc.ts --resume"],
 ["nested-shell",`bash -c '${next}'`],["nested-shell-options",`zsh -o errexit -lc '${next}'`],
 ["eval",`eval '${next}'`],["dynamic-executable","$RUN next"],
 ["literal-executable-assignment",`RUN=bun; $RUN .claude/tools/aidlc-orchestrate.ts next`],
 ["literal-shell-assignment",`RUN='${next}'; bash -c "$RUN"`],
 ["dynamic-shell","bash -c \"$RUN\""],["nice",`nice -n 10 ${next}`],
 ["nohup",`nohup -- ${next}`],["time",`time -f '%e' ${next}`],
 ["env-split",`env -S '${next}'`], ["env-unknown",`env --unknown ${next}`],
 ["command-query",`command -v bun .claude/tools/aidlc-orchestrate.ts next`],
 ["bun-preload",`bun --preload setup.ts .claude/tools/aidlc-orchestrate.ts report --stage x --result completed`],
 ["bun-eval",`bun --eval '${next}'`],
 ["workspace-switch","aidlc intent switch example"], ["workspace-create","aidlc space create other"],
 ["workspace-list","aidlc intent list"],["utility-creation","bun .claude/tools/aidlc-utility.ts intent-create --scope bugfix"],
];
commands.push(
 ["backticks",`echo \`${next}\``],
 ["line-continuation","bu\\\nn .claude/tools/aidlc-orchestrate.ts next"],
 ["eval-option",`eval -- '${next}'`],["eval-dynamic",'eval "$RUN"'],
 ["command-path",`command -p ${next}`],["exec-name",`exec -a worker ${next}`],
 ["exec-flags",`exec -cl ${next}`],["exec-unknown",`exec -z ${next}`],
 ["env-unset",`env -u SECRET ${next}`],["env-chdir",`env --chdir=/tmp ${next}`],
 ["env-signal",`env --default-signal=INT ${next}`],
 ["env-split-attached",`env -S'${next}'`],["env-split-dynamic","env -S '$RUN next'"],
 ["env-help",`env --help ${next}`],["env-null",`env --null ${next}`],
 ["nice-attached",`nice -n-3 ${next}`],["nice-long",`nice --adjustment=+3 ${next}`],
 ["nice-invalid",`nice -n abc ${next}`],["nice-missing","nice -n"],
 ["nohup-help",`nohup --help ${next}`],["time-values",`time --format '%E' --output time.txt ${next}`],
 ["redirection-prefix",`2>/tmp/log env MODE=safe ${next}`],
 ["shell-keywords",`if true; then ${next}; fi`],
 ["dispatcher-scope","aidlc scope change --scope feature"],
 ["dispatcher-config","aidlc config set x y"],
 ["utility-project-dir","bun .claude/tools/aidlc-utility.ts --project-dir /tmp/project intent-create --scope bugfix"],
 ["comment-prose",`echo text # ; ${next}`],
 ["reserved-workspace","aidlc intent rename old new"],
 ["leading-bom",`\uFEFF${next}`],["leading-nel",`\u0085${next}`],
);
for (const depth of [8,9]) { let command=next; for(let level=0;level<depth;level++) command=`bash -c ${JSON.stringify(command)}`; commands.push([`depth-${depth}`,command]); }
const observations:unknown[]=[];
for(const [id,command] of commands){
 for(const [mode,agent_type] of [["main",""],["delegated","aidlc-developer-agent"]]){
  const stdin=JSON.stringify({hook_event_name:"PreToolUse",tool_name:"Bash",agent_type,tool_input:{command}});
  let stderr=""; const original=process.stderr.write;
  process.stderr.write=((chunk: unknown)=>{stderr+=String(chunk);return true;}) as typeof process.stderr.write;
  let exit:number;
  try{exit=await guard.run(stdin);} finally{process.stderr.write=original;}
  observations.push({id:`${mode}/${id}`,mode,stdin,exit,stdout:"",stderr});
 }
}
for(const [id,stdin] of [
 ["malformed-json","{"],["null-json","null"],["array-json","[]"],["missing-tool","{}"],
 ["missing-command",'{"tool_name":"Bash","agent_type":"aidlc-developer-agent","tool_input":{}}'],
 ["non-bash",'{"tool_name":"Read","agent_type":"aidlc-developer-agent","tool_input":{"command":"aidlc next"}}'],
 ["replacement-decoded-invalid-json","\uFFFD"],
]){
 let stderr="";const original=process.stderr.write;process.stderr.write=((chunk:unknown)=>{stderr+=String(chunk);return true;}) as typeof process.stderr.write;
 let exit:number;try{exit=await guard.run(stdin);}finally{process.stderr.write=original;}
 observations.push({id:`envelope/${id}`,mode:"envelope",stdin,exit,stdout:"",stderr});
}
mkdirSync(dirname(destination),{recursive:true});
writeFileSync(destination,JSON.stringify({source:UPSTREAM,capture_command:process.argv,normalization:[],observations},null,2)+"\n");
