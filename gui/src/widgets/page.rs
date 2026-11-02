use adw::prelude::*;
use adw::Clamp;
use gtk::{Align, Box, Frame, Label, Orientation};

/// Shared layout for a page: a centred column with a title and subtitle.
/// Append page content to `content`; return `root` from the page.
pub struct Page {
    pub root: Box,
    pub content: Box,
}

impl Page {
    pub fn new(title: &str, subtitle: &str) -> Self {
        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(28)
            .margin_bottom(28)
            .margin_start(28)
            .margin_end(28)
            .halign(Align::Fill)
            .valign(Align::Start)
            .hexpand(true)
            .vexpand(true)
            .build();

        let clamp = Clamp::builder()
            .maximum_size(1000)
            .tightening_threshold(820)
            .halign(Align::Center)
            .valign(Align::Start)
            .hexpand(true)
            .build();

        let content = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(16)
            .halign(Align::Fill)
            .valign(Align::Start)
            .hexpand(true)
            .build();

        let heading = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_bottom(8)
            .build();

        let title_label = Label::builder()
            .label(title)
            .halign(Align::Start)
            .xalign(0.0)
            .build();
        title_label.add_css_class("page-title");

        let subtitle_label = Label::builder()
            .label(subtitle)
            .halign(Align::Start)
            .xalign(0.0)
            .wrap(true)
            .build();
        subtitle_label.add_css_class("page-subtitle");

        heading.append(&title_label);
        heading.append(&subtitle_label);
        content.append(&heading);

        clamp.set_child(Some(&content));
        root.append(&clamp);

        Self { root, content }
    }
}

/// A padded surface for grouping related controls. Put widgets in the
/// returned inner box.
pub fn card() -> (Frame, Box) {
    let frame = Frame::builder().hexpand(true).build();
    frame.add_css_class("surface");

    let inner = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(20)
        .build();

    frame.set_child(Some(&inner));

    (frame, inner)
}

/// Bold card heading with an optional muted description underneath.
pub fn card_heading(title: &str, description: Option<&str>) -> Box {
    let column = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(3)
        .hexpand(true)
        .build();

    let t = Label::builder()
        .label(title)
        .halign(Align::Start)
        .xalign(0.0)
        .build();
    t.add_css_class("card-title");
    column.append(&t);

    if let Some(text) = description {
        let d = Label::builder()
            .label(text)
            .halign(Align::Start)
            .xalign(0.0)
            .wrap(true)
            .build();
        d.add_css_class("card-description");
        column.append(&d);
    }

    column
}
