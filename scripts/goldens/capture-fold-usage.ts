#!/usr/bin/env bun
/**
 * 固定本家の `aidlc-fold-usage.ts` を **有効なまま** 実プロセスで観測する。
 *
 * U1 の採取は `AIDLC_DISABLE_USAGE_TRACKING=1` を立てており、有効経路の証拠が無い。
 * ここは停止フラグを立てずに撃ち、session/transcript ポインタと利用量台帳
 * (`aidlc/.aidlc-sessions/usage-ledger.json`) の実バイトを採る。
 *
 *   bun scripts/goldens/capture-fold-usage.ts <verified-dist/claude> <output.json>
 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);

const SESSION = "session-fold";

/** 1 回の assistant 応答を表す JSONL 行。 */
function assistant(
  uuid: string,
  msgId: string,
  model: string,
  input: number,
  output: number,
  cacheRead: number,
  cacheWrite: number,
  extra: Record<string, unknown> = {},
): string {
  return JSON.stringify({
    uuid, timestamp: "2026-09-10T00:00:00Z", type: "assistant", ...extra,
    message: {
      id: msgId, role: "assistant", model,
      usage: {
        input_tokens: input, output_tokens: output,
        cache_read_input_tokens: cacheRead,
        cache_creation_input_tokens: cacheWrite,
        cache_creation: { ephemeral_5m_input_tokens: cacheWrite, ephemeral_1h_input_tokens: 0 },
      },
    },
  });
}

/** 会話履歴の素材。分割行・未知世代・id 無し・非 assistant・壊れた行を混ぜてある。 */
const LINES: Record<string, string[]> = {
  simple: [
    assistant("u1", "msg_1", "claude-opus-4-8", 100, 20, 0, 0),
    assistant("u2", "msg_2", "claude-opus-4-8", 200, 40, 0, 0),
  ],
  split: [
    assistant("s1", "msg_a", "claude-sonnet-5", 0, 0, 0, 0),
    assistant("s2", "msg_a", "claude-sonnet-5", 1000, 300, 5000, 2000),
    assistant("s3", "msg_b", "claude-haiku-4-5-20251001", 10, 5, 0, 0),
  ],
  mixed: [
    assistant("m1", "msg_x", "claude-opus-9-9", 70, 7, 0, 0),
    JSON.stringify({ uuid: "m2", timestamp: "2026-09-10T00:00:01Z", type: "user", message: { role: "user", content: "hi" } }),
    "{ this is not json",
    assistant("m3", "", "claude-sonnet-5", 11, 3, 0, 0),
  ],
  bedrock: [
    assistant("b1", "msg_p", "converse/us.anthropic.claude-opus-4-8", 40, 8, 0, 0),
    assistant("b2", "msg_q", "global.anthropic.claude-opus-4-8[1m]", 60, 12, 0, 0),
    assistant("b3", "msg_r", "<synthetic>", 1, 1, 0, 0),
  ],
  short: [assistant("r1", "msg_short", "claude-opus-4-8", 500, 50, 0, 0)],
  sub: [assistant("a1", "msg_sub", "claude-haiku-4-5", 90, 9, 0, 0)],
};

type Step = {
  id: string;
  /** 会話履歴のファイル名 (拡張子なし)。`sub` は sidecar のみを書き足す。 */
  transcript: keyof typeof LINES;
  /** 何行まで書いてから撃つか。 */
  lines: number;
  /** 末尾の改行を落として「書きかけの行」を作るか。 */
  partial?: boolean;
  event: "PreToolUse" | "PostToolUse";
  tool?: { name: string; input: unknown };
  /** sub-agent の sidecar (`<transcript>/subagent-<agentId>.jsonl` 相当) を置くか。 */
  subagent?: { agentId: string; agentType?: string; lines: number };
  /** 封筒そのものを差し替える (不正 JSON・transcript 欠落の検証)。 */
  rawInput?: string;
  /** ファイル名は `transcript` のまま、中身を別の素材で書く (切り詰め・回転の再現)。 */
  writeAs?: keyof typeof LINES;
  /** 会話履歴を消してから撃つ (読めない履歴の検証)。 */
  removeTranscript?: true;
};

const ENGINE_BASH = { name: "Bash", input: { command: "bun .claude/tools/aidlc-orchestrate.ts report --result completed" } };
const PLAIN_READ = { name: "Read", input: { file_path: "/tmp/x" } };

/** 1 本の記録の上で順番に撃つ手順。追記 → 再畳み込みと flush を含む。 */
const STEPS: Step[] = [
  { id: "post/holdback-first", transcript: "simple", lines: 1, event: "PostToolUse" },
  { id: "post/holdback-second", transcript: "simple", lines: 2, event: "PostToolUse" },
  { id: "post/no-new-bytes", transcript: "simple", lines: 2, event: "PostToolUse" },
  { id: "pre/seal-main", transcript: "simple", lines: 2, event: "PreToolUse", tool: PLAIN_READ },
  { id: "pre/flush-all", transcript: "simple", lines: 2, event: "PreToolUse", tool: ENGINE_BASH },
  { id: "post/split-partial-line", transcript: "split", lines: 2, partial: true, event: "PostToolUse" },
  { id: "post/split-complete", transcript: "split", lines: 3, event: "PostToolUse" },
  { id: "pre/split-flush", transcript: "split", lines: 3, event: "PreToolUse", tool: ENGINE_BASH },
  { id: "post/mixed-unknown-model", transcript: "mixed", lines: 4, event: "PostToolUse" },
  { id: "pre/mixed-flush", transcript: "mixed", lines: 4, event: "PreToolUse", tool: ENGINE_BASH },
  { id: "pre/bedrock-flush", transcript: "bedrock", lines: 3, event: "PreToolUse", tool: ENGINE_BASH },
  { id: "pre/subagent-holdback", transcript: "short", lines: 1, event: "PreToolUse", tool: PLAIN_READ, subagent: { agentId: "agent-one", agentType: "aidlc-developer-agent", lines: 1 } },
  { id: "pre/subagent-flush", transcript: "short", lines: 1, event: "PreToolUse", tool: ENGINE_BASH, subagent: { agentId: "agent-one", agentType: "aidlc-developer-agent", lines: 1 } },
  { id: "pre/subagent-without-sidecar", transcript: "short", lines: 1, event: "PreToolUse", tool: ENGINE_BASH, subagent: { agentId: "agent-two", lines: 1 } },
  { id: "post/malformed-envelope", transcript: "simple", lines: 2, event: "PostToolUse", rawInput: "{ not json" },
  { id: "post/no-transcript", transcript: "simple", lines: 2, event: "PostToolUse", rawInput: JSON.stringify({ session_id: SESSION, hook_event_name: "PostToolUse" }) },
  { id: "post/unrelated-tool", transcript: "simple", lines: 2, event: "PostToolUse", tool: { name: "Glob", input: { pattern: "**" } } },
];

/** 会話履歴を root へ書き、その絶対パスを返す。 */
function writeTranscript(root: string, step: Step): string {
  const path = join(root, `${step.transcript}.jsonl`);
  if (step.removeTranscript) {
    rmSync(path, { force: true });
    return path;
  }
  const body = LINES[step.writeAs ?? step.transcript].slice(0, step.lines);
  writeFileSync(path, body.join("\n") + (step.partial ? "" : "\n"), "utf-8");
  if (step.subagent) {
    // 本家 subagentDir: `<transcript>.jsonl` -> `<dir>/subagents/` ではなく
    // 同名ディレクトリを見るので、実装から実測した置き場をそのまま使う。
    const dir = subagentDir(path);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, `agent-${step.subagent.agentId}.jsonl`), LINES.sub.slice(0, step.subagent.lines).join("\n") + "\n", "utf-8");
    if (step.subagent.agentType !== undefined) {
      writeFileSync(join(dir, `agent-${step.subagent.agentId}.meta.json`), JSON.stringify({ agentType: step.subagent.agentType }) + "\n", "utf-8");
    }
  }
  return path;
}

/**
 * 固定本家 `tools/aidlc-usage.ts:489-493` の `subagentDir` と同じ置き場。
 * `<dir>/<session>.jsonl` に対する `<dir>/<session>/subagents/`。
 */
function subagentDir(mainTranscriptPath: string): string {
  return join(mainTranscriptPath.replace(/\.jsonl$/, ""), "subagents");
}

function envelope(step: Step, transcriptPath: string): string {
  if (step.rawInput !== undefined) return step.rawInput;
  return JSON.stringify({
    session_id: SESSION,
    hook_event_name: step.event,
    ...(step.tool ? { tool_name: step.tool.name, tool_input: step.tool.input } : {}),
    transcript_path: transcriptPath,
  });
}

/** 切り詰め・回転・履歴消失。cursor の巻き戻しと「読めないなら書かない」を採る。 */
const ROTATE_STEPS: Step[] = [
  { id: "flush/full", transcript: "simple", lines: 2, event: "PreToolUse", tool: ENGINE_BASH },
  { id: "flush/again-no-new-bytes", transcript: "simple", lines: 2, event: "PreToolUse", tool: ENGINE_BASH },
  { id: "flush/truncated", transcript: "simple", lines: 1, writeAs: "short", event: "PreToolUse", tool: ENGINE_BASH },
  { id: "flush/removed", transcript: "simple", lines: 1, removeTranscript: true, event: "PreToolUse", tool: ENGINE_BASH },
  { id: "flush/grown-again", transcript: "simple", lines: 2, event: "PreToolUse", tool: ENGINE_BASH },
];

type Scenario = { id: string; withIntent: boolean; disabled?: boolean; steps: Step[] };

const SCENARIOS: Scenario[] = [
  // 記録が生まれていない workspace — workflowKey は record: の退避形になる。
  { id: "bare", withIntent: false, steps: STEPS },
  // Brownfield bugfix の記録がある workspace — workflowKey は intent:<uuid>、
  // byStage は state ファイルの Current Stage で立つ。
  { id: "intent", withIntent: true, steps: STEPS },
  // 切り詰め・回転・履歴消失。
  { id: "rotate", withIntent: false, steps: ROTATE_STEPS },
  // 停止フラグ。producer は 1 バイトも書かない。
  { id: "disabled", withIntent: true, disabled: true, steps: [STEPS[1]] },
];

const observations: unknown[] = [];
for (const scenario of SCENARIOS) {
  const temporary = mkdtempSync(join(tmpdir(), "aidlc-fold-usage-"));
  const root = join(temporary, "workspace");
  try {
    cpSync(dist, root, { recursive: true });
    mkdirSync(join(root, "src"));
    writeFileSync(join(root, "src/lib.rs"), "pub fn smoke() {}\n");
    if (scenario.withIntent) {
      const setup = captureObservation({
        root,
        argv: [process.execPath, join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "fold-usage", "--arguments", "Verify the usage producer"],
        environment: { AIDLC_DISABLE_USAGE_TRACKING: "1" },
      });
      assert.equal(setup.output.exit_code, 0, setup.output.stderr);
    }
    for (const step of scenario.steps) {
      const transcriptPath = writeTranscript(root, step);
      const observed = captureObservation({
        root,
        argv: [process.execPath, join(root, ".claude/hooks/aidlc-fold-usage.ts")],
        stdin: envelope(step, transcriptPath),
        environment: scenario.disabled ? { AIDLC_DISABLE_USAGE_TRACKING: "1" } : { AIDLC_DISABLE_USAGE_TRACKING: "" },
      });
      assert.equal(observed.output.exit_code, 0, `${scenario.id}/${step.id}: ${observed.output.stderr}`);
      assert.equal(observed.output.stdout, "", `${scenario.id}/${step.id}: stdout は空である`);
      observations.push({ id: `${scenario.id}/${step.id}`, transcript_path: transcriptPath, ...observed });
    }
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

mkdirSync(dirname(destination), { recursive: true });
writeFileSync(
  destination,
  JSON.stringify(
    {
      source: UPSTREAM,
      capture_method:
        "固定配布全文検証後、停止フラグを立てずに実フックを別プロセスで呼ぶ。1 本の一時 workspace 上で追記 → 再畳み込み → flush を順に撃ち、各回の全ファイルを無加工で保存する。intent シナリオだけ intent-create を先に実行する (その 1 回だけ利用量を無効化し、台帳を汚さない)。",
      normalization: [],
      observations,
    },
    null,
    2,
  ) + "\n",
);
console.log(`wrote ${destination} (${observations.length} observations)`);
