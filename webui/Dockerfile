# syntax=docker/dockerfile:1.7

ARG NODE_VERSION=20.19.2
ARG PUBLIC_API_BASE_URL=http://enhanced-kg:42069

FROM node:${NODE_VERSION}-bullseye-slim AS builder
ARG PUBLIC_API_BASE_URL
ENV NODE_ENV=production \
    PUBLIC_API_BASE_URL=${PUBLIC_API_BASE_URL} \
    CI=true
WORKDIR /app

COPY pnpm-lock.yaml pnpm-workspace.yaml ./
COPY webui/package.json ./webui/package.json

RUN corepack enable \
    && pnpm install --filter webui --frozen-lockfile

COPY webui ./webui

RUN pnpm --filter webui run build \
    && pnpm install --filter webui --prod --frozen-lockfile

FROM node:${NODE_VERSION}-bullseye-slim AS runner
ARG PUBLIC_API_BASE_URL
ENV NODE_ENV=production \
    PUBLIC_API_BASE_URL=${PUBLIC_API_BASE_URL} \
    HOST=0.0.0.0 \
    PORT=3000
WORKDIR /app/webui
COPY --from=builder /app/node_modules /app/node_modules
COPY --from=builder /app/webui/node_modules ./node_modules
COPY --from=builder /app/webui/build ./build
COPY --from=builder /app/webui/package.json ./package.json
EXPOSE 3000
CMD ["node", "build/index.js"]
