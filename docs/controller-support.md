# Controller support

Only the attached **UNI FAN SL controller (`0cf2:a100`) has been tested on physical hardware**. The additional drivers below implement the model-specific commands from L-Connect 3 v2.1.29 and established Linux drivers. They have packet and parser tests, but remain explicitly untested until exercised on their respective controllers.

| Controller | USB PID, VID `0cf2` | Cooling / RPM | Lighting | Fans per bank | Hardware validation |
| --- | --- | --- | --- | --- | --- |
| UNI FAN SL | `a100` | Yes | Preserve, static, rainbow, off | 1–4 | Tested |
| UNI FAN SL, legacy | `7750` | Yes | Preserve only | 1–4 | Untested; opt-in required |
| UNI FAN AL | `a101` | Yes | Preserve, static, rainbow, off; both rings | 1–4 | Untested; opt-in required |
| UNI FAN SL-Infinity | `a102` | Yes | Preserve, static, rainbow, off; both rings | 1–4 | Untested; opt-in required |
| UNI FAN SL V2 | `a103` | Yes | Preserve, static, rainbow, off | 1–6 | Untested; opt-in required |
| UNI FAN AL V2 | `a104` | Yes | Preserve, static, rainbow, off; both rings | 1–6 | Untested; opt-in required |
| UNI FAN SL V2A | `a105` | Yes | Same protocol as SL V2 | 1–6 | Untested; opt-in required |

These are the lighting modes exposed by the application. Four independent fan banks are available on each profile. Cooling uses raw duty percentages with a conservative 40% floor, rather than extrapolating minimum speeds for untested fans. The floor is an application policy, not a measurement of every model's starting duty.

## Selecting an additional controller

Settings exposes controller selection and an explicit untested-controller toggle. The corresponding configuration defaults are:

```json
"controller": {
  "allow_untested": false,
  "serial": null
}
```

With the default configuration, only the tested `a100` profile can be opened for control. Set `allow_untested` to `true` to enable the additional profiles. Set `serial` to a discovered controller's serial to select it explicitly. W-Connect controls one hub at a time. Multiple eligible controllers or interfaces with the same serial are rejected rather than chosen by enumeration order; connect one at a time if they cannot be uniquely identified. An explicit selection never silently falls back to another controller.

`GET /api/devices` returns known controller candidates and the support registry. Discovery enumerates USB metadata without opening handles or sending commands. Opening an untested controller reads its HID report descriptor before sending any fan or lighting commands. This verifies report ID `E0`, a 65-byte input report, and the appropriate output/feature command capacity. Incompatible descriptors produce a clear error. Lighting is disabled if the output report cannot accommodate the model's full color buffer; cooling can still operate when its descriptor is valid.

Unknown product IDs are excluded. The tested `a100` retains its observed interface-1 / usage-page `ff72` / usage `a1` matching. Other registered IDs require a vendor-defined usage collection, or explicit interface 1 when usage metadata is unavailable, plus the descriptor validation above. We do not assume that every Lian Li device shares the tested hub's descriptor. The current vendor maps `a106` to SL Redragon, not AL V2; it is intentionally not supported. TL, wireless, LCD/display, AIO and pump controllers are outside this driver.

## Protocol differences implemented

| Family | PWM sync register | RPM byte offsets, including report ID | Quantity encoding | Lighting ports / frame commit |
| --- | --- | --- | --- | --- |
| SL | `31` | 1, 3, 5, 7 | `32`, bank in high nibble, quantity in low nibble | One port per bank / frame 1 |
| AL | `42` | 1, 3, 5, 7 | `40`, one-based bank, separate quantity | Inner + outer ports / frame 1 |
| SL-Infinity | `62` | 1, 3, 5, 7 | `60`, one-based bank, separate quantity | Inner + outer ports / frame 1 |
| SL V2 / V2A | `62` | **2, 4, 6, 8** | `60`, bank in high nibble, quantity in low nibble | One port per bank / **frame 4** |
| AL V2 | `62` | **2, 4, 6, 8** | `60`, one-based bank, separate quantity | Inner + outer ports / frame 1 |

All duty commands use `E0 (20|bank) 00 duty`. The legacy `7750` sends its four-byte mode and duty commands through output reports and requests the input report directly. Modern profiles use feature commands, including `E0 50 00` before the RPM input request. The added modern profiles preserve the vendor's short three-, four- and six-byte requests, also used by lian-li-linux; HIDAPI pads feature transfers to the descriptor length on Windows. The tested `a100` keeps its existing eleven-byte feature and 353-byte output transfers unchanged.

Lighting uploads red/blue/green bytes. SL V2 reserves the full 96-LED buffer. AL and SL-Infinity use separate 8-LED inner and 12-LED outer sections per fan; AL V2 extends those sections to six fans. Dual-ring rainbow selects the vendor's independent ring effect `05`, not the different combined-effect IDs. Static/off/breathing use full per-LED buffers, while extended rainbow uses the vendor's four-color palettes per fan (six for AL V2). The declared output descriptor size supplies zero padding for the color transfer.

`preserve` returns without sending lighting commands, including on the legacy profile. Legacy lighting remains unavailable because the current vendor release does not identify that controller and cooling evidence alone does not establish its complete RGB transaction. A fan-count or RGB incompatibility must not prevent the application from maintaining cooling.

## Evidence and validation

The [independent protocol evidence](extended-protocol-evidence.md) records the supplied vendor classes, exact PID mapping, transport details and pinned primary open-source comparisons. The [vendor extraction notes](vendor-reverse-engineering.md) explain the reproducible local analysis. Vendor source and the installer are not redistributed with W-Connect.

Unit tests cover exact model-specific mode/duty bytes across all four banks, the V2 RPM offset, short and malformed RPM rejection, unknown IDs, opt-in and multi-controller selection, quantity/ring/frame layout, brightness and fan-count bounds, HID descriptor parsing, and preservation of the tested SL wire format. These tests use byte fixtures and never access the hardware. They establish implementation consistency with the inspected protocol, not physical validation of the additional models.
