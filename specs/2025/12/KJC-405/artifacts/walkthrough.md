# Walkthrough - KJC-405: Auto create branch on kj spec

I have implemented the automatic branch creation feature for the `kj spec` command.

## Changes

### `src/utils/git.ts`

- Added `createBranch` and `checkoutBranch` helper functions.

### `src/commands/spec.ts`

- Added logic to check the current branch.
- If on a default branch (`main`, `master`, `dev`), it automatically creates and checks out the new ticket branch.
- If on a non-default branch, it prompts the user for confirmation before creating the new branch.

## Verification Results

### Automated Tests
- Ran `npm run build` to ensure the project builds successfully.

### Manual Verification
- **Default Branch**: Verified that running `kj spec` from `main` creates and switches to the new branch.
- **Same Branch**: Verified that running `kj spec` on the same branch does not trigger a prompt or switch.
- **Non-Default Branch**: Verified that running `kj spec` from a different feature branch prompts the user and switches upon confirmation.
