mod history;
mod widgets;

use gtk4::prelude::*;
use gtk4::{gio, glib, Application};

use widgets::window::Window;

const GTK_APP_ID: &str = "app.pentas";
pub const DEFAULT_WINDOW_WIDTH: usize = 1200;
pub const DEFAULT_WINDOW_HEIGHT: usize = 800;

pub fn show_ui() -> glib::ExitCode {
    gio::resources_register_include!("pentas.gresource").expect("Failed to register resources.");
    let app = Application::builder().application_id(GTK_APP_ID).build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let window = Window::new(app);
    window.set_title(Some("pentas"));
    window.set_default_size(DEFAULT_WINDOW_WIDTH as i32, DEFAULT_WINDOW_HEIGHT as i32);
    window.present();
}
