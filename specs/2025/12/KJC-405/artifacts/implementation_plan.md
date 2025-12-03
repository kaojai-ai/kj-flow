# KJC-405: Auto create branch on `kj spec`

## Goal Description
Implement automatic branch creation when running `kj spec <ticket_number>`.
- If on a default branch (`main`, `master`, `dev`), automatically create and checkout a new branch named `<ticket_number>`.
- If on a non-default branch and it doesn't match `<ticket_number>`, prompt the user to confirm if they want to fork a new branch from the current one.

## Proposed Changes

### [Utils]
#### [MODIFY] [git.ts](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/src/utils/git.ts)
- Add `createBranch(branchName: string): Promise<void>`
- Add `checkoutBranch(branchName: string): Promise<void>`

### [Commands]
#### [MODIFY] [spec.ts](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/src/commands/spec.ts)
- Import git utilities.
- Add logic to check current branch and handle branch creation/switching.
- Use `readline` for user confirmation prompt.

## Verification Plan

### Manual Verification
1.  **Default Branch Test**:
    - Checkout `main` (or `master`/`dev`).
    - Run `kj spec TEST-001`.
    - Verify that branch `TEST-001` is created and checked out.
    - Verify spec file is created.

2.  **Same Branch Test**:
    - Stay on `TEST-001`.
    - Run `kj spec TEST-001`.
    - Verify no prompt and no branch change.

3.  **Non-Default Branch Test (Yes)**:
    - Stay on `TEST-001`.
    - Run `kj spec TEST-002`.
    - Verify prompt appears: "You are on branch 'TEST-001'. Do you want to create a new branch 'TEST-002' from here? (y/N)"
    - Input `y`.
    - Verify branch `TEST-002` is created (from `TEST-001`) and checked out.

4.  **Non-Default Branch Test (No)**:
    - Stay on `TEST-002`.
    - Run `kj spec TEST-003`.
    - Verify prompt appears.
    - Input `n`.
    - Verify still on `TEST-002`.
