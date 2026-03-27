# Sunshine (Moonlight Streaming) — Setup

## Overview
Sunshine is a self-hosted game/desktop streaming server by LizardByte. Connect to it with the Moonlight client from another device on the network.

## Installation
Installed via `.deb` package from [GitHub releases](https://github.com/LizardByte/Sunshine/releases). Use `update_sunshine_to_latest.sh` to update.

## Key Details

- **Package**: `sunshine-ubuntu-24.04-amd64.deb`
- **Updating**: Run `update_sunshine_to_latest.sh` (checks installed vs latest GitHub release, downloads and installs if newer)
- **Config**: `~/.config/sunshine/sunshine.conf`
- **Web UI**: `https://localhost:47990` (for pairing, settings)
- **Service**: `~/.config/systemd/user/sunshine.service` (user-level systemd)
- **Encoder**: VAAPI (H.264 + HEVC)

## Required Group Memberships

The user must be in these groups for Sunshine to function:
- `video` — GPU access
- `render` — GPU render nodes
- `input` — keyboard/mouse relay

Added via: `sudo usermod -aG video,render,input $USER` (requires logout/reboot to take effect).

## Service Configuration — Lessons Learned

The default service file fails because Sunshine can't find the display. On Wayland, the service must explicitly pass display environment variables:

```ini
[Unit]
Description=Sunshine Game Streaming Server
After=network.target graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
Environment=DISPLAY=:0
Environment=WAYLAND_DISPLAY=wayland-0
Environment=XDG_SESSION_TYPE=wayland
ExecStart=/usr/bin/sunshine
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
```

Without these environment variables, the log shows:
```
Error: Environment variable WAYLAND_DISPLAY has not been defined
Error: Unable to initialize capture method
Fatal: Unable to find display or encoder during startup.
```

## Troubleshooting

- **"Unable to find display or encoder"** — check that WAYLAND_DISPLAY and DISPLAY are set in the service file, and that the user is in the `video` and `render` groups.
- **"Permission denied" for virtual mouse/keyboard/gamepad** — user is not in the `input` group.
- **Encoder failures (nvenc, vaapi, software all fail)** — display environment variables are missing; Sunshine can't access the GPU without them.
- **After updating**: restart with `systemctl --user restart sunshine`.
