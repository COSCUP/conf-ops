#!/bin/bash
# deploy.sh — Production deployment script
# Run this on the production server
set -euo pipefail

DEPLOY_DIR="${DEPLOY_DIR:-/opt/confops}"
COMPOSE_FILE="docker-compose.prod.yml"

cd "${DEPLOY_DIR}"

echo "==> Pulling latest images..."
docker compose -f "${COMPOSE_FILE}" pull

echo "==> Starting services..."
docker compose -f "${COMPOSE_FILE}" up -d --remove-orphans

echo "==> Waiting for health check..."
sleep 5

# Check if the app is healthy
if docker compose -f "${COMPOSE_FILE}" exec -T confops-app curl -sf http://localhost:8080/healthz > /dev/null 2>&1; then
    echo "==> Health check passed"
else
    echo "==> WARNING: Health check failed. Check logs with:"
    echo "    docker compose -f ${COMPOSE_FILE} logs confops-app"
    exit 1
fi

echo "==> Cleaning up old images..."
docker image prune -f

echo "==> Deployment complete"
