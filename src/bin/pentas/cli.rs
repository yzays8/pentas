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
        default_value_t = TreeTraceLevel::Silent,
        value_name = "LEVEL",
        help = "The verbosity level of the tree trace"
    )]
    pub tree_trace: TreeTraceLevel,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TreeTraceLevel {
    #[default]
    Silent,
    Normal,
    Debug,
}

impl fmt::Display for TreeTraceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        match self {
            TreeTraceLevel::Silent => write!(f, "silent"),
            TreeTraceLevel::Normal => write!(f, "normal"),
            TreeTraceLevel::Debug => write!(f, "debug"),
        }
    }
}

impl From<TreeTraceLevel> for pentas::TreeTraceLevel {
    fn from(level: TreeTraceLevel) -> Self {
        match level {
            TreeTraceLevel::Silent => pentas::TreeTraceLevel::Silent,
            TreeTraceLevel::Normal => pentas::TreeTraceLevel::Normal,
            TreeTraceLevel::Debug => pentas::TreeTraceLevel::Debug,
        }
    }
}
