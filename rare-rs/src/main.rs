mod models;
mod legendary;

use eframe::egui;
use legendary::Legendary;
use models::RareGame;
use std::sync::mpsc::{channel, Receiver, Sender};

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
}

#[derive(PartialEq)]
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
                        self.loading = true;
                        self.trigger_refresh();
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
                        self.loading = true;
                        self.trigger_refresh();
                    }
                }
                return;
            }

            match self.current_page {
                Page::Library => self.show_library(ui),
                Page::Downloads => {
                    ui.heading("Downloads");
                    ui.label("No active downloads.");
                }
                Page::Settings => {
                    ui.heading("Settings");
                    ui.label("General Settings");
                    ui.checkbox(&mut false, "Enable debug logs");
                }
                Page::Login => self.show_login(ui),
            }
        });

        // Ensure UI updates if we are loading
        if self.loading {
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

        egui::ScrollArea::vertical().show(ui, |ui| {
            for rgame in &self.library {
                if !self.search_query.is_empty() && !rgame.title().to_lowercase().contains(&self.search_query.to_lowercase()) {
                    continue;
                }

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(rgame.title()).strong().size(16.0));
                            ui.label(format!("Developer: {}", rgame.developer()));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if rgame.is_installed() {
                                if ui.button("Launch").clicked() {
                                    if let Err(e) = self.legendary.launch(&rgame.game.app_name) {
                                        self.status_message = format!("Launch failed: {}", e);
                                    } else {
                                        self.status_message = format!("Launched {}", rgame.title());
                                    }
                                }
                            } else {
                                if ui.button("Install").clicked() {
                                    if let Err(e) = self.legendary.install(&rgame.game.app_name) {
                                        self.status_message = format!("Install failed: {}", e);
                                    } else {
                                        self.status_message = format!("Installing {}", rgame.title());
                                    }
                                }
                            }
                        });
                    });
                });
                ui.add_space(5.0);
            }
        });
    }

    fn show_login(&mut self, ui: &mut egui::Ui) {
        ui.heading("Login to Epic Games");
        ui.add_space(10.0);
        ui.label("Rare uses Legendary as a backend. To log in, please run the following command in your terminal:");
        ui.add_space(5.0);
        ui.code("legendary auth");
        ui.add_space(5.0);
        ui.label("Then follow the instructions provided by Legendary.");

        if ui.button("I have logged in, refresh library").clicked() {
            self.legendary.mock = false;
            self.loading = true;
            self.trigger_refresh();
        }
    }
}
