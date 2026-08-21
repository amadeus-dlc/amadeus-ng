# amadeus-ng

Rust reimplementation of AI-DLC Workflows. Specs live in `docs/specs/`
(00-policy is the top document), ADRs in `docs/adr/`, Quint models in `formal/`.

### Fable 5 Delegation Policy

To avoid hitting the Fable 5 rate limit prematurely, reserve the main session for
requirements clarification, design, planning, audits, reviews, and final integration
decisions. During implementation, delegate well-scoped execution tasks to subagents
whenever the expected resource savings exceed the coordination overhead:

- Use Sonnet for routine implementation with clear boundaries.
- Use Opus for complex or high-risk implementation that requires stronger reasoning.
- Use Fable 5 directly for exceptionally difficult or tightly coupled work that
  cannot be delegated safely or efficiently. Keep small, well-scoped tasks in the
  main session when delegation overhead would exceed the expected resource savings.

Every delegation prompt must define the scope, owned files, acceptance criteria,
and verification steps. Assign non-overlapping write scopes. The Fable 5 main
session remains responsible for reviewing the complete diff, confirming final
verification, and deciding whether the integrated result is acceptable.
