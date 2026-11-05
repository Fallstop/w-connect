# Independent evidence for additional UNI FAN controllers

Reviewed 6 November 2026. This review inspected the supplied L-Connect 3 v2.1.29 extraction and primary open-source driver implementations. **No additional controller was connected or tested, and this review sent no USB commands.** Protocol evidence is distinct from hardware validation.

## Identity and cooling matrix

All IDs below use VID `0cf2`. Banks are zero-based in commands (`g = 0..3`). Disable motherboard PWM synchronization before setting a bank's raw duty; the high nibble selects the bank, while the low nibble's bit chooses synchronization. Thus software control uses `0x10 << g`.

| PID | Family | Software PWM command | Duty command | RPM layout, including report ID | Evidence |
| --- | --- | --- | --- | --- | --- |
| `a100` | UNI FAN SL v1 | `E0 10 31 (10<<g)` | `E0 (20\|g) 00 duty` | Big-endian pairs at offsets 1, 3, 5, 7 | Vendor SLFanDevice; actual parent-session hardware validation |
| `7750` | Legacy UNI FAN SL | Same command bytes as SL, sent as **output** reports | Same raw command format, output transport | Big-endian pairs at offsets 1, 3, 5, 7 | uni-sync and liquidctl; absent from this vendor release's KnownDevices map |
| `a101` | UNI FAN AL v1 | `E0 10 42 (10<<g) 00 00` | `E0 (20\|g) 00 duty` | Big-endian pairs at offsets 1, 3, 5, 7 | Vendor ALFanDevice + liquidctl + lian-li-linux |
| `a102` | UNI FAN SL-Infinity | `E0 10 62 (10<<g) 00 00` | Same duty format | Big-endian pairs at offsets 1, 3, 5, 7 | Vendor SLInfinityDevice + liquidctl + lian-li-linux |
| `a103`, `a105` | UNI FAN SL V2 / SL V2A | `E0 10 62 (10<<g) 00 00` | Same duty format | **Extra payload status byte:** pairs at offsets 2, 4, 6, 8 | Vendor SLV2FanDevice + liquidctl + lian-li-linux |
| `a104` | UNI FAN AL V2 | `E0 10 62 (10<<g) 00 00` | Same duty format | **Extra payload status byte:** pairs at offsets 2, 4, 6, 8 | Vendor ALV2FanDevice + liquidctl + lian-li-linux |

The vendor's `KnownDevices.cs` maps `a106` to **SL Redragon**, not AL V2. Its presence does not justify adding unrelated AIO, TL, wireless, display or firmware interfaces to an ENE fan driver. `MainService.cs` dispatches both SLV2Fan and SLV2AFan to the same `SLV2FanDevice` and `SLV2FanController` implementation.

Modern vendor drivers query RPM by sending feature command `E0 50 00`, then requesting a 65-byte input report whose report ID is `E0`. Their helper strips the report ID before the model-specific parser. SLV2 and ALV2 then skip another payload byte, which is why a shared offset-1 parser would be wrong. Minimum valid returned lengths must include the report ID: nine bytes for the older layout, ten for V2. A short report must remain an error, never be converted to zero RPM.

For legacy `7750`, liquidctl directly requests input report `E0`, length 65, without issuing the vendor's feature selector. The installed vendor release does not prove feature-report compatibility for this legacy PID. Using the established output transport and direct input request is the evidence-backed legacy path; it should remain explicitly untested.

The vendor sends raw duty bytes. Some older tools expose a normalized speed scale and transform it into a raw duty byte; those transformations are UI conventions, not evidence that firmware accepts a different wire encoding. A conservative application floor of 40% is compatible with the verified SL policy but is not a calibration of minimum speed on every new model.

## Transport length and interface identification

The vendor device classes call `SendFeatureReport(data, data.Length)` with three bytes for status selection, four for duty and six for modern PWM mode selection. `HidApiHidDevice.SendFeatureReport` forwards that length directly. The HIDAPI 2.6.7 Windows C backend (`windows/hid.c`, function `hid_send_feature_report`) pads short feature buffers to the device descriptor's `FeatureReportByteLength`; therefore, short application-level requests do not require guessing a universal eleven-byte feature size.

lian-li-linux also sends these short buffers unchanged through its ENE controller and Linux hidraw transport. Its `HidrawTransport::send_feature_report` forwards the slice to hidapi. Windows padding and Linux short requests are therefore independently accounted for. The `a100` eleven-byte feature and 353-byte output behavior was validated against the attached hardware and should not be changed merely to normalize new code.

A universal interface-1 / usage-page `ff72` / usage `a1` rule is **not established by the reviewed evidence**. Those values were observed on the attached `a100`. The vendor's ENE KnownDevices entries have no usage constraint, its ControllerFinder adds usage filters only for other product families, uni-sync filters the ENE candidates by exact VID/PID, and lian-li-linux registers these ENE PIDs with no usage-page filter. Do not silently reuse the attached hub's metadata as proof for every model. Where metadata differs, checking the actual HID report descriptor for the expected `E0` input and command capabilities is more defensible than accepting all vendor HID interfaces. If multiple candidate interfaces or controllers remain ambiguous, require a deterministic selection and avoid writes.

## Lighting is model-specific

Every reviewed vendor ENE `SetColorSetting` method orders bytes **red, blue, green**. That common fact does not make the complete lighting protocol interchangeable.

| Family | Quantity encoding | Lighting structure and other differences |
| --- | --- | --- |
| SL v1 | `E0 10 32 ((g<<4)\|count)` | Single lighting port per bank, 16 LEDs per fan; tested implementation uses commit frame 1. |
| AL v1 | `E0 10 40 (g+1) count 00` | Two logical ports per bank (`g*2`, `g*2+1`); 8 inner + 12 outer LEDs per fan. RGB motherboard sync is register `41`; merge switch is `43`. |
| SL-Infinity | `E0 10 60 (g+1) count 00` | Two logical ports, 8 inner + 12 outer LEDs; sync register `61`, merge-order register `63`; commit frame 1. |
| SL V2 / V2A | `E0 10 60 ((g<<4)\|count) 00 00` | Single lighting port, 16 LEDs per fan, up to six fans; vendor static buffers cover 96 LEDs and commit frame **4**. |
| AL V2 | `E0 10 60 (g+1) count 00` | Two logical ports with six-fan capacity; sync `61`, merge order `63`, commit frame 1. Combined effects are not always the same IDs as inner/outer effects. |

For example, AL V2's combined Rainbow maps to `0x2b`, while separate inner and outer Rainbow each map to `0x05`. AL v1 combined Rainbow maps to `0x28`. Blindly changing only PID and keeping SL's whole-bank effect command would be incorrect. SLV2's static/rainbow modes are 1/5, but its quantity and frame commit still differ.

Keep lighting disabled for a new profile unless the implementation covers its complete port mapping, quantity encoding, effect selection, color payload and commit sequence with packet tests. Cooling/RPM support can be complete while that model's lighting remains explicitly unsupported and untouched.

## Reproducible source evidence

The local extraction is described in [vendor-reverse-engineering.md](vendor-reverse-engineering.md). Relevant paths below are under ignored `.research/vendor/decompiled/`; no vendor code is redistributed by this document:

- `core/LConnectCore.Devices/KnownDevices.cs`, lines 17–23: exact ENE PID identities.
- `core/LConnectCore.Devices/ControllerFinder.cs`, lines 18–28: family-specific usage filtering.
- `core/LConnectCore.Devices/HidApiHidDevice.cs`, `Open` and `SendFeatureReport`: descriptor lengths and unchanged feature forwarding.
- `core/LConnectCore.Products.Ene6K77Fan/{SLFanDevice,ALFanDevice,SLInfinityDevice,SLV2FanDevice,ALV2FanDevice}.cs`: `GetFanSpeed`, `SetFanSpeed`, `SetFanMotherboardSync`, `SetQuantity`, `SetColorSetting`, and `getInputReportData`.
- `service/LConnectService/MainService.cs`, lines 1458–1461: SL V2A dispatches through SL V2's implementation.
- `service/LConnectService.Products.Ene6K77Fan/SLV2FanController.cs`, methods `syncLightingFrame`, `convertToLEDColor`, and lighting lookups: frame 4, 96 LEDs and static/rainbow modes.
- Equivalent AL/SLInfinity/ALV2 service controllers contain dual-ring palette conversion and effect lookup tables.

Primary prior-art snapshots inspected:

- [liquidctl driver, commit 292dc56](https://github.com/liquidctl/liquidctl/blob/292dc561361cfbc0cdee155c26662131fe160989/liquidctl/driver/lianli_uni.py): exact PID list, PWM register matrix, RPM offsets and legacy output writes.
- [uni-sync driver, commit 05f3946](https://github.com/EightB1ts/uni-sync/blob/05f39462178b60f590bfd9daa3b4863038c0f6ad/src/devices/mod.rs): legacy PID and output-command evidence.
- [lian-li-linux ENE controller, commit 896593b](https://github.com/sgtaziz/lian-li-linux/blob/896593b3b84f0d9225471222a319c4c95144c635/crates/lianli-devices/src/ene6k77/controller.rs), [model metadata](https://github.com/sgtaziz/lian-li-linux/blob/896593b3b84f0d9225471222a319c4c95144c635/crates/lianli-devices/src/ene6k77/model.rs), and [hidraw transport](https://github.com/sgtaziz/lian-li-linux/blob/896593b3b84f0d9225471222a319c4c95144c635/crates/lianli-transport/src/hidraw.rs): independent modern feature transport, V2 RPM layout, fan capacities and per-model lighting differences.
- [HIDAPI Windows transport](https://github.com/libusb/hidapi/blob/hidapi-0.15.0/windows/hid.c): `hid_send_feature_report` padding behavior, also inspected in the locally compiled Rust hidapi 2.6.7 dependency's bundled C source.

The strongest source is the supplied vendor's exact model implementation. Prior art confirms interoperability behavior and exposes older hardware variants. Neither source substitutes for testing the additional physical controllers.
