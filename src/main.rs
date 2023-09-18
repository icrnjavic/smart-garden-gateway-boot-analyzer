use core::time::Duration;
use serialport::{available_ports, SerialPort};
use std::env;
use std::io::Read;

fn open_serial_port(path: &str) -> Result<Box<dyn SerialPort>, serialport::Error> {
    serialport::new(path, 115_200)
        .timeout(Duration::from_millis(100))
        .open()
}

fn remove_non_printable(s: &str) -> String {
    s.chars()
        .filter(|&c| c.is_ascii_graphic() || c.is_ascii_whitespace())
        .collect()
}

fn send(serial_port: &mut Box<dyn SerialPort>, buf: &[u8]) -> Result<(), serialport::Error> {
    serial_port.write_all(buf)?;
    serial_port.flush()?;
    Ok(())
}

fn receive(serial_port: &mut Box<dyn SerialPort>) -> Option<String> {
    let mut buf = [0; 1000];
    let bytes_read = serial_port.read(&mut buf).unwrap_or(0);
    if bytes_read == 0 {
        return None;
    }
    let s = remove_non_printable(&String::from_utf8_lossy(&buf));
    if s.is_empty() {
        return None;
    }
    Some(s)
}

fn main() {
    let mut patterns = vec![
        "U-Boot",
        "DRAM:  128 MiB",
        "Net:   eth0: eth@10110000",
        "=>",
    ];

    let args: Vec<String> = env::args().collect();
    let serial_port_name = if args.len() > 1 {
        &args[1]
    } else {
        "/dev/ttyUSB0"
    };

    let mut serial_port: Box<dyn SerialPort>;

    if let Ok(p) = open_serial_port(serial_port_name) {
        serial_port = p;
    } else {
        eprintln!("Could not open serial port {serial_port_name}");
        eprintln!("You can pass one of the following as argument:");
        if let Ok(serial_port_list) = available_ports() {
            for port in serial_port_list {
                println!("{}", port.port_name);
            }
        } else {
            eprintln!("Failed to get serial port list");
        }
        std::process::exit(1);
    }

    let mut console_output = String::new();
    let mut timeout_counter = 0;

    loop {
        if let Some(s) = receive(&mut serial_port) {
            console_output += s.as_str();
            timeout_counter = 0;
        } else {
            timeout_counter += 1;
        }

        if console_output.contains(patterns[0]) {
            println!("{} ✔️", patterns[0]);
            patterns.drain(..1);
            if patterns.is_empty() {
                break;
            }
        }

        if timeout_counter >= 100 {
            println!("{} ❌️", patterns[0]);
            patterns.drain(..1);
            if patterns.is_empty() {
                break;
            }
            continue;
        }

        send(&mut serial_port, b"x").expect("Failed to write to serial port");
    }
}
