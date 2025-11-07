#!/usr/bin/env bash
set -euo pipefail

echo "🚀 Starting deployment on Hetzner VPS..."

# --- Validate required environment variables ---
if [[ -z "${PRIVATE_KEY:-}" ]]; then
    echo "❌ PRIVATE_KEY environment variable is not set"
    exit 1
fi

if [[ -z "${DOCKER_IMAGE:-}" ]]; then
    echo "❌ DOCKER_IMAGE environment variable is not set"
    exit 1
fi

# Add RPC_URL validation if your bot requires it
if [[ -z "${RPC_URL:-}" ]]; then
    echo "❌ RPC_URL environment variable is not set"
    exit 1
fi

echo "✅ All required environment variables are set"

# --- Deploy new container ---
echo "🐳 Pulling latest image: $DOCKER_IMAGE"
if ! docker pull "$DOCKER_IMAGE"; then
    echo "❌ Failed to pull Docker image"
    exit 1
fi

echo "🧹 Stopping old container..."
docker compose down --remove-orphans --timeout 30 || true

echo "🚀 Starting updated container..."
if ! docker compose up -d; then
    echo "❌ Failed to start containers with docker compose"
    exit 1
fi

# --- Health check ---
echo "⏳ Waiting for container to start..."
sleep 10  # Increased sleep for more reliable startup

CONTAINER_NAME=$(docker compose ps --services | head -1)

if [[ -z "$CONTAINER_NAME" ]]; then
    echo "❌ Could not determine container name"
    docker compose logs
    exit 1
fi

# More specific health check
if docker compose ps "$CONTAINER_NAME" | grep -q "Up"; then
    echo "✅ Container started successfully!"
    echo "📊 Container status:"
    docker compose ps
    
    echo "📋 Recent logs:"
    docker compose logs --tail=20
    
    # Verify environment variables are set in container (optional)
    echo "🔍 Verifying environment variables in container..."
    if docker exec "$CONTAINER_NAME" printenv RPC_URL >/dev/null 2>&1; then
        echo "✅ RPC_URL is set in container"
    else
        echo "⚠️  RPC_URL not found in container environment"
    fi
    
else
    echo "❌ Container failed to start. Check logs:"
    docker compose logs
    exit 1
fi

echo "🎯 Deployment complete! Container is running in detached mode."
echo "💡 Commands:"
echo "   docker compose logs -f     # Follow logs"
echo "   docker attach $CONTAINER_NAME  # Attach to TUI"
echo "   To detach safely: Ctrl+P, Ctrl+Q"
echo "   docker compose down        # Stop container"