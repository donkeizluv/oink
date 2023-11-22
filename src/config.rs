use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use anyhow::Result;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub off_traits: Option<HashSet<String>>,
    pub layers: Vec<LayerConfig>,
    pub extra: Option<Map<String, Value>>,

    #[serde(skip)]
    pub config_name: String,
    #[serde(skip)]
    pub bl: Option<HashMap<String, String>>,
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

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct BlackList {
    pub list: Vec<BlackListLine>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]

pub struct BlackListLine {
    pub trait_name: String,
    pub excludes: Vec<String>,
}

impl AppConfig {
    pub fn load_configs(
        config_folders: &str,
        bl_filename: &str,
        bl_case_sen: bool,
    ) -> Result<Vec<Self>> {
        let config_files = fs::read_dir(config_folders)?;
        let mut configs: Vec<Self> = vec![];

        let bl = match File::open(bl_filename) {
            Ok(mut bl_file) => {
                let mut contents = String::new();
                bl_file.read_to_string(&mut contents)?;

                let parsed_bl: BlackList = serde_json::from_str(&contents)
                    .unwrap_or_else(|_| panic!("unable to parse config file: {}", bl_filename));
                println!(
                    "Found blacklist config of {} lines | trait names is case sensitive: {}",
                    parsed_bl.list.len(),
                    bl_case_sen
                );

                Some(AppConfig::bl(parsed_bl, bl_case_sen)?)
            }
            Err(_) => {
                println!("No blacklist config found");
                None
            }
        };

        for file in config_files {
            let a_file = file.unwrap();
            let file_name = a_file.file_name().to_str().unwrap_or("").to_string();
            let contents = fs::read_to_string(a_file.path())?;

            let mut parsed: Self = serde_json::from_str(&contents)
                .unwrap_or_else(|_| panic!("unable to parse config file: {}", file_name));

            parsed.config_name = file_name.split('.').collect::<Vec<&str>>()[0].to_string();
            // bl
            parsed.bl = bl.clone();

            configs.push(parsed);
        }

        Ok(configs)
    }

    fn bl(bl_config: BlackList, bl_case_sen: bool) -> anyhow::Result<HashMap<String, String>> {
        let mut bl = HashMap::new();

        for line in bl_config.list.iter() {
            for exclude in line.excludes.iter() {
                let (new_exc, new_trait) = if bl_case_sen {
                    (exclude.clone(), line.trait_name.clone())
                } else {
                    (exclude.to_lowercase(), line.trait_name.to_lowercase())
                };

                if bl.insert(new_exc, new_trait).is_some() {
                    panic!("blacklist already contained exclude of [{}], try merging it into excludes of trait_name \"{}\" ", exclude, exclude)
                }
            }
        }

        Ok(bl)
    }

    pub fn is_bl(&self, traits: &HashSet<String>, bl_case_sen: bool) -> bool {
        let case_traits = traits
            .iter()
            .map(|t| {
                if bl_case_sen {
                    t.clone()
                } else {
                    t.to_lowercase()
                }
            })
            .collect::<HashSet<String>>();

        self.bl.as_ref().is_some_and(|bl| {
            case_traits.iter().any(|t| {
                bl.get_key_value(t)
                    .is_some_and(|(_, v)| case_traits.contains(v))
            })
        })
    }
}
