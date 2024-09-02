#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

// TODO: connecting to ftp server and syncing logic
// TODO: connecting to raincloud server and syncing logic

use serde_json::Result;
use std::fs;
use eframe::egui;

const CONFIG_DIR: &str = ".rc";

fn check_config_folder() {
    let mut home = home::home_dir().unwrap();
    home.push(CONFIG_DIR);
    if !home.exists() {
        let _ = fs::create_dir(&home);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Json {
    ftp_config: FtpDetails,
    saves: Vec<SaveUI>,
}

impl Default for Json {
    fn default() -> Self {
        Self {
            ftp_config: FtpDetails {ip: "".to_owned(), user: "".to_owned(), passwd: "".to_owned(), port: 21},
            saves: Vec::new(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct FtpDetails {
    ip: String,
    user: String,
    passwd: String,
    port: i64,
}

fn save_config_data(ftp_details: &FtpDetails, saves: &Vec<SaveUI>) -> Result<()> {
    let mut home = home::home_dir().unwrap(); 
    home.push(CONFIG_DIR);
    home.push("config.json");
    let json_data = Json {ftp_config: ftp_details.clone(), saves: saves.to_vec()};
    let j = serde_json::to_string(&json_data)?;
    let path = &home;
    fs::write(path, &j).expect("Unable to write file");
    Ok(()) 
}

fn load_config_data() -> Json {
    let mut home = home::home_dir().unwrap();
    home.push(CONFIG_DIR);
    home.push("config.json");
    let file_result = fs::read(&home);
    let file_slice = match file_result {
        Ok(file) => file,
        Err(_error) => return Json::default(),
    };
    serde_json::from_slice(&file_slice).unwrap()
}

fn main() -> eframe::Result {
    check_config_folder();
    load_config_data();
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_resizable(false)
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SaveUI {
    pub name: String,
    pub path: String,
}

impl SaveUI {
    fn display(&mut self, ui: &mut egui::Ui, edit: bool) -> SaveData {
        let mut data = SaveData {to_delete: false, editing: edit};
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
                let result = rfd::FileDialog::new().set_directory("/").pick_folder();
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
    ftp: FtpDetails,
    saves: Vec<SaveUI>,
    editing: i64,
}

impl Default for MyApp {
    fn default() -> Self {
        let data = load_config_data();
        Self {
            ftp: data.ftp_config,
            saves: data.saves.clone(),
            editing: -1,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Saves", |ui| {
                    if ui.button("New").clicked() {
                        let s = SaveUI {name: "".to_string(), path: "".to_string()};
                        self.saves.push(s);
                    }
                });
            });
            let mut to_remove = Vec::new();
            let mut i: usize = 0;
            for mut save in &mut self.saves {
                let data = SaveUI::display(&mut save, ui, i == self.editing as usize);
                if data.to_delete { to_remove.push(i); }
                if data.editing { self.editing = i as i64; } else { if self.editing == i as i64 { self.editing = -1}}
                i += 1;
            }
            for num in &mut to_remove {
                self.saves.remove(*num);
            }
        });
    }

    fn on_exit(&mut self, _: std::option::Option<&eframe::glow::Context>) {
        let _ = save_config_data(&self.ftp, &self.saves);
        println!("Saved data");
    }
}
