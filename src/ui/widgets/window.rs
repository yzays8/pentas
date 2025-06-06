use gtk4::{
    Application, gio,
    glib::{self, Object},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::app::TreeTraceLevel;

mod imp {
    use glib::subclass::InitializingObject;
    use gtk4::{
        CompositeTemplate, glib, glib::closure_local, prelude::*,
        style_context_add_provider_for_display, subclass::prelude::*,
    };

    use crate::ui::widgets::{content::ContentArea, toolbar::Toolbar};

    // "/pentas" is just a prefix. See resources.gresource.xml
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
                    content_area.on_toolbar_entry_activate(&query);
                }),
            );

            let content_area = self.content_area.get();
            self.toolbar.connect_closure(
                "backward-button-clicked",
                false,
                closure_local!(move |_: Toolbar, _: String| {
                    content_area.on_backward_button_click();
                }),
            );

            let content_area = self.content_area.get();
            self.toolbar.connect_closure(
                "forward-button-clicked",
                false,
                closure_local!(move |_: Toolbar, _: String| {
                    content_area.on_forward_button_click();
                }),
            );

            let obj = self.obj();
            let toolbar = self.toolbar.get();
            self.content_area.connect_local(
                "history-updated",
                false,
                glib::clone!(
                    #[strong]
                    obj,
                    move |values: &[glib::Value]| {
                        assert_eq!(
                            values.first().unwrap().get::<ContentArea>(),
                            Ok(obj.imp().content_area.get())
                        );
                        let query = values.get(1).unwrap().get::<String>().unwrap();
                        let title = values.get(2).unwrap().get::<String>().unwrap();
                        let is_history_rewindable = values.get(3).unwrap().get::<bool>().unwrap();
                        let is_history_forwardable = values.get(4).unwrap().get::<bool>().unwrap();
                        if title.is_empty() {
                            obj.set_title(Some("pentas"));
                        } else {
                            obj.set_title(Some(format!("pentas - {}", title).as_str()));
                        }
                        toolbar.on_history_update(
                            &query,
                            is_history_rewindable,
                            is_history_forwardable,
                        );
                        None
                    }
                ),
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

    pub fn set_tree_trace_level(&self, tree_trace_level: TreeTraceLevel) {
        self.imp()
            .content_area
            .set_tree_trace_level(tree_trace_level);
    }
}
