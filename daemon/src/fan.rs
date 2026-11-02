//! Manual fan control with safety rails.
//!
//! The daemon is the only component allowed to *write* fan speeds. The GUI
//! asks over D-Bus, and everything here is designed so that a crashed GUI,
//! a stopped daemon, or a hot CPU always ends with the firmware back in
//! charge of the fans.
//!
//! Exception: Turbo. Turbo means "maximum fans, always". Once the user asks
//! for it, nothing but the user (choosing another mode) may take it away:
//! the thermal watchdog never reverts it, and it is periodically re-asserted
//! in case firmware or a resume from sleep resets the fans.

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use crate::ffi;

/// Lowest duty cycle a user may request. Fans never get switched off.
pub const MIN_PERCENT: u8 = 20;
pub const MAX_PERCENT: u8 = 100;

/// If the CPU reaches this temperature while in manual mode the daemon
/// hands control back to the firmware.
pub const CRITICAL_TEMP_C: i32 = 90;

/// Curve limits: a small, always-sane set of (temperature °C, percent).
pub const CURVE_MIN_POINTS: usize = 2;
pub const CURVE_MAX_POINTS: usize = 8;

/// Turbo, fixed speeds and curves are re-written to the hardware every
/// this many watchdog ticks.
const TURBO_REASSERT_TICKS: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FanMode {
    /// Firmware controls the fans.
    #[default]
    Auto,
    /// Fixed speeds chosen by the user.
    Manual,
    /// Both fans at 100%, never overridden by the daemon.
    Turbo,
    /// Speed follows CPU temperature along a user-defined curve.
    Curve,
}

impl FanMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            FanMode::Auto => "auto",
            FanMode::Manual => "manual",
            FanMode::Turbo => "turbo",
            FanMode::Curve => "curve",
        }
    }
}

#[derive(Clone, Debug, Default)]
struct State {
    mode: FanMode,
    cpu_percent: u8,
    gpu_percent: u8,
    curve: Vec<(u8, u8)>,
    /// Percent last written while in curve mode (avoids redundant writes).
    applied: Option<u8>,
    /// Watchdog ticks since Turbo was last written to the hardware.
    turbo_ticks: u32,
    notice: String,
}

#[derive(Clone, Debug, Default)]
pub struct FanStatus {
    pub mode: FanMode,
    pub cpu_percent: u8,
    pub gpu_percent: u8,
    /// Set when the daemon overrode the user (e.g. thermal revert).
    pub notice: String,
}

pub struct FanManager {
    state: Mutex<State>,
}

impl FanManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
        }
    }

    pub fn status(&self) -> FanStatus {
        let s = self.state.lock().unwrap();

        FanStatus {
            mode: s.mode,
            cpu_percent: s.cpu_percent,
            gpu_percent: s.gpu_percent,
            notice: s.notice.clone(),
        }
    }

    pub fn curve(&self) -> Vec<(u8, u8)> {
        self.state.lock().unwrap().curve.clone()
    }

    /// Follow `points` (temperature °C, percent) while in curve mode.
    pub fn set_curve(&self, mut points: Vec<(u8, u8)>) -> Result<(), String> {
        points.sort_by_key(|p| p.0);

        if points.len() < CURVE_MIN_POINTS || points.len() > CURVE_MAX_POINTS {
            return Err(format!(
                "A fan curve needs {} to {} points",
                CURVE_MIN_POINTS, CURVE_MAX_POINTS
            ));
        }

        for p in &points {
            if p.0 > 100 || p.1 < MIN_PERCENT || p.1 > MAX_PERCENT {
                return Err(format!(
                    "Curve points must be 0-100°C and {}-{}%",
                    MIN_PERCENT, MAX_PERCENT
                ));
            }
        }

        for pair in points.windows(2) {
            if pair[1].0 <= pair[0].0 {
                return Err("Curve temperatures must be increasing".into());
            }

            if pair[1].1 < pair[0].1 {
                return Err(
                    "Fan speed can't drop as temperature rises".into(),
                );
            }
        }

        let temp = cpu_temperature().ok_or_else(|| {
            "No CPU temperature sensor found; refusing custom curve"
                .to_string()
        })?;

        if temp >= CRITICAL_TEMP_C {
            return Err(format!(
                "CPU is at {}°C; custom curves are unavailable above {}°C",
                temp, CRITICAL_TEMP_C
            ));
        }

        let mut state = self.state.lock().unwrap();

        state.curve = points;
        state.mode = FanMode::Curve;
        state.applied = None;
        state.notice.clear();

        apply_curve(&mut state, temp)
    }

    /// Maximum fans, always. Deliberately has no temperature checks: more
    /// cooling is never unsafe, so this works even when the CPU is hot or
    /// the sensor is missing, and the watchdog never reverts it.
    pub fn set_turbo(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();

        with_context(|ctx| unsafe {
            ffi::acer_fan_set_manual(ctx, MAX_PERCENT as i32, MAX_PERCENT as i32)
        })?;

        state.mode = FanMode::Turbo;
        state.cpu_percent = MAX_PERCENT;
        state.gpu_percent = MAX_PERCENT;
        state.applied = None;
        state.turbo_ticks = 0;
        state.notice.clear();

        Ok(())
    }

    /// CPU and GPU fans can be set independently. Both at 100% is Turbo.
    pub fn set_manual(&self, cpu: u8, gpu: u8) -> Result<(), String> {
        if cpu == MAX_PERCENT && gpu == MAX_PERCENT {
            return self.set_turbo();
        }

        if cpu < MIN_PERCENT
            || gpu < MIN_PERCENT
            || cpu > MAX_PERCENT
            || gpu > MAX_PERCENT
        {
            return Err(format!(
                "Fan speed must be between {}% and {}%",
                MIN_PERCENT, MAX_PERCENT
            ));
        }

        // Refuse to take over the fans if we cannot watch the temperature.
        let temp = cpu_temperature().ok_or_else(|| {
            "No CPU temperature sensor found; refusing manual fan control"
                .to_string()
        })?;

        if temp >= CRITICAL_TEMP_C {
            return Err(format!(
                "CPU is at {}°C; manual fan control is unavailable above {}°C",
                temp, CRITICAL_TEMP_C
            ));
        }

        let mut state = self.state.lock().unwrap();

        with_context(|ctx| unsafe {
            ffi::acer_fan_set_manual(ctx, cpu as i32, gpu as i32)
        })?;

        state.mode = FanMode::Manual;
        state.cpu_percent = cpu;
        state.gpu_percent = gpu;
        state.applied = None;
        state.notice.clear();

        Ok(())
    }

    pub fn set_auto(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();

        with_context(|ctx| unsafe { ffi::acer_fan_set_auto(ctx) })?;

        state.mode = FanMode::Auto;
        state.applied = None;
        state.notice.clear();

        Ok(())
    }

    /// Called periodically. Reverts to automatic control if the CPU gets
    /// too hot or the temperature can no longer be read, and drives the
    /// fans while a custom curve is active.
    pub fn watchdog_tick(&self) {
        let mut state = self.state.lock().unwrap();

        if state.mode == FanMode::Auto {
            return;
        }

        // Turbo is never overridden: no thermal revert, no sensor check.
        // Just keep re-asserting maximum fans.
        if state.mode == FanMode::Turbo {
            state.turbo_ticks += 1;

            if state.turbo_ticks >= TURBO_REASSERT_TICKS {
                state.turbo_ticks = 0;

                if let Err(e) = with_context(|ctx| unsafe {
                    ffi::acer_fan_set_manual(
                        ctx,
                        MAX_PERCENT as i32,
                        MAX_PERCENT as i32,
                    )
                }) {
                    eprintln!("fan turbo: {}", e);
                }
            }

            return;
        }

        let temp = cpu_temperature();

        let reason = match temp {
            Some(t) if t >= CRITICAL_TEMP_C => Some(format!(
                "CPU reached {}°C: automatic fan control restored",
                t
            )),
            None => Some(
                "Temperature sensor unavailable: automatic fan control restored"
                    .to_string(),
            ),
            _ => None,
        };

        if let Some(reason) = reason {
            eprintln!("fan watchdog: {}", reason);

            // Whether or not this succeeds, report auto mode: the safest
            // assumption is to keep retrying auto.
            let _ = with_context(|ctx| unsafe { ffi::acer_fan_set_auto(ctx) });

            state.mode = FanMode::Auto;
            state.applied = None;
            state.notice = reason;
            return;
        }

        // Firmware or a resume from sleep can quietly hand the fans back,
        // so fixed speeds and curves are re-written every few ticks.
        state.turbo_ticks += 1;
        let reassert = state.turbo_ticks >= TURBO_REASSERT_TICKS;

        if reassert {
            state.turbo_ticks = 0;
        }

        let mode = state.mode;

        match mode {
            FanMode::Curve => {
                if reassert {
                    state.applied = None;
                }

                if let Some(t) = temp {
                    if let Err(e) = apply_curve(&mut state, t) {
                        eprintln!("fan curve: {}", e);
                    }
                }
            }
            FanMode::Manual if reassert => {
                let (cpu, gpu) = (state.cpu_percent, state.gpu_percent);

                if let Err(e) = with_context(|ctx| unsafe {
                    ffi::acer_fan_set_manual(ctx, cpu as i32, gpu as i32)
                }) {
                    eprintln!("fan manual: {}", e);
                }
            }
            _ => {}
        }
    }

    /// Best-effort revert, used on daemon shutdown.
    pub fn shutdown(&self) {
        let _ = self.set_auto();
    }
}

/// Percent the curve asks for at `temp` (flat outside the end points,
/// linear in between), never below `MIN_PERCENT`.
fn curve_percent(curve: &[(u8, u8)], temp: i32) -> u8 {
    let (Some(first), Some(last)) = (curve.first(), curve.last()) else {
        return MIN_PERCENT;
    };

    if temp <= first.0 as i32 {
        return first.1.max(MIN_PERCENT);
    }

    if temp >= last.0 as i32 {
        return last.1.max(MIN_PERCENT);
    }

    for pair in curve.windows(2) {
        let (t0, p0) = (pair[0].0 as f64, pair[0].1 as f64);
        let (t1, p1) = (pair[1].0 as f64, pair[1].1 as f64);
        let t = temp as f64;

        if t >= t0 && t <= t1 {
            let frac = (t - t0) / (t1 - t0);
            let p = p0 + (p1 - p0) * frac;

            return (p.round() as u8).clamp(MIN_PERCENT, MAX_PERCENT);
        }
    }

    last.1.max(MIN_PERCENT)
}

fn apply_curve(state: &mut State, temp: i32) -> Result<(), String> {
    let target = curve_percent(&state.curve, temp);

    if state.applied == Some(target) {
        return Ok(());
    }

    with_context(|ctx| unsafe {
        ffi::acer_fan_set_manual(ctx, target as i32, target as i32)
    })?;

    state.applied = Some(target);
    state.cpu_percent = target;
    state.gpu_percent = target;

    Ok(())
}

fn with_context<F>(f: F) -> Result<(), String>
where
    F: FnOnce(*mut ffi::AcerContext) -> i32,
{
    let ctx = unsafe { ffi::acer_open() };

    if ctx.is_null() {
        return Err("Failed to open Acer driver".into());
    }

    let ret = f(ctx);

    unsafe { ffi::acer_close(ctx) };

    if ret < 0 {
        Err(format!(
            "Driver call failed: {}",
            std::io::Error::from_raw_os_error(-ret)
        ))
    } else {
        Ok(())
    }
}

/// CPU package temperature in °C, if a known sensor is present.
pub fn cpu_temperature() -> Option<i32> {
    let entries = fs::read_dir("/sys/class/hwmon").ok()?;

    for entry in entries.flatten() {
        let dir = entry.path();

        let name = fs::read_to_string(dir.join("name")).unwrap_or_default();
        let name = name.trim();

        // coretemp = Intel, k10temp = AMD
        if name != "coretemp" && name != "k10temp" {
            continue;
        }

        if let Some(t) = read_millidegrees(&dir.join("temp1_input")) {
            return Some(t);
        }
    }

    None
}

fn read_millidegrees(path: &Path) -> Option<i32> {
    let raw = fs::read_to_string(path).ok()?;
    let milli: i32 = raw.trim().parse().ok()?;
    Some(milli / 1000)
}
