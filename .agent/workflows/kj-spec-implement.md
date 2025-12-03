---
description: Implement based on spec of ticket-number, and create artifact info before finish.
---

User will provide the ticket-number
you will find the same ticket-number as a folder name in specs/YYYY/MM/[ticket-number]

Task 1)
Follow the spec, use that as a user prompt to implement what user request.
Don't introduce any new spec or functionality.

Task 2)
After implementation is done, Update (or created if not existed) 3 files under specs/YYYY/MM/[ticket-number]/artifacts
- implementation_plan.md
- walkthrough.md
- metadata.json contain `pr_title` and `pr_summary`
- use conventional commit as `pr_title` (e.g. <type>[optional scope]: <description>)
Before the end.
- `pr_summary` is a string in markdown format, prefer bullet styl
