#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs, unused_variables)]

pub mod data;
pub mod settings;
pub mod sync;

use core::panic;
use eframe::egui;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

struct ThreadData {
    join_handle: JoinHandle<()>,
    sender: Sender<String>,
    receiver: Receiver<String>,
}

fn main() -> eframe::Result {
    let n: usize = thread::available_parallelism().unwrap().into();
    let mut threads: Vec<ThreadData> = Vec::new();

    for id in 1..(n - 1) {
        let (sender, thread_recv): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (thread_send, recv): (Sender<String>, Receiver<String>) = mpsc::channel();

        let handle = thread::Builder::new()
            .name(format!("Worker thread {id}").to_string())
            .spawn(move || {
                let mut running = true;
                while running {
                    let _err = sender.send("free".to_string());
                    let data: String = recv.recv().unwrap();
                    let mut data_iter = data.split(";");
                    let command = data_iter.next().unwrap();
                    match command {
                        "sync" => {
                            let save_num: usize = data_iter.next().unwrap().parse().unwrap();
                            let data = data::load_config_data();
                            sync::sync_save_ftp(
                                &sender,
                                save_num,
                                &data.saves[save_num].name,
                                &data.saves[save_num].path,
                                &data.ftp_config.ip,
                                &data.ftp_config.user,
                                &data.ftp_config.passwd,
                                data.ftp_config.port,
                            )
                            .unwrap();
                        }
                        "kill" => running = false,
                        _ => panic!("Invalid command sent to thread"),
                    }
                }
            })
            .unwrap();
        let thread = ThreadData {
            join_handle: handle,
            sender: thread_send.clone(),
            receiver: thread_recv,
        };
        threads.push(thread);
    }
    data::check_config_folder();
    data::load_config_data();
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_resizable(true)
            .with_maximize_button(false),
        ..Default::default()
    };
    let app = MyApp {
        threads,
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
            Ok(Box::new(app))
        }),
    );
    result
}

struct SaveInfo {
    to_delete: bool,
    editing: bool,
    sync_info: String,
    sync_request: bool,
    syncing: bool,
    thread: usize,
}
impl Default for SaveInfo {
    fn default() -> Self {
        Self {
            to_delete: false,
            editing: false,
            sync_info: "".to_string(),
            sync_request: false,
            syncing: false,
            thread: 0,
        }
    }
}
impl Clone for SaveInfo {
    fn clone(&self) -> Self {
        Self {
            thread: self.thread,
            to_delete: self.to_delete,
            editing: self.editing,
            sync_info: self.sync_info.clone(),
            sync_request: self.sync_request,
            syncing: false,
        }
    }
}

impl data::SaveUI {
    fn display(&mut self, ui: &mut egui::Ui, data: &mut SaveInfo) -> SaveInfo {
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
        data.clone()
    }
}

struct MyApp {
    server: String,
    ftp: data::FtpDetails,
    saves: Vec<data::SaveUI>,
    save_info: Vec<SaveInfo>,
    settings_window: settings::SettingsWindow,
    threads: Vec<ThreadData>,
}

impl Default for MyApp {
    fn default() -> Self {
        let data = data::load_config_data();
        let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
        let mut save_info = Vec::new();
        for save in &data.saves {
            save_info.push(SaveInfo::default());
        }
        Self {
            server: data.server,
            ftp: data.ftp_config,
            saves: data.saves.clone(),
            save_info,
            settings_window: settings::SettingsWindow::default(),
            threads: Vec::new(),
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
            for mut save in &mut self.saves {
                let result = self.threads[self.save_info[i].thread].receiver.try_recv(); // TODO: get text from specific thread
                match result {
                    Ok(text) => {}
                    Err(err) => (),
                }
                let mut data = data::SaveUI::display(&mut save, ui, &mut self.save_info[i]);
                if data.to_delete {
                    to_remove.push(i);
                }
                if data.sync_request && !to_sync.contains(&i) && !self.save_info[i].syncing {
                    data.syncing = true;
                    data.sync_request = false;
                    to_sync.push(i);
                }
                self.save_info[i] = data.clone();
                i += 1;
            }
            for num in &mut to_remove {
                self.saves.remove(*num);
            }
            let _ = data::save_config_data(self.server.clone(), &self.ftp, &self.saves);
            for num in to_sync {
                let mut i = 0;
                for t in &self.threads {
                    let result = t.receiver.try_recv();
                    match result {
                        Ok(str) => {
                            if str == "free".to_string() {
                                t.sender.send(format!("sync;{}", num)).unwrap();
                                self.save_info[i].thread = i;
                                break;
                            };
                        }
                        Err(err) => (),
                    }
                    i += 1;
                }
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
        let _err = data::purge_tmp_folder();
        let _err = data::save_config_data(self.server.clone(), &self.ftp, &self.saves);
        while self.threads.len() > 0 {
            let t = self.threads.remove(0);
            let _ = t.sender.send("kill".to_string());
            let _ = t.join_handle.join();
        }
        println!("Saved data");
    }
}
