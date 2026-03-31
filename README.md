# dotfiles

Personal scripts, notes, and machine setup files.

## Setup

1. Clone the repo to your preferred location:
   ```bash
   git clone git@github.com:jlgriff/dotfiles.git ~/git/dotfiles
   ```

2. Create `~/.dotfiles.env` with your machine-specific config:
   ```bash
   # Path to this dotfiles repo clone (must match the clone location above)
   DOTFILES_DIR="$HOME/git/dotfiles"

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

   Source it:
   ```bash
   source ~/.dotfiles.env
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
   rclone config create "${CLOUD_REMOTE%:}" "$CLOUD_RCLONE_BACKEND" \
     username="$CLOUD_USERNAME" \
     password="$(secret-tool lookup service cloud account password)" \
     --obscure
   ```

4. Set git identity from the env values:
   ```bash
   git config --global user.name "$GIT_USER_NAME"
   git config --global user.email "$GIT_USER_EMAIL"
   ```

5. Symlink scripts into your PATH:
   ```bash
   ln -sf "$DOTFILES_DIR/scripts/rclone_pull.sh" ~/.local/bin/rclone_pull.sh
   ln -sf "$DOTFILES_DIR/scripts/rsync_backup_local.sh" ~/.local/bin/rsync_backup_local.sh
   ln -sf "$DOTFILES_DIR/scripts/cleanup_sync_logs.sh" ~/.local/bin/cleanup_sync_logs.sh
   ln -sf "$DOTFILES_DIR/scripts/update_sunshine_to_latest.sh" ~/.local/bin/update_sunshine_to_latest.sh
   ln -sf "$DOTFILES_DIR/scripts/check_ri_update.sh" ~/.local/bin/check_ri_update.sh
   ```

6. Build and symlink Rust tools (requires [Rust](https://rustup.rs/)):
   ```bash
   cd "$DOTFILES_DIR/scripts/finance-extract" && cargo build --release
   for tool in amazon-order-extract walmart-order-extract gnucash-account-extract gnucash-transaction-extract; do
     ln -sf "$DOTFILES_DIR/scripts/finance-extract/target/release/$tool" ~/.local/bin/"$tool"
   done
   ```
   Run any tool with `--help` for usage details (e.g. `amazon-order-extract --help`).

## Finance Extract Workflow

The tools in `scripts/finance-extract/` prepare financial data as JSON so an AI
can generate `.qif` transaction files for import into GnuCash.

1. **Save order HTML pages** — For each Amazon or Walmart order, open the order
   details page in a browser, save as "Webpage, HTML Only", and place the files
   into a directory per retailer (e.g. `~/Downloads/Orders/Amazon/`).

2. **Extract order data** — Point each extractor at the input directory (`-i`)
   and optionally specify an output directory (`-o`). Each tool parses item
   names, prices, totals, tax, and refunds from the saved HTML into JSON:
   ```bash
   amazon-order-extract -i <amazon-html-dir> -o <output-dir>
   walmart-order-extract -i <walmart-html-dir> -o <output-dir>
   ```
   Example:
   ```bash
   amazon-order-extract -i ~/Downloads/Orders/Amazon -o ~/Downloads
   walmart-order-extract -i ~/Downloads/Orders/Walmart -o ~/Downloads
   ```

3. **Extract GnuCash accounts** — Pull the current chart of accounts from
   the GnuCash file (`-f`) so the AI knows which account paths are valid:
   ```bash
   gnucash-account-extract -f <gnucash-file> -o <output-file>
   ```
   Example:
   ```bash
   gnucash-account-extract -f ~/Documents/transactions.gnucash -o ~/Downloads/accounts.json
   ```

4. **Extract recent transactions** — Pull the most recent N transactions (`-n`)
   as categorization precedent so the AI can see how similar purchases were
   classified:
   ```bash
   gnucash-transaction-extract -f <gnucash-file> -n <count> -o <output-file>
   ```
   Example:
   ```bash
   gnucash-transaction-extract -f ~/Documents/transactions.gnucash -n 200 -o ~/Downloads/transactions.json
   ```

5. **Feed to AI** — Provide the four JSON files to an AI along with
   [`notes/CONTEXT_finance_extract.md`](notes/CONTEXT_finance_extract.md),
   which contains the transaction output format, categorization rules, and
   examples. The AI will use the accounts and recent transactions as precedent
   to generate new GnuCash transactions from the order data.

## Structure

```
scripts/                # Shell scripts (backup, updates, etc.)
  finance-extract/      # Cargo workspace — Rust CLIs for financial data extraction
crontab.txt             # Cron schedule for automated scripts
claude-settings.json    # Claude Code permissions (symlinked from ~/.claude/settings.json)
notes/                  # Setup guides and reference docs
```
