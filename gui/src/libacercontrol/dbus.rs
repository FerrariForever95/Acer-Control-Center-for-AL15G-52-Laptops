use std::collections::HashMap;

use zbus::blocking::{Connection, Proxy};

const SERVICE: &str = "io.github.stanlysilas.AcerControl";
const PATH: &str = "/io/github/stanlysilas/AcerControl";
const INTERFACE: &str = "io.github.stanlysilas.AcerControl";

pub struct AcerControlDBus {
    connection: Connection,
}

impl AcerControlDBus {
    pub fn new() -> Result<Self, String> {
        let connection = Connection::system()
            .map_err(|e| e.to_string())?;

        Ok(Self { connection })
    }

    fn proxy(&self) -> Result<Proxy<'_>, String> {
        Proxy::new(
            &self.connection,
            SERVICE,
            PATH,
            INTERFACE,
        )
        .map_err(|e| e.to_string())
    }

    pub fn ping(&self) -> Result<String, String> {
        let proxy = self.proxy()?;

        proxy
            .call("Ping", &())
            .map_err(|e| e.to_string())
    }

    pub fn keyboard_brightness(&self) -> Result<u8, String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetKeyboardBrightness", &())
            .map_err(|e| e.to_string())
    }

    pub fn set_keyboard_brightness(&self, brightness: u8) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("SetKeyboardBrightness", &(brightness))
            .map_err(|e| e.to_string())
    }

    pub fn keyboard_color(&self) -> Result<(u8, u8, u8), String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetKeyboardColor", &())
            .map_err(|e| e.to_string())
    }

    pub fn set_keyboard_color(
        &self,
        red: u8,
        green: u8,
        blue: u8,
    ) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>(
                "SetKeyboardColor",
                &(red, green, blue),
            )
            .map_err(|e| e.to_string())
    }

    pub fn set_fan_manual(&self, cpu: u8, gpu: u8) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("SetFanManual", &(cpu, gpu))
            .map_err(|e| e.to_string())
    }

    pub fn set_fan_auto(&self) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("SetFanAuto", &())
            .map_err(|e| e.to_string())
    }

    pub fn set_fan_curve(&self, points: Vec<(u8, u8)>) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("SetFanCurve", &(points,))
            .map_err(|e| e.to_string())
    }

    pub fn fan_curve(&self) -> Result<Vec<(u8, u8)>, String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetFanCurve", &())
            .map_err(|e| e.to_string())
    }

    /// (mode, cpu_percent, gpu_percent, notice); mode is "auto", "manual",
    /// "turbo" or "curve".
    pub fn fan_status(&self) -> Result<(String, u8, u8, String), String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetFanStatus", &())
            .map_err(|e| e.to_string())
    }

    /// CPU temperature in °C, or None when the daemon has no sensor.
    pub fn cpu_temperature(&self) -> Result<Option<i32>, String> {
        let proxy = self.proxy()?;

        let t: i32 = proxy
            .call("GetCpuTemperature", &())
            .map_err(|e| e.to_string())?;

        Ok(if t < 0 { None } else { Some(t) })
    }

    /// Fade through `colors` forever; `seconds` is the time per fade.
    pub fn start_keyboard_animation(
        &self,
        colors: Vec<(u8, u8, u8)>,
        seconds: f64,
    ) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("StartKeyboardAnimation", &(colors, seconds))
            .map_err(|e| e.to_string())
    }

    pub fn stop_keyboard_animation(&self) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("StopKeyboardAnimation", &())
            .map_err(|e| e.to_string())
    }

    /// (running, colours, seconds per fade)
    pub fn keyboard_animation(
        &self,
    ) -> Result<(bool, Vec<(u8, u8, u8)>, f64), String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetKeyboardAnimation", &())
            .map_err(|e| e.to_string())
    }

    pub fn start_keyboard_rainbow(&self, seconds: f64) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("StartKeyboardRainbow", &(seconds,))
            .map_err(|e| e.to_string())
    }

    pub fn start_keyboard_reactive(
        &self,
        base: (u8, u8, u8),
        pulse: (u8, u8, u8),
        seconds: f64,
    ) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("StartKeyboardReactive", &(base, pulse, seconds))
            .map_err(|e| e.to_string())
    }

    /// (kind, colours, seconds); kind is "none", "fade", "rainbow" or
    /// "reactive" (colours are then [base, pulse]).
    pub fn keyboard_effect(
        &self,
    ) -> Result<(String, Vec<(u8, u8, u8)>, f64), String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetKeyboardEffect", &())
            .map_err(|e| e.to_string())
    }

    /// (unix time, percent, charging) samples from the last `window_secs`.
    pub fn battery_history(
        &self,
        window_secs: u32,
    ) -> Result<Vec<(u64, u8, bool)>, String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetBatteryHistory", &(window_secs,))
            .map_err(|e| e.to_string())
    }

    // -----------------------------------------------------------------
    // Profiles
    // -----------------------------------------------------------------

    /// (name, one-line summary) for every saved profile.
    pub fn list_profiles(&self) -> Result<Vec<(String, String)>, String> {
        let proxy = self.proxy()?;

        proxy
            .call("ListProfiles", &())
            .map_err(|e| e.to_string())
    }

    /// Save the daemon's current fan and keyboard settings as `name`.
    pub fn save_profile(&self, name: &str) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("SaveProfile", &(name,))
            .map_err(|e| e.to_string())
    }

    pub fn apply_profile(&self, name: &str) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("ApplyProfile", &(name,))
            .map_err(|e| e.to_string())
    }

    pub fn delete_profile(&self, name: &str) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("DeleteProfile", &(name,))
            .map_err(|e| e.to_string())
    }

    /// Name of the profile applied at startup; empty when there is none.
    pub fn startup_profile(&self) -> Result<String, String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetStartupProfile", &())
            .map_err(|e| e.to_string())
    }

    /// An empty name clears the startup profile.
    pub fn set_startup_profile(&self, name: &str) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("SetStartupProfile", &(name,))
            .map_err(|e| e.to_string())
    }

    // -----------------------------------------------------------------
    // NVIDIA GPU
    // -----------------------------------------------------------------

    /// Key/value status text; see daemon/src/gpu.rs for the keys.
    pub fn gpu_status(&self) -> Result<HashMap<String, String>, String> {
        let proxy = self.proxy()?;

        proxy
            .call("GetGpuStatus", &())
            .map_err(|e| e.to_string())
    }

    /// true lets the kernel suspend an idle GPU; false keeps it awake.
    pub fn set_gpu_power_control(&self, auto: bool) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("SetGpuPowerControl", &(auto,))
            .map_err(|e| e.to_string())
    }

    /// Start a graphics mode change: "integrated", "hybrid" or "nvidia".
    pub fn set_gpu_mode(&self, mode: &str) -> Result<(), String> {
        let proxy = self.proxy()?;

        proxy
            .call::<_, _, ()>("SetGpuMode", &(mode,))
            .map_err(|e| e.to_string())
    }
}
