#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_resizable(false)
            .with_maximize_button(false),
        ..Default::default()
    };
    eframe::run_native(
        "raincloud",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(2.0);
            cc.egui_ctx.set_zoom_factor(1.0);

            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    )
}

struct SaveUI {
    pub name: String,
    pub path: String,
}

impl SaveUI {
    fn display(&mut self, ui: &mut egui::Ui) -> bool {
        let mut ret = false;
        ui.horizontal(|ui| {
            ui.add_sized(
                [80.0, 20.0],
                egui::widgets::Label::new(format!("{}", self.name)),
            );
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
            if ui.button("-").clicked() {
                ret = true;
            }
        });
        ret
    }
}

struct MyApp {
    saves: Vec<SaveUI>,
    save_name_buffer: String,
}

impl Default for MyApp {
    fn default() -> Self {
        let s = Self {
            saves: Vec::new(),
            save_name_buffer: "".to_owned(),
        };
        s
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Saves");
            let mut to_remove = Vec::new();
            let mut i = 0;
            for mut save in &mut self.saves {
                if SaveUI::display(&mut save, ui) {
                    to_remove.push(i);
                }
                i += 1;
            }
            for num in &mut to_remove {
                self.saves.remove(*num);
            }
            ui.horizontal(|ui| {
                ui.add_sized(
                    [100.0, 20.0],
                    egui::TextEdit::singleline(&mut self.save_name_buffer),
                );
                if ui.button("+").clicked() {
                    self.saves.push(SaveUI {
                        name: self.save_name_buffer.clone(),
                        path: "".to_string(),
                    });
                    self.save_name_buffer = "".to_string();
                }
            })
        });
    }
}
