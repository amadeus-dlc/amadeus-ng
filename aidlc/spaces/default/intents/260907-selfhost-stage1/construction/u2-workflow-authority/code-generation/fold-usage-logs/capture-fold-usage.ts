// 本家 2.7.1 (固定コミット a277af21) の aidlc-fold-usage.ts と、本 build の
// `aidlc hook fold-usage` へ **同じ入力** を与え、生成物を突き合わせる採取器。
//
//   bun capture-fold-usage.ts <固定コミットのdist/claude> <本buildのバイナリ> <出力json>
//
// 一時 workspace (mkdtemp) だけを触る。本リポジトリの記録・監査・状態・memory・
// 受領証・usage-ledger は読み書きしない。

import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [distClaude, buildBinary, outPath] = process.argv.slice(2);
if (!distClaude || !buildBinary || !outPath) {
  console.error(
    "usage: bun capture-fold-usage.ts <dist/claude> <aidlc binary> <out.json>",
  );
  process.exit(2);
}

const SESSION = "11111111-2222-4333-8444-555555555555";

function assistantLine(
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
    uuid,
    timestamp: "2026-09-10T00:00:00Z",
    type: "assistant",
    ...extra,
    message: {
      id: msgId,
      role: "assistant",
      model,
      usage: {
        input_tokens: input,
        output_tokens: output,
        cache_read_input_tokens: cacheRead,
        cache_creation_input_tokens: cacheWrite,
        cache_creation: {
          ephemeral_5m_input_tokens: cacheWrite,
          ephemeral_1h_input_tokens: 0,
        },
      },
    },
  });
}

// 採取に使う会話履歴。分割行 (同一 message.id が連続する新形式) と、
// 未知モデル、id 無し行を混ぜてある。
const TRANSCRIPTS: Record<string, string[]> = {
  // 1 呼出し 1 行だけの素直な形。
  simple: [
    assistantLine("u1", "msg_1", "claude-opus-4-8", 100, 20, 0, 0),
    assistantLine("u2", "msg_2", "claude-opus-4-8", 200, 40, 0, 0),
  ],
  // 新形式の分割行: 先頭行が usage=0、最終行に実測値。
  split: [
    assistantLine("s1", "msg_a", "claude-sonnet-5", 0, 0, 0, 0),
    assistantLine("s2", "msg_a", "claude-sonnet-5", 1000, 300, 5000, 2000),
    assistantLine("s3", "msg_b", "claude-haiku-4-5-20251001", 10, 5, 0, 0),
  ],
  // 未知世代 + id 無し行 + 非 assistant 行 + 壊れた行。
  mixed: [
    assistantLine("m1", "msg_x", "claude-opus-9-9", 70, 7, 0, 0),
    JSON.stringify({
      uuid: "m2",
      timestamp: "2026-09-10T00:00:01Z",
      type: "user",
      message: { role: "user", content: "hi" },
    }),
    "{ this is not json",
    assistantLine("m3", "", "claude-sonnet-5", 11, 3, 0, 0),
  ],
};

type Envelope = {
  session_id: string;
  hook_event_name: string;
  tool_name?: string;
  tool_input?: unknown;
  transcript_path: string;
};

function envelope(
  event: string,
  transcriptPath: string,
  tool?: { name: string; input: unknown },
): Envelope {
  return {
    session_id: SESSION,
    hook_event_name: event,
    ...(tool ? { tool_name: tool.name, tool_input: tool.input } : {}),
    transcript_path: transcriptPath,
  };
}

type Step = {
  label: string;
  transcript: keyof typeof TRANSCRIPTS;
  // 会話履歴のうち何行までを書いてから撃つか (追記の途中を再現する)。
  lines: number;
  // 末尾の改行を落として「書きかけの行」を作るか。
  partial?: boolean;
  event: string;
  tool?: { name: string; input: unknown };
};

// 撃つ順番。追記 → 再畳み込みを含める。
const STEPS: Step[] = [
  { label: "post/first", transcript: "simple", lines: 1, event: "PostToolUse" },
  { label: "post/second", transcript: "simple", lines: 2, event: "PostToolUse" },
  {
    label: "pre/seal-main",
    transcript: "simple",
    lines: 2,
    event: "PreToolUse",
    tool: { name: "Read", input: { file_path: "/tmp/x" } },
  },
  {
    label: "pre/flush-all",
    transcript: "simple",
    lines: 2,
    event: "PreToolUse",
    tool: {
      name: "Bash",
      input: {
        command:
          "bun .claude/tools/aidlc-orchestrate.ts report --result completed",
      },
    },
  },
  { label: "post/split-partial", transcript: "split", lines: 2, partial: true, event: "PostToolUse" },
  { label: "post/split-complete", transcript: "split", lines: 3, event: "PostToolUse" },
  {
    label: "pre/split-flush",
    transcript: "split",
    lines: 3,
    event: "PreToolUse",
    tool: {
      name: "Bash",
      input: { command: "bun .claude/tools/aidlc-orchestrate.ts report --result completed" },
    },
  },
  { label: "post/mixed", transcript: "mixed", lines: 4, event: "PostToolUse" },
  {
    label: "pre/mixed-flush",
    transcript: "mixed",
    lines: 4,
    event: "PreToolUse",
    tool: {
      name: "Bash",
      input: { command: "bun .claude/tools/aidlc-orchestrate.ts report --result completed" },
    },
  },
];

function writeTranscript(root: string, step: Step): string {
  const path = join(root, `${step.transcript}.jsonl`);
  const lines = TRANSCRIPTS[step.transcript].slice(0, step.lines);
  const text = lines.join("\n") + (step.partial ? "" : "\n");
  writeFileSync(path, text, "utf-8");
  return path;
}

function readJson(path: string): unknown {
  try {
    return JSON.parse(readFileSync(path, "utf-8"));
  } catch {
    return null;
  }
}

function readText(path: string): string | null {
  try {
    return readFileSync(path, "utf-8");
  } catch {
    return null;
  }
}

// 一時 workspace の絶対パスを伏せ、比較できる形にする。
function normalize(value: unknown, root: string): unknown {
  const text = JSON.stringify(value);
  if (text === undefined) return value;
  return JSON.parse(text.split(root).join("<ROOT>"));
}

type Run = {
  implementation: string;
  steps: {
    label: string;
    exit: number | null;
    stdout: string;
    stderr: string;
    ledger: unknown;
    ledger_bytes: number | null;
    ledger_trailing_newline: boolean | null;
    session_transcript: string | null;
    current_transcript: string | null;
    current_session: string | null;
  }[];
};

function runOne(implementation: "upstream" | "build"): Run {
  const root = mkdtempSync(join(tmpdir(), `aidlc-fold-${implementation}-`));
  mkdirSync(join(root, "aidlc"), { recursive: true });
  if (implementation === "upstream") {
    cpSync(join(distClaude, ".claude"), join(root, ".claude"), {
      recursive: true,
    });
  }
  const sessions = join(root, "aidlc", ".aidlc-sessions");
  const out: Run["steps"] = [];
  for (const step of STEPS) {
    const transcriptPath = writeTranscript(root, step);
    const input = JSON.stringify(
      envelope(step.event, transcriptPath, step.tool),
    );
    const result =
      implementation === "upstream"
        ? spawnSync("bun", [join(root, ".claude", "hooks", "aidlc-fold-usage.ts")], {
            input,
            cwd: root,
            encoding: "utf-8",
            env: { ...process.env, AIDLC_DISABLE_USAGE_TRACKING: "" },
          })
        : spawnSync(buildBinary, ["hook", "fold-usage"], {
            input,
            cwd: root,
            encoding: "utf-8",
            env: { ...process.env, AIDLC_DISABLE_USAGE_TRACKING: "" },
          });
    const ledgerPath = join(sessions, "usage-ledger.json");
    const ledgerRaw = readText(ledgerPath);
    out.push({
      label: step.label,
      exit: result.status,
      stdout: result.stdout ?? "",
      stderr: result.stderr ?? "",
      ledger: normalize(readJson(ledgerPath), root),
      ledger_bytes: ledgerRaw === null ? null : Buffer.byteLength(ledgerRaw),
      ledger_trailing_newline:
        ledgerRaw === null ? null : ledgerRaw.endsWith("\n"),
      session_transcript: readText(join(sessions, `${SESSION}.transcript`))?.replace(
        root,
        "<ROOT>",
      ) ?? null,
      current_transcript:
        readText(join(sessions, "current.transcript"))?.replace(root, "<ROOT>") ??
        null,
      current_session: readText(join(sessions, ".current-session")),
    });
  }
  return { implementation, steps: out };
}

// 停止フラグを立てた 1 発。producer は何も書かない契約である。
function runDisabled(implementation: "upstream" | "build"): unknown {
  const root = mkdtempSync(join(tmpdir(), `aidlc-fold-off-${implementation}-`));
  mkdirSync(join(root, "aidlc"), { recursive: true });
  if (implementation === "upstream") {
    cpSync(join(distClaude, ".claude"), join(root, ".claude"), { recursive: true });
  }
  const step = STEPS[0];
  const transcriptPath = writeTranscript(root, step);
  const input = JSON.stringify(envelope("PostToolUse", transcriptPath));
  const result =
    implementation === "upstream"
      ? spawnSync("bun", [join(root, ".claude", "hooks", "aidlc-fold-usage.ts")], {
          input,
          cwd: root,
          encoding: "utf-8",
          env: { ...process.env, AIDLC_DISABLE_USAGE_TRACKING: "1" },
        })
      : spawnSync(buildBinary, ["hook", "fold-usage"], {
          input,
          cwd: root,
          encoding: "utf-8",
          env: { ...process.env, AIDLC_DISABLE_USAGE_TRACKING: "1" },
        });
  const sessions = join(root, "aidlc", ".aidlc-sessions");
  return {
    implementation,
    exit: result.status,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
    sessions_dir_exists: existsSync(sessions),
    ledger_exists: existsSync(join(sessions, "usage-ledger.json")),
    current_transcript_exists: existsSync(join(sessions, "current.transcript")),
  };
}

const capture = {
  captured_at: new Date().toISOString(),
  upstream_source: distClaude,
  build_binary: buildBinary,
  transcripts: TRANSCRIPTS,
  runs: [runOne("upstream"), runOne("build")],
  disabled: [runDisabled("upstream"), runDisabled("build")],
};

writeFileSync(outPath, `${JSON.stringify(capture, null, 2)}\n`, "utf-8");
console.log(`wrote ${outPath}`);
