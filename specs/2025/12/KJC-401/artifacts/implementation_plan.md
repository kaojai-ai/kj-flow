# Implementation Plan - KJC-401

Remove OpenAI dependencies and update `kj pr create` to use `metadata.json` for PR details.

## User Review Required
- **Breaking Change**: `kj pr create` will no longer generate PR descriptions using OpenAI. It requires `metadata.json` to be present in the ticket's artifacts folder.
- **New Feature**: Added `--dry-run` flag to `kj pr create` for testing purposes.

## Proposed Changes

### Package.json
#### [MODIFY] [package.json](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/package.json)
- Remove `openai` dependency.

### Source Code
#### [DELETE] [src/utils/openai.ts](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/src/utils/openai.ts)
- Remove this file as it's no longer needed.

#### [MODIFY] [src/commands/pr.ts](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/src/commands/pr.ts)
- Remove `openai` import.
- Remove `getDiff` usage for PR generation.
- Implement logic to locate `specs/YYYY/MM/[ticket]/artifacts/metadata.json`.
- Read `pr_title` and `pr_summary` from metadata.
- Construct PR body using the specified format:
  ```markdown
  # AI Summary
  {pr_summary}

  # User Prompt
  {user_prompt}

  # Artifacts
  - [implementation_plan.md](link)
  - [walkthrough.md](link)
  ```
- Add `--dry-run` option to print the command instead of executing it.

## Verification Plan

### Automated Tests
- None existing for this CLI.

### Manual Verification
1.  **Setup**:
    - Create `specs/2025/12/KJC-401/artifacts/metadata.json` with dummy data.
    - Ensure `specs/2025/12/KJC-401/KJC-401.md` exists.
2.  **Execution**:
    - Run `pnpm build`.
    - Run `node dist/index.js pr create KJC-401 --dry-run`.
3.  **Validation**:
    - Check output to ensure it finds the metadata and spec file.
    - Verify the generated PR title and body match the expected format.
    - Verify `gh` command is constructed correctly.
