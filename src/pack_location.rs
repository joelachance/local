//! Pack location: local path. All pack I/O goes through this abstraction.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Pack location: a local directory.
#[derive(Clone, Debug)]
pub enum PackLocation {
    Local(PathBuf),
}

impl PackLocation {
    /// Local filesystem path.
    pub fn local(path: impl Into<PathBuf>) -> Self {
        PackLocation::Local(path.into())
    }

    /// Path for local pack.
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            PackLocation::Local(p) => Some(p.as_path()),
        }
    }

    /// Read a file by relative path (e.g. "manifest.json", "state/file_state.json").
    pub fn read_file(&self, rel_path: &str) -> Result<Vec<u8>> {
        match self {
            PackLocation::Local(root) => {
                let p = root.join(rel_path);
                std::fs::read(&p).with_context(|| format!("failed to read {}", p.display()))
            }
        }
    }

    /// Write a file by relative path.
    pub fn write_file(&self, rel_path: &str, data: &[u8]) -> Result<()> {
        match self {
            PackLocation::Local(root) => {
                let p = root.join(rel_path);
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent).context("failed to create parent dir")?;
                }
                std::fs::write(&p, data).with_context(|| format!("failed to write {}", p.display()))
            }
        }
    }
}
