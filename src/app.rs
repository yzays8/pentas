use anyhow::{bail, Ok, Result};
use gtk4::prelude::*;
use gtk4::{gio, glib, Application};

use crate::renderer::Renderer;
use crate::ui::widgets::window::Window;

const APP_ID: &str = "app.pentas";
pub const DEFAULT_WINDOW_WIDTH: usize = 1200;
pub const DEFAULT_WINDOW_HEIGHT: usize = 800;

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

        Self::show_ui();

        Ok(())
    }

    fn show_ui() -> glib::ExitCode {
        gio::resources_register_include!("pentas.gresource")
            .expect("Failed to register resources.");
        let app = Application::builder().application_id(APP_ID).build();

        app.connect_activate(Self::build_ui);
        app.run()
    }

    fn build_ui(app: &Application) {
        let window = Window::new(app);
        window.set_title(Some("pentas"));
        window.set_default_size(DEFAULT_WINDOW_WIDTH as i32, DEFAULT_WINDOW_HEIGHT as i32);
        window.present();
    }
}
