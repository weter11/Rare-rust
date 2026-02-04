mod models;
mod legendary;

use eframe::egui;
use legendary::{Legendary, InstallProgress, AuthProgress};
use models::RareGame;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::collections::HashMap;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rare",
        options,
        Box::new(|_cc| Box::new(RareApp::new(_cc))),
    )
}

struct ActiveInstall {
    progress: f32,
    status: String,
    rx: Receiver<InstallProgress>,
}

struct RareApp {
    legendary: Legendary,
    library: Vec<RareGame>,
    search_query: String,
    current_page: Page,
    error: Option<String>,
    loading: bool,
    status_message: String,
    tx: Sender<Result<Vec<RareGame>, String>>,
    rx: Receiver<Result<Vec<RareGame>, String>>,
    auth_code: String,
    active_installs: HashMap<String, ActiveInstall>,
    auth_rx: Option<Receiver<AuthProgress>>,
}

#[derive(PartialEq, Debug)]
enum Page {
    Library,
    Downloads,
    Settings,
    Login,
}

impl RareApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        let legendary = Legendary::new(false);
        let app = Self {
            legendary,
            library: Vec::new(),
            search_query: String::new(),
            current_page: Page::Library,
            error: None,
            loading: true,
            status_message: "Ready".to_string(),
            tx,
            rx,
            auth_code: String::new(),
            active_installs: HashMap::new(),
            auth_rx: None,
        };

        app.trigger_refresh();
        app
    }

    fn trigger_refresh(&self) {
        let tx = self.tx.clone();
        let legendary = Legendary::new(self.legendary.mock);
        std::thread::spawn(move || {
            let res = legendary.get_library();
            let _ = tx.send(res);
        });
    }

    fn check_updates(&mut self) {
        if let Ok(res) = self.rx.try_recv() {
            self.loading = false;
            match res {
                Ok(lib) => {
                    self.library = lib;
                    self.error = None;
                }
                Err(e) => {
                    if e.contains("No saved credentials") {
                        self.error = Some("No saved credentials".to_string());
                        self.current_page = Page::Login;
                    } else {
                        self.error = Some(e);
                        // Fallback to mock for demo
                        self.legendary.mock = true;
                        self.trigger_refresh();
                    }
                }
            }
        }

        if let Some(rx) = &self.auth_rx {
            if let Ok(res) = rx.try_recv() {
                self.loading = false;
                match res {
                    AuthProgress::Finished => {
                        self.status_message = "Successfully logged in".to_string();
                        self.error = None;
                        self.current_page = Page::Library;
                        self.refresh_library();
                    }
                    AuthProgress::Error(e) => {
                        self.status_message = format!("Login failed: {}", e);
                    }
                }
                self.auth_rx = None;
            }
        }

        let mut finished = Vec::new();
        for (app_name, install) in &mut self.active_installs {
            while let Ok(msg) = install.rx.try_recv() {
                match msg {
                    InstallProgress::Percentage(p) => install.progress = p,
                    InstallProgress::Status(s) => install.status = s,
                    InstallProgress::Finished => {
                        finished.push(app_name.clone());
                    }
                    InstallProgress::Error(e) => {
                        self.status_message = format!("Error installing {}: {}", app_name, e);
                        finished.push(app_name.clone());
                    }
                }
            }
        }

        for app_name in finished {
            self.active_installs.remove(&app_name);
            self.refresh_library();
        }
    }

    fn refresh_library(&mut self) {
        self.loading = true;
        self.trigger_refresh();
    }
}

impl eframe::App for RareApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_updates();

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Rare");
            ui.separator();

            ui.selectable_value(&mut self.current_page, Page::Library, "Library");
            ui.selectable_value(&mut self.current_page, Page::Downloads, "Downloads");
            ui.selectable_value(&mut self.current_page, Page::Settings, "Settings");
            ui.selectable_value(&mut self.current_page, Page::Login, "Login");

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.label(&self.status_message);
                if self.loading {
                    ui.spinner();
                } else {
                    if ui.button("Refresh").clicked() {
                        self.refresh_library();
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                if error == "No saved credentials" {
                    self.show_login(ui);
                } else {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                    if ui.button("Retry").clicked() {
                        self.refresh_library();
                    }
                }
                return;
            }

            match self.current_page {
                Page::Library => self.show_library(ui),
                Page::Downloads => self.show_downloads(ui),
                Page::Settings => {
                    ui.heading("Settings");
                    ui.label("General Settings");
                    ui.checkbox(&mut self.legendary.mock, "Mock mode");
                }
                Page::Login => self.show_login(ui),
            }
        });

        if self.loading || !self.active_installs.is_empty() || self.auth_rx.is_some() {
            ctx.request_repaint();
        }
    }
}

impl RareApp {
    fn show_library(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Library");
            ui.add(egui::TextEdit::singleline(&mut self.search_query).hint_text("Search games..."));
        });
        ui.separator();

        let dlc_map: HashMap<String, Vec<&RareGame>> = self.library.iter()
            .filter(|g| g.is_dlc())
            .fold(HashMap::new(), |mut acc, g| {
                if let Some(main) = g.main_game_app_name() {
                    acc.entry(main).or_default().push(g);
                }
                acc
            });

        egui::ScrollArea::vertical().show(ui, |ui| {
            for rgame in &self.library {
                if rgame.is_dlc() { continue; }
                if !self.search_query.is_empty() && !rgame.title().to_lowercase().contains(&self.search_query.to_lowercase()) {
                    continue;
                }

                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(rgame.title()).strong().size(16.0));
                                ui.label(format!("Developer: {}", rgame.developer()));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if let Some(install) = self.active_installs.get(&rgame.game.app_name) {
                                    ui.add(egui::ProgressBar::new(install.progress / 100.0).text(format!("{:.1}%", install.progress)));
                                } else if rgame.is_installed() {
                                    if ui.button("Launch").clicked() {
                                        if let Err(e) = self.legendary.launch(&rgame.game.app_name) {
                                            self.status_message = format!("Launch failed: {}", e);
                                        } else {
                                            self.status_message = format!("Launched {}", rgame.title());
                                        }
                                    }
                                } else {
                                    if ui.button("Install").clicked() {
                                        let rx = self.legendary.install_game(rgame.game.app_name.clone());
                                        self.active_installs.insert(rgame.game.app_name.clone(), ActiveInstall {
                                            progress: 0.0,
                                            status: "Starting...".to_string(),
                                            rx,
                                        });
                                    }
                                }
                            });
                        });

                        if let Some(dlcs) = dlc_map.get(&rgame.game.app_name) {
                            ui.collapsing("DLCs", |ui| {
                                for dlc in dlcs {
                                    ui.horizontal(|ui| {
                                        ui.label(dlc.title());
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if let Some(install) = self.active_installs.get(&dlc.game.app_name) {
                                                ui.label(format!("{:.1}%", install.progress));
                                            } else if dlc.is_installed() {
                                                ui.label("Installed");
                                            } else {
                                                if ui.button("Install").clicked() {
                                                    let rx = self.legendary.install_game(dlc.game.app_name.clone());
                                                    self.active_installs.insert(dlc.game.app_name.clone(), ActiveInstall {
                                                        progress: 0.0,
                                                        status: "Starting...".to_string(),
                                                        rx,
                                                    });
                                                }
                                            }
                                        });
                                    });
                                }
                            });
                        }
                    });
                });
                ui.add_space(5.0);
            }
        });
    }

    fn show_downloads(&mut self, ui: &mut egui::Ui) {
        ui.heading("Downloads");
        ui.separator();
        if self.active_installs.is_empty() {
            ui.label("No active downloads.");
        } else {
            for (app_name, install) in &self.active_installs {
                ui.group(|ui| {
                    ui.label(app_name);
                    ui.add(egui::ProgressBar::new(install.progress / 100.0).text(format!("{:.1}%", install.progress)));
                    ui.label(&install.status);
                });
            }
        }
    }

    fn show_login(&mut self, ui: &mut egui::Ui) {
        ui.heading("Login to Epic Games");
        ui.add_space(10.0);
        ui.label("Rare uses Legendary as a backend. To log in, please click the button below to get an authorization code:");
        ui.add_space(5.0);

        if ui.button("Open Login URL in Browser").clicked() {
            let _ = webbrowser::open("https://www.epicgames.com/id/api/redirect?clientId=34a02cf8f4414e29b15921876da36f9a&responseType=code");
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label("Authorization Code:");
            ui.text_edit_singleline(&mut self.auth_code);
        });

        if ui.button("Log In").clicked() && !self.loading {
            self.loading = true;
            self.auth_rx = Some(self.legendary.auth_with_code(self.auth_code.clone()));
        }

        ui.add_space(20.0);
        ui.separator();
        ui.label("Alternatively, use legendary auth in your terminal.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initial_state() {
        let (tx, rx) = channel();
        let legendary = Legendary::new(true); // mock
        let app = RareApp {
            legendary,
            library: Vec::new(),
            search_query: String::new(),
            current_page: Page::Library,
            error: None,
            loading: true,
            status_message: "Ready".to_string(),
            tx,
            rx,
            auth_code: String::new(),
            active_installs: HashMap::new(),
            auth_rx: None,
        };
        assert_eq!(app.current_page, Page::Library);
        assert!(app.loading);
    }
}
