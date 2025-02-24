mod painter;
mod widgets;

use gtk4::prelude::*;
use gtk4::{Application, gio, glib};

use crate::app::TreeTraceLevel;
use widgets::window::Window;

const GTK_APP_ID: &str = "app.pentas";
pub const DEFAULT_WINDOW_WIDTH: i32 = 1200;
pub const DEFAULT_WINDOW_HEIGHT: i32 = 800;

pub fn show_ui(tree_trace_level: TreeTraceLevel) -> glib::ExitCode {
    gio::resources_register_include!("pentas.gresource").expect("Failed to register resources.");
    let app = Application::builder().application_id(GTK_APP_ID).build();

    app.connect_activate(move |app| {
        build_ui(app, tree_trace_level);
    });
    // https://github.com/gtk-rs/gtk4-rs/issues/1626
    app.run_with_args::<glib::GString>(&[])
}

fn build_ui(app: &Application, tree_trace_level: TreeTraceLevel) {
    let window = Window::new(app);
    window.set_title(Some("pentas"));
    window.set_default_size(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);
    window.set_tree_trace_level(tree_trace_level);
    window.present();
}
