//! Model-specific ENE UNI FAN control. No commands are sent by discovery/open.
//! Only 0cf2:a100 has been exercised on hardware here. See controller-support.md.
use hidapi::{DeviceInfo, HidApi, HidDevice};
use serde::Serialize;
use std::{thread, time::Duration};

pub const NAME: &str = "Lian Li UNI FAN SL";
pub const MIN_DUTY: u8 = 40;
const REPORT_ID: u8 = 0xe0;
const INPUT_LEN: usize = 65;
const COMMAND_DELAY: Duration = Duration::from_millis(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Sl,
    Al,
    SlInfinity,
    SlV2,
    AlV2,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Model {
    pub product_id: u16,
    pub name: &'static str,
    pub hardware_tested: bool,
    pub lighting_supported: bool,
    pub max_fans_per_bank: u8,
    #[serde(skip)]
    family: Family,
    #[serde(skip)]
    legacy_output: bool,
}
const MODELS: [Model; 7] = [
    Model {
        product_id: 0xa100,
        name: NAME,
        hardware_tested: true,
        lighting_supported: true,
        max_fans_per_bank: 4,
        family: Family::Sl,
        legacy_output: false,
    },
    Model {
        product_id: 0x7750,
        name: "Lian Li UNI FAN SL (legacy)",
        hardware_tested: false,
        lighting_supported: false,
        max_fans_per_bank: 4,
        family: Family::Sl,
        legacy_output: true,
    },
    Model {
        product_id: 0xa101,
        name: "Lian Li UNI FAN AL",
        hardware_tested: false,
        lighting_supported: true,
        max_fans_per_bank: 4,
        family: Family::Al,
        legacy_output: false,
    },
    Model {
        product_id: 0xa102,
        name: "Lian Li UNI FAN SL-Infinity",
        hardware_tested: false,
        lighting_supported: true,
        max_fans_per_bank: 4,
        family: Family::SlInfinity,
        legacy_output: false,
    },
    Model {
        product_id: 0xa103,
        name: "Lian Li UNI FAN SL V2",
        hardware_tested: false,
        lighting_supported: true,
        max_fans_per_bank: 6,
        family: Family::SlV2,
        legacy_output: false,
    },
    Model {
        product_id: 0xa104,
        name: "Lian Li UNI FAN AL V2",
        hardware_tested: false,
        lighting_supported: true,
        max_fans_per_bank: 6,
        family: Family::AlV2,
        legacy_output: false,
    },
    Model {
        product_id: 0xa105,
        name: "Lian Li UNI FAN SL V2A",
        hardware_tested: false,
        lighting_supported: true,
        max_fans_per_bank: 6,
        family: Family::SlV2,
        legacy_output: false,
    },
];
pub fn supported_models() -> &'static [Model] {
    &MODELS
}
fn model_for(pid: u16) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.product_id == pid)
}
impl Model {
    fn pwm_register(self) -> u8 {
        match self.family {
            Family::Sl => 0x31,
            Family::Al => 0x42,
            _ => 0x62,
        }
    }
    fn rpm_offset(self) -> usize {
        if matches!(self.family, Family::SlV2 | Family::AlV2) {
            2
        } else {
            1
        }
    }
    fn dual_ring(self) -> bool {
        matches!(self.family, Family::Al | Family::SlInfinity | Family::AlV2)
    }
    fn largest_color_report(self) -> usize {
        2 + usize::from(self.max_fans_per_bank) * if self.dual_ring() { 12 } else { 16 } * 3
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Discovered {
    pub name: String,
    pub model_name: String,
    pub serial: String,
    pub product_id: u16,
    pub hardware_tested: bool,
    pub lighting_supported: bool,
    pub max_fans_per_bank: u8,
    pub path: String,
}
fn candidate(info: &DeviceInfo) -> Option<&'static Model> {
    if info.vendor_id() != 0x0cf2 {
        return None;
    }
    let model = model_for(info.product_id())?;
    // The tested hub keeps its observed collection/interface match. Other known
    // PIDs have no vendor usage-page restriction; permit vendor collections only,
    // then validate their actual E0 report descriptor before control is possible.
    let matches = if model.hardware_tested {
        matches_interface(info.interface_number(), info.path().to_bytes())
            && info.usage_page() == 0xff72
            && info.usage() == 0xa1
    } else {
        info.usage_page() >= 0xff00
            || (info.usage_page() == 0
                && matches_interface(info.interface_number(), info.path().to_bytes()))
    };
    matches.then_some(model)
}
fn discovered(info: &DeviceInfo, model: &Model) -> Discovered {
    Discovered {
        name: info
            .product_string()
            .unwrap_or(model.name)
            .trim()
            .to_owned(),
        model_name: model.name.into(),
        serial: info.serial_number().unwrap_or("").into(),
        product_id: model.product_id,
        hardware_tested: model.hardware_tested,
        lighting_supported: model.lighting_supported,
        max_fans_per_bank: model.max_fans_per_bank,
        path: info.path().to_string_lossy().into_owned(),
    }
}
/// Enumerate known vendor collections. Does not open devices or send reports.
/// Descriptor compatibility is checked when the selected device is opened.
pub fn discover() -> Result<Vec<Discovered>, String> {
    let api = HidApi::new().map_err(|e| format!("USB discovery failed: {e}"))?;
    let mut devices: Vec<_> = api
        .device_list()
        .filter_map(|info| candidate(info).map(|m| discovered(info, m)))
        .collect();
    devices.sort_by(|a, b| {
        (a.product_id, &a.serial, &a.path).cmp(&(b.product_id, &b.serial, &b.path))
    });
    Ok(devices)
}
fn select_device(
    devices: &[Discovered],
    allow_untested: bool,
    serial: Option<&str>,
) -> Result<usize, String> {
    let selected: Vec<_> = devices
        .iter()
        .enumerate()
        .filter(|(_, d)| serial.is_none_or(|s| d.serial == s))
        .collect();
    if selected.is_empty() {
        return Err(if serial.is_some() {
            "Selected controller is not connected".into()
        } else {
            "No supported Lian Li UNI FAN controller is connected".into()
        });
    }
    let eligible: Vec<_> = selected
        .iter()
        .filter(|(_, d)| allow_untested || d.hardware_tested)
        .collect();
    if eligible.is_empty() {
        return Err("An untested controller was found. Enable untested controller support in Settings before taking control".into());
    }
    if eligible.len() != 1 {
        return Err("Multiple eligible controller interfaces were found. Select a unique controller serial in Settings, or connect one controller at a time".into());
    }
    Ok(eligible[0].0)
}
pub struct Controller {
    device: HidDevice,
    model: &'static Model,
    output_len: usize,
    pub name: String,
    pub serial: String,
    pub model_name: String,
    pub product_id: u16,
    pub hardware_tested: bool,
    pub lighting_supported: bool,
    pub max_fans_per_bank: u8,
}
impl Controller {
    #[allow(dead_code)] // Kept for the read-only diagnostic example.
    pub fn connect() -> Result<Self, String> {
        Self::connect_with_options(false, None)
    }
    pub fn connect_with_options(
        allow_untested: bool,
        serial: Option<&str>,
    ) -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| format!("USB discovery failed: {e}"))?;
        let candidates: Vec<_> = api
            .device_list()
            .filter_map(|info| candidate(info).map(|m| (info, m)))
            .collect();
        let descriptions: Vec<_> = candidates
            .iter()
            .map(|(info, m)| discovered(info, m))
            .collect();
        // Selection and the opt-in gate run before even opening an HID handle.
        let index = select_device(&descriptions, allow_untested, serial)?;
        let (info, model) = candidates[index];
        let device=info.open_device(&api).map_err(|e|format!("Cannot open {}: {e}. On Linux, install the supplied USB access rule; close other fan-control apps.",model.name))?;
        let output_len = if model.hardware_tested {
            353
        } else {
            let mut descriptor = [0u8; hidapi::MAX_REPORT_DESCRIPTOR_SIZE];
            let count = device.get_report_descriptor(&mut descriptor).map_err(|e| {
                format!("Cannot verify the untested controller's HID descriptor: {e}")
            })?;
            let caps = report_caps(&descriptor[..count.min(descriptor.len())])?;
            validate_caps(model, caps)?;
            caps.output
        };
        let lighting_supported =
            model.lighting_supported && output_len >= model.largest_color_report();
        Ok(Self {
            device,
            model,
            output_len,
            name: descriptions[index].name.clone(),
            serial: descriptions[index].serial.clone(),
            model_name: model.name.into(),
            product_id: model.product_id,
            hardware_tested: model.hardware_tested,
            lighting_supported,
            max_fans_per_bank: model.max_fans_per_bank,
        })
    }
    pub fn rpm(&mut self) -> Result<[u32; 4], String> {
        // Legacy 7750 prior art reads directly; the current vendor selector is
        // verified only for A100..A105 and is not guessed for legacy firmware.
        if !self.model.legacy_output {
            self.send_control(&[REPORT_ID, 0x50, 0])?;
        }
        let mut report = [0; INPUT_LEN];
        report[0] = REPORT_ID;
        let count = self
            .device
            .get_input_report(&mut report)
            .map_err(|e| format!("Cannot read fan speeds: {e}"))?;
        parse_rpm(self.model, &report[..count.min(report.len())])
    }
    /// Raw percent, conservatively limited to 40..100 on every implemented hub.
    pub fn set_duty(&mut self, channel: usize, duty: u8) -> Result<(), String> {
        let (mode, speed) = speed_reports(self.model, channel, duty)?;
        self.send_control(&mode)?;
        self.send_control(&speed)
    }
    pub fn lighting(
        &mut self,
        channel: usize,
        fan_count: u8,
        mode: &str,
        color: [u8; 3],
        brightness: u8,
    ) -> Result<(), String> {
        validate_channel(channel)?;
        if mode == "preserve" {
            return Ok(());
        }
        if !self.lighting_supported {
            return Err("Lighting is not implemented or descriptor-compatible for this controller; existing lighting is preserved".into());
        }
        for packet in lighting_reports(self.model, channel, fan_count, mode, color, brightness)? {
            match packet {
                Packet::Control(data) => self.send_control(&data)?,
                Packet::Color(data) => self.send_output(&data)?,
            }
        }
        Ok(())
    }
    fn send_control(&self, data: &[u8]) -> Result<(), String> {
        if self.model.legacy_output {
            // Legacy Linux implementations use four-byte output requests.
            // HIDAPI handles the Windows descriptor-required padding itself.
            return self.write_output(data);
        }
        // Preserve the exact eleven-byte transfer exercised on the user's A100.
        let bytes = if self.model.hardware_tested {
            padded(data, 11)?
        } else {
            data.to_vec()
        };
        self.device.send_feature_report(&bytes).map_err(|e| {
            format!(
                "Controller command {:02x}/{:02x} failed: {e}",
                data[1], data[2]
            )
        })?;
        thread::sleep(COMMAND_DELAY);
        Ok(())
    }
    fn send_output(&self, data: &[u8]) -> Result<(), String> {
        let bytes = padded(data, self.output_len)?;
        self.write_output(&bytes)
    }
    fn write_output(&self, bytes: &[u8]) -> Result<(), String> {
        let count = self
            .device
            .write(bytes)
            .map_err(|e| format!("Controller output failed: {e}"))?;
        if count != bytes.len() {
            return Err(format!(
                "Incomplete controller output: {count}/{} bytes",
                bytes.len()
            ));
        }
        thread::sleep(COMMAND_DELAY);
        Ok(())
    }
}
fn padded(data: &[u8], length: usize) -> Result<Vec<u8>, String> {
    if length < data.len() || length > 4096 {
        return Err("Command does not fit the verified HID report".into());
    }
    let mut bytes = vec![0; length];
    bytes[..data.len()].copy_from_slice(data);
    Ok(bytes)
}
fn matches_interface(number: i32, path: &[u8]) -> bool {
    number == 1
        || (number == -1
            && path
                .windows(7)
                .any(|w| w.eq_ignore_ascii_case(b"&mi_01#") || w.eq_ignore_ascii_case(b"&mi_01&")))
}
fn validate_channel(channel: usize) -> Result<(), String> {
    if channel < 4 {
        Ok(())
    } else {
        Err("Fan bank must be between 1 and 4".into())
    }
}
fn speed_reports(model: &Model, channel: usize, duty: u8) -> Result<(Vec<u8>, Vec<u8>), String> {
    validate_channel(channel)?;
    if !(MIN_DUTY..=100).contains(&duty) {
        return Err(format!("Fan duty must be {MIN_DUTY}–100%"));
    }
    let mut mode = vec![REPORT_ID, 0x10, model.pwm_register(), 0x10 << channel];
    // L-Connect uses a six-byte feature command for modern sync selection.
    if !model.legacy_output {
        mode.resize(6, 0);
    }
    Ok((mode, vec![REPORT_ID, 0x20 | channel as u8, 0, duty]))
}
fn parse_rpm(model: &Model, report: &[u8]) -> Result<[u32; 4], String> {
    let offset = model.rpm_offset();
    if report.len() < offset + 8 || report[0] != REPORT_ID {
        return Err(format!("Invalid fan-speed report ({} bytes)", report.len()));
    }
    let mut rpms = [0; 4];
    for (i, rpm) in rpms.iter_mut().enumerate() {
        *rpm = u16::from_be_bytes([report[offset + i * 2], report[offset + i * 2 + 1]]) as u32;
        if *rpm > 10000 {
            return Err("Controller returned implausible fan speeds".into());
        }
    }
    Ok(rpms)
}
#[derive(Debug, PartialEq)]
enum Packet {
    Control(Vec<u8>),
    Color(Vec<u8>),
}
fn lighting_reports(
    model: &Model,
    channel: usize,
    fan_count: u8,
    mode: &str,
    color: [u8; 3],
    brightness: u8,
) -> Result<Vec<Packet>, String> {
    validate_channel(channel)?;
    if !model.lighting_supported {
        return Err("Lighting is not implemented for this model".into());
    }
    if !(1..=model.max_fans_per_bank).contains(&fan_count) || brightness > 100 {
        return Err(format!(
            "Lighting requires 1–{} fans and 0–100% brightness",
            model.max_fans_per_bank
        ));
    }
    let effect = match mode {
        "static" | "off" => 1,
        "breathing" => 2,
        "rainbow" => 5,
        _ => return Err(format!("Unsupported lighting mode: {mode}")),
    };
    let bright = if brightness == 0 || mode == "off" {
        8
    } else {
        4 - (u16::from(brightness) + 12).clamp(25, 100) as u8 / 25
    };
    let sync = match model.family {
        Family::Sl => 0x30,
        Family::Al => 0x41,
        _ => 0x61,
    };
    let mut packets = vec![Packet::Control(vec![REPORT_ID, 0x10, sync, 0, 0, 0])];
    match model.family {
        Family::Sl => packets.push(Packet::Control(vec![REPORT_ID, 0x10, 0x34, 0, 0, 0])),
        Family::Al => packets.push(Packet::Control(vec![REPORT_ID, 0x10, 0x43, 0, 0, 0])),
        _ => {} // On newer firmware the selected independent effect defines scope.
    }
    let quantity = match model.family {
        Family::Sl => vec![
            REPORT_ID,
            0x10,
            0x32,
            ((channel as u8) << 4) | fan_count,
            0,
            0,
        ],
        Family::SlV2 => vec![
            REPORT_ID,
            0x10,
            0x60,
            ((channel as u8) << 4) | fan_count,
            0,
            0,
        ],
        Family::Al => vec![REPORT_ID, 0x10, 0x40, channel as u8 + 1, fan_count, 0],
        _ => vec![REPORT_ID, 0x10, 0x60, channel as u8 + 1, fan_count, 0],
    };
    packets.push(Packet::Control(quantity));
    let rings: Vec<(u8, usize)> = if model.dual_ring() {
        vec![(channel as u8 * 2, 8), (channel as u8 * 2 + 1, 12)]
    } else {
        vec![(channel as u8, 16)]
    };
    for (port, leds) in rings {
        let static_mode = mode != "rainbow";
        let stride = if static_mode {
            leds
        } else if model.family == Family::AlV2 {
            6
        } else {
            4
        };
        let max = usize::from(model.max_fans_per_bank);
        let mut colors = vec![0; 2 + max * stride * 3];
        colors[0] = REPORT_ID;
        colors[1] = 0x30 | port;
        for fan in 0..usize::from(fan_count) {
            for led in 0..stride {
                let rgb = if mode == "off" && !model.hardware_tested {
                    [0, 0, 0]
                } else if mode == "rainbow" && model.family == Family::AlV2 {
                    [
                        [255, 0, 0],
                        [255, 255, 0],
                        [0, 255, 0],
                        [0, 255, 255],
                        [0, 0, 255],
                        [255, 0, 255],
                    ][led % 6]
                } else if mode == "rainbow" && !model.hardware_tested {
                    // Independent Rainbow consumes a palette on these models;
                    // repeating the saved static color can yield a single hue.
                    [[255, 0, 0], [255, 255, 0], [0, 255, 0], [0, 0, 255]][led % 4]
                } else {
                    color
                };
                let off = 2 + (fan * stride + led) * 3;
                colors[off..off + 3].copy_from_slice(&[rgb[0], rgb[2], rgb[1]]);
            }
        }
        // Tested SL uses its existing full per-LED buffer even for rainbow.
        if model.hardware_tested && mode == "rainbow" {
            colors = vec![0; 2 + usize::from(model.max_fans_per_bank) * 16 * 3];
            colors[0] = REPORT_ID;
            colors[1] = 0x30 | port;
            for led in 0..usize::from(fan_count) * 16 {
                let off = 2 + led * 3;
                colors[off..off + 3].copy_from_slice(&[color[0], color[2], color[1]]);
            }
        }
        packets.push(Packet::Color(colors));
        packets.push(Packet::Control(vec![
            REPORT_ID,
            0x10 | port,
            effect,
            0,
            0,
            bright,
        ]));
    }
    packets.push(Packet::Control(vec![
        REPORT_ID,
        0x60,
        0,
        if model.family == Family::SlV2 { 4 } else { 1 },
    ]));
    Ok(packets)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ReportCaps {
    input: usize,
    output: usize,
    feature: usize,
}
#[derive(Clone, Copy, Default)]
struct Globals {
    id: u32,
    size: u32,
    count: u32,
}
/// Parse only standard HID size/count/ID globals and report main items. Counts
/// aggregate across fields; PUSH/POP are honored and malformed descriptors fail.
fn report_caps(descriptor: &[u8]) -> Result<ReportCaps, String> {
    let mut pos = 0;
    let mut globals = Globals::default();
    let mut stack = Vec::new();
    let mut bits = [0usize; 3];
    while pos < descriptor.len() {
        let prefix = descriptor[pos];
        pos += 1;
        if prefix == 0xfe {
            if pos + 2 > descriptor.len() {
                return Err("Truncated HID long item".into());
            }
            let len = usize::from(descriptor[pos]);
            pos += 2;
            if pos + len > descriptor.len() {
                return Err("Truncated HID long item payload".into());
            }
            pos += len;
            continue;
        }
        let size = match prefix & 3 {
            3 => 4,
            n => n as usize,
        };
        if pos + size > descriptor.len() {
            return Err("Truncated HID descriptor item".into());
        }
        let mut value = 0u32;
        for (i, byte) in descriptor[pos..pos + size].iter().enumerate() {
            value |= u32::from(*byte) << (i * 8);
        }
        pos += size;
        let kind = (prefix >> 2) & 3;
        let tag = prefix >> 4;
        if kind == 1 {
            match tag {
                7 => globals.size = value,
                8 => globals.id = value,
                9 => globals.count = value,
                10 => {
                    if stack.len() >= 16 {
                        return Err("HID descriptor stack limit exceeded".into());
                    }
                    stack.push(globals)
                }
                11 => globals = stack.pop().ok_or("Unbalanced HID descriptor POP")?,
                _ => {}
            }
        } else if kind == 0 && globals.id == u32::from(REPORT_ID) {
            let index = match tag {
                8 => Some(0),
                9 => Some(1),
                11 => Some(2),
                _ => None,
            };
            if let Some(index) = index {
                let field = (globals.size as usize)
                    .checked_mul(globals.count as usize)
                    .ok_or("Oversized HID report")?;
                bits[index] = bits[index]
                    .checked_add(field)
                    .ok_or("Oversized HID report")?;
                if bits[index] > 4095 * 8 {
                    return Err("Oversized HID report".into());
                }
            }
        }
    }
    if !stack.is_empty() {
        return Err("Unbalanced HID descriptor PUSH".into());
    }
    let length = |n: usize| if n == 0 { 0 } else { n.div_ceil(8) + 1 };
    Ok(ReportCaps {
        input: length(bits[0]),
        output: length(bits[1]),
        feature: length(bits[2]),
    })
}
fn validate_caps(model: &Model, caps: ReportCaps) -> Result<(), String> {
    let control = if model.legacy_output {
        caps.output
    } else {
        caps.feature
    };
    let minimum_control = if model.legacy_output { 4 } else { 6 };
    if caps.input != INPUT_LEN || control < minimum_control {
        return Err(format!("Unexpected E0 HID descriptor for {} (input {}, output {}, feature {}); no commands sent",model.name,caps.input,caps.output,caps.feature));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model(pid: u16) -> &'static Model {
        model_for(pid).unwrap()
    }
    fn fake(pid: u16, serial: &str) -> Discovered {
        let m = model(pid);
        Discovered {
            name: m.name.into(),
            model_name: m.name.into(),
            serial: serial.into(),
            product_id: pid,
            hardware_tested: m.hardware_tested,
            lighting_supported: m.lighting_supported,
            max_fans_per_bank: m.max_fans_per_bank,
            path: format!("fake-{serial}"),
        }
    }
    #[test]
    fn registry_excludes_unverified_ids_and_gates_untested_before_open() {
        assert!(model_for(0xa106).is_none());
        assert!(model_for(0xa107).is_none());
        assert_eq!(MODELS.iter().filter(|m| m.hardware_tested).count(), 1);
        assert!(select_device(&[fake(0xa101, "al")], false, None).is_err());
        assert_eq!(select_device(&[fake(0xa101, "al")], true, None).unwrap(), 0);
        assert_eq!(
            select_device(&[fake(0xa101, "al"), fake(0xa100, "sl")], false, None).unwrap(),
            1
        );
    }
    #[test]
    fn multiple_hubs_require_unique_selection() {
        let devices = [fake(0xa101, "al"), fake(0xa100, "sl")];
        assert!(select_device(&devices, true, None).is_err());
        assert_eq!(select_device(&devices, true, Some("al")).unwrap(), 0);
        assert!(select_device(&devices, false, Some("al")).is_err());
        assert!(select_device(&devices, true, Some("absent")).is_err());
        assert!(select_device(
            &[fake(0xa100, "dup"), fake(0xa100, "dup")],
            false,
            Some("dup")
        )
        .is_err());
    }
    #[test]
    fn model_specific_mode_registers_and_raw_speed_all_banks() {
        for (pid, register) in [
            (0x7750, 0x31),
            (0xa100, 0x31),
            (0xa101, 0x42),
            (0xa102, 0x62),
            (0xa103, 0x62),
            (0xa104, 0x62),
            (0xa105, 0x62),
        ] {
            for channel in 0..4 {
                let (mode, speed) = speed_reports(model(pid), channel, 60).unwrap();
                assert_eq!(&mode[..4], &[0xe0, 0x10, register, 0x10 << channel]);
                assert_eq!(speed, vec![0xe0, 0x20 | channel as u8, 0, 60]);
            }
            assert!(speed_reports(model(pid), 4, 60).is_err());
            assert!(speed_reports(model(pid), 0, 39).is_err());
            assert!(speed_reports(model(pid), 0, 101).is_err());
        }
    }
    #[test]
    fn rpm_offsets_distinguish_v2_and_reject_bad_packets() {
        for pid in [0x7750, 0xa100, 0xa101, 0xa102] {
            assert_eq!(
                parse_rpm(model(pid), &[0xe0, 3, 32, 7, 108, 0, 0, 5, 70]).unwrap(),
                [800, 1900, 0, 1350]
            );
        }
        for pid in [0xa103, 0xa104, 0xa105] {
            assert_eq!(
                parse_rpm(model(pid), &[0xe0, 0x99, 3, 32, 7, 108, 0, 0, 5, 70]).unwrap(),
                [800, 1900, 0, 1350]
            );
            assert!(parse_rpm(model(pid), &[0xe0; 9]).is_err());
        }
        assert!(parse_rpm(model(0xa100), &[]).is_err());
        assert!(parse_rpm(model(0xa100), &[0; 9]).is_err());
        assert!(parse_rpm(model(0xa100), &[0xe0, 255, 255, 0, 0, 0, 0, 0, 0]).is_err());
    }
    #[test]
    fn observed_sl_transfers_remain_byte_identical() {
        let packets = lighting_reports(model(0xa100), 2, 3, "static", [17, 34, 51], 100).unwrap();
        assert_eq!(packets.len(), 6);
        assert_eq!(packets[0], Packet::Control(vec![0xe0, 0x10, 0x30, 0, 0, 0]));
        assert_eq!(packets[1], Packet::Control(vec![0xe0, 0x10, 0x34, 0, 0, 0]));
        assert_eq!(
            packets[2],
            Packet::Control(vec![0xe0, 0x10, 0x32, 0x23, 0, 0])
        );
        let Packet::Color(colors) = &packets[3] else {
            panic!()
        };
        let padded = padded(colors, 353).unwrap();
        assert_eq!(&padded[..5], &[0xe0, 0x32, 17, 51, 34]);
        assert_eq!(&padded[143..146], &[17, 51, 34]);
        assert!(padded[146..].iter().all(|b| *b == 0));
        assert_eq!(packets[4], Packet::Control(vec![0xe0, 0x12, 1, 0, 0, 0]));
        assert_eq!(packets[5], Packet::Control(vec![0xe0, 0x60, 0, 1]));
    }
    #[test]
    fn lighting_layouts_quantity_ports_frames_and_capacity() {
        for (pid, qty, sync, ports, frame) in [
            (0xa101, vec![0xe0, 0x10, 0x40, 3, 4, 0], 0x41, vec![4, 5], 1),
            (0xa102, vec![0xe0, 0x10, 0x60, 3, 4, 0], 0x61, vec![4, 5], 1),
            (0xa103, vec![0xe0, 0x10, 0x60, 0x24, 0, 0], 0x61, vec![2], 4),
            (0xa104, vec![0xe0, 0x10, 0x60, 3, 4, 0], 0x61, vec![4, 5], 1),
            (0xa105, vec![0xe0, 0x10, 0x60, 0x24, 0, 0], 0x61, vec![2], 4),
        ] {
            let p = lighting_reports(model(pid), 2, 4, "static", [17, 34, 51], 100).unwrap();
            assert_eq!(p[0], Packet::Control(vec![0xe0, 0x10, sync, 0, 0, 0]));
            assert!(p.contains(&Packet::Control(qty)));
            let colors: Vec<_> = p
                .iter()
                .filter_map(|v| {
                    if let Packet::Color(c) = v {
                        Some(c)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(colors.len(), ports.len());
            for (c, port) in colors.iter().zip(ports) {
                assert_eq!(&c[..5], &[0xe0, 0x30 | port, 17, 51, 34]);
                assert!(c.len() <= model(pid).largest_color_report());
            }
            assert_eq!(p.last(), Some(&Packet::Control(vec![0xe0, 0x60, 0, frame])));
        }
    }
    #[test]
    fn breathing_uploads_native_effect_and_one_color_buffer() {
        let packets = lighting_reports(model(0xa100), 0, 3, "breathing", [17, 34, 51], 70).unwrap();
        assert!(packets.contains(&Packet::Control(vec![0xe0, 0x10, 2, 0, 0, 1])));
        let colors: Vec<_> = packets
            .iter()
            .filter_map(|packet| {
                if let Packet::Color(colors) = packet {
                    Some(colors)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(colors.len(), 1);
        assert_eq!(&colors[0][..2], &[0xe0, 0x30]);
        for led in colors[0][2..146].as_chunks::<3>().0 {
            assert_eq!(led, &[17, 51, 34]);
        }
        assert!(colors[0][146..].iter().all(|byte| *byte == 0));
        assert_eq!(
            packets.last(),
            Some(&Packet::Control(vec![0xe0, 0x60, 0, 1]))
        );
        assert_eq!(packets.len(), 6);
    }
    #[test]
    fn brightness_off_and_model_fan_limits() {
        for (percent, encoded) in [(0, 8), (25, 3), (50, 2), (75, 1), (100, 0)] {
            let p = lighting_reports(model(0xa100), 0, 1, "rainbow", [0; 3], percent).unwrap();
            assert!(p.contains(&Packet::Control(vec![0xe0, 0x10, 5, 0, 0, encoded])));
        }
        assert!(lighting_reports(model(0x7750), 0, 1, "static", [0; 3], 100).is_err());
        assert!(lighting_reports(model(0xa100), 0, 5, "static", [0; 3], 100).is_err());
        assert!(lighting_reports(model(0xa103), 0, 6, "static", [0; 3], 100).is_ok());
        assert!(lighting_reports(model(0xa104), 0, 7, "static", [0; 3], 100).is_err());
        let p = lighting_reports(model(0xa101), 0, 1, "off", [255; 3], 100).unwrap();
        for packet in p {
            if let Packet::Color(data) = packet {
                assert!(data[2..].iter().all(|b| *b == 0));
            }
        }
        let old_off = lighting_reports(model(0xa100), 0, 1, "off", [17, 34, 51], 100).unwrap();
        assert!(old_off
            .iter()
            .any(|p| matches!(p, Packet::Color(c) if c[2..5] == [17, 51, 34])));
    }
    #[test]
    fn extended_rainbow_uses_full_spectrum_palettes_and_all_bank_addresses() {
        for pid in [0xa101, 0xa102, 0xa103, 0xa104, 0xa105] {
            let dual = [0xa101, 0xa102, 0xa104].contains(&pid);
            let slots = if pid == 0xa104 { 6 } else { 4 };
            let max_fans = if pid >= 0xa103 { 6 } else { 4 };
            let spectrum: &[u8] = if slots == 6 {
                &[
                    255, 0, 0, 255, 0, 255, 0, 0, 255, 0, 255, 255, 0, 255, 0, 255, 255, 0,
                ]
            } else {
                &[255, 0, 0, 255, 0, 255, 0, 0, 255, 0, 255, 0]
            };
            for (bank, ports) in if dual {
                vec![
                    (0, vec![0, 1]),
                    (1, vec![2, 3]),
                    (2, vec![4, 5]),
                    (3, vec![6, 7]),
                ]
            } else {
                vec![(0, vec![0]), (1, vec![1]), (2, vec![2]), (3, vec![3])]
            } {
                let p = lighting_reports(model(pid), bank, 4, "rainbow", [23, 45, 67], 75).unwrap();
                for port in ports {
                    let color = p
                        .iter()
                        .find_map(|p| match p {
                            Packet::Color(c) if c[1] == 0x30 | port => Some(c),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(color.len(), 2 + max_fans * slots * 3);
                    for fan in 0..4 {
                        assert_eq!(
                            &color[2 + fan * slots * 3..2 + (fan + 1) * slots * 3],
                            spectrum
                        );
                    }
                    assert!(color[2 + 4 * slots * 3..].iter().all(|b| *b == 0));
                    assert!(p.contains(&Packet::Control(vec![0xe0, 0x10 | port, 5, 0, 0, 1])));
                }
                let qty = if dual {
                    vec![
                        0xe0,
                        0x10,
                        if pid == 0xa101 { 0x40 } else { 0x60 },
                        [1, 2, 3, 4][bank],
                        4,
                        0,
                    ]
                } else {
                    vec![0xe0, 0x10, 0x60, [0x04, 0x14, 0x24, 0x34][bank], 0, 0]
                };
                assert!(p.contains(&Packet::Control(qty)));
            }
        }
    }
    #[test]
    fn descriptor_sizes_must_match_e0_and_model_rpm_layout() {
        let d = [
            0x85, 0xe0, 0x75, 8, 0x95, 64, 0x81, 2, 0x96, 0x60, 1, 0x91, 2, 0x95, 10, 0xb1, 2,
        ];
        let caps = report_caps(&d).unwrap();
        assert_eq!(
            caps,
            ReportCaps {
                input: 65,
                output: 353,
                feature: 11
            }
        );
        for m in &MODELS {
            assert!(validate_caps(m, caps).is_ok());
        }
        assert!(report_caps(&[0x96, 0x60]).is_err());
        assert!(report_caps(&[0xb4]).is_err());
        assert!(validate_caps(
            model(0xa103),
            ReportCaps {
                input: 9,
                output: 353,
                feature: 11
            }
        )
        .is_err());
        assert!(validate_caps(
            model(0xa101),
            ReportCaps {
                input: 64,
                output: 353,
                feature: 11
            }
        )
        .is_err());
        let mut wrong = d;
        wrong[1] = 0xe1;
        assert!(validate_caps(model(0xa101), report_caps(&wrong).unwrap()).is_err());
        assert!(padded(&[0; 7], 6).is_err());
    }
    #[test]
    fn interface_matching_requires_explicit_interface_one() {
        assert!(matches_interface(1, b"/dev/hidraw8"));
        assert!(matches_interface(
            -1,
            br"\\?\hid#vid_0cf2&pid_a100&MI_01#serial"
        ));
        assert!(matches_interface(
            -1,
            br"\\?\hid#vid_0cf2&pid_a100&mi_01&col01#serial"
        ));
        assert!(!matches_interface(-1, b"/dev/hidraw8"));
        assert!(!matches_interface(
            0,
            br"\\?\hid#vid_0cf2&pid_a100&mi_00#serial"
        ));
        assert!(!matches_interface(
            -1,
            br"\\?\hid#vid_0cf2&pid_a100&mi_010#serial"
        ));
    }
}
