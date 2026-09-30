#!/bin/bash
# Read the reported profile and current settings; no Set VCP commands.
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
exec python3 "$script_dir/read_current_profile.py" "$@"
