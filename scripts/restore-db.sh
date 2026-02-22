#!/bin/bash
# restore-db.sh — Restore PostgreSQL database from backup
# Usage: ./restore-db.sh <backup_file.sql.gz>
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <backup_file.sql.gz>"
    echo ""
    echo "Available backups:"
    ls -lh "${BACKUP_DIR:-/data/backups/postgres}/"*.sql.gz 2>/dev/null || echo "  No backups found"
    exit 1
fi

BACKUP_FILE="$1"
COMPOSE_FILE="${COMPOSE_FILE:-/opt/confops/docker-compose.prod.yml}"

if [ ! -f "${BACKUP_FILE}" ]; then
    echo "ERROR: Backup file not found: ${BACKUP_FILE}"
    exit 1
fi

echo "WARNING: This will overwrite the current database!"
echo "Backup file: ${BACKUP_FILE}"
echo ""
read -p "Are you sure you want to proceed? (yes/no): " CONFIRM

if [ "${CONFIRM}" != "yes" ]; then
    echo "Aborted."
    exit 0
fi

echo "[$(date)] Stopping application services..."
docker compose -f "${COMPOSE_FILE}" stop confops-app confops-worker

echo "[$(date)] Dropping and recreating database..."
docker compose -f "${COMPOSE_FILE}" exec -T postgres \
    psql -U confops -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'confops' AND pid <> pg_backend_pid();" postgres || true

docker compose -f "${COMPOSE_FILE}" exec -T postgres \
    dropdb -U confops --if-exists confops

docker compose -f "${COMPOSE_FILE}" exec -T postgres \
    createdb -U confops confops

echo "[$(date)] Restoring from backup..."
gunzip -c "${BACKUP_FILE}" | docker compose -f "${COMPOSE_FILE}" exec -T postgres \
    psql -U confops confops

echo "[$(date)] Restarting application services..."
docker compose -f "${COMPOSE_FILE}" start confops-app confops-worker

echo "[$(date)] Waiting for health check..."
sleep 10

if docker compose -f "${COMPOSE_FILE}" exec -T confops-app curl -sf http://localhost:8080/healthz > /dev/null 2>&1; then
    echo "[$(date)] Restore complete. Application is healthy."
else
    echo "[$(date)] WARNING: Application health check failed after restore."
    echo "Check logs: docker compose -f ${COMPOSE_FILE} logs confops-app"
    exit 1
fi
