mod application;
mod window;

mod dashboard;
mod rgb;
mod cooling;
mod battery;
mod settings;
mod profiles;
mod gpu;

mod libacercontrol;

mod widgets;

fn main() -> glib::ExitCode {
    env_logger::init();

    adw::init().expect("Failed to initialize Libadwaita");

    application::AcerControlCenterApplication::new().run()
}
