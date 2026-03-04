# satori

Local memory pack CLI + daemon (Rust).

## Runtime modes

### Local Runtime (Native Rust Binary, Fast Path)

Use a native release build for local speed and no Docker dependency:

```bash
bun run local:build
AUTH_SECRET=dev-secret bun run local:start
bun run local:status
```

This starts:

- FalkorDB over Unix socket (`FALKORDB_SOCKET`, default `/tmp/falkordb.sock`)
- Rust API daemon (`target/release/satori serve ...`) on `API_PORT` (default `4242`)

Stop services:

```bash
bun run local:stop
```

### Development/Deployment with Docker

Build and run the single-container stack from this repo root:

```bash
cargo build --release
docker build -f docker/Dockerfile -t satori .
docker run -p 4242:4242 -v satori-data:/data -e AUTH_SECRET=dev-secret satori
```

Health:

```bash
curl -s http://127.0.0.1:4242/health
```

### Shared environment contract

- `FALKORDB_SOCKET` (default `/tmp/falkordb.sock`)
- `LANCEDB_PATH` (default local: `./.local-data/lance`, docker: `/data/lance`)
- `API_PORT` (default `4242` for runtime scripts; CLI `serve` still defaults to `7821` when called directly)
- `AUTH_SECRET` (recommended; used as required runtime secret contract)

## Quick start

```bash
cargo run -- init --pack ./memory-pack --provider fastembed --model BAAI/bge-small-en-v1.5 --dim 384
cargo run -- index --pack ./memory-pack --source ./specs
cargo run -- serve --pack ./memory-pack --port 7821
```

In another terminal:

```bash
curl -s http://127.0.0.1:7821/health
curl -s -X POST http://127.0.0.1:7821/query -H "content-type: application/json" -d '{"query":"local memory pack","mode":"hybrid","top_k":5}'
curl -s -X POST http://127.0.0.1:7821/index -H "content-type: application/json" -d '{}'
```

## Implemented commands

- `satori init --pack <path> [--provider hash|fastembed] [--model <model>] [--dim <n>]`
- `satori index --pack <path> --source <path>...`
- `satori query "<query>" --pack <path> [--mode vector|hybrid] [--top-k N] [--json]`
- `satori serve --pack <path> [--host 127.0.0.1] [--port 7821]`
- `satori status --pack <path>`

## Notes

- Storage is persisted in LanceDB files under `<pack>/lancedb/` (for example `chunks.lance`).
- Current retrieval is true hybrid: vector search + LanceDB FTS search with RRF fusion.
- `fastembed` is implemented for ONNX-based local embeddings.
- Default init now uses `fastembed`; if model init fails, runtime falls back to `hash`.
- `serve` performs a single startup index pass using manifest sources, then serves a static snapshot.
- `/mcp` endpoint implements a minimal MCP JSON-RPC tool surface:
  - `memory_query`
  - `memory_status`
  - `memory_sources`
