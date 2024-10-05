#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

pub mod data;
pub mod server_settings;

// TODO: connecting to ftp server and syncing logic
// TODO: connecting to raincloud server and syncing logic

use eframe::egui;

fn main() -> eframe::Result {
    data::check_config_folder();
    data::load_config_data();
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_resizable(true)
            .with_maximize_button(false),
        ..Default::default()
    };
    let result = eframe::run_native(
        "raincloud",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            cc.egui_ctx.set_pixels_per_point(2.0);
            cc.egui_ctx.set_zoom_factor(1.0);
            Ok(Box::<MyApp>::default())
        }),
    );
    result
}

struct SaveData {
    to_delete: bool,
    editing: bool,
}

impl data::SaveUI {
    fn display(&mut self, ui: &mut egui::Ui, edit: bool) -> SaveData {
        let mut data = SaveData {
            to_delete: false,
            editing: edit,
        };
        ui.horizontal(|ui| {
            if data.editing {
                ui.add_sized([80.0, 20.0], egui::TextEdit::singleline(&mut self.name));
                if ui.button("Done").clicked() {
                    data.editing = false;
                }
            } else {
                ui.add_sized(
                    [80.0, 20.0],
                    egui::widgets::Label::new(format!("{}", self.name)),
                );
                if ui.button("Edit").clicked() {
                    data.editing = true;
                }
            }
            ui.text_edit_singleline(&mut self.path);
            if ui.button("Folder").clicked() {
                let result = rfd::FileDialog::new().set_directory("~").pick_folder();
                if result != None {
                    let result = result.unwrap().to_str().unwrap().to_string();
                    self.path = result;
                }
            }
            if ui.button("Sync").clicked() {
                println!("Hello");
            }
            if ui.button("Delete").clicked() {
                data.to_delete = true;
            }
        });
        data
    }
}

struct MyApp {
    server: String,
    ftp: data::FtpDetails,
    saves: Vec<data::SaveUI>,
    editing: i64,
    server_settings_window: server_settings::ServerSettingsWindow,
}

impl Default for MyApp {
    fn default() -> Self {
        let data = data::load_config_data();
        Self {
            server: data.server,
            ftp: data.ftp_config,
            saves: data.saves.clone(),
            editing: -1,
            server_settings_window: server_settings::ServerSettingsWindow::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Saves", |ui| {
                    if ui.button("New").clicked() {
                        let s = data::SaveUI {
                            name: "".to_string(),
                            path: "".to_string(),
                        };
                        self.saves.push(s);
                        self.editing = (self.saves.len() - 1) as i64;
                    }
                    if ui.button("Sync All").clicked() {}
                });
                ui.menu_button("Server", |ui| {
                    ui.label("Selected Server");
                    let mut temp = self.server == "ftp".to_string();
                    if ui.checkbox(&mut temp, "FTP").clicked() && temp {
                        self.server = "ftp".to_string();
                    }
                    let mut temp = self.server == "onedrive".to_string();
                    if ui.checkbox(&mut temp, "Onedrive").clicked() && temp {
                        self.server = "onedrive".to_string();
                    }
                    if ui.button("Settings").clicked() {
                        self.server_settings_window.open = true;
                    }
                });
            });
            let mut to_remove = Vec::new();
            let mut i: usize = 0;
            if self.saves.len() == 0 {
                ui.label("No saves to show");
            }
            for mut save in &mut self.saves {
                let data = data::SaveUI::display(&mut save, ui, i == self.editing as usize);
                if data.to_delete {
                    to_remove.push(i);
                }
                if data.editing {
                    self.editing = i as i64;
                } else {
                    if self.editing == i as i64 {
                        self.editing = -1
                    }
                }
                i += 1;
            }
            for num in &mut to_remove {
                self.saves.remove(*num);
            }
            if self.server_settings_window.open {
                let mut ftp = server_settings::FTPSettings {
                    ip: self.ftp.ip.clone(),
                    user: self.ftp.user.clone(),
                    password: self.ftp.passwd.clone(),
                    port: self.ftp.port,
                };
                self.server_settings_window.draw(ctx, &mut ftp);
                self.ftp.ip = ftp.ip.clone();
                self.ftp.user = ftp.user.clone();
                self.ftp.passwd = ftp.password.clone();
                self.ftp.port = ftp.port;
            }
        });
    }

    fn on_exit(&mut self, _: std::option::Option<&eframe::glow::Context>) {
        let _ = data::save_config_data(self.server.clone(), &self.ftp, &self.saves);
        println!("Saved data");
    }
}
