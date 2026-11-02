//! Keyboard lighting effects: smooth colour fades, a rainbow cycle, and a
//! pulse that reacts to key presses.
//!
//! Effects run inside the daemon, so they keep going when the GUI is closed.
//! The GUI only chooses an effect and its settings.

use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::backend::HardwareBackend;
use crate::keys;

pub const MIN_COLORS: usize = 2;
pub const MAX_COLORS: usize = 8;
pub const MIN_SECONDS: f64 = 0.2;
pub const MAX_SECONDS: f64 = 120.0;

type Rgb = (u8, u8, u8);

#[derive(Clone)]
enum Effect {
    /// A -> B -> ... -> A, forever.
    Fade { colors: Vec<Rgb>, seconds: f64 },
    /// Every hue in turn; `seconds` is one full trip around the colour wheel.
    Rainbow { seconds: f64 },
    /// `base` while idle; each key press flashes to `pulse`, which then
    /// fades back to `base` over `seconds`.
    Reactive { base: Rgb, pulse: Rgb, seconds: f64 },
}

struct Inner {
    effect: Option<Effect>,
    started: Instant,
    last_tick: Instant,
    last_written: Option<Rgb>,
    /// 1.0 right after a key press, decaying to 0.0.
    energy: f64,
    /// Stops the key listener thread when reactive mode ends.
    key_listener_stop: Option<Arc<AtomicBool>>,
}

pub struct Lighting {
    backend: Arc<dyn HardwareBackend>,
    inner: Mutex<Inner>,
    key_pressed: Arc<AtomicBool>,
}

impl Lighting {
    pub fn new(backend: Arc<dyn HardwareBackend>) -> Self {
        Self {
            backend,
            inner: Mutex::new(Inner {
                effect: None,
                started: Instant::now(),
                last_tick: Instant::now(),
                last_written: None,
                energy: 0.0,
                key_listener_stop: None,
            }),
            key_pressed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn check_seconds(seconds: f64) -> Result<(), String> {
        if !(MIN_SECONDS..=MAX_SECONDS).contains(&seconds) {
            return Err(format!(
                "Time must be between {} and {} seconds",
                MIN_SECONDS, MAX_SECONDS
            ));
        }

        Ok(())
    }

    fn set_effect(&self, effect: Effect) {
        let mut inner = self.inner.lock().unwrap();

        // Leaving reactive mode: stop listening to the keyboard.
        if let Some(stop) = inner.key_listener_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }

        // Entering reactive mode: start listening.
        if matches!(effect, Effect::Reactive { .. }) {
            let stop = Arc::new(AtomicBool::new(false));
            keys::spawn_listener(self.key_pressed.clone(), stop.clone());
            inner.key_listener_stop = Some(stop);
        }

        inner.effect = Some(effect);
        inner.started = Instant::now();
        inner.last_tick = Instant::now();
        inner.last_written = None;
        inner.energy = 0.0;
    }

    /// Fade through `colors` in order, then back to the first, forever.
    pub fn start_fade(&self, colors: Vec<Rgb>, seconds: f64) -> Result<(), String> {
        if colors.len() < MIN_COLORS || colors.len() > MAX_COLORS {
            return Err(format!(
                "Choose between {} and {} colours",
                MIN_COLORS, MAX_COLORS
            ));
        }

        Self::check_seconds(seconds)?;
        self.set_effect(Effect::Fade { colors, seconds });
        Ok(())
    }

    pub fn start_rainbow(&self, seconds: f64) -> Result<(), String> {
        Self::check_seconds(seconds)?;
        self.set_effect(Effect::Rainbow { seconds });
        Ok(())
    }

    pub fn start_reactive(
        &self,
        base: Rgb,
        pulse: Rgb,
        seconds: f64,
    ) -> Result<(), String> {
        Self::check_seconds(seconds)?;
        self.set_effect(Effect::Reactive {
            base,
            pulse,
            seconds,
        });
        Ok(())
    }

    /// Stop any effect and leave the keyboard on whatever colour it reached.
    pub fn stop(&self) {
        let mut inner = self.inner.lock().unwrap();

        if let Some(stop) = inner.key_listener_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }

        inner.effect = None;
    }

    /// (kind, colours, seconds). kind is "none", "fade", "rainbow" or
    /// "reactive". For reactive the colours are [base, pulse].
    pub fn status(&self) -> (String, Vec<Rgb>, f64) {
        let inner = self.inner.lock().unwrap();

        match &inner.effect {
            None => ("none".into(), Vec::new(), 4.0),
            Some(Effect::Fade { colors, seconds }) => {
                ("fade".into(), colors.clone(), *seconds)
            }
            Some(Effect::Rainbow { seconds }) => {
                ("rainbow".into(), Vec::new(), *seconds)
            }
            Some(Effect::Reactive {
                base,
                pulse,
                seconds,
            }) => ("reactive".into(), vec![*base, *pulse], *seconds),
        }
    }

    /// Advance the running effect. Cheap to call often: the hardware is
    /// only written when the colour actually changes.
    pub fn tick(&self) {
        let color = {
            let mut inner = self.inner.lock().unwrap();

            let Some(effect) = inner.effect.clone() else {
                return;
            };

            let now = Instant::now();
            let dt = now.duration_since(inner.last_tick).as_secs_f64();
            inner.last_tick = now;

            let color = match effect {
                Effect::Fade { colors, seconds } => color_at(
                    &colors,
                    seconds,
                    inner.started.elapsed().as_secs_f64(),
                ),
                Effect::Rainbow { seconds } => {
                    let hue = (inner.started.elapsed().as_secs_f64() / seconds)
                        .fract()
                        * 360.0;
                    hsv_to_rgb(hue)
                }
                Effect::Reactive {
                    base,
                    pulse,
                    seconds,
                } => {
                    if self.key_pressed.swap(false, Ordering::Relaxed) {
                        inner.energy = 1.0;
                    } else {
                        inner.energy = (inner.energy - dt / seconds).max(0.0);
                    }

                    // Squared so the flash is punchy and the tail is soft.
                    let e = inner.energy * inner.energy;

                    (
                        lerp(base.0, pulse.0, e),
                        lerp(base.1, pulse.1, e),
                        lerp(base.2, pulse.2, e),
                    )
                }
            };

            if inner.last_written == Some(color) {
                return;
            }

            inner.last_written = Some(color);
            color
        };

        if let Err(e) = self.backend.set_keyboard_color(color.0, color.1, color.2) {
            eprintln!("keyboard effect: {}", e);
        }
    }
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// Colour `elapsed` seconds into a fade loop. Each fade uses a cosine ease
/// so it starts and ends gently instead of changing at a constant rate.
fn color_at(colors: &[Rgb], seconds: f64, elapsed: f64) -> Rgb {
    let n = colors.len();

    if n == 0 {
        return (0, 0, 0);
    }

    let position = elapsed / seconds;
    let index = position.floor();
    let frac = position - index;

    let from = colors[(index as usize) % n];
    let to = colors[(index as usize + 1) % n];

    let eased = 0.5 - 0.5 * (PI * frac).cos();

    (
        lerp(from.0, to.0, eased),
        lerp(from.1, to.1, eased),
        lerp(from.2, to.2, eased),
    )
}

/// Fully saturated colour for a hue in degrees (0-360).
fn hsv_to_rgb(hue: f64) -> Rgb {
    let h = (hue % 360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();

    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };

    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}
