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

5. Symlink shell configuration and scripts:
   ```bash
   mkdir -p ~/.config ~/.local/bin
   ln -sf "$DOTFILES_DIR/zshrc" ~/.zshrc
   ln -sf "$DOTFILES_DIR/starship.toml" ~/.config/starship.toml
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
   for tool in amazon-order-extract walmart-order-extract gnucash-account-extract gnucash-transaction-extract gnucash-transaction-create; do
     ln -sf "$DOTFILES_DIR/scripts/finance-extract/target/release/$tool" ~/.local/bin/"$tool"
   done
   ```
   Run any tool with `--help` for usage details (e.g. `amazon-order-extract --help`).

7. `setup_caveman_skill.sh` installs the upstream Caveman ultra-compressed
   response mode for supported AI agents.

8. Install repository-owned, agent-neutral skills for Codex, Claude Code, or
   both. `install_skills.sh` symlinks every folder in `skills/` into each
   agent's skill directory, so one source folder serves both agents:
   ```bash
   "$DOTFILES_DIR/scripts/install_skills.sh" --dry-run   # show what would change
   "$DOTFILES_DIR/scripts/install_skills.sh"
   ```
   Claude Code reads `~/.claude/skills` and Codex reads `~/.agents/skills`;
   override either with `CLAUDE_SKILLS_DIR` or `CODEX_SKILLS_DIR`. The same run
   links `instructions/interaction.md` into `~/.claude/CLAUDE.md` and
   `~/.codex/AGENTS.md` (override with `CLAUDE_INSTRUCTIONS` /
   `CODEX_INSTRUCTIONS`). The script refuses to replace a path that exists as a
   real file or folder, so move a hand-installed copy aside first. Re-run after
   adding a skill — editing an installed file needs no re-run.

## Agent Skills

Every skill is a single agent-neutral `SKILL.md` whose frontmatter carries only
`name` and `description`, which is what keeps one file valid for both Claude
Code and Codex. Agents load a skill two ways: implicitly, by matching its
`description` against the task, and explicitly — `/name` in Claude Code,
`$name` in Codex.

- **parse-amazon-invoice-pdfs** — Extracts one or many Amazon Order Details
  PDFs through Poppler before an agent parses items, quantities, prices, and
  totals. Includes a macOS/Linux batch helper, arithmetic checks, privacy
  rules, and the finance-extract-compatible Amazon summary schema.
- **build-feature** — Research first, drive the work with tests, implement the
  smallest thing that works, then cull the scaffolding tests that no longer
  earn their place.
- **code-comments** — A one-line doc on every function and almost nothing
  else; an inline comment is a signal to rename or extract instead.
- **commit-message** — One sentence, under 25 words, leading past-tense verb,
  no prefix in front of it.
- **demo-video** — Routes a demo or screen-recording request to whatever
  recording skill the target repo already owns.
- **failing-test-first** — Show a test failing before adding it; a test never
  seen to fail is not evidence.
- **revise-message** — Tightens a draft message for clarity and flow while
  preserving your voice.

### Agent instructions

`instructions/interaction.md` holds the default working stance every agent reads
on every turn — how to lead with the answer, how to report unverified state, how
terse to be, and the standing defaults for comments, scope, styling, and commit
messages. A skill loads only when its `description` matches the task, so
anything that must apply to *every* response belongs here instead of in a skill.

### Adding a skill

```bash
mkdir -p skills/my-skill && $EDITOR skills/my-skill/SKILL.md
scripts/install_skills.sh
```

Keep frontmatter to `name` and `description` only — `install_skills.sh` fails
the run otherwise, because Codex rejects extra keys. Put all "when to use" text
in `description`, since that is the trigger for both agents. Never name a host
tool ("read the file", not "use the Read tool"), and keep paths relative to the
skill directory.

## Finance Extract Workflow

The tools in `scripts/finance-extract/` prepare financial data as JSON so an AI
can create a neutral, balanced transaction file. A renderer validates that file
and independently creates both Markdown for review and multi-split CSV for
GnuCash import; neither generated format depends on the other.

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
   which contains the transaction schema, categorization rules, and examples.
   The AI will use the accounts and recent transactions as precedent to create
   a balanced neutral JSON file from the statement and order data.

6. **Create review and import files** — Validate the neutral JSON and create
   both outputs directly from it. Input and output paths are command-line
   arguments; no paths or account names are built into the tool:
   ```bash
   gnucash-transaction-create \
     --input <transactions.json> \
     --markdown-output <transactions.md> \
     --csv-output <transactions.csv> \
     --expected-transactions <count>
   ```
   Each transaction must mark exactly one split with `"source": true`. Amounts
   are exact decimal strings such as `"-1234.50"`. The renderer puts the source
   split last in Markdown and first in CSV. Optional `source_description` and
   `review_notes` values appear only in Markdown. The renderer refuses to
   replace either output unless `--force` is supplied and supports configurable
   CSV and money separators. Omit output arguments to create
   `<input-stem>.md` and `<input-stem>_gnucash.csv` beside the input. Its
   transaction IDs are per-file grouping keys used to keep otherwise identical
   adjacent transactions separate during import; they are not GnuCash ledger
   GUIDs. Run `gnucash-transaction-create --help` for all options.
   Review the Markdown, apply any corrections to the neutral JSON, then rerun
   the renderer so both generated files stay synchronized.

7. **Import into GnuCash** — Use **File → Import → Import Transactions from
   CSV**, choose the built-in **GnuCash Export Format** settings, and leave the
   global Account blank. This preset enables multi-split mode, skips the header,
   and maps the export-format columns by position. Select the date and currency
   formats matching the file, then review account mappings and transaction
   matches before applying the import.

## Structure

```
scripts/                # Shell scripts (backup, updates, etc.)
  finance-extract/      # Cargo workspace — Rust CLIs for financial data extraction
instructions/           # Default agent stance, linked to each agent's always-read file
skills/                 # Agent-neutral SKILL.md workflows and bundled helpers
                        # (installed with scripts/install_skills.sh)
starship.toml           # Starship prompt configuration (symlinked from ~/.config/starship.toml)
zshrc                   # Zsh configuration (symlinked from ~/.zshrc)
crontab.txt             # Cron schedule for automated scripts
claude-settings.json    # Claude Code permissions (symlinked from ~/.claude/settings.json)
notes/                  # Setup guides and reference docs
```
