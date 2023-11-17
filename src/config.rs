use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub amount: usize,
    pub tolerance: usize,
    pub path: PathBuf,
    pub layers: Vec<LayerConfig>,
    pub extra: Option<Map<String, Value>>,
    #[serde(skip)]
    pub config_name: String,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct SetConfig {
    pub name: String,
    pub amount: usize,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct LayerConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_if_traits: Option<Vec<IfTrait>>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct IfTrait {
    pub layer: String,
    pub traits: Vec<String>,
}

impl AppConfig {
    pub fn load_configs(config_folders: &str) -> Result<Vec<Self>> {
        let config_files = fs::read_dir(config_folders)?;
        let mut configs: Vec<Self> = vec![];

        for file in config_files {
            let a_file = file.unwrap();
            let file_name = a_file.file_name().to_str().unwrap_or("").to_string();
            let contents = fs::read_to_string(a_file.path())?;

            let mut parsed: Self = serde_json::from_str(&contents)
                .expect(&format!("unable to parse config file: {}", file_name));

            parsed.config_name = file_name.split(".").collect::<Vec<&str>>()[0].to_string();
            configs.push(parsed);
        }

        Ok(configs)
    }
}
