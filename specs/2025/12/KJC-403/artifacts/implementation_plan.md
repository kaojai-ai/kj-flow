# Implementation Plan - KJC-403: CI/CD and README Update

## Goal Description
Implement a CI/CD workflow to automatically build and publish the `kj-flow` CLI to the npm registry. Update the README to provide clear instructions on how to start using and testing the CLI, following the workflow described in the spec.

## User Review Required
> [!IMPORTANT]
> The CI/CD workflow requires an `NPM_TOKEN` secret to be set in the GitHub repository settings.

## Proposed Changes

### CI/CD
#### [NEW] [.github/workflows/publish.yml](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/.github/workflows/publish.yml)
- Create a GitHub Action workflow that triggers on release creation.
- Steps:
    - Checkout code.
    - Setup Node.js (and pnpm).
    - Install dependencies.
    - Build the project (`pnpm build`).
    - Publish to npm (`pnpm publish --no-git-checks`).

### Configuration
#### [MODIFY] [package.json](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/package.json)
- Add `files` array to include `dist`, `README.md`, `LICENSE`.
- Ensure `publishConfig` is set to `access: public`.

### Documentation
#### [MODIFY] [README.md](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/README.md)
- Update "Prerequisites" section.
- Update "Workflow" section to match the spec (KJC-403).
- Add instructions on how to use `kj spec`, `kj pr create`, etc.

## Verification Plan

### Automated Tests
- None for this infrastructure task.

### Manual Verification
- **Dry Run Publish**: Run `npm publish --dry-run` locally to verify that the correct files are included in the package.
- **README Review**: Preview `README.md` to ensure formatting and instructions are clear.
- **Workflow Lint**: Visually inspect the YAML file for syntax errors.
