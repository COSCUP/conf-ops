#!/bin/bash
# backup-db.sh — PostgreSQL database backup
# Add to crontab for scheduled execution
set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-/data/backups/postgres}"
COMPOSE_FILE="${COMPOSE_FILE:-/opt/confops/docker-compose.prod.yml}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/confops_${TIMESTAMP}.sql.gz"

mkdir -p "${BACKUP_DIR}"

echo "[$(date)] Starting database backup..."

# Execute pg_dump inside the postgres container
docker compose -f "${COMPOSE_FILE}" exec -T postgres \
    pg_dump -U confops confops | gzip > "${BACKUP_FILE}"

BACKUP_SIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
echo "[$(date)] Backup created: ${BACKUP_FILE} (${BACKUP_SIZE})"

# Remove backups older than retention period
DELETED=$(find "${BACKUP_DIR}" -name "*.sql.gz" -mtime "+${RETENTION_DAYS}" -delete -print | wc -l)
if [ "${DELETED}" -gt 0 ]; then
    echo "[$(date)] Cleaned up ${DELETED} old backup(s)"
fi

# Optional: sync to remote backup location
# rsync -az "${BACKUP_DIR}/" backup-server:/backups/confops/postgres/
# rclone sync "${BACKUP_DIR}" remote:confops-backups/postgres/

echo "[$(date)] Database backup complete"
