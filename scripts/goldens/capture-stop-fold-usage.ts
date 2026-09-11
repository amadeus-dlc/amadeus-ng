#!/usr/bin/env bun
/**
 * 固定本家の Stop フック (`hooks/aidlc-continue-workflow.ts`) が行う利用量台帳の flush-all
 * (`aidlc-continue-workflow.ts:1328-1338` — `writeCurrentTranscriptPath` と
 * `foldTranscriptIntoLedger(..., "flush-all")`) を、**有効なまま**実プロセスで観測する。
 *
 * `capture-fold-usage.ts` が Pre/PostToolUse の producer を採ったのに対し、ここは turn-end の
 * producer を採る。停止フラグを立てない場面と立てた場面、記録の無い workspace、Codex 形式の
 * 会話履歴パスを 1 回ずつ撃ち、`aidlc/.aidlc-sessions/` 配下の実バイトを採る。
 *
 *   bun scripts/goldens/capture-stop-fold-usage.ts <verified-dist/claude> <output.json>
 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);

const SESSION = "session-stop";

/** 1 回の assistant 応答を表す JSONL 行 (`capture-fold-usage.ts` と同じ素材)。 */
function assistant(
  uuid: string,
  msgId: string,
  model: string,
  input: number,
  output: number,
  cacheRead: number,
  cacheWrite: number,
): string {
  return JSON.stringify({
    uuid, timestamp: "2026-09-10T00:00:00Z", type: "assistant",
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

/** main の会話履歴 3 行 (最後の群は Stop の flush-all で締まる)。 */
const MAIN: string[] = [
  assistant("u1", "msg_1", "claude-opus-4-8", 100, 20, 0, 0),
  assistant("u2", "msg_2", "claude-sonnet-5", 1000, 300, 5000, 2000),
  assistant("u3", "msg_3", "claude-opus-4-8", 200, 40, 0, 0),
];
/** sub-agent の sidecar 1 行 (flush-all は sub-agent の最後の群も締める)。 */
const SUB: string[] = [assistant("a1", "msg_sub", "claude-haiku-4-5", 90, 9, 0, 0)];

type Scenario = {
  id: string;
  /** Brownfield bugfix の記録を先に作るか (Stop は状態ファイルが無ければ折り畳まずに通す)。 */
  withIntent: boolean;
  /** 停止フラグを立てるか。 */
  disabled?: boolean;
  /** 会話履歴のファイル名。Codex の rollout 形は Claude 形式ではないので折り畳まれない。 */
  transcript: string;
  /** 会話履歴を書かずに撃つ (読めない履歴)。 */
  missingTranscript?: true;
};

const SCENARIOS: Scenario[] = [
  // 記録がある workspace — byStage は state ファイルの Current Stage、workflows は intent:<uuid>。
  { id: "intent", withIntent: true, transcript: "main.jsonl" },
  // 同じ workspace で停止フラグ。producer は 1 バイトも書かない。
  { id: "disabled", withIntent: true, disabled: true, transcript: "main.jsonl" },
  // 記録の無い workspace — 状態ファイルが無いので Stop は折り畳みに達しない。
  { id: "bare", withIntent: false, transcript: "main.jsonl" },
  // Codex の rollout 形のパス — Claude 形式でないので Stop は折り畳まない。
  { id: "codex-rollout", withIntent: true, transcript: "rollout-2026-09-10T00-00-00.jsonl" },
  // 会話履歴が読めない — ポインタは書くが、台帳は読めた分 (0 バイト) で書き直される。
  { id: "missing-transcript", withIntent: true, transcript: "gone.jsonl", missingTranscript: true },
];

/**
 * 固定本家 `tools/aidlc-usage.ts:489-493` の `subagentDir` と同じ置き場。
 * `<dir>/<session>.jsonl` に対する `<dir>/<session>/subagents/`。
 */
function subagentDir(mainTranscriptPath: string): string {
  return join(mainTranscriptPath.replace(/\.jsonl$/, ""), "subagents");
}

const observations: unknown[] = [];
for (const scenario of SCENARIOS) {
  const temporary = mkdtempSync(join(tmpdir(), "aidlc-stop-fold-usage-"));
  const root = join(temporary, "workspace");
  try {
    cpSync(dist, root, { recursive: true });
    mkdirSync(join(root, "src"));
    writeFileSync(join(root, "src/lib.rs"), "pub fn smoke() {}\n");
    if (scenario.withIntent) {
      const setup = captureObservation({
        root,
        argv: [process.execPath, join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "stop-fold", "--arguments", "Verify the turn-end usage producer"],
        environment: { AIDLC_DISABLE_USAGE_TRACKING: "1" },
      });
      assert.equal(setup.output.exit_code, 0, setup.output.stderr);
    }
    const transcriptPath = join(root, scenario.transcript);
    if (!scenario.missingTranscript) {
      writeFileSync(transcriptPath, MAIN.join("\n") + "\n", "utf-8");
      const dir = subagentDir(transcriptPath);
      mkdirSync(dir, { recursive: true });
      writeFileSync(join(dir, "agent-agent-one.jsonl"), SUB.join("\n") + "\n", "utf-8");
      writeFileSync(join(dir, "agent-agent-one.meta.json"), JSON.stringify({ agentType: "aidlc-developer-agent" }) + "\n", "utf-8");
    }
    const stdin = JSON.stringify({
      hook_event_name: "Stop",
      stop_hook_active: false,
      session_id: SESSION,
      transcript_path: transcriptPath,
    });
    const observed = captureObservation({
      root,
      argv: [process.execPath, join(root, ".claude/hooks/aidlc-continue-workflow.ts")],
      stdin,
      environment: scenario.disabled ? { AIDLC_DISABLE_USAGE_TRACKING: "1" } : { AIDLC_DISABLE_USAGE_TRACKING: "" },
    });
    assert.equal(observed.output.exit_code, 0, `${scenario.id}: ${observed.output.stderr}`);
    observations.push({ id: `stop/${scenario.id}`, transcript_path: transcriptPath, ...observed });
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
        "固定配布全文検証後、Stop フックを実プロセスで呼ぶ。intent シナリオは intent-create を先に実行し (その 1 回だけ利用量を無効化し、台帳を汚さない)、main の会話履歴 3 行と sub-agent の sidecar 1 行を置いてから 1 回だけ撃ち、各回の全ファイルを無加工で保存する。比較の対象は `aidlc/.aidlc-sessions/` 配下 (台帳と会話履歴ポインタ) と終了コードである。",
      normalization: [],
      observations,
    },
    null,
    2,
  ) + "\n",
);
console.log(`wrote ${destination} (${observations.length} observations)`);
