use adw::prelude::*;
use gtk::{
    Align, Box, Image, Label, ListBox, ListBoxRow, Orientation, SelectionMode,
    Stack,
};

/// (page id used by the stack, label, icon)
pub const PAGES: [(&str, &str, &str); 7] = [
    ("dashboard", "Dashboard", "go-home-symbolic"),
    ("profiles", "Profiles", "view-list-symbolic"),
    ("cooling", "Cooling", "weather-windy-symbolic"),
    ("rgb", "Keyboard", "input-keyboard-symbolic"),
    ("gpu", "GPU", "video-display-symbolic"),
    ("battery", "Battery", "battery-good-symbolic"),
    ("settings", "Settings", "preferences-system-symbolic"),
];

pub struct Sidebar;

impl Sidebar {
    pub fn new(stack: &Stack) -> Box {
        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .hexpand(false)
            .vexpand(true)
            .build();

        root.add_css_class("sidebar");

        let branding = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .margin_top(22)
            .margin_bottom(18)
            .margin_start(20)
            .margin_end(20)
            .build();

        let logo = Image::from_icon_name("computer-symbolic");
        logo.set_pixel_size(28);

        let names = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .valign(Align::Center)
            .build();

        let title = Label::builder().label("Acer").halign(Align::Start).build();
        title.add_css_class("sidebar-title");

        let subtitle = Label::builder()
            .label("Control Center")
            .halign(Align::Start)
            .build();
        subtitle.add_css_class("sidebar-subtitle");

        names.append(&title);
        names.append(&subtitle);

        branding.append(&logo);
        branding.append(&names);

        let list = ListBox::builder()
            .selection_mode(SelectionMode::Single)
            .build();

        list.add_css_class("sidebar-list");

        for (_, label, icon) in PAGES {
            add_row(&list, icon, label);
        }

        let stack_clone = stack.clone();

        list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                if let Some((id, _, _)) = PAGES.get(row.index() as usize) {
                    stack_clone.set_visible_child_name(id);
                }
            }
        });

        list.select_row(list.row_at_index(0).as_ref());

        root.append(&branding);
        root.append(&list);

        root
    }
}

fn add_row(list: &ListBox, icon: &str, title: &str) {
    let row_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(9)
        .margin_bottom(9)
        .margin_start(12)
        .margin_end(12)
        .build();

    let image = Image::from_icon_name(icon);
    image.set_pixel_size(18);

    let label = Label::builder()
        .label(title)
        .halign(Align::Start)
        .hexpand(true)
        .build();

    label.add_css_class("sidebar-item-label");

    row_box.append(&image);
    row_box.append(&label);

    let row = ListBoxRow::new();
    row.add_css_class("sidebar-row");
    row.set_child(Some(&row_box));

    list.append(&row);
}
