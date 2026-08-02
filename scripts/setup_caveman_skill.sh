#!/usr/bin/env bash
# Usage:
#   ./setup_caveman_skill.sh            # install/update the caveman skill for every detected agent
#   ./setup_caveman_skill.sh --dry-run  # show what would be fetched/written, change nothing
#   ./setup_caveman_skill.sh -h|--help  # show help
#
# Installs the "caveman" skill (ultra-compressed /caveman response mode) for both
# Claude and ChatGPT. Downloads the skill markdown from the upstream project
# (github.com/JuliusBrussee/caveman), pinned to CAVEMAN_REF, and installs it to
# each target that is present on this machine:
#
#   Claude Code   $CLAUDE_DIR/skills/caveman/SKILL.md      (auto-discovered; no settings change)
#   Codex CLI     $CODEX_DIR/prompts/caveman.md            (invoke with /caveman)
#   ChatGPT web   $CHATGPT_OUT                             (paste into Custom Instructions)
#
# Claude Code and Codex CLI are skipped when not installed. The ChatGPT web app
# has no local config, so the script cannot install anything there — it writes a
# condensed, paste-ready version of the skill and prints where to paste it.
# Safe to re-run; an existing file is backed up to <file>.bak before overwriting.
#
# Env overrides:
#   CAVEMAN_REF   upstream git ref (tag/branch/commit) to fetch. Default: v1.9.1
#   CLAUDE_DIR    global Claude Code config dir. Default: $HOME/.claude
#   CODEX_DIR     global Codex CLI config dir. Default: $HOME/.codex
#   CHATGPT_OUT   where to write the ChatGPT custom-instructions text.
#                 Default: $HOME/.local/share/caveman/chatgpt-custom-instructions.md
#
# Once installed, invoke with /caveman (or /caveman lite|full|ultra).
# Turn it off by saying "normal mode" / "stop caveman".

set -euo pipefail

REF="${CAVEMAN_REF:-v1.9.1}"
CLAUDE_DIR="${CLAUDE_DIR:-$HOME/.claude}"
CODEX_DIR="${CODEX_DIR:-$HOME/.codex}"
CHATGPT_OUT="${CHATGPT_OUT:-$HOME/.local/share/caveman/chatgpt-custom-instructions.md}"
REPO="JuliusBrussee/caveman"
SKILL_URL="https://raw.githubusercontent.com/$REPO/$REF/skills/caveman/SKILL.md"
CLAUDE_DEST="$CLAUDE_DIR/skills/caveman/SKILL.md"
CODEX_DEST="$CODEX_DIR/prompts/caveman.md"
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

# ---------- Helpers ----------

# install_file <source-file> <destination> — write with backup, or describe under --dry-run.
install_file() {
  local src="$1"
  local dest="$2"

  if [ "$DRY_RUN" -eq 1 ]; then
    echo "  would write $(wc -l < "$src" | tr -d ' ') lines to $dest"
    if [ -f "$dest" ]; then
      if cmp -s "$src" "$dest"; then
        echo "    (already up to date — no change)"
      else
        echo "    (existing file differs — would back it up to $dest.bak)"
      fi
    fi
    return 0
  fi

  mkdir -p "$(dirname "$dest")"

  if [ -f "$dest" ] && ! cmp -s "$src" "$dest"; then
    cp "$dest" "$dest.bak"
    echo "  backed up existing file to $dest.bak"
  fi

  cp "$src" "$dest"
  chmod 0644 "$dest"
  echo "  installed -> $dest"
}

# ---------- Fetch upstream skill ----------

echo "Fetching caveman skill ($REPO @ $REF)..."
SKILL_TMP="$(mktemp)"
CODEX_TMP="$(mktemp)"
CHATGPT_TMP="$(mktemp)"
trap 'rm -f "$SKILL_TMP" "$CODEX_TMP" "$CHATGPT_TMP"' EXIT

if ! curl -fsSL "$SKILL_URL" -o "$SKILL_TMP"; then
  echo "Error: failed to download $SKILL_URL" >&2
  exit 1
fi

# Guard against error pages / moved refs: the real file is YAML-frontmatter
# markdown that declares the caveman skill.
if ! head -n 1 "$SKILL_TMP" | grep -q '^---' || ! grep -q '^name: caveman' "$SKILL_TMP"; then
  echo "Error: downloaded file does not look like the caveman SKILL.md" >&2
  echo "  URL: $SKILL_URL" >&2
  echo "  (is CAVEMAN_REF=$REF a valid ref?)" >&2
  exit 1
fi

# ---------- Claude Code ----------

echo
echo "Claude Code:"
if [ -d "$CLAUDE_DIR" ] || command -v claude >/dev/null 2>&1; then
  install_file "$SKILL_TMP" "$CLAUDE_DEST"
else
  echo "  skipped — no $CLAUDE_DIR and no 'claude' on PATH"
fi

# ---------- Codex CLI ----------

# Codex reads custom prompts from $CODEX_DIR/prompts/*.md and exposes each as a
# slash command. Its frontmatter keys differ from a Claude skill's, so swap the
# upstream frontmatter for a Codex-flavored one and keep the body verbatim.
{
  printf -- '---\n'
  printf 'description: Ultra-compressed caveman response mode (lite/full/ultra/wenyan)\n'
  printf 'argument-hint: "[lite|full|ultra|wenyan-lite|wenyan-full|wenyan-ultra]"\n'
  printf -- '---\n\n'
  printf 'Caveman level: $ARGUMENTS (default: full when no level given).\n\n'
  sed '1,/^---$/d' "$SKILL_TMP"
} > "$CODEX_TMP"

echo
echo "Codex CLI:"
if [ -d "$CODEX_DIR" ] || command -v codex >/dev/null 2>&1; then
  install_file "$CODEX_TMP" "$CODEX_DEST"
else
  echo "  skipped — no $CODEX_DIR and no 'codex' on PATH"
fi

# ---------- ChatGPT web app ----------

# The web app has no local config to install into, and its Custom Instructions
# fields cap out around 1500 characters — too short for the full SKILL.md. Write
# a condensed rendering of the same rules for the user to paste in by hand.
cat > "$CHATGPT_TMP" <<'EOF'
Always respond in "caveman" mode: terse, high-density technical English.

Rules:
- Drop articles (a/an/the), filler (just, really, basically, actually, simply), pleasantries (sure, certainly, happy to), and hedging.
- Sentence fragments are fine. Prefer short synonyms (big not extensive, fix not implement a solution for).
- Keep all technical substance. Technical terms, identifiers, and numbers stay exact. Quote errors verbatim. Never compress code blocks.
- Pattern: [thing] [action] [reason]. [next step].

Levels (default: full):
- lite: no filler or hedging, keep articles and full sentences.
- full: drop articles, fragments OK, short synonyms.
- ultra: abbreviate (DB, auth, config, req, res, fn, impl), strip conjunctions, arrows for causality (X -> Y), one word where one word will do.
Switch when I say "caveman lite", "caveman full", or "caveman ultra".

Stay in caveman mode for every response, for the whole conversation. Do not drift back to verbose prose. Exit only when I say "normal mode" or "stop caveman".

Write normally (no caveman) for: security warnings, confirmation of irreversible actions, multi-step instructions where fragments risk misreading, and when I ask you to clarify or repeat something. Resume caveman right after.
EOF

echo
echo "ChatGPT web app:"
install_file "$CHATGPT_TMP" "$CHATGPT_OUT"
echo "  ($(wc -c < "$CHATGPT_TMP" | tr -d ' ') characters — Custom Instructions caps near 1500)"

# ---------- Summary ----------

if [ "$DRY_RUN" -eq 1 ]; then
  echo
  echo "Dry run — nothing written."
  exit 0
fi

echo
echo "Claude Code / Codex CLI (restart the agent if it is already running):"
echo "  /caveman                    activate caveman mode (default: full)"
echo "  /caveman lite|full|ultra    set intensity level"
echo "  say \"normal mode\" / \"stop caveman\" to turn it off"
echo
echo "ChatGPT web app — paste the generated text by hand:"
echo "  chatgpt.com -> Settings -> Personalization -> Custom Instructions"
echo "  -> \"What traits should ChatGPT have?\", then paste:"
echo "  $CHATGPT_OUT"
