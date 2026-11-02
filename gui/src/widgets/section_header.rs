use gtk::prelude::*;
use gtk::{Align, Box, Label, Orientation};

use crate::widgets::status_chip::StatusChip;
use crate::widgets::text::sentence_case;

pub struct SectionHeader {
    root: Box,
    title: Label,
}

impl SectionHeader {
    pub fn new(title: &str) -> Self {
        let root = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .halign(Align::Fill)
            .valign(Align::Center)
            .hexpand(true)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let title_label = Label::builder()
            .label(sentence_case(title))
            .halign(Align::Start)
            .hexpand(true)
            .build();

        title_label.add_css_class("section-header-title");

        root.append(&title_label);

        Self {
            root,
            title: title_label,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.root
    }

    pub fn set_title(&self, title: &str) {
        self.title.set_label(&sentence_case(title));
    }

    pub fn set_status(&self, chip: &StatusChip) {
        self.root.append(chip.widget());
    }
}
