//! Saved profiles: a named snapshot of the fan mode and keyboard lighting.
//!
//! Profiles are captured from the live state, so the GUI never has to
//! describe a profile itself: it just says "save what I have now as X" and
//! "apply X". One profile can be marked to apply when the daemon starts.
//!
//! Storage is a tiny hand-readable text file, for example:
//!
//! ```text
//! startup=Gaming
//!
//! [Gaming]
//! fan=turbo
//! keyboard=rainbow:4
//! brightness=3
//! ```

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::backend::HardwareBackend;
use crate::fan::{FanManager, FanMode};
use crate::lighting::Lighting;

const DIR: &str = "/var/lib/acer-control-center";
const FILE_NAME: &str = "profiles.conf";

pub const MAX_PROFILES: usize = 16;
pub const MAX_NAME_CHARS: usize = 32;

type Rgb = (u8, u8, u8);

#[derive(Clone, Debug, PartialEq)]
pub enum FanSetting {
    Auto,
    Turbo,
    Manual { cpu: u8, gpu: u8 },
    Curve(Vec<(u8, u8)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeyboardSetting {
    Static(Rgb),
    Fade { colors: Vec<Rgb>, seconds: f64 },
    Rainbow { seconds: f64 },
    Reactive { base: Rgb, pulse: Rgb, seconds: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    pub fan: FanSetting,
    /// None leaves the keyboard lighting as it is.
    pub keyboard: Option<KeyboardSetting>,
    pub brightness: Option<u8>,
}

// ---------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------

/// Trim and validate a profile name. Only characters that are safe in the
/// storage file and easy to read in the UI are allowed.
pub fn clean_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();

    if name.is_empty() {
        return Err("Enter a name for the profile".into());
    }

    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "Profile names can be at most {} characters",
            MAX_NAME_CHARS
        ));
    }

    let ok = name.chars().all(|c| {
        c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')')
    });

    if !ok {
        return Err(
            "Use letters, numbers, spaces and - _ . ( ) in profile names"
                .into(),
        );
    }

    Ok(name.to_string())
}

// ---------------------------------------------------------------------
// Text format
// ---------------------------------------------------------------------

fn fmt_rgb(c: Rgb) -> String {
    format!("{},{},{}", c.0, c.1, c.2)
}

fn parse_rgb(s: &str) -> Option<Rgb> {
    let mut it = s.split(',').map(|v| v.trim().parse::<u8>().ok());

    let r = it.next()??;
    let g = it.next()??;
    let b = it.next()??;

    if it.next().is_some() {
        return None;
    }

    Some((r, g, b))
}

fn fan_to_text(fan: &FanSetting) -> String {
    match fan {
        FanSetting::Auto => "auto".into(),
        FanSetting::Turbo => "turbo".into(),
        FanSetting::Manual { cpu, gpu } => format!("manual:{},{}", cpu, gpu),
        FanSetting::Curve(points) => {
            let list: Vec<String> = points
                .iter()
                .map(|(t, p)| format!("{},{}", t, p))
                .collect();

            format!("curve:{}", list.join(";"))
        }
    }
}

fn fan_from_text(text: &str) -> Option<FanSetting> {
    let (kind, rest) = match text.split_once(':') {
        Some((k, r)) => (k, r),
        None => (text, ""),
    };

    match kind {
        "auto" => Some(FanSetting::Auto),
        "turbo" => Some(FanSetting::Turbo),
        "manual" => {
            let (cpu, gpu) = rest.split_once(',')?;

            Some(FanSetting::Manual {
                cpu: cpu.trim().parse().ok()?,
                gpu: gpu.trim().parse().ok()?,
            })
        }
        "curve" => {
            let mut points = Vec::new();

            for item in rest.split(';') {
                let (t, p) = item.split_once(',')?;
                points.push((t.trim().parse().ok()?, p.trim().parse().ok()?));
            }

            Some(FanSetting::Curve(points))
        }
        _ => None,
    }
}

fn keyboard_to_text(kb: &KeyboardSetting) -> String {
    match kb {
        KeyboardSetting::Static(c) => format!("static:{}", fmt_rgb(*c)),
        KeyboardSetting::Fade { colors, seconds } => {
            let list: Vec<String> =
                colors.iter().map(|c| fmt_rgb(*c)).collect();

            format!("fade:{}:{}", seconds, list.join(";"))
        }
        KeyboardSetting::Rainbow { seconds } => {
            format!("rainbow:{}", seconds)
        }
        KeyboardSetting::Reactive {
            base,
            pulse,
            seconds,
        } => format!(
            "reactive:{}:{}:{}",
            seconds,
            fmt_rgb(*base),
            fmt_rgb(*pulse)
        ),
    }
}

fn keyboard_from_text(text: &str) -> Option<KeyboardSetting> {
    let mut parts = text.splitn(4, ':');
    let kind = parts.next()?;

    match kind {
        "static" => Some(KeyboardSetting::Static(parse_rgb(parts.next()?)?)),
        "rainbow" => Some(KeyboardSetting::Rainbow {
            seconds: parts.next()?.trim().parse().ok()?,
        }),
        "fade" => {
            let seconds = parts.next()?.trim().parse().ok()?;
            let mut colors = Vec::new();

            for c in parts.next()?.split(';') {
                colors.push(parse_rgb(c)?);
            }

            Some(KeyboardSetting::Fade { colors, seconds })
        }
        "reactive" => {
            let seconds = parts.next()?.trim().parse().ok()?;
            let base = parse_rgb(parts.next()?)?;
            let pulse = parse_rgb(parts.next()?)?;

            Some(KeyboardSetting::Reactive {
                base,
                pulse,
                seconds,
            })
        }
        _ => None,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Store {
    profiles: Vec<Profile>,
    startup: Option<String>,
}

fn store_to_text(store: &Store) -> String {
    let mut out = String::new();

    if let Some(name) = &store.startup {
        out.push_str(&format!("startup={}\n", name));
    }

    for p in &store.profiles {
        out.push_str(&format!("\n[{}]\n", p.name));
        out.push_str(&format!("fan={}\n", fan_to_text(&p.fan)));

        if let Some(kb) = &p.keyboard {
            out.push_str(&format!("keyboard={}\n", keyboard_to_text(kb)));
        }

        if let Some(b) = p.brightness {
            out.push_str(&format!("brightness={}\n", b));
        }
    }

    out
}

fn store_from_text(text: &str) -> Store {
    let mut store = Store::default();
    let mut current: Option<Profile> = None;

    let finish = |store: &mut Store, current: &mut Option<Profile>| {
        if let Some(p) = current.take() {
            if store.profiles.len() < MAX_PROFILES
                && !store.profiles.iter().any(|q| q.name == p.name)
            {
                store.profiles.push(p);
            }
        }
    };

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            finish(&mut store, &mut current);

            current = clean_name(name).ok().map(|name| Profile {
                name,
                fan: FanSetting::Auto,
                keyboard: None,
                brightness: None,
            });

            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let (key, value) = (key.trim(), value.trim());

        match (&mut current, key) {
            (None, "startup") => store.startup = Some(value.to_string()),
            (Some(p), "fan") => {
                if let Some(f) = fan_from_text(value) {
                    p.fan = f;
                }
            }
            (Some(p), "keyboard") => p.keyboard = keyboard_from_text(value),
            (Some(p), "brightness") => p.brightness = value.parse().ok(),
            _ => {}
        }
    }

    finish(&mut store, &mut current);

    // A startup profile that no longer exists is ignored.
    if let Some(name) = &store.startup {
        if !store.profiles.iter().any(|p| &p.name == name) {
            store.startup = None;
        }
    }

    store
}

// ---------------------------------------------------------------------
// Summaries
// ---------------------------------------------------------------------

fn fan_summary(fan: &FanSetting) -> String {
    match fan {
        FanSetting::Auto => "Auto".into(),
        FanSetting::Turbo => "Turbo".into(),
        FanSetting::Manual { cpu, gpu } if cpu == gpu => {
            format!("Fixed {}%", cpu)
        }
        FanSetting::Manual { cpu, gpu } => {
            format!("CPU {}% / GPU {}%", cpu, gpu)
        }
        FanSetting::Curve(_) => "Custom curve".into(),
    }
}

fn keyboard_summary(kb: &Option<KeyboardSetting>) -> String {
    match kb {
        None => "unchanged".into(),
        Some(KeyboardSetting::Static(c)) => {
            format!("Static #{:02X}{:02X}{:02X}", c.0, c.1, c.2)
        }
        Some(KeyboardSetting::Fade { colors, .. }) => {
            format!("Fade ({} colours)", colors.len())
        }
        Some(KeyboardSetting::Rainbow { .. }) => "Rainbow".into(),
        Some(KeyboardSetting::Reactive { .. }) => "React to keys".into(),
    }
}

pub fn summary(p: &Profile) -> String {
    format!(
        "Fans: {}  ·  Keyboard: {}",
        fan_summary(&p.fan),
        keyboard_summary(&p.keyboard)
    )
}

// ---------------------------------------------------------------------
// Store with persistence
// ---------------------------------------------------------------------

pub struct Profiles {
    path: PathBuf,
    store: Mutex<Store>,
}

impl Profiles {
    pub fn load() -> Self {
        Self::load_from(PathBuf::from(DIR).join(FILE_NAME))
    }

    pub fn load_from(path: PathBuf) -> Self {
        let store = fs::read_to_string(&path)
            .map(|t| store_from_text(&t))
            .unwrap_or_default();

        Self {
            path,
            store: Mutex::new(store),
        }
    }

    fn persist(&self, store: &Store) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Cannot create {}: {}", dir.display(), e))?;
        }

        // Write to a temporary file first so a crash can't leave a
        // half-written profile list behind.
        let tmp = self.path.with_extension("tmp");

        fs::write(&tmp, store_to_text(store))
            .map_err(|e| format!("Cannot save profiles: {}", e))?;

        fs::rename(&tmp, &self.path)
            .map_err(|e| format!("Cannot save profiles: {}", e))
    }

    /// (name, one-line summary) for every profile, in saved order.
    pub fn list(&self) -> Vec<(String, String)> {
        self.store
            .lock()
            .unwrap()
            .profiles
            .iter()
            .map(|p| (p.name.clone(), summary(p)))
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<Profile> {
        self.store
            .lock()
            .unwrap()
            .profiles
            .iter()
            .find(|p| p.name == name)
            .cloned()
    }

    /// Add a profile, or replace the one with the same name.
    pub fn upsert(&self, profile: Profile) -> Result<(), String> {
        let mut guard = self.store.lock().unwrap();
        let mut next = guard.clone();

        match next.profiles.iter_mut().find(|p| p.name == profile.name) {
            Some(existing) => *existing = profile,
            None => {
                if next.profiles.len() >= MAX_PROFILES {
                    return Err(format!(
                        "You can keep at most {} profiles",
                        MAX_PROFILES
                    ));
                }

                next.profiles.push(profile);
            }
        }

        self.persist(&next)?;
        *guard = next;

        Ok(())
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        let mut guard = self.store.lock().unwrap();
        let mut next = guard.clone();

        let before = next.profiles.len();
        next.profiles.retain(|p| p.name != name);

        if next.profiles.len() == before {
            return Err("No such profile".into());
        }

        if next.startup.as_deref() == Some(name) {
            next.startup = None;
        }

        self.persist(&next)?;
        *guard = next;

        Ok(())
    }

    pub fn startup(&self) -> Option<String> {
        self.store.lock().unwrap().startup.clone()
    }

    pub fn set_startup(&self, name: Option<String>) -> Result<(), String> {
        let mut guard = self.store.lock().unwrap();
        let mut next = guard.clone();

        if let Some(n) = &name {
            if !next.profiles.iter().any(|p| &p.name == n) {
                return Err("No such profile".into());
            }
        }

        next.startup = name;

        self.persist(&next)?;
        *guard = next;

        Ok(())
    }
}

// ---------------------------------------------------------------------
// Capture / apply
// ---------------------------------------------------------------------

/// Snapshot the current fan mode and keyboard lighting.
pub fn capture(
    name: String,
    fans: &FanManager,
    lighting: &Lighting,
    backend: &dyn HardwareBackend,
) -> Profile {
    let status = fans.status();

    let fan = match status.mode {
        FanMode::Auto => FanSetting::Auto,
        FanMode::Turbo => FanSetting::Turbo,
        FanMode::Manual => FanSetting::Manual {
            cpu: status.cpu_percent,
            gpu: status.gpu_percent,
        },
        FanMode::Curve => {
            let curve = fans.curve();

            if curve.is_empty() {
                FanSetting::Auto
            } else {
                FanSetting::Curve(curve)
            }
        }
    };

    let (kind, colors, seconds) = lighting.status();

    let keyboard = match kind.as_str() {
        "fade" => Some(KeyboardSetting::Fade { colors, seconds }),
        "rainbow" => Some(KeyboardSetting::Rainbow { seconds }),
        "reactive" if colors.len() >= 2 => Some(KeyboardSetting::Reactive {
            base: colors[0],
            pulse: colors[1],
            seconds,
        }),
        _ => backend.get_keyboard_color().ok().map(KeyboardSetting::Static),
    };

    Profile {
        name,
        fan,
        keyboard,
        brightness: backend.get_keyboard_brightness().ok(),
    }
}

/// Apply a profile. Every part is attempted even if another part fails;
/// the error lists whatever could not be applied.
pub fn apply(
    profile: &Profile,
    fans: &FanManager,
    lighting: &Lighting,
    backend: &dyn HardwareBackend,
) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    if let Some(b) = profile.brightness {
        if let Err(e) = backend.set_keyboard_brightness(b) {
            errors.push(format!("Keyboard brightness: {}", e));
        }
    }

    if let Some(kb) = &profile.keyboard {
        let result = match kb {
            KeyboardSetting::Static(c) => {
                // A fixed colour ends any running effect.
                lighting.stop();
                backend.set_keyboard_color(c.0, c.1, c.2)
            }
            KeyboardSetting::Fade { colors, seconds } => {
                lighting.start_fade(colors.clone(), *seconds)
            }
            KeyboardSetting::Rainbow { seconds } => {
                lighting.start_rainbow(*seconds)
            }
            KeyboardSetting::Reactive {
                base,
                pulse,
                seconds,
            } => lighting.start_reactive(*base, *pulse, *seconds),
        };

        if let Err(e) = result {
            errors.push(format!("Keyboard lighting: {}", e));
        }
    }

    let fan_result = match &profile.fan {
        FanSetting::Auto => fans.set_auto(),
        FanSetting::Turbo => fans.set_turbo(),
        FanSetting::Manual { cpu, gpu } => fans.set_manual(*cpu, *gpu),
        FanSetting::Curve(points) => fans.set_curve(points.clone()),
    };

    if let Err(e) = fan_result {
        errors.push(format!("Fans: {}", e));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
