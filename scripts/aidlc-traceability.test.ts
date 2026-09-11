import { afterEach, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

const project = resolve(import.meta.dir, "..");
const scratch: string[] = [];
afterEach(() => { for (const path of scratch.splice(0)) rmSync(path, { recursive: true, force: true }); });

/** 実際のCLI検査が読む独立した作業記録を作る。 */
function fixture(withStories = false) {
  const root = mkdtempSync(join(tmpdir(), "aidlc-traceability-"));
  scratch.push(root);
  const record = "aidlc/spaces/default/intents/test";
  const put = (path: string, body: string) => {
    const target = join(root, path);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, body);
    return target;
  };
  put("aidlc/spaces/default/intents/active-intent", "test\n");
  put(`${record}/aidlc-state.md`, "# State\n");
  put(`${record}/inception/requirements-analysis/requirements.md`, "# 要求\n\n- FR1 開始\n- FR1.1 拒否\n- FR2 完了\n");
  if (withStories) put(`${record}/inception/user-stories/stories.md`, "# シナリオ\n\n## US1.1 開始\n\n## US1.2 完了\n");
  const unitDir = `${record}/inception/units-generation`;
  put(`${unitDir}/unit-of-work.md`, "| Unit ID | Directory |\n|---|---|\n| U1 | u1-start |\n| U2 | u2-finish |\n");
  put(`${unitDir}/unit-of-work-dependency.md`, "# 依存\n\n```yaml\nunits:\n  - name: u1-start\n    kind: service\n    depends_on: []\n  - name: u2-finish\n    kind: service\n    depends_on: [u1-start]\n```\n");
  const ids = withStories ? ["US1.1", "US1.2"] : ["FR1", "FR1.1", "FR2"];
  const target = (index: number) => index === ids.length - 1 ? "U2" : "U1";
  const map = (rows = ids) => put(`${unitDir}/unit-of-work-story-map.md`,
    "| Requirement ID | 内容 | Primary Unit ID |\n|---|---|---|\n" +
    rows.map(id => `| ${id} | 対象の振る舞い | ${target(ids.indexOf(id))} |`).join("\n") + "\n");
  map();
  const data = { stage: "units-generation", upstream_ids: ids,
    coverage: ids.map((id, index) => ({ id, status: "OK", target: target(index) })) };
  const trace = () => put(`${unitDir}/traceability.json`, JSON.stringify(data));
  return { root, ids, map, data, trace, put, record, unitDir };
}

function codeFixture() {
  const f = fixture();
  f.put(`${f.record}/inception/requirements-analysis/requirements.md`, "# 要求\n\nFR1 開始\nFR1.1 拒否\nFR2 完了\nNFR1 共通品質\n");
  f.put(`${f.unitDir}/unit-of-work-story-map.md`, "| Requirement ID | 内容 | Primary Unit ID | Directory | 支援単位 |\n|---|---|---|---|---|\n| FR1 | U1という文字があるだけでは割当にしない | U2 | u2-finish | U1: 採取 |\n| FR1.1 | 下位要求 | U1 | u1-start | — |\n| FR2 | 完了 | U2 | u2-finish | — |\n");
  f.put("src/implementation.ts", "export const implemented = true;\n");
  const data = { stage: "code-generation", unit: "u1-start", upstream_ids: ["FR1", "FR1.1", "NFR1"],
    coverage: ["FR1", "FR1.1", "NFR1"].map(id => ({ id, status: "OK", target: "src/implementation.ts" })) };
  const trace = () => f.put(`${f.record}/construction/${data.unit}/code-generation/traceability.json`, JSON.stringify(data));
  return { ...f, data, trace };
}

/** 終了コードだけでなく、検査CLIが返すJSONの判定を確認する。 */
function run(harness: string, f: { root: string; trace: () => string }) {
  const result = Bun.spawnSync([process.execPath, join(project, harness, "tools/aidlc-sensor-traceability.ts"), "--output-path", f.trace()], {
    cwd: f.root,
    env: { ...process.env, AIDLC_PROJECT_DIR: f.root, AIDLC_HARNESS_DIR: harness,
      AIDLC_HARNESS_NAME: harness === ".kimi-code" ? "kimi" : harness.slice(1) },
  });
  expect(result.exitCode).toBe(0);
  return JSON.parse(result.stdout.toString());
}

for (const harness of [".codex", ".claude", ".kimi-code"]) {
  describe(`${harness}: 要求から作業単位への直接対応`, () => {
    test("コード生成では主担当と支援の割当から対象UnitのFRだけを導く", () => {
      const result = run(harness, codeFixture());
      expect(result.pass).toBe(true);
      expect(result.missing_from_upstream_ids).toEqual([]);
    });
    test("コード生成で未割当FRを足しても担当FRを宣言から消しても拒否する", () => {
      const f = codeFixture();
      f.data.upstream_ids.push("FR2");
      f.data.coverage.push({ id: "FR2", status: "OK", target: "src/implementation.ts" });
      const extra = run(harness, f);
      expect(extra.pass).toBe(false);
      expect(extra.invalid_entries.some((message: string) => message.includes("FR2"))).toBe(true);
      f.data.upstream_ids = ["NFR1"];
      f.data.coverage = [{ id: "NFR1", status: "OK", target: "src/implementation.ts" }];
      const missing = run(harness, f);
      expect(missing.pass).toBe(false);
      expect(missing.missing_from_upstream_ids).toEqual(["FR1", "FR1.1"]);
    });
    test("zero-Unitのコード生成は要求全体を検査する", () => {
      const f = codeFixture();
      rmSync(join(f.root, f.unitDir), { recursive: true });
      f.data.upstream_ids.push("FR2");
      f.data.coverage.push({ id: "FR2", status: "OK", target: "src/implementation.ts" });
      const input = { ...f, trace: () => {
        const { unit, ...data } = f.data;
        return f.put(`${f.record}/construction/code-generation/traceability.json`, JSON.stringify(data));
      } };
      expect(run(harness, input).pass).toBe(true);
      f.data.upstream_ids = f.data.upstream_ids.filter(id => id !== "FR2");
      f.data.coverage = f.data.coverage.filter(entry => entry.id !== "FR2");
      expect(run(harness, input).missing_from_upstream_ids).toContain("FR2");
    });
    test("他Unitと欠落した割当表も権威ある入力で検査する", () => {
      const f = codeFixture();
      f.data.unit = "u2-finish";
      f.data.upstream_ids = ["FR1", "FR2", "NFR1"];
      f.data.coverage = f.data.upstream_ids.map(id => ({ id, status: "OK", target: "src/implementation.ts" }));
      expect(run(harness, f).pass).toBe(true);
      f.data.unit = "u9-undeclared";
      expect(run(harness, f).pass).toBe(false);
      f.data.unit = "u2-finish";
      rmSync(join(f.root, f.unitDir, "unit-of-work-story-map.md"));
      expect(run(harness, f).pass).toBe(false);
    });
    test("Unitの宣言がなければディレクトリ名だけで担当範囲を認めない", () => {
      const f = codeFixture();
      rmSync(join(f.root, f.unitDir, "unit-of-work-dependency.md"));
      expect(run(harness, f).pass).toBe(false);
    });
    test("説明欄にUnit名を書くだけではFRの割当を増やせない", () => {
      const f = codeFixture();
      f.put(`${f.unitDir}/unit-of-work-story-map.md`, "| Requirement ID | 内容 | Primary Unit ID | Directory | 支援単位 |\n|---|---|---|---|---|\n| FR1 | U1との関係を検討 | U2 | u2-finish | — |\n| FR1.1 | 下位要求 | U1 | u1-start | — |\n| FR2 | 完了 | U2 | u2-finish | — |\n");
      const result = run(harness, f);
      expect(result.pass).toBe(false);
      expect(result.invalid_entries.some((message: string) => message.includes("FR1:"))).toBe(true);
    });
    test("既存USのコード生成ではUnitに割り当てたACを引き続き検査する", () => {
      const f = codeFixture();
      f.put(`${f.record}/inception/user-stories/stories.md`, "# Stories\n\nUS1.1 開始\nAC1.1.1 開始できる\nUS1.2 完了\nAC1.2.1 完了できる\n");
      f.put(`${f.unitDir}/unit-of-work-story-map.md`, "| Story ID | Primary Unit ID |\n|---|---|\n| US1.1 | U1 |\n| US1.2 | U2 |\n");
      f.data.upstream_ids = ["AC1.1.1"];
      f.data.coverage = [{ id: "AC1.1.1", status: "OK", target: "src/implementation.ts" }];
      expect(run(harness, f).pass).toBe(true);
      f.data.upstream_ids = ["AC1.2.1"];
      f.data.coverage = [{ id: "AC1.2.1", status: "OK", target: "src/implementation.ts" }];
      expect(run(harness, f).missing_from_upstream_ids).toContain("AC1.1.1");
    });
    test("シナリオ資料がなくてもFRと下位要求を表から照合する", () => {
      const result = run(harness, fixture());
      expect(result.reason).toBeUndefined();
      expect(result.pass).toBe(true);
      expect(result.findings_count).toBe(0);
    });
    test("FRの割当漏れと誤った対象単位を見逃さない", () => {
      const f = fixture();
      f.map(f.ids.slice(0, -1));
      const missing = run(harness, f);
      expect(missing.pass).toBe(false);
      expect(missing.gaps).toContain("FR2");
      f.map();
      f.data.coverage[0].target = "U2";
      const wrong = run(harness, f);
      expect(wrong.pass).toBe(false);
      expect(wrong.invalid_targets).toContain('FR1: target "U2" is not mapped in unit-of-work-story-map.md');
    });
    test("シナリオ資料がある場合はUSを引き続き照合する", () => {
      const f = fixture(true);
      expect(run(harness, f).pass).toBe(true);
      f.map(f.ids.slice(0, -1));
      const missing = run(harness, f);
      expect(missing.pass).toBe(false);
      expect(missing.gaps).toContain("US1.2");
    });
  });
}
