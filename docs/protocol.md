> This document records the original SL v1 hardware investigation. For the expanded 0.2 driver scope and verification status, see [controller-support.md](controller-support.md).

# UNI FAN SL protocol and implementation evidence

W-Connect deliberately supports the attached **ENE / Lian Li UNI FAN SL v1 hub, USB `0cf2:a100`**. It does not assume that another UNI FAN model speaks the same protocol. All channel indices below are zero-based; the UI displays banks 1–4.

## Evidence levels

- **Observed on this machine, without sending control commands:** the USB identity, serial, interface layout, kernel driver, and HID report descriptor below.
- **Verified against vendor code:** commands, raw duty semantics, report types, RBG color order, RPM selector and decoding. The original installer was extracted separately; see [vendor reverse engineering](vendor-reverse-engineering.md). Locally inspected vendor classes include `LConnectCore.Products.Ene6K77Fan/SLFanDevice.cs` and `SLFanController.cs` under the ignored `.research/vendor/decompiled/core` directory.
- **Corroborated by prior art:** independent implementations listed below use matching SL commands. Their coverage is not proof that every command has been exercised on this particular computer.
- **Not established by this research:** measured RPM response to a changed duty, lighting appearance, resume/disconnect behavior on real hardware, or a working Windows deployment. Code tests verify packet construction, not physical fan behavior.

No firmware image was flashed, vendor executable launched, kernel driver replaced, or speed/lighting command sent during this research.

## Attached hardware

`lsusb -v -d 0cf2:a100` and `/sys/class/hidraw/hidraw8/device/uevent` identified:

| Field | Observed value |
|---|---|
| Product | `LianLi-UNI FAN-SL-v1.8` |
| VID / PID | `0cf2` / `a100` |
| Serial | Present; omitted from public documentation |
| USB interface 0 | Vendor-specific, no endpoints |
| USB interface 1 | HID, bound to `hid-generic` |
| Interrupt input endpoint | `0x82`, 32-byte maximum USB packet |
| HID device node at discovery | `/dev/hidraw8` (do not hard-code this) |
| Access at discovery | root-only, mode `0600` |

The 43-byte report descriptor is:

```text
06 72 ff 09 a1 a1 01 85 e0 09 10 15 00 26 ff 00
75 08 95 40 81 02 09 11 15 00 26 ff 00 75 08 96
60 01 91 02 09 04 95 0a b1 02 c0
```

This declares usage page `0xff72`, usage `0xa1`, report ID `0xe0`, 64-byte input payloads, 352-byte output payloads, and 10-byte feature payloads. Including the report ID, application buffers are **65, 353, and 11 bytes** respectively. Endpoint packet size does not equal HID report size. `src/device.rs` matches VID/PID, interface number, usage page, and usage, then opens that path; discovery itself changes no settings.

## Transport

Use standard HID APIs on Linux and Windows. Control commands use **Feature reports**, color data uses **Output reports**, and measured RPM uses **Get Input Report**. The first application-buffer byte is the `0xe0` report ID, not a leading zero. W-Connect pads reports to the descriptor lengths and leaves unused bytes zero.

For a libusb implementation, corresponding control requests would be:

| Operation | bmRequestType | bRequest | wValue | wIndex |
|---|---|---|---|---|
| Set feature | `0x21` | `0x09` | `0x03e0` | `1` |
| Set output | `0x21` | `0x09` | `0x02e0` | `1` |
| Get input | `0xa1` | `0x01` | `0x01e0` | `1` |

HIDAPI already performs this platform translation. There is no need to detach the kernel HID driver for the chosen hidraw backend or install a WinUSB replacement on Windows. The vendor inserts 20 ms between commands; W-Connect uses 30 ms and keeps each operation serialized. USB write success confirms transport completion, **not an application-level acknowledgment**. This device protocol exposes no independent acknowledgment of a speed or lighting change; RPM readback is the available cooling verification.

## Cooling commands

All hex sequences include the report ID and are padded to 11 bytes:

| Purpose | Meaningful bytes |
|---|---|
| Disable motherboard PWM sync for bank `c` | `e0 10 31 (10 << c)` |
| Enable motherboard PWM sync for bank `c` | `e0 10 31 (11 << c)` |
| Set raw duty percent `p` for bank `c` | `e0 (20 | c) 00 p` |
| Select RPM readback | `e0 50 00` |

Banks range `0..3`. The high nibble of the sync byte selects which bank changes; its corresponding low bit is the requested sync state. W-Connect disables PWM sync before issuing a bank's duty. It does not depend on a motherboard PWM cable being connected.

The vendor uses the **raw percent duty** directly. Some tools instead map a normalized UI range onto the SL fan's nominal 800–1900 RPM range using `floor((800 + 11 * percentage) / 19)`. That is a UI policy, not a required protocol transformation. W-Connect exposes raw duty, limits its own driver to `40..100`, and rejects values outside that range. The 40% floor is a conservative application policy, not a discovered firmware restriction. The vendor's own default curve goes lower.

To read RPM, select `e0 50 00`, wait, and request the 65-byte input report with report ID `e0`. Bytes `1–2`, `3–4`, `5–6`, `7–8` are the four unsigned big-endian RPM readings. Zero is a valid disconnected/stopped-bank result, not a transport failure. Short responses, wrong report ID, and implausible RPM data are errors, never fabricated telemetry. The read selector changes the readback page only; it does not change fan speed or lighting.

## Lighting commands

These are **SL v1** commands. SL-Infinity and AL use different ring/port layouts.

| Purpose | Report | Meaningful bytes |
|---|---|---|
| Disable motherboard ARGB sync | Feature | `e0 10 30 00` |
| Leave merged-bank lighting | Feature | `e0 10 34` |
| Set bank fan quantity | Feature | `e0 10 32 ((c << 4) | count)` |
| Upload bank colors | Output | `e0 (30 | c) [R B G]…` |
| Set effect | Feature | `e0 (10 | c) mode speed direction brightness` |
| Commit | Feature | `e0 60 00 01` |

The SL v1 has at most four fans per bank and sixteen LED addresses per fan. Static colors repeat each fan's color across its sixteen addresses. Color triplets are **red, blue, green**, not RGB or GRB. The remainder of the 353-byte output report is zero-filled.

W-Connect implements static (`mode=1`), breathing (`2`, available in the driver), rainbow (`5`), and off (static with brightness `8`). Effect speed `0` is medium; direction `0` is left-to-right. Brightness is discrete: `0` = 100%, `1` = 75%, `2` = 50%, `3` = 25%, `8` = off. The app rounds a requested brightness to these hardware levels. `preserve` sends no lighting commands.

Disabling motherboard sync and leaving merge mode are controller-wide lighting changes. They are necessary for independent bank effects and are applied only when the user requests a lighting change. Fan count is user configuration; it cannot be inferred reliably from one tachometer reading per bank.

## Platform requirements and operating policy

**Linux:** permit the selected HID node through a narrowly scoped udev rule matching vendor `0cf2` and product `a100`. Interactive desktop use can use `TAG+="uaccess"`; a background service requires a persistent group/user access arrangement as desktop ACLs may disappear at logout. Apply rules and retrigger this device or reconnect it. Never grant every HID device world-write access. A permission failure must show an actionable error while monitoring continues where possible.

**Windows:** use the native HID driver and enumerate interface `MI_01`. HIDAPI normally derives interface `1` from the USB hardware identifier. If its metadata lookup leaves interface `-1`, W-Connect accepts only an explicit `&mi_01#` or `&mi_01&` segment in the HID path, while retaining the VID/PID/usage checks. This follows the bundled HIDAPI 2.6.7 Windows enumeration implementation. No replacement USB driver is required. Stop L-Connect or other fan-control applications before running W-Connect, because independent programs writing to the same hub can override one another. Native HID support is independent of CPU/GPU temperature acquisition; Windows temperature collection needs an explicit supported sensor provider.

**Cooling policy:** software curves run on the host; this command set does not upload an autonomous temperature curve to the controller. Losing the process or power state may leave the last commanded speed in place. The service must implement fresh-sensor checks, emergency full duty, reconnect handling, and a deliberate exit policy. A crash or sudden power loss cannot be made safe by a destructor alone. Restoring motherboard PWM on exit is only appropriate if that cable and motherboard curve are known to exist.

## Prior art and discrepancies

1. [EightB1ts/uni-sync, `devices/mod.rs`](https://github.com/EightB1ts/uni-sync/blob/05f39462178b60f590bfd9daa3b4863038c0f6ad/src/devices/mod.rs), MIT. Establishes the `a100` identity, sync bit masks, speed register, and cross-platform HID feasibility. It writes commands as output reports and normalizes its speed UI. W-Connect follows the vendor's feature-report transport and raw percentage instead.
2. [liquidctl, `lianli_uni.py`](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/lianli_uni.py), GPL-3.0-or-later. Independent corroboration of fan status byte offsets and fixed-speed commands. Its initialization enables motherboard PWM synchronization; calling that routine blindly would therefore not be appropriate here. No liquidctl code is bundled.
3. [sgtaziz/lian-li-linux, `ene6k77/controller.rs`](https://github.com/sgtaziz/lian-li-linux/blob/896593b3b84f0d9225471222a319c4c95144c635/crates/lianli-devices/src/ene6k77/controller.rs), MIT. Corroborates vendor feature controls, RPM selector, fan quantity encoding, commit, RBG order, and hardware effect settings. Its broader libusb transport detaches HID drivers; W-Connect does not need that architecture for this specific HID controller.
4. [OpenRGB's dedicated SL controller](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/LianLiController/LianLiUniHubSLController/LianLiUniHubSLController.cpp) and [constants](https://github.com/CalcProgrammer1/OpenRGB/blob/master/Controllers/LianLiController/LianLiUniHubSLController/LianLiUniHubSLController.h), GPL-2.0-only. Explicitly discusses the same `LianLi-UNI FAN-SL-v1.8` product string and confirms SL color order, effect values, brightness and quantity. Its output-report implementation has a pointer-size send-length issue in the inspected color routine, so it is evidence for byte layout rather than a transport template. No OpenRGB code is bundled.
5. [EightB1ts/FanControl.LianLi](https://github.com/EightB1ts/FanControl.LianLi), a Windows implementation, corroborates native HID input report retrieval and the same SL speed/status commands.

Research checkouts and vendor binaries live under ignored `.research/` only. The implementation is independently written from the documented protocol facts; proprietary vendor code and firmware are not distribution assets.

## Non-mutating diagnostic

With other controller applications stopped, run `cargo run --example probe --locked`. This opens the matched hub, selects the RPM readback page, and prints identity and all four measured speeds. It does not set duty, sync mode, fan quantity, or lighting. The example is deliberately separate from the daemon so diagnosis does not initialize a cooling profile. `cargo test --example probe --locked` runs protocol tests without USB access.
