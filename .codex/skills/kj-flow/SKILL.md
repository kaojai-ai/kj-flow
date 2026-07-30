---
name: kj-flow
description: Coordinate isolated multi-repository development tasks between humans and coding agents with the installed kj CLI.
---

# KJ Flow task workflow

Verify the command and workspace first:

```bash
command -v kj
kj --json doctor
kj --json repo list
```

Create one task containing every repository that must change:

```bash
kj task create feature-auth --repo frontend --repo api
kj task codex feature-auth --primary frontend
```

Run and inspect local services through the task so ports and processes remain isolated:

```bash
kj task start feature-auth frontend
kj task logs feature-auth frontend --follow
kj task stop feature-auth
```

Finish safely:

```bash
kj task finish feature-auth
kj task finish feature-auth --apply
```

Rules:

- Prefer `--json` for inspection and automation.
- Use the same task ID across repositories participating in one change.
- Do not create worktrees manually inside service repositories.
- Do not run raw development servers when port isolation matters; use `kj task start` or `kj task start ... --foreground`.
- `finish` is a preview unless `--apply` is supplied.
- Never force removal, delete branches, overwrite environment files, or bypass dirty/unpushed checks.
- There is no network API or raw request escape hatch; `kj` operates only on local Git repositories and processes.
