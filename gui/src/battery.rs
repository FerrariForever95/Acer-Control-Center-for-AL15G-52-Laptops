use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use adw::Clamp;
use gtk::glib::{self, ControlFlow};
use gtk::prelude::*;
use gtk::{Align, Box, Frame, Grid, Label, Orientation, ToggleButton};

use crate::libacercontrol::dbus::AcerControlDBus;
use crate::widgets::battery_chart::BatteryChart;
use crate::widgets::hero_card::HeroCard;
use crate::widgets::metric_card::MetricCard;
use crate::widgets::section_header::SectionHeader;
use crate::widgets::status_chip::{StatusChip, StatusColor};

const RANGES: [(&str, u64); 4] = [
    ("1 hour", 3600),
    ("6 hours", 6 * 3600),
    ("24 hours", 24 * 3600),
    ("7 days", 7 * 24 * 3600),
];

pub struct BatteryPage;

struct BatteryInfo {
    percent: u8,
    status: String,
    power_w: Option<f64>,
    /// Full charge now vs. when new, in percent.
    health: Option<f64>,
    cycles: Option<u32>,
    /// Minutes until empty (discharging) or full (charging).
    minutes: Option<u32>,
}

fn battery_dir() -> Option<PathBuf> {
    for entry in fs::read_dir("/sys/class/power_supply").ok()?.flatten() {
        let dir = entry.path();

        let kind = fs::read_to_string(dir.join("type")).unwrap_or_default();

        if kind.trim() == "Battery" {
            return Some(dir);
        }
    }

    None
}

fn read_f64(dir: &std::path::Path, file: &str) -> Option<f64> {
    fs::read_to_string(dir.join(file)).ok()?.trim().parse().ok()
}

fn read_battery() -> Option<BatteryInfo> {
    let dir = battery_dir()?;

    let percent = read_f64(&dir, "capacity")? as u8;
    let status = fs::read_to_string(dir.join("status"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Unknown".into());

    // Batteries report either energy (µWh) or charge (µAh).
    let (now, full, design, rate) = if dir.join("energy_now").exists() {
        (
            read_f64(&dir, "energy_now"),
            read_f64(&dir, "energy_full"),
            read_f64(&dir, "energy_full_design"),
            read_f64(&dir, "power_now"),
        )
    } else {
        (
            read_f64(&dir, "charge_now"),
            read_f64(&dir, "charge_full"),
            read_f64(&dir, "charge_full_design"),
            read_f64(&dir, "current_now"),
        )
    };

    let power_w = read_f64(&dir, "power_now")
        .map(|p| p / 1e6)
        .or_else(|| {
            let current = read_f64(&dir, "current_now")?;
            let voltage = read_f64(&dir, "voltage_now")?;
            Some(current * voltage / 1e12)
        })
        .filter(|p| *p > 0.0);

    let health = match (full, design) {
        (Some(f), Some(d)) if d > 0.0 => Some((f / d * 100.0).min(100.0)),
        _ => None,
    };

    let minutes = match (status.as_str(), now, full, rate) {
        ("Discharging", Some(n), _, Some(r)) if r > 0.0 => {
            Some((n / r * 60.0).round() as u32)
        }
        ("Charging", Some(n), Some(f), Some(r)) if r > 0.0 && f > n => {
            Some(((f - n) / r * 60.0).round() as u32)
        }
        _ => None,
    };

    Some(BatteryInfo {
        percent,
        status,
        power_w,
        health,
        cycles: read_f64(&dir, "cycle_count").map(|c| c as u32).filter(|c| *c > 0),
        minutes,
    })
}

fn format_minutes(minutes: u32) -> String {
    let h = minutes / 60;
    let m = minutes % 60;

    if h == 0 {
        format!("{} min", m)
    } else {
        format!("{} h {:02} min", h, m)
    }
}

impl BatteryPage {
    pub fn new() -> Box {
        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(18)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        root.set_hexpand(true);
        root.set_vexpand(true);

        let clamp = Clamp::builder()
            .maximum_size(1120)
            .tightening_threshold(960)
            .halign(Align::Center)
            .build();

        let content = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(18)
            .build();

        // Hero
        let hero = Rc::new(HeroCard::new("BATTERY", "Reading battery...", "--%"));
        let state_chip = Rc::new(StatusChip::new("--", StatusColor::Gray));
        hero.add_chip(&state_chip);
        content.append(hero.widget());

        // History chart
        let history_header = SectionHeader::new("CHARGE HISTORY");
        content.append(history_header.widget());

        let range_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["linked"])
            .halign(Align::Start)
            .build();

        let chart = Rc::new(BatteryChart::new());
        let range_secs = Rc::new(Cell::new(RANGES[1].1));

        let mut range_buttons: Vec<ToggleButton> = Vec::new();

        for (i, (label, _)) in RANGES.iter().enumerate() {
            let button = ToggleButton::with_label(label);

            if let Some(first) = range_buttons.first() {
                button.set_group(Some(first));
            }

            button.set_active(i == 1);
            range_row.append(&button);
            range_buttons.push(button);
        }

        content.append(&range_row);

        let chart_frame = Frame::builder().hexpand(true).build();
        chart_frame.add_css_class("fan-card");
        chart.widget().set_margin_top(12);
        chart.widget().set_margin_bottom(12);
        chart.widget().set_margin_start(12);
        chart.widget().set_margin_end(12);
        chart_frame.set_child(Some(chart.widget()));
        content.append(&chart_frame);

        let note = Label::builder()
            .halign(Align::Start)
            .wrap(true)
            .visible(false)
            .build();
        note.add_css_class("cooling-note");
        content.append(&note);

        // Details
        content.append(SectionHeader::new("DETAILS").widget());

        let power_card = Rc::new(MetricCard::new("Power", "--", ""));
        let time_card = Rc::new(MetricCard::new("Time", "--", ""));
        let health_card = Rc::new(MetricCard::new("Battery Health", "--", "Of original capacity"));
        let cycles_card = Rc::new(MetricCard::new("Charge Cycles", "--", "Since new"));

        let grid = Grid::builder()
            .column_spacing(16)
            .row_spacing(16)
            .column_homogeneous(true)
            .build();

        grid.attach(power_card.widget(), 0, 0, 1, 1);
        grid.attach(time_card.widget(), 1, 0, 1, 1);
        grid.attach(health_card.widget(), 0, 1, 1, 1);
        grid.attach(cycles_card.widget(), 1, 1, 1, 1);
        content.append(&grid);

        clamp.set_child(Some(&content));
        root.append(&clamp);

        // Live readings (cheap sysfs reads)
        let refresh: Rc<dyn Fn()> = {
            let hero = hero.clone();
            let state_chip = state_chip.clone();
            let power_card = power_card.clone();
            let time_card = time_card.clone();
            let health_card = health_card.clone();
            let cycles_card = cycles_card.clone();

            Rc::new(move || {
                let Some(info) = read_battery() else {
                    hero.set_value("--");
                    hero.set_subtitle("No battery detected");
                    state_chip.set_label("NO BATTERY");
                    state_chip.set_color(StatusColor::Gray);
                    return;
                };

                hero.set_value(&format!("{}%", info.percent));

                let (chip, color, subtitle) = match info.status.as_str() {
                    "Charging" => (
                        "CHARGING",
                        StatusColor::Green,
                        match info.minutes {
                            Some(m) => format!("Full in about {}", format_minutes(m)),
                            None => "Charging".to_string(),
                        },
                    ),
                    "Discharging" => (
                        "ON BATTERY",
                        StatusColor::Blue,
                        match info.minutes {
                            Some(m) => format!("About {} left", format_minutes(m)),
                            None => "Running on battery".to_string(),
                        },
                    ),
                    "Full" => ("FULL", StatusColor::Green, "Fully charged".to_string()),
                    _ => (
                        "PLUGGED IN",
                        StatusColor::Green,
                        "Connected to power".to_string(),
                    ),
                };

                state_chip.set_label(chip);
                state_chip.set_color(color);
                hero.set_subtitle(&subtitle);

                match info.power_w {
                    Some(w) => {
                        power_card.set_value(&format!("{:.1} W", w));
                        power_card.set_subtitle(if info.status == "Charging" {
                            "Charging rate"
                        } else {
                            "Current draw"
                        });
                    }
                    None => {
                        power_card.set_value("--");
                        power_card.set_subtitle("Not reported");
                    }
                }

                match (info.status.as_str(), info.minutes) {
                    ("Charging", Some(m)) => {
                        time_card.set_value(&format_minutes(m));
                        time_card.set_subtitle("Until full");
                    }
                    ("Discharging", Some(m)) => {
                        time_card.set_value(&format_minutes(m));
                        time_card.set_subtitle("Remaining");
                    }
                    _ => {
                        time_card.set_value("--");
                        time_card.set_subtitle("Not available");
                    }
                }

                health_card.set_value(
                    &info
                        .health
                        .map(|h| format!("{:.0}%", h))
                        .unwrap_or_else(|| "--".into()),
                );

                cycles_card.set_value(
                    &info
                        .cycles
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "--".into()),
                );
            })
        };

        refresh();

        {
            let refresh = refresh.clone();

            glib::timeout_add_local(Duration::from_secs(2), move || {
                refresh();
                ControlFlow::Continue
            });
        }

        // History from the daemon (it records while the app is closed too).
        let dbus: Rc<Option<AcerControlDBus>> = Rc::new(AcerControlDBus::new().ok());
        let last_error: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        let load_history: Rc<dyn Fn()> = {
            let chart = chart.clone();
            let range_secs = range_secs.clone();
            let dbus = dbus.clone();
            let note = note.clone();
            let last_error = last_error.clone();

            Rc::new(move || {
                chart.set_window(range_secs.get());

                let result = match dbus.as_ref() {
                    Some(d) => d.battery_history(range_secs.get() as u32),
                    None => Err("Service offline.".to_string()),
                };

                match result {
                    Ok(points) => {
                        note.set_visible(false);
                        chart.set_points(points);
                    }
                    Err(e) => {
                        *last_error.borrow_mut() = e;
                        note.set_label("History unavailable while the service is offline.");
                        note.set_visible(true);
                    }
                }
            })
        };

        load_history();

        {
            let load_history = load_history.clone();

            glib::timeout_add_local(Duration::from_secs(30), move || {
                load_history();
                ControlFlow::Continue
            });
        }

        for (i, button) in range_buttons.iter().enumerate() {
            let range_secs = range_secs.clone();
            let load_history = load_history.clone();

            button.connect_toggled(move |b| {
                if b.is_active() {
                    range_secs.set(RANGES[i].1);
                    load_history();
                }
            });
        }

        root
    }
}
