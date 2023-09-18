use core::time::Duration;
use serialport::SerialPort;
use std::io::Read;

static INSTRUCTIONS_LM: &str = "Linux Module (probably) faulty, return to UniElec";
static INSTRUCTIONS_BUTTON: &str = "Check button";

/// # Errors
///
/// Will return `Err` if serial port cannot be opened.
pub fn open_serial_port(path: &str) -> Result<Box<dyn SerialPort>, serialport::Error> {
    serialport::new(path, 115_200)
        .timeout(Duration::from_millis(100))
        .open()
}

pub fn analyze(serial_port: &mut Box<dyn SerialPort>) {
    let early_check_info = [
        ("U-Boot SPL", "No U-Boot detected"),
        ("DRAM:  128 MiB", "Wrong RAM size detected"),
        (
            "Net:   eth0: eth@10110000",
            "Ethernet could not be initialized",
        ),
        ("=>", "Could not enter U-Boot shell"),
    ];
    let console_output = enter_u_boot(serial_port);

    for (pattern, issue) in early_check_info {
        if !console_output.contains(pattern) {
            report_issue(issue, INSTRUCTIONS_LM);
            return;
        }
    }

    let u_boot_check_info = [
        (
            "mtd list",
            "spi-nand0",
            "NAND flash not detected",
            INSTRUCTIONS_LM,
        ),
        (
            "gpio input PA11",
            "gpio: pin PA11 (gpio 11) value is 1",
            "Button stuck",
            INSTRUCTIONS_BUTTON,
        ),
    ];

    for (cmd, pattern, issue, instructions) in u_boot_check_info {
        if !run_u_boot_check(serial_port, cmd, pattern, issue, instructions) {
            return;
        }
    }

    println!("! No issues found");
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

fn enter_u_boot(serial_port: &mut Box<dyn SerialPort>) -> String {
    let mut console_output = String::new();
    let mut timeout_counter = 0;

    loop {
        send(serial_port, b"x").expect("Failed to write to serial port");

        if let Some(s) = receive(serial_port) {
            console_output += s.as_str();
            timeout_counter = 0;
        } else {
            timeout_counter += 1;
        }

        if console_output.contains("=>") || timeout_counter >= 100 {
            break;
        }
    }
    send(serial_port, b"\x03").expect("Failed to write to serial port"); // clear prompt
    console_output
}

fn run_u_boot_cmd(serial_port: &mut Box<dyn SerialPort>, cmd: &str) -> String {
    send(serial_port, format!("{cmd}\n").as_bytes()).expect("Failed to write to serial port");

    let mut console_output = String::new();
    let mut timeout_counter = 0;

    loop {
        if let Some(s) = receive(serial_port) {
            console_output += s.as_str();
            timeout_counter = 0;
        } else {
            timeout_counter += 1;
        }

        if timeout_counter >= 10 {
            break;
        }
    }
    console_output
}

fn report_issue(issue: &str, instructions: &str) {
    println!("! {issue}");
    println!("-> {instructions}");
}

fn run_u_boot_check(
    serial_port: &mut Box<dyn SerialPort>,
    cmd: &str,
    pattern: &str,
    issue: &str,
    instructions: &str,
) -> bool {
    let console_output = run_u_boot_cmd(serial_port, cmd);

    if !console_output.contains(pattern) {
        report_issue(issue, instructions);
        return false;
    }
    true
}
