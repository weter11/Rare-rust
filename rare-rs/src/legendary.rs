use std::process::{Command, Stdio};
use crate::models::{Game, InstalledGame, RareGame};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use regex::Regex;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

pub struct Legendary {
    pub mock: bool,
}

#[derive(Debug)]
pub enum InstallProgress {
    Percentage(f32),
    Status(String),
    Finished,
    Error(String),
}

#[derive(Debug)]
pub enum AuthProgress {
    Finished,
    Error(String),
}

impl Legendary {
    pub fn new(mock: bool) -> Self {
        Self { mock }
    }

    pub fn list_games(&self) -> Result<Vec<Game>, String> {
        if self.mock {
            let mut rl_metadata = HashMap::new();
            rl_metadata.insert("developer".to_string(), serde_json::json!("Psyonix"));

            let mut dlc_metadata = HashMap::new();
            dlc_metadata.insert("mainGameItem".to_string(), serde_json::json!("Sugar"));
            dlc_metadata.insert("developer".to_string(), serde_json::json!("Psyonix"));

            return Ok(vec![
                Game {
                    app_name: "Sugar".to_string(),
                    app_title: "Rocket League".to_string(),
                    asset_infos: HashMap::new(),
                    metadata: rl_metadata,
                },
                Game {
                    app_name: "SugarDLC1".to_string(),
                    app_title: "Rocket League - Supersonic Fury".to_string(),
                    asset_infos: HashMap::new(),
                    metadata: dlc_metadata,
                },
                Game {
                    app_name: "Burrito".to_string(),
                    app_title: "Fortnite".to_string(),
                    asset_infos: HashMap::new(),
                    metadata: HashMap::new(),
                },
            ]);
        }

        let output = Command::new("legendary")
            .args(&["list-games", "--json"])
            .output()
            .map_err(|e| e.to_string())?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No saved credentials") {
            return Err("No saved credentials".to_string());
        }

        if !output.status.success() {
            return Err(stderr.to_string());
        }

        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
    }

    pub fn list_installed(&self) -> Result<Vec<InstalledGame>, String> {
        if self.mock {
            return Ok(vec![
                InstalledGame {
                    app_name: "Sugar".to_string(),
                    install_path: "/mock/Rocket League".to_string(),
                    version: "1.0".to_string(),
                    platform: "Windows".to_string(),
                    executable: Some("RocketLeague.exe".to_string()),
                }
            ]);
        }

        let output = Command::new("legendary")
            .args(&["list", "--json"])
            .output()
            .map_err(|e| e.to_string())?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No saved credentials") {
            return Err("No saved credentials".to_string());
        }

        if !output.status.success() {
            return Err(stderr.to_string());
        }

        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
    }

    pub fn get_library(&self) -> Result<Vec<RareGame>, String> {
        let games = self.list_games()?;
        let installed = self.list_installed()?;
        let mut installed_map: HashMap<String, InstalledGame> = installed
            .into_iter()
            .map(|ig| (ig.app_name.clone(), ig))
            .collect();

        let library = games.into_iter().map(|g| {
            let ig = installed_map.remove(&g.app_name);
            RareGame {
                game: g,
                installed: ig,
            }
        }).collect();

        Ok(library)
    }

    pub fn launch(&self, app_name: &str) -> Result<(), String> {
        if self.mock {
            println!("Mock Launch: {}", app_name);
            return Ok(());
        }

        Command::new("legendary")
            .args(&["launch", app_name])
            .spawn()
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn auth_with_code(&self, code: String) -> Receiver<AuthProgress> {
        let (tx, rx) = channel();
        let is_mock = self.mock;

        std::thread::spawn(move || {
            if is_mock {
                std::thread::sleep(Duration::from_millis(500));
                let _ = tx.send(AuthProgress::Finished);
                return;
            }

            let output = match Command::new("legendary")
                .args(&["auth", "--code", &code])
                .output() {
                    Ok(o) => o,
                    Err(e) => {
                        let _ = tx.send(AuthProgress::Error(e.to_string()));
                        return;
                    }
                };

            if output.status.success() {
                let _ = tx.send(AuthProgress::Finished);
            } else {
                let _ = tx.send(AuthProgress::Error(String::from_utf8_lossy(&output.stderr).to_string()));
            }
        });

        rx
    }

    pub fn install_game(&self, app_name: String) -> Receiver<InstallProgress> {
        let (tx, rx) = channel();
        let is_mock = self.mock;

        std::thread::spawn(move || {
            if is_mock {
                let _ = tx.send(InstallProgress::Status("Starting mock install...".to_string()));
                for i in 0..=10 {
                    std::thread::sleep(Duration::from_millis(200));
                    let _ = tx.send(InstallProgress::Percentage(i as f32 * 10.0));
                }
                let _ = tx.send(InstallProgress::Finished);
                return;
            }

            let mut child = match Command::new("legendary")
                .args(&["install", &app_name, "--skip-sdl"])
                .stderr(Stdio::piped())
                .stdout(Stdio::null())
                .spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(InstallProgress::Error(e.to_string()));
                        return;
                    }
                };

            let stderr = child.stderr.take().unwrap();
            let reader = BufReader::new(stderr);
            let re = Regex::new(r"Progress: (\d+\.\d+)%").unwrap();

            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Some(caps) = re.captures(&line) {
                        if let Ok(p) = caps[1].parse::<f32>() {
                            let _ = tx.send(InstallProgress::Percentage(p));
                        }
                    } else {
                        let _ = tx.send(InstallProgress::Status(line));
                    }
                }
            }

            let status = child.wait().unwrap();
            if status.success() {
                let _ = tx.send(InstallProgress::Finished);
            } else {
                let _ = tx.send(InstallProgress::Error("Legendary exited with error".to_string()));
            }
        });

        rx
    }
}
