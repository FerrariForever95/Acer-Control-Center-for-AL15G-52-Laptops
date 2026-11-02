use std::collections::HashMap;
use std::sync::Arc;

use zbus::interface;

use crate::service::AcerService;

pub struct AcerControlDBus {
    service: Arc<AcerService>,
}

impl AcerControlDBus {
    pub fn new(service: Arc<AcerService>) -> Self {
        Self { service }
    }
}

#[interface(name = "io.github.stanlysilas.AcerControl")]
impl AcerControlDBus {
    async fn ping(&self) -> String {
        "pong".into()
    }

    async fn get_keyboard_brightness(
        &self,
    ) -> zbus::fdo::Result<u8> {
        self.service
            .get_keyboard_brightness()
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn set_keyboard_brightness(
        &self,
        value: u8,
    ) -> zbus::fdo::Result<()> {
        self.service
            .set_keyboard_brightness(value)
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn get_keyboard_color(
        &self,
    ) -> zbus::fdo::Result<(u8, u8, u8)> {
        self.service
            .get_keyboard_color()
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn set_keyboard_color(
        &self,
        r: u8,
        g: u8,
        b: u8,
    ) -> zbus::fdo::Result<()> {
        self.service
            .set_keyboard_color(r, g, b)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Pin CPU and GPU fans to fixed duty cycles (percent).
    async fn set_fan_manual(
        &self,
        cpu: u8,
        gpu: u8,
    ) -> zbus::fdo::Result<()> {
        self.service
            .set_fan_manual(cpu, gpu)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Return the fans to firmware control.
    async fn set_fan_auto(&self) -> zbus::fdo::Result<()> {
        self.service
            .set_fan_auto()
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Follow a temperature curve: (temperature °C, percent) points.
    async fn set_fan_curve(
        &self,
        points: Vec<(u8, u8)>,
    ) -> zbus::fdo::Result<()> {
        self.service
            .set_fan_curve(points)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// The last curve that was set (empty if none yet).
    async fn get_fan_curve(&self) -> zbus::fdo::Result<Vec<(u8, u8)>> {
        Ok(self.service.fan_curve())
    }

    /// (mode, cpu_percent, gpu_percent, notice)
    /// mode is one of "auto", "manual", "turbo", "curve".
    async fn get_fan_status(
        &self,
    ) -> zbus::fdo::Result<(String, u8, u8, String)> {
        let s = self.service.fan_status();

        Ok((
            s.mode.as_str().to_string(),
            s.cpu_percent,
            s.gpu_percent,
            s.notice,
        ))
    }

    /// CPU temperature in °C, or -1 when no sensor is available.
    async fn get_cpu_temperature(&self) -> zbus::fdo::Result<i32> {
        Ok(self.service.cpu_temperature().unwrap_or(-1))
    }

    /// Fade through the given colours forever (A -> B -> ... -> A).
    /// `seconds` is the time taken to fade from one colour to the next.
    async fn start_keyboard_animation(
        &self,
        colors: Vec<(u8, u8, u8)>,
        seconds: f64,
    ) -> zbus::fdo::Result<()> {
        self.service
            .start_keyboard_animation(colors, seconds)
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn stop_keyboard_animation(&self) -> zbus::fdo::Result<()> {
        self.service.stop_keyboard_animation();
        Ok(())
    }

    /// (running, colours, seconds per fade)
    async fn get_keyboard_animation(
        &self,
    ) -> zbus::fdo::Result<(bool, Vec<(u8, u8, u8)>, f64)> {
        Ok(self.service.keyboard_animation())
    }

    /// Cycle through every colour; `seconds` is one full trip round the wheel.
    async fn start_keyboard_rainbow(
        &self,
        seconds: f64,
    ) -> zbus::fdo::Result<()> {
        self.service
            .start_keyboard_rainbow(seconds)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Keys flash `pulse` over `base`, fading back over `seconds`.
    async fn start_keyboard_reactive(
        &self,
        base: (u8, u8, u8),
        pulse: (u8, u8, u8),
        seconds: f64,
    ) -> zbus::fdo::Result<()> {
        self.service
            .start_keyboard_reactive(base, pulse, seconds)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// (kind, colours, seconds); kind is "none", "fade", "rainbow" or
    /// "reactive".
    async fn get_keyboard_effect(
        &self,
    ) -> zbus::fdo::Result<(String, Vec<(u8, u8, u8)>, f64)> {
        Ok(self.service.keyboard_effect())
    }

    /// Maximum fans, always (both fans at 100%, never auto-reverted).
    async fn set_fan_turbo(&self) -> zbus::fdo::Result<()> {
        self.service
            .set_fan_manual(100, 100)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Saved profiles as (name, summary).
    async fn list_profiles(
        &self,
    ) -> zbus::fdo::Result<Vec<(String, String)>> {
        Ok(self.service.list_profiles())
    }

    /// Save the current fan and keyboard settings as `name`.
    async fn save_profile(&self, name: String) -> zbus::fdo::Result<()> {
        self.service
            .save_profile(name)
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn apply_profile(&self, name: String) -> zbus::fdo::Result<()> {
        self.service
            .apply_profile(&name)
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn delete_profile(&self, name: String) -> zbus::fdo::Result<()> {
        self.service
            .delete_profile(&name)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Profile applied when the daemon starts (empty string: none).
    async fn get_startup_profile(&self) -> zbus::fdo::Result<String> {
        Ok(self.service.startup_profile())
    }

    async fn set_startup_profile(
        &self,
        name: String,
    ) -> zbus::fdo::Result<()> {
        self.service
            .set_startup_profile(name)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// NVIDIA GPU status as key/value text (see gpu.rs for the keys).
    async fn get_gpu_status(
        &self,
    ) -> zbus::fdo::Result<HashMap<String, String>> {
        // Querying the graphics tool can take a moment; keep it off the
        // D-Bus executor so other calls (fans, lighting) are never held up.
        let service = self.service.clone();

        tokio::task::spawn_blocking(move || service.gpu_status())
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// true: let the kernel suspend an idle GPU; false: keep it awake.
    async fn set_gpu_power_control(
        &self,
        auto: bool,
    ) -> zbus::fdo::Result<()> {
        self.service
            .set_gpu_power_control(auto)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Start a graphics mode change: "integrated", "hybrid" or "nvidia".
    /// Returns immediately; progress is reported in GetGpuStatus.
    async fn set_gpu_mode(&self, mode: String) -> zbus::fdo::Result<()> {
        self.service
            .set_gpu_mode(mode)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Battery samples (unix time, percent, charging) from the last
    /// `window_secs` seconds.
    async fn get_battery_history(
        &self,
        window_secs: u32,
    ) -> zbus::fdo::Result<Vec<(u64, u8, bool)>> {
        Ok(self.service.battery_history(window_secs))
    }
}
