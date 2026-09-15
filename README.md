# KJ Flow

**One development task. Multiple repositories. Isolated humans and AI agents.**

KJ Flow is an open-source CLI by [KaoJai](https://kaojai.ai) for teams where
developers and coding agents work on the same software at the same time. The
`kj` command creates a clean Git worktree per repository, assigns isolated
ports, prepares local environment files, runs development processes, and opens
an agent with only the task's repositories writable.

KJ Flow is local-first. It does not require a hosted service, account, database,
or API token.

## Why

Parallel development becomes unreliable when people and agents reuse one
checkout:

- branches and uncommitted changes get mixed;
- worktrees are created from the wrong repository;
- local servers fight over ports;
- fresh worktrees cannot start without local environment files; and
- cleanup removes work that was not created by the current task.

KJ Flow gives each task an explicit boundary:

```text
workspace/
├── frontend/                         # canonical Git repository
├── api/                              # canonical Git repository
├── contracts/                        # canonical Git repository
└── worktrees/
    └── feature-auth/
        ├── .kj/                      # private task state, PIDs and logs
        ├── frontend/                 # branch codex/feature-auth
        ├── api/                      # branch codex/feature-auth
        └── contracts/                # branch codex/feature-auth
```

The workspace itself is a non-Git directory. Every immediate child Git
repository keeps its own history, remote, branch and pull request.

## Platform

KJ Flow currently supports macOS and Linux. It requires Git. Rust 1.85 or
newer is required only to build from source. [OpenAI Codex](https://github.com/openai/codex)
is optional and required only for `kj task codex`.

## Install

### Quick install

Install the latest published binary without `sudo`:

```bash
curl -fsSL https://github.com/kaojai-ai/kj-flow/releases/latest/download/kj-installer.sh | sh
```

The installer supports macOS and Linux on ARM64 and x86-64. It downloads the
matching archive, verifies its SHA-256 checksum, and installs `kj` into
`~/.local/bin` by default. Set `KJ_INSTALL_DIR` to install elsewhere.

If `~/.local/bin` is not on your `PATH`, add it using your shell's normal
profile file, then open a new terminal.

### Direct download

Download the archive matching your platform and `kj-checksums.txt` from the
[latest release](https://github.com/kaojai-ai/kj-flow/releases/latest). Verify
the archive before extracting it:

```bash
shasum -a 256 -c kj-checksums.txt --ignore-missing
tar -xzf kj-<target>.tar.gz
install -m 755 kj ~/.local/bin/kj
```

On Linux systems without `shasum`, use `sha256sum -c kj-checksums.txt`.

### Build from source

```bash
git clone https://github.com/kaojai-ai/kj-flow.git
cd kj-flow
make install-local
```

`make install-local` builds the current checkout and installs it into
`~/.local/bin`.

After any installation method:

```bash
kj init --workspace /path/to/workspace
kj --json doctor
```

Configuration is stored at `~/.config/kj-flow/config.toml`. Override its
location with `KJ_CONFIG_PATH`, or override the workspace for one command with
`KJ_WORKSPACE_ROOT`.

## Human-agent workflow

```bash
# Discover repositories.
kj repo list

# Create one isolated task spanning every repository it needs.
kj task create feature-auth --repo frontend --repo api --repo contracts
kj task show feature-auth

# Open Codex with frontend as the primary directory and the others writable.
kj task codex feature-auth --primary frontend

# Run services with task-reserved ports and private logs.
kj task start feature-auth frontend
kj task logs feature-auth frontend --follow
kj task stop feature-auth frontend

# Preview safety checks, then remove only this task's worktrees.
kj task finish feature-auth
kj task finish feature-auth --apply
```

`task create` performs complete preflight before creating worktrees. Remote
repositories branch from fetched `origin/HEAD`; local-only repositories branch
from their current default branch. A failed multi-repository creation rolls
back only worktrees and branches created by that invocation.

`task finish --apply` refuses to remove worktrees while processes are recorded,
files are dirty, or task commits have not been pushed. Local branches are
retained.

## Commands

```text
kj init --workspace <path>
kj --json doctor
kj repo list

kj task create <task-id> --repo <repo> [--repo <repo>...]
kj task list
kj task show <task-id>
kj task codex <task-id> [--primary <repo>]

kj task start <task-id> <repo> [--foreground] [-- <command>]
kj task logs <task-id> <repo> [--follow]
kj task stop <task-id> [<repo>]
kj task env sync <task-id> <repo> [--apply]

kj task finish <task-id>
kj task finish <task-id> --apply
```

## Configuration

The generated user config contains the workspace, port range and branch prefix:

```toml
workspace_root = "/path/to/workspace"
port_start = 41000
port_end = 49999
branch_prefix = "codex"
```

Use `kj init --workspace /path/to/workspace --worktree-root /path/to/checkouts`
to keep task worktrees elsewhere. A relative `worktree_root` stays inside the
workspace; omitting it uses `<workspace>/worktrees`.

KJ Flow infers `pnpm dev` only when a repository has a `package.json`
containing a `scripts.dev` entry. Repository commands that need explicit port
arguments can be configured in either:

1. `<workspace>/.kj/repos.toml` for workspace defaults; or
2. `~/.config/kj-flow/repos.toml` for user overrides.

User overrides win:

```toml
[repos.frontend]
dev_command = ["pnpm", "exec", "vite", "--port", "{port}"]
```

An explicit command after `--` always wins:

```bash
kj task start feature-auth api -- cargo run
```

## Environment, processes and security

Task creation optionally copies safe regular `.env`, `.env.local`, and
`.env.development` files when they exist in canonical repositories. Symlinks,
production/AWS filenames, tracked destinations and every other filename are
skipped. Copies use mode `0600`. `task env sync --apply` explicitly refreshes
safe untracked destinations.

KJ Flow injects `PORT`, `KJ_TASK_ID`, and `KJ_REPO` into development processes
without modifying copied env files. Background processes run in their own
process groups and write combined output to
`worktrees/<task-id>/.kj/logs/<repo>.log`.

The `.kj` task-state directory uses mode `0700`; manifests, runtime state, PID
files, locks and logs use `0600`. HTTP(S) Git credentials are removed from
repository-list output.

Application output can still contain secrets. Treat task logs as sensitive,
and never pass credentials directly in command-line arguments.

## JSON contract

`--json` emits one stable envelope to stdout:

```json
{"ok":true,"data":{"healthy":true,"checks":[]}}
```

Errors are non-zero:

```json
{"ok":false,"error":{"code":"command_failed","message":"..."}}
```

Interactive modes (`task codex`, `task start --foreground`, and
`task logs --follow`) reject `--json`.

## Development

```bash
make check
```

The test suite uses disposable local Git repositories. It never uses a
developer's real repositories as fixtures.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

## License

Apache License 2.0. See [LICENSE](LICENSE).

KJ Flow is built and maintained by KaoJai.
