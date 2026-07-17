set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

west build -b frdm_mcxa266 "$SCRIPT_DIR/samples/zephyr-odp" -d "$(west topdir)/build"