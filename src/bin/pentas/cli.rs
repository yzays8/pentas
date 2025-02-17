use std::fmt;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, value_name = "HTML", help = "The HTML file to parse in CLI mode")]
    pub no_window_html: Option<String>,

    #[arg(
        long,
        value_name = "CSS",
        conflicts_with = "no_window_html",
        help = "The CSS file to parse in CLI mode"
    )]
    pub no_window_css: Option<String>,

    #[arg(
        long,
        short,
        default_value_t = VerbosityLevel::Quiet,
        value_name = "LEVEL",
        help = "Set the verbosity level"
    )]
    pub verbose: VerbosityLevel,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
}

impl fmt::Display for VerbosityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        match self {
            VerbosityLevel::Quiet => write!(f, "quiet"),
            VerbosityLevel::Normal => write!(f, "normal"),
            VerbosityLevel::Verbose => write!(f, "verbose"),
        }
    }
}

impl From<VerbosityLevel> for pentas::VerbosityLevel {
    fn from(level: VerbosityLevel) -> Self {
        match level {
            VerbosityLevel::Quiet => pentas::VerbosityLevel::Quiet,
            VerbosityLevel::Normal => pentas::VerbosityLevel::Normal,
            VerbosityLevel::Verbose => pentas::VerbosityLevel::Verbose,
        }
    }
}
