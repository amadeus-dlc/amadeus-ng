<!--
  .claude/rules/aidlc.md — the AIDLC method @-import stub (NOT a copy).

  The AIDLC method (the layered practice files: org/team/project + phase rules)
  is authored ONCE at the workspace root under aidlc/spaces/default/memory/ —
  the single hand-editable source of truth, identical on every harness. This
  file is a REFERENCE, not a copy: it pulls that method into Claude's ambient
  context via @-imports so casual chat (outside an AIDLC stage) sees the
  standing practices. AIDLC's own stage resolver reads the same tree directly
  (it never needs this stub).

  Claude @-imports name an EXPLICIT file each (no glob support — verified
  against code.claude.com/docs memory.md), resolve relative paths from THIS
  file's location, and follow a nested chain up to four hops. From
  .claude/rules/ the workspace root is ../../, so the method tree is
  ../../aidlc/spaces/default/memory/. The @-lines below ship pointed at the
  always-present `default` space; this file stays committed (it carries this
  load-bearing wiring beyond the pointer), and `/aidlc space <name>` re-points
  these @-lines IN PLACE so the next turn's ambient context follows the active
  space. At `default` the re-point is a byte-identical no-op. (AIDLC's own stage
  resolver follows the active-space cursor directly and never needs this stub.)

  Edit the METHOD at aidlc/spaces/default/memory/*, never here. If a new method
  file is added there, add a matching @-line below.
-->

@../../aidlc/spaces/default/memory/org.md
@../../aidlc/spaces/default/memory/team.md
@../../aidlc/spaces/default/memory/project.md
@../../aidlc/spaces/default/memory/phases/ideation.md
@../../aidlc/spaces/default/memory/phases/inception.md
@../../aidlc/spaces/default/memory/phases/construction.md
@../../aidlc/spaces/default/memory/phases/operation.md

<!--
  Coding-rules canon — ALWAYS machine-loaded (owner ruling 2026-08-30).

  project.md mandates reading aidlc/spaces/default/knowledge/aidlc-shared/
  coding-rules/ before writing any code, spec, or review. Relying on the agent
  to re-read it each time proved fragile (a freshly ruled rule was missed in
  PR #61 and needed a fix-up commit), so the canon is pulled into ambient
  context here, one @-line per rule file (no glob support). When a rule file
  is added or renamed there, add or update the matching @-line below.

  Deliberately NOT imported: CONSISTENCY-AUDIT-2026-08-24.md — a point-in-time
  audit snapshot, not a rule.
-->

@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/abstract-data-type.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/aggregate-commands.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/aggregate-references.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/command-query-separation.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/cqrs-boundaries.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-equality.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-persistence-neutrality.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/domain-services.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/factory-naming.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/field-visibility.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/good-examples.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/infrastructure-layer.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/interior-mutability.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/no-backward-compatibility.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/tell-dont-ask.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/ubiquitous-language.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/upstream-contracts.md
@../../aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md
