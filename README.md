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
- GitHub CLI authenticated: `gh auth login`

### Installation
```bash
npm i -g kj-flow
```

---

## License
MIT
