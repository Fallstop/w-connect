# Validation record — 6 November 2026

## Actual Linux hardware

Host: Manjaro Linux x86_64; UNI FAN SL v1.8, USB `0cf2:a100`, interface1. CPU temperature via k10temp; GPU NVIDIA RTX3090 via nvidia-smi.

- Read-only diagnostic opened the actual HID interface and read populated banks1/3 at1470/1335 RPM, banks2/4 at0 RPM.
- Native control process successfully commanded100% on all four banks. Populated banks rose to approximately1815 RPM; subsequent samples varied around1755–1815 RPM.
- CPU read89–90°C during validation and GPU45–50°C. Thermal protection held all banks at100%; no lower-speed sweep was performed under this workload. Curve interpolation and cooldown smoothing were tested in software.
- Static red on bank1 and blue on bank3 were confirmed with the supplied camera. Bank1 maps to top exhaust; bank3 maps to bottom intake. Those names were saved. Rainbow lighting was restored afterward. Camera credentials and footage are excluded from source/distribution.
- Local USB access rule installed, scoped to this exact VID/PID. App runs unprivileged.
- Linux user service installed and enabled. Clean stop followed by read-only probe confirmed populated banks remained at full cooling, then the service restarted and reconnected successfully.
- HTTP tests rejected missing mutation header, foreign Origin, foreign Host and duty0 config. Emergency latch/resume worked and did not change saved config.
- Optimized Linux executable approximately1.5MB. One idle observation showed approximately5MB resident memory; sensor subprocesses and browser memory are additional. This is an observation, not a sustained benchmark.

## Automated validation

- Eighteen Rust unit tests (current source): packet encoding and channel selection, descriptor matching, RPM parsing/rejection, RBG lighting, fan-count limits, thermal override/missing sensors/stale data, ramp-down, interpolation, configuration validation and origin protection.
- Sixteen DOM behavior tests (current source): real/stale telemetry, draft preservation, save headers, emergency/resume, curve constraints, demo mode, unsupported lighting, critical-temperature notices.
- Rust formatting, Clippy with warnings denied, JavaScript syntax and release build.
- Review by separate agents cross-checked protocol against vendor decompilation and prior art, then examined control-loop safety and installers.

## Design refinement and extended support

The original mixed-functionality design ratings were superseded by [fresh-context aesthetic reviews](design-review-neutral.md). Those ratings remain below the requested 9.5 threshold; passing behavior and accessibility checks is not an aesthetic score.

The neutral-theme revision passed browser checks across six view/theme combinations and 360, 390, 768, 1280 and 1440 px widths. Status changes (normal, hot, sensor unavailable, connection lost, recovery) left header and cooling-control geometry unchanged. The action footer did not overlap scrolling content. Keyboard curve editing, draft retention and reduced-motion handling passed. Axe reported zero A/AA findings. Both fonts are embedded, licensed and served locally. The extracted Linux package passed its USB-free smoke test with the new font routes.

Additional driver tests cover exact family-specific PWM registers, legacy output transport, V2 RPM offsets, independent lighting rings, six-fan quantities, frame commits, spectrum palettes, descriptor validation, unknown-ID rejection, opt-in and unique selection. UI tests cover explicit activation, six-fan capability refresh and ensuring imports do not grant untested-driver permission. The original SL transfers remain covered by regression fixtures. A post-expansion browser scan, including the expanded controller catalog, also reported zero A/AA findings across all six view/theme combinations. The installed0.2 release reconnected to the actual SL hub, read both populated banks, returned the seven-model catalog with untested support disabled, and rejected a six-fan configuration for the four-fan SL model before any hardware change.

## Controller-resident lighting

SL v1 `0cf2:a100` was tested again with native breathing on the top bank while the bottom bank retained rainbow. After a clean service stop, process lookup confirmed that no `w-connect` process remained. The camera recorded 30 seconds at two frames per second: the top frame strips completed repeated red breathing cycles, and the bottom rainbow continued moving. This demonstrates controller-resident effects while powered, independently of both UI and service. The preview now uses SL frame strips, and its breathing period approximates the observed cycle.

Restarting the service loaded and applied the saved breathing profile. The original rainbow profile was then restored and confirmed identical to the pre-test configuration. The populated banks read approximately 1800/1770 RPM afterward; clean stop had requested full cooling. Camera footage and credentials remain private and excluded from source/distribution.

No system shutdown, SATA-power removal or firmware flash was performed. Controller persistence after complete power removal remains unverified; no nonvolatile-save command was found. See [controller-memory evidence](protocol.md#controller-memory-and-application-exit).

## Colour picker and camera checks (2026-11-07)

The inline hue/saturation/value picker was tested with pointer dragging outside the field, touch input at desktop and mobile widths, native keyboard controls, exact hex values, and transitions through black and white without losing the chosen hue. Edits remain drafts until Save. Browser checks found no A/AA violations across the three views in both themes, no horizontal overflow at 360–1440px, and no JavaScript errors. Lighting screenshots include the picker.

The camera was used for repeated native red, green, blue, purple, static, and breathing checks, including a 24-second rainbow-to-green-breathing transition. Primary colour selections changed the actual strips; green breathing remained green across repeated cycles after rainbow. The saved purple (`#613583`) looked pale and uneven between strips on camera, but did not cycle through rainbow hues in these captures. This does **not** establish a calibrated match between the screen and LEDs or resolve every reported colour mismatch. No speculative colour correction or transport change was shipped. The preview breathing period was adjusted to approximately seven seconds with a fully dark minimum, matching these captures more closely.

A temporary batch-upload experiment did not improve the purple appearance and was removed. The original saved profile and cooling settings were restored after testing. Camera recordings, URLs and configuration backups remain private in ignored runtime files.

## Windows

The initial Rust application successfully cross-built for Windows x64 with the HIDAPI C backend. Windows-target tests compiled with `--no-run`. The cross-built executable is1.35MiB and imports Windows system DLLs only; it requires no MinGW runtime DLL. The public release workflow additionally builds with MSVC and runs unit, UI and extracted-package smoke tests on a Windows runner. Consult the linked Actions run for each release's result; CI has no physical controller attached.

The temperature helper was restored with locked packages and cross-published for Windows x64 without warnings. A managed-entrypoint Linux smoke test returned unavailable sensors as JSON as intended. The helper is approximately36MB including its compressed .NET runtime. Windows needs PawnIO and elevation for CPU temperature access. Windows USB, sensors, installer, logoff and scheduled-task behavior require testing on actual Windows hardware; Linux tests do not prove them.

## Boundaries

Only SL v1 `0cf2:a100` has been physically tested. Six additional IDs now have opt-in experimental profiles with model-specific packet tests; none were connected or claimed as hardware-validated. Firmware-update, pump, LCD and wireless interfaces remain excluded. Lighting brightness steps are derived from verified commands; calibrated photometric brightness was not measured. No physical USB unplug or abrupt-OS-failure test was performed. An abrupt kill or USB failure can leave the hub at its last duty because no firmware watchdog is known. A blocked USB control loop makes telemetry unavailable after ten seconds; the UI does not continue to claim fresh readings.
