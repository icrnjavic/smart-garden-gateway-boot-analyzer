use std::io::prelude::*;

use smart_garden_gateway_boot_analyzer::{analyze, open_serial_port};

fn exit_with_error(msg: &str) {
    eprint!("{msg}\n\nHit \"return\" to exit...");
    std::io::stderr().flush().unwrap();
    let _ = std::io::stdin().read(&mut [0u8]).unwrap();
    std::process::exit(1);
}

fn main() {
    let serial_port_name = if let Ok(ports) = serialport::available_ports() {
        match ports.len() {
            0 => {
                exit_with_error("No serial ports found");
                std::unreachable!();
            }
            1 => ports[0].port_name.clone(),
            _ => {
                let choices = ports.into_iter().map(|p| p.port_name).collect();

                inquire::Select::new("Select serial port", choices)
                    .prompt()
                    .expect("Failed to prompt for serial port")
            }
        }
    } else {
        exit_with_error("Failed to get serial port list");
        std::unreachable!();
    };

    let mut serial_port =
        open_serial_port(serial_port_name.as_str()).expect("Failed to open serial port");

    // Disable DUT power. The signal is inverted on our (current) hardware.
    serial_port
        .write_request_to_send(true)
        .expect("Failed to set RTS");

    loop {
        if let Ok(false) = inquire::Confirm::new("Continue?")
            .with_default(true)
            .prompt()
        {
            break;
        }

        serial_port
            .write_request_to_send(false)
            .expect("Failed to clear RTS");

        analyze(&mut serial_port, std::io::stderr());

        serial_port
            .write_request_to_send(true)
            .expect("Failed to set RTS");
    }
}
