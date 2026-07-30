# Contributing to KJ Flow

Thank you for helping improve human-agent software development workflows.

## Before opening a change

- Search existing issues and pull requests.
- Keep core behavior generic. Team-specific repository commands belong in
  `repos.toml`, not source defaults.
- Open an issue before changing the task manifest, JSON envelope, filesystem
  layout, branch naming, or environment-copying policy.
- Never include credentials, production data, private repository URLs, or local
  environment-file contents.

## Development

KJ Flow supports macOS and Linux and requires Rust 1.85 or newer.

```bash
git clone https://github.com/kaojai-ai/kj-flow.git
cd kj-flow
make check
```

Tests must use temporary local Git repositories. Do not point tests at a real
workspace.

## Pull requests

Keep pull requests focused and include:

- the user-visible behavior;
- contract impact (`breaking` or `non-breaking`);
- the smallest relevant verification; and
- security implications for Git operations, processes, env files, logs, ports,
  or cleanup.

By submitting a contribution, you agree that it is licensed under the Apache
License 2.0.
