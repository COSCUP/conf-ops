#!/bin/bash
# backup-files.sh — File storage backup using rsync
set -euo pipefail

SOURCE_DIR="${SOURCE_DIR:-/data/confops/files}"
BACKUP_DIR="${BACKUP_DIR:-/data/backups/files}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo "[$(date)] Starting file storage backup..."

mkdir -p "${BACKUP_DIR}"

# Local backup using rsync
rsync -az --delete "${SOURCE_DIR}/" "${BACKUP_DIR}/"

echo "[$(date)] File storage backup complete to ${BACKUP_DIR}"

# Optional: sync to remote backup location
# rsync -az --delete "${SOURCE_DIR}/" backup-server:/backups/confops/files/
# rclone sync "${SOURCE_DIR}" remote:confops-backups/files/
