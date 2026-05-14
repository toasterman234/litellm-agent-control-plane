#!/bin/sh
set -e

# Optionally clone a repo into REPO_DIR so the agent has a working tree.
if [ -n "$REPO_URL" ] && [ ! -d "$REPO_DIR/.git" ]; then
  echo "[entrypoint] cloning $REPO_URL into $REPO_DIR"
  mkdir -p "$REPO_DIR"
  git clone --depth=1 ${REPO_BRANCH:+--branch "$REPO_BRANCH"} "$REPO_URL" "$REPO_DIR"
fi

exec node /app/server.js
