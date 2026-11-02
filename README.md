# Acer Control Center

A native Linux control center for Acer gaming laptops (GTK4 / Libadwaita).

## Features

- **Dashboard**: temperature, fans, battery, cooling mode, keyboard lighting
- **Cooling**: live fan gauges, Auto / Silent / Balanced / Turbo, fixed speeds
  (CPU and GPU fans linked or separate), drag-to-edit custom fan curve
- **GPU**: NVIDIA status, sleep when idle, Integrated / Hybrid / NVIDIA mode
  switching through [EnvyControl](https://github.com/bayasdev/envycontrol)
- **Keyboard RGB**: static color, brightness, fade, rainbow, react to keys
- **Profiles**: save and apply fan + lighting setups, optionally at startup
- **Battery**: charge, health, cycles and history graph

## Supported hardware

Tested on the Acer Nitro AL15G-53. Other Clevo-based Acer laptops may work but
are untested.

## Install

```bash
git clone https://github.com/stanlysilas/acer-control-center.git
cd acer-control-center
sudo ./install.sh
```

Reboot, then open **Acer Control Center** from your application menu.

Uninstall with `sudo ./uninstall.sh`.

## Fan safety

Enforced by the background service, not the app:

- Fans never go below 20%.
- Manual and curve modes are refused if no CPU temperature sensor is found.
- At 90°C the firmware takes back control (Turbo is exempt and stays at 100%).
- Fan settings are re-applied every few seconds, so a sleep/resume can't
  silently reset them.
- Automatic control is restored when the service stops.

## GPU modes

- *Sleep when idle* writes `auto` to the GPU's PCI `power/control`. It applies
  immediately and resets on reboot. The NVIDIA driver also needs dynamic power
  management enabled.
- *Integrated* disables the GPU at boot (best battery; outputs wired to the GPU
  stop working). *Hybrid* and *NVIDIA* re-enable it.
- Mode changes need EnvyControl and a restart. The GPU page shows a
  **Restart now** button while a change is pending.
- Some BIOS/MUX settings override all of the above.

## D-Bus API

Service `io.github.stanlysilas.AcerControl` on the system bus:

| Area | Methods |
| --- | --- |
| Fans | `SetFanManual(cpu, gpu)`, `SetFanAuto()`, `SetFanTurbo()`, `SetFanCurve(points)`, `GetFanCurve()`, `GetFanStatus()`, `GetCpuTemperature()` |
| Keyboard | `Get/SetKeyboardBrightness`, `Get/SetKeyboardColor`, `StartKeyboardAnimation`, `StartKeyboardRainbow`, `StartKeyboardReactive`, `StopKeyboardAnimation`, `GetKeyboardEffect` |
| GPU | `GetGpuStatus()`, `SetGpuPowerControl(auto)`, `SetGpuMode(mode)` |
| Profiles | `ListProfiles`, `SaveProfile`, `ApplyProfile`, `DeleteProfile`, `Get/SetStartupProfile` |
| Battery | `GetBatteryHistory(window_secs)` |

Data lives in `/var/lib/acer-control-center/` (battery history, profiles).

## Project layout

`driver/` kernel module (DKMS) · `lib/` libacercontrol · `daemon/` system
service · `gui/` GTK app · `packaging/` systemd, D-Bus, udev, desktop, icon

## License

GPL-3.0-or-later
