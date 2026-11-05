# Windows temperature helper

The Rust controller uses native Windows HID support. CPU/GPU temperatures come from the optional `w-connect-sensors.exe` helper, built from this folder and placed next to `w-connect.exe`.

The helper uses **LibreHardwareMonitorLib 0.9.6**, pinned with a NuGet lock file. This was the latest stable package checked on 6 November 2026. It enables only CPU and GPU sampling, emits exactly one JSON line, and exits. There are no Lian Li DLLs or services in this helper. Package information: [NuGet](https://www.nuget.org/packages/LibreHardwareMonitorLib/0.9.6), [source and license](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/tree/v0.9.6).

## Build

With .NET SDK 8 or newer, from the repository root:

```powershell
dotnet restore windows/Sensors/WConnect.Sensors.csproj --locked-mode
dotnet publish windows/Sensors/WConnect.Sensors.csproj -c Release --no-restore -o dist/windows
```

Copy `dist/windows/w-connect-sensors.exe` next to `w-connect.exe`, retaining the accompanying license/notice files. The helper includes the pinned .NET 8.0.33 runtime; end users do not need the SDK or runtime installed. Native dependencies are extracted by .NET as needed. Single-file compression reduces the optional helper's size. Trimming is disabled because hardware detection uses dynamic/native bindings.

## CPU access

LibreHardwareMonitor 0.9.6 uses the **PawnIO driver** for CPU register access. Install it from the [official PawnIO project](https://github.com/namazso/PawnIO), then start the main W-Connect process with administrator privileges. Windows security policy and hardware support may still prevent particular sensors from being available. GPU driver APIs may work without elevation.

The helper's manifest deliberately specifies `asInvoker`: it inherits the parent's privileges and will not prompt for UAC on every sample. For automatic startup, run the parent through Task Scheduler with **Run with highest privileges**. Close L-Connect while W-Connect owns the controller.

Missing CPU readings are returned as `null`, with a diagnostic `source`; they must trigger the controller's configured sensor-failure policy. Do not substitute ACPI thermal-zone temperature for CPU package temperature.

## Output contract

```json
{"cpu":58.5,"gpu":49,"source":"LibreHardwareMonitor 0.9.6"}
```

Values are Celsius. CPU is the maximum current CPU temperature (package/core/CCD), excluding distance-to-TjMax and thermal-margin readings. GPU is the maximum current GPU temperature, including exposed hotspot/memory sensors. This is conservative for case cooling. Multiple CPUs/GPUs contribute to the respective maximum. Nonfinite, zero, negative and above-150°C readings are unavailable.

The sampling deadline is 2.8 seconds after managed startup; timeout returns both readings as `null`. A blocked driver cannot keep this short-lived worker process alive after its main entry point exits. The parent should additionally enforce its seven-second process deadline, covering runtime startup and pathological driver behavior. Diagnostic output goes to stderr; stdout remains JSON.

Cross-publishing was verified from Linux. Windows hardware, driver installation, UAC inheritance and the three-second typical sampling target require validation on an actual Windows boot.

## Distribution

LibreHardwareMonitor is MPL-2.0 and includes third-party components. Preserve the upstream [license](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/v0.9.6/LICENSE) and [third-party notices](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/v0.9.6/THIRD-PARTY-NOTICES.txt) when distributing this helper, as well as the .NET runtime license notices from the publish output. The PawnIO installer is a separate prerequisite, not silently bundled or installed by W-Connect.
