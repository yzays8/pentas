use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{glib, template_callbacks};

mod imp {
    use std::sync::OnceLock;

    use glib::subclass::InitializingObject;
    use gtk4::glib::subclass::Signal;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;
    use gtk4::{glib, Button, CompositeTemplate, Entry};

    // "/pentas" is just a prefix. See resouces.gresource.xml
    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/pentas/ui/toolbar.ui")]
    pub struct Toolbar {
        #[template_child]
        pub backward_button: TemplateChild<Button>,
        #[template_child]
        pub forward_button: TemplateChild<Button>,
        #[template_child]
        pub entry: TemplateChild<Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Toolbar {
        const NAME: &'static str = "Toolbar";
        type Type = super::Toolbar;
        type ParentType = gtk4::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Toolbar {
        fn constructed(&self) {
            self.parent_constructed();

            let provider = gtk4::CssProvider::new();
            provider.load_from_data(
                "entry {
                    border-radius: 30px;
                    border: 1px solid #ccc;
                    padding-left: 25px;
                    font-size: 16px;
                }",
            );
            self.entry
                .style_context()
                .add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            self.entry.set_hexpand(true);
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("toolbar-entry-activated")
                        .param_types([glib::Type::STRING])
                        .build(),
                    Signal::builder("backward-button-clicked")
                        .param_types([glib::Type::STRING])
                        .build(),
                    Signal::builder("forward-button-clicked")
                        .param_types([glib::Type::STRING])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for Toolbar {}
    impl BoxImpl for Toolbar {}
}

glib::wrapper! {
    pub struct Toolbar(ObjectSubclass<imp::Toolbar>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

#[template_callbacks]
impl Toolbar {
    #[template_callback]
    fn on_backward_button_click(&self) {
        self.emit_by_name::<()>("backward-button-clicked", &[&""]);
    }

    #[template_callback]
    fn on_forward_button_click(&self) {
        self.emit_by_name::<()>("forward-button-clicked", &[&""]);
    }

    #[template_callback]
    fn on_entry_activate(&self) {
        self.emit_by_name::<()>("toolbar-entry-activated", &[&self.imp().entry.text()]);
    }

    pub fn on_history_update(
        &self,
        query: &str,
        is_history_rewindable: bool,
        is_history_forwardable: bool,
    ) {
        self.imp().entry.set_text(query);
        match (
            self.imp().backward_button.is_sensitive(),
            is_history_rewindable,
        ) {
            (true, false) => {
                self.imp().backward_button.set_sensitive(false);
            }
            (false, true) => {
                self.imp().backward_button.set_sensitive(true);
            }
            _ => {}
        }
        match (
            self.imp().forward_button.is_sensitive(),
            is_history_forwardable,
        ) {
            (true, false) => {
                self.imp().forward_button.set_sensitive(false);
            }
            (false, true) => {
                self.imp().forward_button.set_sensitive(true);
            }
            _ => {}
        }
    }
}
