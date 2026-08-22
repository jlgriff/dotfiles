#!/usr/bin/env bash

set -euo pipefail

usage() {
  printf '%s\n' \
    'Usage: extract-text.sh [--flow] [--output-dir DIR] PDF_OR_DIRECTORY...' \
    '' \
    'Extract searchable text from Amazon invoice PDFs.' \
    'Directories are scanned non-recursively for PDF files.' \
    'Default output preserves layout and is written to stdout.' \
    '--flow uses reading order; --output-dir writes one .txt file per PDF.'
}

output_dir=""
layout_mode=1
inputs=()

while (($#)); do
  case "$1" in
    --output-dir)
      if (($# < 2)); then
        printf 'error: --output-dir requires a directory\n' >&2
        exit 2
      fi
      output_dir=$2
      shift 2
      ;;
    --flow)
      layout_mode=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      while (($#)); do
        inputs+=("$1")
        shift
      done
      ;;
    -*)
      printf 'error: unknown option: %s\n' "$1" >&2
      exit 2
      ;;
    *)
      inputs+=("$1")
      shift
      ;;
  esac
done

if ((${#inputs[@]} == 0)); then
  usage >&2
  exit 2
fi

if ! command -v pdftotext >/dev/null 2>&1; then
  printf 'error: pdftotext not found; install Poppler first\n' >&2
  exit 127
fi

pdfs=()

add_pdf() {
  local candidate=$1
  local directory
  local filename
  local lowered
  local existing

  lowered=$(printf '%s' "$candidate" | tr '[:upper:]' '[:lower:]')
  case "$lowered" in
    *.pdf) ;;
    *) return ;;
  esac

  directory=${candidate%/*}
  filename=${candidate##*/}
  if [[ "$directory" == "$candidate" ]]; then
    directory=.
  fi
  directory=$(cd "$directory" && pwd -P)
  candidate="$directory/$filename"

  if ((${#pdfs[@]} > 0)); then
    for existing in "${pdfs[@]}"; do
      if [[ "$existing" == "$candidate" ]]; then
        return
      fi
    done
  fi
  pdfs+=("$candidate")
}

shopt -s nullglob
for input in "${inputs[@]}"; do
  if [[ -f "$input" ]]; then
    add_pdf "$input"
  elif [[ -d "$input" ]]; then
    for candidate in "$input"/*; do
      [[ -f "$candidate" ]] && add_pdf "$candidate"
    done
  else
    printf 'error: input does not exist: %s\n' "$input" >&2
    exit 2
  fi
done

if ((${#pdfs[@]} == 0)); then
  printf 'error: no PDF files found\n' >&2
  exit 1
fi

if [[ -n "$output_dir" ]]; then
  mkdir -p "$output_dir"
fi

extract_pdf() {
  local pdf=$1
  local destination=$2
  if ((layout_mode)); then
    pdftotext -layout "$pdf" "$destination"
  else
    pdftotext "$pdf" "$destination"
  fi
}

for pdf in "${pdfs[@]}"; do
  if [[ -n "$output_dir" ]]; then
    filename=${pdf##*/}
    stem=${filename%.*}
    destination="$output_dir/$stem.txt"
    if [[ -e "$destination" ]]; then
      printf 'error: output already exists: %s\n' "$destination" >&2
      exit 1
    fi
    extract_pdf "$pdf" "$destination"
    printf '%s\n' "$destination"
  else
    printf '%s\n' "===== BEGIN AMAZON PDF: $pdf ====="
    extract_pdf "$pdf" -
    printf '%s\n' "===== END AMAZON PDF: $pdf ====="
  fi
done
