use std::collections::HashMap;
use std::sync::Arc;

use crate::backend::HardwareBackend;
use crate::battery::BatteryHistory;
use crate::fan::{self, FanManager, FanStatus};
use crate::gpu::Gpu;
use crate::lighting::Lighting;
use crate::profiles::{self, Profiles};

pub struct AcerService {
    backend: Arc<dyn HardwareBackend>,
    fans: Arc<FanManager>,
    lighting: Arc<Lighting>,
    battery: Arc<BatteryHistory>,
    profiles: Arc<Profiles>,
    gpu: Arc<Gpu>,
}

impl AcerService {
    pub fn new(
        backend: Arc<dyn HardwareBackend>,
        fans: Arc<FanManager>,
        lighting: Arc<Lighting>,
        battery: Arc<BatteryHistory>,
        profiles: Arc<Profiles>,
        gpu: Arc<Gpu>,
    ) -> Self {
        Self {
            backend,
            fans,
            lighting,
            battery,
            profiles,
            gpu,
        }
    }

    // -----------------------------------------------------------------
    // Profiles
    // -----------------------------------------------------------------

    /// (name, one-line summary) for every saved profile.
    pub fn list_profiles(&self) -> Vec<(String, String)> {
        self.profiles.list()
    }

    /// Save the current fan mode and keyboard lighting under `name`
    /// (replacing a profile with the same name).
    pub fn save_profile(&self, name: String) -> Result<(), String> {
        let name = profiles::clean_name(&name)?;

        let profile = profiles::capture(
            name,
            &self.fans,
            &self.lighting,
            self.backend.as_ref(),
        );

        self.profiles.upsert(profile)
    }

    pub fn apply_profile(&self, name: &str) -> Result<(), String> {
        let profile = self
            .profiles
            .get(name)
            .ok_or_else(|| "No such profile".to_string())?;

        profiles::apply(
            &profile,
            &self.fans,
            &self.lighting,
            self.backend.as_ref(),
        )
    }

    pub fn delete_profile(&self, name: &str) -> Result<(), String> {
        self.profiles.delete(name)
    }

    /// Name of the profile applied at startup, or empty for none.
    pub fn startup_profile(&self) -> String {
        self.profiles.startup().unwrap_or_default()
    }

    /// An empty name clears the startup profile.
    pub fn set_startup_profile(&self, name: String) -> Result<(), String> {
        if name.is_empty() {
            self.profiles.set_startup(None)
        } else {
            self.profiles.set_startup(Some(name))
        }
    }

    /// Apply the startup profile, if one is set. Returns its name.
    pub fn apply_startup_profile(&self) -> Result<Option<String>, String> {
        let Some(name) = self.profiles.startup() else {
            return Ok(None);
        };

        self.apply_profile(&name)?;

        Ok(Some(name))
    }

    // -----------------------------------------------------------------
    // NVIDIA GPU
    // -----------------------------------------------------------------

    pub fn gpu_status(&self) -> HashMap<String, String> {
        self.gpu.status()
    }

    pub fn set_gpu_power_control(&self, auto: bool) -> Result<(), String> {
        self.gpu.set_power_control(auto)
    }

    pub fn set_gpu_mode(&self, mode: String) -> Result<(), String> {
        self.gpu.set_mode(&mode)
    }

    pub fn start_keyboard_rainbow(&self, seconds: f64) -> Result<(), String> {
        self.lighting.start_rainbow(seconds)
    }

    pub fn start_keyboard_reactive(
        &self,
        base: (u8, u8, u8),
        pulse: (u8, u8, u8),
        seconds: f64,
    ) -> Result<(), String> {
        self.lighting.start_reactive(base, pulse, seconds)
    }

    pub fn keyboard_effect(&self) -> (String, Vec<(u8, u8, u8)>, f64) {
        self.lighting.status()
    }

    pub fn battery_history(&self, window_secs: u32) -> Vec<(u64, u8, bool)> {
        self.battery.window(window_secs)
    }

    pub fn start_keyboard_animation(
        &self,
        colors: Vec<(u8, u8, u8)>,
        seconds: f64,
    ) -> Result<(), String> {
        self.lighting.start_fade(colors, seconds)
    }

    pub fn stop_keyboard_animation(&self) {
        self.lighting.stop();
    }

    pub fn keyboard_animation(&self) -> (bool, Vec<(u8, u8, u8)>, f64) {
        let (kind, colors, seconds) = self.lighting.status();

        (kind == "fade", colors, seconds)
    }

    pub fn set_fan_manual(&self, cpu: u8, gpu: u8) -> Result<(), String> {
        self.fans.set_manual(cpu, gpu)
    }

    pub fn set_fan_auto(&self) -> Result<(), String> {
        self.fans.set_auto()
    }

    pub fn set_fan_curve(&self, points: Vec<(u8, u8)>) -> Result<(), String> {
        self.fans.set_curve(points)
    }

    pub fn fan_curve(&self) -> Vec<(u8, u8)> {
        self.fans.curve()
    }

    pub fn fan_status(&self) -> FanStatus {
        self.fans.status()
    }

    pub fn cpu_temperature(&self) -> Option<i32> {
        fan::cpu_temperature()
    }

    pub fn get_keyboard_brightness(
        &self,
    ) -> Result<u8, String> {
        self.backend.get_keyboard_brightness()
    }

    pub fn set_keyboard_brightness(
        &self,
        value: u8,
    ) -> Result<(), String> {
        self.backend.set_keyboard_brightness(value)
    }

    pub fn get_keyboard_color(
        &self,
    ) -> Result<(u8, u8, u8), String> {
        self.backend.get_keyboard_color()
    }

    pub fn set_keyboard_color(
        &self,
        r: u8,
        g: u8,
        b: u8,
    ) -> Result<(), String> {
        // Picking a fixed colour ends any running transition.
        self.lighting.stop();
        self.backend.set_keyboard_color(r, g, b)
    }
}
