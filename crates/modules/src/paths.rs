//! Path safety: resolve inside workspace root, reject escapes.

use std::path::{Path, PathBuf};

/// Resolve `input` against `root`, rejecting `..` escapes and symlinks that leave root.
pub fn safe_resolve(root: &Path, input: &str) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("cannot canonicalize root: {e}"))?;
    // Reject absolute paths that are not under root; relative inputs joined to root.
    let candidate = if Path::new(input).is_absolute() {
        PathBuf::from(input)
    } else {
        root.join(input)
    };
    // Lexically reject .. traversal.
    let mut normalized = PathBuf::new();
    for comp in candidate.components() {
        match comp {
            std::path::Component::ParentDir => {
                // pop if possible; if root exhausted, reject
                if !normalized.pop() {
                    return Err("path escapes workspace root".into());
                }
            }
            std::path::Component::CurDir => {}
            c => normalized.push(c.as_os_str()),
        }
    }
    // Ensure under root.
    if !normalized.starts_with(&root) {
        return Err("path escapes workspace root".into());
    }
    // If exists, canonicalize and re-check (catches symlink escapes).
    if normalized.exists() {
        let canon = normalized
            .canonicalize()
            .map_err(|e| format!("cannot canonicalize path: {e}"))?;
        if !canon.starts_with(&root) {
            return Err("path escapes workspace root via symlink".into());
        }
        Ok(canon)
    } else {
        // For new files, check parent exists and is inside root.
        if let Some(parent) = normalized.parent() {
            if parent.exists() {
                let pc = parent
                    .canonicalize()
                    .map_err(|e| format!("cannot canonicalize parent: {e}"))?;
                if !pc.starts_with(&root) {
                    return Err("path escapes workspace root via symlink".into());
                }
            }
        }
        Ok(normalized)
    }
}
EOF