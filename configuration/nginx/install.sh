#!/usr/bin/env bash
set -euo pipefail

ACME_WEBROOT="/var/lib/letsencrypt"
NGINX_CONF_DIR="/etc/nginx"
CERTBOT_HOOKS_DIR="/etc/letsencrypt/renewal-hooks/deploy"
DOMAIN="network.coworkinmontpellier.org"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$(id -u)" -ne 0 ]; then
    echo "Error: this script must be run as root." >&2
    exit 1
fi

if ! command -v nginx &>/dev/null; then
    echo "Error: nginx is not installed." >&2
    exit 1
fi

if ! command -v certbot &>/dev/null; then
    echo "Error: certbot is not installed." >&2
    exit 1
fi

echo "==> Ensuring certbot webroot exists..."
mkdir -p "${ACME_WEBROOT}"

echo "==> Installing nginx configuration..."
cp "${SCRIPT_DIR}/nginx.conf" "${NGINX_CONF_DIR}/nginx.conf"
mkdir -p "${NGINX_CONF_DIR}/conf.d"
cp "${SCRIPT_DIR}/conf.d/"*.conf "${NGINX_CONF_DIR}/conf.d/"

echo "==> Installing certbot renewal hook..."
mkdir -p "${CERTBOT_HOOKS_DIR}"
cp "${SCRIPT_DIR}/renewal-hooks/deploy/reload-nginx.sh" "${CERTBOT_HOOKS_DIR}/"
chmod +x "${CERTBOT_HOOKS_DIR}/reload-nginx.sh"

echo "==> Testing nginx configuration..."
nginx -t

echo "==> Enabling and starting nginx..."
systemctl enable --now nginx

if [ ! -f "/etc/letsencrypt/live/${DOMAIN}/fullchain.pem" ]; then
    echo "==> No certificate found for ${DOMAIN}, requesting one via certbot..."
    # Stop nginx temporarily so certbot standalone can bind port 80 for the first issuance
    systemctl stop nginx
    certbot certonly --standalone -d "${DOMAIN}" --agree-tos --non-interactive
    systemctl start nginx
else
    echo "==> Certificate already exists for ${DOMAIN}, switching renewal authenticator to webroot..."
    RENEWAL_CONF="/etc/letsencrypt/renewal/${DOMAIN}.conf"
    sed -i 's/^authenticator = .*/authenticator = webroot/' "${RENEWAL_CONF}"
    if ! grep -q "^webroot_path" "${RENEWAL_CONF}"; then
        sed -i "/^\[renewalparams\]/a webroot_path = ${ACME_WEBROOT}" "${RENEWAL_CONF}"
    else
        sed -i "s|^webroot_path = .*|webroot_path = ${ACME_WEBROOT}|" "${RENEWAL_CONF}"
    fi
    sed -i "/^webroot_map/d" "${RENEWAL_CONF}"
    echo "${DOMAIN} = ${ACME_WEBROOT}" >> "${RENEWAL_CONF}"
fi

echo "==> Reloading nginx..."
nginx -s reload

echo "==> Done. Status:"
systemctl status nginx --no-pager