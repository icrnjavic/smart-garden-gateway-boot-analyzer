use log::{error, info};
use smart_garden_gateway_doctor::analyzer::{analyze, Analysis};
use smart_garden_gateway_doctor::config::Config;
use smart_garden_gateway_doctor::jig::{open_serial_port, power_off_dut, power_on_dut};
use std::time::Duration;

static TITLE: &str = "GARDENA smart Gateway Doctor";
static SPACING: f32 = 20.0;

struct App {
    lm_id: String,
    serial_port_list: Vec<String>,
    serial_port_index: usize,
    message: String,
    instructions: String,
}

impl Default for App {
    fn default() -> Self {
        let serial_port_list = vec![String::from("No serial port selected")];

        Self {
            lm_id: String::new(),
            serial_port_list,
            serial_port_index: 0,
            message: String::new(),
            instructions: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_serial_port_list();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(egui::RichText::new(TITLE).size(20.0));

            ui.add(egui::Separator::default().spacing(SPACING));
            ui.horizontal(|ui| {
                ui.label("Enter PCB ID: ");
                let field_resp =
                    ui.add_sized([650.0, 20.0], egui::TextEdit::singleline(&mut self.lm_id));
                let enter_pressed =
                    field_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let button_presp = ui.button("Start");
                if enter_pressed || button_presp.clicked() {
                    if self.serial_port_index > 0 {
                        info!("LM ID: {}", self.lm_id);
                        self.message.clear();
                        self.instructions.clear();
                        let analysis = run(self.serial_port_list[self.serial_port_index].as_str());
                        self.message = String::from(analysis.message);
                        if let Some(instructions) = analysis.instructions {
                            self.instructions = String::from(instructions);
                        }
                    } else {
                        error!("No serial port selected");
                    }
                }
            });

            ui.add(egui::Separator::default().spacing(SPACING));

            ui.label(format!("Result: {}", self.message));
            ui.label(format!("Instructions: {}", self.instructions));

            ui.add(egui::Separator::default().spacing(SPACING));

            egui::ScrollArea::vertical()
                .id_source("some inner")
                .max_height(400.0)
                .show(ui, |ui| {
                    ui.push_id("second", |ui| {
                        egui_logger::logger_ui(ui);
                    });
                });

            ui.add(egui::Separator::default().spacing(SPACING));

            if egui::ComboBox::from_id_source("serial_port")
                .show_index(
                    ui,
                    &mut self.serial_port_index,
                    self.serial_port_list.len(),
                    |i| self.serial_port_list[i].as_str(),
                )
                .changed()
            {
                self.select_serial_port();
            }
        });
        std::thread::sleep(Duration::from_millis(100));
        ctx.request_repaint();
    }
}

impl App {
    fn update_serial_port_list(&mut self) {
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

    fn select_serial_port(&mut self) {
        if self.serial_port_index > 0 {
            let serial_port_name = self.serial_port_list[self.serial_port_index].clone();
            info!("Serial port {serial_port_name} selected");

            let mut config = Config::new();
            config.serial_port = serial_port_name;
            config.save();
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

fn run(serial_port_name: &str) -> Analysis {
    // TODO: run in separate thread?
    let mut analysis = Analysis::default();

    if let Ok(mut serial_port) = open_serial_port(serial_port_name) {
        info!("Starting analysis...");

        let config = Config::new();
        power_on_dut(&mut serial_port, config.invert_rts);
        analysis = analyze(&mut serial_port);
        power_off_dut(&mut serial_port, config.invert_rts);

        info!("Done");
    } else {
        error!("Failed to open serial port {serial_port_name}");
    }

    analysis
}
