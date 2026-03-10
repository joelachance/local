//! Input hardening for agent-invoked CLI. Rejects adversarial inputs that agents
//! may hallucinate: path traversal, embedded query params, control chars, etc.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

/// Rejects ASCII control characters (0x00-0x1F) and DEL (0x7F).
pub fn reject_control_chars(s: &str) -> Result<()> {
    for c in s.chars() {
        if c.is_control() {
            return Err(anyhow!(
                "invalid input: control character (U+{:04X}) not allowed",
                c as u32
            ));
        }
    }
    Ok(())
}

/// Validates resource IDs (job IDs, etc.). Rejects `?`, `#`, `%` and control chars.
pub fn validate_resource_id(s: &str) -> Result<()> {
    reject_control_chars(s)?;
    for c in s.chars() {
        if c == '?' || c == '#' || c == '%' {
            return Err(anyhow!(
                "invalid resource id: '{}' not allowed (possible embedded query or encoding)",
                c
            ));
        }
    }
    Ok(())
}

/// Validates path-like inputs for sources, ontology, etc.
/// Rejects: `..`, path traversal, control chars, `%` (pre-encoded).
pub fn validate_path(s: &str) -> Result<PathBuf> {
    reject_control_chars(s)?;
    if s.contains('%') {
        return Err(anyhow!(
            "invalid path: '%' not allowed (possible pre-encoded string)"
        ));
    }
    let p = PathBuf::from(s);
    if p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(anyhow!("invalid path: '..' traversal not allowed"));
    }
    Ok(p)
}

/// Validates that a path does not escape the given root when canonicalized.
pub fn validate_path_within_root(path: &Path, root: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize().map_err(|e| {
        anyhow!("path not found or invalid: {} ({})", path.display(), e)
    })?;
    let root_canonical = root.canonicalize().map_err(|e| {
        anyhow!("root path not found: {} ({})", root.display(), e)
    })?;
    if !canonical.starts_with(&root_canonical) {
        return Err(anyhow!(
            "path {} escapes root {}",
            canonical.display(),
            root_canonical.display()
        ));
    }
    Ok(canonical)
}
