use clap::Parser;

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Path to the projects config file
    #[clap(short, long, default_value = "configs")]
    pub config_folder: String,

    /// Path to blacklist config file
    #[clap(short, long, default_value = "blacklist.json")]
    pub bl_file: String,

    /// Blacklist name case sentivity, default is false
    #[clap(long, default_value = "false")]
    pub bl_case_sen: bool,
}

/// CLI for generating jpegs
#[derive(Parser, Debug)]
pub enum Commands {
    /// Clean the output directory
    Clean,
    /// Generate an NFT collection
    Gen(ConfigArgs),
}

impl Default for Commands {
    fn default() -> Self {
        Self::new()
    }
}

impl Commands {
    pub fn new() -> Self {
        Commands::parse()
    }
}
