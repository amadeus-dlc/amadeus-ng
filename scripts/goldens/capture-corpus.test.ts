import { test, expect } from "bun:test";
import { capture } from "./capture-supplemental";
import { mkdtempSync, rmSync, mkdirSync, writeFileSync } from "node:fs";
import { captureObservation } from "./capture-observation";
import { captureStage1 } from "./capture-stage1";
import { tmpdir } from "node:os";
import { join } from "node:path";

test("補完採取が逐次実行の生入力と全出力を保存する", () => {
  const root = mkdtempSync(join(tmpdir(), "corpus-test-"));
  try {
    const archive = Bun.spawnSync(["git", "-C", "vendor/aidlc-workflows", "archive", "a277af218f0df7f325d3b8be7b6d90fce2c5bd40", "dist/claude"]);
    expect(archive.exitCode).toBe(0);
    expect(Bun.spawnSync(["tar", "-xf", "-", "-C", root], { stdin: archive.stdout }).exitCode).toBe(0);
    const result = capture(join(root, "dist/claude")) as any;
    expect(result.observations?.[0]?.input?.argv).toContain("intent-create");
    expect(result.observations[0].output.exit_code).toBe(0);
    expect(result.observations[0].output.stdout).toContain("golden");
    expect(result.observations[0].initial_files).toBeDefined();
    expect(result.observations[0].changed_files).toBeDefined();
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 30000);

test("設定変更前のファイルと変更後のバイトを保存する", () => {
  const root = mkdtempSync(join(tmpdir(), "observation-"));
  try {
    mkdirSync(join(root, ".claude"));
    writeFileSync(join(root, ".claude/settings.json"), "before");
    const result = captureObservation({ root, argv: [process.execPath, "-e", 'require("fs").writeFileSync(".claude/settings.json", "after")'] });
    expect(result.initial_files[".claude/settings.json"]).toBe("YmVmb3Jl");
    expect(result.changed_files[".claude/settings.json"]).toBe("YWZ0ZXI=");
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("保護を有効にした合成会話で計画承認後だけbeginできる", () => {
  const root = mkdtempSync(join(tmpdir(), "stage1-contract-"));
  try {
    const archive = Bun.spawnSync(["git", "-C", "vendor/aidlc-workflows", "archive", "a277af218f0df7f325d3b8be7b6d90fce2c5bd40", "dist/claude"]);
    expect(archive.exitCode).toBe(0);
    expect(Bun.spawnSync(["tar", "-xf", "-", "-C", root], { stdin: archive.stdout }).exitCode).toBe(0);
    const result = captureStage1(join(root, "dist/claude"));
    expect(result.observations[0].output.stdout).toContain("bugfix scope, 9 stages");
    expect(result.observations[0].output.stdout).toContain("Project type: Brownfield");
    expect(result.observations[0].output.stdout).toContain("First post-init stage: reverse-engineering");
    expect(result.observations.find(row => row.id === "plan/before-answer")?.output.exit_code).toBe(1);
    expect(result.observations.find(row => row.id === "plan/approved-begin")?.output.exit_code).toBe(0);
    expect(result.observations.find(row => row.id === "plan/wrong-session")?.output.exit_code).toBe(1);
    expect(result.observations.find(row => row.id === "plan/changed-content")?.output.exit_code).toBe(1);
    expect(result.observations.find(row => row.id === "plan/stale-receipt")?.output.exit_code).toBe(1);
    expect(result.observations.find(row => row.id === "plan-hook/before-answer")?.output.exit_code).toBe(2);
    expect(result.observations.find(row => row.id === "plan-hook/approved-write")?.output.exit_code).toBe(0);
    expect(result.observations.find(row => row.id === "plan-hook/wrong-target")?.output.exit_code).toBe(2);
    expect(result.observations.find(row => row.id === "summary/answer")?.output.exit_code).toBe(0);
    expect(result.observations.find(row => row.id === "review/completed")?.output.exit_code).toBe(0);
    expect(result.observations.find(row => row.id === "review-hook/frozen-write")?.output.exit_code).toBe(2);
    expect(result.observations.find(row => row.id === "link/architect")?.output.exit_code).toBe(0);
    expect(JSON.parse(result.observations.find(row => row.id === "report/approved")?.output.stdout ?? "null")?.kind).not.toBe("error");
    expect(result.observations.find(row => row.id === "report/approved")?.output.exit_code).toBe(0);
    for (const id of ["hook/deliver-stage-rules", "hook/reviewer-scope", "hook/log-subagent", "hook/sync-workflow-state", "hook/rebuild-stage-graph", "hook/validate-state", "hook/session-end"]) expect(result.observations.find(row => row.id === id)?.output.exit_code).toBe(0);
    for (const id of ["question/answer", "review-brief/summary", "review-brief/review", "review-brief/context", "project-description/read"]) expect(result.observations.find(row => row.id === id)?.output.exit_code).toBe(0);
    expect(result.observations.find(row => row.id === "hook/fold-usage-disabled")?.output.exit_code).toBe(0);
    expect(result.observations.find(row => row.id === "hook/fold-usage-disabled")?.changed_files).toEqual({});
    for (const id of ["review-brief/summary", "review-brief/review", "review-brief/context", "project-description/read", "testing-posture/render", "testing-posture/fingerprint"]) expect(result.observations.find(row => row.id === id)?.changed_files).toEqual({});
  } finally { rmSync(root, { recursive: true, force: true }); }
}, 30000);

for (const example of [
  { name: "標準入力・標準エラー・非ゼロ終了", code: 'process.stdout.write(require("fs").readFileSync(0));process.stderr.write("error\\n");process.exit(7)', input: "入力\n", status: 7, stdout: "入力\n", stderr: "error\n" },
  { name: "signal終了", code: 'process.kill(process.pid,"SIGTERM")', signal: "SIGTERM", status: null },
  { name: "親の秘密と保護無効化を継承しない", code: 'process.stdout.write(String(process.env.AIDLC_SKIP_ARTIFACT_GUARD)+":"+String(process.env.AWS_SECRET_ACCESS_KEY))', stdout: "undefined:undefined", status: 0 },
  { name: "バイナリ標準出力", code: 'process.stdout.write(Buffer.from([255,0,65]))', bytes: "/wBB", status: 0 },
  { name: "開始不能を成功に丸めない", code: "", executable: "/no/such/capture-executable", status: null, error: true },
]) {
  test(example.name, () => {
    const root = mkdtempSync(join(tmpdir(), "observation-edge-"));
    const previous = process.env.AIDLC_SKIP_ARTIFACT_GUARD;
    process.env.AIDLC_SKIP_ARTIFACT_GUARD = "1";
    try {
      const observation = captureObservation({ root, argv: [example.executable ?? process.execPath, "-e", example.code], stdin: example.input });
      expect(observation.output.exit_code).toBe(example.status);
      if (example.stdout !== undefined) expect(observation.output.stdout).toBe(example.stdout);
      if (example.stderr) expect(observation.output.stderr).toBe(example.stderr);
      if (example.signal) expect(observation.output.signal).toBe(example.signal);
      if (example.bytes) expect(observation.output.stdout_base64).toBe(example.bytes);
      if (example.error) expect(observation.output.error).toContain("ENOENT");
    } finally {
      if (previous === undefined) delete process.env.AIDLC_SKIP_ARTIFACT_GUARD;
      else process.env.AIDLC_SKIP_ARTIFACT_GUARD = previous;
      rmSync(root, { recursive: true, force: true });
    }
  });
}
