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
- **Work runs through takt, launched from Orca** (the owner's global orchestration
  rules apply). Record the work as an Orca Run and Task, create an Orca worktree
  for the task, and start takt in that worktree's terminal in pipeline mode:
  `scripts/run-takt.sh --config-dir <account> --trust-workspace -- --pipeline -w backend-cqrs --auto-pr -t "<brief>"`.
  Do not start worker agents (claude, codex, …) directly from Orca.
  `--trust-workspace` marks the new worktree as trusted for the takt account;
  without it takt's Claude ignores `permissions.allow` and fails.
- **Workflow:** start from takt's builtin `backend-cqrs`. Add a custom workflow
  under `.takt/workflows/` only when the builtin one is too heavy. Model choices
  follow `.takt/runtime.yaml`.
- **Every brief carries the project rules**: the coding rules (below), the checks
  CI runs (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo lint`, `cargo test --workspace`), and a write scope that does not
  overlap any other brief.
- **Close after each task.** Once its PR is merged and nothing is left
  uncommitted or unpushed, close the terminal and remove the worktree
  (`orca worktree rm`).
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
