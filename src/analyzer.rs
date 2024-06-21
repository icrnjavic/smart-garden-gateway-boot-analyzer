use log::{debug, info, error};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::fs::OpenOptions;

#[derive(Default)]
struct CheckInfo {
    not_expected: Option<&'static str>,
    expected: Option<&'static str>,
    message: &'static str,
    instructions: &'static str,
    command: Option<&'static str>,
}

#[derive(Default)]
pub struct Diagnosis {
    pub message: &'static str,
    pub instructions: Option<&'static str>,
    pub healthy: bool,
}

static INSTRUCTIONS_LM: &str = "Linux Module (probably) faulty, return to UniElec";
static INSTRUCTIONS_BUTTON: &str = "Check button";

pub fn analyze(serial_port: &mut Box<dyn SerialPort>, lm_id: &str) -> Diagnosis {
    let early_check_info = vec![
        CheckInfo {
            not_expected: Some("SPL: failed to boot from all boot devices"),
            message: "U-Boot corrupt",
            instructions: INSTRUCTIONS_LM,
            ..Default::default()
        },
        CheckInfo {
            expected: Some("U-Boot SPL"),
            message: "No or wrong U-Boot detected",
            instructions: INSTRUCTIONS_LM,
            ..Default::default()
        },
        CheckInfo {
            expected: Some("DRAM:  128 MiB"),
            message: "Wrong RAM size detected",
            instructions: INSTRUCTIONS_LM,
            ..Default::default()
        },
        CheckInfo {
            not_expected: Some("F-Data:Magic value not correct"),
            expected: Some("F-Data:factory-data version 1 detected"),
            message: "Factory data missing",
            instructions: INSTRUCTIONS_LM,
            ..Default::default()
        },
        CheckInfo {
            expected: Some("Net:   eth0: eth@10110000"),
            message: "Ethernet could not be initialized",
            instructions: INSTRUCTIONS_LM,
            ..Default::default()
        },
        CheckInfo {
            expected: Some("=>"),
            message: "Could not enter U-Boot shell",
            instructions: INSTRUCTIONS_LM,
            ..Default::default()
        },
    ];
    let console_output = enter_u_boot(serial_port, lm_id);

    for info in early_check_info {
        if info
            .not_expected
            .is_some_and(|x| console_output.contains(x))
            || info.expected.is_some_and(|x| !console_output.contains(x))
        {
            log_issue(info.message, info.instructions);

            return Diagnosis {
                message: info.message,
                instructions: Some(info.instructions),
                healthy: false,
            };
        }
    }

    let u_boot_check_info = vec![
        CheckInfo {
            command: Some("mtd list"),
            not_expected: Some("Could not find a valid device for spi0.1"),
            expected: Some("spi-nand0"),
            message: "NAND flash not detected",
            instructions: INSTRUCTIONS_LM,
        },
        CheckInfo {
            command: Some("gpio input PA11"),
            not_expected: Some("gpio: pin PA11 (gpio 11) value is 0"),
            expected: Some("gpio: pin PA11 (gpio 11) value is 1"),
            message: "Button stuck",
            instructions: INSTRUCTIONS_BUTTON,
        },
    ];

    for info in u_boot_check_info {
        if !run_u_boot_check(serial_port, &info, lm_id) {
            log_issue(info.message, info.instructions);

            return Diagnosis {
                message: info.message,
                instructions: Some(info.instructions),
                healthy: false,
            };
        }
    }


    Diagnosis {
        message: "No issues found",
        healthy: true,
        ..Default::default()
    }
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

fn receive(serial_port: &mut Box<dyn SerialPort>, lm_id: &str) -> Option<String> {
    let mut buf = [0; 1000];
    let bytes_read = serial_port.read(&mut buf).unwrap_or(0);
    if bytes_read == 0 {
        return None;
    }
    let s = remove_non_printable(&String::from_utf8_lossy(&buf));
    if s.is_empty() {
        return None;
    }
    debug!("{s}");
    std::io::stdout().flush().expect("Failed to flush stdout");

    let file_name = format!("{}.txt", lm_id);
    if let Ok(mut file) = OpenOptions::new().append(true).open(&file_name) {
        if let Err(e) = writeln!(file, "{}", s) {
            error!("ailed to append s to file: {}", e);
        }
    } else {
        error!("failed to open the log file");
    }

    Some(s)
}

fn enter_u_boot(serial_port: &mut Box<dyn SerialPort>, lm_id: &str) -> String {
    let mut console_output = String::new();
    let mut timeout_counter = 0;

    loop {
        send(serial_port, b"x").expect("Failed to write to serial port");

        if let Some(s) = receive(serial_port, lm_id) {
            console_output += s.as_str();
            timeout_counter = 0;
        } else {
            timeout_counter += 1;
        }

        if console_output.contains("=>") || timeout_counter >= 10 {
            break;
        }
    }
    send(serial_port, b"\x03").expect("Failed to write to serial port"); // clear prompt
    console_output
}

fn run_u_boot_cmd(serial_port: &mut Box<dyn SerialPort>, cmd: &str, lm_id: &str) -> String {
    send(serial_port, format!("{cmd}\n").as_bytes()).expect("Failed to write to serial port");

    let mut console_output = String::new();
    let mut timeout_counter = 0;

    loop {
        if let Some(s) = receive(serial_port, lm_id) {
            console_output += s.as_str();
            timeout_counter = 0;
        } else {
            timeout_counter += 1;
        }

        if console_output.ends_with("=> ") || timeout_counter >= 10 {
            break;
        }
    }
    console_output
}

fn log_issue(issue: &str, instructions: &str) {
    info!("{issue}");
    info!("{instructions}");
}

fn run_u_boot_check(serial_port: &mut Box<dyn SerialPort>, info: &CheckInfo, lm_id: &str) -> bool {
    let console_output = run_u_boot_cmd(serial_port, info.command.expect("Missing U-Boot command"), lm_id);

    !(info
        .not_expected
        .is_some_and(|x| console_output.contains(x))
        || info.expected.is_some_and(|x| !console_output.contains(x)))
}
