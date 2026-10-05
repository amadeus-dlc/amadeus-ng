# amadeus-ng

Rust reimplementation of AI-DLC Workflows. Quint models live in `formal/`.
Project documents are kept AI-DLC-natively under `aidlc/spaces/default/`
(`memory/` for rules, `knowledge/` for shared documents, `intents/` for workflow
records); there is no hand-maintained `docs/` tree (removed 2026-09-07).

### How development is run (owner ruling 2026-10-05)

Development of this repository does not use the AI-DLC workflow: do not start
`/aidlc`, create intents, or write under `aidlc/spaces/*/intents/`. The documents
under `aidlc/spaces/default/knowledge/` (coding rules, plans) are still referenced.
Implementation runs through takt with the project workflow `amadeus-ng`
(`.takt/workflows/amadeus-ng.yaml`), launched from Orca orchestration in its own
worktree via `scripts/run-takt.sh --config-dir <account> --trust-workspace --
--pipeline -w amadeus-ng ...`. When a task settles, release the worker and remove
its worktree.

Owner-ruled coding rules shared by humans and all agents live in
`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` (one rule per file; see its README). Read them before
writing code — they are enforced by review and, where marked, by
`cargo lint`.

### Fable 5 Delegation Policy

The policy text belongs in the memory layer — `aidlc/spaces/default/memory/project.md`
§ Mandated — because that file is what reaches delegated agents; this file is not.
The memory layer was reset to the shipped template on 2026-09-07, so the line is
absent until it is re-affirmed through practices-discovery.
