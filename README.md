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

Equivalent daemon aliases:

```bash
bun run daemon:start
bun run daemon:status
bun run daemon:stop
```

This starts:

- FalkorDB sidecar over Unix socket (`FALKORDB_SOCKET`, default `/tmp/falkordb.sock`)
- Rust API daemon (`target/release/satori --headless-serve ...`) on `API_PORT` (default `4242`)

On first run, FalkorDB sidecar artifacts are downloaded and verified into `./.local-runtime/falkor/`.

For query synthesis (natural language answers), run `bun run models:fetch` once to download a small GGUF model (~700MB) into `./.local-runtime/models/` (or set `SATORI_ONTOLOGY_MODEL` to your own GGUF path).

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
- `FALKOR_GRAPH` (default `satori`)
- `LANCEDB_PATH` (default local: `./.local-data/lance`, docker: `/data/lance`)
- `API_PORT` (default `4242` for runtime scripts; `--headless-serve` still defaults to `7821` when called directly)
- `AUTH_SECRET` (recommended; used as required runtime secret contract)
- `SATORI_ONTOLOGY_PROVIDER` (`llama` default; `rules` or `candle` optional)
- `SATORI_ONTOLOGY_MODEL` (GGUF model path; default `.local-runtime/models/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf` after `bun run models:fetch`)
- `SATORI_ONTOLOGY_MAX_TOKENS` (default `512`)
- `SATORI_ONTOLOGY_TIMEOUT_MS` (default `20000`)

To enable true in-process `llama.cpp` inference via Rust bindings, build with:

```bash
cargo build --features llama-embedded
```

Notes:
- `llama-embedded` requires local build tooling (`cmake` + C/C++ toolchain).
- Without this feature, the `llama` provider falls back to local `llama-cli` if present.

Sidecar details:

- `FALKOR_RUNTIME_ROOT` (optional, defaults to `./.local-runtime/falkor`)
- Artifacts are checksum-verified before extraction.
- Current native sidecar support: `darwin-arm64`, `linux-x86_64`.

## Quick start (command-first)

```bash
bun run local:build
bun run local:start
./target/release/satori sources add ./specs
./target/release/satori jobs list
./target/release/satori index
./target/release/satori jobs list
./target/release/satori query "local memory pack"
./target/release/satori status
./target/release/satori ontology list
./target/release/satori ontology show --source /absolute/path/to/specs/prd-v1-local-memory.md
./target/release/satori ontology export --source /absolute/path/to/specs/prd-v1-local-memory.md --out ./specs-ontology.json
```

In another terminal:

```bash
curl -s http://127.0.0.1:4242/health
curl -s -X POST http://127.0.0.1:4242/query -H "content-type: application/json" -d '{"query":"local memory pack","mode":"hybrid","top_k":5}'
curl -s -X POST http://127.0.0.1:4242/index -H "content-type: application/json" -d '{}'
curl -s http://127.0.0.1:4242/graph/schema
curl -s -X POST http://127.0.0.1:4242/graph/subgraph -H "content-type: application/json" -d '{"query":"memory pack","depth":2,"limit":25}'
curl -s http://127.0.0.1:4242/ontology/sources
curl -s --get http://127.0.0.1:4242/ontology/source --data-urlencode "path=./specs"
```

If you query a directory or unknown path with `/ontology/source`, the response now includes `error.suggestions[]` with valid file-level `source_path` values you can use directly.

## Spec status

- Current target contract is command-first (`specs/cli-v1.md` V1.1).
- TUI is not part of the required runtime surface.
- Background ingestion is defined as watch lifecycle (`watch start`/`watch stop`) in spec.
- Cloud sync/push is deferred and only extension points are in scope.

## Command contract (V1.1 target)

- `satori status`
- `satori query "<query>" [--mode vector|hybrid] [--top-k N]`
- `satori index`
- `satori sources list`
- `satori sources add <path>`
- `satori sources remove <path>`
- `satori jobs list`
- `satori jobs status <job-id>`
- `satori --headless-serve --pack <path> [--host 127.0.0.1] [--port 7821]`

## Notes

- Storage is persisted in LanceDB files under `<pack>/lancedb/` (for example `chunks.lance`).
- Indexing writes to both LanceDB and Falkor graph (when `FALKORDB_SOCKET` is available).
- `POST /index` and `sources add` enqueue background indexing jobs and return immediately with `job` metadata.
- Use `satori jobs list` / `satori jobs status <job-id>` (or `GET /jobs`) to monitor long-running ingestion.
- Query retrieval fans out to LanceDB and Falkor in parallel, then applies grouped rerank.
- Ontology extraction is local and embedded (no separate Ollama runtime required); cache is stored at `<pack>/state/ontology_cache.json`.
- Per-source ontology artifacts are emitted under `<pack>/ontology/*.ontology.json`.
- `GET /ontology/source` expects a file-level source path; for directories or unknown paths it returns suggestions instead of a bare not-found.
- `satori ontology show --source <path>` prints suggestions with follow-up commands when no exact source artifact exists.
- `llama` provider is selected by default and falls back to `rules` if unavailable.
- `fastembed` is implemented for ONNX-based local embeddings.
- Default init now uses `fastembed`; if model init fails, runtime falls back to `hash`.
- `serve` performs a single startup index pass using manifest sources, then serves a static snapshot.
- Graph inspection endpoints:
  - `GET /graph/schema`
  - `POST /graph/subgraph`
  - `GET /graph/view` (simple browser visualization)
- `/mcp` endpoint implements a minimal MCP JSON-RPC tool surface:
  - `memory_query`
  - `memory_status`
  - `memory_sources`
