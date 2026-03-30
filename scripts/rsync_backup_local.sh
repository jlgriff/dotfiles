#!/usr/bin/env bash

# Usage:
#   ./rsync_backup_local.sh            # backup local cloud drive → USB
#   ./rsync_backup_local.sh --dry-run  # preview changes without applying them
#   ./rsync_backup_local.sh -h|--help  # show help

set -euo pipefail

ENV_FILE="$HOME/.dotfiles.env"
if [ ! -f "$ENV_FILE" ]; then
  echo "Error: $ENV_FILE not found. See README.md for setup instructions." >&2
  exit 1
fi
# shellcheck source=/dev/null
source "$ENV_FILE"

if [ -z "${USB_DRIVE_NAME:-}" ] || [ -z "${CLOUD_LOCAL_DIR:-}" ]; then
  echo "Error: USB_DRIVE_NAME and CLOUD_LOCAL_DIR must be set in $ENV_FILE" >&2
  exit 1
fi

SRC="$HOME/$CLOUD_LOCAL_DIR"
DEST="/media/$(whoami)/$USB_DRIVE_NAME/$CLOUD_LOCAL_DIR"
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
LOG_FILE="$LOG_DIR/backup_local_${RUN_TS}.log"

if [ -t 0 ]; then
  RUN_MODE="manual"
else
  RUN_MODE="cron"
fi

JOB_NAME="backup_local"
ERROR_REPORT="$HOME/Desktop/sync_errors.csv"

log()   { echo "$(date '+%Y-%m-%d %H:%M:%S') [$1] $2" | tee -a "$LOG_FILE"; }
info()  { log INFO "$*"; }
error() { log ERROR "$*" >&2; }

report_error() {
  if [ ! -f "$ERROR_REPORT" ]; then
    printf 'Timestamp,Job,Mode,Error,Log\n' > "$ERROR_REPORT"
  fi
  printf '%s,%s,%s,"%s",%s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$JOB_NAME" "$RUN_MODE" "$1" "$LOG_FILE" >> "$ERROR_REPORT"
}

fatal() {
  error "$*"
  report_error "$*"
  exit 1
}

LOCKFILE="$HOME/.local/share/sync/.rsync_backup_local.lock"
cleanup() { rm -f "$LOCKFILE"; }
trap 'error "Unexpected error at line $LINENO"; report_error "Unexpected error at line $LINENO"; cleanup' ERR
trap cleanup EXIT INT TERM

if [ -f "$LOCKFILE" ]; then
  OLD_PID=$(cat "$LOCKFILE")
  if kill -0 "$OLD_PID" 2>/dev/null; then
    fatal "Another instance is running (PID $OLD_PID). Exiting."
  fi
  info "Removing stale lockfile (PID $OLD_PID no longer running)."
  rm -f "$LOCKFILE"
fi
echo $$ > "$LOCKFILE"

# ---------- Preflight ----------
if [ ! -d "$SRC" ]; then
  fatal "Source directory not found: $SRC"
fi

if ! mountpoint -q "$(dirname "$DEST")"; then
  fatal "USB not mounted: $(dirname "$DEST")"
fi

# ---------- Sync ----------
RSYNC_ARGS=(-a --no-perms --no-owner --no-group --delete --log-file="$LOG_FILE")

if [ "$DRY_RUN" -eq 1 ]; then
  RSYNC_ARGS+=(--dry-run)
  info "START ($RUN_MODE, dry-run): rsync $SRC → $DEST"
else
  info "START ($RUN_MODE): rsync $SRC → $DEST"
fi

if ! rsync "${RSYNC_ARGS[@]}" "$SRC/" "$DEST/"; then
  fatal "FAILED: Sync failed. Check log at $LOG_FILE"
fi

info "DONE: Sync complete."
