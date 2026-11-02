//! Battery charge history, sampled once a minute and saved to disk so the
//! graph in the app covers time when the app was closed.

use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const HISTORY_FILE: &str = "/var/lib/acer-control-center/battery.csv";
const MAX_AGE_SECS: u64 = 7 * 24 * 3600;
const MAX_RETURNED_POINTS: usize = 600;
/// Rewrite the file (dropping old rows) after this many appended samples.
const REWRITE_EVERY: u32 = 500;

#[derive(Clone, Copy)]
struct Sample {
    ts: u64,
    percent: u8,
    charging: bool,
}

struct State {
    samples: VecDeque<Sample>,
    appended: u32,
}

pub struct BatteryHistory {
    path: PathBuf,
    state: Mutex<State>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// (percent, charging) of the first battery, if there is one.
fn read_battery() -> Option<(u8, bool)> {
    for entry in fs::read_dir("/sys/class/power_supply").ok()?.flatten() {
        let dir = entry.path();

        let kind = fs::read_to_string(dir.join("type")).unwrap_or_default();

        if kind.trim() != "Battery" {
            continue;
        }

        let percent: u8 = fs::read_to_string(dir.join("capacity"))
            .ok()?
            .trim()
            .parse()
            .ok()?;

        let status = fs::read_to_string(dir.join("status")).unwrap_or_default();

        return Some((percent.min(100), status.trim() == "Charging"));
    }

    None
}

impl BatteryHistory {
    pub fn load() -> Self {
        let path = PathBuf::from(HISTORY_FILE);
        let mut samples = VecDeque::new();

        if let Ok(text) = fs::read_to_string(&path) {
            let cutoff = now().saturating_sub(MAX_AGE_SECS);

            for line in text.lines() {
                let mut parts = line.split(',');

                let (Some(ts), Some(pct), Some(chg)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };

                let (Ok(ts), Ok(percent)) = (ts.parse::<u64>(), pct.parse::<u8>())
                else {
                    continue;
                };

                if ts >= cutoff {
                    samples.push_back(Sample {
                        ts,
                        percent,
                        charging: chg == "1",
                    });
                }
            }
        }

        Self {
            path,
            state: Mutex::new(State {
                samples,
                appended: 0,
            }),
        }
    }

    /// Take one reading and remember it.
    pub fn record(&self) {
        let Some((percent, charging)) = read_battery() else {
            return;
        };

        let sample = Sample {
            ts: now(),
            percent,
            charging,
        };

        let mut state = self.state.lock().unwrap();

        state.samples.push_back(sample);

        let cutoff = sample.ts.saturating_sub(MAX_AGE_SECS);
        while state.samples.front().map(|s| s.ts < cutoff).unwrap_or(false) {
            state.samples.pop_front();
        }

        state.appended += 1;

        if state.appended >= REWRITE_EVERY {
            state.appended = 0;
            self.rewrite(&state.samples);
        } else {
            self.append(&sample);
        }
    }

    fn append(&self, sample: &Sample) {
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }

        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(
                file,
                "{},{},{}",
                sample.ts,
                sample.percent,
                sample.charging as u8
            );
        }
    }

    fn rewrite(&self, samples: &VecDeque<Sample>) {
        write_all(&self.path, samples);
    }

    /// Samples from the last `window_secs`, thinned to a drawable size.
    pub fn window(&self, window_secs: u32) -> Vec<(u64, u8, bool)> {
        let state = self.state.lock().unwrap();
        let cutoff = now().saturating_sub(window_secs as u64);

        let picked: Vec<&Sample> =
            state.samples.iter().filter(|s| s.ts >= cutoff).collect();

        let step = picked.len().div_ceil(MAX_RETURNED_POINTS).max(1);

        picked
            .iter()
            .enumerate()
            .filter(|(i, _)| i % step == 0 || *i == picked.len() - 1)
            .map(|(_, s)| (s.ts, s.percent, s.charging))
            .collect()
    }
}

fn write_all(path: &Path, samples: &VecDeque<Sample>) {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    let mut text = String::new();

    for s in samples {
        text.push_str(&format!("{},{},{}\n", s.ts, s.percent, s.charging as u8));
    }

    let _ = fs::write(path, text);
}
