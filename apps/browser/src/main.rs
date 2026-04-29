use eframe::egui;
use egui::RichText;
use webox_config::AppConfig;
use webox_memory::TabTelemetry;
use webox_shell::HostShell;
use webox_ui::{
    BrowserCommand, BrowserWindowModel, SurfaceInputEvent, SurfaceMouseButton, SurfaceViewState,
    TabViewState,
};

struct BrowserApp {
    shell: HostShell,
    window_id: String,
    address_input: String,
    last_status: String,
    window_title: String,
    show_diagnostics: bool,
    surface_texture: Option<egui::TextureHandle>,
    surface_texture_token: u64,
}

impl BrowserApp {
    fn bootstrap() -> Self {
        // In release builds use production config, which resolves all asset
        // paths relative to the executable directory (works with $ORIGIN RPATH).
        // In debug builds use development config, which resolves paths relative
        // to the workspace root CWD for a smooth `cargo run` experience.
        #[cfg(debug_assertions)]
        let config = AppConfig::development();
        #[cfg(not(debug_assertions))]
        let config = AppConfig::production();
        Self::from_config(config)
    }

    fn from_config(config: AppConfig) -> Self {
        let mut shell = HostShell::new(config);
        shell.start();

        let window_id = shell.create_window("window-1");
        let home_page = shell.config().startup.home_page.clone();
        let tab = shell.open_tab(&window_id, home_page.as_str()).ok();
        if let Some(tab) = tab {
            let _ = shell.resize_tab_surface(&tab, 1280, 768);
            let _ = shell.focus_tab_surface(&tab, true);
            if shell.live_mvp_ready() {
                let _ = shell.collect_observed_tab_telemetry(&window_id, &tab);
            } else {
                let _ = shell.record_tab_telemetry(
                    &window_id,
                    &TabTelemetry {
                        tab_id: tab,
                        renderer_bytes: 2 * 1024 * 1024,
                        browser_bytes: 512 * 1024,
                        gpu_bytes: 256 * 1024,
                    },
                );
            }
        }

        let runtime_summary = shell.runtime_readiness().summary.clone();
        let live_mvp_ready = shell.live_mvp_ready();

        Self {
            shell,
            window_id,
            address_input: home_page,
            last_status: format!(
                "Runtime: {}; live MVP ready: {}",
                runtime_summary, live_mvp_ready
            ),
            window_title: "webox - ready".to_string(),
            show_diagnostics: !live_mvp_ready,
            surface_texture: None,
            surface_texture_token: 0,
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
            self.last_status = format!(
                "Navigation requested for {}; waiting for engine-observed state",
                self.address_input
            );
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
                let _ = self.shell.resize_tab_surface(&tab, 1280, 768);
                self.sync_address_from_active_tab();
                self.last_status = format!("Opened tab {tab}; waiting for engine callbacks");
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

    fn diagnostics_lines(&self) -> Vec<String> {
        let readiness = self.shell.runtime_readiness();
        let settings = self.shell.engine().launch_settings();
        let mut lines = vec![
            format!("Runtime readiness: {:?}", readiness.state),
            format!("Live MVP ready: {}", readiness.live_mvp_ready),
            format!("Simulated mode: {}", readiness.simulated),
            format!("Summary: {}", readiness.summary),
            format!("CEF distribution root: {}", settings.distribution_root),
            format!("CEF resources: {}", settings.resources_dir),
            format!("CEF locales: {}", settings.locales_dir),
            format!("CEF subprocess: {}", settings.subprocess_path),
            format!("Data path: {}", settings.data_dir),
            format!("Cache path: {}", settings.cache_dir),
            format!("Log path: {}", settings.log_dir),
            format!("Remote debugging port: {}", settings.remote_debugging_port),
        ];

        if !readiness.missing_paths.is_empty() {
            lines.push(format!(
                "Missing runtime paths: {}",
                readiness.missing_paths.join(", ")
            ));
        }
        if !readiness.checked_paths.is_empty() {
            lines.push(format!(
                "Checked paths: {}",
                readiness.checked_paths.join("; ")
            ));
        }
        for diagnostic in self.shell.startup_diagnostics() {
            lines.push(format!(
                "Startup diagnostic [{}]: {}",
                diagnostic.component, diagnostic.detail
            ));
        }

        // Per-tab state
        if let Some(tab) = self.active_tab() {
            lines.push(format!(
                "Tab: {} | URL: {} | Loading: {}",
                tab.id, tab.url, tab.is_loading
            ));
            lines.push(format!("Status: {}", tab.status_text));
            if let Some(m) = &tab.memory_indicator {
                lines.push(format!("Memory: {m}"));
            }
            if let Some(a) = &tab.memory_attribution {
                lines.push(format!("Attribution: {a}"));
            }
            let s = &tab.surface;
            lines.push(format!(
                "Surface: frame={} damage={} size={}x{} focused={}",
                s.frame_token, s.damage_events, s.width, s.height, s.focused
            ));
            if let Some(ev) = &s.render_evidence {
                lines.push(format!("Render evidence: {ev}"));
            }
        }

        lines
    }

    fn failure_state_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let readiness = self.shell.runtime_readiness();
        if !readiness.live_mvp_ready {
            lines.push(format!("Live runtime unavailable: {}", readiness.summary));
            if !readiness.missing_paths.is_empty() {
                lines.push(format!(
                    "Missing assets: {}",
                    readiness.missing_paths.join(", ")
                ));
            }
        }
        if self.active_window().tabs.is_empty() {
            lines.push(
                "No live browser tabs are active; runtime startup may have failed.".to_string(),
            );
        }
        if let Some(tab) = self.active_tab() {
            if let Some(failure) = &tab.failure_state {
                lines.push(format!("Active tab failure: {failure}"));
            }
            if let Some(surface_failure) = &tab.surface.host_surface_failure {
                lines.push(format!("Host surface failure: {surface_failure}"));
            }
            if let Some(memory) = &tab.memory_indicator {
                lines.push(format!("Memory state: {memory}"));
            }
        }
        lines
    }

    fn sync_active_surface_metrics(&mut self, ui: &egui::Ui) {
        if let Some(tab_id) = self.active_tab_id() {
            let available = ui.available_size();
            let width = available.x.max(320.0) as u32;
            let height = available.y.max(180.0) as u32;
            let _ = self.shell.resize_tab_surface(&tab_id, width, height);
            // Focus is set once at tab creation; do not call set_focus every
            // frame as it generates unnecessary IPC noise and can interfere
            // with CEF's internal focus state during GPU / SwiftShader init.
        }
    }

    fn render_chrome_toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let active_tab = self.active_tab().cloned();
        let can_navigate = active_tab.is_some();
        let can_go_back = active_tab.as_ref().is_some_and(|tab| tab.can_go_back);
        let can_go_forward = active_tab.as_ref().is_some_and(|tab| tab.can_go_forward);
        let tab_count = self.active_window().tabs.len();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            if ui
                .add_enabled(can_go_back, egui::Button::new("<-"))
                .on_hover_text("Back")
                .clicked()
            {
                self.go_back_active();
            }

            if ui
                .add_enabled(can_go_forward, egui::Button::new("->"))
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

            let address_width = (ui.available_width() - 300.0).max(180.0);
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

            if ui.button("Diagnostics").clicked() {
                self.show_diagnostics = !self.show_diagnostics;
                self.last_status = if self.show_diagnostics {
                    "Runtime diagnostics opened".to_string()
                } else {
                    "Runtime diagnostics closed".to_string()
                };
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

    fn render_failure_banners(&self, ui: &mut egui::Ui) {
        for line in self.failure_state_lines() {
            ui.colored_label(egui::Color32::from_rgb(190, 40, 35), line);
        }
    }

    fn render_diagnostics_panel(&self, ui: &mut egui::Ui) {
        if !self.show_diagnostics {
            return;
        }

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(28, 32, 40))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(95, 110, 130),
            ))
            .show(ui, |ui| {
                ui.heading(RichText::new("Runtime Diagnostics").strong());
                for line in self.diagnostics_lines() {
                    ui.monospace(line);
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
        // Tell CEF the dimensions it should paint at before we render.
        self.sync_active_surface_metrics(ui);

        if let Some(tab) = self.active_tab().cloned() {
            let tab_id = tab.id.clone();
            let surface = tab.surface;
            let available = ui.available_size();

            // Allocate exactly the full available area and paint into it.
            // We do NOT use ui.image() here because egui's image widget maintains
            // the texture's aspect ratio by default — when the window size differs
            // from the CEF paint dimensions, gray letterbox bars appear around the
            // content. Painting directly via the painter fills the rect exactly.
            let (surface_rect, _response) =
                ui.allocate_exact_size(available, egui::Sense::click_and_drag());

            let surface_rect = if let Some(texture) = self.live_surface_texture(ui.ctx(), &surface)
            {
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter()
                    .image(texture.id(), surface_rect, uv, egui::Color32::WHITE);
                surface_rect
            } else {
                // Placeholder while waiting for the first paint from CEF.
                // Use the already-allocated surface_rect so we don't double-allocate.
                ui.painter()
                    .rect_filled(surface_rect, 0.0, egui::Color32::from_rgb(18, 18, 18));
                ui.scope_builder(egui::UiBuilder::new().max_rect(surface_rect), |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("Awaiting live browser frame")
                                .color(egui::Color32::from_gray(140))
                                .size(15.0),
                        );
                    });
                });
                surface_rect
            };

            self.route_surface_input(&tab_id, ui, surface_rect);
        }
    }

    fn live_surface_texture(
        &mut self,
        ctx: &egui::Context,
        surface: &SurfaceViewState,
    ) -> Option<&egui::TextureHandle> {
        let buffer = surface.frame_buffer.as_ref()?;
        if buffer.width == 0 || buffer.height == 0 || buffer.bgra.len() < 4 {
            return None;
        }
        if self.surface_texture_token != surface.frame_token || self.surface_texture.is_none() {
            let pixels = buffer
                .bgra
                .chunks_exact(4)
                .map(|bgra| {
                    egui::Color32::from_rgba_unmultiplied(bgra[2], bgra[1], bgra[0], bgra[3])
                })
                .collect::<Vec<_>>();
            let image =
                egui::ColorImage::new([buffer.width as usize, buffer.height as usize], pixels);
            if let Some(handle) = &mut self.surface_texture {
                // Reuse the existing GPU texture slot — avoids allocating a new
                // handle every frame which exhausts GPU context resources in WSL.
                handle.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.surface_texture = Some(ctx.load_texture(
                    format!("webox-live-surface-{}", surface.surface_id),
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            self.surface_texture_token = surface.frame_token;
        }
        self.surface_texture.as_ref()
    }

    fn route_surface_input(&mut self, tab_id: &str, ui: &egui::Ui, rect: egui::Rect) {
        let events = ui.input(|input| input.events.clone());
        for event in events {
            match event {
                egui::Event::PointerMoved(pos) if rect.contains(pos) => {
                    let local = pos - rect.min;
                    let _ = self.shell.dispatch_surface_input(
                        tab_id,
                        SurfaceInputEvent::PointerMove {
                            x: local.x as i32,
                            y: local.y as i32,
                        },
                    );
                }
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    ..
                } if rect.contains(pos) => {
                    let local = pos - rect.min;
                    let button = match button {
                        egui::PointerButton::Primary => SurfaceMouseButton::Left,
                        egui::PointerButton::Middle => SurfaceMouseButton::Middle,
                        egui::PointerButton::Secondary => SurfaceMouseButton::Right,
                        _ => SurfaceMouseButton::Left,
                    };
                    let _ = self.shell.dispatch_surface_input(
                        tab_id,
                        SurfaceInputEvent::PointerButton {
                            x: local.x as i32,
                            y: local.y as i32,
                            button,
                            pressed,
                            click_count: 1,
                        },
                    );
                }
                egui::Event::MouseWheel { delta, .. } => {
                    if let Some(pos) = ui.ctx().pointer_hover_pos() {
                        if rect.contains(pos) {
                            let local = pos - rect.min;
                            let _ = self.shell.dispatch_surface_input(
                                tab_id,
                                SurfaceInputEvent::Wheel {
                                    x: local.x as i32,
                                    y: local.y as i32,
                                    delta_x: delta.x as i32,
                                    delta_y: delta.y as i32,
                                },
                            );
                        }
                    }
                }
                egui::Event::Key { key, pressed, .. } => {
                    // Only forward to CEF when no egui widget (e.g. address bar)
                    // currently holds keyboard focus. Without this guard, typing
                    // in the address bar floods CEF with spurious key events and
                    // crashes the X connection.
                    if ui.ctx().memory(|m| m.focused().is_none())
                        && rect.contains(ui.ctx().pointer_hover_pos().unwrap_or(rect.center()))
                    {
                        let _ = self.shell.dispatch_surface_input(
                            tab_id,
                            SurfaceInputEvent::Key {
                                key_code: key_code(key),
                                pressed,
                            },
                        );
                    }
                }
                egui::Event::Text(text) => {
                    if ui.ctx().memory(|m| m.focused().is_none())
                        && rect.contains(ui.ctx().pointer_hover_pos().unwrap_or(rect.center()))
                    {
                        let _ = self
                            .shell
                            .dispatch_surface_input(tab_id, SurfaceInputEvent::Text { text });
                    }
                }
                egui::Event::WindowFocused(focused) => {
                    let _ = self
                        .shell
                        .dispatch_surface_input(tab_id, SurfaceInputEvent::Focus { focused });
                }
                _ => {}
            }
        }
    }
}

fn key_code(key: egui::Key) -> i32 {
    match key {
        egui::Key::Enter => 13,
        egui::Key::Escape => 27,
        egui::Key::Backspace => 8,
        egui::Key::Tab => 9,
        egui::Key::Space => 32,
        egui::Key::ArrowLeft => 37,
        egui::Key::ArrowUp => 38,
        egui::Key::ArrowRight => 39,
        egui::Key::ArrowDown => 40,
        egui::Key::Delete => 46,
        egui::Key::Home => 36,
        egui::Key::End => 35,
        egui::Key::PageUp => 33,
        egui::Key::PageDown => 34,
        other => format!("{:?}", other)
            .chars()
            .next()
            .map(|character| character as i32)
            .unwrap_or_default(),
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

        // Drive the CEF message loop once per frame so CEF can process
        // network requests, IPC, JavaScript timers, and on_paint callbacks.
        self.shell.tick();

        // Request continuous repaints so CEF message loop is driven every frame
        // even when there is no user input.
        ctx.request_repaint();

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

        egui::CentralPanel::default()
            // Remove the default egui inner margin so the CEF surface fills
            // edge-to-edge. Without this, CentralPanel adds ~8px padding on all
            // sides, producing visible borders around the browser content.
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                self.render_failure_banners(ui);
                self.render_diagnostics_panel(ui);
                if self.show_diagnostics || !self.failure_state_lines().is_empty() {
                    ui.separator();
                }
                if self.active_tab().is_some() {
                    self.render_live_surface(ui);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("No active tab")
                                .color(egui::Color32::from_gray(120))
                                .size(15.0),
                        );
                    });
                }
            });

        egui::Panel::bottom("status").show_inside(ui, |ui| {
            let system_report = self.shell.supported_system_report(16 * 1024 * 1024 * 1024);
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Status: {}", self.last_status));
                ui.separator();
                ui.label(format!("Live MVP ready: {}", self.shell.live_mvp_ready()));
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
            Some("Loading...")
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
        assert!(
            tab.memory_attribution
                .as_deref()
                .is_some_and(|attribution| attribution.contains("live_mvp_evidence=false"))
        );
    }

    #[test]
    fn diagnostics_surface_runtime_paths_and_failures() {
        let app = BrowserApp::from_config(AppConfig::development());

        assert!(app.show_diagnostics);
        assert!(
            app.diagnostics_lines()
                .iter()
                .any(|line| line.contains("CEF subprocess"))
        );
        assert!(
            app.diagnostics_lines()
                .iter()
                .any(|line| line.contains("Remote debugging port"))
        );
        assert!(
            app.failure_state_lines()
                .iter()
                .any(|line| line.contains("Live runtime unavailable"))
        );
    }
}
