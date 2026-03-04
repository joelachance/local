#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUN_DIR="${ROOT_DIR}/.local-run"
DATA_DIR="${ROOT_DIR}/.local-data"

SOCKET_PATH="${FALKORDB_SOCKET:-/tmp/falkordb.sock}"
LANCEDB_PATH="${LANCEDB_PATH:-${DATA_DIR}/lance}"
GRAPH_PATH="${GRAPH_PATH:-${DATA_DIR}/graph}"
PACK_PATH="${PACK_PATH:-${ROOT_DIR}/memory-pack}"
API_HOST="${API_HOST:-127.0.0.1}"
API_PORT="${API_PORT:-4242}"
AUTH_SECRET="${AUTH_SECRET:-dev-local-secret}"

mkdir -p "$RUN_DIR" "$LANCEDB_PATH" "$GRAPH_PATH" "$PACK_PATH"

if [ ! -x "${ROOT_DIR}/target/release/satori" ]; then
  "${SCRIPT_DIR}/local-build.sh"
fi

if [ -f "${RUN_DIR}/falkordb.pid" ] && kill -0 "$(cat "${RUN_DIR}/falkordb.pid")" 2>/dev/null; then
  echo "falkordb already running"
else
  rm -f "$SOCKET_PATH"

  if command -v falkordb-server >/dev/null 2>&1; then
    nohup falkordb-server \
      --unixsocket "$SOCKET_PATH" \
      --unixsocketperm 777 \
      --save 60 1 \
      --dir "$GRAPH_PATH" \
      > "${RUN_DIR}/falkordb.log" 2>&1 &
  elif command -v redis-server >/dev/null 2>&1 && [ -n "${FALKORDB_MODULE:-}" ]; then
    nohup redis-server \
      --loadmodule "${FALKORDB_MODULE}" \
      --unixsocket "$SOCKET_PATH" \
      --unixsocketperm 777 \
      --save 60 1 \
      --dir "$GRAPH_PATH" \
      > "${RUN_DIR}/falkordb.log" 2>&1 &
  else
    echo "could not start FalkorDB. Install falkordb-server or set FALKORDB_MODULE with redis-server." >&2
    exit 1
  fi

  echo $! > "${RUN_DIR}/falkordb.pid"
fi

i=0
while [ ! -S "$SOCKET_PATH" ] && [ "$i" -lt 20 ]; do
  sleep 0.5
  i=$((i + 1))
done

if [ ! -S "$SOCKET_PATH" ]; then
  echo "timed out waiting for FalkorDB socket at $SOCKET_PATH" >&2
  exit 1
fi

if [ -f "${RUN_DIR}/satori-api.pid" ] && kill -0 "$(cat "${RUN_DIR}/satori-api.pid")" 2>/dev/null; then
  echo "satori api already running"
else
  nohup env \
    FALKORDB_SOCKET="$SOCKET_PATH" \
    LANCEDB_PATH="$LANCEDB_PATH" \
    API_PORT="$API_PORT" \
    AUTH_SECRET="$AUTH_SECRET" \
    "${ROOT_DIR}/target/release/satori" \
    serve \
    --pack "$PACK_PATH" \
    --host "$API_HOST" \
    --port "$API_PORT" \
    > "${RUN_DIR}/satori-api.log" 2>&1 &
  echo $! > "${RUN_DIR}/satori-api.pid"
fi

echo "started local stack"
echo "socket: $SOCKET_PATH"
echo "api: http://${API_HOST}:${API_PORT}"
