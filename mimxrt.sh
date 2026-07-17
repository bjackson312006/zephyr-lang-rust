#!/bin/bash
# Quick build of the zephyr-odp sample for the MIMXRT685-EVK.
# Run from anywhere inside the west workspace; the app path is resolved
# relative to this script and the build dir to the workspace top level.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

west build -b mimxrt685_evk/mimxrt685s/cm33 "$SCRIPT_DIR/samples/zephyr-odp" -d "$(west topdir)/build"