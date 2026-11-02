use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use gtk::prelude::*;
use gtk::DrawingArea;

const LEFT: f64 = 46.0;
const RIGHT: f64 = 18.0;
const TOP: f64 = 22.0;
const BOTTOM: f64 = 30.0;

/// Samples further apart than this are drawn as a gap (app/daemon was off).
const GAP_SECS: u64 = 5 * 60;

type Sample = (u64, u8, bool);

struct State {
    points: RefCell<Vec<Sample>>,
    window_secs: Cell<u64>,
}

/// Line chart of battery percentage over time. Green while charging,
/// blue while on battery.
pub struct BatteryChart {
    area: DrawingArea,
    state: Rc<State>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn span_label(secs: u64) -> String {
    if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 48 * 3600 {
        let hours = secs as f64 / 3600.0;

        if hours.fract() == 0.0 {
            format!("{:.0}h", hours)
        } else {
            format!("{:.1}h", hours)
        }
    } else {
        format!("{:.0}d", secs as f64 / 86400.0)
    }
}

fn color(charging: bool) -> (f64, f64, f64) {
    if charging {
        (0.29, 0.87, 0.50)
    } else {
        (0.23, 0.59, 1.0)
    }
}

impl BatteryChart {
    pub fn new() -> Self {
        let state = Rc::new(State {
            points: RefCell::new(Vec::new()),
            window_secs: Cell::new(6 * 3600),
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
            let pw = width - LEFT - RIGHT;
            let ph = height - TOP - BOTTOM;

            let window = draw_state.window_secs.get();
            let now = now_secs();
            let start = now.saturating_sub(window);

            let x_of = |ts: u64| {
                LEFT + (ts.saturating_sub(start) as f64 / window as f64) * pw
            };
            let y_of = |pct: f64| TOP + (1.0 - pct / 100.0) * ph;

            cr.set_antialias(gtk::cairo::Antialias::Best);
            cr.select_font_face(
                "Sans",
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Normal,
            );
            cr.set_font_size(11.0);

            // Grid + percentage labels
            cr.set_line_width(1.0);

            for p in [0.0, 25.0, 50.0, 75.0, 100.0] {
                let y = y_of(p);

                cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
                cr.move_to(LEFT, y);
                cr.line_to(LEFT + pw, y);
                cr.stroke().unwrap();

                cr.set_source_rgba(0.65, 0.67, 0.70, 1.0);
                cr.move_to(8.0, y + 4.0);
                cr.show_text(&format!("{:.0}%", p)).unwrap();
            }

            // Time labels
            cr.set_source_rgba(0.65, 0.67, 0.70, 1.0);

            let labels = [
                (0.0, format!("-{}", span_label(window))),
                (0.5, format!("-{}", span_label(window / 2))),
                (1.0, "Now".to_string()),
            ];

            for (frac, text) in labels {
                let x = LEFT + frac * pw;
                let offset = if frac == 0.0 {
                    0.0
                } else if frac == 1.0 {
                    -22.0
                } else {
                    -12.0
                };

                cr.move_to(x + offset, height - 8.0);
                cr.show_text(&text).unwrap();
            }

            // Legend
            for (i, (label, charging)) in
                [("Charging", true), ("On battery", false)].iter().enumerate()
            {
                let x = LEFT + pw - 160.0 + i as f64 * 84.0;
                let (r, g, b) = color(*charging);

                cr.set_source_rgba(r, g, b, 1.0);
                cr.arc(x, 10.0, 4.0, 0.0, PI * 2.0);
                cr.fill().unwrap();

                cr.set_source_rgba(0.65, 0.67, 0.70, 1.0);
                cr.move_to(x + 9.0, 14.0);
                cr.show_text(label).unwrap();
            }

            let points: Vec<Sample> = draw_state
                .points
                .borrow()
                .iter()
                .copied()
                .filter(|p| p.0 >= start)
                .collect();

            if points.is_empty() {
                cr.set_source_rgba(0.65, 0.67, 0.70, 1.0);
                cr.move_to(LEFT + pw / 2.0 - 60.0, TOP + ph / 2.0);
                cr.show_text("Collecting data...").unwrap();
                return;
            }

            // Split into runs separated by gaps.
            let mut runs: Vec<Vec<Sample>> = Vec::new();

            for p in &points {
                match runs.last_mut() {
                    Some(run)
                        if p.0.saturating_sub(run.last().unwrap().0) <= GAP_SECS =>
                    {
                        run.push(*p)
                    }
                    _ => runs.push(vec![*p]),
                }
            }

            cr.set_line_width(2.5);
            cr.set_line_join(gtk::cairo::LineJoin::Round);
            cr.set_line_cap(gtk::cairo::LineCap::Round);

            for run in &runs {
                // Soft fill under the line.
                if run.len() > 1 {
                    let (r, g, b) = color(run.last().unwrap().2);

                    cr.move_to(x_of(run[0].0), y_of(0.0));

                    for p in run {
                        cr.line_to(x_of(p.0), y_of(p.1 as f64));
                    }

                    cr.line_to(x_of(run.last().unwrap().0), y_of(0.0));
                    cr.close_path();
                    cr.set_source_rgba(r, g, b, 0.10);
                    cr.fill().unwrap();
                }

                // The line, coloured by charging state per segment.
                for pair in run.windows(2) {
                    let (r, g, b) = color(pair[1].2);

                    cr.set_source_rgba(r, g, b, 1.0);
                    cr.move_to(x_of(pair[0].0), y_of(pair[0].1 as f64));
                    cr.line_to(x_of(pair[1].0), y_of(pair[1].1 as f64));
                    cr.stroke().unwrap();
                }
            }

            // Latest reading
            let last = points[points.len() - 1];
            let (r, g, b) = color(last.2);
            let (lx, ly) = (x_of(last.0), y_of(last.1 as f64));

            cr.set_source_rgba(r, g, b, 1.0);
            cr.arc(lx, ly, 5.0, 0.0, PI * 2.0);
            cr.fill().unwrap();

            cr.set_source_rgba(0.92, 0.92, 0.94, 1.0);
            let text_x = if lx > LEFT + pw - 50.0 { lx - 40.0 } else { lx + 10.0 };
            cr.move_to(text_x, ly - 10.0);
            cr.show_text(&format!("{}%", last.1)).unwrap();
        });

        Self { area, state }
    }

    pub fn widget(&self) -> &DrawingArea {
        &self.area
    }

    pub fn set_points(&self, points: Vec<Sample>) {
        *self.state.points.borrow_mut() = points;
        self.area.queue_draw();
    }

    pub fn set_window(&self, secs: u64) {
        self.state.window_secs.set(secs.max(60));
        self.area.queue_draw();
    }
}
