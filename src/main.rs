#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

pub mod data;
pub mod settings;
pub mod sync;

use std::sync::mpsc;

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
    sync_request: bool,
}

impl data::SaveUI {
    fn display(&mut self, ui: &mut egui::Ui, edit: bool) -> SaveData {
        let mut data = SaveData {
            to_delete: false,
            editing: edit,
            sync_request: false,
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
                data.sync_request = true;
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
    settings_window: settings::SettingsWindow,
    // TODO: add threads and channels to keep track of concurrent processes
}

impl Default for MyApp {
    fn default() -> Self {
        let data = data::load_config_data();
        Self {
            server: data.server,
            ftp: data.ftp_config,
            saves: data.saves.clone(),
            editing: -1,
            settings_window: settings::SettingsWindow::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut to_sync = Vec::new();
        egui::CentralPanel::default().show(ctx, |ui| {
            // TODO: add custom window top thing
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
                    if ui.button("Sync All").clicked() {
                        let mut n = 0;
                        for _ in &self.saves {
                            to_sync.push(n);
                            n += 1;
                        }
                    }
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
                        self.settings_window.open = true;
                    }
                });
            });
            let mut to_remove = Vec::new();
            let mut i: usize = 0;
            if self.saves.len() == 0 {
                ui.label("No saves to show");
            }
            // TODO: implement info text recieved from threads of saves
            for mut save in &mut self.saves {
                let data = data::SaveUI::display(&mut save, ui, i == self.editing as usize);
                if data.to_delete {
                    to_remove.push(i);
                }
                if data.sync_request && !to_sync.contains(&i) {
                    to_sync.push(i);
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
            let _ = data::save_config_data(self.server.clone(), &self.ftp, &self.saves);
            for num in to_sync {
                std::thread::spawn(move || {
                    let i = num.clone();
                    let data = data::load_config_data();
                    let res = sync::sync_save_ftp(
                        &data.saves[i].name,
                        &data.saves[i].path,
                        &data.ftp_config.ip,
                        &data.ftp_config.user,
                        &data.ftp_config.passwd,
                        data.ftp_config.port,
                    );
                    match res {
                        Ok(()) => println!("Success for {}", &data.saves[i].name),
                        Err(err) => println!("Failed. {}", err),
                    }
                });
            }
            if self.settings_window.open {
                let mut ftp = settings::FTPSettings {
                    ip: self.ftp.ip.clone(),
                    user: self.ftp.user.clone(),
                    password: self.ftp.passwd.clone(),
                    port: self.ftp.port,
                };
                self.settings_window.draw(ctx, &mut ftp);
                self.ftp.ip = ftp.ip.clone();
                self.ftp.user = ftp.user.clone();
                self.ftp.passwd = ftp.password.clone();
                self.ftp.port = ftp.port;
            }
        });
    }

    fn on_exit(&mut self, _: std::option::Option<&eframe::glow::Context>) {
        data::purge_tmp_folder().unwrap();
        let _ = data::save_config_data(self.server.clone(), &self.ftp, &self.saves);
        // TODO: Handle any threads still running
        println!("Saved data");
    }
}
