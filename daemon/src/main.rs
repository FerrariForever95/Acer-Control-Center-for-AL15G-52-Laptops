mod backend;
mod battery;
mod dbus;
mod fan;
mod ffi;
mod gpu;
mod keys;
mod lighting;
mod profiles;
mod service;
mod sysfs;

use std::sync::Arc;

use anyhow::Result;
use battery::BatteryHistory;
use dbus::AcerControlDBus;
use fan::FanManager;
use gpu::Gpu;
use lighting::Lighting;
use profiles::Profiles;
use service::AcerService;
use sysfs::SysfsBackend;
use std::time::Duration;
use tokio::signal::unix::{signal, SignalKind};
use zbus::connection::Builder;

#[tokio::main]
async fn main() -> Result<()> {
    let backend = Arc::new(SysfsBackend::new());

    let fans = Arc::new(FanManager::new());

    let lighting = Arc::new(Lighting::new(backend.clone()));

    let battery = Arc::new(BatteryHistory::load());

    let profiles = Arc::new(Profiles::load());
    let gpu = Arc::new(Gpu::new());

    let service = Arc::new(AcerService::new(
        backend,
        fans.clone(),
        lighting.clone(),
        battery.clone(),
        profiles,
        gpu,
    ));

    let _connection = Builder::system()?
        .name("io.github.stanlysilas.AcerControl")?
        .serve_at(
            "/io/github/stanlysilas/AcerControl",
            AcerControlDBus::new(service.clone()),
        )?
        .build()
        .await?;

    println!("Acer Control Daemon started.");

    // Startup profile: the fan mode and keyboard lighting the user chose to
    // have at boot. The keyboard driver may not be ready the instant the
    // daemon starts, so a failed attempt is retried for a short while.
    {
        let service = service.clone();

        tokio::spawn(async move {
            for attempt in 1..=10 {
                match service.apply_startup_profile() {
                    Ok(Some(name)) => {
                        println!("Applied startup profile \"{}\".", name);
                        break;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!(
                            "Startup profile (attempt {}/10): {}",
                            attempt, e
                        );

                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                }
            }
        });
    }

    // Thermal watchdog: if the CPU gets too hot while fans are pinned,
    // the firmware is put back in charge.
    {
        let fans = fans.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));

            loop {
                interval.tick().await;
                fans.watchdog_tick();
            }
        });
    }

    // Battery history: one sample a minute, saved to disk.
    {
        let battery = battery.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;
                battery.record();
            }
        });
    }

    // Keyboard effects (~25 updates per second).
    {
        let lighting = lighting.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_millis(40));

            loop {
                interval.tick().await;
                lighting.tick();
            }
        });
    }

    // Never leave the fans pinned when the daemon goes away.
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    tokio::select! {
        _ = sigterm.recv() => {},
        _ = sigint.recv() => {},
    }

    println!("Shutting down, restoring automatic fan control.");
    fans.shutdown();

    Ok(())
}
