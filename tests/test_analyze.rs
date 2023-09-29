use core::time::Duration;
use mockall::mock;
use rstest::rstest;
use serde::Deserialize;
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, StopBits};
use smart_garden_gateway_boot_analyzer::analyze;

mock! {
    pub SerialPort {}
    impl std::io::Read for SerialPort {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            Ok([]);
        }
    }
    impl serialport::SerialPort for SerialPort {
        fn name(&self) -> Option<String>;
        fn baud_rate(&self) -> serialport::Result<u32>;
        fn data_bits(&self) -> serialport::Result<DataBits>;
        fn flow_control(&self) -> serialport::Result<FlowControl>;
        fn parity(&self) -> serialport::Result<Parity>;
        fn stop_bits(&self) -> serialport::Result<StopBits>;
        fn timeout(&self) -> Duration;
        fn set_baud_rate(&mut self, baud_rate: u32) -> serialport::Result<()>;
        fn set_data_bits(&mut self, data_bits: DataBits) -> serialport::Result<()>;
        fn set_flow_control(&mut self, flow_control: FlowControl) -> serialport::Result<()>;
        fn set_parity(&mut self, parity: Parity) -> serialport::Result<()>;
        fn set_stop_bits(&mut self, stop_bits: StopBits) -> serialport::Result<()>;
        fn set_timeout(&mut self, timeout: Duration) -> serialport::Result<()>;
        fn write_request_to_send(&mut self, level: bool) -> serialport::Result<()>;
        fn write_data_terminal_ready(&mut self, level: bool) -> serialport::Result<()>;
        fn read_clear_to_send(&mut self) -> serialport::Result<bool>;
        fn read_data_set_ready(&mut self) -> serialport::Result<bool>;
        fn read_ring_indicator(&mut self) -> serialport::Result<bool>;
        fn read_carrier_detect(&mut self) -> serialport::Result<bool>;
        fn bytes_to_read(&self) -> serialport::Result<u32>;
        fn bytes_to_write(&self) -> serialport::Result<u32>;
        fn clear(&self, buffer_to_clear: ClearBuffer) -> serialport::Result<()>;
        fn try_clone(&self) -> serialport::Result<Box<dyn serialport::SerialPort>>;
        fn set_break(&self) -> serialport::Result<()>;
        fn clear_break(&self) -> serialport::Result<()>;
    }
    impl std::io::Write for SerialPort {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
        fn flush(&mut self) -> std::io::Result<()>;
    }
}

fn zero() -> usize {
    0
}

#[derive(Deserialize, Clone)]
struct TestData {
    console_output: Vec<String>,
    #[serde(default = "zero")]
    index: usize,
    message: String,
}

impl TestData {
    #[allow(clippy::unnecessary_wraps)]
    pub fn read_console_output(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.index >= self.console_output.len() {
            return Ok(0);
        }
        let s = &self.console_output[self.index];
        buf[..s.len()].copy_from_slice(s.as_bytes());
        self.index += 1;
        Ok(s.len())
    }
}

#[rstest]
fn test_analyze(
    #[values(
        "button_stuck",
        "no_fdata",
        "no_issues",
        "no_nand",
        "no_phy",
        "no_u-boot_prompt",
        "no_u-boot",
        "wrong_ram_size"
    )]
    case: &str,
) {
    let file_path = std::path::PathBuf::from(format!(
        "{}/tests/data/{case}.toml",
        env!("CARGO_MANIFEST_DIR")
    ));
    let file_content = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", &file_path.display()));
    let test_data: TestData = toml::from_str(file_content.as_str())
        .unwrap_or_else(|_| panic!("Failed to parse test data {}", &file_path.display()));
    let mut serial_port = Box::new(MockSerialPort::new());

    serial_port.expect_write().returning(|buf| Ok(buf.len()));
    serial_port.expect_flush().returning(|| Ok(()));
    serial_port.expect_read().returning({
        let mut t = test_data.clone();
        move |buf| t.read_console_output(buf)
    });

    let mut buf = std::io::Cursor::new(vec![0u8; 100_000]);

    analyze(
        &mut (serial_port as Box<dyn serialport::SerialPort>),
        &mut buf,
    );

    let output = String::from_utf8_lossy(buf.get_ref());
    let message = test_data.message.as_str();

    assert!(
        output.contains(message),
        "\"{message}\" not found in:\n{output}"
    );
}
