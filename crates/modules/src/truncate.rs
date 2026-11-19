//! Shared truncation helper.

/// Truncate output to `max_bytes`, appending a marker.
pub fn truncate_output(s: &str, max_bytes: usize) -> (String, bool) {
    if s.len() <= max_bytes {
        return (s.to_string(), false);
    }
    // Walk back to a char boundary.
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = s[..end].to_string();
    out.push_str("\n...[truncated]...\n");
    (out, true)
}
EOF