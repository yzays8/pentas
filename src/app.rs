use anyhow::{Ok, Result};

use crate::renderer::Renderer;
use crate::ui::show_ui;

#[derive(Debug)]
pub struct Config {
    pub no_window_html: Option<String>,
    pub no_window_css: Option<String>,
    pub is_tracing_enabled: bool,
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
                Renderer::display_html(
                    &std::fs::read_to_string(p)?,
                    self.config.is_tracing_enabled,
                )?;
            }
            (None, Some(p)) => {
                Renderer::display_css(&std::fs::read_to_string(p)?)?;
            }
            (None, None) => {
                show_ui(self.config.is_tracing_enabled);
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}
