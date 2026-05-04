# Mount & Blade: Warband — Floris Expanded Setup

## Installation Method

Steam with the **native Linux build** of Warband — no Proton, no Wine prefix.
Steam App ID: 48700. Warband modules are folders dropped into the game's
`Modules/` directory; no installer-into-prefix dance required (unlike Civ IV).

## Current Mod Version

Floris Expanded Mod Pack 2.54 (installed 2026-05-04).

## Key Paths

- **Game install**: `~/.local/share/Steam/steamapps/common/MountBlade Warband`
- **Native game binary**: `.../mb_warband_linux` (64-bit ELF)
- **Wrapper script** (the one Steam should invoke): `.../mb_warband.sh`
- **Configurator wrapper**: `.../mbw_config.sh` (runs `mbw_config_linux` GUI)
- **Modules directory**: `.../Modules/`
- **Floris Expanded module**: `.../Modules/Floris Expanded Mod Pack 2.54/`
- **Floris internal module name** (from its `module.ini`): `Floris254e`
- **User config + savegames root**: `~/.mbwarband/`
- **Module persistence file**: `~/.mbwarband/last_module_warband` (plain text, contains the selected module's folder name)
- **Game config**: `~/.mbwarband/rgl_config.txt` (resolution, sound, graphics quality, etc.)
- **Savegames**: `~/.mbwarband/Savegames/`

## Other Modules Present

`Native`, `Napoleonic Wars`, `Viking Conquest` (came with the Steam install).
Floris Expanded is module #4.

## Desktop Integration

- `~/Desktop/Floris.desktop` — launches Warband + Floris via Steam.
- `~/.local/share/applications/Floris.desktop` — same, for the GNOME app grid.
- `Exec=bash -c 'steam -applaunch 48700'` — note **no `-m` flag**. The module
  is read from `~/.mbwarband/last_module_warband` so we don't need to (and
  must not — see below) pass it on the command line.
- Icon at `~/.local/share/icons/floris.bmp`, copied from the Floris module's
  `main.bmp` (340×275 BMP, the in-game splash). GTK / gdk-pixbuf handle BMP
  fine for desktop icons. Note that `.desktop` files don't expand `~` — the
  `Icon=` field needs an absolute path.
- `StartupWMClass=steam_app_48700` — required for GNOME to match the running
  window to the launcher icon.

## Installer Format & Extraction

Floris is distributed as a single Inno Setup installer (`Floris254.exe`,
~1.1 GB) that bundles **all four variants** (Basic, Gameplay, Expanded,
Dev Suite). Extract on Linux **without Wine** using `innoextract`:

```bash
sudo apt install innoextract
innoextract -d /tmp/floris-extract ~/Downloads/Floris254.exe
# everything lands under /tmp/floris-extract/app/
mv "/tmp/floris-extract/app/Modules/Floris Expanded Mod Pack 2.54" \
   "$HOME/.local/share/Steam/steamapps/common/MountBlade Warband/Modules/"
rm -rf /tmp/floris-extract  # extraction inflates ~1.1 GB → ~8 GB; clean up
```

There's a stray `app/Modules/Modules/Floris Dev Suite 2.54` (28K stub) and a
top-level `app/languages/` folder (German/English UI overrides for the *base
game*, not the mod) — both can be ignored unless you want a localised UI.

## Critical Lessons Learned

### Steam Launch Options must be EMPTY for Floris to work

Set Launch Options for app 48700 to nothing.

If launch options contain `-m "Floris Expanded Mod Pack 2.54"` (or anything
that looks like an arg), Steam invokes `mb_warband_linux` **directly** instead
of going through `mb_warband.sh`. The wrapper is what sets
`LD_LIBRARY_PATH=$PROGRAM_DIRECTORY` so the binary can find `libsteam_api.so`
from the install directory. Without it, the game starts, fails the Steam API
init, and exits with code 2 — looks like an instant crash to the user. The
gameprocess log shows the bypassed invocation:

```
.../mb_warband_linux -m "Floris Expanded Mod Pack 2.54"   # exit code 2
```

vs. the working invocation:

```
.../mb_warband.sh                                          # wrapper sets LD_LIBRARY_PATH first
```

### mbw_config.sh works from terminal but crashes via Steam

Steam's "Configure Mount&Blade: Warband (choose module)" entry runs the binary
directly without the `.sh` wrapper, same `libsteam_api.so` failure → crash.
Run the configurator from a terminal instead:

```bash
cd ~/.local/share/Steam/steamapps/common/MountBlade\ Warband
./mbw_config.sh
```

The `cd` matters — `mbw_config.sh` does **not** cd into the program directory
itself, so when run from elsewhere the binary's CWD is wrong and the "Current
Module" dropdown shows up empty (it looks for `./Modules/` relative to CWD).

### Steam Compatibility tab must NOT force a Proton tool

Properties → Compatibility → "Force the use of a specific Steam Play
compatibility tool" must be unchecked. If it gets enabled, Steam wraps the
native Linux launch in Proton, which breaks things in non-obvious ways and
creates a stray `~/.local/share/Steam/steamapps/compatdata/48700/` directory.

### Saved-game compatibility

Floris Gameplay/Expanded saves are **not** compatible with Native or with
older Floris versions. Switching modules means starting over.

## Updating / Switching Variants

There's no auto-updater — Floris hasn't shipped a new version since 2.54
(the team went dormant ~2014-2016). To switch from Expanded to Gameplay/Basic
(or vice versa), the bundled installer is the same — re-extract and copy the
desired `Modules/Floris ___ Mod Pack 2.54` folder into Warband's `Modules/`,
then re-run `mbw_config.sh` to point Warband at the new module.

To uninstall Floris cleanly: delete its module folder, then either delete or
edit `~/.mbwarband/last_module_warband` to point at another module (e.g.
`Native`).
