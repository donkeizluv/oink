use std::{collections::HashSet, path::PathBuf};

use anyhow::{anyhow, Context};
use image::{DynamicImage, GenericImageView};
use rand::Rng;
use sha3::{Digest, Keccak256};

use crate::config::LayerConfig;

#[derive(Debug, Clone)]
pub struct Trait {
    pub layer: String,
    pub name: String,
    pub weight: u32,
    pub image: Option<DynamicImage>,
}

pub type TraitSet = Vec<Trait>;

#[derive(Default)]
pub struct Layers {
    pub trait_sets: Vec<TraitSet>,
    pub width: u32,
    pub height: u32,
}

const DEFAULT_WEIGHT: u32 = 50;

impl Layers {
    pub fn load(&mut self, layers: &[LayerConfig], path: PathBuf) -> anyhow::Result<()> {
        let mut trait_sets = vec![];
        let mut trait_names = HashSet::new();

        let layer_paths = layers
            .iter()
            .map(|layer| (layer, path.join(layer.name.clone())))
            .filter(|(_, path)| path.is_dir());

        for (layer_config, layer_path) in layer_paths {
            let mut trait_set: TraitSet = vec![];

            let layer_name = layer_config
                .display_name
                .as_ref()
                .unwrap_or(&layer_config.name)
                .clone();

            let trait_paths = layer_path
                .read_dir()
                .with_context(|| format!("{} is not a folder", layer_path.display()))?
                .map(|dir| dir.unwrap().path())
                .filter(|path| path.is_file())
                .filter(|path| matches!(path.extension(), Some(ext) if ext == "png"));

            for trait_path in trait_paths {
                let image = image::open(&trait_path)
                    .with_context(|| format!("failed to load image {}", trait_path.display()))?;

                let (width, height) = image.dimensions();

                if self.width == 0 && self.height == 0 {
                    self.width = width;
                    self.height = height;
                }

                let file_name = trait_path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                if file_name.contains('#') {
                    let parts: Vec<&str> = file_name.split('#').collect();

                    let name = parts[0];

                    let weight = parts[1]
                        .parse()
                        .with_context(|| format!("{} is not a parsable number", parts[1]))?;

                    trait_set.push(Trait {
                        layer: layer_name.clone(),
                        name: name.to_owned(),
                        image: Some(image),
                        weight,
                    })
                } else {
                    trait_set.push(Trait {
                        layer: layer_name.clone(),
                        name: file_name.clone(),
                        image: Some(image),
                        weight: DEFAULT_WEIGHT,
                    })
                }

                if !trait_names.insert(file_name.clone()) {
                    return Err(anyhow!(format!("Duplicated trait name of {}", file_name)));
                }
            }

            let mut already_has_none = false;

            if let Some(weight) = layer_config.none {
                trait_set.push(Trait {
                    layer: layer_name.clone(),
                    name: "None".to_string(),
                    weight,
                    image: None,
                });

                already_has_none = true;
            }

            if !already_has_none && (layer_config.exclude_if_traits.is_some()) {
                trait_set.push(Trait {
                    layer: layer_name,
                    name: "None".to_string(),
                    weight: 0,
                    image: None,
                });
            }

            trait_sets.push(trait_set);
        }

        self.trait_sets = trait_sets;

        Ok(())
    }

    pub fn create_unique(&self, layers: &[LayerConfig]) -> (Vec<usize>, String) {
        let mut random = Vec::new();
        let mut rng = rand::thread_rng();
        let mut trait_names = HashSet::new();

        for (trait_list, layer_config) in self.trait_sets.iter().zip(layers) {
            if let Some(exclude_if_traits) = &layer_config.exclude_if_traits {
                if exclude_if_traits.iter().any(|if_trait| {
                    // search through previously applied layers for a match
                    random.iter().enumerate().any(|(bucket, index)| {
                        let bucket = &self.trait_sets[bucket];
                        let nft_trait: &Trait = &bucket[*index];
                        // if filter only contains layer exclude that layer
                        if if_trait.traits.is_empty() {
                            return nft_trait.layer == if_trait.layer;
                        }
                        if if_trait.layer.is_empty() {
                            return if_trait.traits.iter().any(|t| t == &nft_trait.name);
                        }

                        // if filter contains both, both must be match
                        nft_trait.layer == if_trait.layer
                            && if_trait.traits.iter().any(|t| t == &nft_trait.name)
                    })
                }) {
                    random.push(trait_list.len() - 1);

                    continue;
                };
            }

            let total_weight = trait_list.iter().fold(0, |acc, elem| acc + elem.weight);
            let random_num = rng.gen_range(0.0..1.0);
            let mut n = (random_num * total_weight as f64).floor();

            for (index, elem) in trait_list.iter().enumerate() {
                n -= elem.weight as f64;

                if n < 0.0 {
                    random.push(index);
                    trait_names.insert(format!("{}-{}", layer_config.name, elem.name));
                    break;
                }
            }
        }

        (random, Layers::hash_dna(trait_names))
    }

    fn hash_dna(traits: HashSet<String>) -> String {
        let mut sorted: Vec<Vec<u8>> = traits
            .into_iter()
            .map(|t| t.as_bytes().to_owned())
            .collect();
        sorted.sort();

        let mut hasher = Keccak256::new();
        hasher.update(sorted.into_iter().flatten().collect::<Vec<u8>>());

        format!("{:x}", hasher.finalize())
    }
}
