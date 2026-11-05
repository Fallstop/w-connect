using System.Text.Json;
using LibreHardwareMonitor.Hardware;

// stdout is a single machine-readable result. Library diagnostics cannot corrupt it.
TextWriter output = Console.Out;
Console.SetOut(TextWriter.Null);
Snapshot snapshot;
if (!OperatingSystem.IsWindows())
{
    snapshot = new(null, null, "LibreHardwareMonitor: Windows required");
}
else
{
    // Sensor drivers can block. The process is disposable and the caller also has a
    // seven-second kill deadline. Thread-pool workers do not keep the process alive.
    Task<Snapshot> sample = Task.Run(ReadSensors);
    if (await Task.WhenAny(sample, Task.Delay(2800)) == sample)
        snapshot = await sample;
    else
        snapshot = new(null, null, "LibreHardwareMonitor: sensor query timed out");
}
output.WriteLine(JsonSerializer.Serialize(snapshot));
output.Flush();

static Snapshot ReadSensors()
{
    Computer computer = new() { IsCpuEnabled = true, IsGpuEnabled = true };
    try
    {
        computer.Open();
        List<float> cpu = [];
        List<float> gpu = [];
        foreach (IHardware hardware in computer.Hardware)
            SampleHardware(hardware, hardware.HardwareType, cpu, gpu);

        string source = "LibreHardwareMonitor 0.9.6";
        if (cpu.Count == 0)
            source += ": CPU unavailable (check PawnIO and administrator access)";
        if (gpu.Count == 0)
            source += ": GPU unavailable";
        return new(Maximum(cpu), Maximum(gpu), source);
    }
    catch (Exception error)
    {
        // Never substitute zero or retain an old value when the query failed.
        Console.Error.WriteLine($"Sensor query failed: {error.GetType().Name}: {error.Message}");
        return new(null, null, "LibreHardwareMonitor: sensor access failed");
    }
    finally
    {
        try { computer.Close(); }
        catch (Exception error) { Console.Error.WriteLine($"Sensor cleanup failed: {error.GetType().Name}"); }
    }
}

static void SampleHardware(IHardware hardware, HardwareType category, List<float> cpu, List<float> gpu)
{
    try
    {
        hardware.Update();
        foreach (ISensor sensor in hardware.Sensors)
        {
            if (sensor.SensorType != SensorType.Temperature || sensor.Value is not float value ||
                !float.IsFinite(value) || value <= 0 || value > 150)
                continue;
            // Intel exposes distance-to-TjMax as Temperature too. It is a thermal
            // margin, inversely related to temperature, so must never drive a curve.
            if (sensor.Name.Contains("Distance", StringComparison.OrdinalIgnoreCase) ||
                sensor.Name.Contains("Margin", StringComparison.OrdinalIgnoreCase))
                continue;
            if (category == HardwareType.Cpu)
                cpu.Add(value);
            else if (category is HardwareType.GpuAmd or HardwareType.GpuNvidia or HardwareType.GpuIntel)
                gpu.Add(value);
        }
    }
    catch (Exception error)
    {
        Console.Error.WriteLine($"Cannot sample {hardware.Name}: {error.GetType().Name}");
    }
    foreach (IHardware child in hardware.SubHardware)
        SampleHardware(child, category, cpu, gpu);
}

static float? Maximum(List<float> values) => values.Count == 0 ? null : values.Max();

internal sealed record Snapshot(float? cpu, float? gpu, string source);
