use anyhow::{bail, Ok, Result};

use crate::renderer::Renderer;
use crate::ui::show_ui;

#[derive(Debug)]
pub struct Config {
    pub html_path: Option<String>,
    pub css_path: Option<String>,
    pub is_tracing_enabled: bool,
    pub is_rendering_disabled: bool,
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
        if self.config.is_rendering_disabled {
            match (&self.config.html_path, &self.config.css_path) {
                (Some(p), None) => {
                    Renderer::display_html(
                        &std::fs::read_to_string(p)?,
                        self.config.is_tracing_enabled,
                    )?;
                }
                (None, Some(p)) => {
                    Renderer::display_css(&std::fs::read_to_string(p)?)?;
                }
                _ => bail!("Provide either HTML or CSS file."),
            }
            return Ok(());
        }

        show_ui();

        Ok(())
    }
}
