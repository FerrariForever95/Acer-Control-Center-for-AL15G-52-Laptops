//! Key-press detection for the reactive lighting effect.
//!
//! Privacy: this only ever notes *that* a key was pressed. Which key it was
//! is never looked at, stored or sent anywhere, and the keyboard device is
//! only opened while the reactive effect is running.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Size of `struct input_event` (timeval + type + code + value).
const EVENT_SIZE: usize = if cfg!(target_pointer_width = "64") { 24 } else { 16 };

const EV_KEY: u16 = 0x01;
const O_NONBLOCK: i32 = 0o4000;

/// Event device paths of real keyboards (anything with a letter-key
/// capability set, e.g. the laptop keyboard or a USB keyboard).
fn keyboards() -> Vec<PathBuf> {
    let Ok(text) = fs::read_to_string("/proc/bus/input/devices") else {
        return Vec::new();
    };

    let mut found = Vec::new();

    for block in text.split("\n\n") {
        let mut handlers = "";
        let mut ev = "";

        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("H: Handlers=") {
                handlers = rest;
            } else if let Some(rest) = line.strip_prefix("B: EV=") {
                ev = rest.trim();
            }
        }

        // EV=120013 is the standard main-keyboard capability set.
        if ev != "120013" || !handlers.split_whitespace().any(|h| h == "kbd") {
            continue;
        }

        if let Some(event) = handlers
            .split_whitespace()
            .find(|h| h.starts_with("event"))
        {
            found.push(PathBuf::from(format!("/dev/input/{}", event)));
        }
    }

    found
}

fn open_all() -> Vec<File> {
    keyboards()
        .into_iter()
        .filter_map(|path| {
            OpenOptions::new()
                .read(true)
                .custom_flags(O_NONBLOCK)
                .open(path)
                .ok()
        })
        .collect()
}

/// Start a background thread that sets `pressed` on every key press until
/// `stop` becomes true.
pub fn spawn_listener(pressed: Arc<AtomicBool>, stop: Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut files = open_all();
        let mut buf = [0u8; EVENT_SIZE * 32];

        while !stop.load(Ordering::Relaxed) {
            if files.is_empty() {
                // Keyboard not found (or unplugged): try again shortly.
                thread::sleep(Duration::from_secs(2));
                files = open_all();
                continue;
            }

            let mut any_data = false;
            let mut dead = Vec::new();

            for (i, file) in files.iter_mut().enumerate() {
                match file.read(&mut buf) {
                    Ok(0) => dead.push(i),
                    Ok(n) => {
                        any_data = true;

                        for event in buf[..n].chunks_exact(EVENT_SIZE) {
                            let kind = u16::from_ne_bytes([
                                event[EVENT_SIZE - 8],
                                event[EVENT_SIZE - 7],
                            ]);
                            let value = i32::from_ne_bytes([
                                event[EVENT_SIZE - 4],
                                event[EVENT_SIZE - 3],
                                event[EVENT_SIZE - 2],
                                event[EVENT_SIZE - 1],
                            ]);

                            // value 1 = key down (ignore release and repeat).
                            if kind == EV_KEY && value == 1 {
                                pressed.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(_) => dead.push(i),
                }
            }

            for i in dead.into_iter().rev() {
                files.remove(i);
            }

            if !any_data {
                thread::sleep(Duration::from_millis(8));
            }
        }
    });
}
