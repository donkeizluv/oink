use clap::Parser;

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Path to the projects config file
    #[clap(short, long, default_value = "oink.json")]
    pub config: String,
}

/// A CLI for managing NFT projects
#[derive(Parser, Debug)]
pub enum Commands {
    /// Clean the output directory
    Clean,
    /// Generate an NFT collection
    Gen(ConfigArgs),
    /// Create a new project
    New { name: String },
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
