#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="/opt/coworker-portal"
ENV_FILE="/etc/coworker-portal/env"
SERVICE_NAME="coworker-portal"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_DIR="$(dirname "$SCRIPT_DIR")"

if [ "$(id -u)" -ne 0 ]; then
    echo "Error: this script must be run as root." >&2
    exit 1
fi

if [ ! -f "${ENV_FILE}" ]; then
    echo "Error: env file not found at ${ENV_FILE}." >&2
    echo "       Create it with the required variables before running this script:" >&2
    echo "         DATABASE_URL, JWT_SECRET, LISTEN_ADDR, BILL_ISSUER_ADDRESS," >&2
    echo "         GUEST_USER_ID, DJANGO_BASE_URL, DJANGO_SUPERUSER_USERNAME," >&2
    echo "         DJANGO_SUPERUSER_PASSWORD, UNIFY_BASE_URL, UNIFY_USERNAME," >&2
    echo "         UNIFY_PASSWORD, UNIFY_SITE" >&2
    exit 1
fi

echo "==> Creating user and directories..."
if ! id "${SERVICE_NAME}" &>/dev/null; then
    useradd -r -s /sbin/nologin "${SERVICE_NAME}"
fi
mkdir -p "${INSTALL_DIR}" "$(dirname "${ENV_FILE}")"

echo "==> Installing binary and assets..."
cp "${RELEASE_DIR}/coworker-portal" "${INSTALL_DIR}/"
cp -r "${RELEASE_DIR}/public" "${INSTALL_DIR}/public"
chown -R "${SERVICE_NAME}:${SERVICE_NAME}" "${INSTALL_DIR}"

echo "==> Installing systemd unit..."
cp "${SCRIPT_DIR}/coworker-portal.service" /etc/systemd/system/
systemctl daemon-reload

echo "==> Enabling and starting service..."
systemctl enable --now "${SERVICE_NAME}"

echo "==> Done. Status:"
systemctl status "${SERVICE_NAME}" --no-pager