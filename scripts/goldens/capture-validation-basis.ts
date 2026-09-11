#!/usr/bin/env bun
/** 完了時Validation Basisを固定元の公開関数で実測する。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";
import { UPSTREAM, digest, verifySource } from "./upstream-source";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
const parent = mkdtempSync(join(tmpdir(), "aidlc-validation-basis-"));
try {
  const code = join(parent, "source");
  cpSync(dist, code, { recursive: true });
  const sourceFile = join(code, ".claude/tools/aidlc-validity.ts");
  const source = await import(pathToFileURL(sourceFile).href);
  const record = "aidlc/spaces/default/intents/260909-validation";
  const producer = { slug: "producer", phase: "inception", execution: "ALWAYS", produces: ["input"], optional_produces: ["optional-input"] };
  const base = { slug: "consumer", phase: "inception", execution: "ALWAYS", produces: ["output"], optional_produces: ["optional-output"], consumes: [{ artifact: "input", required: true }] };
  const defaultState = "- **Scope**: bugfix\n- **Project Type**: Brownfield\n";
  const cases: Array<{ id: string; stage?: Record<string, unknown>; stages?: unknown[]; state?: string; files?: Record<string, string>; directories?: string[] }> = [
    { id: "required-missing" },
    { id: "required-present", files: { [`${record}/inception/producer/input.md`]: "# 入力\r\n本文\n", [`${record}/inception/consumer/output.md`]: "# output\n" } },
    { id: "optional-output-present", files: { [`${record}/inception/consumer/optional-output.md`]: "optional\n" } },
    { id: "optional-input-absent", stage: { ...base, consumes: [{ artifact: "optional-input", required: false }] } },
    { id: "optional-input-present", stage: { ...base, consumes: [{ artifact: "optional-input", required: false }] }, files: { [`${record}/inception/producer/optional-input.md`]: "present\n" } },
    { id: "optional-owner-absent", stage: { ...base, consumes: [{ artifact: "unowned", required: false }] } },
    { id: "required-owner-absent", stage: { ...base, consumes: [{ artifact: "unowned", required: true }] } },
    { id: "owner-ambiguous", stages: [producer, { ...producer, slug: "duplicate" }, base] },
    { id: "conditional-brownfield", stage: { ...base, consumes: [{ artifact: "input", required: true, conditional_on: "brownfield" }] } },
    { id: "conditional-greenfield", stage: { ...base, consumes: [{ artifact: "input", required: true, conditional_on: "brownfield" }] }, state: "- **Scope**: bugfix\n- **Project Type**: Greenfield\n" },
    { id: "conditional-unknown", stage: { ...base, consumes: [{ artifact: "input", required: true, conditional_on: "brownfield" }] }, state: "- **Scope**: bugfix\n" },
    { id: "directory-is-not-a-file", directories: [`${record}/inception/consumer/output.md`] },
    { id: "known-filenames", stage: { ...base, produces: ["build-test-results", "load-test-results", "traceability"] }, files: { [`${record}/inception/consumer/test-results.md`]: "results\n", [`${record}/inception/consumer/traceability.json`]: "{}\n" } },
    { id: "ordering-edge-excluded", stage: { ...base, requires_stage: ["producer"] } },
    { id: "graph-contract-change", stage: { ...base, execution: "CONDITIONAL", condition: "brownfield" } },
    { id: "codekb-input", stage: { ...base, consumes: [{ artifact: "source-map", required: true }] }, stages: [{ slug: "reverse-engineering", phase: "inception", produces: ["source-map"] }, { ...base, consumes: [{ artifact: "source-map", required: true }] }], files: { "aidlc/spaces/default/codekb/workspace/source-map.md": "# Source map\n" } },
  ];
  const graph = JSON.parse(readFileSync(join(code, ".claude/tools/data/stage-graph.json"), "utf8"));
  for (const slug of ["reverse-engineering", "requirements-analysis", "code-generation", "build-and-test", "deployment-pipeline", "deployment-execution"]) {
    const stage = graph.find((stage: { slug: string }) => stage.slug === slug);
    assert(stage, slug);
    cases.push({ id: `bugfix/${slug}`, stage, stages: graph });
  }
  const observations = cases.map((test, index) => {
    const root = join(parent, `workspace-${index}`);
    mkdirSync(root);
    mkdirSync(join(root, "aidlc/spaces/default/intents"), { recursive: true });
    writeFileSync(join(root, "aidlc/active-space"), "default\n");
    const files = test.files ?? {};
    for (const [name, content] of Object.entries(files)) { mkdirSync(dirname(join(root, name)), { recursive: true }); writeFileSync(join(root, name), content); }
    for (const name of test.directories ?? []) mkdirSync(join(root, name), { recursive: true });
    const stage = test.stage ?? base;
    const stages = test.stages ?? [producer, stage];
    const state = test.state ?? defaultState;
    const options = { resolution: { recordPath: join(root, record), codekbRepos: ["workspace"] } };
    const result = source.stageValidationAuditFields(root, stage, state, stages, options);
    assert.equal(Object.keys(result).length, 1);
    return { id: test.id, input: { stage, stages, state, record, codekb_repos: ["workspace"], files, directories: test.directories ?? [] }, fields: result };
  });
  const basis = (id: string) => observations.find(test => test.id === id)!.fields["Validation Basis"];
  assert.equal(basis("required-missing"), basis("ordering-edge-excluded"));
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, source_sha256: digest(readFileSync(sourceFile)), normalization: [], capture_method: "固定元stageValidationAuditFieldsを実ファイル・明示recordPath/codekbReposで実行。scope bugfixのUnitなし解決と固定配布6工程を含む。監査への永続化やCLI全体の実測ではない。", observations }, null, 2) + "\n");
} finally { rmSync(parent, { recursive: true, force: true }); }
