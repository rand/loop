#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  cat <<'USAGE' >&2
Usage: scripts/safe_run.sh <command> [args...]

Runs a potentially heavy command only if:
- no other heavy command is running (unless explicitly allowed), and
- minimum available memory threshold is met.

Environment variables:
  LOOP_MIN_AVAILABLE_MIB     Minimum available memory required (default: 2048)
  LOOP_ALLOW_PARALLEL_HEAVY  Set to 1 to allow concurrent heavy jobs (default: 0)
  LOOP_HEAVY_LOCK_FILE       Lock file path (default: /tmp/loop-heavy.lock)
USAGE
  exit 64
fi

MIN_AVAILABLE_MIB="${LOOP_MIN_AVAILABLE_MIB:-2048}"
ALLOW_PARALLEL_HEAVY="${LOOP_ALLOW_PARALLEL_HEAVY:-0}"
LOCK_FILE="${LOOP_HEAVY_LOCK_FILE:-/tmp/loop-heavy.lock}"
LOCK_DIR="${LOCK_FILE}.d"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "safe_run: required command '$1' is not available" >&2
    exit 69
  fi
}

require_cmd vm_stat
require_cmd awk

get_page_size() {
  vm_stat | awk -F'[()]' '/page size/ { gsub(/[^0-9]/, "", $2); print $2; exit }'
}

get_pages_for_label() {
  local label="$1"
  vm_stat | awk -F: -v target="$label" '
    $1 ~ target {
      gsub(/[^0-9]/, "", $2)
      print $2
      exit
    }
  '
}

get_pages_or_zero() {
  local label="$1"
  local value
  value="$(get_pages_for_label "$label")"
  if [ -z "$value" ]; then
    echo "0"
  else
    echo "$value"
  fi
}

to_mib() {
  local pages="$1"
  local page_size="$2"
  awk -v p="$pages" -v s="$page_size" 'BEGIN { printf "%d", (p * s) / 1048576 }'
}

is_heavy_cmd() {
  local cmdline="$1"
  case "$cmdline" in
    *"cargo check"*|*"cargo test"*|*"rustc"*|*"pytest"*|*"uv run pytest"*|*"maturin"*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

COMMAND_STRING="$*"
HEAVY=0
if is_heavy_cmd "$COMMAND_STRING"; then
  HEAVY=1
fi

if [ "$HEAVY" -eq 1 ]; then
  if [ "$ALLOW_PARALLEL_HEAVY" != "1" ]; then
    mkdir -p "$(dirname "$LOCK_FILE")"
    if ! mkdir "$LOCK_DIR" 2>/dev/null; then
      OWNER_PID=""
      if [ -f "$LOCK_DIR/pid" ]; then
        OWNER_PID="$(cat "$LOCK_DIR/pid" 2>/dev/null || true)"
      fi

      if [ -n "$OWNER_PID" ] && ! kill -0 "$OWNER_PID" 2>/dev/null; then
        rm -f "$LOCK_DIR/pid" >/dev/null 2>&1 || true
        rmdir "$LOCK_DIR" 2>/dev/null || true
        if ! mkdir "$LOCK_DIR" 2>/dev/null; then
          echo "safe_run: lock recovery failed for $LOCK_FILE; aborting" >&2
          exit 75
        fi
      else
        echo "safe_run: another heavy command holds lock $LOCK_FILE; aborting" >&2
        exit 75
      fi
    fi
    echo "$$" >"$LOCK_DIR/pid"
    trap 'rm -f "$LOCK_DIR/pid" >/dev/null 2>&1 || true; rmdir "$LOCK_DIR" >/dev/null 2>&1 || true' EXIT
  fi

  PAGE_SIZE="$(get_page_size)"
  FREE_PAGES="$(get_pages_or_zero '^Pages free')"
  INACTIVE_PAGES="$(get_pages_or_zero '^Pages inactive')"
  SPECULATIVE_PAGES="$(get_pages_or_zero '^Pages speculative')"
  PURGEABLE_PAGES="$(get_pages_or_zero '^Pages purgeable')"

  if [ -z "$PAGE_SIZE" ]; then
    echo "safe_run: unable to read vm_stat metrics; aborting for safety" >&2
    exit 70
  fi

  AVAILABLE_PAGES="$((FREE_PAGES + INACTIVE_PAGES + SPECULATIVE_PAGES + PURGEABLE_PAGES))"
  AVAILABLE_MIB="$(to_mib "$AVAILABLE_PAGES" "$PAGE_SIZE")"
  if [ "$AVAILABLE_MIB" -lt "$MIN_AVAILABLE_MIB" ]; then
    echo "safe_run: available memory ${AVAILABLE_MIB}MiB is below threshold ${MIN_AVAILABLE_MIB}MiB; aborting" >&2
    exit 75
  fi

  echo "safe_run: heavy command admitted (available=${AVAILABLE_MIB}MiB, threshold=${MIN_AVAILABLE_MIB}MiB)" >&2
fi

exec "$@"
