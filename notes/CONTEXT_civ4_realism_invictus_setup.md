# Civ IV BtS Realism Invictus — Proton Setup

## Installation Method
Steam with Proton (not standalone Wine). Proton manages the Wine prefix automatically.
Proton version: 10.0-200 (Proton Hotfix). Steam App ID: 8800.

## Current Mod Version
Realism Invictus 3.81 (installed 2026-03-25).

## Key Paths

- **Game install**: `~/.local/share/Steam/steamapps/common/Sid Meier's Civilization IV Beyond the Sword`
- **Convenience symlink**: `~/Games/Sid Meier's Civilization IV Beyond the Sword` → same
- **Realism Invictus mod**: `.../Beyond the Sword/Mods/Realism Invictus/` (contains `Assets`, `Resource`, `PrivateMaps`)
- **Proton prefix**: `~/.local/share/Steam/steamapps/compatdata/8800/pfx`
- **Game config (INI)**: `.../compatdata/8800/pfx/drive_c/users/steamuser/Documents/My Games/Beyond the Sword/CivilizationIV.ini`
- **RI mod settings**: `.../My Games/Beyond the Sword/Realism Invictus/Settings/` (multiple BUG mod INIs)
- **Saves**: `.../My Games/Beyond the Sword/Saves/`

## Other Mods Present
14 other mods in the Mods directory: Afterworld, Broken Star, Charlemagne, Crossroads of the World, Defense, FfH Age of Ice, Final Frontier, Gods of Old, MesoAmerica, Next War, Rhye's and Fall of Civilization, The Road to War.

## Desktop Integration
- `~/Desktop/Realism Invictus.desktop` — launches RI directly via Steam
- Uses `bash -c 'steam -applaunch 8800 "mod=\\Mods\\Realism Invictus"'` as the Exec command. Do NOT use `steam://run/` URLs — they mangle the mod path arguments.
- Icon copied to `~/.local/share/icons/realism-invictus.png` (original at `.../Mods/Realism Invictus/Icons/invictus_37.png`). Using a simple path avoids issues with the apostrophe in the game path.
- `StartupWMClass=steam_app_8800` — required for GNOME to match the running window to the launcher icon
- Also installed to `~/.local/share/applications/Realism Invictus.desktop` for app grid/pin-to-dash access

## Prefix Dependencies
`d3dx9` was installed into the Proton prefix via `protontricks --no-bwrap 8800 d3dx9`. This places native d3dx9 DLLs (versions 24-43) in `system32` and `syswow64`. Required to prevent "error loading shader libraries" when loading RI.

protontricks installed via pipx (`pipx install protontricks`). The apt version is too old and fails with "Invalid file magic number". Requires `--no-bwrap` flag due to bubblewrap permission issues.

## Updating Realism Invictus — Lessons Learned

Run `check_ri_update.sh` to check SourceForge for a newer release and download the installer to `~/Downloads`. The script tracks the installed version in `~/.ri_version` and verifies the download via MD5.

The RI installer (.exe) is an NSIS installer that must be run via the Proton Wine binary. The game directory is hidden (under `~/.local`) so the installer's file browser can't see it (hidden dirs not shown, symlinks not recognized).

### Workaround: Temporarily move BtS to a visible location

```bash
# Move BtS to /tmp so the installer can browse to it
mv ".../Beyond the Sword" /tmp/BtS

# Run the installer
STEAM_COMPAT_DATA_PATH="$HOME/.local/share/Steam/steamapps/compatdata/8800" \
STEAM_COMPAT_CLIENT_INSTALL_PATH="$HOME/.local/share/Steam" \
WINEPREFIX="$HOME/.local/share/Steam/steamapps/compatdata/8800/pfx" \
"$HOME/.local/share/Steam/steamapps/common/Proton Hotfix/files/bin/wine" \
"/path/to/Realism Invictus Setup (Full).exe"

# In the installer, browse to Z:\tmp\BtS
# After installer finishes, move back:
mv /tmp/BtS ".../Beyond the Sword"
```

### CRITICAL: The installer asks for the "Beyond the Sword folder" but treats the selected path as the GAME ROOT (parent of Beyond the Sword)

When you point it at `Z:\tmp\BtS`, it creates `BtS/Beyond the Sword/Mods/Realism Invictus/` (nested one level too deep) rather than writing directly into `BtS/Mods/Realism Invictus/`. After moving back, the new mod ends up at `.../Beyond the Sword/Beyond the Sword/Mods/Realism Invictus/` — which the game never loads.

**Fix after install**: Move the mod from the nested location to the correct one:
```bash
BTS='.../Beyond the Sword'
rm -rf "$BTS/Mods/Realism Invictus"
mv "$BTS/Beyond the Sword/Mods/Realism Invictus" "$BTS/Mods/Realism Invictus"
rm -rf "$BTS/Beyond the Sword"  # clean up the nested duplicate
```

**Better approach**: Point the installer at the PARENT directory instead (i.e., `Z:\tmp` after moving `Beyond the Sword` to `/tmp/Beyond the Sword` — keeping the original directory name), so the installer finds `Beyond the Sword` inside it and writes to the correct location.
