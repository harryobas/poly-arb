#!/usr/bin/env bash
set -euo pipefail

echo "🚀 Starting deployment on Hetzner VPS..."
#cd /root/arb-bot

# --- Validate required environment variables ---
if [[ -z "${PRIVATE_KEY:-}" ]]; then
    echo "❌ PRIVATE_KEY environment variable is not set"
    exit 1
fi

if [[ -z "${DOCKER_IMAGE:-}" ]]; then
    echo "❌ DOCKER_IMAGE environment variable is not set"
    exit 1
fi

# --- Secure Docker secret creation ---
echo "🔐 Updating Docker secret 'private_key'..."
TEMP_KEY_FILE=$(mktemp)
echo "$PRIVATE_KEY" > "$TEMP_KEY_FILE"

if docker secret inspect private_key >/dev/null 2>&1; then
    docker secret rm private_key || echo "⚠️  Could not remove old secret, continuing..."
fi

docker secret create private_key "$TEMP_KEY_FILE"
shred -u "$TEMP_KEY_FILE"
echo "✅ Docker secret updated successfully"

# --- Deploy new container ---
echo "🐳 Pulling latest image: $DOCKER_IMAGE"
docker pull "$DOCKER_IMAGE"

echo "🧹 Stopping old container..."
docker compose down --remove-orphans --timeout 30 || true

echo "🚀 Starting updated container..."
docker compose up -d

# --- Health check ---
echo "⏳ Waiting for container to start..."
sleep 5

CONTAINER_NAME=$(docker compose ps --services | head -1)

if [[ -z "$CONTAINER_NAME" ]]; then
    echo "❌ Could not determine container name"
    docker compose logs
    exit 1
fi

if docker compose ps | grep -q "Up"; then
    echo "✅ Container started successfully!"
    echo "📊 Container status:"
    docker compose ps
    
    echo "📋 Recent logs:"
    docker compose logs --tail=20
    
    # Optional: Interactive attachment (commented out for automation)
    # echo "🎛️  To attach to TUI: docker attach $CONTAINER_NAME"
    # echo "📤 To detach safely: Ctrl+P, Ctrl+Q"
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