# dotfiles

Personal scripts, notes, and machine setup files.

## Setup

1. Clone the repo:
   ```bash
   git clone git@github.com:jlgriff/dotfiles.git ~/git/dotfiles
   ```

2. Create `~/.dotfiles.env` with your machine-specific config:
   ```bash
   # Cloud sync (rclone remote name, local folder, login username, and rclone backend type)
   CLOUD_REMOTE="Dropbox:"
   CLOUD_LOCAL_DIR="Dropbox"
   CLOUD_USERNAME="your-cloud-username"
   CLOUD_RCLONE_BACKEND="dropbox"  # rclone backend type (e.g. drive, s3, protondrive)

   # USB drive name for local backups (used by rsync_backup_local.sh)
   USB_DRIVE_NAME="SAMSUNG_T7"

   # Git identity (use GitHub noreply email to keep personal email out of commits)
   # Find your noreply email at https://github.com/settings/emails
   GIT_USER_NAME="your-github-username"
   GIT_USER_EMAIL="your-id+your-github-username@users.noreply.github.com"
   ```

3. Store your cloud drive password in the system keyring.

   Install `secret-tool` for secure credential storage:
   ```bash
   sudo apt install libsecret-tools
   ```

   Save the password (you'll be prompted to enter it):
   ```bash
   secret-tool store --label="Cloud Drive" service cloud account password
   ```

   Configure the rclone remote using the stored credentials:
   ```bash
   source ~/.dotfiles.env
   rclone config create "${CLOUD_REMOTE%:}" "$CLOUD_RCLONE_BACKEND" \
     username="$CLOUD_USERNAME" \
     password="$(secret-tool lookup service cloud account password)" \
     --obscure
   ```

4. Set git identity from the env values:
   ```bash
   source ~/.dotfiles.env
   git config --global user.name "$GIT_USER_NAME"
   git config --global user.email "$GIT_USER_EMAIL"
   ```

5. Symlink scripts into your PATH:
   ```bash
   ln -sf ~/git/dotfiles/scripts/rclone_pull.sh ~/.local/bin/rclone_pull.sh
   ln -sf ~/git/dotfiles/scripts/rsync_backup_local.sh ~/.local/bin/rsync_backup_local.sh
   ln -sf ~/git/dotfiles/scripts/cleanup_sync_logs.sh ~/.local/bin/cleanup_sync_logs.sh
   ln -sf ~/git/dotfiles/scripts/update_sunshine_to_latest.sh ~/.local/bin/update_sunshine_to_latest.sh
   ln -sf ~/git/dotfiles/scripts/check_ri_update.sh ~/.local/bin/check_ri_update.sh
   ```

## Structure

```
scripts/     # Shell scripts (backup, updates, etc.)
notes/       # Setup guides and reference docs
```
