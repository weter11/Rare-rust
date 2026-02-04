use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub app_name: String,
    pub asset_id: String,
    pub build_version: String,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub app_name: String,
    pub app_title: String,
    #[serde(default)]
    pub asset_infos: HashMap<String, AssetInfo>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledGame {
    pub app_name: String,
    pub install_path: String,
    pub version: String,
    pub platform: String,
    pub executable: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RareGame {
    pub game: Game,
    pub installed: Option<InstalledGame>,
}

impl RareGame {
    pub fn is_installed(&self) -> bool {
        self.installed.is_some()
    }

    pub fn title(&self) -> &str {
        &self.game.app_title
    }

    pub fn developer(&self) -> &str {
        self.game.metadata.get("developer")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
    }

    pub fn is_dlc(&self) -> bool {
        self.game.metadata.contains_key("mainGameItem")
    }

    pub fn main_game_app_name(&self) -> Option<String> {
        self.game.metadata.get("mainGameItem")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}
