use std::fmt;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(
        long,
        value_name = "WIDTH,HEIGHT",
        default_values_t = [1200, 800],
        value_delimiter = ',',
        global = true,
        help = "Initial window size"
    )]
    pub window_size: Vec<i32>,

    #[arg(
        long,
        short,
        value_name = "LEVEL",
        default_value_t = DumpLevel::Off,
        global = true,
        help = "Dump level"
    )]
    pub dump: DumpLevel,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Headless(HeadlessArgs),
}

#[derive(Debug, Parser)]
pub struct HeadlessArgs {
    #[arg(help = "Target URL to process")]
    pub url: Option<String>,

    #[arg(
        long,
        value_name = "HTML",
        conflicts_with = "url",
        help = "A local HTML file to parse"
    )]
    pub local_html: Option<String>,

    #[arg(long, value_name = "CSS", conflicts_with_all = ["url", "local_html", "dump"], help = "A local CSS file to parse")]
    pub local_css: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum DumpLevel {
    #[default]
    Off,
    All,
    Debug,
}

impl fmt::Display for DumpLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        match self {
            DumpLevel::Off => write!(f, "off"),
            DumpLevel::All => write!(f, "all"),
            DumpLevel::Debug => write!(f, "debug"),
        }
    }
}

impl From<DumpLevel> for pentas::DumpLevel {
    fn from(level: DumpLevel) -> Self {
        match level {
            DumpLevel::Off => pentas::DumpLevel::Off,
            DumpLevel::All => pentas::DumpLevel::All,
            DumpLevel::Debug => pentas::DumpLevel::Debug,
        }
    }
}
