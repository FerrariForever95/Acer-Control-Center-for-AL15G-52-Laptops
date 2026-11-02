//! NVIDIA GPU control for hybrid-graphics laptops.
//!
//! What can be done safely from here:
//!
//! * Read status: is the GPU visible, which driver owns it, is it asleep.
//! * Runtime power control: write `auto` to the PCI power/control file so the
//!   kernel may put an idle GPU into its D3 low-power state (and `on` to keep
//!   it awake). Takes effect immediately, resets on reboot.
//! * Graphics mode (Integrated / Hybrid / NVIDIA): delegated to EnvyControl,
//!   which is the tool that knows how to do it per distribution. Switching
//!   rewrites system configuration and needs a reboot, so it runs in the
//!   background and the result is reported through the status.
//!
//! Nothing here unloads drivers or removes PCI devices by itself.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PCI_DEVICES: &str = "/sys/bus/pci/devices";
const NVIDIA_VENDOR: &str = "0x10de";
const MODE_CACHE_FOR: Duration = Duration::from_secs(30);

pub const MODES: [&str; 3] = ["integrated", "hybrid", "nvidia"];

struct ModeCache {
    at: Instant,
    switch_state: String,
    mode: String,
}

pub struct Gpu {
    /// "idle", "running:<mode>", "done:<mode>" or "failed:<message>".
    switch_state: Arc<Mutex<String>>,
    mode_cache: Mutex<Option<ModeCache>>,
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn which(name: &str) -> bool {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();

    for extra in ["/usr/local/sbin", "/usr/local/bin", "/usr/sbin", "/usr/bin"]
    {
        dirs.push(PathBuf::from(extra));
    }

    dirs.iter().any(|d| d.join(name).is_file())
}

/// The graphics-switching tool installed on this system, if any.
fn detect_tool() -> &'static str {
    if which("envycontrol") {
        "envycontrol"
    } else if which("prime-select") {
        "prime-select"
    } else {
        "none"
    }
}

fn query_mode(tool: &str) -> String {
    let output = match tool {
        "envycontrol" => Command::new("envycontrol").arg("--query").output(),
        "prime-select" => Command::new("prime-select").arg("query").output(),
        _ => return String::new(),
    };

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .trim()
            .to_lowercase(),
        _ => String::new(),
    }
}

/// The NVIDIA display/3D controller (PCI class 0x03xx), if it is visible.
fn find_display_device() -> Option<PathBuf> {
    for entry in fs::read_dir(PCI_DEVICES).ok()?.flatten() {
        let dir = entry.path();

        if read_trim(&dir.join("vendor")).as_deref() != Some(NVIDIA_VENDOR) {
            continue;
        }

        let class = read_trim(&dir.join("class")).unwrap_or_default();

        if class.starts_with("0x03") {
            return Some(dir);
        }
    }

    None
}

/// Value of the first `Key:` line in a /proc text file.
fn proc_value(path: &str, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;

    text.lines()
        .find_map(|l| l.strip_prefix(key))
        .map(|v| v.trim().to_string())
}

impl Gpu {
    pub fn new() -> Self {
        Self {
            switch_state: Arc::new(Mutex::new("idle".into())),
            mode_cache: Mutex::new(None),
        }
    }

    fn current_mode(&self, tool: &str, switch_state: &str) -> String {
        let mut cache = self.mode_cache.lock().unwrap();

        if let Some(c) = cache.as_ref() {
            if c.at.elapsed() < MODE_CACHE_FOR && c.switch_state == switch_state
            {
                return c.mode.clone();
            }
        }

        let mode = query_mode(tool);

        *cache = Some(ModeCache {
            at: Instant::now(),
            switch_state: switch_state.to_string(),
            mode: mode.clone(),
        });

        mode
    }

    /// Everything the GUI shows, as plain key/value text.
    pub fn status(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();

        let tool = detect_tool();
        let switch_state = self.switch_state.lock().unwrap().clone();

        m.insert("tool".into(), tool.to_string());
        m.insert("mode".into(), self.current_mode(tool, &switch_state));
        m.insert("switch".into(), switch_state);

        let Some(dir) = find_display_device() else {
            m.insert("present".into(), "no".into());
            return m;
        };

        m.insert("present".into(), "yes".into());

        let address = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let device_id = read_trim(&dir.join("device")).unwrap_or_default();

        let model = proc_value(
            &format!("/proc/driver/nvidia/gpus/{}/information", address),
            "Model:",
        )
        .unwrap_or_else(|| {
            format!("NVIDIA GPU (10de:{})", device_id.trim_start_matches("0x"))
        });

        let driver = fs::read_link(dir.join("driver"))
            .ok()
            .and_then(|p| {
                p.file_name().map(|n| n.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "none".into());

        m.insert("address".into(), address.clone());
        m.insert("model".into(), model);
        m.insert("driver".into(), driver);

        m.insert(
            "runtime_status".into(),
            read_trim(&dir.join("power/runtime_status")).unwrap_or_default(),
        );
        m.insert(
            "power_control".into(),
            read_trim(&dir.join("power/control")).unwrap_or_default(),
        );

        // Only present while the proprietary driver is loaded.
        m.insert(
            "rtd3".into(),
            proc_value(
                &format!("/proc/driver/nvidia/gpus/{}/power", address),
                "Runtime D3 status:",
            )
            .unwrap_or_default(),
        );
        m.insert(
            "dynamic_pm".into(),
            proc_value(
                "/proc/driver/nvidia/params",
                "DynamicPowerManagement:",
            )
            .unwrap_or_default(),
        );

        m
    }

    /// `auto`: the kernel may suspend an idle GPU. `on`: keep it awake.
    /// Applies to every PCI function of the GPU's slot (graphics, audio...).
    pub fn set_power_control(&self, auto: bool) -> Result<(), String> {
        let dev = find_display_device().ok_or_else(|| {
            "No NVIDIA GPU is visible (it may be switched off by Integrated mode)"
                .to_string()
        })?;

        let name = dev
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let slot = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(&name);
        let prefix = format!("{}.", slot);
        let value = if auto { "auto" } else { "on" };

        let mut wrote = false;
        let mut last_error: Option<String> = None;

        for entry in fs::read_dir(PCI_DEVICES)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let fname = entry.file_name().to_string_lossy().to_string();

            if !fname.starts_with(&prefix) {
                continue;
            }

            let path = entry.path();

            if read_trim(&path.join("vendor")).as_deref() != Some(NVIDIA_VENDOR)
            {
                continue;
            }

            match fs::write(path.join("power/control"), value) {
                Ok(()) => wrote = true,
                Err(e) => last_error = Some(e.to_string()),
            }
        }

        if wrote {
            Ok(())
        } else {
            Err(last_error
                .unwrap_or_else(|| "GPU power control is not available".into()))
        }
    }

    /// Start a graphics mode change in the background. Needs EnvyControl.
    pub fn set_mode(&self, mode: &str) -> Result<(), String> {
        if !MODES.contains(&mode) {
            return Err("Unknown graphics mode".into());
        }

        if detect_tool() != "envycontrol" {
            return Err(
                "Switching modes needs EnvyControl, which is not installed"
                    .into(),
            );
        }

        {
            let mut state = self.switch_state.lock().unwrap();

            if state.starts_with("running") {
                return Err("A mode change is already in progress".into());
            }

            *state = format!("running:{}", mode);
        }

        let state = self.switch_state.clone();
        let mode = mode.to_string();

        std::thread::spawn(move || {
            let result = Command::new("envycontrol").args(["-s", &mode]).output();

            let message = match result {
                Ok(o) if o.status.success() => format!("done:{}", mode),
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    let stdout = String::from_utf8_lossy(&o.stdout);

                    let detail = stderr
                        .lines()
                        .chain(stdout.lines())
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .last()
                        .unwrap_or("EnvyControl reported an error")
                        .to_string();

                    format!("failed:{}", detail)
                }
                Err(e) => format!("failed:{}", e),
            };

            *state.lock().unwrap() = message;
        });

        Ok(())
    }
}
