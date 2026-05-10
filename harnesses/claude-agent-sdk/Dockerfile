# syntax=docker/dockerfile:1.7
#
# Claude Agent SDK harness — peer of harnesses/opencode.
#
# Same shape as the opencode image (debian-slim runtime, sandbox user,
# /work workdir, port 4096, entrypoint clones $REPO_URL then starts the
# server). The differences:
#   - no opencode binary; we ship a Node server that drives @anthropic-ai/claude-agent-sdk
#   - pre-install gh + uv so agents don't have to bootstrap them at runtime
#     (the gap that bit session adc694c3 / 2f12d7b3 today).

FROM node:20-bookworm AS builder
WORKDIR /build
COPY package.json tsconfig.json ./
RUN npm install --no-audit --no-fund
COPY src ./src
RUN npx tsc

# ------------------------------------------------------------------ runtime
FROM node:20-bookworm-slim

# gh CLI: pinned release + checksum (matches the opencode harness's posture).
ARG GH_VERSION=2.92.0
ARG GH_SHA256=8f8212b1a9cec261a8839e0893168f50d3fc70f095da257feef4229234cefdf8

RUN apt-get update && apt-get install -y --no-install-recommends \
      git ca-certificates bash python3 curl jq \
    && curl -fsSL -o /tmp/gh.deb \
         "https://github.com/cli/cli/releases/download/v${GH_VERSION}/gh_${GH_VERSION}_linux_amd64.deb" \
    && echo "${GH_SHA256}  /tmp/gh.deb" | sha256sum -c - \
    && dpkg -i /tmp/gh.deb \
    && rm /tmp/gh.deb \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1001 -s /bin/bash sandbox \
    && mkdir -p /work \
    && chown sandbox:sandbox /work

# uv — installed to a system path so $PATH already finds it for any user.
# Static binary release; no shell pipe.
ARG UV_VERSION=0.5.13
RUN curl -fsSL -o /tmp/uv.tar.gz \
      "https://github.com/astral-sh/uv/releases/download/${UV_VERSION}/uv-x86_64-unknown-linux-gnu.tar.gz" \
    && tar -xzf /tmp/uv.tar.gz -C /tmp \
    && install -m 0755 "/tmp/uv-x86_64-unknown-linux-gnu/uv" /usr/local/bin/uv \
    && install -m 0755 "/tmp/uv-x86_64-unknown-linux-gnu/uvx" /usr/local/bin/uvx \
    && rm -rf /tmp/uv.tar.gz /tmp/uv-x86_64-unknown-linux-gnu

# Harness server — only the production node_modules + compiled JS. We do a
# fresh `npm install --omit=dev` here so the runtime image doesn't carry
# tsc / @types — keeps the layer small.
WORKDIR /opt/harness
COPY --from=builder /build/package.json ./
COPY --from=builder /build/dist ./dist
RUN npm install --omit=dev --no-audit --no-fund \
    && chown -R sandbox:sandbox /opt/harness \
    # Symlink the SDK's bundled `claude` binary into a stable PATH location
    # so the SDK's auto-resolver finds it regardless of cwd, AND so an agent
    # running inside the sandbox can invoke `claude` as a tool too.
    && ln -sf /opt/harness/node_modules/@anthropic-ai/claude-agent-sdk-linux-x64/claude \
         /usr/local/bin/claude

WORKDIR /work
COPY --chown=sandbox:sandbox entrypoint.sh /entrypoint.sh
RUN chmod 755 /entrypoint.sh

USER sandbox
EXPOSE 4096
ENTRYPOINT ["/entrypoint.sh"]
