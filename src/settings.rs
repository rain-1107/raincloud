use core::panic;
use eframe::egui;

pub struct FTPSettings {
    pub ip: String,
    pub user: String,
    pub password: String,
    pub port: u16,
}
pub struct SettingsWindow {
    pub open: bool,
    current_tab_index: usize,
}

impl Default for SettingsWindow {
    fn default() -> Self {
        Self {
            open: false,
            current_tab_index: 0,
        }
    }
}

impl SettingsWindow {
    pub fn draw(&mut self, ctx: &egui::Context, ftp_settings: &mut FTPSettings) {
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("immediate_viewport"),
            egui::ViewportBuilder::default()
                .with_title("Server Settings")
                .with_inner_size([200.0, 250.0])
                .with_resizable(false),
            |ctx, class| {
                assert!(
                    class == egui::ViewportClass::Immediate,
                    "This egui backend doesn't support multiple viewports"
                );

                // Draw code here
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::menu::bar(ui, |ui| {
                        if ui.button("General").clicked() {
                            self.current_tab_index = 0;
                        }
                        if ui.button("FTP").clicked() {
                            self.current_tab_index = 1;
                        }
                        if ui.button("OneDrive").clicked() {
                            self.current_tab_index = 2;
                        }
                    });
                    match self.current_tab_index {
                        // General
                        0 => {
                            ui.label("General");
                        }
                        // FTP
                        1 => {
                            ui.horizontal(|ui| {
                                ui.label("IP: ");
                                ui.text_edit_singleline(&mut ftp_settings.ip);
                            });
                            ui.horizontal(|ui| {
                                ui.label("User: ");
                                ui.text_edit_singleline(&mut ftp_settings.user);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Password: ");
                                ui.text_edit_singleline(&mut ftp_settings.password);
                            });
                        }
                        // OneDrive
                        2 => {
                            ui.label("OneDrive");
                        }
                        // Panic
                        _ => panic!("How did we get here? Invalid server settings page index."),
                    }
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    self.open = false;
                }
            },
        );
    }
}
