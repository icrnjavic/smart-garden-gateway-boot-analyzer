use log::{error, info};
use serialport::SerialPort;
use smart_garden_gateway_doctor::analyzer::{analyze, Diagnosis};
use smart_garden_gateway_doctor::config::Config;
use smart_garden_gateway_doctor::jig::{open_serial_port, power_off_dut, power_on_dut};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static TITLE: &str = "GARDENA smart Gateway Doctor";
static SPACING: f32 = 20.0;

struct App {
    lm_id: String,
    serial_port_list: Vec<String>,
    serial_port_index: usize,
    serial_port: Option<Arc<Mutex<Box<dyn SerialPort>>>>,
    message: String,
    message_color: egui::Color32,
    instructions: String,
    busy: bool,
    tx: Sender<Diagnosis>,
    rx: Receiver<Diagnosis>,
}

impl Default for App {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let serial_port_list = vec![String::from("No serial port selected")];

        Self {
            lm_id: String::new(),
            serial_port_list,
            serial_port_index: 0,
            serial_port: None,
            message: String::new(),
            message_color: egui::Color32::default(),
            instructions: String::new(),
            busy: false,
            tx,
            rx,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_serial_port_info();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(egui::RichText::new(TITLE).color(egui::Color32::WHITE).size(20.0));
            

            ui.add(egui::Separator::default().spacing(SPACING));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Scan IPRID QR code: ").color(egui::Color32::WHITE).size(14.0));

                let field_resp = ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::singleline(&mut self.lm_id),
                );
                if field_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.busy = true;
                    self.check_lm_id_and_run();
                }
                if !self.busy {
                    field_resp.request_focus();
                }
            });

            ui.add(egui::Separator::default().spacing(SPACING));

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Issue:").color(egui::Color32::WHITE).size(13.0));

                ui.colored_label(self.message_color, &self.message);
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Instructions:").color(egui::Color32::WHITE).size(13.0));

                ui.colored_label(self.message_color, &self.instructions);
            });

            ui.add(egui::Separator::default().spacing(SPACING));

            egui_logger::logger_ui(ui);

            ui.add(egui::Separator::default().spacing(SPACING));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                if egui::ComboBox::from_id_source("serial_port")
                    .show_index(
                        ui,
                        &mut self.serial_port_index,
                        self.serial_port_list.len(),
                        |i| self.serial_port_list[i].as_str(),
                    )
                    .changed()
                {
                    self.open_serial_port();
                }
            });
        });

        if self.serial_port.is_none() {
            self.open_serial_port();
        }

        if let Ok(diagnosis) = self.rx.try_recv() {
            self.message = String::from(diagnosis.message);
            if let Some(instructions) = diagnosis.instructions {
                self.instructions = String::from(instructions);
            }
            self.message_color = if diagnosis.healthy {
                egui::Color32::GREEN
            } else {
                egui::Color32::RED
            };
            self.busy = false;
        }

        std::thread::sleep(Duration::from_millis(100));
        ctx.request_repaint();
    }
}

impl App {
    fn update_serial_port_info(&mut self) {
        if let Ok(ports) = serialport::available_ports() {
            let mut port_name = &self.serial_port_list[self.serial_port_index];
            let config = Config::new();
            let configured_port = &config.serial_port;
            if !configured_port.is_empty() {
                port_name = configured_port;
            }
            let mut port_names: Vec<String> = ports.into_iter().map(|p| p.port_name).collect();
            port_names.sort();
            let mut port_index = 0;
            if let Ok(i) = port_names.binary_search_by(|s| s.cmp(port_name)) {
                port_index = i + 1;
            }
            self.serial_port_list.drain(1..);
            self.serial_port_list.extend(port_names);
            self.serial_port_index = port_index;
        }
    }

    fn open_serial_port(&mut self) {
        if self.serial_port_index > 0 {
            if let Some(s) = self.serial_port.clone() {
                if s.try_lock().is_err() {
                    error!("Failed to change serial port, port in use");
                    return;
                }
            }

            let serial_port_name = self.serial_port_list[self.serial_port_index].clone();

            if let Ok(serial_port) = open_serial_port(&serial_port_name) {
                info!("Successfully opened serial port {serial_port_name}");
                self.serial_port = Some(Arc::new(Mutex::new(serial_port)));

                let mut config = Config::new();

                if let Some(s) = self.serial_port.clone() {
                    if let Ok(mut serial_port) = s.lock() {
                        power_off_dut(&mut serial_port, config.invert_rts);
                    }
                }

                config.serial_port = serial_port_name;
                config.save();
            } else {
                error!("Failed to open serial port {serial_port_name}");
            }
        }
    }

    fn abort(&mut self, error: &str) {
        error!("{error}");
        self.busy = false;
    }

    fn check_lm_id_and_run(&mut self) {
        egui_logger::clear_log();

        info!("LM ID: {}", self.lm_id);

        self.message.clear();
        self.instructions.clear();

        let re = regex::Regex::new(r"^[0-9a-f]{8}[-']([0-9a-f]{4}[-']){3}[0-9a-f]{12}$")
            .expect("Failed to create regular expression");
        if re.is_match(self.lm_id.as_str()) {
            self.run();
        } else {
            self.abort("Invalid IPRID entered");
        }

        self.lm_id.clear();
    }

    fn run(&mut self) {
        if let Some(s) = &self.serial_port {
            let s = s.clone();
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                if let Ok(mut serial_port) = s.try_lock() {
                    info!("Starting diagnosis...");

                    let config = Config::new();
                    power_on_dut(&mut serial_port, config.invert_rts);
                    let diagnosis = analyze(&mut serial_port);
                    power_off_dut(&mut serial_port, config.invert_rts);

                    if tx.send(diagnosis).is_err() {
                        error!("Failed to send diagnosis to main thread");
                    }
                    info!("Done");
                } else {
                    let diagnosis = Diagnosis {
                        message: "Failed to access serial port",
                        healthy: false,
                        ..Default::default()
                    };
                    if tx.send(diagnosis).is_err() {
                        error!("Failed to send diagnosis to main thread");
                    }
                }
            });
        } else {
            self.abort("No serial port selected");
        }
    }
}

fn main() {
    egui_logger::init_with_max_level(log::LevelFilter::Debug).unwrap();
    let _ = eframe::run_native(
        TITLE,
        eframe::NativeOptions::default(),
        Box::new(|_cc| Box::<App>::default()),
    );
}
