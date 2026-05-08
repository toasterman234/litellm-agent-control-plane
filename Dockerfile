# syntax=docker/dockerfile:1.7

# ---------- 1. install ----------
FROM node:20-alpine AS deps
WORKDIR /app

# Only copy lockfiles first so `npm ci` is cached unless deps change.
COPY package.json package-lock.json ./
RUN npm ci

# ---------- 2. build ----------
FROM node:20-alpine AS builder
WORKDIR /app

COPY --from=deps /app/node_modules ./node_modules
COPY . .

# `output: "standalone"` in next.config.ts emits .next/standalone with a
# minimal node_modules — that's what the runtime stage runs.
RUN npm run build

# ---------- 3. run ----------
FROM node:20-alpine AS runner
WORKDIR /app

ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME=0.0.0.0
# Tell Next.js it's behind /api/proxy on the same origin — no Pulumi gymnastics.
ENV NEXT_TELEMETRY_DISABLED=1

# Run as non-root.
RUN addgroup --system --gid 1001 nodejs \
 && adduser  --system --uid 1001 nextjs

COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
COPY --from=builder --chown=nextjs:nodejs /app/public ./public

USER nextjs
EXPOSE 3000

# server.js is what `output: "standalone"` writes — equivalent to `next start`
# but without dragging in the dev/test toolchain.
CMD ["node", "server.js"]
