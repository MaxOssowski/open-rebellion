#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# --own-data: the public build. It ships only the engine (page, loader,
# WASM): the player picks their own copy of the game in the browser
# (docs/plans/2026-11-24-bring-your-own-data.md), so no original game data
# leaves this machine.
OWN_DATA=0
ARGS=()
for arg in "$@"; do
    if [ "$arg" = "--own-data" ]; then
        OWN_DATA=1
    else
        ARGS+=("$arg")
    fi
done
VERSION="${ARGS[0]:-dev}"
if [[ ! "${VERSION}" =~ ^[A-Za-z0-9._-]+$ ]]; then
    echo "ERROR: version may contain only letters, numbers, dots, underscores, and hyphens."
    exit 1
fi
echo "Packaging Open Rebellion web build (v${VERSION})..."

# Build WASM
bash "$ROOT/scripts/build-wasm.sh"

# Create distribution directory
NAME="open-rebellion-web-${VERSION}"
if [ "$OWN_DATA" = 1 ]; then
    NAME="open-rebellion-web-own-data-${VERSION}"
fi
DIST="dist/${NAME}"
rm -rf "${DIST}"
mkdir -p "${DIST}"

# Copy web assets
cp web/index.html "${DIST}/"
cp web/gl.js "${DIST}/"
cp web/open-rebellion.wasm "${DIST}/"

if [ "$OWN_DATA" = 1 ]; then
    # Refuse anything beyond the engine: the page, its loader and the WASM.
    extra="$(cd "${DIST}" && find . -type f ! -name index.html ! -name gl.js ! -name open-rebellion.wasm)"
    if [ -n "$extra" ]; then
        echo "ERROR: the own-data build holds more than the engine: ${extra}"
        exit 1
    fi
else
    # Runtime data is required by the WASM loading screen. Ship the deterministic
    # pack rather than thousands of loose files; the original game data remains
    # local, generated, and intentionally untracked.
    if [ ! -f web/data/runtime.orpk ]; then
        echo "ERROR: scripts/build-wasm.sh did not create web/data/runtime.orpk."
        exit 1
    fi
    mkdir -p "${DIST}/data"
    cp web/data/runtime.orpk "${DIST}/data/"
    # The cutscenes load on demand, outside the pack (index.html's <video>).
    if ls web/data/video/*.webm >/dev/null 2>&1; then
        mkdir -p "${DIST}/data/video"
        cp web/data/video/*.webm "${DIST}/data/video/"
    fi

    # Refuse to create an artifact that can compile but cannot boot.
    if [ ! -s "${DIST}/data/runtime.orpk" ]; then
        echo "ERROR: packaged browser runtime pack is missing or empty."
        exit 1
    fi
fi

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi
}

# Record hashes for every shipped runtime file so release and deployment
# verification can prove which data and code the browser loaded.
(
    cd "${DIST}"
    find . -type f ! -name SHA256SUMS -print \
        | LC_ALL=C sort \
        | while IFS= read -r file; do sha256 "${file}"; done \
        > SHA256SUMS
)

# Create zip
ZIP="${ROOT}/dist/${NAME}.zip"
rm -f "${ZIP}"
cd dist
if command -v zip >/dev/null 2>&1; then
    zip -rq "${NAME}.zip" "${NAME}/"
else
    python3 -c 'import shutil, sys; shutil.make_archive(sys.argv[1], "zip", ".", sys.argv[1])' "${NAME}"
fi
cd ..

echo "Created: ${ZIP}"
ls -lh "${ZIP}"
