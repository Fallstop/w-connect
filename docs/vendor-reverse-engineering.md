# L-Connect 3 interoperability notes

Inspected 6 November 2026. The supplied installer was treated as an archive; no vendor executable, service, driver, or firmware was run. These are protocol observations, not redistributed vendor implementation code.

## Evidence and extraction

Source: `20260924-L-Connect 3-x64-v2.1.29-bc599de7.exe`, 1,515,066,872 bytes, SHA-256 `2e6535db640ef3ba3185ca5f81a33316abe97e599cd890e3983974a3774c6138`.

The installer is NSISBI with flags `0x70`. Stock 7-Zip 26.03 rejects its NSIS framing, but extracts the PE overlay as `[0]`. The overlay has a 36-byte first header followed by independent LZMA chunks framed with little-endian 24-bit lengths. Inserting an unknown-length 8-byte field after each chunk's five LZMA properties bytes permits Python's standard `lzma` decoder to read it. The first decoded chunk is an 8-byte length followed by a 112,062-byte compiled installer header. Subsequent decoded chunks form the file data block; each file has an 8-byte little-endian length prefix. Header file references use 36-byte NSISBI instruction records and 64-bit data offsets.

Extraction scripts, indexed names and offsets, original binaries and ILSpy 9.1.0 decompilation are retained only in ignored `.research/vendor/`. Useful extracted files:

| File | Data-block offset | Bytes | SHA-256 |
| --- | ---: | ---: | --- |
| L-Connect.Core.dll | 210273619 | 563056 | `4d11f179841fd9877b0cc79fa85e446625367a4ea91327edc0c0f3d6ed9ec600` |
| L-Connect-Service.exe | 207727075 | 2543984 | `c3f5fed2ca352c016c880e49995b6705e79a3ad1d97f44c1e792277ca04a023a` |

The application uses .NET/WPF, CefSharp, media libraries and separate Windows services. Its hardware layer calls `hidapi.dll`; the USB device does not require the entire UI stack.

## UNI FAN SL v1 protocol, USB 0cf2:a100

Primary evidence: `.research/vendor/decompiled/core/LConnectCore.Products.Ene6K77Fan/SLFanDevice.cs`, `LConnectCore.Devices/KnownDevices.cs`, and `.research/vendor/decompiled/service/LConnectService.Products.Ene6K77Fan/SLFanController.cs`. `KnownDevices` explicitly maps decimal VID 3314 / PID 41216 to `SLFan`.

Bytes below are hexadecimal, including report ID `E0`. Port/group indexes are zero-based 0–3. The vendor waits 20 ms after feature/output writes. All non-lighting commands here use **feature reports**, not output reports.

| Operation | Report bytes | Observation |
| --- | --- | --- |
| Set fan speed | `E0 (20\|port) 00 duty` | `duty` is the raw percentage, not an RPM-derived scaling |
| Request RPM | `E0 50 00` | Follow with **GET_INPUT_REPORT**, 65-byte buffer beginning `E0` |
| RPM response | `E0 hi0 lo0 hi1 lo1 hi2 lo2 hi3 lo3 …` | Four unsigned big-endian RPMs |
| Request firmware | `E0 50 01` | Follow with GET_INPUT_REPORT; separate from RPM selector |
| Set motherboard fan sync | `E0 10 31 mask 00 00` | `mask = (1 << (port + 4)) \| (enabled << port)` |
| Set motherboard lighting sync | `E0 10 30 enabled 00 00` | Boolean 0/1 |
| Set fan count | `E0 10 32 count 00 00` | `count = (port << 4) \| quantity`; quantity 0–4 |
| Set lighting colors | `E0 (30\|port) R B G …` | **Output report**, RBG order (not RGB or GRB) |
| Set lighting effect | `E0 (10\|port) mode speed direction brightness` | Feature report |
| Synchronize lighting frame | `E0 60 00 01` | Vendor sends frame 1 after applying effects |
| Identify port | `E0 (10\|port) 11 FF 00 02` | Vendor's temporary ping effect |
| Stop merged effects | `E0 10 34 00 00 00` | Individual port effects may need this after Windows merge mode |

The C# vendor calls pass variable command lengths to HIDAPI. On Windows, HIDAPI handles report padding; W-Connect uses the connected device's descriptor-sized reports (11 bytes for feature, 353 bytes for output, 65 bytes for input). The driver must not assume Linux interrupt `read()` is equivalent to the vendor's explicit input report request.

`SLFanController.startWriteFanSpeed` evaluates a piecewise linear temperature/percentage curve once per second and directly passes the result into `SetFanSpeed`. There is no `(800 + 11 * percentage) / 19` conversion in this path. Vendor default points are `(0°C,10%)`, `(25,20)`, `(40,50)`, `(55,62)`, `(70,75)`, `(90,100)`. W-Connect deliberately enforces its own conservative operating floor; the vendor's low values are not evidence that every connected fan starts safely at those values.

Lighting static mode is `01`, breathing `02`, rainbow morph `04`, rainbow `05`, color cycle `23`. Speed is `02` slowest, `01` slow, `00` normal, `FF` faster, `FE` fastest. Direction is `00` right / `01` left. Brightness is inverted: `00` highest, `01` higher, `02` normal, `03` lower, `08` off. For this SL model, the service normalizes nominal brightness `04` to off.

Static and breathing payloads always contain 64 LED colors: four fan slots × 16 LEDs. Unspecified slots are black. Other effects use 16 colors, consisting of four repeated sets of four selected colors. Fan count is configured separately. These observations apply to the SL v1 class; adjacent PID variants have separate vendor implementations and must not be silently treated as compatible.

## Windows approach and limits

Use the operating system HID API through the cross-platform HIDAPI crate. Do not replace the standard HID driver with WinUSB merely to send these commands. Temperature reading is independent of controller access; W-Connect's Windows helper uses LibreHardwareMonitor, with its own documented CPU access requirements.

No firmware update protocol was implemented. No hardware watchdog was demonstrated in the inspected SL path. Host temperature polling, stale-sensor handling, reconnect and exit behavior must therefore be explicit. Static reverse engineering establishes command semantics; physical device tests remain necessary to verify transport and actual behavior.

Extraction format cross-checks: [7-Zip NSIS parser](https://github.com/ip7z/7zip/blob/main/CPP/7zip/Archive/Nsis/NsisIn.h) documents the NSISBI flag extensions; [NSISExtractor](https://github.com/KokerZhou/NSISExtractor) documents NSISBI framing and instruction layouts. No third-party or vendor source was copied into the runtime implementation.
