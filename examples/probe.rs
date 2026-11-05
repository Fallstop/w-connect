//! Read the selected hub's identity and measured RPM without changing cooling
//! or lighting. The sole feature command selects the RPM readback page.
//! Run while the W-Connect service and other controller programs are stopped:
//!     cargo run --example probe
#[allow(dead_code)]
#[path = "../src/device.rs"]
mod device;

fn main() -> Result<(), String> {
    let mut controller = device::Controller::connect()?;
    let rpms = controller.rpm()?;
    println!("Controller: {}", controller.name);
    println!(
        "USB: 0cf2:a100 · interface 1 · serial {}",
        controller.serial
    );
    for (bank, rpm) in rpms.iter().enumerate() {
        println!("Bank {}: {} RPM", bank + 1, rpm);
    }
    println!("Cooling and lighting settings unchanged.");
    Ok(())
}
