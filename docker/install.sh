#!/usr/bin/env sh
SYSTEM_ARCH="$(uname -m)"
CORVUS_VERSION="$1"
CORVUS_FILENAME="corvus-${SYSTEM_ARCH}-unknown-linux-musl.tar.xz"
CORVUS_URL="https://github.com/proctorlabs/corvus/releases/download/${CORVUS_VERSION}/${CORVUS_FILENAME}"

mkdir -p /dist
apk add curl ca-certificates tar xz

echo "Downloading package from ${CORVUS_URL}"
curl -sL "${CORVUS_URL}" | tar xJ -C /dist
