#!/usr/bin/env bash

# Usage:
#   ./update_sunshine_to_latest.sh           # check for and install latest Sunshine release
#   ./update_sunshine_to_latest.sh -h|--help # show help

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

DEB_NAME="sunshine-ubuntu-24.04-amd64.deb"
DOWNLOAD_DIR="$HOME/Downloads"

CURRENT_VERSION=$(dpkg-query -W -f='${Version}' sunshine 2>/dev/null || echo "none")
echo "Current version: $CURRENT_VERSION"

echo "Fetching latest Sunshine release..."
RELEASE_JSON=$(curl -s https://api.github.com/repos/LizardByte/Sunshine/releases/latest)
LATEST_TAG=$(echo "$RELEASE_JSON" | python3 -c "import sys, json; print(json.loads(sys.stdin.read(), strict=False)['tag_name'])")
LATEST_VERSION="${LATEST_TAG#v}"

echo "Latest version:  $LATEST_VERSION"

if [ "$CURRENT_VERSION" = "$LATEST_VERSION" ]; then
    echo "Already up to date."
    exit 0
fi

DOWNLOAD_URL=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
data = json.loads(sys.stdin.read(), strict=False)
for a in data['assets']:
    if a['name'] == '${DEB_NAME}':
        print(a['browser_download_url'])
        break
")

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not find $DEB_NAME in latest release" >&2
    exit 1
fi

echo "Downloading: $DOWNLOAD_URL"
curl -L -o "$DOWNLOAD_DIR/$DEB_NAME" "$DOWNLOAD_URL"

echo "Installing..."
sudo dpkg -i "$DOWNLOAD_DIR/$DEB_NAME"

echo "Restarting Sunshine..."
systemctl --user restart sunshine

echo "Done!"
