#!/bin/bash
# Flash script for the zephyr-odp sample on the mimxrt685_evk board.
# On WSL it copies the ELF to Windows and flashes via the Windows probe-rs
# (works around WSL USB passthrough issues). On native Linux it flashes
# directly with the local probe-rs.
# You'll need probe-rs installed (on Windows for WSL, locally for native Linux).
#
# Run from anywhere inside the west workspace; the build dir is resolved
# relative to the workspace top level (override with the BUILD_DIR env var).

set -e

# Detect whether we're running under WSL or native Linux.
if [ -n "$WSL_DISTRO_NAME" ] || grep -qiE "(microsoft|wsl)" /proc/version 2>/dev/null; then
    IS_WSL=1
    echo "Detected WSL."
else
    IS_WSL=0
    echo "Detected Native Linux."
fi

# probe-rs chip target (override with the CHIP env var if needed).
CHIP="${CHIP:-MIMXRT685SFVKB}"
WS="$(west topdir)"
BUILD_DIR="${BUILD_DIR:-$WS/build}"
BINARY="$BUILD_DIR/zephyr/zephyr.elf"
WINDOWS_TEMP="C:\\temp"
WSL_WINDOWS_TEMP="/mnt/c/temp"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY"
    echo "Run 'west build' first, or set BUILD_DIR environment variable"
    exit 1
fi

if [ "$IS_WSL" -eq 1 ]; then
    # Copy binary to Windows temp directory
    echo "Copying binary to Windows..."
    mkdir -p "$WSL_WINDOWS_TEMP"
    cp "$BINARY" "$WSL_WINDOWS_TEMP/zephyr.elf"

    # Flash using Windows probe-rs
    echo "Flashing via Windows probe-rs..."
    powershell.exe -c "probe-rs run --chip $CHIP $WINDOWS_TEMP\\zephyr.elf"
else
    # Flash directly with the local probe-rs
    echo "Flashing via probe-rs..."
    probe-rs run --chip "$CHIP" "$BINARY"
fi

echo "Done!"
