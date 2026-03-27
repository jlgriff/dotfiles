# Dotfiles Overview

Personal scripts, notes, and settings for this machine (Ubuntu 24.04, Wayland).

## scripts/

- **rclone_pull.sh** — Syncs cloud remote → local directory. Reads `CLOUD_REMOTE` and `CLOUD_LOCAL_DIR` from `~/.dotfiles.env`. Supports `--dry-run`. Has locking and logging. Runs via cron.
- **rsync_backup_local.sh** — Backs up local cloud directory → USB drive. Reads `CLOUD_LOCAL_DIR` and `USB_DRIVE_NAME` from `~/.dotfiles.env`. Supports `--dry-run`. Runs via cron.
- **update_sunshine_to_latest.sh** — Fetches the latest Sunshine release from GitHub, compares against installed version, downloads and installs if newer. Run manually.
- **check_ri_update.sh** — Checks SourceForge for a newer Realism Invictus release, downloads (but does not install) the Full installer if one is found. Tracks installed version in `~/.ri_version`. Run manually.
- **cleanup_sync_logs.sh** — Deletes rclone/rsync log files older than 30 days. Supports `--dry-run`. Runs daily via cron.

## notes/

- **CONTEXT_civ4_realism_invictus_setup.md** — Full setup guide for Civ IV Beyond the Sword with Realism Invictus mod under Proton/Steam on Linux. Covers paths, installer quirks, protontricks dependencies, and desktop integration.
- **CONTEXT_sunshine_setup.md** — Sunshine/Moonlight streaming setup on Wayland. Covers required groups, systemd service config with display env vars, VAAPI encoder, and troubleshooting.

## crontab.txt

Cron schedule for automated scripts. Not auto-applied — load with `crontab crontab.txt` after changes.

## claude-settings.json

Claude Code permissions (allowed/denied shell commands). Symlinked from `~/.claude/settings.json`.

## Conventions

- **NEVER commit sensitive, personal, or identifying information to this repo. It is public.** Use `$USER`/`$HOME` instead of hardcoded usernames, and put machine-specific values in `~/.dotfiles.env` (gitignored).
- Scripts are symlinked from `~/.local/bin/` or `~/Documents/` into this repo.
- **When adding or updating scripts/notes, update this file and README.md to keep the summaries and symlink instructions current.**
