# docs/SECURITY-REVIEW.md

Static review of the codebase. No execution performed.

## Findings

1. `crates/modules/src/bash_tool.rs` (execute): uses `sh -c` with user input; denylist/allowlist not yet implemented. Severity: High. Fix: add configurable denylist + optional allowlist; kill process group on timeout (currently kills child only).
2. `crates/modules/src/paths.rs` (safe_resolve): canonicalizes only if path exists; new-file parent check exists but symlinked intermediate dirs not fully resolved. Severity: Medium. Fix: canonicalize nearest existing ancestor.
3. `crates/modules/src/read_tool.rs`: ToolContext.approval_tx placeholder; no approval gating. Severity: Medium. Fix: wire ApprovalBroker.
4. `crates/daemon/src/main.rs`: token from env or random; no owner-only file perms; no rate limiting; no body size limits. Severity: High. Fix: implement per spec.
5. `crates/modules/src/openai_provider.rs`: API key stored in struct; ensure never logged. Severity: Medium. Fix: redact in Debug (not derived).
6. `crates/core/src/agent.rs`: ToolContext.cwd uses current_dir placeholder; should use configured workspace. Severity: Low.
7. Secret redaction in traces/sessions not implemented. Severity: High. Fix: add redaction pass.
8. Environment scrubbing for child processes not implemented. Severity: Medium.

## Applied fixes in this phase

- deny.toml, clippy lints, SECURITY.md added.
- Safe defaults documented (ask for bash/edit/patch/web_fetch/MCP).
- (Code-level hardening changes are scaffolds; full implementation deferred to integration pass.)
EOF