# kj-flow 🌊
A unified CLI for the AI-driven SDLC, from prompt to deploy, preserving durable context for humans and agents. It establishes a repeatable, reviewable, and auditable workflow backbone.

## Scope Responsibilities
- Capture **prompts** + decisions
- Turn prompts into **specs** + acceptance criteria
- Generate **test plans** + scaffolds
- Create **PRs** with meaningful summaries
- Assist **review** (checklists, risk notes)
- Assist **deploy** (release notes, runbooks, automation hooks)

## Planned Commands
```bash
kj prompt "Add debounce batching to event processor"
kj spec
kj test plan
kj pr create
kj review prep
kj deploy plan
```

---

## Current Focus

### 👉 PR Creation with Context
The initial release of `kj-flow` focuses on generating a GitHub Pull Request that embeds critical workflow context, including:
- Original user prompt(s)
- IDE artifacts + conversation trace (plans, specs, test notes, etc.)
- Linear/Jira link (optional)
- AI-generated PR title + description based on `git diff` and artifacts

### Usage
```bash
kj pr create --ticket ABC-123
# or
kj pr create --ticket https://linear.app/TEAM/issue/ABC-123
```

### PR Bundle
`kj-flow` integrates a `.kj-flow/pr/` directory into your branch, linking it from the PR body. This bundle serves as an audit log and an AI-ready context feed for future tooling.

```
.kj-flow/pr/
  prompt.md
  conversation.jsonl
  artifacts/
  generated/
    pr_title.txt
    pr_body.md
  meta.json
```

---

## Install (dev preview)

### Prerequisites
- Node.js 24+
- Git
- GitHub CLI (`gh`) authenticated: `gh auth login`
- IDE configured with Antigravity (or similar AI assistant)

### Installation
```bash
npm i -g kj-flow
```

## Workflow

1. **Create Branch**: `git checkout -b <ticket_number>`
2. **Read Spec**: `kj spec <ticket_number>`
3. **Draft Prompt**: Write your user prompt in the spec file to keep history.
4. **Implement**: Use your AI assistant (e.g., via `/kj-spec-implement <ticket_number>`) to read the spec, implement changes, and create artifacts.
5. **Update Spec**: If requirements change, update the spec file and run `kj spec update <ticket_number>`.
6. **Create PR**: Run `kj pr create`. This will generate a PR with a description including the original prompt, AI implementation plan, and artifacts.
7. **Merge & Release**: Merge the PR and follow the release process.

---

## License
MIT
