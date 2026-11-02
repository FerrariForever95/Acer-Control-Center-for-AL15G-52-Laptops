use std::fs;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib::{self, ControlFlow};
use gtk::prelude::*;
use gtk::{Align, Box, Grid, Orientation};

use crate::libacercontrol::dbus::AcerControlDBus;
use crate::libacercontrol::telemetry;
use crate::widgets::hero_card::HeroCard;
use crate::widgets::metric_card::MetricCard;
use crate::widgets::section_header::SectionHeader;
use crate::widgets::status_chip::{StatusChip, StatusColor};

pub struct Dashboard {
    pub root: Box,
}

/// Laptop name from firmware (DMI), e.g. "Acer Nitro AN515-58".
fn laptop_name() -> String {
    let read = |file: &str| {
        fs::read_to_string(format!("/sys/class/dmi/id/{}", file))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };

    let vendor = read("sys_vendor");
    let product = read("product_name");

    match (vendor.is_empty(), product.is_empty()) {
        (false, false) if product.to_lowercase().starts_with(&vendor.to_lowercase()) => {
            product
        }
        (false, false) => format!("{} {}", vendor, product),
        (true, false) => product,
        _ => "Acer laptop".to_string(),
    }
}

/// Battery percentage and charging state straight from sysfs.
fn update_battery(card: &MetricCard) {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return;
    };

    for entry in entries.flatten() {
        let dir = entry.path();

        let kind = fs::read_to_string(dir.join("type")).unwrap_or_default();

        if kind.trim() != "Battery" {
            continue;
        }

        let percent = fs::read_to_string(dir.join("capacity"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let status = fs::read_to_string(dir.join("status")).unwrap_or_default();

        card.set_value(&format!("{}%", percent));
        card.set_subtitle(match status.trim() {
            "Charging" => "Charging",
            "Discharging" => "On battery",
            "Full" => "Fully charged",
            _ => "Plugged in",
        });

        return;
    }

    card.set_value("--");
    card.set_subtitle("No battery");
}

fn update_uptime(card: &MetricCard) {
    let secs = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(str::to_string))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;

    let (d, h, m) = (secs / 86400, (secs % 86400) / 3600, (secs % 3600) / 60);

    card.set_value(&if d > 0 {
        format!("{}d {}h", d, h)
    } else if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    });
}

impl Dashboard {
    pub fn new() -> Self {
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
        root.set_vexpand(false);

        let content = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(18)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        content.set_hexpand(true);
        content.set_vexpand(false);

        let hero = Rc::new(HeroCard::new(
            &laptop_name().to_uppercase(),
            "System status",
            "Ready",
        ));

        let status_chip = Rc::new(StatusChip::new("ONLINE", StatusColor::Green));
        hero.add_chip(&status_chip);

        content.append(hero.widget());

        content.append(SectionHeader::new("AT A GLANCE").widget());

        let temp_card = Rc::new(MetricCard::new("CPU Temperature", "-- °C", "Reading sensor"));
        let fan_card = Rc::new(MetricCard::new("Fans", "-- %", "Average speed"));
        let mode_card = Rc::new(MetricCard::new("Cooling Mode", "--", ""));
        let light_card = Rc::new(MetricCard::new("Keyboard Lighting", "--", ""));
        let battery_card = Rc::new(MetricCard::new("Battery", "--%", ""));
        let uptime_card = Rc::new(MetricCard::new("Uptime", "--", "Since last restart"));

        let grid = Grid::builder()
            .column_spacing(16)
            .row_spacing(16)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        grid.set_hexpand(true);
        grid.set_column_homogeneous(true);

        grid.attach(temp_card.widget(), 0, 0, 1, 1);
        grid.attach(fan_card.widget(), 1, 0, 1, 1);
        grid.attach(battery_card.widget(), 0, 1, 1, 1);
        grid.attach(uptime_card.widget(), 1, 1, 1, 1);
        grid.attach(mode_card.widget(), 0, 2, 1, 1);
        grid.attach(light_card.widget(), 1, 2, 1, 1);

        content.append(&grid);
        root.append(&content);

        let dbus: Rc<Option<AcerControlDBus>> = Rc::new(AcerControlDBus::new().ok());

        let refresh: Rc<dyn Fn()> = {
            let hero = hero.clone();
            let status_chip = status_chip.clone();
            let temp_card = temp_card.clone();
            let fan_card = fan_card.clone();
            let mode_card = mode_card.clone();
            let light_card = light_card.clone();
            let battery_card = battery_card.clone();
            let uptime_card = uptime_card.clone();
            let dbus = dbus.clone();

            Rc::new(move || {
                update_battery(&battery_card);
                update_uptime(&uptime_card);

                let status = dbus.as_ref().as_ref().and_then(|d| {
                    d.fan_status().ok().map(|s| (d, s))
                });

                let Some((d, (mode, cpu, gpu, _notice))) = status else {
                    hero.set_value("Service offline");
                    status_chip.set_label("OFFLINE");
                    status_chip.set_color(StatusColor::Red);
                    return;
                };

                hero.set_value("Ready");
                status_chip.set_label("ONLINE");
                status_chip.set_color(StatusColor::Green);

                match d.cpu_temperature() {
                    Ok(Some(t)) => {
                        temp_card.set_value(&format!("{} °C", t));
                        temp_card.set_subtitle(if t >= 85 {
                            "Hot"
                        } else if t >= 70 {
                            "Warm"
                        } else {
                            "Normal"
                        });
                    }
                    _ => {
                        temp_card.set_value("-- °C");
                        temp_card.set_subtitle("Sensor unavailable");
                    }
                }

                // Fan speed: prefer the live reading, fall back to the target.
                let live = (telemetry::cpu_fan_rpm().ok(), telemetry::gpu_fan_rpm().ok());
                let avg = match live {
                    (Some(c), Some(g)) => Some((c + g) / 2),
                    _ => None,
                };

                match avg {
                    Some(v) => {
                        fan_card.set_value(&format!("{}%", v));
                        fan_card.set_subtitle("Average speed");
                    }
                    None => {
                        fan_card.set_value(&format!("{}%", (cpu as u16 + gpu as u16) / 2));
                        fan_card.set_subtitle("Target speed");
                    }
                }

                let (text, sub) = match mode.as_str() {
                    "manual" => ("Fixed speed", "Set by you"),
                    "turbo" => ("Turbo", "Max fans, always"),
                    "curve" => ("Custom curve", "Follows temperature"),
                    _ => ("Automatic", "Firmware controlled"),
                };
                mode_card.set_value(text);
                mode_card.set_subtitle(sub);

                match d.keyboard_animation() {
                    Ok((true, _, _)) => {
                        light_card.set_value("Color transition");
                        light_card.set_subtitle("Fading between colors");
                    }
                    _ => match d.keyboard_color() {
                        Ok((r, g, b)) => {
                            light_card.set_value(&format!("#{:02X}{:02X}{:02X}", r, g, b));
                            light_card.set_subtitle("Solid color");
                        }
                        Err(_) => {
                            light_card.set_value("--");
                            light_card.set_subtitle("Not available");
                        }
                    },
                }
            })
        };

        refresh();

        glib::timeout_add_local(Duration::from_secs(2), move || {
            refresh();
            ControlFlow::Continue
        });

        Self { root }
    }
}
