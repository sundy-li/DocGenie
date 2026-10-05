#!/usr/bin/env bash
# makepad-test calls cargo build --release internally. Only the test process
# receives this adapter; the pinned runtime and ordinary builds are unchanged.
set -euo pipefail
: "${DOCGENIE_REAL_CARGO:?set by test-ui-debug.py}"
args=()
for arg in "$@"; do
    [[ "$arg" == "--release" ]] || args+=("$arg")
done
exec "$DOCGENIE_REAL_CARGO" "${args[@]}"
