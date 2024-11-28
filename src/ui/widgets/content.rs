use gtk4::glib;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::renderer::{RenderObject, Renderer};

mod imp {
    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk4::prelude::*;
    use gtk4::subclass::prelude::*;
    use gtk4::{cairo, glib, CompositeTemplate};

    use super::RenderObject;
    use crate::ui::history::History;

    // "/pentas" is just a prefix. See resouces.gresource.xml
    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/pentas/ui/content.ui")]
    pub struct ContentArea {
        #[template_child]
        pub drawing_area: TemplateChild<gtk4::DrawingArea>,
        // todo: make propety
        pub objects: RefCell<Vec<RenderObject>>,
        pub history: RefCell<History>,
        // todo: make propety
        pub current_history_index: RefCell<usize>,
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

            let obj = self.obj();
            self.drawing_area.get().set_draw_func(glib::clone!(
                #[weak]
                obj,
                move |_, ctx, _, _| {
                    obj.imp().draw(ctx);
                }
            ));

            // The initial history is an empty page.
            self.history.borrow_mut().add("".to_string(), vec![]);
        }
    }

    impl WidgetImpl for ContentArea {}

    impl BoxImpl for ContentArea {}

    impl ContentArea {
        pub fn add_object(&self, object: RenderObject) {
            self.objects.borrow_mut().push(object);
        }

        pub fn draw(&self, ctx: &cairo::Context) {
            for object in self.objects.borrow().iter() {
                match object {
                    RenderObject::Text {
                        text,
                        x,
                        y,
                        size,
                        color,
                    } => {
                        ctx.set_source_rgb(color.0, color.1, color.2);
                        ctx.set_font_size(*size);
                        let lines = text.split("\n");
                        for (i, line) in lines.enumerate() {
                            ctx.move_to(*x, *y + size * i as f64);
                            let _ = ctx.show_text(line);
                        }
                    }
                    RenderObject::Rectangle {
                        x,
                        y,
                        width,
                        height,
                        color,
                    } => {
                        ctx.set_source_rgb(color.0, color.1, color.2);
                        ctx.rectangle(*x, *y, *width, *height);
                        let _ = ctx.fill();
                    }
                }
            }
        }

        pub fn clear(&self) {
            self.objects.borrow_mut().clear();
            self.objects.borrow_mut().push(RenderObject::Rectangle {
                x: 0.0,
                y: 0.0,
                width: self.drawing_area.width() as f64,
                height: self.drawing_area.height() as f64,
                color: (1.0, 1.0, 1.0),
            });
            self.drawing_area.queue_draw();
            self.objects.borrow_mut().clear();
        }

        pub fn present(&self) {
            self.drawing_area.queue_draw();
        }

        pub fn get_current_history_index(&self) -> usize {
            *self.current_history_index.borrow()
        }

        pub fn set_current_history_index(&self, index: usize) {
            *self.current_history_index.borrow_mut() = index;
        }
    }
}

glib::wrapper! {
    pub struct ContentArea(ObjectSubclass<imp::ContentArea>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl ContentArea {
    pub fn on_toolbar_entry_activated(&self, query: &str) {
        self.imp().clear();

        let renderer = Renderer::new(Some("demo/test_html.html".to_string()), None);
        *self.imp().objects.borrow_mut() = renderer.run().unwrap();

        self.imp()
            .history
            .borrow_mut()
            .add(query.to_string(), self.imp().objects.borrow().clone());

        // This is inaccurate behaviour.
        self.imp()
            .set_current_history_index(self.imp().history.borrow().entries.len() - 1);
        self.imp().present();
    }

    pub fn on_backward_button_clicked(&self) {
        let index = self.imp().get_current_history_index();
        if index > 0 {
            self.imp().set_current_history_index(index - 1);
            self.imp().clear();
            *self.imp().objects.borrow_mut() = self
                .imp()
                .history
                .borrow()
                .get(index - 1)
                .unwrap()
                .objects
                .clone();
            self.imp().present();
        }
    }

    pub fn on_forward_button_clicked(&self) {
        let index = self.imp().get_current_history_index();
        if index < self.imp().history.borrow().entries.len() - 1 {
            self.imp().set_current_history_index(index + 1);
            self.imp().clear();
            *self.imp().objects.borrow_mut() = self
                .imp()
                .history
                .borrow()
                .get(index + 1)
                .unwrap()
                .objects
                .clone();
            self.imp().present();
        }
    }
}
