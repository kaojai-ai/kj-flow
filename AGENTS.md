# KJ Flow agent guidance

KJ Flow is a local-first Rust CLI for isolated human-agent development across
multiple Git repositories. `README.md` is the source of truth for user-facing
behavior; keep this file limited to implementation constraints.

## Product boundary

- Keep the core generic. Repository-specific commands belong in
  `<workspace>/.kj/repos.toml` or `~/.config/kj-flow/repos.toml`.
- Keep KaoJai/KJ branding, but never embed private infrastructure, credentials,
  customer data, or business contracts.
- Do not add a hosted service, account requirement, telemetry, or network API
  to the local workflow without an explicit product decision.

## Safety contracts

- Resolve remote task bases from fetched `origin/HEAD`, never from the canonical
  checkout's current branch.
- Cleanup may remove only paths recorded for the selected task. Failed creation
  may roll back only resources created by that invocation. Retain local
  branches after a successful finish.
- Do not weaken stopped-process, clean-worktree, or pushed-commit checks.
- Keep `.kj` directories private. Never persist command arguments or environment
  contents, follow env-file symlinks, or expose credentials in output.
- `--json` stdout is a stable machine contract. Keep it to one success or error
  envelope and send diagnostics to stderr.
- Treat manifest schema, filesystem layout, environment-copy rules, branch
  naming, and JSON shapes as contracts. Breaking changes must bump the schema
  or version and be documented.

## Working style

- Prefer the smallest direct change in the existing modules. Avoid speculative
  abstractions, compatibility layers, and business-specific defaults.
- Use temporary Git repositories for integration tests; never use a developer's
  real workspace as a fixture.
- Support macOS and Linux. Guard or document platform-specific behavior.
- Match verification to risk:
  - docs and examples: `git diff --check`;
  - focused Rust behavior: `cargo fmt --check` and the relevant test;
  - Git, worktree, process, env, state, JSON-contract, dependency, or release
    changes: `make check`.
