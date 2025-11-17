# syntax=docker/dockerfile:1.7

ARG NODE_VERSION=20.19.2
ARG PUBLIC_API_BASE_URL=https://enhanced-kg.railway.internal

FROM node:${NODE_VERSION}-bullseye-slim AS builder
ARG PUBLIC_API_BASE_URL
ENV NODE_ENV=production \
    PUBLIC_API_BASE_URL=${PUBLIC_API_BASE_URL} \
    CI=true
WORKDIR /app

# Copy workspace metadata first for better caching
COPY pnpm-lock.yaml pnpm-workspace.yaml ./
COPY webui/package.json ./webui/package.json

RUN corepack enable \
    && pnpm install --filter webui --frozen-lockfile --ignore-scripts

COPY webui ./webui

RUN pnpm --filter webui run prepare \
    && pnpm --filter webui build \
    && pnpm install --filter webui --prod --frozen-lockfile --ignore-scripts

FROM node:${NODE_VERSION}-bullseye-slim AS runner
ARG PUBLIC_API_BASE_URL
ENV NODE_ENV=production \
    PUBLIC_API_BASE_URL=${PUBLIC_API_BASE_URL} \
    HOST=0.0.0.0 \
    PORT=3000
WORKDIR /app
COPY --from=builder /app/webui/node_modules ./node_modules
COPY --from=builder /app/webui/build ./build
COPY --from=builder /app/webui/package.json ./package.json
EXPOSE 3000
CMD ["node", "build/index.js"]
