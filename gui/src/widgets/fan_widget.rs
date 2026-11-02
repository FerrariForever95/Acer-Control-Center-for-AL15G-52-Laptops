use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib::{self, ControlFlow};
use gtk::prelude::*;
use gtk::{Align, Box, DrawingArea, Orientation};

struct FanState {
    speed: Cell<f64>,
    angle: Cell<f64>,
    color: Cell<(f64, f64, f64)>,
}

#[derive(Clone)]
pub struct FanWidget {
    root: Box,
    drawing: DrawingArea,
    state: Rc<FanState>,
}

impl FanWidget {
    pub fn new() -> Self {
        let state = Rc::new(FanState {
            speed: Cell::new(0.0),
            angle: Cell::new(0.0),
            color: Cell::new((0.22, 0.75, 1.0)),
        });

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Center)
            .build();
            
        root.set_size_request(150,150);

        let drawing = DrawingArea::builder()
            .content_width(150)
            .content_height(150)
            .build();

        drawing.set_size_request(150, 150);

        {
            let state = state.clone();

            drawing.set_draw_func(move |_, cr, width, height| {
                let w = width as f64;
                let h = height as f64;

                let cx = w / 2.0;
                let cy = h / 2.0;
                let radius = 62.0;

                let angle = state.angle.get();
                let (r, g, b) = state.color.get();

                cr.set_antialias(gtk::cairo::Antialias::Best);

                cr.set_source_rgba(0.15, 0.16, 0.18, 1.0);
                cr.arc(cx, cy, radius + 10.0, 0.0, PI * 2.0);
                cr.fill().unwrap();

                cr.set_source_rgba(0.08, 0.09, 0.10, 1.0);
                cr.arc(cx, cy, radius + 3.0, 0.0, PI * 2.0);
                cr.fill().unwrap();

                cr.set_source_rgba(r * 0.25, g * 0.25, b * 0.25, 0.25);
                cr.arc(cx, cy, radius + 1.0, 0.0, PI * 2.0);
                cr.fill().unwrap();

                for blade in 0..5 {
                    let a = angle + blade as f64 * (2.0 * PI / 5.0);

                    cr.save().unwrap();

                    cr.translate(cx, cy);
                    cr.rotate(a);

                    cr.move_to(0.0, -6.0);
                    cr.curve_to(
                        radius * 0.15,
                        -radius * 0.15,
                        radius * 0.95,
                        -radius * 0.28,
                        radius * 0.72,
                        0.0,
                    );
                    cr.curve_to(
                        radius * 0.95,
                        radius * 0.28,
                        radius * 0.15,
                        radius * 0.15,
                        0.0,
                        6.0,
                    );
                    cr.close_path();

                    cr.set_source_rgba(r, g, b, 0.9);
                    cr.fill().unwrap();

                    cr.restore().unwrap();
                }

                cr.set_source_rgba(0.92, 0.93, 0.95, 1.0);
                cr.arc(cx, cy, 12.0, 0.0, PI * 2.0);
                cr.fill().unwrap();

                cr.set_source_rgba(0.12, 0.13, 0.15, 1.0);
                cr.arc(cx, cy, 5.0, 0.0, PI * 2.0);
                cr.fill().unwrap();
            });
        }

        root.append(&drawing);

        {
            let drawing = drawing.downgrade();
            let state = state.clone();

            glib::timeout_add_local(Duration::from_millis(16), move || {
                let Some(drawing) = drawing.upgrade() else {
                    return ControlFlow::Break;
                };

                let speed = state.speed.get();

                let delta = speed / 100.0 * 0.35;

                state.angle.set((state.angle.get() + delta) % (PI * 2.0));

                drawing.queue_draw();

                ControlFlow::Continue
            });
        }

        Self {
            root,
            drawing,
            state,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.root
    }

    pub fn set_speed(&self, percent: f64) {
        self.state
            .speed
            .set(percent.clamp(0.0, 100.0));
    }

    pub fn set_color(&self, r: f64, g: f64, b: f64) {
        self.state.color.set((r, g, b));
        self.drawing.queue_draw();
    }
}
