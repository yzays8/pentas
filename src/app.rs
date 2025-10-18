use gtk4::{DrawingArea, prelude::WidgetExt};

use crate::{
    error::Result,
    net,
    renderer::Renderer,
    ui::{WindowContext, show_ui},
};

#[derive(Debug)]
pub struct Config {
    pub window_size: (i32, i32),
    pub url: Option<String>,
    pub is_headless: bool,
    pub local_html: Option<String>,
    pub local_css: Option<String>,
    pub dump_level: DumpLevel,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum DumpLevel {
    #[default]
    Off,
    All,
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
        // GUI mode
        if !self.config.is_headless {
            let context = WindowContext {
                window_size: self.config.window_size,
                dump_level: self.config.dump_level,
            };
            show_ui(context);
            return Ok(());
        }

        // Headless mode
        match (
            &self.config.url,
            &self.config.local_html,
            &self.config.local_css,
        ) {
            (Some(url), None, None) => {
                gtk4::init()?;
                let mut renderer = Renderer::with_ctx(&DrawingArea::default().pango_context());
                renderer.set_dump_level(self.config.dump_level);
                let html = net::get(url)?.text();
                renderer.print_box_tree(&html, self.config.window_size)?;
            }
            (None, Some(p), None) => {
                if self.config.is_headless {
                    gtk4::init()?;
                    let mut renderer = Renderer::with_ctx(&DrawingArea::default().pango_context());
                    renderer.set_dump_level(self.config.dump_level);
                    let html = std::fs::read_to_string(p)?;
                    renderer.print_box_tree(&html, self.config.window_size)?;
                }
            }
            (None, None, Some(p)) => {
                let renderer = Renderer::new();
                let css = std::fs::read_to_string(p)?;
                renderer.print_style_sheet(&css)?;
            }
            (None, None, None) => {}
            _ => unreachable!(),
        }

        Ok(())
    }
}
