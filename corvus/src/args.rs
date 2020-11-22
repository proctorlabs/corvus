use super::*;
use std::path::PathBuf;
use structopt::{clap::AppSettings::*, StructOpt};

pub fn parse() -> Result<Options> {
    Ok(Options::from_args())
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "corvus",
    rename_all = "kebab_case",
    author,
    about,
    settings = &[DeriveDisplayOrder, DisableHelpSubcommand, UnifiedHelpMessage]
)]
pub struct Options {
    /// Location of the agent configuration file
    #[structopt(short = "c", long, parse(from_os_str), default_value = "corvus.toml")]
    pub config: PathBuf,

    /// Generate configuration with default values
    #[structopt(short, long)]
    pub generate: bool,

    /// Verbosity level of output
    #[structopt(short = "v", long, parse(from_occurrences))]
    pub verbosity: u64,
}
