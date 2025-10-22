use gtk4::{glib, prelude::*, subclass::prelude::ObjectSubclassIsExt};

use crate::net::{self, Url};

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::subclass::InitializingObject;
    use gtk4::{CompositeTemplate, glib, glib::subclass::Signal, prelude::*, subclass::prelude::*};

    use crate::{
        history::History,
        renderer::{Renderer, paint},
    };

    // "/pentas" is just a prefix. See resources.gresource.xml
    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/pentas/ui/content.ui")]
    pub struct ContentArea {
        /// https://www.w3.org/TR/CSS22/visuren.html#viewport
        #[template_child]
        pub viewport: TemplateChild<gtk4::Viewport>,
        /// https://www.w3.org/TR/CSS22/intro.html#the-canvas
        #[template_child]
        pub canvas: TemplateChild<gtk4::DrawingArea>,

        pub renderer: RefCell<Renderer>,
        pub history: RefCell<History>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContentArea {
        const NAME: &'static str = "ContentArea";
        type Type = super::ContentArea;
        type ParentType = gtk4::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContentArea {
        fn constructed(&self) {
            self.parent_constructed();

            let canvas_obj = self.obj();
            self.canvas.get().set_draw_func(glib::clone!(
                #[strong]
                canvas_obj,
                // Note: Each time the canvas is resized, this closure is called.
                move |_, gfx_ctx, _, _| {
                    let history = canvas_obj.imp().history.borrow();
                    let curr_page = history.get_current().unwrap();

                    let parsed_obj = match &curr_page.parsed_obj {
                        Some(parsed) => parsed.clone(),
                        None => return,
                    };

                    let text_ctx = canvas_obj.imp().canvas.get().create_pango_context();
                    let render_objs_info = canvas_obj
                        .imp()
                        .renderer
                        .borrow()
                        .get_render_objs_info(
                            parsed_obj,
                            &text_ctx,
                            canvas_obj.imp().viewport.width(),
                            canvas_obj.imp().viewport.height(),
                        )
                        .unwrap();

                    let new_page_width = render_objs_info.max_width.round() as i32;
                    let new_page_height = render_objs_info.max_height.round() as i32 + 5;

                    // Adjust the width and the height of the canvas for scrolling.
                    if canvas_obj.imp().canvas.width() != new_page_width {
                        canvas_obj.imp().canvas.set_width_request(new_page_width);
                    }
                    if canvas_obj.imp().canvas.height() != new_page_height {
                        canvas_obj.imp().canvas.set_height_request(new_page_height);
                    }

                    paint(gfx_ctx, &text_ctx, &render_objs_info.objs)
                }
            ));

            // The initial history is a blank page.
            self.history.borrow_mut().add("", "", None);
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("history-updated")
                        .param_types([
                            glib::Type::STRING,
                            glib::Type::STRING,
                            glib::Type::BOOL,
                            glib::Type::BOOL,
                        ])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for ContentArea {}
    impl BoxImpl for ContentArea {}

    impl ContentArea {
        /// Paints all added objects.
        pub fn paint(&self) {
            self.canvas.queue_draw();
        }
    }
}

glib::wrapper! {
    pub struct ContentArea(ObjectSubclass<imp::ContentArea>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl ContentArea {
    pub fn on_toolbar_entry_activate(&self, query: &str) {
        let url = match Url::from_str(query) {
            Ok(url) => url,
            Err(e) => {
                eprintln!("Invalid URL: {}", e);
                return;
            }
        };
        let host_name = match (url.host, url.port) {
            (Some(host), Some(port)) => format!("{}:{}", host, port),
            (Some(host), None) => host.to_string(),
            (None, _) => {
                unreachable!();
            }
        };

        let html = match net::get(query) {
            Ok(res) => res.text(),
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };

        let parsed_obj = self.imp().renderer.borrow().get_parsed_obj(&html).unwrap();
        let title = parsed_obj.clone().title.unwrap_or(host_name.to_string());

        self.imp()
            .history
            .borrow_mut()
            .add(query, &title, Some(parsed_obj));
        self.emit_by_name::<()>(
            "history-updated",
            &[
                &query,
                &title,
                &self.imp().history.borrow().is_rewindable(),
                &self.imp().history.borrow().is_forwardable(),
            ],
        );

        self.imp().paint();
    }

    pub fn on_backward_button_click(&self) {
        if self.imp().history.borrow().is_rewindable() {
            let history = self.imp().history.borrow_mut().rewind().unwrap().clone();
            self.emit_by_name::<()>(
                "history-updated",
                &[
                    &history.query,
                    &history.title,
                    &self.imp().history.borrow().is_rewindable(),
                    &self.imp().history.borrow().is_forwardable(),
                ],
            );
            self.imp().paint();
        }
    }

    pub fn on_forward_button_click(&self) {
        if self.imp().history.borrow().is_forwardable() {
            let history = self.imp().history.borrow_mut().forward().unwrap().clone();
            self.emit_by_name::<()>(
                "history-updated",
                &[
                    &history.query,
                    &history.title,
                    &self.imp().history.borrow().is_rewindable(),
                    &self.imp().history.borrow().is_forwardable(),
                ],
            );
            self.imp().paint();
        }
    }
}
