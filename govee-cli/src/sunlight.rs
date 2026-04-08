//! Sunlight mode: crossfades between day and night LED behaviors based on solar
//! position. Presets pair two animated themes (e.g. ocean by day, fireplace by
//! night) and smoothly blend between them during sunrise/sunset transitions.

use chrono::{Local, NaiveTime, Timelike};
use govee_lan::{send_brightness, send_color_temp, UdpSender};
use govee_themes::themes::{pa, wp, Behavior, Delay, Rgb};
use std::time::Duration;

use crate::cli::{CliSunlightPreset, SunlightArgs};
use crate::{RUNNING, ctrlc_setup, resolve_or_exit};

/// Which phase of the solar cycle we're in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SolarPhase {
    Day,
    Night,
    /// Dawn transition: 0.0 = full night → 1.0 = full day.
    Dawn(f64),
    /// Dusk transition: 0.0 = full day → 1.0 = full night.
    Dusk(f64),
}

/// A sunlight preset: day behavior, night behavior, and frame delay.
struct Preset {
    day: Behavior,
    night: Behavior,
    delay: Delay,
}

/// Compute the current solar phase given local time and sunrise/sunset.
pub fn solar_phase(
    now: NaiveTime,
    sunrise: NaiveTime,
    sunset: NaiveTime,
    transition_mins: u32,
) -> SolarPhase {
    let half = chrono::TimeDelta::minutes(transition_mins as i64 / 2);

    let sunrise_start = sunrise - half;
    let sunrise_end = sunrise + half;
    let sunset_start = sunset - half;
    let sunset_end = sunset + half;

    if now >= sunrise_end && now < sunset_start {
        SolarPhase::Day
    } else if now >= sunrise_start && now < sunrise_end {
        let total = (sunrise_end - sunrise_start).num_seconds() as f64;
        let elapsed = (now - sunrise_start).num_seconds() as f64;
        SolarPhase::Dawn((elapsed / total).clamp(0.0, 1.0))
    } else if now >= sunset_start && now < sunset_end {
        let total = (sunset_end - sunset_start).num_seconds() as f64;
        let elapsed = (now - sunset_start).num_seconds() as f64;
        SolarPhase::Dusk((elapsed / total).clamp(0.0, 1.0))
    } else {
        SolarPhase::Night
    }
}

/// Compute sunrise/sunset times from latitude/longitude for the current date.
fn solar_times(lat: f64, lon: f64) -> (NaiveTime, NaiveTime) {
    use sunrise::{Coordinates, SolarDay, SolarEvent};

    let today = Local::now().date_naive();
    let coord = Coordinates::new(lat, lon).expect("invalid coordinates");
    let solar = SolarDay::new(coord, today);

    let rise_utc = solar.event_time(SolarEvent::Sunrise);
    let set_utc = solar.event_time(SolarEvent::Sunset);

    let rise_local = rise_utc.with_timezone(&Local).time();
    let set_local = set_utc.with_timezone(&Local).time();

    (rise_local, set_local)
}

/// Parse "HH:MM" into a NaiveTime.
fn parse_time(s: &str) -> NaiveTime {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        crate::ui::error_hint("Invalid time format", "Expected HH:MM (e.g. 07:00)");
        std::process::exit(1);
    }
    let h: u32 = parts[0].parse().unwrap_or_else(|_| {
        crate::ui::error_hint("Invalid hour", "Expected HH:MM");
        std::process::exit(1);
    });
    let m: u32 = parts[1].parse().unwrap_or_else(|_| {
        crate::ui::error_hint("Invalid minute", "Expected HH:MM");
        std::process::exit(1);
    });
    NaiveTime::from_hms_opt(h, m, 0).unwrap_or_else(|| {
        crate::ui::error_hint("Invalid time", "Hour 0-23, minute 0-59");
        std::process::exit(1);
    })
}

/// Linearly blend two segment color arrays.
fn blend_segments(a: &[Rgb], b: &[Rgb], t: f64) -> Vec<Rgb> {
    a.iter()
        .zip(b)
        .map(|(&(ar, ag, ab), &(br, bg, bb))| {
            let f = |a: u8, b: u8| -> u8 {
                (a as f64 * (1.0 - t) + b as f64 * t).round() as u8
            };
            (f(ar, br), f(ag, bg), f(ab, bb))
        })
        .collect()
}

/// Get the preset behaviors for a given preset name.
fn get_preset(preset: CliSunlightPreset) -> Preset {
    match preset {
        CliSunlightPreset::Coastal => Preset {
            day: Behavior::Wave {
                palette: vec![
                    pa(0.0, 0, 40, 80),
                    pa(0.5, 0, 100, 160),
                    pa(1.0, 0, 160, 220),
                ],
                waves: vec![wp(0.8, 0.7, 0.0), wp(0.5, 1.2, 1.0)],
                weights: vec![0.6, 0.4],
            },
            night: Behavior::Heat {
                palette: vec![
                    pa(0.0, 80, 0, 0),
                    pa(0.25, 124, 9, 0),
                    pa(0.5, 168, 35, 4),
                    pa(0.75, 211, 79, 13),
                    pa(1.0, 255, 140, 30),
                ],
                volatility: 0.15,
                spark_chance: 0.1,
                spark_boost: 0.4,
                dim_chance: 0.2,
                dim_range: (0.2, 0.6),
                diffusion: 0.0,
            },
            delay: Delay::Fixed(80),
        },
        CliSunlightPreset::Arctic => Preset {
            day: Behavior::Wave {
                palette: vec![
                    pa(0.0, 0, 200, 80),
                    pa(0.3, 120, 60, 160),
                    pa(0.7, 0, 100, 200),
                    pa(1.0, 60, 180, 100),
                ],
                waves: vec![wp(0.3, 0.8, 0.0), wp(1.5, 2.0, 0.0)],
                weights: vec![0.7, 0.3],
            },
            night: Behavior::Wave {
                palette: vec![
                    pa(0.0, 0, 180, 60),
                    pa(0.25, 60, 220, 100),
                    pa(0.5, 150, 100, 200),
                    pa(0.75, 40, 80, 180),
                    pa(1.0, 100, 200, 140),
                ],
                waves: vec![wp(0.2, 0.5, 0.0), wp(0.7, 1.5, 2.0)],
                weights: vec![0.6, 0.4],
            },
            delay: Delay::Fixed(80),
        },
        CliSunlightPreset::Ember => Preset {
            day: Behavior::Heat {
                palette: vec![
                    pa(0.0, 100, 40, 0),
                    pa(0.5, 180, 110, 12),
                    pa(1.0, 255, 210, 25),
                ],
                volatility: 0.2,
                spark_chance: 0.15,
                spark_boost: 0.5,
                dim_chance: 0.25,
                dim_range: (0.1, 0.4),
                diffusion: 0.0,
            },
            night: Behavior::Heat {
                palette: vec![
                    pa(0.0, 100, 40, 0),
                    pa(0.4, 200, 100, 10),
                    pa(0.8, 240, 150, 20),
                    pa(1.0, 255, 180, 40),
                ],
                volatility: 0.10,
                spark_chance: 0.06,
                spark_boost: 0.3,
                dim_chance: 0.15,
                dim_range: (0.3, 0.6),
                diffusion: 0.1,
            },
            delay: Delay::Random(100, 250),
        },
        CliSunlightPreset::Simple => unreachable!("simple preset handled separately"),
    }
}

/// Resolve sunrise/sunset times from args (manual or solar calculation).
fn resolve_times(args: &SunlightArgs) -> (NaiveTime, NaiveTime) {
    match (&args.sunrise, &args.sunset) {
        (Some(rise), Some(set)) => (parse_time(rise), parse_time(set)),
        _ => match (args.lat, args.lon) {
            (Some(lat), Some(lon)) => solar_times(lat, lon),
            _ => {
                crate::ui::error_hint(
                    "No location info",
                    "Provide --lat/--lon or --sunrise/--sunset",
                );
                std::process::exit(1);
            }
        },
    }
}

/// Run the simple (flat Kelvin) sunlight loop.
fn run_simple_loop(args: &SunlightArgs, ip: &str) {
    let (sunrise, sunset) = resolve_times(args);

    crate::ui::info(
        "Sunlight",
        &format!("simple · {}K day / {}K night", args.day_temp, args.night_temp),
    );
    crate::ui::info(
        "Schedule",
        &format!(
            "rise {} · set {} · {}min transition",
            sunrise.format("%H:%M"),
            sunset.format("%H:%M"),
            args.transition
        ),
    );
    {
        use colored::Colorize;
        println!("  {}", "Press Ctrl+C to stop".dimmed());
    }

    send_brightness(ip, args.brightness).ok();

    let mut last_kelvin: u16 = 0;
    let mut last_brightness: u8 = 0;

    while RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        let now = Local::now().time();
        let phase = solar_phase(now, sunrise, sunset, args.transition);

        let kelvin = match phase {
            SolarPhase::Day => args.day_temp,
            SolarPhase::Night => args.night_temp,
            SolarPhase::Dawn(t) => {
                let k = args.night_temp as f64 * (1.0 - t) + args.day_temp as f64 * t;
                k.round() as u16
            }
            SolarPhase::Dusk(t) => {
                let k = args.day_temp as f64 * (1.0 - t) + args.night_temp as f64 * t;
                k.round() as u16
            }
        };

        if kelvin != last_kelvin {
            send_color_temp(ip, kelvin).ok();
            last_kelvin = kelvin;
            if args.verbose {
                let phase_name = match phase {
                    SolarPhase::Day => "day".to_string(),
                    SolarPhase::Night => "night".to_string(),
                    SolarPhase::Dawn(t) => format!("dawn {:.0}%", t * 100.0),
                    SolarPhase::Dusk(t) => format!("dusk {:.0}%", t * 100.0),
                };
                crate::ui::info("Temp", &format!("{kelvin}K ({phase_name})"));
            }
        }

        // Handle night brightness if set
        if let Some(night_br) = args.night_brightness {
            let br = match phase {
                SolarPhase::Day => args.brightness,
                SolarPhase::Night => night_br,
                SolarPhase::Dawn(t) => {
                    (night_br as f64 * (1.0 - t) + args.brightness as f64 * t).round() as u8
                }
                SolarPhase::Dusk(t) => {
                    (args.brightness as f64 * (1.0 - t) + night_br as f64 * t).round() as u8
                }
            };
            if br != last_brightness {
                send_brightness(ip, br).ok();
                last_brightness = br;
            }
        }

        // Check once per second for responsive Ctrl+C, but only act every 60s
        for _ in 0..60 {
            if !RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }

        // Recalculate solar times at midnight if using lat/lon
        if now.hour() == 0 && now.minute() == 0 {
            if let (Some(lat), Some(lon)) = (args.lat, args.lon) {
                let (new_rise, new_set) = solar_times(lat, lon);
                if args.verbose {
                    crate::ui::info(
                        "Solar",
                        &format!(
                            "recalculated: rise {} · set {}",
                            new_rise.format("%H:%M"),
                            new_set.format("%H:%M")
                        ),
                    );
                }
            }
        }
    }
}

/// Run the animated (DreamView crossfade) sunlight loop.
fn run_animated_loop(args: &SunlightArgs, ip: &str, mirror: bool) {
    let preset = get_preset(args.preset);
    let (sunrise, sunset) = resolve_times(args);
    let n_seg = args.segments;

    let preset_name = match args.preset {
        CliSunlightPreset::Coastal => "coastal",
        CliSunlightPreset::Arctic => "arctic",
        CliSunlightPreset::Ember => "ember",
        CliSunlightPreset::Simple => unreachable!(),
    };

    {
        use colored::Colorize;
        crate::ui::info(
            "Sunlight",
            &format!("{}", preset_name.white().bold()),
        );
        crate::ui::info(
            "Schedule",
            &format!(
                "rise {} · set {} · {}min transition",
                sunrise.format("%H:%M"),
                sunset.format("%H:%M"),
                args.transition
            ),
        );
        crate::ui::info("Segments", &format!("{n_seg}"));
        crate::ui::info("Brightness", &crate::ui::brightness_bar(args.brightness));
        println!("  {}", "Press Ctrl+C to stop".dimmed());
    }

    let sender = UdpSender::new(ip).expect("Failed to create UDP sender");
    crate::dreamview::activate(ip, args.brightness, true);

    let mut rng = rand::rng();
    let mut day_state = crate::themes::init_state(&preset.day, n_seg);
    let mut night_state = crate::themes::init_state(&preset.night, n_seg);
    let mut t_acc: f64 = 0.0;
    let mut last_brightness: u8 = 0;

    // Track sunrise/sunset (may recalculate at midnight)
    let mut current_sunrise = sunrise;
    let mut current_sunset = sunset;
    let mut last_date = Local::now().date_naive();

    while RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        let now = Local::now();

        // Recalculate solar times on date change
        if now.date_naive() != last_date {
            if let (Some(lat), Some(lon)) = (args.lat, args.lon) {
                let (new_rise, new_set) = solar_times(lat, lon);
                current_sunrise = new_rise;
                current_sunset = new_set;
                if args.verbose {
                    crate::ui::info(
                        "Solar",
                        &format!(
                            "recalculated: rise {} · set {}",
                            new_rise.format("%H:%M"),
                            new_set.format("%H:%M")
                        ),
                    );
                }
            }
            last_date = now.date_naive();
        }

        let phase = solar_phase(now.time(), current_sunrise, current_sunset, args.transition);

        // Always render both behaviors to keep state warm
        let day_colors =
            crate::themes::render_frame(&preset.day, &mut rng, &mut day_state, n_seg, t_acc);
        let night_colors =
            crate::themes::render_frame(&preset.night, &mut rng, &mut night_state, n_seg, t_acc);

        let colors = match phase {
            SolarPhase::Day => day_colors,
            SolarPhase::Night => night_colors,
            SolarPhase::Dawn(t) => blend_segments(&night_colors, &day_colors, t),
            SolarPhase::Dusk(t) => blend_segments(&day_colors, &night_colors, t),
        };

        let send_colors = crate::dreamview::apply_mirror(&colors, mirror);
        let _ = sender.send_segments(&send_colors, true);

        let phase_tag = match phase {
            SolarPhase::Day => "day",
            SolarPhase::Night => "night",
            SolarPhase::Dawn(_) => "dawn",
            SolarPhase::Dusk(_) => "dusk",
        };
        crate::ui::status_line(&send_colors, phase_tag);

        // Handle night brightness
        if let Some(night_br) = args.night_brightness {
            let br = match phase {
                SolarPhase::Day => args.brightness,
                SolarPhase::Night => night_br,
                SolarPhase::Dawn(t) => {
                    (night_br as f64 * (1.0 - t) + args.brightness as f64 * t).round() as u8
                }
                SolarPhase::Dusk(t) => {
                    (args.brightness as f64 * (1.0 - t) + night_br as f64 * t).round() as u8
                }
            };
            if br != last_brightness {
                send_brightness(ip, br).ok();
                last_brightness = br;
            }
        }

        let delay_ms = crate::themes::get_delay(&preset.delay, &mut rng);
        std::thread::sleep(Duration::from_millis(delay_ms));
        t_acc += delay_ms as f64 / 1000.0;
    }

    crate::dreamview::shutdown(ip, true);
}

/// Entry point for the sunlight command.
pub fn run_sunlight(args: SunlightArgs, ip: Option<String>, mirror: bool) {
    let ip = resolve_or_exit(ip.as_deref());
    ctrlc_setup();

    match args.preset {
        CliSunlightPreset::Simple => run_simple_loop(&args, &ip),
        _ => run_animated_loop(&args, &ip, mirror),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn test_solar_phase_midday() {
        let phase = solar_phase(t(12, 0), t(7, 0), t(19, 0), 60);
        assert_eq!(phase, SolarPhase::Day);
    }

    #[test]
    fn test_solar_phase_midnight() {
        let phase = solar_phase(t(2, 0), t(7, 0), t(19, 0), 60);
        assert_eq!(phase, SolarPhase::Night);
    }

    #[test]
    fn test_solar_phase_late_night() {
        let phase = solar_phase(t(23, 0), t(7, 0), t(19, 0), 60);
        assert_eq!(phase, SolarPhase::Night);
    }

    #[test]
    fn test_solar_phase_dawn_midpoint() {
        // Sunrise at 7:00, 60min transition → dawn is 6:30–7:30
        // At 7:00, we're halfway through the dawn window
        let phase = solar_phase(t(7, 0), t(7, 0), t(19, 0), 60);
        match phase {
            SolarPhase::Dawn(t) => assert!((t - 0.5).abs() < 0.02),
            other => panic!("expected Dawn, got {other:?}"),
        }
    }

    #[test]
    fn test_solar_phase_dusk_midpoint() {
        // Sunset at 19:00, 60min transition → dusk is 18:30–19:30
        let phase = solar_phase(t(19, 0), t(7, 0), t(19, 0), 60);
        match phase {
            SolarPhase::Dusk(t) => assert!((t - 0.5).abs() < 0.02),
            other => panic!("expected Dusk, got {other:?}"),
        }
    }

    #[test]
    fn test_solar_phase_dawn_start() {
        let phase = solar_phase(t(6, 30), t(7, 0), t(19, 0), 60);
        match phase {
            SolarPhase::Dawn(t) => assert!(t < 0.02),
            other => panic!("expected Dawn(~0), got {other:?}"),
        }
    }

    #[test]
    fn test_solar_phase_dusk_end() {
        let phase = solar_phase(t(19, 29), t(7, 0), t(19, 0), 60);
        match phase {
            SolarPhase::Dusk(t) => assert!(t > 0.95),
            other => panic!("expected Dusk(~1), got {other:?}"),
        }
    }

    #[test]
    fn test_blend_segments_extremes() {
        let a = vec![(255, 0, 0), (0, 255, 0)];
        let b = vec![(0, 0, 255), (255, 255, 255)];

        // t=0 → all A
        let r = blend_segments(&a, &b, 0.0);
        assert_eq!(r, a);

        // t=1 → all B
        let r = blend_segments(&a, &b, 1.0);
        assert_eq!(r, b);
    }

    #[test]
    fn test_blend_segments_midpoint() {
        let a = vec![(200, 0, 0)];
        let b = vec![(0, 200, 0)];
        let r = blend_segments(&a, &b, 0.5);
        assert_eq!(r, vec![(100, 100, 0)]);
    }
}
