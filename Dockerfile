# syntax=docker/dockerfile:1.7

ARG NODE_VERSION=20.19.2
ARG CUDA_VERSION=12.6.2
ARG UBUNTU_VERSION=22.04
ARG RUST_VERSION=1.82.0

FROM node:${NODE_VERSION}-bullseye-slim AS frontend-builder
WORKDIR /app/webui
COPY webui/pnpm-lock.yaml webui/package.json ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY webui/ .
RUN pnpm build

FROM node:${NODE_VERSION}-bullseye-slim AS node-runtime

FROM nvidia/cuda:${CUDA_VERSION}-devel-ubuntu${UBUNTU_VERSION} AS backend-builder
ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    clang \
    cmake \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain ${RUST_VERSION}
ENV PATH="/root/.cargo/bin:${PATH}" \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    CUDA_HOME=/usr/local/cuda

COPY Cargo.toml Cargo.lock ./
COPY runtime/Cargo.toml runtime/
RUN cargo fetch --locked
COPY runtime/ runtime/
RUN cargo build --locked --release

FROM nvidia/cuda:${CUDA_VERSION}-runtime-ubuntu${UBUNTU_VERSION} AS runtime
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    nginx \
    bash \
    tini \
    procps \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=node-runtime /usr/local /usr/local
COPY --from=backend-builder /app/target/release/runtime /usr/local/bin/runtime
COPY --from=frontend-builder /app/webui/build /opt/webui

WORKDIR /opt/app
COPY runtime/config /opt/app/config
COPY docker/nginx.conf /etc/nginx/nginx.conf
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod +x /usr/local/bin/entrypoint.sh \
    && sed -i 's/host:[[:space:]]\+127\.0\.0\.1/host: 0.0.0.0/' /opt/app/config/app.yaml

ENV APP_CONFIG_PATH=/opt/app/config/app.yaml \
    BACKEND_PORT=42069 \
    FRONTEND_PORT=3000 \
    PUBLIC_PORT=8080 \
    RUST_LOG=info

EXPOSE 8080

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]
