use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;

use gtk::glib::ControlFlow;
use gtk::prelude::*;
use gtk::{Align, Box, DrawingArea, Label, Orientation, Overlay};

use crate::widgets::fan_widget::{heat_color, FanWidget};

const SEGMENTS: usize = 48;

struct GaugeState {
    current: Cell<f64>,
    target: Cell<f64>,
    color: Cell<(f64, f64, f64)>,
}

/// Fan gauge: a segmented LED-style arc around an animated fan, with the
/// speed readout underneath.
pub struct Gauge {
    root: Box,
    state: Rc<GaugeState>,
    drawing: DrawingArea,
    fan: FanWidget,
    value_label: Label,
}

impl Gauge {
    pub fn new(title: &str) -> Self {
        let state = Rc::new(GaugeState {
            current: Cell::new(0.0_f64),
            target: Cell::new(0.0_f64),
            color: Cell::new((0.22, 0.75, 1.0)),
        });

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        root.set_size_request(300, 300);
        root.set_hexpand(true);
        root.set_vexpand(true);

        let drawing = DrawingArea::builder()
            .content_width(300)
            .content_height(300)
            .build();

        drawing.set_size_request(300, 300);
        drawing.set_hexpand(true);
        drawing.set_vexpand(true);

        let value_label = Label::new(Some("0%"));
        value_label.add_css_class("gauge-value");
        value_label.set_halign(Align::Center);

        let name_label = Label::new(Some(title));
        name_label.add_css_class("gauge-name");
        name_label.set_halign(Align::Center);

        let fan = FanWidget::new();
        fan.set_color(0.22, 0.75, 1.0);

        // Readout sits in the opening at the bottom of the arc.
        let center_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        center_box.set_margin_top(200);
        center_box.append(&value_label);
        center_box.append(&name_label);

        let overlay = Overlay::new();
        overlay.set_child(Some(&drawing));
        fan.widget().set_halign(Align::Center);
        fan.widget().set_valign(Align::Center);

        overlay.add_overlay(fan.widget());
        overlay.add_overlay(&center_box);
        overlay.set_halign(Align::Center);
        overlay.set_valign(Align::Center);
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);

        let draw_state = state.clone();
        drawing.set_draw_func(move |_, cr, width, height| {
            let width = width as f64;
            let height = height as f64;
            let size = width.min(height);
            let cx = width / 2.0;
            let cy = height * 0.5;
            let radius = size * 0.38;

            let start = deg_to_rad(-210.0);
            let end = deg_to_rad(30.0);
            let step = (end - start) / SEGMENTS as f64;
            let gap = step * 0.16;

            let percent = draw_state.current.get().clamp(0.0_f64, 100.0_f64);
            let base = draw_state.color.get();

            cr.set_antialias(gtk::cairo::Antialias::Best);

            // Soft glow under the lit part of the arc.
            if percent > 0.5 {
                let (r, g, b) = heat_color(base, percent);

                cr.set_line_cap(gtk::cairo::LineCap::Round);
                cr.set_line_width(24.0);
                cr.set_source_rgba(r, g, b, 0.10);
                cr.arc(
                    cx,
                    cy,
                    radius,
                    start,
                    angle_for_percent(percent, start, end),
                );
                cr.stroke().unwrap();
            }

            // Thin inner rail.
            cr.set_line_cap(gtk::cairo::LineCap::Butt);
            cr.set_line_width(1.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.07);
            cr.arc(cx, cy, radius - 14.0, start, end);
            cr.stroke().unwrap();

            // Segments: dark when off, bright toward the head when lit.
            cr.set_line_width(13.0);

            let lit_fraction = (percent / 100.0).max(0.01);

            for i in 0..SEGMENTS {
                let t = i as f64 / SEGMENTS as f64;
                let a0 = start + step * i as f64 + gap / 2.0;
                let a1 = start + step * (i + 1) as f64 - gap / 2.0;

                let lit = (t + 0.5 / SEGMENTS as f64) * 100.0 <= percent;

                if lit {
                    let (r, g, b) = heat_color(base, t * 100.0);
                    let alpha = 0.55 + 0.45 * (t / lit_fraction).clamp(0.0, 1.0);

                    cr.set_source_rgba(r, g, b, alpha);
                } else {
                    cr.set_source_rgba(0.15, 0.16, 0.19, 1.0);
                }

                cr.arc(cx, cy, radius, a0, a1);
                cr.stroke().unwrap();
            }

            // Bright dot at the head of the arc.
            if percent > 0.5 {
                let a = angle_for_percent(percent, start, end);

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.95);
                cr.arc(
                    cx + radius * a.cos(),
                    cy + radius * a.sin(),
                    2.6,
                    0.0,
                    PI * 2.0,
                );
                cr.fill().unwrap();
            }

            // Scale labels inside the arc.
            let font_size = size * 0.042;

            cr.select_font_face(
                "Sans",
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Bold,
            );
            cr.set_font_size(font_size);
            cr.set_source_rgba(0.55, 0.58, 0.64, 1.0);

            for mark in [0.0_f64, 25.0, 50.0, 75.0, 100.0] {
                let a = angle_for_percent(mark, start, end);
                let label_radius = radius - 22.0;
                let text = format!("{:.0}", mark);

                // Approximate centring: no font metrics needed.
                let half_width = font_size * 0.29 * text.len() as f64;

                cr.move_to(
                    cx + label_radius * a.cos() - half_width,
                    cy + label_radius * a.sin() + font_size * 0.35,
                );
                cr.show_text(&text).unwrap();
            }
        });

        root.append(&overlay);

        // Smooth the readout toward the target. Runs on the frame clock, so
        // it pauses while the page is hidden and costs nothing when settled.
        let label_weak = value_label.downgrade();
        let state_for_tick = state.clone();
        let fan_for_tick = fan.clone();

        drawing.add_tick_callback(move |area, _clock| {
            let Some(value_label) = label_weak.upgrade() else {
                return ControlFlow::Break;
            };

            let current = state_for_tick.current.get();
            let target = state_for_tick.target.get();
            let delta = target - current;

            if delta == 0.0 {
                return ControlFlow::Continue;
            }

            let next = if delta.abs() < 0.05 {
                target
            } else {
                current + delta * 0.16
            };

            state_for_tick.current.set(next);
            fan_for_tick.set_speed(next);

            value_label.set_label(&format!("{:.0}%", next.round()));
            area.queue_draw();

            ControlFlow::Continue
        });

        Self {
            root,
            state,
            drawing,
            fan,
            value_label,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.root
    }

    pub fn set_value(&self, percent: f64) {
        self.state.target.set(percent.clamp(0.0_f64, 100.0_f64));
    }

    pub fn set_color(&self, r: f64, g: f64, b: f64) {
        self.state.color.set((r, g, b));
        self.fan.set_color(r, g, b);
        self.drawing.queue_draw();
    }
}

fn deg_to_rad(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

fn angle_for_percent(percent: f64, start: f64, end: f64) -> f64 {
    start + (end - start) * (percent / 100.0)
}
