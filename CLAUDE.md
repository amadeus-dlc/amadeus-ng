# amadeus-ng

Rust reimplementation of AI-DLC Workflows. Quint models live in `formal/`.
Coding rules and design notes currently live under `aidlc/spaces/default/knowledge/`
(to be relocated when AI-DLC is uninstalled — see below).

### Development rules (owner ruling 2026-10-05)

- **No self-hosted development.** This repository is no longer developed with
  AI-DLC itself — self-hosting slowed development down instead of speeding it up.
  Do not run `/aidlc`, create intents, or write under `aidlc/spaces/*/intents/`.
- **AI-DLC is being uninstalled from this repository**, including the shell under
  `.claude/` (and `.codex/`, `.agents/`, `AGENTS.md`, `aidlc/`). Until that lands,
  the installed 2.8.2 shell under `.claude/` and `aidlc/spaces/default/memory/`
  are also read by Rust contract tests as the upstream reference, so they must be
  relocated into test fixtures before they are deleted — never delete them piecemeal.
- **Implement with takt**, using the project workflow `amadeus-ng`
  (`.takt/workflows/amadeus-ng.yaml`). If it falls short, extend that workflow
  rather than improvising another flow.
- **Launch takt from Orca orchestration** so the owner can see what is running:
  one supervised worker per task in its own worktree
  (`orca orchestration worker-start --worktree new-top-level ...`), which runs
  `scripts/run-takt.sh --config-dir <account> --trust-workspace -- --pipeline -w amadeus-ng --auto-pr ...`.
- **Discard after each task.** When the worker settles, release its session
  (`orca orchestration worker-release`) and remove its worktree (`orca worktree rm`).
- **Milestones are handed to Claude Code's `/goal`.** Keep the text within
  4,000 characters, and phrase every completion condition as something the
  session prints (the judge reads only the session output).

Owner-ruled coding rules shared by humans and all agents live in
`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` (one rule per file; see its README). Read them before
writing code — they are enforced by review and, where marked, by
`cargo lint`.

### Fable 5 Delegation Policy

The policy text belongs in the memory layer — `aidlc/spaces/default/memory/project.md`
§ Mandated — because that file is what reaches delegated agents; this file is not.
The memory layer was reset to the shipped template on 2026-09-07, so the line is
absent until it is re-affirmed through practices-discovery.
