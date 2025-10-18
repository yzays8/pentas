mod painter;
mod widgets;

use std::{cell::RefCell, rc::Rc};

use gtk4::{Application, gio, glib, prelude::*};

use crate::{app::DumpLevel, ui::widgets::Window};

const GTK_APP_ID: &str = "app.pentas";

#[derive(Debug, Clone)]
pub struct WindowContext {
    pub window_size: (i32, i32),
    pub dump_level: DumpLevel,
}

pub fn show_ui(ctx: WindowContext) -> glib::ExitCode {
    gio::resources_register_include!("pentas.gresource").expect("Failed to register resources.");
    let app = Application::builder().application_id(GTK_APP_ID).build();
    let ctx = Rc::new(RefCell::new(ctx));
    app.connect_activate(glib::clone!(
        #[strong]
        ctx,
        move |app| {
            build_ui(app, &ctx.borrow());
        }
    ));
    // https://github.com/gtk-rs/gtk4-rs/issues/1626
    app.run_with_args::<glib::GString>(&[])
}

fn build_ui(app: &Application, ctx: &WindowContext) {
    let window = Window::new(app);
    window.set_title(Some("pentas"));
    window.set_default_size(ctx.window_size.0, ctx.window_size.1);
    window.setup_with_context(ctx);
    window.present();
}
