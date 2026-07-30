# Security policy

## Supported versions

Security fixes are applied to the latest release and the `main` branch.

## Reporting a vulnerability

Do not open a public issue for a vulnerability that could expose credentials,
environment files, repository access, local processes, or destructive Git
behavior.

Use GitHub's private vulnerability reporting for this repository. If private
reporting is unavailable, contact KaoJai through
[kaojai.ai](https://kaojai.ai) and include:

- affected version and platform;
- reproduction steps using disposable repositories;
- expected and actual behavior; and
- potential impact.

Never include real tokens, `.env` contents, customer data, or private remote
URLs in a report.

## Security model

KJ Flow executes local Git commands and user-selected development commands with
the current user's permissions. It does not sandbox those commands. Only use it
with repositories and commands you trust.

Task state and copied local environment files are permission-restricted, but
development processes can still write secrets to their own output. Treat task
logs as sensitive.
