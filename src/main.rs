use std::{collections::HashSet, fs, path::Path, process};

use anyhow::anyhow;
use image::RgbaImage;
use indicatif::ProgressBar;
use rayon::prelude::*;
use serde_json::{Map, Value};

use oink::{cli::Commands, config::AppConfig, layers::Layers, rarity::Rarity, utils};

const OUTPUT: &str = "output";

fn main() -> anyhow::Result<()> {
    let cmds = Commands::new();

    let output = Path::new(OUTPUT);

    match cmds {
        Commands::Clean => utils::clean(output)?,

        Commands::Gen(args) => {
            let config = AppConfig::new(&args.config)?;
            let progress = ProgressBar::new(config.amount as u64);

            let (layer_sets, unique_sets) = if let Some(sets) = &config.sets {
                let mut layer_sets = Vec::new();

                let mut unique_sets = Vec::new();

                let mut offset = config.amount;

                let sets_total = sets.iter().fold(0, |acc, set| acc + set.amount);

                if sets_total != config.amount {
                    return Err(anyhow!("amount in sets must equal the total amount"));
                }

                for (set_index, set) in sets.iter().enumerate() {
                    let mut layers = Layers::default();

                    layers.load(
                        config.mode,
                        &config.layers,
                        config.path.join(set.name.clone()),
                    )?;

                    let mut fail_count = 0;

                    let mut uniques = HashSet::new();

                    let mut count = 1;

                    while count <= set.amount {
                        let unique = layers.create_unique(&config.layers, &set.name);

                        if uniques.contains(&unique) {
                            fail_count += 1;

                            if fail_count > config.tolerance {
                                println!(
                                    "You need more features or traits to generate {}",
                                    set.amount
                                );

                                process::exit(1);
                            }

                            continue;
                        }

                        uniques.insert(unique);

                        count += 1;
                    }

                    layer_sets.push(layers);

                    offset -= set.amount;

                    unique_sets.push((uniques, set_index, offset));
                }

                (layer_sets, unique_sets)
            } else {
                let mut layers = Layers::default();

                layers.load(config.mode, &config.layers, config.path)?;

                let mut fail_count = 0;

                let mut uniques = HashSet::new();

                let mut count = 1;

                while count <= config.amount {
                    let unique = layers.create_unique(&config.layers, "");

                    if uniques.contains(&unique) {
                        fail_count += 1;

                        if fail_count > config.tolerance {
                            println!(
                                "You need more features or traits to generate {}",
                                config.amount
                            );

                            process::exit(1);
                        }

                        continue;
                    }

                    uniques.insert(unique);

                    count += 1;
                }

                (vec![layers], vec![(uniques, 0, 0)])
            };

            utils::clean(output)?;

            fs::create_dir(output)?;

            // Calculate rarity
            let mut rarity = Rarity::new(config.amount);

            for (uniques, set_index, _) in &unique_sets {
                for unique in uniques {
                    for (index, trait_list) in unique.iter().zip(&layer_sets[*set_index].data) {
                        let nft_trait = &trait_list[*index];

                        rarity.count_trait(&nft_trait.layer, &nft_trait.name);
                    }
                }
            }

            // Generate the images
            unique_sets
                .par_iter()
                .for_each(|(uniques, set_index, offset)| {
                    let layers = &layer_sets[*set_index];

                    uniques
                        .iter()
                        .enumerate()
                        .collect::<Vec<(usize, &Vec<usize>)>>()
                        .par_iter()
                        .for_each(|(mut count, unique)| {
                            if config.start_at_one {
                                count += 1
                            }

                            let mut base = RgbaImage::new(layers.width, layers.height);

                            let mut trait_info = Map::new();

                            for (index, trait_list) in unique.iter().zip(&layers.data) {
                                let nft_trait = &trait_list[*index];

                                trait_info.insert(
                                    nft_trait.layer.to_owned(),
                                    Value::String(nft_trait.name.to_owned()),
                                );

                                if let Some(image) = &nft_trait.image {
                                    utils::merge(&mut base, image);
                                }
                            }

                            let nft_image_path =
                                output.join(format!("{}#{}.png", config.name, count + offset));
                            base.save(nft_image_path).expect("failed to create image");

                            let attributes_path =
                                output.join(format!("{}#{}.json", config.name, count + offset));
                            // let metadata_path = folder_name.join("metadata.json");

                            let attributes = serde_json::to_string_pretty(&trait_info)
                                .expect("failed to create attributes");

                            fs::write(attributes_path, attributes)
                                .expect("failed to create attributes");

                            // let meta = metadata::build_with_attributes(
                            //     trait_info,
                            //     config.policy_id.clone(),
                            //     config.name.clone(),
                            //     config.display_name.as_ref(),
                            //     config.extra.clone(),
                            //     count + offset,
                            // );

                            // fs::write(metadata_path, meta).expect("failed to create metadata");

                            progress.inc(1);
                        });
                });

            let rarity_path = output.join("rarity.json");

            let rarity_data = serde_json::to_string_pretty(&rarity.data)?;

            fs::write(rarity_path, rarity_data)?;

            progress.finish();
        }

        Commands::New { name } => {
            let root_dir = Path::new(&name);

            if root_dir.exists() {
                return Err(anyhow!("{} already exists", root_dir.display()));
            }

            fs::create_dir(root_dir)?;

            let app_config = AppConfig::prompt()?;

            let config_file_path = root_dir.join("oink.json");

            let images_path = root_dir.join(&app_config.path);

            fs::create_dir(&images_path)?;

            for layer in &app_config.layers {
                let layer_path = images_path.join(&layer.name);

                fs::create_dir(&layer_path)?;
            }

            let contents = serde_json::to_string_pretty(&app_config)?;

            fs::write(config_file_path, contents)?;

            println!("\nDone! ✅ To get started:\n");
            println!("cd {}", &name);
            println!("and add some traits into the images/ directory 🚀");
        }
    }

    Ok(())
}
