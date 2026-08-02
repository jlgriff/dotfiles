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
   ln -sf "$DOTFILES_DIR/scripts/setup_caveman_skill.sh" ~/.local/bin/setup_caveman_skill.sh
   ```

6. Build and symlink Rust tools (requires [Rust](https://rustup.rs/)):
   ```bash
   cd "$DOTFILES_DIR/scripts/finance-extract" && cargo build --release
   for tool in amazon-order-extract walmart-order-extract gnucash-account-extract gnucash-transaction-extract; do
     ln -sf "$DOTFILES_DIR/scripts/finance-extract/target/release/$tool" ~/.local/bin/"$tool"
   done
   ```
   Run any tool with `--help` for usage details (e.g. `amazon-order-extract --help`).

7. Install the caveman skill for Claude and ChatGPT (optional):
   ```bash
   setup_caveman_skill.sh --dry-run   # preview
   setup_caveman_skill.sh
   ```
   Installs to each agent found on the machine, skipping the rest:

   | Target | Destination | Activate with |
   |---|---|---|
   | Claude Code | `~/.claude/skills/caveman/SKILL.md` | `/caveman` |
   | Codex CLI | `~/.codex/prompts/caveman.md` | `/caveman` |
   | ChatGPT web app | `~/.local/share/caveman/chatgpt-custom-instructions.md` | paste by hand (see below) |

   The ChatGPT web app has no local config, so the script only generates the
   text. Paste it into **Settings → Personalization → Custom Instructions →
   "What traits should ChatGPT have?"** — the condensed version fits that
   field's ~1500 character cap.

   For Claude Code and Codex CLI, activate with `/caveman` (or
   `/caveman lite|full|ultra`) and turn it off by saying "normal mode".
   Override `CAVEMAN_REF`, `CLAUDE_DIR`, `CODEX_DIR`, or `CHATGPT_OUT` to
   change the pinned upstream version or the install paths.

## Finance Extract Workflow

The tools in `scripts/finance-extract/` prepare financial data as JSON so an AI
can generate `.qif` transaction files for import into GnuCash.

1. **Save order pages** — For each order, save it into a directory per retailer
   (e.g. `~/Downloads/Orders/Amazon/`):
   - **Amazon** — open the order details page and save as "Webpage, HTML Only".
   - **Walmart** — open the order's invoice/receipt view and print to PDF
     (preferred — PDFs include per-line quantities). Saved HTML is still
     accepted but lacks quantities. PDF parsing requires `pdftotext`
     (`sudo apt install poppler-utils`).

2. **Extract order data** — Point each extractor at the input directory (`-i`)
   and optionally specify an output directory (`-o`). Each tool parses item
   names, prices, quantities, totals, tax, and refunds into JSON:
   ```bash
   amazon-order-extract -i <amazon-html-dir> -o <output-dir>
   walmart-order-extract -i <walmart-html-dir> -o <output-dir>
   ```
   Example:
   ```bash
   amazon-order-extract -i ~/Downloads/Orders/Amazon -o ~/Downloads   # HTML
   walmart-order-extract -i ~/Downloads/Orders/Walmart -o ~/Downloads  # PDF or HTML
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

4. **Extract recent transactions** — Pull transactions as categorization
   precedent so the AI can see how similar purchases were classified. Exports
   all transactions by default, or use `-n` to limit:
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
