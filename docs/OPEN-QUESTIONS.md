# Open Questions

Decisions required before Phase 1.

1. **Desktop framework**: Tauri vs egui/eframe. Tauri gives native OS webview + smaller binary but adds JS/TS build; egui is pure Rust. Recommend egui for Phase 1 simplicity; decide before scaffolding `kestrel-desktop`.
2. **MCP scope in Phase 1**: client-only, or also expose kestrel as an MCP server? Recommend client-only first.
3. **Provider abstraction**: single `complete()` with streaming via events, or separate `stream()` method? Current draft uses `CompletionOpts.stream`. Decide and lock.
4. **Tool execution concurrency**: sequential vs parallel tool calls within one turn. Recommend sequential for Phase 1 (simpler approvals), parallel later.
5. **Approval transport**: oneshot channel per call vs centralized approval broker. Recommend centralized broker in daemon for multi-client (TUI+desktop) consistency.
6. **Session ID format**: UUID v4 vs ULID. Recommend ULID for sortable filenames.
7. **Context compaction trigger**: token count threshold vs message count. Recommend token count via provider tokenizer or char/4 estimate.
8. **Memory backend default**: file JSONL vs SQLite. Recommend file JSONL for Phase 1, SQLite optional.
9. **Recipe schema versioning**: strict semver on `version` field with migration, or accept any. Recommend strict with migration hooks.
10. **Guardrail implementation**: trait object vs enum dispatch. Recommend trait object (`Box<dyn Guardrail>`) for composability.
11. **Streaming granularity**: token deltas vs sentence chunks. Recommend token deltas.
12. **Error type strategy**: thiserror per crate vs shared protocol error. Recommend per-crate thiserror + protocol-level error enum for IPC.
13. **Plugin loading**: compile-time (feature flags) vs runtime (dylib). Recommend compile-time feature flags for Phase 1.
14. **TUI framework**: ratatui (confirmed) - any objection?
15. **License of references**: confirm codex/rust-sdk licenses before any adaptation; until then inspiration only.

## NEEDS VERIFICATION

- All reference projects were missing/empty; study could not be performed. Re-run Phase 0 once `./reference/` is populated.
- Crate API versions not verified (no compile). Isolated behind modules; verify during Phase 1.
