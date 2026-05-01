#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# run_fuzz.sh — Convenience launcher for cargo-fuzz with sanitizer support.
#
# Usage:
#   ./run_fuzz.sh <target> [sanitizer] [extra cargo-fuzz args...]
#
# Examples:
#   # Basic run (no sanitizers)
#   ./run_fuzz.sh parse
#
#   # With ASAN (Address Sanitizer)
#   ./run_fuzz.sh parse address
#
#   # With UBSAN (Undefined Behavior Sanitizer)
#   ./run_fuzz.sh roundtrip undefined
#
#   # With MSAN (Memory Sanitizer — Linux only, nightly required)
#   ./run_fuzz.sh parse memory
#
#   # Pass extra arguments (e.g., -max_len, -dict)
#   ./run_fuzz.sh parse address -- -max_len=65536 -dict=dictionary/parse.dict
#
#   # Run with a specific corpus
#   ./run_fuzz.sh parse address corpus/parse
#
# Dictionaries are automatically attached for the "parse" target when
# dictionary/parse.dict exists.
# ---------------------------------------------------------------------------
set -euo pipefail

TARGET="${1:-parse}"
SANITIZER="${2:-}"

# Shift away TARGET and optional SANITIZER so $@ contains extra args
if [ -n "$SANITIZER" ]; then
    shift 2
else
    shift 1
fi

EXTRA_ARGS=("$@")

# If no extra args and default dictionary exists for parse, attach it
if [ "$TARGET" = "parse" ] && [ ${#EXTRA_ARGS[@]} -eq 0 ] && [ -f "dictionary/parse.dict" ]; then
    EXTRA_ARGS=("--" "-dict=dictionary/parse.dict")
fi

CMD=("cargo" "+nightly" "fuzz" "run" "--release" "$TARGET")

if [ -n "$SANITIZER" ]; then
    case "$SANITIZER" in
        address)
            export RUSTFLAGS="${RUSTFLAGS:-} -Zsanitizer=address"
            echo "[*] Running with ASAN (Address Sanitizer)"
            ;;
        undefined)
            export RUSTFLAGS="${RUSTFLAGS:-} -Zsanitizer=undefined"
            echo "[*] Running with UBSAN (Undefined Behavior Sanitizer)"
            ;;
        memory)
            export RUSTFLAGS="${RUSTFLAGS:-} -Zsanitizer=memory"
            echo "[*] Running with MSAN (Memory Sanitizer)"
            ;;
        *)
            echo "[!] Unknown sanitizer: $SANITIZER"
            echo "[!] Valid options: address, undefined, memory"
            exit 1
            ;;
    esac
fi

echo "[*] Target: $TARGET"
echo "[*] Command: ${CMD[*]} ${EXTRA_ARGS[*]}"
exec "${CMD[@]}" "${EXTRA_ARGS[@]}"