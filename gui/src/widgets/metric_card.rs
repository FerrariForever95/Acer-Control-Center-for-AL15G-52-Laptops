use gtk::prelude::*;
use gtk::{Align, Box, Frame, Label, Orientation};

pub struct MetricCard {
    frame: Frame,
    value_label: Label,
    subtitle_label: Label,
    title_label: Label,
}

impl MetricCard {
    pub fn new(title: &str, value: &str, subtitle: &str) -> Self {
        let frame = Frame::builder()
            .hexpand(true)
            .vexpand(false)
            .build();

        frame.add_css_class("metric-card");

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(18)
            .margin_end(18)
            .halign(Align::Start)
            .build();

        let title_label = Label::builder()
            .label(title)
            .halign(Align::Start)
            .build();

        title_label.add_css_class("metric-card-title");

        let value_label = Label::builder()
            .label(value)
            .halign(Align::Start)
            .build();

        value_label.add_css_class("metric-card-value");

        let subtitle_label = Label::builder()
            .label(subtitle)
            .halign(Align::Start)
            .build();

        subtitle_label.add_css_class("metric-card-subtitle");

        root.append(&title_label);
        root.append(&value_label);
        root.append(&subtitle_label);

        frame.set_child(Some(&root));

        Self {
            frame,
            value_label,
            subtitle_label,
            title_label,
        }
    }

    pub fn widget(&self) -> &Frame {
        &self.frame
    }

    pub fn set_value(&self, value: &str) {
        self.value_label.set_label(value);
    }

    pub fn set_subtitle(&self, subtitle: &str) {
        self.subtitle_label.set_label(subtitle);
    }

    pub fn set_title(&self, title: &str) {
        self.title_label.set_label(title);
    }
}
