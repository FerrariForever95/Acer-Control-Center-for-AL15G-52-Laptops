use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar, NavigationPage, NavigationSplitView};
use gtk::{Box, Orientation, PolicyType, ScrolledWindow, Stack};

use crate::battery::BatteryPage;
use crate::cooling::CoolingPage;
use crate::dashboard::Dashboard;
use crate::gpu::GpuPage;
use crate::profiles::ProfilesPage;
use crate::rgb::RgbPage;
use crate::settings::SettingsPage;
use crate::widgets::sidebar::Sidebar;

pub struct MainWindow {
    window: ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let stack = Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();

        stack.add_css_class("content-stack");

        let dashboard = Dashboard::new();

        let dashboard_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&dashboard.root)
            .build();

        let rgb_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&RgbPage::new())
            .build();
            
        let cooling_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&CoolingPage::new())
            .build();
            
        let battery_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&BatteryPage::new())
            .build();

        let gpu_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&GpuPage::new())
            .build();

        let profiles_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&ProfilesPage::new())
            .build();

        let settings_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&SettingsPage::new())
            .build();
            

        // The ids must match widgets::sidebar::PAGES.
        stack.add_titled(&dashboard_scroll, Some("dashboard"), "Dashboard");
        stack.add_titled(&profiles_scroll, Some("profiles"), "Profiles");
        stack.add_titled(&cooling_scroll, Some("cooling"), "Cooling");
        stack.add_titled(&rgb_scroll, Some("rgb"), "Keyboard");
        stack.add_titled(&gpu_scroll, Some("gpu"), "GPU");
        stack.add_titled(&battery_scroll, Some("battery"), "Battery");
        stack.add_titled(&settings_scroll, Some("settings"), "Settings");

        let sidebar = Sidebar::new(&stack);

        let sidebar_page = NavigationPage::builder()
            .title("Navigation")
            .child(&sidebar)
            .build();

        let content = Box::builder()
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();

        let header = HeaderBar::builder()
            .show_title(false)
            .show_end_title_buttons(true)
            .build();

        header.add_css_class("main-headerbar");

        content.append(&header);
        content.append(&stack);

        let navigation = NavigationSplitView::builder()
            .sidebar(&sidebar_page)
            .content(
                &NavigationPage::builder()
                    .title("Acer Control Center")
                    .child(&content)
                    .build(),
            )
            .min_sidebar_width(210.0)
            .max_sidebar_width(250.0)
            .show_content(true)
            .collapsed(false)
            .build();

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Acer Control Center")
            .default_width(1180)
            .default_height(780)
            .width_request(900)
            .height_request(600)
            .content(&navigation)
            .build();

        window.add_css_class("main-window");

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
