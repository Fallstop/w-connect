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

- Sixteen Rust unit tests (0.2): packet encoding and channel selection, descriptor matching, RPM parsing/rejection, RBG lighting, fan-count limits, thermal override/missing sensors/stale data, ramp-down, interpolation, configuration validation and origin protection.
- Fourteen DOM behavior tests (current source): real/stale telemetry, draft preservation, save headers, emergency/resume, curve constraints, demo mode, unsupported lighting, critical-temperature notices.
- Rust formatting, Clippy with warnings denied, JavaScript syntax and release build.
- Review by separate agents cross-checked protocol against vendor decompilation and prior art, then examined control-loop safety and installers.

## Design refinement and extended support

The original mixed-functionality design ratings were superseded by [fresh-context aesthetic reviews](design-review-neutral.md). Those ratings remain below the requested 9.5 threshold; passing behavior and accessibility checks is not an aesthetic score.

The neutral-theme revision passed browser checks across six view/theme combinations and 360, 390, 768, 1280 and 1440 px widths. Status changes (normal, hot, sensor unavailable, connection lost, recovery) left header and cooling-control geometry unchanged. The action footer did not overlap scrolling content. Keyboard curve editing, draft retention and reduced-motion handling passed. Axe reported zero A/AA findings. Both fonts are embedded, licensed and served locally. The extracted Linux package passed its USB-free smoke test with the new font routes.

Additional driver tests cover exact family-specific PWM registers, legacy output transport, V2 RPM offsets, independent lighting rings, six-fan quantities, frame commits, spectrum palettes, descriptor validation, unknown-ID rejection, opt-in and unique selection. UI tests cover explicit activation, six-fan capability refresh and ensuring imports do not grant untested-driver permission. The original SL transfers remain covered by regression fixtures. A post-expansion browser scan, including the expanded controller catalog, also reported zero A/AA findings across all six view/theme combinations. The installed0.2 release reconnected to the actual SL hub, read both populated banks, returned the seven-model catalog with untested support disabled, and rejected a six-fan configuration for the four-fan SL model before any hardware change.

## Windows

The initial Rust application successfully cross-built for Windows x64 with the HIDAPI C backend. Windows-target tests compiled with `--no-run`. The cross-built executable is1.35MiB and imports Windows system DLLs only; it requires no MinGW runtime DLL. The public release workflow additionally builds with MSVC and runs unit, UI and extracted-package smoke tests on a Windows runner. Consult the linked Actions run for each release's result; CI has no physical controller attached.

The temperature helper was restored with locked packages and cross-published for Windows x64 without warnings. A managed-entrypoint Linux smoke test returned unavailable sensors as JSON as intended. The helper is approximately36MB including its compressed .NET runtime. Windows needs PawnIO and elevation for CPU temperature access. Windows USB, sensors, installer, logoff and scheduled-task behavior require testing on actual Windows hardware; Linux tests do not prove them.

## Boundaries

Only SL v1 `0cf2:a100` has been physically tested. Six additional IDs now have opt-in experimental profiles with model-specific packet tests; none were connected or claimed as hardware-validated. Firmware-update, pump, LCD and wireless interfaces remain excluded. Lighting brightness steps are derived from verified commands; calibrated photometric brightness was not measured. No physical USB unplug or abrupt-OS-failure test was performed. An abrupt kill or USB failure can leave the hub at its last duty because no firmware watchdog is known. A blocked USB control loop makes telemetry unavailable after ten seconds; the UI does not continue to claim fresh readings.
