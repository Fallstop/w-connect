#!/bin/sh
# Run once with sudo or pkexec. Only the explicitly supported USB model IDs are granted access.
set -eu
if [ "$(id -u)" != 0 ]; then
  echo 'Run: sudo sh scripts/install-usb-access.sh' >&2
  exit 1
fi
cat > /etc/udev/rules.d/70-w-connect.rules <<'RULE'
# W-Connect: exact supported ENE UNI FAN IDs; no wildcard vendor access.
# Permissions permit discovery/opening, not automatic activation of untested drivers.
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0cf2", ATTRS{idProduct}=="7750", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0cf2", ATTRS{idProduct}=="a100", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0cf2", ATTRS{idProduct}=="a101", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0cf2", ATTRS{idProduct}=="a102", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0cf2", ATTRS{idProduct}=="a103", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0cf2", ATTRS{idProduct}=="a104", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0cf2", ATTRS{idProduct}=="a105", TAG+="uaccess"
RULE
udevadm control --reload-rules
udevadm trigger --subsystem-match=hidraw --action=add
udevadm settle
echo 'W-Connect USB access installed. Reconnect the hub or log out/in if access remains denied.'
