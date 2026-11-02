use gtk::prelude::*;
use gtk::{Align, Box, Label, Orientation};

use crate::widgets::text::sentence_case;

#[derive(Clone, Copy)]
pub enum StatusColor {
    Green,
    Blue,
    Orange,
    Red,
    Gray,
}

pub struct StatusChip {
    root: Box,
    text: Label,
}

impl StatusChip {
    pub fn new(label: &str, color: StatusColor) -> Self {
        let root = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(Align::Start)
            .valign(Align::Center)
            .margin_top(4)
            .margin_bottom(4)
            .margin_start(8)
            .margin_end(8)
            .build();

        root.add_css_class("status-chip");

        match color {
            StatusColor::Green => root.add_css_class("status-green"),
            StatusColor::Blue => root.add_css_class("status-blue"),
            StatusColor::Orange => root.add_css_class("status-orange"),
            StatusColor::Red => root.add_css_class("status-red"),
            StatusColor::Gray => root.add_css_class("status-gray"),
        }

        let dot = Label::new(Some("●"));
        dot.add_css_class("status-chip-dot");

        let text = Label::new(Some(&sentence_case(label)));
        text.add_css_class("status-chip-label");

        root.append(&dot);
        root.append(&text);

        Self { root, text }
    }

    pub fn widget(&self) -> &Box {
        &self.root
    }

    pub fn set_label(&self, label: &str) {
        self.text.set_label(&sentence_case(label));
    }

    pub fn set_color(&self, color: StatusColor) {
        self.root.remove_css_class("status-green");
        self.root.remove_css_class("status-blue");
        self.root.remove_css_class("status-orange");
        self.root.remove_css_class("status-red");
        self.root.remove_css_class("status-gray");

        match color {
            StatusColor::Green => self.root.add_css_class("status-green"),
            StatusColor::Blue => self.root.add_css_class("status-blue"),
            StatusColor::Orange => self.root.add_css_class("status-orange"),
            StatusColor::Red => self.root.add_css_class("status-red"),
            StatusColor::Gray => self.root.add_css_class("status-gray"),
        }
    }
}
