# syntax=docker/dockerfile:1.7

ARG NODE_VERSION=20.19.2
ARG PUBLIC_API_BASE_URL=https://enhanced-kg.railway.internal

FROM node:${NODE_VERSION}-bullseye-slim AS builder
ARG PUBLIC_API_BASE_URL
ENV NODE_ENV=production \
    PUBLIC_API_BASE_URL=${PUBLIC_API_BASE_URL}
WORKDIR /app
COPY webui/pnpm-lock.yaml webui/package.json ./
RUN corepack enable && pnpm install --frozen-lockfile
COPY webui/ .
RUN pnpm build && pnpm prune --prod

FROM node:${NODE_VERSION}-bullseye-slim AS runner
ARG PUBLIC_API_BASE_URL
ENV NODE_ENV=production \
    PUBLIC_API_BASE_URL=${PUBLIC_API_BASE_URL} \
    HOST=0.0.0.0 \
    PORT=3000
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/pnpm-lock.yaml ./pnpm-lock.yaml
EXPOSE 3000
CMD ["node", "build/index.js"]
