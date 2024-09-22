use anyhow::{bail, Ok, Result};

use crate::renderer::{display_box_tree, display_style_sheet};

pub struct Config {
    pub html_path: Option<String>,
    pub css_path: Option<String>,
    pub is_tracing_enabled: bool,
}

pub struct Runner {
    config: Config,
}

impl Runner {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Result<()> {
        match (&self.config.html_path, &self.config.css_path) {
            (Some(html_path), None) => {
                display_box_tree(html_path.to_owned(), self.config.is_tracing_enabled)?;
            }
            (None, Some(css_path)) => {
                display_style_sheet(css_path.to_owned())?;
            }
            _ => bail!("Provide either HTML or CSS file."),
        }

        Ok(())
    }
}
