//! Animated fan rotor.
//!
//! An 11-blade impeller in a glowing housing. It has real inertia (it winds
//! up and coasts down instead of jumping), motion-blur ghosts and a swirl
//! disc at high speed, and shifts toward red as the fan nears full speed.
//! Frames are driven by the widget's frame clock, so nothing is redrawn
//! while the page is hidden or the fan is stopped.

use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Instant;

use gtk::cairo::{Antialias, Context, LinearGradient, RadialGradient};
use gtk::glib::ControlFlow;
use gtk::prelude::*;
use gtk::{Align, Box, DrawingArea, Orientation};

const BLADES: usize = 11;

/// Visual spin rates in radians per second. Kept low enough that the blades
/// never appear to spin backwards (wagon-wheel effect) at 60 fps.
const OMEGA_MIN: f64 = 1.2;
const OMEGA_MAX: f64 = 11.0;

struct FanState {
    /// Target fan speed, percent.
    speed: Cell<f64>,
    /// Current spin rate, rad/s.
    omega: Cell<f64>,
    angle: Cell<f64>,
    color: Cell<(f64, f64, f64)>,
    last_frame: Cell<Option<Instant>>,
    dirty: Cell<bool>,
}

#[derive(Clone)]
pub struct FanWidget {
    root: Box,
    drawing: DrawingArea,
    state: Rc<FanState>,
}

/// Blend `base` toward hot red as `speed` climbs past 70%.
pub fn heat_color(base: (f64, f64, f64), speed: f64) -> (f64, f64, f64) {
    let t = ((speed - 70.0) / 30.0).clamp(0.0, 1.0) * 0.65;
    let hot = (1.0, 0.25, 0.18);

    (
        base.0 + (hot.0 - base.0) * t,
        base.1 + (hot.1 - base.1) * t,
        base.2 + (hot.2 - base.2) * t,
    )
}

impl FanWidget {
    pub fn new() -> Self {
        let state = Rc::new(FanState {
            speed: Cell::new(0.0),
            omega: Cell::new(0.0),
            angle: Cell::new(0.0),
            color: Cell::new((0.22, 0.75, 1.0)),
            last_frame: Cell::new(None),
            dirty: Cell::new(true),
        });

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Center)
            .build();

        root.set_size_request(180, 180);

        let drawing = DrawingArea::builder()
            .content_width(180)
            .content_height(180)
            .build();

        drawing.set_size_request(180, 180);

        {
            let state = state.clone();

            drawing.set_draw_func(move |_, cr, width, height| {
                draw_fan(
                    cr,
                    width as f64,
                    height as f64,
                    state.angle.get(),
                    state.omega.get(),
                    state.speed.get(),
                    state.color.get(),
                );
            });
        }

        root.append(&drawing);

        {
            let state = state.clone();

            drawing.add_tick_callback(move |area, _clock| {
                let now = Instant::now();

                let dt = state
                    .last_frame
                    .get()
                    .map(|t| now.duration_since(t).as_secs_f64())
                    .unwrap_or(0.0)
                    .min(0.1);

                state.last_frame.set(Some(now));

                let speed = state.speed.get();

                let target = if speed < 1.0 {
                    0.0
                } else {
                    OMEGA_MIN + (OMEGA_MAX - OMEGA_MIN) * speed / 100.0
                };

                let omega = state.omega.get();

                // Winds up quickly, coasts down slowly.
                let tau = if target > omega { 0.7 } else { 1.6 };

                let mut next = omega + (target - omega) * (1.0 - (-dt / tau).exp());

                if target == 0.0 && next < 0.02 {
                    next = 0.0;
                }

                state.omega.set(next);

                let dirty = state.dirty.replace(false);

                if next > 0.0 || dirty {
                    state
                        .angle
                        .set((state.angle.get() + next * dt) % (PI * 2.0));
                    area.queue_draw();
                }

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
        self.state.speed.set(percent.clamp(0.0, 100.0));
        self.state.dirty.set(true);
    }

    pub fn set_color(&self, r: f64, g: f64, b: f64) {
        self.state.color.set((r, g, b));
        self.state.dirty.set(true);
        self.drawing.queue_draw();
    }
}

/// One swept blade, pointing along +x from the hub.
fn blade_path(cr: &Context) {
    cr.move_to(12.0, -3.0);
    cr.curve_to(24.0, -12.0, 42.0, -11.0, 50.0, 4.0);
    cr.curve_to(41.0, -3.0, 26.0, 3.0, 12.0, 4.0);
    cr.close_path();
}

fn draw_fan(
    cr: &Context,
    w: f64,
    h: f64,
    angle: f64,
    omega: f64,
    speed: f64,
    base: (f64, f64, f64),
) {
    let (r, g, b) = heat_color(base, speed);
    let spin = (omega / OMEGA_MAX).clamp(0.0, 1.0);
    let load = (speed / 100.0).clamp(0.0, 1.0);
    let step = 2.0 * PI / BLADES as f64;

    cr.set_antialias(Antialias::Best);
    cr.save().unwrap();
    cr.translate(w / 2.0, h / 2.0);

    let s = w.min(h) / 140.0;
    cr.scale(s, s);

    // Outer glow, stronger the harder the fan works.
    let glow = RadialGradient::new(0.0, 0.0, 28.0, 0.0, 0.0, 69.0);
    glow.add_color_stop_rgba(0.0, r, g, b, 0.10 + 0.25 * load);
    glow.add_color_stop_rgba(1.0, r, g, b, 0.0);
    cr.set_source(&glow).unwrap();
    cr.arc(0.0, 0.0, 69.0, 0.0, 2.0 * PI);
    cr.fill().unwrap();

    // Housing.
    cr.set_source_rgba(0.06, 0.065, 0.08, 1.0);
    cr.arc(0.0, 0.0, 57.0, 0.0, 2.0 * PI);
    cr.fill().unwrap();

    cr.set_source_rgba(0.20, 0.21, 0.25, 1.0);
    cr.set_line_width(3.0);
    cr.arc(0.0, 0.0, 57.0, 0.0, 2.0 * PI);
    cr.stroke().unwrap();

    cr.set_source_rgba(r, g, b, 0.25 + 0.55 * load);
    cr.set_line_width(1.5);
    cr.arc(0.0, 0.0, 57.0, 0.0, 2.0 * PI);
    cr.stroke().unwrap();

    cr.set_source_rgba(0.035, 0.04, 0.05, 1.0);
    cr.arc(0.0, 0.0, 52.0, 0.0, 2.0 * PI);
    cr.fill().unwrap();

    // Motion-blur ghosts trailing behind the blades.
    let ghosts = (spin * 6.0).round() as usize;
    let trail = 0.05 + 0.05 * spin;

    for k in (1..=ghosts).rev() {
        let alpha = 0.16 * (1.0 - (k as f64 - 1.0) / ghosts as f64);

        for blade in 0..BLADES {
            let a = angle - k as f64 * trail + blade as f64 * step;

            cr.save().unwrap();
            cr.rotate(a);
            blade_path(cr);
            cr.set_source_rgba(r, g, b, alpha);
            cr.fill().unwrap();
            cr.restore().unwrap();
        }
    }

    // Swirl disc: at speed the blades smear into one translucent wheel.
    if spin > 0.55 {
        let amount = 0.22 * (spin - 0.55) / 0.45;

        let swirl = RadialGradient::new(0.0, 0.0, 16.0, 0.0, 0.0, 50.0);
        swirl.add_color_stop_rgba(0.0, r, g, b, 0.0);
        swirl.add_color_stop_rgba(1.0, r, g, b, amount);
        cr.set_source(&swirl).unwrap();
        cr.arc(0.0, 0.0, 50.0, 0.0, 2.0 * PI);
        cr.fill().unwrap();
    }

    // The blades themselves: dark at the hub, bright at the tip.
    let blade_alpha = 1.0 - 0.3 * spin;

    let paint = LinearGradient::new(12.0, 0.0, 50.0, 0.0);
    paint.add_color_stop_rgba(0.0, r * 0.2, g * 0.2, b * 0.2, blade_alpha);
    paint.add_color_stop_rgba(1.0, r, g, b, blade_alpha);

    for blade in 0..BLADES {
        cr.save().unwrap();
        cr.rotate(angle + blade as f64 * step);
        blade_path(cr);
        cr.set_source(&paint).unwrap();
        cr.fill_preserve().unwrap();
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
        cr.set_line_width(0.6);
        cr.stroke().unwrap();
        cr.restore().unwrap();
    }

    // Hub.
    let hub = RadialGradient::new(-3.0, -3.0, 1.0, 0.0, 0.0, 14.0);
    hub.add_color_stop_rgba(0.0, 0.30, 0.32, 0.36, 1.0);
    hub.add_color_stop_rgba(1.0, 0.07, 0.08, 0.10, 1.0);
    cr.set_source(&hub).unwrap();
    cr.arc(0.0, 0.0, 13.0, 0.0, 2.0 * PI);
    cr.fill().unwrap();

    cr.set_source_rgba(r, g, b, 0.9);
    cr.set_line_width(1.5);
    cr.arc(0.0, 0.0, 13.0, 0.0, 2.0 * PI);
    cr.stroke().unwrap();

    cr.set_source_rgba(r, g, b, 1.0);
    cr.arc(0.0, 0.0, 4.5, 0.0, 2.0 * PI);
    cr.fill().unwrap();

    cr.set_source_rgba(0.05, 0.055, 0.07, 1.0);
    cr.arc(0.0, 0.0, 1.8, 0.0, 2.0 * PI);
    cr.fill().unwrap();

    cr.restore().unwrap();
}
