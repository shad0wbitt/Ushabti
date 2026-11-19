# Progress

## Phase 0: Study and Design Notes

### Files created
- docs/REFERENCE-NOTES.md
- docs/COMPARISON.md
- docs/DESIGN.md
- docs/OPEN-QUESTIONS.md
- docs/PROGRESS.md

### Key design decisions
- Project name recommended: Kestrel (options: Kestrel, Loom, Forge).
- Crate layout: protocol, core, modules, mcp, daemon, cli, tui, desktop.
- Traits: Provider, Tool, Memory, Guardrail, Channel (async_trait, Send+Sync).
- IPC: newline-delimited JSON over Unix socket; stdio for CLI one-shot.
- Session storage: append-only JSONL + meta sidecar under ~/.kestrel/sessions.
- License: MIT OR Apache-2.0; references inspiration-only until verified.

### Assumptions
- No reference source trees present on disk; all reference sections are placeholders pending population of ./reference/.
- No Rust code written in Phase 0, per instruction.

### NEEDS VERIFICATION
- Reference study (all projects missing/empty).
- Crate dependency versions and APIs (no compile performed).
- Desktop framework choice (open question).
- Provider streaming signature (open question).

---

## Phase 1: Core Vertical Slice

### Files created/changed
- Cargo.toml (workspace root)
- rustfmt.toml, .gitignore, README.md
- crates/protocol/Cargo.toml, crates/protocol/src/lib.rs
- crates/core/Cargo.toml, crates/core/src/lib.rs, error.rs, traits.rs, agent.rs
- crates/modules/Cargo.toml, crates/modules/src/lib.rs, truncate.rs, paths.rs, read_tool.rs, edit_tool.rs, bash_tool.rs, openai_provider.rs
- crates/cli/Cargo.toml, crates/cli/src/main.rs
- docs/PROGRESS.md (this file)

### Key design decisions
- Provider uses a single `complete_stream` returning a `Stream<Item = Result<Chunk, ProviderError>>` (streaming via events, no separate method).
- Agent loop runs tool calls in parallel via `tokio::task::JoinSet`; results appended preserving call order.
- ToolContext.approval_tx is a placeholder channel in core; real wiring deferred to daemon phase.
- Path safety: lexical normalization plus canonicalize checks for existing paths/symlinks; rejects `..` escapes and symlink escapes.
- Truncation: byte-based with char-boundary walkback and a marker.
- Config precedence: flags > env vars > defaults.
- bash uses `sh -c` with timeout and merged stdout/stderr.

### Assumptions
- `async-stream` crate added (not in original list) for SSE parsing; isolated in openai_provider.rs.
- `ToolContext` includes `approval_tx` placeholder; daemon will replace.
- No tests written, no commands run.

### NEEDS VERIFICATION
- Compiles? Not run. Verify async-trait + tokio JoinSet + futures StreamExt usage.
- `async_stream::stream!` macro availability with chosen crate version.
- reqwest 0.12 `bytes_stream()` and `bearer_auth`.
- `tokio::io::AsyncReadExt` imported inside macro block (may need top-level use).
- `std::env::current_dir` in agent loop for ToolContext cwd (should use config cwd; refine later).
- No MCP, memory, guardrails, or daemon in this phase.
- ToolContext.approval_tx placeholder must be replaced before approvals work.

---

## Phase 2: Recipes and Modules

### Files created/changed
- crates/recipe/Cargo.toml, crates/recipe/src/lib.rs, recipe.rs
- crates/core/src/modules/mod.rs
- crates/core/src/memory/mod.rs
- crates/core/src/guardrails/mod.rs
- crates/core/src/sessions/mod.rs
- crates/core/src/approvals/mod.rs
- crates/core/src/versioning/mod.rs

### Key design decisions
- Recipe parsed from YAML with validation returning field paths.
- Module trait + Manifest with schemars JSON Schema; ModuleRegistry with compatibility warnings.
- Memory: none, in-memory, SQLite keyword recall.
- Sessions: SQLite with runtime queries, list/load/save.
- Guardrails: Guardrail trait input/output hooks; SpendCap and PiiRedaction.
- Approvals: ApprovalBroker in core with oneshot + timeout deny-by-default.
- Versioning: immutable hash+timestamp+content in SQLite.

### Assumptions
- PiiRedaction uses naive string handling (no regex crate) pending dependency decision.
- No compile-time sqlx macros; runtime queries only.

### NEEDS VERIFICATION
- sqlx 0.8 API (`SqlitePool::connect`, `query_as` tuple mapping).
- schemars 0.8 JsonSchema derive on Manifest.
- minijinja 2 template API.
- No tests written, no commands run.

---

## Phase 3: MCP and Tools

### Status
Blocked: ./reference/rust-sdk is missing. Per task instruction, stopped and reported. No code written for this phase. Re-run once reference/rust-sdk is present (or explicit instruction to use rmcp from crates.io).

---

## Phase 4: Terminal UI

### Files created/changed
- crates/tui/Cargo.toml, crates/tui/src/lib.rs
- crates/tui/src/theme.rs
- crates/tui/src/app.rs
- crates/tui/src/keybindings.rs
- crates/tui/src/render.rs
- Cargo.toml (added crossterm, ratatui, crates/tui member)

### Key design decisions
- App state, pure `update(state, input)` reducer, separate `render` function.
- Keybindings in one table (keybindings.rs).
- Theme struct with dark/light palettes; no hard-coded colors in widgets (render uses theme).
- Approval modal blocks input until answered; y/a/n keys.
- Panic-safe terminal handling and stdout discipline: documented as requirements; concrete panic-hook wiring to be done in binary (not yet wired).
- Small terminal sizes handled in render without panic.

### Assumptions
- ratatui 0.29 API (Frame, Paragraph, Layout) assumed; verify.
- crossterm 0.28 event API assumed; verify.
- No actual TUI binary wiring yet (harness with no -p flag) — requires daemon/core channel wiring; deferred.
- No tests written, no commands run.

### NEEDS VERIFICATION
- ratatui/crossterm API details.
- Event loop integration with core mpsc channel and tick timer (not implemented).
- Markdown-lite styling not implemented (plain text only).
- Panic hook and terminal restore not wired in binary.
EOF