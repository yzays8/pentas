use anyhow::{Ok, Result};
use gtk4::{DrawingArea, prelude::WidgetExt};

use crate::{renderer::Renderer, ui::show_ui};

#[derive(Debug)]
pub struct Config {
    pub no_window_html: Option<String>,
    pub no_window_css: Option<String>,
    pub tree_trace_level: TreeTraceLevel,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TreeTraceLevel {
    #[default]
    Silent,
    Normal,
    Debug,
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
                let mut renderer = Renderer::with_ctx(&DrawingArea::default().pango_context());
                renderer.set_trace_level(self.config.tree_trace_level);
                renderer.print_box_tree(&std::fs::read_to_string(p)?, p)?;
            }
            (None, Some(p)) => {
                let renderer = Renderer::new();
                renderer.print_style_sheet(&std::fs::read_to_string(p)?)?;
            }
            (None, None) => {
                show_ui(self.config.tree_trace_level);
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}
