mod embed;
mod indexer;
mod lancedb_store;
mod pack;
mod query;
mod server;
mod types;

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::indexer::run_index;
use crate::pack::{init_pack, load_file_state, load_index, load_manifest};
use crate::query::run_query;
use crate::server::run_server;

#[derive(Parser)]
#[command(name = "satori", version, about = "Local memory pack CLI + daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, default_value_t = false)]
        force: bool,
        #[arg(long, default_value = "fastembed")]
        provider: String,
        #[arg(long, default_value = "BAAI/bge-small-en-v1.5")]
        model: String,
        #[arg(long, default_value_t = 384)]
        dim: usize,
    },
    Index {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, num_args = 1..)]
        source: Vec<PathBuf>,
    },
    Query {
        query: String,
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, default_value = "hybrid")]
        mode: String,
        #[arg(long, default_value_t = 8)]
        top_k: usize,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    Serve {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 7821)]
        port: u16,
    },
    Status {
        #[arg(long)]
        pack: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            pack,
            force,
            provider,
            model,
            dim,
        } => {
            init_pack(&pack, force, &provider, &model, dim)?;
            println!("initialized pack: {}", pack.display());
        }
        Commands::Index { pack, source } => {
            let (scanned, updated, chunks) = run_index(&pack, &source)?;
            println!(
                "index complete: scanned={} updated_files={} chunks={} pack={}",
                scanned,
                updated,
                chunks,
                pack.display()
            );
        }
        Commands::Query {
            query,
            pack,
            mode,
            top_k,
            json,
        } => {
            let resp = run_query(&pack, &query, &mode, top_k)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else {
                for hit in resp.results {
                    println!("[{:.3}] {} ({})", hit.score, hit.file_path, hit.chunk_id);
                }
            }
        }
        Commands::Serve { pack, host, port } => {
            let manifest = load_manifest(&pack)?;
            let sources: Vec<PathBuf> = manifest
                .sources
                .iter()
                .map(|s| PathBuf::from(&s.root_path))
                .collect();
            if !sources.is_empty() {
                let (scanned, updated, chunks) = run_index(&pack, &sources)?;
                println!(
                    "startup index complete: scanned={} updated_files={} chunks={}",
                    scanned, updated, chunks
                );
            } else {
                println!("startup index skipped: no sources configured in manifest");
            }
            let port = std::env::var("API_PORT")
                .ok()
                .and_then(|p| u16::from_str(&p).ok())
                .unwrap_or(port);
            let falkordb_socket = std::env::var("FALKORDB_SOCKET").ok();
            println!("serving pack {} on {}:{}", pack.display(), host, port);
            run_server(pack, host, port, falkordb_socket).await?;
        }
        Commands::Status { pack } => {
            let manifest = load_manifest(&pack)?;
            let index = load_index(&pack)?;
            let states = load_file_state(&pack)?;
            println!("pack: {}", pack.display());
            println!("pack_id: {}", manifest.pack_id);
            println!("format: {}", manifest.format_version);
            println!(
                "embedding: provider={} model={} dim={}",
                manifest.embedding.provider, manifest.embedding.model, manifest.embedding.dimension
            );
            println!("sources: {}", manifest.sources.len());
            println!("files_state: {}", states.len());
            println!("chunks: {}", index.docs.len());
        }
    }

    Ok(())
}
