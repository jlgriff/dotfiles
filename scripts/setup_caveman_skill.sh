#!/usr/bin/env bash
# Usage:
#   ./setup_caveman_skill.sh            # install/update the caveman skill into ~/.claude
#   ./setup_caveman_skill.sh --dry-run  # show what would be fetched/written, change nothing
#   ./setup_caveman_skill.sh -h|--help  # show help
#
# Installs the "caveman" skill (ultra-compressed /caveman response mode) into the
# global Claude Code config so it is available in every project. Downloads the
# skill markdown from the upstream project (github.com/JuliusBrussee/caveman),
# pinned to CAVEMAN_REF, and writes it to $CLAUDE_DIR/skills/caveman/SKILL.md.
# Claude Code auto-discovers skills in that directory, so no settings change is
# needed. Safe to re-run; an existing SKILL.md is backed up to SKILL.md.bak
# before it is overwritten.
#
# Env overrides:
#   CAVEMAN_REF   upstream git ref (tag/branch/commit) to fetch. Default: v1.9.1
#   CLAUDE_DIR    global Claude Code config dir. Default: $HOME/.claude
#
# Once installed, invoke in Claude Code with /caveman (or /caveman lite|full|ultra).
# Turn it off by saying "normal mode" / "stop caveman".

set -euo pipefail

REF="${CAVEMAN_REF:-v1.9.1}"
CLAUDE_DIR="${CLAUDE_DIR:-$HOME/.claude}"
REPO="JuliusBrussee/caveman"
SKILL_URL="https://raw.githubusercontent.com/$REPO/$REF/skills/caveman/SKILL.md"
DEST_DIR="$CLAUDE_DIR/skills/caveman"
DEST="$DEST_DIR/SKILL.md"
DRY_RUN=0

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      printf 'Unknown option: %s\n' "$arg" >&2
      exit 2
      ;;
  esac
done

echo "Fetching caveman skill ($REPO @ $REF)..."
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

if ! curl -fsSL "$SKILL_URL" -o "$TMP"; then
  echo "Error: failed to download $SKILL_URL" >&2
  exit 1
fi

# Guard against error pages / moved refs: the real file is YAML-frontmatter
# markdown that declares the caveman skill.
if ! head -n 1 "$TMP" | grep -q '^---' || ! grep -q '^name: caveman' "$TMP"; then
  echo "Error: downloaded file does not look like the caveman SKILL.md" >&2
  echo "  URL: $SKILL_URL" >&2
  echo "  (is CAVEMAN_REF=$REF a valid ref?)" >&2
  exit 1
fi

if [ "$DRY_RUN" -eq 1 ]; then
  echo "Dry run — would write $(wc -l < "$TMP" | tr -d ' ') lines to:"
  echo "  $DEST"
  if [ -f "$DEST" ]; then
    if cmp -s "$TMP" "$DEST"; then
      echo "  (already up to date — no change)"
    else
      echo "  (existing file differs — would back it up to $DEST.bak)"
    fi
  fi
  exit 0
fi

mkdir -p "$DEST_DIR"

if [ -f "$DEST" ] && ! cmp -s "$TMP" "$DEST"; then
  cp "$DEST" "$DEST.bak"
  echo "Backed up existing skill to $DEST.bak"
fi

cp "$TMP" "$DEST"
chmod 0644 "$DEST"

echo "Installed caveman skill -> $DEST"
echo
echo "Use it in Claude Code (restart Claude Code if it is already running):"
echo "  /caveman                    activate caveman mode (default: full)"
echo "  /caveman lite|full|ultra    set intensity level"
echo "  say \"normal mode\" / \"stop caveman\" to turn it off"
