use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    fs,
    hash::{Hash, Hasher},
    path::Path,
    process,
};

use image::RgbaImage;
use indicatif::ProgressBar;
use rayon::prelude::*;
use serde_json::{Map, Value};

use oink::{
    cli::Commands,
    config::AppConfig,
    layers::{Layers, TraitSet},
    rarity::Rarity,
    utils,
};

const OUTPUT: &str = "output";

fn main() -> anyhow::Result<()> {
    let cmds = Commands::new();

    let output = Path::new(OUTPUT);

    match cmds {
        Commands::Clean => utils::clean(output)?,
        Commands::Gen(args) => {
            type Set = (String, Layers, HashSet<Vec<usize>>);
            let configs = AppConfig::load_configs(&args.config_folder)?;

            // for keeping track of uniqueness
            let mut all_dna: Vec<HashSet<String>> = vec![];
            // config specific sets
            let mut sets: Vec<Set> = vec![];

            let config_count = configs.len();
            println!("Loading {:?} config(s)", config_count);
            let conf_progress = ProgressBar::new(config_count as u64);
            for config in configs {
                let mut layers = Layers::default();
                layers.load(&config.layers, config.path)?;

                let mut fail_count = 0;
                let mut uniques = HashSet::new();
                let mut count = 1;

                while count <= config.amount {
                    let (unique, dna) = layers.create_unique(&config.layers);
                    if all_dna
                        .iter()
                        .any(|s| s.symmetric_difference(&dna).count() == 0)
                    {
                        fail_count += 1;
                        if fail_count > config.tolerance {
                            println!(
                                "You need more features or traits to generate {}",
                                config.amount
                            );

                            process::exit(1);
                        }
                        continue;
                    } else {
                        uniques.insert(unique);
                        all_dna.push(dna);
                        count += 1;
                    }
                }
                let config_name = config.config_name.split(".").collect::<Vec<&str>>();
                sets.push((config_name[0].to_string(), layers, uniques));

                conf_progress.inc(1);
            }
            conf_progress.finish();

            // Prep folders
            utils::clean(output)?;
            fs::create_dir(output)?;
            for set in &sets {
                fs::create_dir(output.join(set.0.to_owned()))?;
            }

            println!("Generating...");
            let gen_progress = ProgressBar::new(all_dna.len() as u64);
            // Generate the images
            sets.into_iter()
                .collect::<Vec<Set>>()
                .par_iter()
                .for_each(|set| {
                    let (cfg_name, layers, set) = set;

                    set.into_iter()
                        .collect::<Vec<&Vec<usize>>>()
                        .par_iter()
                        .for_each(|dna| {
                            // TODO correctly hash the dna traits
                            let mut base = RgbaImage::new(layers.width, layers.height);
                            let mut trait_info = vec![];
                            let cfg_output = output.join(cfg_name);

                            for (index, trait_list) in dna.iter().zip(&layers.trait_sets) {
                                let nft_trait = &trait_list[*index];

                                trait_info.push((
                                    nft_trait.layer.to_owned(),
                                    format!("{}-{}", nft_trait.layer, nft_trait.name),
                                ));

                                if let Some(image) = &nft_trait.image {
                                    utils::merge(&mut base, image);
                                }
                            }

                            // sort to make sure the hash func works correctly
                            trait_info.sort();
                            let mut hasher = DefaultHasher::new();
                            trait_info.hash(&mut hasher);

                            let nft_image_path =
                                cfg_output.join(format!("{:x}.png", hasher.finish()));
                            base.save(nft_image_path).expect("failed to create image");

                            // Write attrs

                            let trait_map = {
                                let mut map = Map::new();
                                for info in trait_info {
                                    map.insert(info.0.to_owned(), Value::String(info.1.to_owned()));
                                }
                                map
                            };
                            let attributes_path =
                                cfg_output.join(format!("{:x}.json", hasher.finish()));
                            let attributes = serde_json::to_string_pretty(&trait_map)
                                .expect("failed to create attributes");

                            fs::write(attributes_path, attributes)
                                .expect("failed to create attributes");

                            gen_progress.inc(1);
                        });
                });

            // Calculate rarity
            // let mut rarity = Rarity::new(config.amount);
            // for (uniques, set_index, _) in &unique_sets {
            //     for unique in uniques {
            //         for (index, trait_list) in unique.iter().zip(&layer_sets[*set_index].data) {
            //             let nft_trait = &trait_list[*index];

            //             rarity.count_trait(&nft_trait.layer, &nft_trait.name);
            //         }
            //     }
            // }

            // let rarity_path = output.join("rarity.json");
            // let rarity_data = serde_json::to_string_pretty(&rarity.data)?;
            // fs::write(rarity_path, rarity_data)?;

            gen_progress.finish();
            println!("Finish!");
        }
    }

    Ok(())
}
