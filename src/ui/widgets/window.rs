use glib::Object;
use gtk4::{gio, glib, Application};

mod imp {
    use glib::subclass::InitializingObject;
    use gtk4::glib::closure_local;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;
    use gtk4::{glib, style_context_add_provider_for_display, CompositeTemplate};

    use crate::ui::widgets::content::ContentArea;
    use crate::ui::widgets::toolbar::Toolbar;

    // "/pentas" is just a prefix. See resouces.gresource.xml
    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/pentas/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub toolbar: TemplateChild<Toolbar>,
        #[template_child]
        pub content_area: TemplateChild<ContentArea>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "PentasWindow";
        type Type = super::Window;
        type ParentType = gtk4::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            let provider = gtk4::CssProvider::new();
            provider.load_from_data(
                "window {
                    background-color: #ffffff;
                }",
            );
            style_context_add_provider_for_display(
                &gtk4::gdk::Display::default().unwrap(),
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            let content_area = self.content_area.get();
            self.toolbar.connect_closure(
                "toolbar-entry-activated",
                false,
                closure_local!(move |_: Toolbar, query: String| {
                    content_area.on_toolbar_entry_activated(&query);
                }),
            );
            let content_area = self.content_area.get();
            self.toolbar.connect_closure(
                "backward-button-clicked",
                false,
                closure_local!(move |_: Toolbar, _: String| {
                    content_area.on_backward_button_clicked();
                }),
            );
            let content_area = self.content_area.get();
            self.toolbar.connect_closure(
                "forward-button-clicked",
                false,
                closure_local!(move |_: Toolbar, _: String| {
                    content_area.on_forward_button_clicked();
                }),
            );
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable,
                    gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl Window {
    pub fn new(app: &Application) -> Self {
        Object::builder().property("application", app).build()
    }
}
