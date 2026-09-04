# amadeus-ng

Rust reimplementation of AI-DLC Workflows. Specs live in `docs/specs/`
(00-policy is the top document), ADRs in `docs/adr/`, Quint models in `formal/`.

Owner-ruled coding rules shared by humans and all agents live in
`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` (one rule per file; see its README). Read them before
writing code — they are enforced by review and, where marked, by
`cargo lint`.

### Fable 5 Delegation Policy

The policy text lives in the memory layer — `aidlc/spaces/default/memory/project.md`
§ Mandated (the line that begins with "ALWAYS 実装は委譲し"). That file is the single
source of truth because it is what reaches delegated agents; this file is not.
