use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub temp: f64,
    pub duty: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    pub mode: String,
    pub color: String,
    pub brightness: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub name: String,
    pub fan_count: u8,
    pub mode: String,
    pub source: String,
    pub fixed_duty: u8,
    pub curve: Vec<Point>,
    pub lighting: Lighting,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ControllerConfig {
    pub allow_untested: bool,
    pub serial: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub controller: ControllerConfig,
    pub channels: Vec<Channel>,
    pub poll_ms: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            controller: ControllerConfig::default(),
            poll_ms: 2000,
            channels: (0..4)
                .map(|i| Channel {
                    name: format!("Fan bank {}", i + 1),
                    fan_count: 3,
                    mode: "curve".into(),
                    source: "max".into(),
                    fixed_duty: 60,
                    curve: vec![
                        Point {
                            temp: 30.,
                            duty: 40.,
                        },
                        Point {
                            temp: 50.,
                            duty: 45.,
                        },
                        Point {
                            temp: 65.,
                            duty: 65.,
                        },
                        Point {
                            temp: 75.,
                            duty: 85.,
                        },
                        Point {
                            temp: 85.,
                            duty: 100.,
                        },
                    ],
                    lighting: Lighting {
                        mode: "preserve".into(),
                        color: "#67c8df".into(),
                        brightness: 70,
                    },
                })
                .collect(),
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .controller
            .serial
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 128 || s.chars().any(char::is_control))
        {
            return Err("Controller serial must contain 1–128 printable bytes".into());
        }
        if self.channels.len() != 4 {
            return Err("Exactly four fan banks are required".into());
        }
        if !(1000..=5000).contains(&self.poll_ms) {
            return Err("Polling must be between 1000 and 5000 ms".into());
        }
        for c in &self.channels {
            if c.name.trim().is_empty()
                || c.name.chars().count() > 40
                || c.name.chars().any(char::is_control)
            {
                return Err("Bank names must contain 1–40 printable characters".into());
            }
            if !(1..=6).contains(&c.fan_count) {
                return Err("Each bank supports 1–6 fans, subject to controller limits".into());
            }
            if !["curve", "fixed"].contains(&c.mode.as_str())
                || !["cpu", "gpu", "max"].contains(&c.source.as_str())
            {
                return Err("Unknown control mode or sensor".into());
            }
            if !(40..=100).contains(&c.fixed_duty) {
                return Err("Fan duty must be 40–100%".into());
            }
            if c.curve.len() < 2 || c.curve.len() > 10 {
                return Err("A curve needs 2–10 points".into());
            }
            for (i, p) in c.curve.iter().enumerate() {
                if !p.temp.is_finite()
                    || !p.duty.is_finite()
                    || !(20.0..=85.0).contains(&p.temp)
                    || !(40.0..=100.0).contains(&p.duty)
                {
                    return Err("Curve temperatures must be 20–85°C and duties 40–100%".into());
                }
                if i > 0 && (p.temp <= c.curve[i - 1].temp || p.duty < c.curve[i - 1].duty) {
                    return Err(
                        "Curve temperatures must increase, and fan speeds cannot decrease".into(),
                    );
                }
            }
            if c.curve.last().unwrap().duty != 100. {
                return Err("Curves must end at 100% by 85°C".into());
            }
            if !["preserve", "static", "rainbow", "off"].contains(&c.lighting.mode.as_str())
                || c.lighting.brightness > 100
            {
                return Err("Invalid lighting mode or brightness".into());
            }
            let h = c.lighting.color.as_bytes();
            if h.len() != 7 || h[0] != b'#' || !h[1..].iter().all(u8::is_ascii_hexdigit) {
                return Err("Colors must use #RRGGBB".into());
            }
        }
        Ok(())
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path.parent().ok_or("Invalid configuration path")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        use std::io::Write;
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(&serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        #[cfg(not(windows))]
        {
            fs::rename(tmp, path).map_err(|e| e.to_string())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            #[link(name = "kernel32")]
            extern "system" {
                fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
            }
            let from: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) } == 0 {
                Err(std::io::Error::last_os_error().to_string())
            } else {
                Ok(())
            }
        }
    }
}
pub fn interpolate(points: &[Point], temp: f64) -> f64 {
    if temp <= points[0].temp {
        return points[0].duty;
    }
    for p in points.windows(2) {
        if temp <= p[1].temp {
            return p[0].duty
                + (p[1].duty - p[0].duty) * (temp - p[0].temp) / (p[1].temp - p[0].temp);
        }
    }
    points.last().unwrap().duty
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_configs_keep_untested_drivers_disabled() {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        value.as_object_mut().unwrap().remove("controller");
        let c: Config = serde_json::from_value(value).unwrap();
        assert_eq!(c.controller, ControllerConfig::default());
        let mut c = Config::default();
        c.channels[0].fan_count = 6;
        assert!(c.validate().is_ok());
        c.controller.serial = Some("bad\nserial".into());
        assert!(c.validate().is_err());
    }
    #[test]
    fn curve_interpolates_and_clamps() {
        let c = Config::default();
        let p = &c.channels[0].curve;
        assert_eq!(interpolate(p, 0.), 40.);
        assert_eq!(interpolate(p, 40.), 42.5);
        assert_eq!(interpolate(p, 100.), 100.);
    }
    #[test]
    fn rejects_unsafe_and_malformed_config() {
        let mut c = Config::default();
        assert!(c.validate().is_ok());
        c.channels[0].curve[1].temp = 30.;
        assert!(c.validate().is_err());
        c = Config::default();
        c.channels[0].curve.last_mut().unwrap().duty = 90.;
        assert!(c.validate().is_err());
        c = Config::default();
        c.channels[0].fixed_duty = 0;
        assert!(c.validate().is_err());
        c = Config::default();
        c.channels[0].lighting.color = "#💨".into();
        assert!(c.validate().is_err());
    }
}
