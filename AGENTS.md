# Dotfiles Overview

Personal scripts, notes, and settings for this machine (Ubuntu 24.04, Wayland).

## scripts/

- **rclone_pull.sh** — Syncs cloud remote → local directory. Reads `CLOUD_REMOTE` and `CLOUD_LOCAL_DIR` from `~/.dotfiles.env`. Supports `--dry-run`. Has locking and logging. Logs errors to `~/Desktop/sync_errors.csv`. Runs via cron.
- **rsync_backup_local.sh** — Backs up local cloud directory → USB drive. Reads `CLOUD_LOCAL_DIR` and `USB_DRIVE_NAME` from `~/.dotfiles.env`. Supports `--dry-run`. Logs errors to `~/Desktop/sync_errors.csv`. Runs via cron.
- **update_sunshine_to_latest.sh** — Fetches the latest Sunshine release from GitHub, compares against installed version, downloads and installs if newer. Run manually.
- **check_ri_update.sh** — Checks SourceForge for a newer Realism Invictus release, downloads (but does not install) the Full installer if one is found. Tracks installed version in `~/.ri_version`. Run manually.
- **cleanup_sync_logs.sh** — Deletes rclone/rsync log files older than 30 days. Supports `--dry-run`. Runs daily via cron.
- **setup_caveman_skill.sh** — Installs the upstream Caveman ultra-compressed response mode for supported AI agents.
- **finance-extract/** — Cargo workspace containing portable Rust CLIs for extracting financial data for AI processing and rendering validated transactions for review and GnuCash import. One `cargo build --release` builds all tools. Binaries symlinked from `~/.local/bin/`. Run manually.
  - **amazon-order-extract** — Extracts order details from saved Amazon order HTML files (standard + Fresh/Whole Foods layouts). Tracks processed files via `.processed`. Records per-item `quantity`; HTML saves omit quantities for multi-item orders (JS-rendered), so it warns when items don't reconcile to the subtotal and recovers quantity only for single-item orders.
  - **walmart-order-extract** — Extracts order details from saved Walmart order **PDF** invoices (preferred — include per-line quantities) or legacy HTML files. Same JSON output schema as amazon-order-extract. PDF parsing requires `pdftotext` (poppler-utils).
  - **gnucash-account-extract** — Extracts account paths from a GnuCash file (gzip-compressed XML). Outputs JSON grouped by category. Supports filtering by category.
  - **gnucash-transaction-extract** — Extracts the most recent N transactions from a GnuCash file as JSON (date, description, splits with account/amount/memo). Useful as categorization precedent for AI.
  - **gnucash-transaction-create** — Reads neutral transaction JSON, validates every split and balance, then independently creates a Markdown review file and GnuCash export-format multi-split CSV. Adds import-only grouping keys, refuses accidental overwrites, and supports configurable output paths, delimiters, money formats, and source-split designation through the input data. Contains no machine- or account-specific defaults.

## notes/

- **CONTEXT_civ4_realism_invictus_setup.md** — Full setup guide for Civ IV Beyond the Sword with Realism Invictus mod under Proton/Steam on Linux. Covers paths, installer quirks, protontricks dependencies, and desktop integration.
- **CONTEXT_mb_warband_floris_setup.md** — Setup guide for Mount & Blade: Warband (native Linux, no Proton) with the Floris Expanded mod pack. Covers paths, `innoextract` for the installer, the Steam launch-option trap that bypasses the wrapper script, and desktop integration.
- **CONTEXT_sunshine_setup.md** — Sunshine/Moonlight streaming setup on Wayland. Covers required groups, systemd service config with display env vars, VAAPI encoder, and troubleshooting.
- **CONTEXT_finance_extract.md** — Guide for an AI to turn finance-extract JSON output into GnuCash transactions. Covers input files, deduplication, output format, categorization rules, and examples.

## skills/

- **parse-amazon-invoice-pdfs** — Agent-neutral `SKILL.md` for fast Amazon Order Details PDF parsing. Uses a bundled Bash helper and Poppler `pdftotext`; supports macOS/Linux batch extraction, quantity and total validation, privacy filtering, and finance-extract-compatible JSON. Symlink the same folder into either Codex or Claude Code's skill directory.

## crontab.txt

Cron schedule for automated scripts. Not auto-applied — load with `crontab crontab.txt` after changes.

## claude-settings.json

Claude Code permissions (allowed/denied shell commands). Symlinked from `~/.claude/settings.json`.

## zshrc

Zsh configuration. Symlinked from `~/.zshrc`. Initializes the Starship prompt, enables Tab completion and prefix history search, persists shell history, adds `~/.local/bin` and `~/.pulumi/bin` to `PATH`, and sources `~/.dotfiles.env`. Replaces an earlier oh-my-zsh setup — the history settings are here because oh-my-zsh used to supply them.

## starship.toml

Starship prompt configuration. Symlinked from `~/.config/starship.toml`. Starship is not packaged in apt; install it with the official script from `https://starship.rs/install.sh` (pass `-b ~/.local/bin` to avoid needing sudo).

## Conventions

- **NEVER commit sensitive, personal, or identifying information to this repo. It is public.** Use `$USER`/`$HOME` instead of hardcoded usernames, and put machine-specific values in `~/.dotfiles.env` (gitignored).
- `~/.dotfiles.env` defines `DOTFILES_DIR` (path to this repo clone) along with cloud sync, backup, and git identity variables. Scripts and docs should reference `$DOTFILES_DIR` rather than hardcoding the repo path.
- Scripts are symlinked from `~/.local/bin/` or `~/Documents/` into this repo.
- **When adding or updating scripts/notes, update this file and README.md to keep the summaries and symlink instructions current.**
