#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${PROJECT_ROOT}/target/debian-build"
OUTPUT_DIR="${PROJECT_ROOT}/target/debian"
VERSION="0.1.0-1"
PKG_NAME="spectra_${VERSION}_amd64"

echo "=== Building Spectra Debian Package ==="

# Clean staging directory
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}/DEBIAN"
mkdir -p "${BUILD_DIR}/usr/bin"
mkdir -p "${BUILD_DIR}/usr/share/applications"
mkdir -p "${BUILD_DIR}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${BUILD_DIR}/usr/share/doc/spectra"
mkdir -p "${OUTPUT_DIR}"

# Copy control file
cp "${PROJECT_ROOT}/packaging/debian/control" "${BUILD_DIR}/DEBIAN/control"

# Copy binary
cp "${PROJECT_ROOT}/target/release/spectra" "${BUILD_DIR}/usr/bin/spectra"
chmod 755 "${BUILD_DIR}/usr/bin/spectra"

# Copy desktop file
cp "${PROJECT_ROOT}/packaging/debian/spectra.desktop" "${BUILD_DIR}/usr/share/applications/org.spectra.Spectra.desktop"

# Copy icon
cp "${PROJECT_ROOT}/packaging/debian/spectra.svg" "${BUILD_DIR}/usr/share/icons/hicolor/scalable/apps/spectra.svg"

# Copy documentation & license
cp "${PROJECT_ROOT}/LICENSE" "${BUILD_DIR}/usr/share/doc/spectra/copyright"
cp "${PROJECT_ROOT}/README.md" "${BUILD_DIR}/usr/share/doc/spectra/README.md"

# Build package
echo "Packaging into .deb..."
dpkg-deb --build --root-owner-group "${BUILD_DIR}" "${OUTPUT_DIR}/${PKG_NAME}.deb"

echo "=== Generated Debian Package ==="
ls -lh "${OUTPUT_DIR}/${PKG_NAME}.deb"

echo "=== Package Checksum (SHA-256) ==="
sha256sum "${OUTPUT_DIR}/${PKG_NAME}.deb" | tee "${OUTPUT_DIR}/${PKG_NAME}.deb.sha256"

echo "=== Package Inspection ==="
dpkg-deb -I "${OUTPUT_DIR}/${PKG_NAME}.deb"
dpkg-deb -c "${OUTPUT_DIR}/${PKG_NAME}.deb"

echo "=== Build Complete ==="
