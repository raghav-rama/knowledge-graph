# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.82.0
ARG CUDA_VERSION=12.6.2
ARG UBUNTU_VERSION=22.04

FROM nvidia/cuda:${CUDA_VERSION}-devel-ubuntu${UBUNTU_VERSION} AS builder
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
    CARGO_NET_GIT_FETCH_WITH_CLI=true

COPY Cargo.toml Cargo.lock ./
COPY runtime/Cargo.toml runtime/
RUN cargo fetch --locked
COPY runtime/ runtime/
RUN cargo build --locked --release

FROM nvidia/cuda:${CUDA_VERSION}-runtime-ubuntu${UBUNTU_VERSION} AS runtime
ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /opt/runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libclang1 \
    tzdata \
    tini \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/runtime /usr/local/bin/runtime
COPY runtime/config ./config
RUN sed -i 's/host:[[:space:]]*127\.0\.0\.1/host: 0.0.0.0/' ./config/app.yaml \
    && mkdir -p ./pgv-data-test

ENV APP_CONFIG_PATH=/opt/runtime/config/app.yaml \
    RUST_LOG=info

EXPOSE 42069

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/runtime"]
