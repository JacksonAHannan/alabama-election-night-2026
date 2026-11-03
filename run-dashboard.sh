#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
cargo build --locked
exec target/debug/alabama-election-night serve --port 8052 "$@"
