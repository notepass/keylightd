use std::{
    io,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

use argh::FromArgs;
use command::{GetKeyboardBacklight, SetKeyboardBacklight};
use ec::EmbeddedController;

use crate::command::{LedBrightnesses, LedControl, LedFlags, LedId};

mod command;
mod ec;

/// keylightd - automatic keyboard backlight daemon for Framework laptops
#[derive(Debug, FromArgs)]
struct Args {
    /// also listen to touchpad events to enable/disable backlight  [default=true]
    #[argh(option, default = "true")]
    react_to_touchpad: bool,

    /// activity timeout in seconds [default=10]
    #[argh(option, default = "10")]
    timeout: u32,

    /// also control the power LED in the fingerprint module
    #[argh(switch)]
    power: bool,

    /// reduce brightness to 1% instead of 0% on timeout
    #[argh(switch)]
    twilight: bool,
}

fn low_bright(curr_bright: u8, twilight_mode: bool) -> u8 {
    return if twilight_mode {
        if curr_bright == 0 {
            0
        } else {
            1
        }
    } else {
        0
    }
}

fn main() -> anyhow::Result<()> {
    let mut brightness: u8;
    let mut new_bright: u8;

    env_logger::builder()
        .filter_module(
            env!("CARGO_PKG_NAME"),
            if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            },
        )
        .init();

    let args: Args = argh::from_env();
    log::info!("args={:?}", args);

    let ec = EmbeddedController::open()?;

    brightness = ec.command(GetKeyboardBacklight)?.percent;

    let fade_to = |target: u8| -> io::Result<()> {
        let resp = ec.command(GetKeyboardBacklight)?;
        let mut cur = if resp.enabled != 0 { resp.percent } else { 0 };
        while cur != target {
            if cur > target {
                cur -= 1;
            } else {
                cur += 1;
            }

            if args.power {
                // The power LED cannot be faded from software (although the beta BIOS apparently
                // has a switch for dimming it, so maybe it'll work with the next BIOS update).
                // So instead, we treat 0 as off and set it back to auto for any non-zero value.
                if cur == 0 {
                    ec.command(LedControl {
                        led_id: LedId::POWER,
                        flags: LedFlags::NONE,
                        brightness: LedBrightnesses::default(),
                    })?;
                } else if cur == 1 {
                    ec.command(LedControl {
                        led_id: LedId::POWER,
                        flags: LedFlags::AUTO,
                        brightness: LedBrightnesses::default(),
                    })?;
                }
            }

            ec.command(SetKeyboardBacklight { percent: cur })?;

            thread::sleep(Duration::from_millis(3));
        }
        Ok(())
    };

    let act = Arc::new(ActivityState {
        last_activity: Mutex::new(Instant::now()),
        condvar: Condvar::new(),
    });

    for (path, mut device) in evdev::enumerate() {
        // Filter devices so that only the Framework's builtin touchpad and keyboard are listened
        // to. Since we don't support hotplug, listening on USB devices wouldn't work reliably.
        match device.name() {
            Some("PIXA3854:00 093A:0274 Touchpad" | "AT Translated Set 2 keyboard") => {
                if !args.react_to_touchpad && device.name() == Option::from("PIXA3854:00 093A:0274 Touchpad") {
                    log::debug!("Ignoring touchpad inputs!");
                    continue;
                }
                let act = act.clone();
                thread::spawn(move || -> io::Result<()> {
                    let name = device.name();
                    let name = name.as_deref().unwrap_or("<unknown>").to_string();
                    log::info!("starting listener on {}: {name}", path.display());
                    loop {
                        if let Err(e) = device.fetch_events() {
                            log::warn!(
                                "error while fetching events for device '{name}': {e}; closing"
                            );
                            return Err(e);
                        }
                        *act.last_activity.lock().unwrap() = Instant::now();
                        act.condvar.notify_one();

                        // Delay a bit, to avoid busy looping.
                        thread::sleep(Duration::from_millis(500));
                    }
                });
            }
            _ => {}
        }
    }

    log::debug!("idle timeout: {} seconds", args.timeout);
    log::debug!("current brightness level: {}%", brightness);

    let mut state = None;
    loop {
        let guard = act.last_activity.lock().unwrap();
        let last = *guard;
        let (_lock, result) = act
            .condvar
            .wait_timeout_while(guard, Duration::from_secs(args.timeout.into()), |instant| {
                *instant == last
            })
            .unwrap();
        let new_state = !result.timed_out();
        if state != Some(new_state) {
            log::debug!("activity state changed: {state:?} -> {new_state}");
            if new_state {
                // Fade in
                fade_to(brightness)?;
            } else {
                // Fade out
                new_bright = ec.command(GetKeyboardBacklight)?.percent;
                if new_bright != brightness {
                    log::debug!("new brightness level: was: {}%, is now:{}%", brightness, new_bright);
                    brightness = new_bright;
                }
                fade_to(low_bright(brightness, args.twilight))?;
            }
            state = Some(new_state);
        }
    }
}

struct ActivityState {
    last_activity: Mutex<Instant>,
    condvar: Condvar,
}
