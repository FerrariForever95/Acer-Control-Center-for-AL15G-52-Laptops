use adw::prelude::*;
use adw::Clamp;
use gtk::prelude::*;
use gtk::{Align, Box, Button, Frame, Label, Orientation};

use crate::libacercontrol::driver;
use crate::widgets::hero_card::HeroCard;
use crate::widgets::section_header::SectionHeader;

pub struct SettingsPage;

impl SettingsPage {
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

        content.append(
            HeroCard::new("ACER CONTROL CENTER", "About", env!("CARGO_PKG_VERSION"))
                .widget(),
        );

        content.append(SectionHeader::new("SUPPORT").widget());

        let frame = Frame::builder().build();

        let inner = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(20)
            .margin_bottom(20)
            .margin_start(20)
            .margin_end(20)
            .build();

        let text = Label::builder()
            .label("Copy system details for a bug report.")
            .halign(Align::Start)
            .wrap(true)
            .build();

        let copy = Button::with_label("Copy diagnostics");
        copy.add_css_class("pill");
        copy.set_halign(Align::Start);

        copy.connect_clicked(|button| {
            let report = format!(
                "Acer Control Center {}\nDriver: {}\nABI: {}.{}\nInterface: {}\nHardware supported: {}\n",
                env!("CARGO_PKG_VERSION"),
                driver::version().unwrap_or_else(|_| "unknown".into()),
                driver::abi_major(),
                driver::abi_minor(),
                driver::interface().unwrap_or_else(|_| "unknown".into()),
                driver::hardware_supported().unwrap_or(false),
            );

            button.clipboard().set_text(&report);
            button.set_label("Copied");
        });

        inner.append(&text);
        inner.append(&copy);
        frame.set_child(Some(&inner));
        content.append(&frame);

        clamp.set_child(Some(&content));
        root.append(&clamp);

        root
    }
}
