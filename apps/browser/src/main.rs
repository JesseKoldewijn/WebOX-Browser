use eframe::egui;
use egui::RichText;
use webox_config::AppConfig;
use webox_memory::TabTelemetry;
use webox_shell::HostShell;
use webox_ui::{BrowserCommand, BrowserWindowModel, SurfaceViewState, TabViewState};

struct BrowserApp {
    shell: HostShell,
    window_id: String,
    address_input: String,
    last_status: String,
    window_title: String,
}

impl BrowserApp {
    fn bootstrap() -> Self {
        Self::from_config(AppConfig::development())
    }

    fn from_config(config: AppConfig) -> Self {
        let mut shell = HostShell::new(config);
        shell.start();

        let window_id = shell.create_window("window-1");
        let home_page = shell.config().startup.home_page.clone();
        let tab = shell
            .open_tab(&window_id, home_page.as_str())
            .expect("browser should open initial tab");
        shell
            .finish_navigation(&window_id, &tab, "webox home")
            .expect("browser should finish initial navigation");
        let _ = shell.resize_tab_surface(&tab, 1280, 768);
        let _ = shell.focus_tab_surface(&tab, true);
        let _ = shell.record_tab_telemetry(
            &window_id,
            &TabTelemetry {
                tab_id: tab,
                renderer_bytes: 2 * 1024 * 1024,
                browser_bytes: 512 * 1024,
                gpu_bytes: 256 * 1024,
            },
        );

        Self {
            shell,
            window_id,
            address_input: home_page,
            last_status: "CEF bootstrap configured; live browser shell running through engine-backed tab state"
                .to_string(),
            window_title: "webox - ready".to_string(),
        }
    }

    fn active_window(&self) -> &BrowserWindowModel {
        &self.shell.windows()[&self.window_id]
    }

    fn active_tab(&self) -> Option<&TabViewState> {
        let window = self.active_window();
        window
            .active_tab_id
            .as_ref()
            .and_then(|active| window.tabs.iter().find(|tab| &tab.id == active))
    }

    fn sync_address_from_active_tab(&mut self) {
        if let Some(tab) = self.active_tab() {
            self.address_input = tab.url.clone();
        }
    }

    fn active_tab_id(&self) -> Option<String> {
        self.active_tab().map(|tab| tab.id.clone())
    }

    fn active_surface(&self) -> Option<&SurfaceViewState> {
        self.active_tab().map(|tab| &tab.surface)
    }

    fn go_back_active(&mut self) {
        if let Some(tab_id) = self.active_tab_id() {
            self.dispatch(BrowserCommand::Back { tab_id });
            self.sync_address_from_active_tab();
            self.last_status = "Went back using live engine history".to_string();
        }
    }

    fn go_forward_active(&mut self) {
        if let Some(tab_id) = self.active_tab_id() {
            self.dispatch(BrowserCommand::Forward { tab_id });
            self.sync_address_from_active_tab();
            self.last_status = "Went forward using live engine history".to_string();
        }
    }

    fn reload_active(&mut self) {
        if let Some(tab_id) = self.active_tab_id() {
            self.dispatch(BrowserCommand::Reload { tab_id });
            self.last_status = "Reload requested through live engine instance".to_string();
        }
    }

    fn navigate_active_to_address(&mut self) {
        if let Some(tab_id) = self.active_tab_id() {
            self.dispatch(BrowserCommand::Navigate {
                tab_id: tab_id.clone(),
                url: self.address_input.clone(),
            });
            let title = self.address_input.clone();
            let _ = self
                .shell
                .finish_navigation(&self.window_id, &tab_id, &title);
            self.last_status = format!("Navigated to {}", self.address_input);
        }
    }

    fn activate_tab(&mut self, tab_id: &str) {
        self.dispatch(BrowserCommand::ActivateTab {
            tab_id: tab_id.to_string(),
        });
        self.sync_address_from_active_tab();
        if let Some(tab) = self.active_tab() {
            self.last_status = format!("Activated {}", tab.title);
        }
    }

    fn update_window_title(&mut self) {
        let title = if let Some(tab) = self.active_tab() {
            format!("webox - {}", tab.title)
        } else {
            "webox - no tabs".to_string()
        };
        self.window_title = title;
    }

    fn dispatch(&mut self, command: BrowserCommand) {
        if let Err(error) = self.shell.dispatch_command(&self.window_id, command) {
            self.last_status = error;
        }
    }

    fn open_new_tab(&mut self) {
        match self.shell.open_tab(&self.window_id, "https://example.com") {
            Ok(tab) => {
                let _ = self
                    .shell
                    .finish_navigation(&self.window_id, &tab, "Example");
                let _ = self.shell.resize_tab_surface(&tab, 1280, 768);
                self.sync_address_from_active_tab();
                self.last_status = format!("Opened live tab {tab}");
            }
            Err(error) => {
                self.last_status = error;
            }
        }
    }

    fn close_active_tab(&mut self) {
        if let Some(tab_id) = self.active_tab_id() {
            self.close_tab(&tab_id);
        }
    }

    fn close_tab(&mut self, tab_id: &str) {
        let closing_active = self.active_tab_id().as_deref() == Some(tab_id);
        self.dispatch(BrowserCommand::CloseTab {
            tab_id: tab_id.to_string(),
        });
        self.sync_address_from_active_tab();
        self.last_status = if closing_active {
            format!("Closed active live tab {tab_id}")
        } else {
            format!("Closed tab {tab_id}")
        };
    }

    fn stop_app(&mut self, ctx: &egui::Context) {
        self.last_status = "Window close requested".to_string();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn sync_active_surface_metrics(&mut self, ui: &egui::Ui) {
        if let Some(tab_id) = self.active_tab_id() {
            let available = ui.available_size();
            let width = available.x.max(320.0) as u32;
            let height = available.y.max(180.0) as u32;
            let _ = self.shell.resize_tab_surface(&tab_id, width, height);
            let _ = self.shell.focus_tab_surface(&tab_id, true);
        }
    }

    fn render_chrome_toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let can_navigate = self.active_tab_id().is_some();
        let tab_count = self.active_window().tabs.len();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            if ui
                .add_enabled(can_navigate, egui::Button::new("<-"))
                .on_hover_text("Back")
                .clicked()
            {
                self.go_back_active();
            }

            if ui
                .add_enabled(can_navigate, egui::Button::new("->"))
                .on_hover_text("Forward")
                .clicked()
            {
                self.go_forward_active();
            }

            if ui
                .add_enabled(can_navigate, egui::Button::new("Reload"))
                .on_hover_text("Reload active tab")
                .clicked()
            {
                self.reload_active();
            }

            ui.separator();

            let address_width = (ui.available_width() - 210.0).max(180.0);
            let address_response = ui.add_sized(
                [address_width, 28.0],
                egui::TextEdit::singleline(&mut self.address_input)
                    .hint_text("Enter URL or search target")
                    .desired_width(f32::INFINITY),
            );
            let submit = address_response.lost_focus()
                && ui.input(|input| input.key_pressed(egui::Key::Enter));

            if ui
                .add_enabled(can_navigate, egui::Button::new("Go"))
                .clicked()
                || (can_navigate && submit)
            {
                self.navigate_active_to_address();
            }

            if ui.button("+ Tab").clicked() {
                self.open_new_tab();
            }

            ui.separator();

            if ui
                .add_enabled(tab_count > 0, egui::Button::new("Close Tab"))
                .clicked()
            {
                self.close_active_tab();
            }

            if ui.button("Min").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                self.last_status = "Window minimized".to_string();
            }
            if ui.button("Max").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                self.last_status = "Window maximized".to_string();
            }
            if ui.button("Close").clicked() {
                self.stop_app(ctx);
            }
        });
    }

    fn render_tab_strip(&mut self, ui: &mut egui::Ui) {
        let tabs = self.active_window().tabs.clone();
        let active_tab_id = self.active_window().active_tab_id.clone();

        egui::ScrollArea::horizontal()
            .id_salt("tab-strip-scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for tab in tabs {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let selected = active_tab_id.as_deref() == Some(tab.id.as_str());
                                let indicator = if tab.is_loading { "*" } else { "" };
                                let memory = tab
                                    .memory_indicator
                                    .as_ref()
                                    .map(|state| format!(" [{state}]"))
                                    .unwrap_or_default();
                                let label = format!("{indicator}{}{}", tab.title, memory);

                                if ui
                                    .add_sized(
                                        [180.0, 24.0],
                                        egui::Button::new(label).selected(selected),
                                    )
                                    .clicked()
                                {
                                    self.activate_tab(&tab.id);
                                }

                                if ui.small_button("x").clicked() {
                                    self.close_tab(&tab.id);
                                }
                            });
                        });
                    }
                });
            });
    }

    fn render_live_surface(&mut self, ui: &mut egui::Ui) {
        self.sync_active_surface_metrics(ui);

        if let Some(tab) = self.active_tab().cloned() {
            let surface = tab.surface;
            let frame_color = if tab.failure_state.is_some() {
                egui::Color32::from_rgb(70, 24, 24)
            } else if tab.is_loading {
                egui::Color32::from_rgb(36, 52, 76)
            } else {
                egui::Color32::from_rgb(30, 45, 40)
            };

            egui::Frame::group(ui.style())
                .fill(frame_color)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(90)))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(
                        surface.width as f32,
                        (surface.height as f32).min(360.0),
                    ));
                    ui.vertical_centered(|ui| {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Live Browser Surface")
                                .strong()
                                .size(18.0)
                                .color(egui::Color32::WHITE),
                        );
                        ui.label(
                            RichText::new(&surface.frame_label)
                                .monospace()
                                .color(egui::Color32::from_rgb(220, 228, 224)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "surface={}  frame={}  size={}x{}  focused={}",
                                surface.surface_id,
                                surface.frame_token,
                                surface.width,
                                surface.height,
                                surface.focused
                            ))
                            .monospace()
                            .color(egui::Color32::from_rgb(190, 210, 206)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("This panel now reflects engine-driven live surface state and resizing hooks for embedded CEF rendering.")
                                .color(egui::Color32::from_rgb(225, 235, 233)),
                        );
                        if let Some(failure) = &tab.failure_state {
                            ui.add_space(8.0);
                            ui.colored_label(egui::Color32::from_rgb(255, 190, 190), failure);
                        }
                        ui.add_space(12.0);
                    });
                });
        }
    }
}

impl Drop for BrowserApp {
    fn drop(&mut self) {
        self.shell.shutdown();
    }
}

impl eframe::App for BrowserApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.update_window_title();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title.clone()));

        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            ui.add_space(4.0);
            self.render_chrome_toolbar(ui, &ctx);
            ui.add_space(4.0);
        });

        egui::Panel::top("tabs").show_inside(ui, |ui| {
            ui.add_space(2.0);
            self.render_tab_strip(ui);
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(tab) = self.active_tab().cloned() {
                ui.heading(RichText::new(&tab.title).strong());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Current URL:").strong());
                    ui.monospace(&tab.url);
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Loading:").strong());
                    ui.label(if tab.is_loading { "Yes" } else { "No" });
                });
                if let Some(memory_indicator) = &tab.memory_indicator {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 120, 20),
                        format!("Memory: {memory_indicator}"),
                    );
                }
                if let Some(attribution) = &tab.memory_attribution {
                    ui.label(format!("Memory attribution: {attribution}"));
                }
                if let Some(failure_state) = &tab.failure_state {
                    ui.colored_label(egui::Color32::from_rgb(180, 30, 30), failure_state);
                }
                ui.label(format!("Status: {}", tab.status_text));
                ui.separator();
                self.render_live_surface(ui);
            } else {
                ui.heading("No active tab");
                ui.label("Open a new tab to begin browsing.");
            }
        });

        egui::Panel::bottom("status").show_inside(ui, |ui| {
            let system_report = self.shell.supported_system_report(16 * 1024 * 1024 * 1024);
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Status: {}", self.last_status));
                ui.separator();
                ui.label(format!(
                    "High-memory target met: {}",
                    system_report.meets_target
                ));
                if let Some(tab) = self.active_tab() {
                    ui.separator();
                    ui.label(format!("Active tab: {}", tab.id));
                }
                if let Some(surface) = self.active_surface() {
                    ui.separator();
                    ui.label(format!("Surface frame: {}", surface.frame_token));
                }
            });
        });

        let _ = frame;
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("webox"),
        ..Default::default()
    };

    eframe::run_native(
        "webox",
        native_options,
        Box::new(|_cc| Ok(Box::new(BrowserApp::bootstrap()))),
    )
}

#[cfg(test)]
mod tests {
    use super::BrowserApp;
    use webox_config::AppConfig;

    #[test]
    fn chrome_actions_drive_shell_state() {
        let mut app = BrowserApp::from_config(AppConfig::simulated());
        let first_tab = app.active_tab_id().expect("initial tab should exist");

        app.address_input = "https://example.com".to_string();
        app.navigate_active_to_address();

        assert_eq!(
            app.active_tab().map(|tab| tab.url.as_str()),
            Some("https://example.com")
        );
        assert_eq!(
            app.active_tab().map(|tab| tab.title.as_str()),
            Some("https://example.com")
        );

        app.open_new_tab();
        let second_tab = app.active_tab_id().expect("new active tab should exist");
        assert_ne!(first_tab, second_tab);

        app.activate_tab(&first_tab);
        assert_eq!(app.active_tab_id().as_deref(), Some(first_tab.as_str()));
        assert_eq!(app.address_input, "https://example.com");

        app.close_tab(&first_tab);
        assert_eq!(app.active_tab_id().as_deref(), Some(second_tab.as_str()));
        assert_eq!(app.active_window().tabs.len(), 1);
    }

    #[test]
    fn back_and_forward_actions_follow_tab_history() {
        let mut app = BrowserApp::from_config(AppConfig::simulated());

        app.address_input = "https://example.com/one".to_string();
        app.navigate_active_to_address();
        app.address_input = "https://example.com/two".to_string();
        app.navigate_active_to_address();

        app.go_back_active();
        assert_eq!(
            app.active_tab().map(|tab| tab.url.as_str()),
            Some("https://example.com/one")
        );

        app.go_forward_active();
        assert_eq!(
            app.active_tab().map(|tab| tab.url.as_str()),
            Some("https://example.com/two")
        );
    }

    #[test]
    fn browser_surface_reflects_engine_metrics() {
        let app = BrowserApp::from_config(AppConfig::simulated());
        let tab = app.active_tab().expect("initial tab should exist");
        assert!(tab.surface.frame_token > 0);
        assert!(tab.surface.width >= 1280);
        assert!(tab.status_text.contains("Observed memory state"));
        assert!(tab.memory_attribution.is_some());
    }
}
