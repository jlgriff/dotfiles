#!/usr/bin/env bash

# Usage:
#   ./rclone_pull.sh              # sync remote → local (overwrites local to match remote)
#   ./rclone_pull.sh --dry-run    # preview changes without applying them
#   ./rclone_pull.sh -h|--help    # show help

set -euo pipefail

ENV_FILE="$HOME/.dotfiles.env"
if [ ! -f "$ENV_FILE" ]; then
  echo "Error: $ENV_FILE not found. See README.md for setup instructions." >&2
  exit 1
fi
# shellcheck source=/dev/null
source "$ENV_FILE"

if [ -z "${CLOUD_REMOTE:-}" ] || [ -z "${CLOUD_LOCAL_DIR:-}" ]; then
  echo "Error: CLOUD_REMOTE and CLOUD_LOCAL_DIR must be set in $ENV_FILE" >&2
  exit 1
fi

LOCAL_DIR="$HOME/$CLOUD_LOCAL_DIR"
REMOTE_DIR="$CLOUD_REMOTE"
DRY_RUN=0

LOG_DIR="$HOME/.local/share/sync/logs"
mkdir -p "$LOG_DIR"

# ---------- Args ----------
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    -h|--help)
      sed -n '1,30p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      printf 'Unknown option: %s\n' "$arg" >&2
      exit 2
      ;;
  esac
done

# ---------- Helpers ----------
RUN_TS="$(date '+%Y%m%d_%H%M%S')"
LOG_FILE="$LOG_DIR/pull_${RUN_TS}.log"

if [ -t 0 ]; then
  RUN_MODE="manual"
else
  RUN_MODE="cron"
fi

log()   { echo "$(date '+%Y-%m-%d %H:%M:%S') [$1] $2" | tee -a "$LOG_FILE"; }
info()  { log INFO "$*"; }
error() { log ERROR "$*" >&2; }

LOCKFILE="$HOME/.local/share/sync/.rclone_pull.lock"
cleanup() { rm -f "$LOCKFILE"; }
trap 'error "Unexpected error at line $LINENO"; cleanup' ERR
trap cleanup EXIT INT TERM

if [ -f "$LOCKFILE" ]; then
  OLD_PID=$(cat "$LOCKFILE")
  if kill -0 "$OLD_PID" 2>/dev/null; then
    error "Another instance is running (PID $OLD_PID). Exiting."
    exit 1
  fi
  info "Removing stale lockfile (PID $OLD_PID no longer running)."
  rm -f "$LOCKFILE"
fi
echo $$ > "$LOCKFILE"

# ---------- Sync ----------
mkdir -p "$LOCAL_DIR"

RCLONE_ARGS=(
  "$REMOTE_DIR" "$LOCAL_DIR"
  --log-file="$LOG_FILE"
  --log-level NOTICE
  --stats 1m
  --stats-log-level NOTICE
)

if [ "$DRY_RUN" -eq 1 ]; then
  RCLONE_ARGS+=(--dry-run)
  info "START ($RUN_MODE, dry-run): rclone sync $REMOTE_DIR → $LOCAL_DIR"
else
  info "START ($RUN_MODE): rclone sync $REMOTE_DIR → $LOCAL_DIR"
fi

if ! rclone sync "${RCLONE_ARGS[@]}"; then
  error "FAILED: Sync failed. Check log at $LOG_FILE"
  exit 1
fi

info "DONE: Sync complete."
