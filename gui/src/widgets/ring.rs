use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Align, Box, DrawingArea, Label, Orientation, Overlay};

struct RingState {
    fraction: Cell<f64>,
    color: Cell<(f64, f64, f64)>,
}

/// A progress ring with a title, a big value and a caption in the middle.
pub struct Ring {
    root: Overlay,
    state: Rc<RingState>,
    drawing: DrawingArea,
    value: Label,
    caption: Label,
}

impl Ring {
    pub fn new(title: &str, size: i32) -> Self {
        let state = Rc::new(RingState {
            fraction: Cell::new(0.0),
            color: Cell::new((0.29, 0.87, 0.50)),
        });

        let drawing = DrawingArea::builder()
            .content_width(size)
            .content_height(size)
            .build();

        let draw_state = state.clone();
        drawing.set_draw_func(move |_, cr, width, height| {
            let width = width as f64;
            let height = height as f64;
            let cx = width / 2.0;
            let cy = height / 2.0;
            let radius = width.min(height) * 0.40;
            let start = -PI / 2.0;

            cr.set_antialias(gtk::cairo::Antialias::Best);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            cr.set_line_width(14.0);

            cr.set_source_rgba(0.18, 0.19, 0.21, 1.0);
            cr.arc(cx, cy, radius, 0.0, PI * 2.0);
            cr.stroke().unwrap();

            let fraction = draw_state.fraction.get().clamp(0.0, 1.0);

            if fraction > 0.0 {
                let (r, g, b) = draw_state.color.get();
                cr.set_source_rgba(r, g, b, 1.0);
                cr.arc(cx, cy, radius, start, start + PI * 2.0 * fraction);
                cr.stroke().unwrap();
            }
        });

        let title_label = Label::new(Some(title));
        title_label.add_css_class("fan-card-title");

        let value = Label::new(Some("--"));
        value.add_css_class("title-1");

        let caption = Label::new(None);
        caption.add_css_class("dim-label");

        let center = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        center.append(&title_label);
        center.append(&value);
        center.append(&caption);

        let root = Overlay::new();
        root.set_child(Some(&drawing));
        root.add_overlay(&center);
        root.set_halign(Align::Center);
        root.set_valign(Align::Center);
        root.set_size_request(size, size);

        Self {
            root,
            state,
            drawing,
            value,
            caption,
        }
    }

    pub fn widget(&self) -> &Overlay {
        &self.root
    }

    pub fn set(&self, fraction: f64, value: &str, caption: &str) {
        self.state.fraction.set(fraction);
        self.value.set_label(value);
        self.caption.set_label(caption);
        self.drawing.queue_draw();
    }

    pub fn set_color(&self, r: f64, g: f64, b: f64) {
        self.state.color.set((r, g, b));
        self.drawing.queue_draw();
    }
}
