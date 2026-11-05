#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ -f Cargo.toml ]; then
command -v cargo >/dev/null || { echo 'Install Rust (rustup), a C compiler, pkg-config and libudev development files first.' >&2; exit 1; }
cargo build --release --locked
fi
[ -f target/release/w-connect ] || { echo "Release binary missing" >&2; exit 1; }
install -Dm755 target/release/w-connect "$HOME/.local/bin/w-connect"
install -Dm644 assets/icon.svg "$HOME/.local/share/icons/hicolor/scalable/apps/w-connect.svg"
mkdir -p "$HOME/.config/systemd/user" "$HOME/.local/share/applications"
cat > "$HOME/.config/systemd/user/w-connect.service" <<SERVICE
[Unit]
Description=W-Connect fan and lighting control
After=graphical-session.target

[Service]
ExecStart="%h/.local/bin/w-connect"
Restart=always
RestartSec=2
TimeoutStopSec=15
NoNewPrivileges=true

[Install]
WantedBy=default.target
SERVICE
cat > "$HOME/.local/share/applications/w-connect.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=W-Connect
Comment=Fan curves and lighting
Exec=xdg-open http://127.0.0.1:7676
Icon=w-connect
Terminal=false
Categories=Settings;HardwareSettings;
DESKTOP
systemctl --user daemon-reload
systemctl --user enable w-connect.service
systemctl --user restart w-connect.service
echo 'Open http://127.0.0.1:7676. If USB access is denied, run sudo sh scripts/install-usb-access.sh.'
