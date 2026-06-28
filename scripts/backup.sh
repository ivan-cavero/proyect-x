#!/bin/bash
# backup.sh — Backup project-x data to S3/MinIO

set -e

BACKUP_DIR="/tmp/project-x-backup-$(date +%Y%m%d-%H%M%S)"
DATA_DIR="${PROJECT_X_DATA_DIR:-./data}"

echo "📦 Starting backup..."

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Copy data files
if [ -d "$DATA_DIR" ]; then
    echo "  Copying data..."
    cp -r "$DATA_DIR" "$BACKUP_DIR/"
fi

# Copy config
if [ -d "./config" ]; then
    echo "  Copying config..."
    cp -r "./config" "$BACKUP_DIR/"
fi

# Compress
echo "  Compressing..."
tar -czf "$BACKUP_DIR.tar.gz" -C "$(dirname $BACKUP_DIR)" "$(basename $BACKUP_DIR)"

# Upload to S3 (if configured)
if [ -n "$S3_BUCKET" ]; then
    echo "  Uploading to S3..."
    aws s3 cp "$BACKUP_DIR.tar.gz" "s3://$S3_BUCKET/backups/"
    echo "  Uploaded to s3://$S3_BUCKET/backups/"
fi

# Cleanup
rm -rf "$BACKUP_DIR"

echo "✅ Backup complete: $BACKUP_DIR.tar.gz"