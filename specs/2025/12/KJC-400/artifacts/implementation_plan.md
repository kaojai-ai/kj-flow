# KJC-400 Implementation Plan - Add `kj ping` command

## Goal Description
Add a temporary CLI command `kj ping` that simply prints `pong` to the console. This is to verify the CLI structure and command registration.

## Proposed Changes

### CLI Commands

#### [NEW] [ping.ts](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/src/commands/ping.ts)
- Create a new command file `src/commands/ping.ts`.
- Implement a `ping` command using `commander` that outputs "pong".

#### [MODIFY] [index.ts](file:///Users/chaintng/Projects/kaojai-ai/kj-flow/src/index.ts)
- Import `pingCommand` from `./commands/ping`.
- Register `pingCommand` to the main program.

## Verification Plan

### Automated Tests
- Run `npm run build` to ensure the project builds correctly.
- Run `node dist/index.js ping` and verify the output is "pong".

### Manual Verification
- Run `kj ping` (if linked) or `node dist/index.js ping` and check for "pong".
