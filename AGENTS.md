# KJ Flow

- Keep the CLI local-first, deterministic, and safe for humans and agents.
- Keep core behavior generic. Product and repository-specific commands belong in local `repos.toml` configuration, never source defaults.
- Keep KaoJai/KJ branding in project metadata and documentation without embedding private infrastructure or business contracts.
- Never infer a remote task base from a canonical checkout's current `HEAD`; resolve and fetch `origin/HEAD`.
- Never delete worktrees, branches, or task state that the current command did not create.
- Keep `.kj` state private. Never persist command arguments, print environment contents, or expose credentials in remote URLs.
- `--json` stdout is a stable machine interface. Send human diagnostics to stderr.
- Write integration tests with temporary Git repositories. Never use a developer's real repositories as fixtures.
- Support macOS and Linux explicitly. Guard or document platform-specific behavior.
- Run `make check` before committing behavior changes.
