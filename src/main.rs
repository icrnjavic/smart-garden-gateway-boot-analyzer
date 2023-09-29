use smart_garden_gateway_boot_analyzer::{analyze, open_serial_port};

fn main() {
    let serial_port_name = if let Ok(ports) = serialport::available_ports() {
        match ports.len() {
            0 => {
                eprintln!("No serial ports found");
                std::process::exit(1);
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
        eprint!("Failed to get serial port list");
        std::process::exit(1);
    };

    let mut serial_port =
        open_serial_port(serial_port_name.as_str()).expect("Failed to open serial port");

    loop {
        analyze(&mut serial_port, std::io::stderr());

        if let Ok(false) = inquire::Confirm::new("Continue?")
            .with_default(true)
            .prompt()
        {
            break;
        }
    }
}
