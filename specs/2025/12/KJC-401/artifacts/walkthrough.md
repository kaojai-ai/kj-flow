# Walkthrough - KJC-401 Remove OpenAI

I have removed the OpenAI dependency and updated the `kj pr create` command to use `metadata.json` for PR details.

## Changes

### Package.json
- Removed `openai` from dependencies.

### Source Code
- Deleted `src/utils/openai.ts`.
- Updated `src/commands/pr.ts`:
    - Removed OpenAI integration.
    - Added logic to read `metadata.json` from the ticket's artifacts folder.
    - Updated PR body generation to match the requested format.
    - Added `--dry-run` flag.

## Verification Results

### Automated Tests
- Ran `pnpm build` to ensure the project builds successfully.

### Manual Verification
- Created a dummy `metadata.json` in `specs/2025/12/KJC-401/artifacts/`.
- Ran `node dist/index.js pr create KJC-401 --dry-run`.
- Verified the output matches the expected format:
    - Correctly identified the spec file.
    - Correctly read the metadata.
    - Generated the correct PR title and body.

```
Creating PR for ticket: KJC-401...
Found spec file: /Users/chaintng/Projects/kaojai-ai/kj-flow/specs/2025/12/KJC-401/KJC-401.md
--- Dry Run ---
Title: feat: KJC-401 Remove OpenAI
Body:
# AI Summary
Removed OpenAI dependency and updated PR creation logic to use metadata.json.

# User Prompt
# KJC-401:
...

# Artifacts
- [implementation_plan.md](specs/2025/12/KJC-401/artifacts/implementation_plan.md)
- [walkthrough.md](specs/2025/12/KJC-401/artifacts/walkthrough.md)
```
