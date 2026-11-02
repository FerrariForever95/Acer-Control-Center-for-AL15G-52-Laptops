//! Profiles page: save the current fan + keyboard setup, apply it again in
//! one click, and choose one to apply every time the laptop starts.
//!
//! The daemon does the real work (capturing, storing, applying and the
//! startup run); this page only lists profiles and asks for actions.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;
use gtk::{
    Align, Box, Button, CheckButton, Entry, Label, ListBox, ListBoxRow,
    Orientation, SelectionMode,
};

use crate::libacercontrol::dbus::AcerControlDBus;
use crate::widgets::page::{card, card_heading, Page};
use crate::widgets::section_header::SectionHeader;

type Dbus = Rc<Option<AcerControlDBus>>;
type SetStatus = Rc<dyn Fn(&str, bool)>;

pub struct ProfilesPage;

impl ProfilesPage {
    pub fn new() -> Box {
        let page = Page::new(
            "Profiles",
            "Save fan and lighting setups and apply them in one click.",
        );

        let dbus: Dbus = Rc::new(AcerControlDBus::new().ok());

        // One line of feedback shared by every action on the page.
        let status = Label::builder()
            .halign(Align::Start)
            .xalign(0.0)
            .wrap(true)
            .visible(false)
            .build();
        status.add_css_class("form-status");

        let set_status: SetStatus = {
            let status = status.clone();

            Rc::new(move |message: &str, is_error: bool| {
                status.set_label(message);
                status.set_visible(!message.is_empty());

                if is_error {
                    status.add_css_class("form-status-error");
                } else {
                    status.remove_css_class("form-status-error");
                }
            })
        };

        // -------------------------------------------------------------
        // Save the current settings
        // -------------------------------------------------------------
        let (save_card, save_box) = card();

        save_box.append(&card_heading(
            "Save current settings",
            Some(
                "Saves your current fan mode and keyboard lighting.",
            ),
        ));

        let form = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .build();

        let entry = Entry::builder()
            .placeholder_text("Profile name, for example Gaming")
            .hexpand(true)
            .max_length(32)
            .build();

        let save_button = Button::with_label("Save profile");
        save_button.add_css_class("suggested-action");

        form.append(&entry);
        form.append(&save_button);
        save_box.append(&form);

        page.content.append(&save_card);
        page.content.append(&status);

        // -------------------------------------------------------------
        // Saved profiles
        // -------------------------------------------------------------
        page.content
            .append(SectionHeader::new("Saved profiles").widget());

        let list = ListBox::builder()
            .selection_mode(SelectionMode::None)
            .build();
        list.add_css_class("boxed-list");

        let empty = Label::builder()
            .label(
                "No profiles yet.",
            )
            .halign(Align::Start)
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("card-description");

        let startup_note = Label::builder()
            .label(
                "The “At startup” profile is applied at boot.",
            )
            .halign(Align::Start)
            .xalign(0.0)
            .wrap(true)
            .build();
        startup_note.add_css_class("card-description");

        page.content.append(&list);
        page.content.append(&empty);
        page.content.append(&startup_note);

        // -------------------------------------------------------------
        // Reloading the list. Rows need to trigger a reload, so the
        // reload function reaches itself through a shared cell.
        // -------------------------------------------------------------
        let reload_cell: Rc<RefCell<Option<Rc<dyn Fn()>>>> =
            Rc::new(RefCell::new(None));

        let reload: Rc<dyn Fn()> = {
            let cell = reload_cell.clone();

            Rc::new(move || {
                let f = cell.borrow().clone();

                if let Some(f) = f {
                    f();
                }
            })
        };

        let do_reload: Rc<dyn Fn()> = {
            let dbus = dbus.clone();
            let list = list.clone();
            let empty = empty.clone();
            let startup_note = startup_note.clone();
            let reload = reload.clone();
            let set_status = set_status.clone();

            Rc::new(move || {
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }

                let Some(d) = dbus.as_ref() else {
                    return;
                };

                let profiles = match d.list_profiles() {
                    Ok(p) => p,
                    Err(e) => {
                        set_status(&e, true);
                        Vec::new()
                    }
                };

                let startup = d.startup_profile().unwrap_or_default();

                list.set_visible(!profiles.is_empty());
                empty.set_visible(profiles.is_empty());
                startup_note.set_visible(!profiles.is_empty());

                for (name, summary) in profiles {
                    let is_startup = name == startup;

                    list.append(&build_row(
                        &name,
                        &summary,
                        is_startup,
                        dbus.clone(),
                        reload.clone(),
                        set_status.clone(),
                    ));
                }
            })
        };

        *reload_cell.borrow_mut() = Some(do_reload.clone());

        // -------------------------------------------------------------
        // Save action
        // -------------------------------------------------------------
        let do_save: Rc<dyn Fn()> = {
            let dbus = dbus.clone();
            let entry = entry.clone();
            let reload = reload.clone();
            let set_status = set_status.clone();

            Rc::new(move || {
                let Some(d) = dbus.as_ref() else {
                    set_status("Cannot reach the Acer Control daemon.", true);
                    return;
                };

                let name = entry.text().trim().to_string();

                match d.save_profile(&name) {
                    Ok(()) => {
                        set_status(&format!("Saved “{}”.", name), false);
                        entry.set_text("");
                        reload();
                    }
                    Err(e) => set_status(&e, true),
                }
            })
        };

        {
            let do_save = do_save.clone();
            save_button.connect_clicked(move |_| do_save());
        }

        {
            let do_save = do_save.clone();
            entry.connect_activate(move |_| do_save());
        }

        if dbus.is_none() {
            set_status("Cannot reach the Acer Control daemon.", true);
            save_card.set_sensitive(false);
        }

        do_reload();

        page.root
    }
}

fn build_row(
    name: &str,
    summary: &str,
    is_startup: bool,
    dbus: Dbus,
    reload: Rc<dyn Fn()>,
    set_status: SetStatus,
) -> ListBoxRow {
    let row = ListBoxRow::builder()
        .activatable(false)
        .selectable(false)
        .build();

    let inner = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(12)
        .build();

    let text = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(Align::Center)
        .build();

    let title = Label::builder()
        .label(name)
        .halign(Align::Start)
        .xalign(0.0)
        .build();
    title.add_css_class("row-title");

    let subtitle = Label::builder()
        .label(summary)
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .build();
    subtitle.add_css_class("row-subtitle");

    text.append(&title);
    text.append(&subtitle);

    // At startup (exactly one profile can have this)
    let startup = CheckButton::with_label("At startup");
    startup.set_active(is_startup);
    startup.set_valign(Align::Center);
    startup.set_tooltip_text(Some(
        "Apply this profile automatically when the laptop starts",
    ));

    {
        let dbus = dbus.clone();
        let reload = reload.clone();
        let set_status = set_status.clone();
        let name = name.to_string();

        startup.connect_toggled(move |check| {
            let Some(d) = dbus.as_ref() else {
                return;
            };

            let enable = check.is_active();
            let wanted = if enable { name.as_str() } else { "" };

            match d.set_startup_profile(wanted) {
                Ok(()) => {
                    if enable {
                        set_status(
                            &format!("“{}” will be applied at startup.", name),
                            false,
                        );
                    } else {
                        set_status("No profile will be applied at startup.", false);
                    }
                }
                Err(e) => set_status(&e, true),
            }

            // Rebuild so the check marks always match what the daemon has.
            let reload = reload.clone();
            glib::idle_add_local_once(move || reload());
        });
    }

    // Apply
    let apply = Button::with_label("Apply");
    apply.add_css_class("suggested-action");
    apply.set_valign(Align::Center);

    {
        let dbus = dbus.clone();
        let set_status = set_status.clone();
        let name = name.to_string();

        apply.connect_clicked(move |_| {
            let Some(d) = dbus.as_ref() else {
                return;
            };

            match d.apply_profile(&name) {
                Ok(()) => set_status(&format!("Applied “{}”.", name), false),
                Err(e) => set_status(&e, true),
            }
        });
    }

    // Update: replace this profile with whatever is active right now
    let update = Button::with_label("Update");
    update.set_valign(Align::Center);
    update.set_tooltip_text(Some(
        "Replace this profile with your current fan and keyboard settings",
    ));

    {
        let dbus = dbus.clone();
        let reload = reload.clone();
        let set_status = set_status.clone();
        let name = name.to_string();

        update.connect_clicked(move |_| {
            let Some(d) = dbus.as_ref() else {
                return;
            };

            match d.save_profile(&name) {
                Ok(()) => {
                    set_status(
                        &format!("Updated “{}” with your current settings.", name),
                        false,
                    );
                    reload();
                }
                Err(e) => set_status(&e, true),
            }
        });
    }

    // Delete (asks twice: first click arms it for three seconds)
    let delete = Button::from_icon_name("user-trash-symbolic");
    delete.add_css_class("flat");
    delete.set_valign(Align::Center);
    delete.set_tooltip_text(Some("Delete profile"));

    {
        let armed = Rc::new(Cell::new(false));
        let dbus = dbus.clone();
        let reload = reload.clone();
        let set_status = set_status.clone();
        let name = name.to_string();

        delete.connect_clicked(move |button| {
            if !armed.get() {
                armed.set(true);
                button.set_label("Confirm");
                button.add_css_class("destructive-action");

                let armed = armed.clone();
                let button = button.clone();

                glib::timeout_add_local_once(Duration::from_secs(3), move || {
                    if armed.get() {
                        armed.set(false);
                        button.set_icon_name("user-trash-symbolic");
                        button.remove_css_class("destructive-action");
                    }
                });

                return;
            }

            let Some(d) = dbus.as_ref() else {
                return;
            };

            match d.delete_profile(&name) {
                Ok(()) => {
                    set_status(&format!("Deleted “{}”.", name), false);
                    reload();
                }
                Err(e) => set_status(&e, true),
            }
        });
    }

    inner.append(&text);
    inner.append(&startup);
    inner.append(&update);
    inner.append(&apply);
    inner.append(&delete);

    row.set_child(Some(&inner));
    row
}
