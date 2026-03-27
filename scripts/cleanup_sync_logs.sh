#!/usr/bin/env bash

# Usage:
#   ./cleanup_sync_logs.sh            # delete sync logs older than 30 days
#   ./cleanup_sync_logs.sh --dry-run  # preview what would be deleted
#   ./cleanup_sync_logs.sh -h|--help  # show help

set -euo pipefail

LOG_DIR="$HOME/.local/share/sync/logs"
MAX_AGE_DAYS=30
DRY_RUN=0

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

if [ ! -d "$LOG_DIR" ]; then
  echo "Log directory not found: $LOG_DIR"
  exit 0
fi

if [ "$DRY_RUN" -eq 1 ]; then
  echo "Dry run — would delete:"
  find "$LOG_DIR" -name "*.log" -mtime +$MAX_AGE_DAYS -print
else
  DELETED=$(find "$LOG_DIR" -name "*.log" -mtime +$MAX_AGE_DAYS -delete -print | wc -l)
  echo "Deleted $DELETED log file(s) older than $MAX_AGE_DAYS days."
fi
