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
                                egui::Pos2::new(0.0, 0.0),
                                egui::Pos2::new(ui.max_rect().right(), 32.0),
                            ]),
                            egui::Id::new("settings_title_bar"),
                            egui::Sense::click_and_drag(),
                        );
                        if menu_bar_response.drag_started_by(egui::PointerButton::Primary) {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
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
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                if ui.button("❌").clicked() {
                                    self.open = false;
                                }
                            });
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
