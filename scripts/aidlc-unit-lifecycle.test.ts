import { afterAll, describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { AuditShardEvent } from "../.codex/tools/aidlc-lib";

const project = mkdtempSync(join(tmpdir(), "aidlc-unit-lifecycle-test-"));
afterAll(() => rmSync(project, { recursive: true, force: true }));

/** シャード内の位置と時刻を別々に指定し、監査の入力順依存を再現する。 */
function row(event: string, timestamp: string, shard: string, shardIndex: number, pos: number, extra = ""): AuditShardEvent {
  return { event, timestamp, shard, shardIndex, pos,
    block: `**Event**: ${event}\n**Timestamp**: ${timestamp}\n**Stage**: functional-design\n${extra}` };
}

for (const harness of [".codex", ".claude", ".kimi-code"]) {
  const lib = await import(`../${harness}/tools/aidlc-lib.ts`);
  const floor = (rows: AuditShardEvent[]) => lib.latestMainWorkflowStageRunFloorForProject(
    project, "functional-design", true, undefined, rows,
  );
  describe(`${harness}: 複数監査シャードの完了判定`, () => {
    const old = row("STAGE_JUMPED", "2026-08-22T09:31:03Z", "b.md", 1, 0);
    const revised = row("GATE_REJECTED", "2026-09-05T06:37:27Z", "a.md", 0, 0);
    const currentFloor = "GATE_REJECTED:2026-09-05T06:37:27Z#1";

    test("入力順にかかわらず最新の作業回を選び、入力を変更しない", () => {
      for (const rows of [[old, revised], [revised, old]]) {
        const original = [...rows];
        expect(floor(rows)).toBe(currentFloor);
        expect(rows).toEqual(original);
      }
    });

    test("同一シャードの同秒イベントは位置順で数える", () => {
      const earlier = row("GATE_REJECTED", revised.timestamp, "a.md", 0, 1);
      const later = row("GATE_REJECTED", revised.timestamp, "a.md", 0, 2);
      expect(floor([later, old, earlier])).toBe("GATE_REJECTED:2026-09-05T06:37:27Z#2");
    });

    test("別シャードの同秒境界は順序を推測せず曖昧と判定する", () => {
      const tied = row("STAGE_JUMPED", revised.timestamp, "b.md", 1, 2);
      const result = floor([revised, old, tied]);
      expect(result).toStartWith("AMBIGUOUS:2026-09-05T06:37:27Z#");
      expect(floor([tied, old, revised])).toBe(result);
    });

    test("新しい作業回の完了を認識し、古い作業回の完了を持ち越さない", () => {
      const started = row("UNIT_STARTED", "2026-09-05T06:38:00Z", "a.md", 0, 1,
        `**Unit**: u9-canon-docs\n**Run floor**: ${currentFloor}\n`);
      const completed = row("UNIT_COMPLETED", "2026-09-05T06:42:05Z", "a.md", 0, 2,
        `**Unit**: u9-canon-docs\n**Run floor**: ${currentFloor}\n`);
      const stale = row("UNIT_COMPLETED", "2026-08-23T00:00:00Z", "b.md", 1, 1,
        "**Unit**: stale-unit\n**Run floor**: STAGE_JUMPED:2026-08-22T09:31:03Z#1\n");
      const snapshot = lib.unitLifecycleSnapshot(project, "functional-design",
        [revised, started, completed, old, stale], "- **Construction Iteration**: unit-major\n");
      expect([...snapshot.receipts]).toEqual(["u9-canon-docs"]);
      expect(snapshot.checkpoint).toBeNull();
    });
  });
}
