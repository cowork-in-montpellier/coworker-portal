#!/usr/bin/env bash
set -euo pipefail

TARGET="aarch64-unknown-linux-gnu"
BINARY_NAME="coworker-portal"
ARCHIVE_NAME="${BINARY_NAME}-${TARGET}.tar.gz"

echo "==> Building frontend (clean)..."
rm -rf frontend/dist
(cd frontend && npm install && npm run build)

echo "==> Copying frontend dist for archive..."
rm -rf public
cp -r frontend/dist public

echo "==> Cross-compiling for ${TARGET}..."
SKIP_FRONTEND_BUILD=1 cross build --release --target "${TARGET}"

echo "==> Creating release archive..."
STAGING=$(mktemp -d)
trap 'rm -rf "$STAGING"' EXIT

NGINX_DIR="${STAGING}/nginx"

mkdir -p "${NGINX_DIR}"

cp "target/${TARGET}/release/${BINARY_NAME}" "${STAGING}/"
cp -r public "${STAGING}/public"
cp -r install "${STAGING}/install"

cp -r configuration/nginx/. "${NGINX_DIR}/"

tar -czf "${ARCHIVE_NAME}" -C "${STAGING}" .

echo "==> Done: ${ARCHIVE_NAME}"
