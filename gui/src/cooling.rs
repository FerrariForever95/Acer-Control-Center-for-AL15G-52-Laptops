use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use adw::Clamp;
use gtk::glib::{self, ControlFlow};
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, CheckButton, Frame, Grid, Image, Label, Orientation,
    PositionType, Scale,
};

use crate::libacercontrol::dbus::AcerControlDBus;
use crate::libacercontrol::telemetry;
use crate::widgets::curve_editor::CurveEditor;
use crate::widgets::gauge::Gauge;
use crate::widgets::ring::Ring;
use crate::widgets::section_header::SectionHeader;
use crate::widgets::status_chip::{StatusChip, StatusColor};

/// Lowest duty cycle the daemon will accept (keep in sync with daemon/src/fan.rs).
const MIN_PERCENT: f64 = 20.0;

#[derive(Clone, Copy, PartialEq)]
enum Card {
    Auto,
    Fixed(u8),
    /// Maximum fans, always. The daemon never overrides this on its own.
    Turbo,
    Custom,
}

const CARDS: [(&str, &str, &str, Card); 5] = [
    ("view-refresh-symbolic", "Auto", "Firmware decides", Card::Auto),
    (
        "power-profile-power-saver-symbolic",
        "Silent",
        "35% fixed",
        Card::Fixed(35),
    ),
    (
        "power-profile-balanced-symbolic",
        "Balanced",
        "55% fixed",
        Card::Fixed(55),
    ),
    (
        "power-profile-performance-symbolic",
        "Turbo",
        "Max fans, always",
        Card::Turbo,
    ),
    ("document-edit-symbolic", "Custom", "Follows your curve", Card::Custom),
];

pub struct CoolingPage;

fn mode_card(icon: &str, title: &str, subtitle: &str) -> Button {
    let button = Button::new();
    button.add_css_class("mode-card");
    button.set_hexpand(true);

    let inner = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .halign(Align::Center)
        .build();

    let image = Image::from_icon_name(icon);
    image.set_pixel_size(28);

    let title_label = Label::new(Some(title));
    title_label.add_css_class("heading");

    let subtitle_label = Label::new(Some(subtitle));
    subtitle_label.add_css_class("mode-card-subtitle");

    inner.append(&image);
    inner.append(&title_label);
    inner.append(&subtitle_label);

    button.set_child(Some(&inner));
    button
}

fn mode_label(mode: &str) -> (&'static str, StatusColor) {
    match mode {
        "manual" => ("FIXED SPEED", StatusColor::Blue),
        "curve" => ("CUSTOM CURVE", StatusColor::Blue),
        "turbo" => ("TURBO · MAX FAN", StatusColor::Orange),
        _ => ("AUTOMATIC", StatusColor::Green),
    }
}

impl CoolingPage {
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
            .maximum_size(1100)
            .tightening_threshold(900)
            .halign(Align::Center)
            .valign(Align::Start)
            .build();

        clamp.set_hexpand(true);

        let content = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(18)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        content.set_hexpand(true);

        // -----------------------------------------------------------
        // Banner
        // -----------------------------------------------------------
        let banner = Rc::new(adw::Banner::new(""));
        banner.set_button_label(Some("Dismiss"));
        banner.set_revealed(false);
        content.append(banner.as_ref());

        let dismissed: Rc<RefCell<String>> =
            Rc::new(RefCell::new(String::new()));

        {
            let banner_c = banner.clone();
            let dismissed = dismissed.clone();

            banner.connect_button_clicked(move |_| {
                *dismissed.borrow_mut() = banner_c.title().to_string();
                banner_c.set_revealed(false);
            });
        }

        let show_error: Rc<dyn Fn(&str)> = {
            let banner = banner.clone();

            Rc::new(move |msg: &str| {
                banner.set_title(msg);
                banner.set_revealed(true);
            })
        };

        // -----------------------------------------------------------
        // 1. Cockpit: live fans and temperature
        // -----------------------------------------------------------
        let cockpit_header = SectionHeader::new("LIVE COOLING");
        let daemon_chip =
            Rc::new(StatusChip::new("SERVICE OFFLINE", StatusColor::Red));
        daemon_chip.widget().set_visible(false);
        cockpit_header.set_status(&daemon_chip);
        content.append(cockpit_header.widget());

        let cpu_gauge = Rc::new(Gauge::new("CPU FAN"));
        let gpu_gauge = Rc::new(Gauge::new("GPU FAN"));
        cpu_gauge.set_color(0.22, 0.75, 1.0);
        gpu_gauge.set_color(1.0, 0.45, 0.16);

        let temp_ring = Rc::new(Ring::new("CPU TEMP", 240));
        temp_ring.set(0.0, "-- °C", "Waiting for sensor");

        let cockpit = Grid::builder()
            .column_spacing(16)
            .column_homogeneous(true)
            .hexpand(true)
            .build();

        cockpit.attach(cpu_gauge.widget(), 0, 0, 1, 1);
        cockpit.attach(temp_ring.widget(), 1, 0, 1, 1);
        cockpit.attach(gpu_gauge.widget(), 2, 0, 1, 1);

        let cockpit_frame = Frame::builder().hexpand(true).build();
        cockpit_frame.add_css_class("fan-card");
        cockpit.set_margin_top(12);
        cockpit.set_margin_bottom(12);
        cockpit.set_margin_start(12);
        cockpit.set_margin_end(12);
        cockpit_frame.set_child(Some(&cockpit));
        content.append(&cockpit_frame);

        // -----------------------------------------------------------
        // 2. Mode cards
        // -----------------------------------------------------------
        let mode_header = SectionHeader::new("COOLING MODE");
        let mode_chip =
            Rc::new(StatusChip::new("AUTOMATIC", StatusColor::Green));
        mode_header.set_status(&mode_chip);
        content.append(mode_header.widget());

        let cards_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .homogeneous(true)
            .hexpand(true)
            .build();

        let mut card_buttons: Vec<(Button, Card)> = Vec::new();

        for (icon, title, subtitle, card) in CARDS {
            let button = mode_card(icon, title, subtitle);
            cards_row.append(&button);
            card_buttons.push((button, card));
        }

        let card_buttons = Rc::new(card_buttons);
        content.append(&cards_row);

        // Fixed-speed sliders: CPU and GPU fans can be linked (move
        // together) or controlled separately.
        let fixed_row = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .build();

        let make_row = |name: &str| {
            let row = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(12)
                .build();

            let label = Label::builder()
                .label(name)
                .halign(Align::Start)
                .width_chars(11)
                .xalign(0.0)
                .build();
            label.add_css_class("heading");

            let scale = Rc::new(Scale::with_range(
                Orientation::Horizontal,
                MIN_PERCENT,
                100.0,
                1.0,
            ));
            scale.set_value(50.0);
            scale.set_draw_value(true);
            scale.set_value_pos(PositionType::Right);
            scale.set_format_value_func(|_, v| format!("{:.0}%", v));
            scale.set_hexpand(true);

            for mark in [25.0, 50.0, 75.0, 100.0] {
                scale.add_mark(mark, PositionType::Bottom, None);
            }

            row.append(&label);
            row.append(scale.as_ref());

            (row, scale)
        };

        let (cpu_row, cpu_scale) = make_row("CPU fan");
        let (gpu_row, gpu_scale) = make_row("GPU fan");

        let link_check = Rc::new(CheckButton::with_label(
            "Link CPU and GPU fans (move together)",
        ));
        link_check.set_active(true);
        link_check.set_halign(Align::Start);

        fixed_row.append(&cpu_row);
        fixed_row.append(&gpu_row);
        fixed_row.append(link_check.as_ref());
        content.append(&fixed_row);

        // -----------------------------------------------------------
        // 3. Curve editor
        // -----------------------------------------------------------
        let curve_header = SectionHeader::new("CUSTOM FAN CURVE");
        content.append(curve_header.widget());

        let curve_hint = Label::builder()
            .label(
                "Drag points to set fan speed by CPU temperature.",
            )
            .halign(Align::Start)
            .wrap(true)
            .build();
        curve_hint.add_css_class("curve-hint");
        content.append(&curve_hint);

        let editor = Rc::new(CurveEditor::new());

        let editor_frame = Frame::builder().hexpand(true).build();
        editor_frame.add_css_class("fan-card");
        editor.widget().set_margin_top(12);
        editor.widget().set_margin_bottom(12);
        editor.widget().set_margin_start(12);
        editor.widget().set_margin_end(12);
        editor_frame.set_child(Some(editor.widget()));
        content.append(&editor_frame);

        let reset_button = Button::with_label("Reset curve");
        reset_button.set_halign(Align::End);
        content.append(&reset_button);

        let safety_label = Label::builder()
            .label(
                "Minimum 20%. Auto control returns at 90°C. Turbo stays \
                 at 100% until you change mode.",
            )
            .halign(Align::Start)
            .wrap(true)
            .build();
        safety_label.add_css_class("cooling-note");
        content.append(&safety_label);

        clamp.set_child(Some(&content));
        root.append(&clamp);

        // -----------------------------------------------------------
        // Behaviour
        // -----------------------------------------------------------
        let dbus: Rc<Option<AcerControlDBus>> =
            Rc::new(AcerControlDBus::new().ok());

        if dbus.is_none() {
            show_error("Cannot reach the Acer Control daemon.");
            cards_row.set_sensitive(false);
            fixed_row.set_sensitive(false);
            editor_frame.set_sensitive(false);
            reset_button.set_sensitive(false);
        }

        let updating = Rc::new(Cell::new(false));
        let generation = Rc::new(Cell::new(0u32));
        let current_mode: Rc<RefCell<String>> =
            Rc::new(RefCell::new(String::new()));

        let go_auto: Rc<dyn Fn()> = {
            let dbus = dbus.clone();
            let banner = banner.clone();
            let show_error = show_error.clone();

            Rc::new(move || {
                if let Some(d) = dbus.as_ref() {
                    match d.set_fan_auto() {
                        Ok(()) => banner.set_revealed(false),
                        Err(e) => show_error(&e),
                    }
                }
            })
        };

        let apply_fixed: Rc<dyn Fn(u8, u8)> = {
            let dbus = dbus.clone();
            let banner = banner.clone();
            let show_error = show_error.clone();

            Rc::new(move |cpu: u8, gpu: u8| {
                if let Some(d) = dbus.as_ref() {
                    // 100% on both fans is Turbo on the daemon side.
                    match d.set_fan_manual(cpu, gpu) {
                        Ok(()) => banner.set_revealed(false),
                        Err(e) => show_error(&e),
                    }
                }
            })
        };

        let apply_curve: Rc<dyn Fn()> = {
            let dbus = dbus.clone();
            let editor = editor.clone();
            let banner = banner.clone();
            let show_error = show_error.clone();

            Rc::new(move || {
                if let Some(d) = dbus.as_ref() {
                    match d.set_fan_curve(editor.points()) {
                        Ok(()) => banner.set_revealed(false),
                        Err(e) => show_error(&e),
                    }
                }
            })
        };

        // Cards
        for (button, card) in card_buttons.iter() {
            let card = *card;
            let go_auto = go_auto.clone();
            let apply_fixed = apply_fixed.clone();
            let apply_curve = apply_curve.clone();
            let cpu_scale = cpu_scale.clone();
            let gpu_scale = gpu_scale.clone();
            let link_check = link_check.clone();
            let updating = updating.clone();

            button.connect_clicked(move |_| match card {
                Card::Auto => go_auto(),
                Card::Fixed(v) => {
                    updating.set(true);
                    cpu_scale.set_value(v as f64);
                    gpu_scale.set_value(v as f64);
                    link_check.set_active(true);
                    updating.set(false);
                    apply_fixed(v, v);
                }
                Card::Turbo => {
                    updating.set(true);
                    cpu_scale.set_value(100.0);
                    gpu_scale.set_value(100.0);
                    link_check.set_active(true);
                    updating.set(false);
                    apply_fixed(100, 100);
                }
                Card::Custom => apply_curve(),
            });
        }

        // Fixed-speed sliders (debounced). When linked, moving one fan
        // moves the other too.
        {
            let schedule: Rc<dyn Fn()> = {
                let apply_fixed = apply_fixed.clone();
                let generation = generation.clone();
                let cpu_scale = cpu_scale.clone();
                let gpu_scale = gpu_scale.clone();

                Rc::new(move || {
                    let gen_id = generation.get().wrapping_add(1);
                    generation.set(gen_id);

                    let apply_fixed = apply_fixed.clone();
                    let generation = generation.clone();
                    let cpu_scale = cpu_scale.clone();
                    let gpu_scale = gpu_scale.clone();

                    glib::timeout_add_local_once(
                        Duration::from_millis(250),
                        move || {
                            if generation.get() == gen_id {
                                apply_fixed(
                                    cpu_scale.value().round() as u8,
                                    gpu_scale.value().round() as u8,
                                );
                            }
                        },
                    );
                })
            };

            {
                let updating = updating.clone();
                let link_check = link_check.clone();
                let gpu_scale = gpu_scale.clone();
                let schedule = schedule.clone();

                cpu_scale.connect_value_changed(move |s| {
                    if updating.get() {
                        return;
                    }

                    if link_check.is_active() {
                        updating.set(true);
                        gpu_scale.set_value(s.value());
                        updating.set(false);
                    }

                    schedule();
                });
            }

            {
                let updating = updating.clone();
                let link_check = link_check.clone();
                let cpu_scale = cpu_scale.clone();
                let schedule = schedule.clone();

                gpu_scale.connect_value_changed(move |s| {
                    if updating.get() {
                        return;
                    }

                    if link_check.is_active() {
                        updating.set(true);
                        cpu_scale.set_value(s.value());
                        updating.set(false);
                    }

                    schedule();
                });
            }

            // Re-linking snaps the GPU fan to the CPU fan's speed.
            {
                let updating = updating.clone();
                let cpu_scale = cpu_scale.clone();
                let gpu_scale = gpu_scale.clone();
                let schedule = schedule.clone();

                link_check.connect_toggled(move |c| {
                    if c.is_active() && !updating.get() {
                        updating.set(true);
                        gpu_scale.set_value(cpu_scale.value());
                        updating.set(false);
                        schedule();
                    }
                });
            }
        }

        // Curve editor: apply when a point is released
        {
            let apply_curve = apply_curve.clone();

            editor.connect_commit(move |_| apply_curve());
        }

        {
            let editor = editor.clone();
            let apply_curve = apply_curve.clone();
            let current_mode = current_mode.clone();

            reset_button.connect_clicked(move |_| {
                editor.reset();

                if *current_mode.borrow() == "curve" {
                    apply_curve();
                }
            });
        }

        // Status refresh: the UI always reflects what the daemon is doing,
        // including thermal reverts.
        let synced = Rc::new(Cell::new(false));

        let refresh: Rc<dyn Fn()> = {
            let dbus = dbus.clone();
            let daemon_chip = daemon_chip.clone();
            let mode_chip = mode_chip.clone();
            let temp_ring = temp_ring.clone();
            let editor = editor.clone();
            let cpu_scale = cpu_scale.clone();
            let gpu_scale = gpu_scale.clone();
            let link_check = link_check.clone();
            let card_buttons = card_buttons.clone();
            let banner = banner.clone();
            let dismissed = dismissed.clone();
            let updating = updating.clone();
            let current_mode = current_mode.clone();
            let synced = synced.clone();

            Rc::new(move || {
                let Some(d) = dbus.as_ref() else {
                    daemon_chip.widget().set_visible(true);
                    return;
                };

                let temp = d.cpu_temperature().ok().flatten();

                match temp {
                    Some(t) => {
                        let (caption, r, g, b) = if t >= 85 {
                            ("Hot", 0.94, 0.27, 0.27)
                        } else if t >= 70 {
                            ("Warm", 0.98, 0.57, 0.24)
                        } else {
                            ("Normal", 0.29, 0.87, 0.50)
                        };

                        temp_ring.set_color(r, g, b);
                        temp_ring.set(
                            t as f64 / 100.0,
                            &format!("{} °C", t),
                            caption,
                        );
                    }
                    None => temp_ring.set(0.0, "-- °C", "Sensor unavailable"),
                }

                editor.set_temperature(temp);

                let Ok((mode, cpu, gpu, notice)) = d.fan_status() else {
                    daemon_chip.widget().set_visible(true);
                    return;
                };

                daemon_chip.widget().set_visible(false);

                let (text, color) = mode_label(&mode);
                mode_chip.set_label(text);
                mode_chip.set_color(color);

                editor.set_live(mode == "curve");

                // First contact, or the mode changed behind our back
                // (thermal revert, another client): adopt the real state.
                let changed = *current_mode.borrow() != mode;

                if changed || !synced.get() {
                    if mode == "manual" || mode == "turbo" {
                        updating.set(true);
                        cpu_scale.set_value(cpu as f64);
                        gpu_scale.set_value(gpu as f64);
                        link_check.set_active(cpu == gpu);
                        updating.set(false);
                    }

                    if !synced.get() || mode == "curve" {
                        if let Ok(points) = d.fan_curve() {
                            editor.set_points(points);
                        }
                    }

                    *current_mode.borrow_mut() = mode.clone();
                    synced.set(true);
                }

                for (button, card) in card_buttons.iter() {
                    let active = match card {
                        Card::Auto => mode == "auto",
                        Card::Fixed(v) => {
                            mode == "manual" && cpu == *v && gpu == *v
                        }
                        Card::Turbo => mode == "turbo",
                        Card::Custom => mode == "curve",
                    };

                    if active {
                        button.add_css_class("mode-card-active");
                    } else {
                        button.remove_css_class("mode-card-active");
                    }
                }

                if !notice.is_empty() && *dismissed.borrow() != notice {
                    banner.set_title(&notice);
                    banner.set_revealed(true);
                }
            })
        };

        refresh();

        {
            let refresh = refresh.clone();

            glib::timeout_add_local(Duration::from_secs(1), move || {
                refresh();
                ControlFlow::Continue
            });
        }

        // Live fan gauges.
        {
            let cpu_gauge = cpu_gauge.clone();
            let gpu_gauge = gpu_gauge.clone();

            glib::timeout_add_local(Duration::from_millis(500), move || {
                if let Ok(v) = telemetry::cpu_fan_rpm() {
                    cpu_gauge.set_value(v as f64);
                }

                if let Ok(v) = telemetry::gpu_fan_rpm() {
                    gpu_gauge.set_value(v as f64);
                }

                ControlFlow::Continue
            });
        }

        root
    }
}
