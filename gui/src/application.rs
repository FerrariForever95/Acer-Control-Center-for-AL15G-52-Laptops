use adw::prelude::*;
use adw::{Application, StyleManager};
use gtk::gio;
use gtk::{CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};

use crate::window::MainWindow;

pub struct AcerControlCenterApplication {
    app: Application,
}

impl AcerControlCenterApplication {
    pub fn new() -> Self {
        let app = Application::builder()
            .application_id("io.github.stanlysilas.AcerControlCenter")
            .flags(gio::ApplicationFlags::HANDLES_OPEN)
            .build();

        let instance = Self { app };

        instance.setup_startup();
        instance.setup_activate();

        instance
    }

    pub fn run(self) -> glib::ExitCode {
        self.app.run()
    }

    fn setup_startup(&self) {
        self.app.connect_startup(|_| {
            gtk::Window::set_default_icon_name(
                "io.github.stanlysilas.AcerControlCenter",
            );
            setup_theme();
            load_css();

            log::info!("Acer Control Center initialized.");
        });
    }

    fn setup_activate(&self) {
        self.app.connect_activate(|app| {
            let window = MainWindow::new(app);

            window.present();

            log::info!("Main window presented.");
        });
    }
}

fn setup_theme() {
    let manager = StyleManager::default();
    manager.set_color_scheme(adw::ColorScheme::PreferDark);
}

fn load_css() {
    let provider = CssProvider::new();

    // Embedded so the installed binary is styled no matter where it is launched from.
    provider.load_from_string(include_str!("../resources/css/style.css"));

    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("No display available."),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    log::info!("Stylesheet loaded.");
}
