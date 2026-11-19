# Reference Notes

Phase 0 study. References were looked for under `./reference/` and at the project root.

## Availability

| Project | Location checked | Status |
|---|---|---|
| deepseek-harness | `/Users/mac/Documents/PROJECTS/deepseek-harness`, `./reference/deepseek-harness` | EMPTY / MISSING (directory exists but contains no files) |
| codex | `./reference/codex`, `/Users/mac/.claude/skills/codex` (skill only, not source) | MISSING as source tree |
| jcode | `./reference/jcode` | MISSING |
| heartbit | `./reference/heartbit` | MISSING |
| rust-sdk | `./reference/rust-sdk` | MISSING |

No reference source trees were present on disk, so no file paths below are claims about inspected code. Each section records what would have been studied and the license status is "unknown - not inspected". Drop the reference repos into `./reference/<name>/` and re-run Phase 0 to fill these in.

## deepseek-harness

- Files read: none (empty directory).
- Agent loop: not observed. Expected to study: main turn loop, tool dispatch, streaming.
- Edit tool: not observed.
- Bash tool: not observed.
- Read tool: not observed.
- Permissions/approvals: not observed.
- Truncation: not observed.
- Error handling: not observed.
- Context management: not observed.
- Plugin/module system: not observed.
- License: unknown.

## codex

- Files read: none (only a skill wrapper at `/Users/mac/.claude/skills/codex`, not the Rust source).
- Agent loop: not observed.
- Edit/bash/read tools: not observed.
- Permissions/approvals: not observed.
- Truncation: not observed.
- Error handling: not observed.
- Context management: not observed.
- Plugin/module system: not observed.
- License: unknown (OpenAI codex is Apache-2.0 upstream; verify before adapting).

## jcode

- Files read: none.
- All sections: not observed.
- License: unknown.

## heartbit

- Files read: none.
- All sections: not observed.
- License: unknown.

## rust-sdk

- Files read: none (no MCP Rust SDK tree present).
- All sections: not observed. Expected to study MCP client/server, tool schema generation, transport.
- License: unknown (Anthropic rust-sdk is MIT/Apache-2.0 upstream; verify before adapting).

## What to avoid (pending real study)

- Do not copy any reference code until licenses are confirmed.
- Do not assume tool schemas or approval UX from memory; re-study once repos are present.
- Re-run Phase 0 after references are added; this document is a placeholder scaffold.
