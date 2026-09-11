#!/usr/bin/env bun
/** 固定本家のTesting Posture契約の解決結果。 */
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const dist = process.argv[2], destination = process.argv[3];
assert(dist && destination);
verifySource(dist);
const upstream = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-testing-posture.ts")).href);
const inputs = [{ id: "empty-bugfix-minimal", sections: {}, options: { scope: "bugfix", testStrategy: "minimal", projectType: "greenfield" } }];
inputs.push({ id: "empty-bugfix-brownfield", sections: {}, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } });
inputs.push({ id: "team-explicit-tdd", sections: { team: "- **Methodology**: tdd\n- **Ordering**: Red → Green → Refactor.\n" }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } });
inputs.push({ id: "contradicting-project-method", sections: { team: "- Methodology: tdd", project: "- Methodology: test-after" }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } });
for (const [id, scope, strategy] of [["feature-standard", "feature", "standard"], ["bugfix-comprehensive", "bugfix", "comprehensive"], ["classic-standard", "classic", "standard"]]) {
  inputs.push({ id, sections: {}, options: { scope, testStrategy: strategy, projectType: "brownfield" } });
}
const section = (path: string) => readFileSync(path, "utf8").split(/^## Testing Posture[ \t]*$/m)[1]?.split(/^## /m)[0] ?? "";
const originalOrg = section(join(dist, "aidlc/spaces/default/memory/org.md"));
inputs.push({ id: "distribution-default-org", sections: { org: originalOrg }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } });
inputs.push({ id: "selfhost-team-tdd", sections: { org: originalOrg, team: section(join(import.meta.dir, "../../aidlc/spaces/default/memory/team.md")) }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } });
inputs.push({ id: "bare-bold-methodology", sections: { team: "**Methodology**: tdd" }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } });
for (const [id, team] of [
  ["commented-methodology", "<!--\n- Methodology: tdd\n-->\n"],
  ["fenced-methodology", "```md\n- Methodology: tdd\n```\n"],
  ["fenced-comment-literal", "```md\n<!--\n```\n- Methodology: tdd\n"],
  ["visible-notes-with-comment", "- Methodology: tdd <!-- note -->\nCoverage stays green.\n"],
]) { inputs.push({ id, sections: { team }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } }); }
for (const method of ["bdd", "atdd", "custom"]) { inputs.push({ id: `explicit-${method}`, sections: { team: `- Methodology: ${method}` }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } }); }
for (const [id, team] of [
  ["prose-tdd", "We use TDD for all behavior."], ["prose-bdd", "Use behaviour-driven scenarios."],
  ["prose-atdd", "Use ATDD for acceptance."], ["prose-test-after", "Implementation-first is affirmed."],
  ["ambiguous-prose", "Choose between TDD and BDD."],
  ["custom-specialization", "- Methodology: custom\n- Ordering: TDD for domain; BDD for endpoints."],
]) { inputs.push({ id, sections: { team }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } }); }
inputs.push(
  { id: "custom-preserves-team", sections: { team: "- Methodology: tdd", project: "- Methodology: custom\n- Ordering: TDD for domain; BDD for endpoints." }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } },
  { id: "custom-discards-team", sections: { team: "- Methodology: tdd", project: "- Methodology: custom\n- Ordering: BDD for endpoints." }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } },
  { id: "mixed-ordering-before-first-class", sections: { team: "Tests first, then tests first-class, then tests after implementation." }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } },
);
inputs.push({ id: "mixed-refactor-after-first-class", sections: { team: "tdd\nTests first, then first-class; refactor after green." }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } });
for (const [id, team] of [["long-s-is-not-test-driven", "teſt-driven"], ["long-s-is-not-mixed-tests", "Teſts first; teſts after implementation."]]) { inputs.push({ id, sections: { team }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } }); }
for (const count of [39, 40, 41]) { inputs.push({ id: `utf16-ordering-${count}-astral`, sections: { team: `tdd\nTests ${"🚀".repeat(count)} first; refactor after green.` }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } }); }
for (const count of [19, 20]) { inputs.push({ id: `utf16-before-implementation-${count}-astral`, sections: { team: `tdd\nTests before ${"🚀".repeat(count)} implementation; refactor after green.` }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } }); }
inputs.push({ id: "custom-ecmascript-whitespace", sections: { team: "custom ordering\nKeep\u0085each\uFEFFword." }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } });
for (const [id, team] of [["notes-preserve-nel", "\u0085We use TDD.\u0085"], ["methodology-trims-bom", "- Methodology: \uFEFFtdd\uFEFF"]]) { inputs.push({ id, sections: { team }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } }); }
for (const [id, team] of [["methodology-nel-with-markers", "Methodology: **\u0085tdd**"], ["ordering-keeps-nel", "Methodology: tdd\nOrdering: \u0085Preserve this order.\u0085"], ["ordering-trims-bom", "Methodology: tdd\nOrdering: \uFEFFPreserve this order.\uFEFF"]]) { inputs.push({ id, sections: { team }, options: { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" } }); }
const observations = inputs.map(input => {
  try { return { ...input, contract: upstream.resolveTestingPostureFromSections(input.sections, input.options), error: null }; }
  catch (error) { return { ...input, contract: null, error: String((error as Error).message) }; }
});
const lib = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-lib.ts")).href);
for (const [id, team] of [
  ["document-bom-heading-suffix", "## Testing Posture\uFEFF\n- Methodology: tdd\n"],
  ["document-nel-heading-suffix", "## Testing Posture\u0085\n- Methodology: tdd\n"],
  ["document-posture-section", "# Team\n\n## Unrelated\n- Methodology: test-after\n\n## Testing Posture\n\n- Methodology: tdd\n\n## Deployment\n- Methodology: test-after\n"],
  ["document-hidden-headings", "<!--\n## Testing Posture\n- Methodology: bdd\n-->\n```md\n## Testing Posture\n- Methodology: atdd\n```\n## Testing Posture\n- Methodology: tdd\n\n## Next\n"],
]) {
  const root = mkdtempSync(join(tmpdir(), "aidlc-posture-documents-"));
  const documents = { team };
  const options = { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" };
  try {
    const memory = join(root, "aidlc/spaces/default/memory");
    mkdirSync(memory, { recursive: true });
    writeFileSync(join(memory, "team.md"), team);
    const state = lib.stateFilePath(root);
    mkdirSync(dirname(state), { recursive: true });
    writeFileSync(state, "- **Scope**: bugfix\n- **Test Strategy**: Minimal\n- **Project Type**: Brownfield\n");
    observations.push({ id, documents, options, contract: upstream.resolveTestingPosture(root), error: null });
  } finally { rmSync(root, { recursive: true, force: true }); }
}
for (const observation of observations) { if (observation.contract) observation.rendered = upstream.renderTestingContract(observation.contract); }
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_command: [process.execPath, import.meta.path, dist, destination], tool_versions: { bun: Bun.version }, normalization: "none", observations }, null, 2) + "\n");
