#!/usr/bin/env bash
# Usage:
#   ./install_skills.sh            # symlink skills and instructions into each detected agent
#   ./install_skills.sh --dry-run  # show what would change, change nothing
#
# Both Claude Code and Codex read <skills-dir>/<name>/SKILL.md with the same YAML
# frontmatter, so a symlink is the whole adapter — no per-agent transform is needed.
#
#   Claude Code   $CLAUDE_SKILLS_DIR/<name>   default: $HOME/.claude/skills
#   Codex         $CODEX_SKILLS_DIR/<name>    default: $HOME/.agents/skills
#
# Skills load only when their description matches the task, so anything that must apply to
# every response belongs in instructions/interaction.md, which is linked to the file each
# agent reads on every turn:
#
#   Claude Code   $CLAUDE_INSTRUCTIONS        default: $HOME/.claude/CLAUDE.md
#   Codex         $CODEX_INSTRUCTIONS         default: $HOME/.codex/AGENTS.md
#
# Refuses to replace a path that exists and is not a symlink; move that copy aside first.
# Re-run after adding a skill. Editing an installed file needs no re-run.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/skills"
INSTRUCTIONS="$ROOT/instructions/interaction.md"

DRY=0
[[ "${1:-}" == "--dry-run" ]] && DRY=1

CLAUDE_SKILLS_DIR="${CLAUDE_SKILLS_DIR:-${CLAUDE_CONFIG_DIR:-$HOME/.claude}/skills}"
CODEX_SKILLS_DIR="${CODEX_SKILLS_DIR:-$HOME/.agents/skills}"

# A default stance must load every turn, so it goes in each agent's always-read
# file rather than in a skill, which loads only when its description matches.
CLAUDE_INSTRUCTIONS="${CLAUDE_INSTRUCTIONS:-${CLAUDE_CONFIG_DIR:-$HOME/.claude}/CLAUDE.md}"
CODEX_INSTRUCTIONS="${CODEX_INSTRUCTIONS:-${CODEX_HOME:-$HOME/.codex}/AGENTS.md}"

# validate <file> — reject frontmatter keys beyond name/description; Codex forbids extras.
validate() {
  local file=$1 bad
  bad=$(awk '
    /^---[[:space:]]*$/ { fence++; next }
    fence == 1 && /^[A-Za-z_][A-Za-z0-9_-]*:/ {
      key = $0; sub(/:.*/, "", key)
      if (key != "name" && key != "description") print key
    }
    fence >= 2 { exit }
  ' "$file")
  if [[ -n "$bad" ]]; then
    echo "INVALID  $file — disallowed frontmatter keys: $(echo "$bad" | tr '\n' ' ')" >&2
    return 1
  fi
  grep -q '^name:' "$file" && grep -q '^description:' "$file" || {
    echo "INVALID  $file — missing required name or description" >&2
    return 1
  }
}

# link <src> <dest> — point dest at src, leaving a real file or directory untouched.
link() {
  local src=$1 dest=$2
  if [[ -L "$dest" ]]; then
    if [[ "$(readlink "$dest")" == "$src" ]]; then
      echo "ok       $dest"
      return
    fi
    echo "relink   $dest (was -> $(readlink "$dest"))"
  elif [[ -e "$dest" ]]; then
    echo "SKIP     $dest — exists and is not a symlink; move or delete it first" >&2
    return
  else
    echo "link     $dest"
  fi
  (( DRY )) && return
  rm -f "$dest"
  ln -s "$src" "$dest"
}

fail=0
for skill in "$SRC"/*/; do
  validate "$skill/SKILL.md" || fail=1
done
(( fail )) && { echo "aborted: fix the invalid skills above" >&2; exit 1; }

(( DRY )) && echo "-- dry run, no changes --"

[[ -f "$INSTRUCTIONS" ]] || { echo "aborted: missing $INSTRUCTIONS" >&2; exit 1; }
for dest in "$CLAUDE_INSTRUCTIONS" "$CODEX_INSTRUCTIONS"; do
  echo "==> $dest"
  (( DRY )) || mkdir -p "$(dirname "$dest")"
  link "$INSTRUCTIONS" "$dest"
done

for target in "$CLAUDE_SKILLS_DIR" "$CODEX_SKILLS_DIR"; do
  echo "==> $target"
  (( DRY )) || mkdir -p "$target"
  for skill in "$SRC"/*/; do
    link "${skill%/}" "$target/$(basename "$skill")"
  done
done
