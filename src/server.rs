use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::indexer::run_index;
use crate::pack::load_manifest;
use crate::query::run_query;

#[derive(Clone)]
struct AppState {
    pack: Arc<PathBuf>,
    falkordb_socket: Option<String>,
}

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
}

fn default_mode() -> String {
    "hybrid".to_string()
}

fn default_top_k() -> usize {
    8
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    falkordb_socket: Option<String>,
    falkordb_connected: Option<bool>,
}

pub async fn run_server(
    pack: PathBuf,
    host: String,
    port: u16,
    falkordb_socket: Option<String>,
) -> Result<()> {
    let state = AppState {
        pack: Arc::new(pack),
        falkordb_socket,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/query", post(query))
        .route("/index", post(index_now))
        .route("/mcp", post(mcp))
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let socket_path = state.falkordb_socket.clone();
    let connected = socket_path.as_deref().map(can_connect_to_socket);
    let ok = connected.unwrap_or(true);
    let status = if ok { "ok" } else { "degraded" }.to_string();
    let code = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        code,
        Json(HealthResponse {
            status,
            falkordb_socket: socket_path,
            falkordb_connected: connected,
        }),
    )
}

#[cfg(unix)]
fn can_connect_to_socket(path: &str) -> bool {
    std::os::unix::net::UnixStream::connect(path).is_ok()
}

#[cfg(not(unix))]
fn can_connect_to_socket(_path: &str) -> bool {
    false
}

async fn status(State(state): State<AppState>) -> Json<Value> {
    let manifest = load_manifest(&state.pack).ok();
    Json(json!({
        "status": "ok",
        "pack_path": state.pack.display().to_string(),
        "sources": manifest.map(|m| m.sources).unwrap_or_default()
    }))
}

async fn query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match run_query(&state.pack, &req.query, &req.mode, req.top_k) {
        Ok(resp) => Ok(Json(json!(resp))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error":{"code":"QUERY_FAILED","message":e.to_string()}})),
        )),
    }
}

async fn index_now(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let manifest = load_manifest(&state.pack).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":{"code":"PACK_INVALID","message":e.to_string()}})),
        )
    })?;
    let sources: Vec<PathBuf> = manifest
        .sources
        .iter()
        .map(|s| PathBuf::from(&s.root_path))
        .collect();
    let (scanned, updated, chunks) = run_index(&state.pack, &sources).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":{"code":"INDEX_FAILED","message":e.to_string()}})),
        )
    })?;
    Ok(Json(json!({
        "status":"ok",
        "scanned": scanned,
        "updated_files": updated,
        "chunks": chunks
    })))
}

async fn mcp(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let method = payload.get("method").and_then(Value::as_str).unwrap_or("");
    let id = payload.get("id").cloned().unwrap_or(json!(null));

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {"name":"satori","version":"0.1.0"},
            "capabilities": {"tools": {}}
        }),
        "tools/list" => json!({
            "tools": [
                {"name":"memory_query","description":"Query local memory pack","inputSchema":{
                    "type":"object","properties":{
                        "query":{"type":"string"},
                        "mode":{"type":"string","enum":["vector","hybrid"]},
                        "top_k":{"type":"number"}
                    },
                    "required":["query"]
                }},
                {"name":"memory_status","description":"Return daemon status","inputSchema":{"type":"object","properties":{}}},
                {"name":"memory_sources","description":"List active memory source roots","inputSchema":{"type":"object","properties":{}}}
            ]
        }),
        "tools/call" => {
            let name = payload
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = payload
                .get("params")
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));

            match name {
                "memory_query" => {
                    let query = args
                        .get("query")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let mode = args
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("hybrid")
                        .to_string();
                    let top_k = args.get("top_k").and_then(Value::as_u64).unwrap_or(8) as usize;

                    match run_query(&state.pack, &query, &mode, top_k) {
                        Ok(r) => json!({"content":[{"type":"text","text":serde_json::to_string(&r).unwrap_or_default()}]}),
                        Err(e) => json!({"isError": true, "content":[{"type":"text","text":e.to_string()}]}),
                    }
                }
                "memory_status" => json!({
                    "content":[{"type":"text","text":json!({"status":"ok","pack_path":state.pack.display().to_string()}).to_string()}]
                }),
                "memory_sources" => json!({
                    "content":[{"type":"text","text":match load_manifest(&state.pack) {
                        Ok(m) => json!({"sources":m.sources}).to_string(),
                        Err(_) => json!({"sources":[]}).to_string(),
                    }}]
                }),
                _ => json!({"isError": true, "content":[{"type":"text","text":"unknown tool"}]}),
            }
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error":{"code":"BAD_METHOD","message":"unsupported method"}})),
            ));
        }
    };

    Ok(Json(json!({"jsonrpc":"2.0","id":id,"result":result})))
}
