use anyhow::{Ok, Result};
use gtk4::prelude::WidgetExt;
use gtk4::{self, DrawingArea};

use crate::renderer::{display_css, display_html};
use crate::ui::show_ui;

#[derive(Debug)]
pub struct Config {
    pub no_window_html: Option<String>,
    pub no_window_css: Option<String>,
    pub verbosity: VerbosityLevel,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum VerbosityLevel {
    #[default]
    Quiet,
    Normal,
    Verbose,
}

#[derive(Debug)]
pub struct Runner {
    config: Config,
}

impl Runner {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Result<()> {
        match (&self.config.no_window_html, &self.config.no_window_css) {
            (Some(p), None) => {
                gtk4::init()?;
                display_html(
                    &std::fs::read_to_string(p)?,
                    &DrawingArea::new().pango_context(),
                    self.config.verbosity,
                )?;
            }
            (None, Some(p)) => {
                display_css(&std::fs::read_to_string(p)?)?;
            }
            (None, None) => {
                show_ui(self.config.verbosity);
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}
