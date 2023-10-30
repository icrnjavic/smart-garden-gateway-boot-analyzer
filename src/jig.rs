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
