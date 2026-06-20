#!/bin/sh
set -e

# Enable all runtime profiles
export COMPOSE_PROFILES=opencode,deepagents,pydantic-deepagents

# Configure Docker Sandbox to bypass proxy for cliproxy
echo "Configuring Docker Sandbox networking..."
docker sandbox ls 2>/dev/null | tail -n +2 | while read -r sandbox_name; do
  if [ ! -z "$sandbox_name" ]; then
    echo "  Setting up proxy bypass for $sandbox_name..."
    docker sandbox network proxy "$sandbox_name" --bypass-host "host.docker.internal" 2>/dev/null || true
  fi
done

cd "/Volumes/Extra Storage Crucial 1TB SSD/Projects/Infrastructure/lap"
/usr/local/bin/docker-compose -f compose.yaml up -d
