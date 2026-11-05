use crate::{
    config::{interpolate, Channel},
    sensors::Sensors,
};
// Only falling speeds are smoothed. Heating and emergency cooling respond immediately.
pub fn target(c: &Channel, s: &Sensors, stale: bool, previous: u8) -> (u8, &'static str) {
    if stale {
        return (100, "Temperature data stale · full cooling");
    }
    if s.cpu.is_some_and(|v| v >= 85.) || s.gpu.is_some_and(|v| v >= 85.) {
        return (100, "Temperature ≥85°C · full cooling");
    }
    let temp = match c.source.as_str() {
        "cpu" => s.cpu,
        "gpu" => s.gpu,
        _ => s.cpu.zip(s.gpu).map(|(a, b)| a.max(b)),
    };
    let Some(temp) = temp else {
        return (100, "Required sensor unavailable · full cooling");
    };
    let requested = if c.mode == "fixed" {
        c.fixed_duty
    } else {
        interpolate(&c.curve, temp).round() as u8
    }
    .clamp(40, 100);
    let duty = if requested < previous {
        requested.max(previous.saturating_sub(3))
    } else {
        requested
    };
    (
        duty,
        if c.mode == "fixed" {
            "Fixed speed"
        } else {
            "Following temperature curve"
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    #[test]
    fn safety_overrides_fixed_and_missing_sensors() {
        let mut c = Config::default().channels.remove(0);
        c.mode = "fixed".into();
        c.fixed_duty = 40;
        let s = Sensors {
            cpu: Some(90.),
            gpu: Some(40.),
            source: String::new(),
        };
        assert_eq!(target(&c, &s, false, 40).0, 100);
        let s = Sensors {
            cpu: Some(30.),
            gpu: None,
            source: String::new(),
        };
        assert_eq!(target(&c, &s, false, 40).0, 100);
        c.source = "cpu".into();
        assert_eq!(target(&c, &s, false, 40).0, 40);
        assert_eq!(target(&c, &s, true, 40).0, 100);
    }
    #[test]
    fn falls_slowly_and_rises_immediately() {
        let c = Config::default().channels.remove(0);
        let mut s = Sensors {
            cpu: Some(30.),
            gpu: Some(30.),
            source: String::new(),
        };
        assert_eq!(target(&c, &s, false, 80).0, 77);
        s.cpu = Some(75.);
        assert_eq!(target(&c, &s, false, 40).0, 85);
    }
}
