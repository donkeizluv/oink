use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context};
use config::{Config, File};
use dialoguer::{console::Term, theme::ColorfulTheme, Confirm, Input, Select};
use directories_next::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub start_at_one: bool,
    pub amount: usize,
    pub tolerance: usize,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sets: Option<Vec<SetConfig>>,
    pub layers: Vec<LayerConfig>,
    pub extra: Option<Map<String, Value>>,
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
    pub exclude_if_sets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_if_traits: Option<Vec<IfTrait>>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct IfTrait {
    pub layer: String,
    pub traits: Vec<String>,
}

impl AppConfig {
    pub fn new(file_name: &str) -> anyhow::Result<Self> {
        let mut s = Config::default();

        let (_, global_path) = get_global_config_paths()?;

        let global_file_name = global_path
            .to_str()
            .context("failed to load global config")?;

        s.merge(File::with_name(global_file_name).required(false))?;

        s.merge(File::with_name(file_name).required(true))?;

        s.try_into()
            .map_err(|e| anyhow!("loading config\nReason: {}", e.to_string()))
    }

    pub fn prompt() -> anyhow::Result<Self> {
        let name: String = Input::new()
            .with_prompt("enter asset name")
            .allow_empty(false)
            .interact_text()?;

        let display_name: String = Input::new()
            .with_prompt("enter display name")
            .allow_empty(true)
            .interact_text()?;

        let display_name = if !display_name.is_empty() {
            Some(display_name)
        } else {
            None
        };

        let policy_id: String = Input::new()
            .with_prompt("enter policy id")
            .allow_empty(false)
            .interact_text()?;

        let policy_id = if !policy_id.is_empty() {
            Some(policy_id)
        } else {
            None
        };

        let mut extra = Map::new();

        let twitter: String = Input::new()
            .with_prompt("enter twitter url")
            .allow_empty(true)
            .interact_text()?;

        if !twitter.is_empty() {
            extra.insert("twitter".to_string(), Value::String(twitter));
        };

        let website: String = Input::new()
            .with_prompt("enter website url")
            .allow_empty(true)
            .interact_text()?;

        if !website.is_empty() {
            extra.insert("website".to_string(), Value::String(website));
        };

        let copyright: String = Input::new()
            .with_prompt("enter copyright")
            .allow_empty(true)
            .interact_text()?;

        if !copyright.is_empty() {
            extra.insert("copyright".to_string(), Value::String(copyright));
        };

        let amount: usize = Input::new().with_prompt("enter amount").interact_text()?;

        let mut layers = Vec::new();

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("would you like to enter layers?")
            .default(true)
            .interact()?
        {
            loop {
                let layer: String = Input::new().allow_empty(true).interact_text()?;

                if layer.is_empty() {
                    break;
                }

                layers.push(LayerConfig {
                    name: layer,
                    display_name: None,
                    none: None,
                    exclude_if_sets: None,
                    exclude_if_traits: None,
                });
            }
        }

        Ok(Self {
            policy_id,
            name,
            display_name,
            start_at_one: false,
            amount,
            tolerance: 50,
            path: "images".into(),
            sets: None,
            layers,
            extra: Some(extra),
        })
    }
}

pub fn get_global_config_paths() -> anyhow::Result<(PathBuf, PathBuf)> {
    let project = ProjectDirs::from("rs", "", "oink").context("getting global config folder")?;

    let config_dir = project.config_dir().to_path_buf();
    let config_dir_str = config_dir
        .to_str()
        .context("getting global config file path")?;

    let path = [config_dir_str, "global.json"].iter().collect();

    Ok((config_dir, path))
}

pub fn create_global_config_paths() -> anyhow::Result<(PathBuf, PathBuf)> {
    let (global_config_dir, global_config_file) = get_global_config_paths()?;

    if !global_config_dir.exists() {
        fs::create_dir_all(&global_config_dir)?;
    }

    if !global_config_file.exists() {
        fs::write(&global_config_file, b"{}")?;
    }

    Ok((global_config_dir, global_config_file))
}
