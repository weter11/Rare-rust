use std::process::Command;
use crate::models::{Game, InstalledGame, RareGame};
use std::collections::HashMap;

pub struct Legendary {
    pub mock: bool,
}

impl Legendary {
    pub fn new(mock: bool) -> Self {
        Self { mock }
    }

    pub fn list_games(&self) -> Result<Vec<Game>, String> {
        if self.mock {
            return Ok(vec![
                Game {
                    app_name: "Sugar".to_string(),
                    app_title: "Rocket League".to_string(),
                    asset_infos: HashMap::new(),
                    metadata: HashMap::new(),
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

    pub fn install(&self, app_name: &str) -> Result<(), String> {
        if self.mock {
            println!("Mock Install: {}", app_name);
            return Ok(());
        }

        Command::new("legendary")
            .args(&["install", app_name])
            .spawn()
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
