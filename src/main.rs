#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs, unused_variables)]

pub mod data;
pub mod settings;
pub mod sync;

use core::panic;
use eframe::egui;
use egui::Pos2;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

const SCALE: f32 = 1.5;

struct ThreadData {
    join_handle: JoinHandle<()>,
    sender: Sender<String>,
    receiver: Receiver<String>,
}

fn main() -> eframe::Result {
    let available_threads: usize = thread::available_parallelism().unwrap().into();
    let mut threads: Vec<ThreadData> = Vec::new();

    for id in 0..available_threads {
        let (send_to_main, recv_from_thread): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (send_to_thread, recv_from_main): (Sender<String>, Receiver<String>) = mpsc::channel();

        let handle = thread::Builder::new()
            .name(format!("Worker thread {id}").to_string())
            .spawn(move || {
                let mut running = true;
                while running {
                    send_to_main
                        .send("free".to_string())
                        .expect("Sending 'free' message to main");
                    let result = recv_from_main.recv();
                    let data: String;
                    match result {
                        Ok(text) => data = text,
                        Err(err) => {
                            continue;
                        }
                    }
                    let mut data_iter = data.split(";");
                    let command = data_iter.next().expect("Getting command from message");
                    match command {
                        "sync" => {
                            let save_num: usize = data_iter
                                .next()
                                .expect("Getting save number")
                                .parse()
                                .expect("Casting to usize");
                            let data = data::load_config_data();
                            let result = sync::sync_save_ftp(
                                &send_to_main,
                                save_num,
                                &data.saves[save_num].name,
                                &data.saves[save_num].path,
                                &data.ftp_config.ip,
                                &data.ftp_config.user,
                                &data.ftp_config.passwd,
                                data.ftp_config.port,
                            );
                            match result {
                                Ok(_) => (),
                                Err(err) => println!("{}", err),
                            }
                            send_to_main
                                .send("done".to_string())
                                .expect("Failed to send 'done' message to main");
                        }
                        "join" => {
                            println!("Joining thread {}", id);
                            running = false
                        }
                        _ => panic!("Invalid command sent to thread"),
                    }
                }
            })
            .unwrap();
        let thread = ThreadData {
            join_handle: handle,
            sender: send_to_thread,
            receiver: recv_from_thread,
        };
        threads.push(thread);
    }
    data::check_config_folder();
    data::load_config_data();
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_decorations(false)
            .with_transparent(true),
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
            cc.egui_ctx.set_pixels_per_point(SCALE);
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
            syncing: self.syncing,
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
            ui.label(&data.sync_info);
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
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut to_sync = Vec::new();
        let panel_frame = egui::Frame {
            fill: ctx.style().visuals.window_fill(),
            rounding: 7.5.into(),
            stroke: ctx.style().visuals.widgets.noninteractive.fg_stroke,
            inner_margin: 5.0.into(),
            ..Default::default()
        };
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                let menu_bar_response = ui.interact(
                    egui::Rect::from_points(&[
                        Pos2::new(0.0, 0.0),
                        Pos2::new(ui.max_rect().right(), 32.0),
                    ]),
                    egui::Id::new("title_bar"),
                    egui::Sense::click_and_drag(),
                );
                if menu_bar_response.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("Saves", |ui| {
                        if ui.button("New").clicked() {
                            let s = data::SaveUI {
                                name: "".to_string(),
                                path: "".to_string(),
                            };
                            let info = SaveInfo::default();
                            self.saves.push(s);
                            self.save_info.push(info);
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.button("❌").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
                let mut to_remove = Vec::new();
                let mut save_num: usize = 0;
                if self.saves.len() == 0 {
                    ui.label("No saves to show");
                }
                for mut save in &mut self.saves {
                    if self.save_info[save_num].syncing {
                        let result = self.threads[self.save_info[save_num].thread]
                            .receiver
                            .try_recv();
                        match result {
                            Ok(text) => {
                                println!("{}", text);
                                if text == "done".to_string() {
                                    self.save_info[save_num].syncing = false;
                                } else {
                                    self.save_info[save_num].sync_info = text;
                                }
                            }
                            Err(err) => (),
                        }
                    }
                    self.save_info[save_num] =
                        data::SaveUI::display(&mut save, ui, &mut self.save_info[save_num]);
                    if self.save_info[save_num].to_delete {
                        to_remove.push(save_num);
                    }
                    if self.save_info[save_num].sync_request
                        && !to_sync.contains(&save_num)
                        && !self.save_info[save_num].syncing
                    {
                        self.save_info[save_num].syncing = true;
                        self.save_info[save_num].sync_request = false;
                        to_sync.push(save_num);
                    }
                    save_num += 1;
                }
                for save_num in &mut to_remove {
                    self.save_info.remove(*save_num);
                    self.saves.remove(*save_num);
                }
                let _ = data::save_config_data(self.server.clone(), &self.ftp, &self.saves);
                for save_num in to_sync {
                    let mut thread_num = 0;
                    for t in &self.threads {
                        let result = t.receiver.try_recv();
                        println!("Initialising work on thread {}", thread_num);
                        match result {
                            Ok(str) => {
                                if str == "free".to_string() {
                                    t.sender.send(format!("sync;{}", save_num)).unwrap();
                                    self.save_info[save_num].thread = thread_num;
                                    break;
                                };
                            }
                            Err(err) => (),
                        }
                        thread_num += 1;
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
            let _ = t.sender.send("join".to_string());
            let _ = t.join_handle.join();
        }
        println!("Saved data");
    }
}
