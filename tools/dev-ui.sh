#!/usr/bin/env bash
# One build, persistent native process, opt-in script_mod! hot reload.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ ! -d .runtime/makepad ]]; then
  echo 'Missing pinned runtime. Run: python3 tools/setup-runtime.py' >&2
  exit 1
fi
# Default to an isolated, persistent dev vault. Explicit override is possible.
export AGENT_DOCS_HOME="${AGENT_DOCS_HOME:-${TMPDIR:-/tmp}/docgenie-dev-vault}"
# Hot reload executes trusted local UI scripts. Do not grant model access here.
unset AGENT_DOCS_API_KEY MINIMAX_API_KEY
mkdir -p "$AGENT_DOCS_HOME"
echo "Dev vault: $AGENT_DOCS_HOME"
echo 'script_mod! edits reload in place; Rust logic edits require restart.'
exec cargo run --locked -p docgenie-desktop -- --hot "$@"
