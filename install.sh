#!/bin/bash
set -e

# Configuration
REPO="Lucas-Marcheguet/odoopod"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="odoopod"

echo "🔍 Détection du système..."

# Détection de l'OS et de l'architecture
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${OS}" in
  linux*)   ASSET_SUFFIX="x86_64-unknown-linux-gnu" ;;
  darwin*)  ASSET_SUFFIX="universal-apple-darwin" ;;
  *)        echo "❌ OS non supporté : ${OS}"; exit 1 ;;
esac

echo "🌐 Récupération de la dernière version..."
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest")
TAG=$(echo "${LATEST_RELEASE}" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${BINARY_NAME}-${ASSET_SUFFIX}"
CHECKSUM_FILE="${BINARY_NAME}.sha256"

if [ -z "$TAG" ]; then
  echo "❌ Impossible de trouver la dernière release."
  exit 1
fi

echo "🚀 Téléchargement de ${BINARY_NAME} ${TAG}..."
curl -L -o "${BINARY_NAME}" "${DOWNLOAD_URL}"

if ! curl -fL -o "${CHECKSUM_FILE}" "${DOWNLOAD_URL}.sha256"; then
  echo "ℹ️ Fichier .sha256 introuvable, tentative avec .sha..."
  curl -fL -o "${CHECKSUM_FILE}" "${DOWNLOAD_URL}.sha"
fi

echo "🛡️ Vérification du checksum..."
EXPECTED_HASH=$(awk '{print $1}' "${CHECKSUM_FILE}")

if [ -z "${EXPECTED_HASH}" ]; then
  echo "❌ Checksum invalide ou introuvable dans ${CHECKSUM_FILE}"
  exit 1
fi

if [ "${OS}" = "darwin" ]; then
  ACTUAL_HASH=$(shasum -a 256 "${BINARY_NAME}" | awk '{print $1}')
else
  ACTUAL_HASH=$(sha256sum "${BINARY_NAME}" | awk '{print $1}')
fi

if [ "${EXPECTED_HASH}" != "${ACTUAL_HASH}" ]; then
  echo "❌ Checksum invalide: attendu ${EXPECTED_HASH}, obtenu ${ACTUAL_HASH}"
  exit 1
fi

echo "📦 Installation dans ${INSTALL_DIR}..."
chmod +x "${BINARY_NAME}"
sudo mv "${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

rm "${CHECKSUM_FILE}"

echo "✅ Installation terminée ! Tapez '${BINARY_NAME}' pour commencer."