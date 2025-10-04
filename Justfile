set shell := ["bash", "-u", "-o", "pipefail", "-c"]

# Default target runs both dev servers
default: dev

# Common commands used by multiple recipes
_dev_backend_command := "cd runtime && cargo watch -x 'run'"
_dev_frontend_command := "cd webui && pnpm run dev"

# Backend dev server with hot reload
dev-backend:
    {{_dev_backend_command}}

# Frontend dev server
dev-frontend:
    {{_dev_frontend_command}}

# Run backend and frontend dev servers side-by-side
# Both processes shut down cleanly on SIGINT
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'kill 0' EXIT
    ({{_dev_backend_command}}) &
    backend_pid=$!
    # ({{_dev_frontend_command}}) &
    # frontend_pid=$!
    # wait $backend_pid $frontend_pid
    ({{_dev_frontend_command}})
    wait $backend_pid

# Backend workflows
fmt-backend:
    cd runtime && cargo fmt --all

lint-backend:
    cd runtime && cargo clippy --all-targets --all-features -- -D warnings

test-backend:
    cd runtime && cargo test

build-backend:
    cd runtime && cargo build --release

# Frontend workflows
fmt-frontend:
    cd webui && pnpm format

lint-frontend:
    cd webui && pnpm lint

check-frontend:
    cd webui && pnpm check

build-frontend:
    cd webui && pnpm build

# Aggregates
fmt: fmt-backend fmt-frontend

lint: lint-backend lint-frontend

test: test-backend check-frontend

check: check-frontend

build: build-backend build-frontend
