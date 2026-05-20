#!/bin/bash

set -euo pipefail
IFS=$'\n'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

(cd "$SCRIPT_DIR/../.." && cargo build)
EXE="$SCRIPT_DIR/../../target/debug/rekordcrate"

prev_file=""
for file in "$SCRIPT_DIR"/*/export.pdb; do
    if ! [[ "$prev_file" ]]; then
        prev_file="$file"
        continue
    fi
    echo "----------------------------------------"
    echo "Comparing $prev_file and $file" >&2
    diff -u <("$EXE" dump-pdb "$prev_file") <("$EXE" dump-pdb "$file") || true
    read -p "Press Enter to continue..."
    prev_file="$file"
done
