//! Unified pack location: local path or S3. All pack I/O goes through this abstraction.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;

/// Run async code from sync context (for S3 operations).
fn block_on_async<F, Fut, T>(f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(f()))
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime for S3");
        rt.block_on(f())
    }
}

/// Pack location: either a local directory or an S3 prefix.
#[derive(Clone, Debug)]
pub enum PackLocation {
    Local(PathBuf),
    S3 {
        bucket: String,
        prefix: String,
        /// Storage options for LanceDB (e.g. region, credentials). Keys/values passed to ConnectBuilder.
        storage_options: Vec<(String, String)>,
    },
}

impl PackLocation {
    /// Local filesystem path.
    pub fn local(path: impl Into<PathBuf>) -> Self {
        PackLocation::Local(path.into())
    }

    /// Parse an S3 URI (s3://bucket/prefix) into bucket and prefix. Prefix may be empty.
    pub fn from_s3_uri(uri: &str) -> Result<Self> {
        let uri = uri.trim();
        if !uri.starts_with("s3://") {
            bail!("invalid S3 URI: must start with s3://");
        }
        let rest = uri.strip_prefix("s3://").unwrap();
        let (bucket, prefix) = match rest.find('/') {
            Some(i) => {
                let bucket = rest[..i].to_string();
                let prefix = rest[i + 1..].trim_end_matches('/').to_string();
                (bucket, prefix)
            }
            None => (rest.to_string(), String::new()),
        };
        if bucket.is_empty() {
            bail!("invalid S3 URI: empty bucket");
        }
        let region = std::env::var("AWS_REGION").or_else(|_| std::env::var("AWS_DEFAULT_REGION")).ok();
        let mut storage_options: Vec<(String, String)> = Vec::new();
        if let Some(ref r) = region {
            storage_options.push(("region".to_string(), r.clone()));
        }
        Ok(PackLocation::S3 {
            bucket,
            prefix,
            storage_options,
        })
    }

    /// Path for local; None for S3.
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            PackLocation::Local(p) => Some(p.as_path()),
            PackLocation::S3 { .. } => None,
        }
    }

    /// Read a file by relative path (e.g. "manifest.json", "state/file_state.json").
    pub fn read_file(&self, rel_path: &str) -> Result<Vec<u8>> {
        match self {
            PackLocation::Local(root) => {
                let p = root.join(rel_path);
                std::fs::read(&p).with_context(|| format!("failed to read {}", p.display()))
            }
            PackLocation::S3 { bucket, prefix, .. } => {
                let key = if prefix.is_empty() {
                    rel_path.to_string()
                } else {
                    format!("{}/{}", prefix.trim_end_matches('/'), rel_path)
                };
                let bucket = bucket.clone();
                block_on_async(move || async move { read_s3_object(&bucket, &key).await })
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
            PackLocation::S3 { bucket, prefix, .. } => {
                let key = if prefix.is_empty() {
                    rel_path.to_string()
                } else {
                    format!("{}/{}", prefix.trim_end_matches('/'), rel_path)
                };
                let bucket = bucket.clone();
                let data = data.to_vec();
                block_on_async(move || async move { write_s3_object(&bucket, &key, &data).await })
            }
        }
    }

    /// URI for the LanceDB database (directory/prefix that contains the lance tables).
    pub fn lancedb_uri(&self) -> String {
        match self {
            PackLocation::Local(p) => p.join("lancedb").to_string_lossy().to_string(),
            PackLocation::S3 { bucket, prefix, .. } => {
                if prefix.is_empty() {
                    format!("s3://{}/lancedb", bucket)
                } else {
                    format!("s3://{}/{}/lancedb", bucket, prefix.trim_end_matches('/'))
                }
            }
        }
    }

    /// Storage options for LanceDB ConnectBuilder (S3 only). Caller can add credentials etc.
    pub fn storage_options(&self) -> Option<&[(String, String)]> {
        match self {
            PackLocation::Local(_) => None,
            PackLocation::S3 { storage_options, .. } => {
                if storage_options.is_empty() {
                    None
                } else {
                    Some(storage_options.as_slice())
                }
            }
        }
    }
}

// S3 helpers (async); we block_on when called from sync code.
async fn read_s3_object(bucket: &str, key: &str) -> Result<Vec<u8>> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .context("S3 GetObject failed")?;
    let data = resp.body.collect().await.context("S3 body read failed")?;
    Ok(data.into_bytes().to_vec())
}

async fn write_s3_object(bucket: &str, key: &str, data: &[u8]) -> Result<()> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = Client::new(&config);
    let body = ByteStream::from(data.to_vec());
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await
        .context("S3 PutObject failed")?;
    Ok(())
}

