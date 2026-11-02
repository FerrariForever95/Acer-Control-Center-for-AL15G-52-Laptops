use gtk::prelude::*;
use gtk::{Align, Box, Frame, Label, Orientation};

use crate::widgets::status_chip::StatusChip;
use crate::widgets::text::sentence_case;

pub struct HeroCard {
    frame: Frame,
    title: Label,
    subtitle: Label,
    value: Label,
    info_box: Box,
}

impl HeroCard {
    pub fn new(title: &str, subtitle: &str, value: &str) -> Self {
        let frame = Frame::builder()
            .hexpand(true)
            .vexpand(false)
            .build();

        frame.add_css_class("hero-card");

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .margin_top(20)
            .margin_bottom(20)
            .margin_start(22)
            .margin_end(22)
            .build();

        let title_label = Label::builder()
            .label(sentence_case(title))
            .halign(Align::Start)
            .build();

        title_label.add_css_class("hero-title");

        let subtitle_label = Label::builder()
            .label(subtitle)
            .halign(Align::Start)
            .wrap(true)
            .build();

        subtitle_label.add_css_class("hero-subtitle");

        let value_label = Label::builder()
            .label(value)
            .halign(Align::Start)
            .build();

        value_label.add_css_class("hero-value");

        let info_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .halign(Align::Start)
            .build();

        root.append(&title_label);
        root.append(&subtitle_label);
        root.append(&value_label);
        root.append(&info_box);

        frame.set_child(Some(&root));

        Self {
            frame,
            title: title_label,
            subtitle: subtitle_label,
            value: value_label,
            info_box,
        }
    }

    pub fn widget(&self) -> &Frame {
        &self.frame
    }

    pub fn add_chip(&self, chip: &StatusChip) {
        self.info_box.append(chip.widget());
    }

    pub fn set_title(&self, title: &str) {
        self.title.set_label(&sentence_case(title));
    }

    pub fn set_subtitle(&self, subtitle: &str) {
        self.subtitle.set_label(subtitle);
    }

    pub fn set_value(&self, value: &str) {
        self.value.set_label(value);
    }
}
