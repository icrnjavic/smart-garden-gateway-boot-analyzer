use core::time::Duration;
use serialport::SerialPort;

/// # Errors
///
/// Will return `Err` if serial port cannot be opened.
pub fn open_serial_port(path: &str) -> Result<Box<dyn SerialPort>, serialport::Error> {
    serialport::new(path, 115_200)
        .timeout(Duration::from_millis(100))
        .open()
}

/// # Panics
///
/// Panics if something unexpected happens.
pub fn power_on_dut(serial_port: &mut Box<dyn SerialPort>, invert_rts: bool) {
    serial_port
        .write_request_to_send(!invert_rts)
        .expect("Failed power on the DUT");
}

/// # Panics
///
/// Panics if something unexpected happens.
pub fn power_off_dut(serial_port: &mut Box<dyn SerialPort>, invert_rts: bool) {
    serial_port
        .write_request_to_send(invert_rts)
        .expect("Failed power off the DUT");
}
