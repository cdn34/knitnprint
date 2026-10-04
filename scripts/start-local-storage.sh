#!/usr/bin/env bash
set -euo pipefail

# Native alternative when Docker Desktop cannot start containers.
project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
seaweedfs_bin="${SEAWEEDFS_BIN:-weed}"
seaweedfs_data_dir="${SEAWEEDFS_DATA_DIR:-$project_dir/.local/seaweedfs}"

if ! command -v "$seaweedfs_bin" >/dev/null 2>&1; then
  echo "Install SeaweedFS 4.48 or set SEAWEEDFS_BIN to its weed binary." >&2
  exit 1
fi

mkdir -p "$seaweedfs_data_dir"
export AWS_ACCESS_KEY_ID=knitnprint
export AWS_SECRET_ACCESS_KEY=knitnprint-local
export S3_BUCKET=knitnprint-media

exec "$seaweedfs_bin" mini \
  -dir="$seaweedfs_data_dir" \
  -ip=127.0.0.1 \
  -ip.bind=127.0.0.1 \
  -s3.port=9100 \
  -admin.port=9101 \
  -s3.allowedOrigins=http://127.0.0.1:3001,http://localhost:3001 \
  -s3.allowDeleteBucketNotEmpty=false \
  -s3.autoCreateBucket=false \
  -webdav=false \
  -s3.port.iceberg=0 \
  -s3.port.lance=0 \
  -admin.user=knitnprint \
  -admin.password=knitnprint-local
