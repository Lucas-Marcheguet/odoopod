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

if [ -z "$TAG" ]; then
  echo "❌ Impossible de trouver la dernière release."
  exit 1
fi

echo "🚀 Téléchargement de ${BINARY_NAME} ${TAG}..."
curl -L -o "${BINARY_NAME}" "${DOWNLOAD_URL}"
curl -L -o "${BINARY_NAME}.sha256" "${DOWNLOAD_URL}.sha256"

echo "🛡️ Vérification du checksum..."
if [ "${OS}" = "darwin" ]; then
  # macOS utilise shasum
  shasum -a 256 -c "${BINARY_NAME}.sha256"
else
  # Linux utilise sha256sum (format de sortie différent souvent requis)
  # On extrait le hash du fichier généré par le workflow
  EXPECTED_HASH=$(awk '{print $1}' "${BINARY_NAME}.sha256")
  echo "${EXPECTED_HASH}  ${BINARY_NAME}" | sha256sum -c -
fi

echo "📦 Installation dans ${INSTALL_DIR}..."
chmod +x "${BINARY_NAME}"
sudo mv "${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

# Nettoyage
rm "${BINARY_NAME}.sha256"

echo "✅ Installation terminée ! Tapez '${BINARY_NAME}' pour commencer."