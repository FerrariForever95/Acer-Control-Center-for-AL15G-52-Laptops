use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use adw::Clamp;
use gtk::gdk::RGBA;
use gtk::glib::{self, ControlFlow};
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, ColorDialog, ColorDialogButton, DrawingArea, Frame, Grid, Label,
    Orientation, Scale,
};

use crate::libacercontrol::dbus::AcerControlDBus;
use crate::widgets::hero_card::HeroCard;
use crate::widgets::metric_card::MetricCard;
use crate::widgets::section_header::SectionHeader;
use crate::widgets::status_chip::{StatusChip, StatusColor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LedColor {
    r: u8,
    g: u8,
    b: u8,
}

struct RgbUiState {
    syncing: Cell<bool>,
    brightness: Cell<u8>,
    color: Cell<LedColor>,
}

pub struct RgbPage;

impl RgbPage {
    pub fn new() -> Box {
        let initial_brightness = read_brightness_from_daemon().unwrap_or(255);
        let initial_color = read_color_from_daemon().unwrap_or(LedColor {
            r: 255,
            g: 255,
            b: 255,
        });

        let state = Rc::new(RgbUiState {
            syncing: Cell::new(false),
            brightness: Cell::new(initial_brightness),
            color: Cell::new(initial_color),
        });

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

        let clamp = Clamp::builder()
            .maximum_size(1120)
            .tightening_threshold(960)
            .halign(Align::Center)
            .valign(Align::Start)
            .build();

        clamp.set_hexpand(true);
        clamp.set_vexpand(false);

        let content = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(18)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        content.set_hexpand(true);
        content.set_vexpand(false);

        let hero = Rc::new(HeroCard::new(
            "KEYBOARD RGB",
            "Keyboard lighting",
            &format_color_hex(initial_color),
        ));

        content.append(hero.widget());

        content.append(SectionHeader::new("CURRENT SETTINGS").widget());

        let brightness_metric = Rc::new(MetricCard::new(
            "Brightness",
            &format!("{}/255", initial_brightness),
            "Backlight level",
        ));
        let color_metric = Rc::new(MetricCard::new(
            "Color",
            &format_color_hex(initial_color),
            "Hex value",
        ));

        let metric_grid = Grid::builder()
            .column_spacing(16)
            .row_spacing(16)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        metric_grid.set_hexpand(true);
        metric_grid.set_column_homogeneous(true);

        metric_grid.attach(brightness_metric.widget(), 0, 0, 1, 1);
        metric_grid.attach(color_metric.widget(), 1, 0, 1, 1);

        content.append(&metric_grid);

        content.append(SectionHeader::new("LIGHTING").widget());

        let split_grid = Grid::builder()
            .column_spacing(16)
            .row_spacing(16)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        split_grid.set_hexpand(true);
        split_grid.set_column_homogeneous(true);

        let preview_card = Frame::builder().build();
        preview_card.add_css_class("rgb-panel-card");
        preview_card.add_css_class("rgb-preview-card");
        preview_card.set_hexpand(true);
        preview_card.set_halign(Align::Fill);
        preview_card.set_vexpand(false);

        let preview_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        let preview_label = Label::builder()
            .label("Keyboard Preview")
            .halign(Align::Start)
            .css_classes(["heading"])
            .build();

        let preview_area = DrawingArea::builder()
            .content_width(620)
            .content_height(220)
            .build();

        preview_area.set_size_request(620, 220);
        preview_area.set_hexpand(false);
        preview_area.set_vexpand(false);

        {
            let draw_state = state.clone();
            preview_area.set_draw_func(move |_, cr, width, height| {
                let width = width as f64;
                let height = height as f64;
                let color = draw_state.color.get();
                let brightness = draw_state.brightness.get();
                draw_keyboard_preview(cr, width, height, color, brightness);
            });
        }

        preview_box.append(&preview_label);
        preview_box.append(&preview_area);
        preview_card.set_child(Some(&preview_box));

        let controls_card = Frame::builder().build();
        controls_card.add_css_class("rgb-panel-card");
        controls_card.add_css_class("rgb-controls-card");
        controls_card.set_hexpand(true);
        controls_card.set_halign(Align::Fill);
        controls_card.set_vexpand(false);

        let controls_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(14)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        let controls_label = Label::builder()
            .label("Controls")
            .halign(Align::Start)
            .css_classes(["rgb-control-heading"])
            .build();

        let controls_grid = Grid::builder()
            .column_spacing(16)
            .row_spacing(12)
            .halign(Align::Fill)
            .valign(Align::Start)
            .build();

        controls_grid.set_hexpand(true);

        let color_label = Label::builder()
            .label("Color")
            .halign(Align::Start)
            .build();

        let color_dialog = ColorDialog::builder().modal(true).build();

        let color_button = ColorDialogButton::builder().build();
        color_button.set_dialog(&color_dialog);
        color_button.set_rgba(&rgba_from_led_color(initial_color));

        let brightness_label = Label::builder()
            .label("Brightness")
            .halign(Align::Start)
            .build();

        let brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);
        brightness_scale.set_digits(0);
        brightness_scale.set_draw_value(true);
        brightness_scale.set_hexpand(true);
        brightness_scale.set_value(initial_brightness as f64);

        let readout_label = Label::builder()
            .halign(Align::Start)
            .wrap(true)
            .build();

        readout_label.add_css_class("rgb-readout");

        update_readout(&readout_label, initial_color, initial_brightness);

        controls_grid.attach(&color_label, 0, 0, 1, 1);
        controls_grid.attach(&color_button, 1, 0, 1, 1);
        controls_grid.attach(&brightness_label, 0, 1, 1, 1);
        controls_grid.attach(&brightness_scale, 1, 1, 1, 1);
        controls_grid.attach(&readout_label, 0, 2, 2, 1);

        controls_box.append(&controls_label);
        controls_box.append(&controls_grid);
        controls_card.set_child(Some(&controls_box));

        split_grid.attach(&preview_card, 0, 0, 1, 1);
        split_grid.attach(&controls_card, 1, 0, 1, 1);

        content.append(&split_grid);

        // ---------------------------------------------------------------
        // Effects: fade between colours, rainbow, or react to key presses.
        // The daemon runs them, so they keep going when the app is closed.
        // ---------------------------------------------------------------
        let anim_header = SectionHeader::new("EFFECTS");
        let anim_chip = Rc::new(StatusChip::new("RUNNING", StatusColor::Green));
        anim_chip.widget().set_visible(false);
        anim_header.set_status(&anim_chip);
        content.append(anim_header.widget());

        let anim_dbus: Rc<Option<AcerControlDBus>> =
            Rc::new(AcerControlDBus::new().ok());

        let anim_running = Rc::new(Cell::new(false));
        let anim_syncing = Rc::new(Cell::new(false));
        let anim_adopted = Rc::new(Cell::new(false));
        let anim_generation = Rc::new(Cell::new(0u32));

        let make_color_button = |r: f32, g: f32, b: f32| {
            let button = ColorDialogButton::builder().build();
            button.set_dialog(&ColorDialog::builder().modal(true).build());
            button.set_rgba(&RGBA::new(r, g, b, 1.0));
            button
        };

        let make_scale = |min: f64, max: f64, value: f64| {
            let scale = Scale::with_range(Orientation::Horizontal, min, max, 0.1);
            scale.set_value(value);
            scale.set_digits(1);
            scale.set_draw_value(true);
            scale.set_hexpand(true);
            scale.set_format_value_func(|_, v| format!("{:.1} s", v));
            scale
        };

        // Fade page
        let from_button = make_color_button(1.0, 0.0, 0.0);
        let to_button = make_color_button(0.0, 0.0, 1.0);
        let fade_scale = make_scale(1.0, 20.0, 4.0);

        // Rainbow page
        let rainbow_scale = make_scale(3.0, 60.0, 10.0);

        // React page
        let base_button = make_color_button(0.0, 0.0, 1.0);
        let pulse_button = make_color_button(1.0, 1.0, 1.0);
        let react_scale = make_scale(0.2, 3.0, 0.8);

        // Gradient bars (fade: A -> B -> A, rainbow: every hue)
        let fade_bar = DrawingArea::builder()
            .content_height(28)
            .hexpand(true)
            .build();

        {
            let from_button = from_button.clone();
            let to_button = to_button.clone();

            fade_bar.set_draw_func(move |_, cr, width, height| {
                let a = from_button.rgba();
                let b = to_button.rgba();
                let w = width as f64;
                let h = height as f64;

                let gradient = gtk::cairo::LinearGradient::new(0.0, 0.0, w, 0.0);
                for (pos, c) in [(0.0, &a), (0.5, &b), (1.0, &a)] {
                    gradient.add_color_stop_rgb(
                        pos,
                        c.red() as f64,
                        c.green() as f64,
                        c.blue() as f64,
                    );
                }

                rounded_rect_simple(cr, w, h);
                cr.set_source(&gradient).unwrap();
                cr.fill().unwrap();
            });
        }

        let rainbow_bar = DrawingArea::builder()
            .content_height(28)
            .hexpand(true)
            .build();

        rainbow_bar.set_draw_func(|_, cr, width, height| {
            let w = width as f64;
            let h = height as f64;

            let gradient = gtk::cairo::LinearGradient::new(0.0, 0.0, w, 0.0);
            let stops = [
                (0.0, (1.0, 0.0, 0.0)),
                (1.0 / 6.0, (1.0, 1.0, 0.0)),
                (2.0 / 6.0, (0.0, 1.0, 0.0)),
                (3.0 / 6.0, (0.0, 1.0, 1.0)),
                (4.0 / 6.0, (0.0, 0.0, 1.0)),
                (5.0 / 6.0, (1.0, 0.0, 1.0)),
                (1.0, (1.0, 0.0, 0.0)),
            ];

            for (pos, (r, g, b)) in stops {
                gradient.add_color_stop_rgb(pos, r, g, b);
            }

            rounded_rect_simple(cr, w, h);
            cr.set_source(&gradient).unwrap();
            cr.fill().unwrap();
        });

        // Mode switcher
        let mode_fade = gtk::ToggleButton::with_label("Fade");
        let mode_rainbow = gtk::ToggleButton::with_label("Rainbow");
        let mode_react = gtk::ToggleButton::with_label("React to keys");
        mode_rainbow.set_group(Some(&mode_fade));
        mode_react.set_group(Some(&mode_fade));
        mode_fade.set_active(true);

        let mode_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["linked"])
            .halign(Align::Start)
            .build();
        mode_row.append(&mode_fade);
        mode_row.append(&mode_rainbow);
        mode_row.append(&mode_react);

        let labelled = |text: &str| {
            Label::builder().label(text).halign(Align::Start).build()
        };

        // Fade options
        let fade_colors = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();
        fade_colors.append(&from_button);
        fade_colors.append(&Label::new(Some("→")));
        fade_colors.append(&to_button);
        fade_colors.append(&fade_bar);

        let fade_page = Grid::builder().column_spacing(16).row_spacing(12).build();
        fade_page.attach(&labelled("Colors"), 0, 0, 1, 1);
        fade_page.attach(&fade_colors, 1, 0, 1, 1);
        fade_page.attach(&labelled("Fade time"), 0, 1, 1, 1);
        fade_page.attach(&fade_scale, 1, 1, 1, 1);
        fade_colors.set_hexpand(true);
        fade_page.set_hexpand(true);

        // Rainbow options
        let rainbow_page = Grid::builder().column_spacing(16).row_spacing(12).build();
        rainbow_page.attach(&labelled("Colors"), 0, 0, 1, 1);
        rainbow_page.attach(&rainbow_bar, 1, 0, 1, 1);
        rainbow_page.attach(&labelled("Full cycle"), 0, 1, 1, 1);
        rainbow_page.attach(&rainbow_scale, 1, 1, 1, 1);
        rainbow_page.set_hexpand(true);

        // React options
        let react_colors = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();
        react_colors.append(&labelled("Idle"));
        react_colors.append(&base_button);
        react_colors.append(&labelled("On key press"));
        react_colors.append(&pulse_button);

        let react_note = Label::builder()
            .label("Flashes on each key press, then fades back. Presses are counted, never recorded.")
            .halign(Align::Start)
            .wrap(true)
            .build();
        react_note.add_css_class("dim-label");

        let react_page = Grid::builder().column_spacing(16).row_spacing(12).build();
        react_page.attach(&labelled("Colors"), 0, 0, 1, 1);
        react_page.attach(&react_colors, 1, 0, 1, 1);
        react_page.attach(&labelled("Fade out"), 0, 1, 1, 1);
        react_page.attach(&react_scale, 1, 1, 1, 1);
        react_page.attach(&react_note, 0, 2, 2, 1);
        react_page.set_hexpand(true);

        let pages = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .hexpand(true)
            .build();
        pages.add_named(&fade_page, Some("fade"));
        pages.add_named(&rainbow_page, Some("rainbow"));
        pages.add_named(&react_page, Some("reactive"));
        pages.set_visible_child_name("fade");

        let start_button = Button::with_label("Start effect");
        start_button.add_css_class("pill");
        start_button.add_css_class("suggested-action");
        start_button.set_halign(Align::Start);

        let anim_error = Label::builder()
            .halign(Align::Start)
            .wrap(true)
            .visible(false)
            .build();
        anim_error.add_css_class("error");

        let current_mode = {
            let mode_fade = mode_fade.clone();
            let mode_rainbow = mode_rainbow.clone();

            Rc::new(move || -> &'static str {
                if mode_fade.is_active() {
                    "fade"
                } else if mode_rainbow.is_active() {
                    "rainbow"
                } else {
                    "reactive"
                }
            })
        };

        let to_rgb = |button: &ColorDialogButton| {
            let c = led_color_from_rgba(&button.rgba());
            (c.r, c.g, c.b)
        };

        let set_running_ui: Rc<dyn Fn(bool)> = {
            let anim_running = anim_running.clone();
            let start_button = start_button.clone();
            let anim_chip = anim_chip.clone();

            Rc::new(move |running: bool| {
                anim_running.set(running);
                anim_chip.widget().set_visible(running);

                if running {
                    start_button.set_label("Stop effect");
                    start_button.remove_css_class("suggested-action");
                } else {
                    start_button.set_label("Start effect");
                    start_button.add_css_class("suggested-action");
                }
            })
        };

        let start_effect: Rc<dyn Fn()> = {
            let anim_dbus = anim_dbus.clone();
            let current_mode = current_mode.clone();
            let from_button = from_button.clone();
            let to_button = to_button.clone();
            let base_button = base_button.clone();
            let pulse_button = pulse_button.clone();
            let fade_scale = fade_scale.clone();
            let rainbow_scale = rainbow_scale.clone();
            let react_scale = react_scale.clone();
            let set_running_ui = set_running_ui.clone();
            let anim_error = anim_error.clone();

            Rc::new(move || {
                let Some(d) = anim_dbus.as_ref() else {
                    anim_error.set_label("Service offline.");
                    anim_error.set_visible(true);
                    return;
                };

                let result = match current_mode() {
                    "fade" => d.start_keyboard_animation(
                        vec![to_rgb(&from_button), to_rgb(&to_button)],
                        fade_scale.value(),
                    ),
                    "rainbow" => d.start_keyboard_rainbow(rainbow_scale.value()),
                    _ => d.start_keyboard_reactive(
                        to_rgb(&base_button),
                        to_rgb(&pulse_button),
                        react_scale.value(),
                    ),
                };

                match result {
                    Ok(()) => {
                        anim_error.set_visible(false);
                        set_running_ui(true);
                    }
                    Err(e) => {
                        anim_error.set_label(&e);
                        anim_error.set_visible(true);
                    }
                }
            })
        };

        {
            let anim_dbus = anim_dbus.clone();
            let anim_running = anim_running.clone();
            let start_effect = start_effect.clone();
            let set_running_ui = set_running_ui.clone();
            let anim_error = anim_error.clone();

            start_button.connect_clicked(move |_| {
                if anim_running.get() {
                    if let Some(d) = anim_dbus.as_ref() {
                        match d.stop_keyboard_animation() {
                            Ok(()) => {
                                anim_error.set_visible(false);
                                set_running_ui(false);
                            }
                            Err(e) => {
                                anim_error.set_label(&e);
                                anim_error.set_visible(true);
                            }
                        }
                    }
                } else {
                    start_effect();
                }
            });
        }

        // Switching mode: show that mode's options (and switch live if an
        // effect is already running).
        for (button, name) in [
            (&mode_fade, "fade"),
            (&mode_rainbow, "rainbow"),
            (&mode_react, "reactive"),
        ] {
            let pages = pages.clone();
            let anim_syncing = anim_syncing.clone();
            let anim_running = anim_running.clone();
            let start_effect = start_effect.clone();

            button.connect_toggled(move |b| {
                if !b.is_active() {
                    return;
                }

                pages.set_visible_child_name(name);

                if !anim_syncing.get() && anim_running.get() {
                    start_effect();
                }
            });
        }

        // Changing colours updates a running effect live.
        for button in [&from_button, &to_button, &base_button, &pulse_button] {
            let fade_bar = fade_bar.clone();
            let anim_syncing = anim_syncing.clone();
            let anim_running = anim_running.clone();
            let start_effect = start_effect.clone();

            button.connect_rgba_notify(move |_| {
                fade_bar.queue_draw();

                if !anim_syncing.get() && anim_running.get() {
                    start_effect();
                }
            });
        }

        // Changing a slider updates a running effect live (debounced).
        for scale in [&fade_scale, &rainbow_scale, &react_scale] {
            let anim_syncing = anim_syncing.clone();
            let anim_running = anim_running.clone();
            let anim_generation = anim_generation.clone();
            let start_effect = start_effect.clone();

            scale.connect_value_changed(move |_| {
                if anim_syncing.get() || !anim_running.get() {
                    return;
                }

                let gen_id = anim_generation.get().wrapping_add(1);
                anim_generation.set(gen_id);

                let anim_generation = anim_generation.clone();
                let start_effect = start_effect.clone();

                glib::timeout_add_local_once(
                    Duration::from_millis(300),
                    move || {
                        if anim_generation.get() == gen_id {
                            start_effect();
                        }
                    },
                );
            });
        }

        let anim_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(14)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();

        anim_box.append(&mode_row);
        anim_box.append(&pages);
        anim_box.append(&start_button);
        anim_box.append(&anim_error);

        let anim_card = Frame::builder().hexpand(true).build();
        anim_card.add_css_class("rgb-panel-card");
        anim_card.set_child(Some(&anim_box));
        content.append(&anim_card);

        if anim_dbus.is_none() {
            anim_card.set_sensitive(false);
        }

        // Keep the UI in step with the daemon (e.g. a fixed colour was
        // picked, which ends the effect).
        {
            let anim_dbus = anim_dbus.clone();
            let anim_running = anim_running.clone();
            let anim_syncing = anim_syncing.clone();
            let anim_adopted = anim_adopted.clone();
            let set_running_ui = set_running_ui.clone();
            let from_button = from_button.clone();
            let to_button = to_button.clone();
            let base_button = base_button.clone();
            let pulse_button = pulse_button.clone();
            let fade_scale = fade_scale.clone();
            let rainbow_scale = rainbow_scale.clone();
            let react_scale = react_scale.clone();
            let mode_fade = mode_fade.clone();
            let mode_rainbow = mode_rainbow.clone();
            let mode_react = mode_react.clone();

            glib::timeout_add_local(Duration::from_millis(1000), move || {
                let Some(d) = anim_dbus.as_ref() else {
                    return ControlFlow::Continue;
                };

                let Ok((kind, colors, seconds)) = d.keyboard_effect() else {
                    return ControlFlow::Continue;
                };

                let running = kind != "none";

                if running != anim_running.get() {
                    set_running_ui(running);
                }

                // First contact: show what the daemon is really doing.
                if !anim_adopted.get() && running {
                    let rgba = |c: (u8, u8, u8)| {
                        RGBA::new(
                            c.0 as f32 / 255.0,
                            c.1 as f32 / 255.0,
                            c.2 as f32 / 255.0,
                            1.0,
                        )
                    };

                    anim_syncing.set(true);

                    match kind.as_str() {
                        "fade" if colors.len() >= 2 => {
                            from_button.set_rgba(&rgba(colors[0]));
                            to_button.set_rgba(&rgba(colors[1]));
                            fade_scale.set_value(seconds);
                            mode_fade.set_active(true);
                        }
                        "rainbow" => {
                            rainbow_scale.set_value(seconds);
                            mode_rainbow.set_active(true);
                        }
                        "reactive" if colors.len() >= 2 => {
                            base_button.set_rgba(&rgba(colors[0]));
                            pulse_button.set_rgba(&rgba(colors[1]));
                            react_scale.set_value(seconds);
                            mode_react.set_active(true);
                        }
                        _ => {}
                    }

                    anim_syncing.set(false);
                }

                anim_adopted.set(true);

                ControlFlow::Continue
            });
        }

        // While an effect runs, make the preview follow the real colour.
        {
            let anim_dbus = anim_dbus.clone();
            let anim_running = anim_running.clone();
            let state = state.clone();
            let hero = hero.clone();
            let brightness_metric = brightness_metric.clone();
            let color_metric = color_metric.clone();
            let preview_area = preview_area.clone();
            let readout_label = readout_label.clone();

            glib::timeout_add_local(Duration::from_millis(100), move || {
                if !anim_running.get() {
                    return ControlFlow::Continue;
                }

                let Some(d) = anim_dbus.as_ref() else {
                    return ControlFlow::Continue;
                };

                if let Ok((r, g, b)) = d.keyboard_color() {
                    let color = LedColor { r, g, b };

                    if color != state.color.get() {
                        state.color.set(color);

                        refresh_display(
                            &hero,
                            &brightness_metric,
                            &color_metric,
                            &readout_label,
                            state.brightness.get(),
                            color,
                        );
                        preview_area.queue_draw();
                    }
                }

                ControlFlow::Continue
            });
        }

        clamp.set_child(Some(&content));
        root.append(&clamp);

        refresh_display(
            &hero,
            &brightness_metric,
            &color_metric,
            &readout_label,
            initial_brightness,
            initial_color,
        );

        {
            let state = state.clone();
            let hero = hero.clone();
            let brightness_metric = brightness_metric.clone();
            let color_metric = color_metric.clone();
            let preview_area = preview_area.clone();
            let readout_label = readout_label.clone();

            color_button.connect_rgba_notify(move |button| {
                if state.syncing.get() {
                    return;
                }

                let color = led_color_from_rgba(&button.rgba());
                state.color.set(color);

                match write_color_via_daemon(color) {
                    Ok(()) => {
                        refresh_display(
                            &hero,
                            &brightness_metric,
                            &color_metric,
                            &readout_label,
                            state.brightness.get(),
                            color,
                        );
                        preview_area.queue_draw();
                    }
                    Err(err) => {
                        readout_label.set_label(&err);
                    }
                }
            });
        }

        {
            let state = state.clone();
            let hero = hero.clone();
            let brightness_metric = brightness_metric.clone();
            let color_metric = color_metric.clone();
            let preview_area = preview_area.clone();
            let readout_label = readout_label.clone();

            brightness_scale.connect_value_changed(move |scale| {
                if state.syncing.get() {
                    return;
                }

                let brightness = scale.value().round().clamp(0.0, 255.0) as u8;
                state.brightness.set(brightness);

                match write_brightness_via_daemon(brightness) {
                    Ok(()) => {
                        refresh_display(
                            &hero,
                            &brightness_metric,
                            &color_metric,
                            &readout_label,
                            brightness,
                            state.color.get(),
                        );
                        preview_area.queue_draw();
                    }
                    Err(err) => {
                        readout_label.set_label(&err);
                    }
                }
            });
        }

        {
            let state = state.clone();
            let hero = hero.clone();
            let brightness_metric = brightness_metric.clone();
            let color_metric = color_metric.clone();
            let preview_area = preview_area.clone();
            let color_button = color_button.clone();
            let brightness_scale = brightness_scale.clone();
            let readout_label = readout_label.clone();
            let anim_running = anim_running.clone();

            glib::timeout_add_local(Duration::from_millis(1000), move || {
                if state.syncing.get() || anim_running.get() {
                    return ControlFlow::Continue;
                }

                let mut changed = false;

                match read_brightness_from_daemon() {
                    Ok(brightness) => {
                        if brightness != state.brightness.get() {
                            state.syncing.set(true);
                            brightness_scale.set_value(brightness as f64);
                            state.brightness.set(brightness);
                            state.syncing.set(false);
                            changed = true;
                        }
                    }
                    Err(err) => {
                        readout_label.set_label(&err);
                    }
                }

                match read_color_from_daemon() {
                    Ok(color) => {
                        if color != state.color.get() {
                            state.syncing.set(true);
                            color_button.set_rgba(&rgba_from_led_color(color));
                            state.color.set(color);
                            state.syncing.set(false);
                            changed = true;
                        }
                    }
                    Err(err) => {
                        readout_label.set_label(&err);
                    }
                }

                if changed {
                    refresh_display(
                        &hero,
                        &brightness_metric,
                        &color_metric,
                        &readout_label,
                        state.brightness.get(),
                        state.color.get(),
                    );
                    preview_area.queue_draw();
                }

                ControlFlow::Continue
            });
        }

        root
    }
}

fn read_brightness_from_daemon() -> Result<u8, String> {
    AcerControlDBus::new()?.keyboard_brightness()
}

fn read_color_from_daemon() -> Result<LedColor, String> {
    let (r, g, b) = AcerControlDBus::new()?.keyboard_color()?;
    Ok(LedColor { r, g, b })
}

fn write_brightness_via_daemon(brightness: u8) -> Result<(), String> {
    AcerControlDBus::new()?.set_keyboard_brightness(brightness)
}

fn write_color_via_daemon(color: LedColor) -> Result<(), String> {
    AcerControlDBus::new()?.set_keyboard_color(color.r, color.g, color.b)
}

fn refresh_display(
    hero: &HeroCard,
    brightness_metric: &MetricCard,
    color_metric: &MetricCard,
    readout_label: &Label,
    brightness: u8,
    color: LedColor,
) {
    let hex = format_color_hex(color);

    hero.set_value(&hex);
    brightness_metric.set_value(&format!("{}/255", brightness));
    color_metric.set_value(&hex);
    update_readout(readout_label, color, brightness);
}

fn rounded_rect_simple(cr: &gtk::cairo::Context, w: f64, h: f64) {
    let r = 10.0_f64.min(h / 2.0);

    cr.new_sub_path();
    cr.arc(w - r, r, r, -PI / 2.0, 0.0);
    cr.arc(w - r, h - r, r, 0.0, PI / 2.0);
    cr.arc(r, h - r, r, PI / 2.0, PI);
    cr.arc(r, r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}

fn led_color_from_rgba(rgba: &RGBA) -> LedColor {
    LedColor {
        r: (rgba.red() * 255.0).round().clamp(0.0, 255.0) as u8,
        g: (rgba.green() * 255.0).round().clamp(0.0, 255.0) as u8,
        b: (rgba.blue() * 255.0).round().clamp(0.0, 255.0) as u8,
    }
}

fn rgba_from_led_color(color: LedColor) -> RGBA {
    RGBA::new(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        1.0,
    )
}

fn format_color_hex(color: LedColor) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

fn update_readout(label: &Label, color: LedColor, brightness: u8) {
    label.set_label(&format!(
        "{}   ·   Brightness: {}/255   ·   RGB: {}, {}, {}",
        format_color_hex(color),
        brightness,
        color.r,
        color.g,
        color.b
    ));
}

fn draw_keyboard_preview(
    cr: &gtk::cairo::Context,
    width: f64,
    height: f64,
    color: LedColor,
    brightness: u8,
) {
    let factor = brightness as f64 / 255.0;
    let preview_r = color.r as f64 / 255.0 * factor;
    let preview_g = color.g as f64 / 255.0 * factor;
    let preview_b = color.b as f64 / 255.0 * factor;

    cr.set_antialias(gtk::cairo::Antialias::Best);
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    cr.set_line_join(gtk::cairo::LineJoin::Round);

    cr.set_source_rgba(0.08, 0.09, 0.10, 1.0);
    rounded_rect(cr, 8.0, 8.0, width - 16.0, height - 16.0, 18.0);
    cr.fill().unwrap();

    cr.set_source_rgba(0.14, 0.15, 0.17, 1.0);
    rounded_rect(cr, 24.0, 28.0, width - 48.0, height - 56.0, 16.0);
    cr.fill().unwrap();

    cr.set_source_rgba(0.20, 0.21, 0.23, 1.0);
    rounded_rect(cr, 36.0, 40.0, width - 72.0, height - 84.0, 12.0);
    cr.fill().unwrap();

    let body_x = 42.0;
    let body_y = 48.0;
    let body_w = width - 84.0;
    let key_h = 26.0;
    let key_gap = 5.0;
    let row_gap = 11.0;

    draw_key_row(
        cr,
        body_x,
        body_y,
        body_w,
        13,
        0.0,
        key_h,
        key_gap,
        preview_r,
        preview_g,
        preview_b,
    );

    draw_key_row(
        cr,
        body_x + 18.0,
        body_y + key_h + row_gap,
        body_w - 36.0,
        12,
        0.0,
        key_h,
        key_gap,
        preview_r,
        preview_g,
        preview_b,
    );

    draw_key_row(
        cr,
        body_x + 36.0,
        body_y + (key_h + row_gap) * 2.0,
        body_w - 72.0,
        11,
        0.0,
        key_h,
        key_gap,
        preview_r,
        preview_g,
        preview_b,
    );

    let spacebar_y = body_y + (key_h + row_gap) * 3.0 + 6.0;
    let spacebar_x = body_x + 80.0;
    let spacebar_w = body_w - 160.0;
    cr.set_source_rgba(preview_r * 0.75, preview_g * 0.75, preview_b * 0.75, 1.0);
    rounded_rect(cr, spacebar_x, spacebar_y, spacebar_w, 24.0, 10.0);
    cr.fill_preserve().unwrap();
    cr.set_source_rgba(0.10, 0.11, 0.13, 1.0);
    cr.set_line_width(1.0);
    cr.stroke().unwrap();

    cr.set_source_rgba(0.60, 0.62, 0.66, 1.0);
    cr.select_font_face(
        "Sans",
        gtk::cairo::FontSlant::Normal,
        gtk::cairo::FontWeight::Bold,
    );
    cr.set_font_size(18.0);
    cr.move_to(44.0, 28.0);
}

fn draw_key_row(
    cr: &gtk::cairo::Context,
    start_x: f64,
    y: f64,
    total_width: f64,
    count: usize,
    row_shift: f64,
    key_h: f64,
    key_gap: f64,
    preview_r: f64,
    preview_g: f64,
    preview_b: f64,
) {
    let key_w = (total_width - (count as f64 - 1.0) * key_gap) / count as f64;
    let x_offset = start_x + row_shift;

    for index in 0..count {
        let x = x_offset + index as f64 * (key_w + key_gap);
        cr.set_source_rgba(preview_r, preview_g, preview_b, 1.0);
        rounded_rect(cr, x, y, key_w, key_h, 5.0);
        cr.fill_preserve().unwrap();
        cr.set_source_rgba(0.10, 0.11, 0.13, 1.0);
        cr.set_line_width(1.0);
        cr.stroke().unwrap();
    }
}

fn rounded_rect(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, (-90.0_f64).to_radians(), 0.0_f64.to_radians());
    cr.arc(x + w - r, y + h - r, r, 0.0_f64.to_radians(), 90.0_f64.to_radians());
    cr.arc(
        x + r,
        y + h - r,
        r,
        90.0_f64.to_radians(),
        180.0_f64.to_radians(),
    );
    cr.arc(x + r, y + r, r, 180.0_f64.to_radians(), 270.0_f64.to_radians());
    cr.close_path();
}
