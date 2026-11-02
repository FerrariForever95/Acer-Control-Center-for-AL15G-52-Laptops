//! GPU page: NVIDIA status, "sleep when idle" control and graphics mode
//! switching (through EnvyControl).

use std::cell::{Cell, RefCell};
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib::{self, ControlFlow};
use gtk::{
    Align, Box, Button, Grid, Image, Label, Orientation, Switch,
};

use crate::libacercontrol::dbus::AcerControlDBus;
use crate::widgets::page::{card, card_heading, Page};
use crate::widgets::section_header::SectionHeader;
use crate::widgets::status_chip::{StatusChip, StatusColor};

/// (daemon name, icon, title, subtitle)
const MODES: [(&str, &str, &str, &str); 3] = [
    (
        "integrated",
        "video-display-symbolic",
        "Integrated",
        "GPU off, best battery",
    ),
    (
        "hybrid",
        "video-joined-displays-symbolic",
        "Hybrid",
        "GPU wakes on demand",
    ),
    (
        "nvidia",
        "applications-games-symbolic",
        "NVIDIA",
        "GPU always on",
    ),
];

pub struct GpuPage;

fn mode_button(icon: &str, title: &str, subtitle: &str) -> Button {
    let button = Button::new();
    button.add_css_class("mode-card");
    button.set_hexpand(true);

    let inner = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .halign(Align::Center)
        .build();

    let image = Image::from_icon_name(icon);
    image.set_pixel_size(26);

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

/// Adds a "name / value" row to the grid and returns the value label.
fn info_row(grid: &Grid, row: i32, name: &str) -> Label {
    let key = Label::builder()
        .label(name)
        .halign(Align::Start)
        .xalign(0.0)
        .build();
    key.add_css_class("dim-label");
    key.set_width_chars(16);

    let value = Label::builder()
        .label("--")
        .halign(Align::Start)
        .xalign(0.0)
        .hexpand(true)
        .wrap(true)
        .selectable(true)
        .build();

    grid.attach(&key, 0, row, 1, 1);
    grid.attach(&value, 1, row, 1, 1);

    value
}

fn muted(text: &str) -> Label {
    let label = Label::builder()
        .label(text)
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .build();
    label.add_css_class("card-description");
    label
}

fn title_for(mode: &str) -> &'static str {
    MODES
        .iter()
        .find(|m| m.0 == mode)
        .map(|m| m.2)
        .unwrap_or("that")
}

/// Friendly name for any mode text the daemon reports (including ones this
/// page has no button for, such as prime-select's "on-demand").
fn display_mode(mode: &str) -> String {
    match MODES.iter().find(|m| m.0 == mode) {
        Some(m) => m.2.to_string(),
        None => capitalise(mode),
    }
}

impl GpuPage {
    pub fn new() -> Box {
        let page = Page::new(
            "GPU",
            "NVIDIA graphics mode and power management.",
        );

        let dbus: Rc<Option<AcerControlDBus>> =
            Rc::new(AcerControlDBus::new().ok());

        // -------------------------------------------------------------
        // Status
        // -------------------------------------------------------------
        let (status_card, status_box) = card();

        let status_top = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let chip = Rc::new(StatusChip::new("Checking", StatusColor::Gray));

        status_top.append(&card_heading("NVIDIA GPU", None));
        status_top.append(chip.widget());
        status_box.append(&status_top);

        let grid = Grid::builder().row_spacing(8).column_spacing(16).build();

        let model_value = info_row(&grid, 0, "Model");
        let address_value = info_row(&grid, 1, "PCI address");
        let driver_value = info_row(&grid, 2, "Driver");
        let power_value = info_row(&grid, 3, "Power state");
        let d3_value = info_row(&grid, 4, "Runtime D3");

        status_box.append(&grid);
        page.content.append(&status_card);

        // -------------------------------------------------------------
        // Sleep when idle
        // -------------------------------------------------------------
        let (sleep_card, sleep_box) = card();

        let sleep_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let power_switch = Switch::builder().valign(Align::Center).build();

        sleep_row.append(&card_heading(
            "Let the GPU sleep when idle",
            Some(
                "Powers the GPU down when nothing uses it. Resets on restart.",
            ),
        ));
        sleep_row.append(&power_switch);
        sleep_box.append(&sleep_row);

        let sleep_hint = muted("");
        sleep_hint.set_visible(false);
        sleep_box.append(&sleep_hint);

        let sleep_error = Label::builder()
            .halign(Align::Start)
            .xalign(0.0)
            .wrap(true)
            .visible(false)
            .build();
        sleep_error.add_css_class("form-status");
        sleep_error.add_css_class("form-status-error");
        sleep_box.append(&sleep_error);

        page.content.append(&sleep_card);

        // -------------------------------------------------------------
        // Graphics mode
        // -------------------------------------------------------------
        page.content
            .append(SectionHeader::new("Graphics mode").widget());

        let (mode_card_frame, mode_box) = card();

        mode_box.append(&card_heading(
            "Choose how the laptop uses the GPU",
            Some("Applied after a restart."),
        ));

        let current_mode = Label::builder()
            .label("Current mode: checking…")
            .halign(Align::Start)
            .xalign(0.0)
            .build();
        current_mode.add_css_class("row-title");
        mode_box.append(&current_mode);

        let cards_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .homogeneous(true)
            .build();

        let mut mode_buttons: Vec<(Button, &'static str)> = Vec::new();

        for (name, icon, title, subtitle) in MODES {
            let button = mode_button(icon, title, subtitle);
            cards_row.append(&button);
            mode_buttons.push((button, name));
        }

        let mode_buttons = Rc::new(mode_buttons);
        mode_box.append(&cards_row);

        // Confirmation row (hidden until a mode is picked)
        let confirm_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .visible(false)
            .build();

        let confirm_label = Label::builder()
            .halign(Align::Start)
            .xalign(0.0)
            .hexpand(true)
            .wrap(true)
            .build();

        let cancel_button = Button::with_label("Cancel");
        let confirm_button = Button::with_label("Switch");
        confirm_button.add_css_class("suggested-action");

        confirm_row.append(&confirm_label);
        confirm_row.append(&cancel_button);
        confirm_row.append(&confirm_button);
        mode_box.append(&confirm_row);

        let mode_status = Label::builder()
            .halign(Align::Start)
            .xalign(0.0)
            .wrap(true)
            .visible(false)
            .build();
        mode_status.add_css_class("form-status");
        mode_box.append(&mode_status);

        let tool_note = muted("");
        tool_note.set_visible(false);
        mode_box.append(&tool_note);

        // Restart prompt (shown while a mode change is waiting for a reboot)
        let restart_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .visible(false)
            .build();

        let restart_label = Label::builder()
            .label("Restart to finish switching graphics mode.")
            .halign(Align::Start)
            .xalign(0.0)
            .hexpand(true)
            .wrap(true)
            .build();
        restart_label.add_css_class("form-status-warning");

        let restart_button = Button::with_label("Restart now");
        restart_button.add_css_class("suggested-action");

        restart_row.append(&restart_label);
        restart_row.append(&restart_button);
        mode_box.append(&restart_row);

        page.content.append(&mode_card_frame);

        // -------------------------------------------------------------
        // Behaviour
        // -------------------------------------------------------------
        if dbus.is_none() {
            chip.set_label("Service offline");
            chip.set_color(StatusColor::Red);
            sleep_card.set_sensitive(false);
            mode_card_frame.set_sensitive(false);

            return page.root;
        }

        let updating = Rc::new(Cell::new(false));
        let pending: Rc<RefCell<Option<&'static str>>> =
            Rc::new(RefCell::new(None));
        let current: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        // Sleep switch
        {
            let dbus = dbus.clone();
            let updating = updating.clone();
            let sleep_error = sleep_error.clone();

            power_switch.connect_active_notify(move |s| {
                if updating.get() {
                    return;
                }

                let Some(d) = dbus.as_ref() else {
                    return;
                };

                match d.set_gpu_power_control(s.is_active()) {
                    Ok(()) => sleep_error.set_visible(false),
                    Err(e) => {
                        sleep_error.set_label(&e);
                        sleep_error.set_visible(true);

                        // Put the switch back to what the system really has.
                        updating.set(true);
                        s.set_active(!s.is_active());
                        updating.set(false);
                    }
                }
            });
        }

        // Mode buttons: pick, then confirm
        for (button, name) in mode_buttons.iter() {
            let name: &'static str = *name;
            let pending = pending.clone();
            let current = current.clone();
            let confirm_row = confirm_row.clone();
            let confirm_label = confirm_label.clone();
            let mode_status = mode_status.clone();

            button.connect_clicked(move |_| {
                if *current.borrow() == name {
                    mode_status.set_label(&format!(
                        "Already in {} mode.",
                        title_for(name)
                    ));
                    mode_status.set_visible(true);
                    confirm_row.set_visible(false);
                    *pending.borrow_mut() = None;
                    return;
                }

                *pending.borrow_mut() = Some(name);

                confirm_label.set_label(&format!(
                    "Switch to {} mode? This changes system configuration \
                     and takes effect after you restart.",
                    title_for(name)
                ));
                confirm_row.set_visible(true);
                mode_status.set_visible(false);
            });
        }

        {
            let pending = pending.clone();
            let confirm_row = confirm_row.clone();

            cancel_button.connect_clicked(move |_| {
                *pending.borrow_mut() = None;
                confirm_row.set_visible(false);
            });
        }

        {
            let dbus = dbus.clone();
            let pending = pending.clone();
            let confirm_row = confirm_row.clone();
            let mode_status = mode_status.clone();

            confirm_button.connect_clicked(move |_| {
                let Some(name) = pending.borrow_mut().take() else {
                    return;
                };

                confirm_row.set_visible(false);

                let Some(d) = dbus.as_ref() else {
                    return;
                };

                mode_status.remove_css_class("form-status-error");

                match d.set_gpu_mode(name) {
                    Ok(()) => {
                        mode_status.set_label(&format!(
                            "Switching to {} mode…",
                            title_for(name)
                        ));
                    }
                    Err(e) => {
                        mode_status.set_label(&e);
                        mode_status.add_css_class("form-status-error");
                    }
                }

                mode_status.set_visible(true);
            });
        }

        // Restart button: first click arms it, second click reboots.
        {
            let armed = Rc::new(Cell::new(false));
            let mode_status = mode_status.clone();

            restart_button.connect_clicked(move |b| {
                if !armed.get() {
                    armed.set(true);
                    b.set_label("Click again to restart");

                    let armed = armed.clone();
                    let b = b.clone();

                    glib::timeout_add_local_once(
                        Duration::from_secs(4),
                        move || {
                            armed.set(false);
                            b.set_label("Restart now");
                        },
                    );

                    return;
                }

                armed.set(false);
                b.set_label("Restarting…");

                mode_status.remove_css_class("form-status-error");
                mode_status.set_label("Restarting the laptop…");
                mode_status.set_visible(true);

                std::thread::spawn(|| {
                    let _ = Command::new("systemctl").arg("reboot").status();
                });
            });
        }

        // Live refresh
        let refresh: Rc<dyn Fn()> = {
            let dbus = dbus.clone();
            let chip = chip.clone();
            let updating = updating.clone();
            let current = current.clone();
            let mode_buttons = mode_buttons.clone();
            let mode_status = mode_status.clone();
            let restart_row = restart_row.clone();

            Rc::new(move || {
                let Some(d) = dbus.as_ref() else {
                    return;
                };

                let Ok(s) = d.gpu_status() else {
                    chip.set_label("Service offline");
                    chip.set_color(StatusColor::Red);
                    return;
                };

                let get = |key: &str| s.get(key).cloned().unwrap_or_default();

                let present = get("present") == "yes";
                let mode = get("mode");
                let tool = get("tool");
                let switch = get("switch");
                let runtime = get("runtime_status");
                let control = get("power_control");
                let rtd3 = get("rtd3");
                let dynamic = get("dynamic_pm");

                // ---- status card
                if present {
                    match runtime.as_str() {
                        "suspended" => {
                            chip.set_label("Sleeping");
                            chip.set_color(StatusColor::Blue);
                        }
                        "active" => {
                            chip.set_label("Active");
                            chip.set_color(StatusColor::Green);
                        }
                        _ => {
                            chip.set_label("Present");
                            chip.set_color(StatusColor::Gray);
                        }
                    }

                    model_value.set_label(&get("model"));
                    address_value.set_label(&get("address"));
                    driver_value.set_label(&get("driver"));

                    let power = if runtime.is_empty() {
                        "Unknown".to_string()
                    } else {
                        let mut text = capitalise(&runtime);

                        if !control.is_empty() {
                            text.push_str(&format!(
                                " (power control: {})",
                                control
                            ));
                        }

                        text
                    };
                    power_value.set_label(&power);

                    let mut d3 = if rtd3.is_empty() {
                        "Not reported (needs the NVIDIA driver)".to_string()
                    } else {
                        rtd3.clone()
                    };

                    if !dynamic.is_empty() {
                        d3.push_str(&format!(
                            "  ·  driver setting: {}",
                            dynamic
                        ));
                    }

                    d3_value.set_label(&d3);
                } else {
                    if mode == "integrated" {
                        chip.set_label("Off · Integrated mode");
                    } else {
                        chip.set_label("Not detected");
                    }

                    chip.set_color(StatusColor::Gray);

                    let none = if mode == "integrated" {
                        "Disabled by Integrated mode"
                    } else {
                        "No NVIDIA GPU found"
                    };

                    model_value.set_label(none);
                    address_value.set_label("--");
                    driver_value.set_label("--");
                    power_value.set_label("--");
                    d3_value.set_label("--");
                }

                // ---- sleep switch
                power_switch.set_sensitive(present);

                updating.set(true);
                power_switch.set_active(control == "auto");
                updating.set(false);

                if present && rtd3.contains("Not supported") {
                    sleep_hint.set_label(
                        "The driver reports that runtime power management \
                         is not supported on this system, so the GPU may \
                         stay awake even with this switched on.",
                    );
                    sleep_hint.set_visible(true);
                } else if present && driver_value.label().as_str() != "nvidia" {
                    sleep_hint.set_label(
                        "The proprietary NVIDIA driver is not in use, so the \
                         driver’s runtime power management is not \
                         available.",
                    );
                    sleep_hint.set_visible(true);
                } else {
                    sleep_hint.set_visible(false);
                }

                // ---- graphics mode
                *current.borrow_mut() = mode.clone();

                let running = switch.starts_with("running:");

                current_mode.set_label(&if mode.is_empty() {
                    "Current mode: unknown".to_string()
                } else {
                    format!("Current mode: {}", display_mode(&mode))
                });

                for (button, name) in mode_buttons.iter() {
                    if *name == mode {
                        button.add_css_class("mode-card-active");
                    } else {
                        button.remove_css_class("mode-card-active");
                    }

                    button.set_sensitive(tool == "envycontrol" && !running);
                }

                match tool.as_str() {
                    "envycontrol" => tool_note.set_visible(false),
                    "prime-select" => {
                        tool_note.set_label(
                            "Install EnvyControl to switch modes from here.",
                        );
                        tool_note.set_visible(true);
                    }
                    _ => {
                        tool_note.set_label(
                            "Install EnvyControl to switch modes from here.",
                        );
                        tool_note.set_visible(true);
                    }
                }

                // A restart is pending when the configured mode and the
                // hardware disagree, or a switch just finished.
                let mismatch = !mode.is_empty()
                    && ((mode == "integrated" && present)
                        || (mode != "integrated" && !present));

                restart_row.set_visible(
                    !running && (mismatch || switch.starts_with("done:")),
                );

                // Progress of a switch started from this page or earlier.
                if running {
                    let target = switch.trim_start_matches("running:");

                    mode_status.remove_css_class("form-status-error");
                    mode_status.set_label(&format!(
                        "Switching to {} mode… this can take a minute. \
                         Don’t turn the laptop off.",
                        title_for(target)
                    ));
                    mode_status.set_visible(true);
                } else if let Some(target) = switch.strip_prefix("done:") {
                    mode_status.remove_css_class("form-status-error");
                    mode_status.set_label(&format!(
                        "Switched to {} mode. Restart the laptop to apply it.",
                        title_for(target)
                    ));
                    mode_status.set_visible(true);
                } else if let Some(message) = switch.strip_prefix("failed:") {
                    mode_status.add_css_class("form-status-error");
                    mode_status.set_label(&format!(
                        "Mode change failed: {}",
                        message
                    ));
                    mode_status.set_visible(true);
                }
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

        page.root
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();

    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
