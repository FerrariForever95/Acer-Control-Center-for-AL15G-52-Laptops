use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{DrawingArea, GestureDrag};

const T_MIN: f64 = 30.0;
const T_MAX: f64 = 100.0;
const MIN_PERCENT: i32 = 20;

const LEFT: f64 = 46.0;
const RIGHT: f64 = 18.0;
const TOP: f64 = 16.0;
const BOTTOM: f64 = 30.0;
const HIT_RADIUS: f64 = 18.0;

pub const DEFAULT_CURVE: [(u8, u8); 5] =
    [(40, 25), (55, 35), (65, 50), (75, 75), (85, 100)];

struct State {
    points: RefCell<Vec<(u8, u8)>>,
    dragging: Cell<Option<usize>>,
    origin: Cell<(f64, f64)>,
    temp: Cell<Option<i32>>,
    live: Cell<bool>,
    on_commit: RefCell<Option<Rc<dyn Fn(Vec<(u8, u8)>)>>>,
}

/// Drag-to-edit fan curve: temperature on X, fan speed on Y.
pub struct CurveEditor {
    area: DrawingArea,
    state: Rc<State>,
}

fn plot_size(width: f64, height: f64) -> (f64, f64) {
    (width - LEFT - RIGHT, height - TOP - BOTTOM)
}

fn t_to_x(t: f64, pw: f64) -> f64 {
    LEFT + (t - T_MIN) / (T_MAX - T_MIN) * pw
}

fn p_to_y(p: f64, ph: f64) -> f64 {
    TOP + (1.0 - p / 100.0) * ph
}

fn x_to_t(x: f64, pw: f64) -> f64 {
    T_MIN + (x - LEFT) / pw * (T_MAX - T_MIN)
}

fn y_to_p(y: f64, ph: f64) -> f64 {
    (1.0 - (y - TOP) / ph) * 100.0
}

fn curve_value(points: &[(u8, u8)], temp: f64) -> f64 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return MIN_PERCENT as f64;
    };

    if temp <= first.0 as f64 {
        return first.1 as f64;
    }

    if temp >= last.0 as f64 {
        return last.1 as f64;
    }

    for pair in points.windows(2) {
        let (t0, p0) = (pair[0].0 as f64, pair[0].1 as f64);
        let (t1, p1) = (pair[1].0 as f64, pair[1].1 as f64);

        if temp >= t0 && temp <= t1 {
            return p0 + (p1 - p0) * (temp - t0) / (t1 - t0);
        }
    }

    last.1 as f64
}

impl CurveEditor {
    pub fn new() -> Self {
        let state = Rc::new(State {
            points: RefCell::new(DEFAULT_CURVE.to_vec()),
            dragging: Cell::new(None),
            origin: Cell::new((0.0, 0.0)),
            temp: Cell::new(None),
            live: Cell::new(false),
            on_commit: RefCell::new(None),
        });

        let area = DrawingArea::builder()
            .content_width(640)
            .content_height(260)
            .hexpand(true)
            .build();

        let draw_state = state.clone();
        area.set_draw_func(move |_, cr, width, height| {
            let width = width as f64;
            let height = height as f64;
            let (pw, ph) = plot_size(width, height);

            let live = draw_state.live.get();
            let (r, g, b) = if live {
                (0.23, 0.59, 1.0)
            } else {
                (0.50, 0.55, 0.63)
            };

            cr.set_antialias(gtk::cairo::Antialias::Best);
            cr.select_font_face(
                "Sans",
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Normal,
            );
            cr.set_font_size(11.0);

            // Grid + axis labels
            cr.set_line_width(1.0);

            for p in [0.0, 25.0, 50.0, 75.0, 100.0] {
                let y = p_to_y(p, ph);

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
                cr.move_to(LEFT, y);
                cr.line_to(LEFT + pw, y);
                cr.stroke().unwrap();

                cr.set_source_rgba(0.65, 0.67, 0.70, 1.0);
                cr.move_to(8.0, y + 4.0);
                cr.show_text(&format!("{:.0}%", p)).unwrap();
            }

            let mut t = T_MIN;
            while t <= T_MAX + 0.1 {
                let x = t_to_x(t, pw);

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.05);
                cr.move_to(x, TOP);
                cr.line_to(x, TOP + ph);
                cr.stroke().unwrap();

                cr.set_source_rgba(0.65, 0.67, 0.70, 1.0);
                cr.move_to(x - 12.0, height - 8.0);
                cr.show_text(&format!("{:.0}°C", t)).unwrap();

                t += 10.0;
            }

            let points = draw_state.points.borrow().clone();

            if points.len() < 2 {
                return;
            }

            // Curve: flat before the first point and after the last.
            let first = points[0];
            let last = points[points.len() - 1];

            cr.move_to(t_to_x(T_MIN, pw), p_to_y(first.1 as f64, ph));

            for p in &points {
                cr.line_to(t_to_x(p.0 as f64, pw), p_to_y(p.1 as f64, ph));
            }

            cr.line_to(t_to_x(T_MAX, pw), p_to_y(last.1 as f64, ph));

            let path = cr.copy_path().unwrap();

            cr.line_to(t_to_x(T_MAX, pw), p_to_y(0.0, ph));
            cr.line_to(t_to_x(T_MIN, pw), p_to_y(0.0, ph));
            cr.close_path();
            cr.set_source_rgba(r, g, b, 0.12);
            cr.fill().unwrap();

            cr.new_path();
            cr.append_path(&path);
            cr.set_source_rgba(r, g, b, 1.0);
            cr.set_line_width(2.5);
            cr.set_line_join(gtk::cairo::LineJoin::Round);
            cr.stroke().unwrap();

            // Current temperature marker
            if let Some(temp) = draw_state.temp.get() {
                let temp = temp as f64;

                if (T_MIN..=T_MAX).contains(&temp) {
                    let x = t_to_x(temp, pw);
                    let value = curve_value(&points, temp);
                    let y = p_to_y(value, ph);

                    cr.set_source_rgba(0.98, 0.57, 0.24, 1.0);
                    cr.set_line_width(1.5);
                    cr.set_dash(&[4.0, 4.0], 0.0);
                    cr.move_to(x, TOP);
                    cr.line_to(x, TOP + ph);
                    cr.stroke().unwrap();
                    cr.set_dash(&[], 0.0);

                    cr.arc(x, y, 4.5, 0.0, PI * 2.0);
                    cr.fill().unwrap();

                    let label = format!(
                        "Now: {:.0}°C → {:.0}%",
                        temp,
                        value.round()
                    );
                    let lx = if x > width * 0.65 { x - 118.0 } else { x + 8.0 };
                    cr.move_to(lx, TOP + 14.0);
                    cr.show_text(&label).unwrap();
                }
            }

            // Handles
            let active = draw_state.dragging.get();

            for (i, p) in points.iter().enumerate() {
                let x = t_to_x(p.0 as f64, pw);
                let y = p_to_y(p.1 as f64, ph);
                let is_active = active == Some(i);

                cr.arc(x, y, if is_active { 8.0 } else { 6.0 }, 0.0, PI * 2.0);
                cr.set_source_rgba(0.08, 0.09, 0.11, 1.0);
                cr.fill_preserve().unwrap();
                cr.set_source_rgba(r, g, b, 1.0);
                cr.set_line_width(2.5);
                cr.stroke().unwrap();

                if is_active {
                    cr.set_source_rgba(0.92, 0.92, 0.94, 1.0);
                    cr.move_to(x - 28.0, y - 14.0);
                    cr.show_text(&format!("{}°C · {}%", p.0, p.1)).unwrap();
                }
            }
        });

        let drag = GestureDrag::new();

        {
            let state = state.clone();
            let area = area.clone();

            drag.connect_drag_begin(move |_, x, y| {
                let (pw, ph) = plot_size(area.width() as f64, area.height() as f64);

                let mut best: Option<(usize, f64)> = None;

                for (i, p) in state.points.borrow().iter().enumerate() {
                    let px = t_to_x(p.0 as f64, pw);
                    let py = p_to_y(p.1 as f64, ph);
                    let dist = ((px - x).powi(2) + (py - y).powi(2)).sqrt();

                    if dist <= HIT_RADIUS
                        && best.map(|(_, d)| dist < d).unwrap_or(true)
                    {
                        best = Some((i, dist));
                        state.origin.set((px, py));
                    }
                }

                state.dragging.set(best.map(|(i, _)| i));
                area.queue_draw();
            });
        }

        {
            let state = state.clone();
            let area = area.clone();

            drag.connect_drag_update(move |_, dx, dy| {
                let Some(i) = state.dragging.get() else { return };

                let (pw, ph) = plot_size(area.width() as f64, area.height() as f64);
                let (ox, oy) = state.origin.get();

                let t = x_to_t(ox + dx, pw).round() as i32;
                let p = y_to_p(oy + dy, ph).round() as i32;

                {
                    let mut pts = state.points.borrow_mut();
                    let n = pts.len();

                    let lo_t = if i == 0 { T_MIN as i32 } else { pts[i - 1].0 as i32 + 1 };
                    let hi_t = if i + 1 == n { T_MAX as i32 } else { pts[i + 1].0 as i32 - 1 };
                    let lo_p = if i == 0 { MIN_PERCENT } else { pts[i - 1].1 as i32 };
                    let hi_p = if i + 1 == n { 100 } else { pts[i + 1].1 as i32 };

                    let t = t.clamp(lo_t, hi_t.max(lo_t));
                    let p = p.clamp(lo_p, hi_p.max(lo_p));

                    pts[i] = (t as u8, p as u8);
                }

                area.queue_draw();
            });
        }

        {
            let state = state.clone();
            let area = area.clone();

            drag.connect_drag_end(move |_, _, _| {
                if state.dragging.get().is_none() {
                    return;
                }

                state.dragging.set(None);
                area.queue_draw();

                let callback = state.on_commit.borrow().clone();
                let points = state.points.borrow().clone();

                if let Some(cb) = callback {
                    cb(points);
                }
            });
        }

        area.add_controller(drag);

        Self { area, state }
    }

    pub fn widget(&self) -> &DrawingArea {
        &self.area
    }

    pub fn points(&self) -> Vec<(u8, u8)> {
        self.state.points.borrow().clone()
    }

    /// Replace the curve (ignored while the user is dragging).
    pub fn set_points(&self, points: Vec<(u8, u8)>) {
        if self.state.dragging.get().is_some() || points.len() < 2 {
            return;
        }

        *self.state.points.borrow_mut() = points;
        self.area.queue_draw();
    }

    pub fn reset(&self) {
        *self.state.points.borrow_mut() = DEFAULT_CURVE.to_vec();
        self.area.queue_draw();
    }

    pub fn set_temperature(&self, temp: Option<i32>) {
        self.state.temp.set(temp);
        self.area.queue_draw();
    }

    /// Live = the daemon is actually following this curve right now.
    pub fn set_live(&self, live: bool) {
        self.state.live.set(live);
        self.area.queue_draw();
    }

    /// Called once when the user releases a dragged point.
    pub fn connect_commit<F: Fn(Vec<(u8, u8)>) + 'static>(&self, f: F) {
        *self.state.on_commit.borrow_mut() = Some(Rc::new(f));
    }
}
