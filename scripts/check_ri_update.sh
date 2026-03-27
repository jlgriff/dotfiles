#!/usr/bin/env bash

# Usage:
#   ./check_ri_update.sh           # check for and download latest Realism Invictus release
#   ./check_ri_update.sh -h|--help # show help

set -euo pipefail

for arg in "$@"; do
  case "$arg" in
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

VERSION_FILE="$HOME/.ri_version"
DOWNLOAD_DIR="$HOME/Downloads"
PROJECT_URL="https://sourceforge.net/projects/civ4mods/best_release.json"

echo "Fetching latest Realism Invictus release..."
RELEASE_JSON=$(curl -s "$PROJECT_URL")

FILENAME=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data['release']['filename'].split('/')[-1])
")

DOWNLOAD_URL=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data['release']['url'])
")

REMOTE_MD5=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data['release']['md5sum'])
")

# Parse version and build date from filename
# e.g. "Realism Invictus 3.81 (2026-02-08) Setup (Full).exe"
LATEST_VERSION=$(echo "$FILENAME" | python3 -c "
import sys, re
m = re.match(r'Realism Invictus ([0-9.]+[a-z]?) \(([0-9-]+)\) Setup \(Full\)\.exe', sys.stdin.read().strip())
if not m:
    print('error')
    sys.exit(1)
print(f'{m.group(1)} ({m.group(2)})')
")

if [ "$LATEST_VERSION" = "error" ]; then
    echo "Error: Could not parse version from filename: $FILENAME" >&2
    exit 1
fi

echo "Latest version:  $LATEST_VERSION"

CURRENT_VERSION="none"
if [ -f "$VERSION_FILE" ]; then
    CURRENT_VERSION=$(cat "$VERSION_FILE")
fi
echo "Current version: $CURRENT_VERSION"

if [ "$CURRENT_VERSION" = "$LATEST_VERSION" ]; then
    echo "Already up to date."
    exit 0
fi

echo "Downloading: $FILENAME"
curl -L -o "$DOWNLOAD_DIR/$FILENAME" "$DOWNLOAD_URL"

echo "Verifying checksum..."
LOCAL_MD5=$(md5sum "$DOWNLOAD_DIR/$FILENAME" | awk '{print $1}')
if [ "$LOCAL_MD5" != "$REMOTE_MD5" ]; then
    echo "Error: MD5 mismatch (expected $REMOTE_MD5, got $LOCAL_MD5)" >&2
    rm -f "$DOWNLOAD_DIR/$FILENAME"
    exit 1
fi

echo "$LATEST_VERSION" > "$VERSION_FILE"
echo "Downloaded to: $DOWNLOAD_DIR/$FILENAME"
echo "Run the installer manually via Proton to update."
