use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use image::RgbaImage;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde_json::{Map, Value};

use oink::{cli::Commands, config::AppConfig, layers::Layers, utils};

const OUTPUT: &str = "output";

type CombDef = (Vec<usize>, String);
type Set = (String, Layers, HashSet<CombDef>);

fn main() -> anyhow::Result<()> {
    let cmds = Commands::new();

    let output = Path::new(OUTPUT);

    match cmds {
        Commands::Clean => utils::clean(output)?,
        Commands::Gen(args) => {
            println!("\n ------- Init Configs ------- \n");
            // Prep folders
            utils::clean(output)?;
            fs::create_dir(output)?;

            let multi_proc_sty = ProgressStyle::with_template(
                "{msg}\n [{elapsed_precise}] {bar:40.green/blue} {pos:>7}/{len:7} \n",
            )?
            .progress_chars("##-");
            let conf_progress = MultiProgress::new();
            let configs = AppConfig::load_configs(&args.config_folder, &args.bl_file)?;

            // for keeping track of uniqueness
            let a_all_dna: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
            // config specific sets
            let a_all_sets: Arc<Mutex<Vec<Set>>> = Arc::new(Mutex::new(vec![]));

            configs
                .into_iter()
                .collect::<Vec<AppConfig>>()
                .par_iter()
                .for_each(|config| {
                    let progress = conf_progress.add(ProgressBar::new(config.amount as u64));
                    progress.set_style(multi_proc_sty.clone());
                    progress.set_message(format!("{} -> Loading", config.config_name));

                    let mut layers = Layers::default();
                    match layers.load(&config.layers, config.path.clone()) {
                        Ok(_) => {}
                        Err(_) => {
                            panic!("unable to load layers")
                        }
                    }

                    let mut fail_count = 0;
                    let mut uniques = HashSet::new();
                    let mut count = 1;

                    while count <= config.amount {
                        match a_all_dna.lock() {
                            Ok(mut all_dna_l) => {
                                let (def, traits, dna) = layers.create_unique(&config.layers);

                                if !config.check_bl(&traits) && all_dna_l.insert(dna.clone()) {
                                    uniques.insert((def, dna));
                                    count += 1;
                                    progress.inc(1);
                                } else {
                                    fail_count += 1;
                                    if fail_count > config.tolerance {
                                        panic!(
                                            "You need more features or traits to generate {}",
                                            config.amount
                                        );
                                    }
                                }
                            }
                            Err(_) => panic!("unable to accquire lock"),
                        }
                    }
                    match a_all_sets.lock() {
                        Ok(mut sets_l) => {
                            sets_l.push((config.config_name.to_string(), layers, uniques));
                        }
                        Err(_) => panic!("unable to lock mutex"),
                    }

                    // create ouput folder
                    fs::create_dir(output.join(&config.config_name))
                        .expect("unable to create config output folder");

                    progress.finish_with_message(format!("{} -> Loaded", config.config_name));
                });
            // conf_progress.clear()?;

            // Generate the images
            println!("\n ------- Generating ------- \n");
            let gen_progresses = MultiProgress::new();

            let sets = Arc::try_unwrap(a_all_sets)
                .map_err(|_| anyhow!("unable to unwrap arc of sets"))?
                .into_inner()
                .map_err(|_| anyhow!("unable to take ownership of mutex"))?;

            sets.into_iter()
                .collect::<Vec<Set>>()
                .par_iter()
                .for_each(|set_data| {
                    let (cfg_name, layers, set) = set_data;

                    let progress = gen_progresses.add(ProgressBar::new(set.len() as u64));
                    progress.set_style(multi_proc_sty.clone());
                    progress.set_message(format!("{} -> Generating NFTs", cfg_name));

                    set.iter()
                        .collect::<Vec<&(Vec<usize>, String)>>()
                        .par_iter()
                        .for_each(|comb| {
                            let (def, dna) = comb;
                            let mut base = RgbaImage::new(layers.width, layers.height);
                            let mut traits_map = Map::new();
                            let cfg_output = output.join(cfg_name);

                            for (index, trait_list) in def.iter().zip(&layers.trait_sets) {
                                let nft_trait = &trait_list[*index];
                                traits_map.insert(
                                    nft_trait.layer.to_owned(),
                                    Value::String(nft_trait.name.to_owned()),
                                );
                                if let Some(image) = &nft_trait.image {
                                    utils::merge(&mut base, image);
                                }
                            }

                            let nft_image_path = cfg_output.join(format!("{}.png", dna));
                            base.save(nft_image_path).expect("failed to create image");

                            // Write attrs
                            let attributes_path = cfg_output.join(format!("{}.json", dna));
                            let attributes = serde_json::to_string_pretty(&traits_map)
                                .expect("failed to create attributes");

                            fs::write(attributes_path, attributes)
                                .expect("failed to create attributes");

                            progress.inc(1);
                        });
                    progress.finish_with_message(format!("{} -> Generation completed", cfg_name));
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

            // gen_progresses.clear()?;
            println!("Finish!");
        }
    }

    Ok(())
}
