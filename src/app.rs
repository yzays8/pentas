use anyhow::{bail, Ok, Result};
use gtk4::prelude::*;
use gtk4::{gio, glib, Application};

use crate::renderer::{display_box_tree, display_style_sheet};
use crate::ui::widgets::window::Window;

const APP_ID: &str = "app.pentas";
pub const DEFAULT_WINDOW_WIDTH: i32 = 1200;
pub const DEFAULT_WINDOW_HEIGHT: i32 = 800;

pub struct Config {
    pub html_path: Option<String>,
    pub css_path: Option<String>,
    pub is_tracing_enabled: bool,
    pub is_rendering_disabled: bool,
}

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
                (Some(html_path), None) => {
                    display_box_tree(html_path.to_owned(), self.config.is_tracing_enabled)?;
                }
                (None, Some(css_path)) => {
                    display_style_sheet(css_path.to_owned())?;
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
        window.set_default_size(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);
        window.present();
    }
}
