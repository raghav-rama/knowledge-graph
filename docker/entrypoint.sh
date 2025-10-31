#!/usr/bin/env bash
set -Eeuo pipefail

BACKEND_BIN="${BACKEND_BIN:-/usr/local/bin/runtime}"
APP_CONFIG_PATH="${APP_CONFIG_PATH:-/opt/app/config/app.yaml}"
BACKEND_PORT="${BACKEND_PORT:-42069}"
FRONTEND_PORT="${FRONTEND_PORT:-3000}"
PUBLIC_PORT="${PUBLIC_PORT:-8080}"

log() {
  printf '[%s] %s\n' "$(date --iso-8601=seconds)" "$*"
}

cleanup() {
  log "Shutting down processes…"
  pkill -TERM -P $$ || true
  wait || true
}
trap cleanup SIGINT SIGTERM EXIT

if [ ! -f "$APP_CONFIG_PATH" ]; then
  log "Config file not found at $APP_CONFIG_PATH"
  exit 1
fi

if ! grep -q 'host: 0.0.0.0' "$APP_CONFIG_PATH"; then
  log "Normalizing server host to 0.0.0.0 inside $APP_CONFIG_PATH"
  sed -i 's/host:\s\+.*/host: 0.0.0.0/' "$APP_CONFIG_PATH"
fi

export APP_CONFIG_PATH
export HOST=0.0.0.0
export PORT="$FRONTEND_PORT"

log "Starting Rust backend on port $BACKEND_PORT"
"$BACKEND_BIN" &
backend_pid=$!

log "Starting SvelteKit frontend (Node) on port $FRONTEND_PORT"
node --enable-source-maps /opt/webui/index.js &
frontend_pid=$!

log "Starting nginx reverse proxy on port $PUBLIC_PORT"
nginx -g 'daemon off;' &
nginx_pid=$!

wait -n "$backend_pid" "$frontend_pid" "$nginx_pid"
