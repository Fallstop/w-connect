use serde::{Deserialize, Serialize};
use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sensors {
    pub cpu: Option<f64>,
    pub gpu: Option<f64>,
    pub source: String,
}
impl Default for Sensors {
    fn default() -> Self {
        Self {
            cpu: None,
            gpu: None,
            source: "Waiting for temperature sensors".into(),
        }
    }
}
fn valid(v: f64) -> Option<f64> {
    if v.is_finite() && (0.0..=125.0).contains(&v) {
        Some(v)
    } else {
        None
    }
}
fn output_timeout(cmd: &mut Command, timeout: Duration) -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                return String::from_utf8(child.wait_with_output().ok()?.stdout).ok();
            }
            Ok(None) => {}
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(20));
    }
}
fn nvidia() -> Option<f64> {
    output_timeout(
        Command::new("nvidia-smi").args([
            "--query-gpu=temperature.gpu",
            "--format=csv,noheader,nounits",
        ]),
        Duration::from_millis(1500),
    )?
    .lines()
    .filter_map(|s| s.trim().parse::<f64>().ok().and_then(valid))
    .max_by(f64::total_cmp)
}
#[cfg(target_os = "linux")]
pub fn read() -> Sensors {
    let mut cpus = vec![];
    let mut gpus = vec![];
    if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let dir = entry.path();
            let name = std::fs::read_to_string(dir.join("name")).unwrap_or_default();
            let cpu = ["k10temp", "coretemp", "zenpower", "k8temp"].contains(&name.trim());
            let gpu = ["amdgpu", "nouveau", "i915", "xe"].contains(&name.trim());
            if !cpu && !gpu {
                continue;
            }
            if let Ok(files) = std::fs::read_dir(dir) {
                for f in files.flatten() {
                    let n = f.file_name();
                    let n = n.to_string_lossy();
                    if n.starts_with("temp") && n.ends_with("_input") {
                        if let Some(v) = std::fs::read_to_string(f.path())
                            .ok()
                            .and_then(|s| s.trim().parse::<f64>().ok())
                            .and_then(|v| valid(v / 1000.))
                        {
                            if cpu {
                                cpus.push(v)
                            } else {
                                gpus.push(v)
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(v) = nvidia() {
        gpus.push(v)
    }
    Sensors {
        cpu: cpus.into_iter().max_by(f64::total_cmp),
        gpu: gpus.into_iter().max_by(f64::total_cmp),
        source: "Linux hwmon · NVIDIA SMI (when available)".into(),
    }
}
#[cfg(target_os = "windows")]
pub fn read() -> Sensors {
    let helper = std::env::current_exe()
        .unwrap_or_default()
        .with_file_name("w-connect-sensors.exe");
    let mut s = output_timeout(&mut Command::new(helper), Duration::from_secs(7))
        .and_then(|s| serde_json::from_str::<Sensors>(&s).ok())
        .unwrap_or_else(||Sensors{cpu:None,gpu:None,source:"Windows temperature helper unavailable or timed out; install the helper, PawnIO, and run elevated".into()});
    s.cpu = s.cpu.and_then(valid);
    s.gpu = s.gpu.and_then(valid);
    if s.gpu.is_none() {
        s.gpu = nvidia();
        if s.gpu.is_some() {
            s.source.push_str(" · NVIDIA SMI fallback");
        }
    }

    s
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn read() -> Sensors {
    Sensors::default()
}
