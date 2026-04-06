use crate::themes::{Behavior, Delay, ThemeDef, ThemeKind};

pub static THEMES: &[ThemeDef] = &[
    // ── Static ──────────────────────────────────────────────────────────
    ThemeDef { name: "movie",  category: "static", kind: ThemeKind::Solid { color: (20, 10, 40) } },
    ThemeDef { name: "chill",  category: "static", kind: ThemeKind::Solid { color: (80, 40, 120) } },
    ThemeDef { name: "party",  category: "static", kind: ThemeKind::Solid { color: (255, 0, 200) } },
    ThemeDef { name: "sunset", category: "static", kind: ThemeKind::Solid { color: (255, 100, 20) } },
    ThemeDef { name: "forest", category: "static", kind: ThemeKind::Solid { color: (10, 120, 30) } },

    // ── Nature ──────────────────────────────────────────────────────────
    ThemeDef {
        name: "candlelight", category: "nature",
        kind: ThemeKind::Animated {
            behavior: Behavior::Heat {
                palette: &[(0.0, 100, 40, 0), (0.5, 180, 110, 12), (1.0, 255, 210, 25)],
                volatility: 0.2, spark_chance: 0.15, spark_boost: 0.5,
                dim_chance: 0.25, dim_range: (0.1, 0.4), diffusion: 0.0,
            },
            delay: Delay::Random(100, 250),
        },
    },
    ThemeDef {
        name: "fireplace", category: "nature",
        kind: ThemeKind::Animated {
            behavior: Behavior::Heat {
                palette: &[
                    (0.0, 80, 0, 0), (0.25, 124, 9, 0), (0.5, 168, 35, 4),
                    (0.75, 211, 79, 13), (1.0, 255, 140, 30),
                ],
                volatility: 0.15, spark_chance: 0.1, spark_boost: 0.4,
                dim_chance: 0.2, dim_range: (0.2, 0.6), diffusion: 0.0,
            },
            delay: Delay::Random(80, 180),
        },
    },
    ThemeDef {
        name: "campfire", category: "nature",
        kind: ThemeKind::Animated {
            behavior: Behavior::Heat {
                palette: &[
                    (0.0, 100, 40, 0), (0.4, 200, 100, 10),
                    (0.8, 240, 150, 20), (1.0, 255, 180, 40),
                ],
                volatility: 0.10, spark_chance: 0.06, spark_boost: 0.3,
                dim_chance: 0.15, dim_range: (0.3, 0.6), diffusion: 0.1,
            },
            delay: Delay::Random(120, 280),
        },
    },
    ThemeDef {
        name: "lava", category: "nature",
        kind: ThemeKind::Animated {
            behavior: Behavior::Heat {
                palette: &[(0.0, 120, 0, 0), (0.5, 188, 15, 0), (1.0, 255, 60, 0)],
                volatility: 0.08, spark_chance: 0.05, spark_boost: 1.0,
                dim_chance: 0.0, dim_range: (0.0, 0.0), diffusion: 0.2,
            },
            delay: Delay::Fixed(80),
        },
    },
    ThemeDef {
        name: "ocean", category: "nature",
        kind: ThemeKind::Animated {
            behavior: Behavior::Wave {
                palette: &[(0.0, 0, 40, 80), (0.5, 0, 100, 160), (1.0, 0, 160, 220)],
                waves: &[(0.8, 0.7, 0.0), (0.5, 1.2, 1.0)],
                weights: &[0.6, 0.4],
            },
            delay: Delay::Fixed(80),
        },
    },
    ThemeDef {
        name: "aurora", category: "nature",
        kind: ThemeKind::Animated {
            behavior: Behavior::Wave {
                palette: &[
                    (0.0, 0, 200, 80), (0.3, 120, 60, 160),
                    (0.7, 0, 100, 200), (1.0, 60, 180, 100),
                ],
                waves: &[(0.3, 0.8, 0.0), (1.5, 2.0, 0.0)],
                weights: &[0.7, 0.3],
            },
            delay: Delay::Fixed(80),
        },
    },
    ThemeDef {
        name: "northern-lights", category: "nature",
        kind: ThemeKind::Animated {
            behavior: Behavior::Wave {
                palette: &[
                    (0.0, 0, 180, 60), (0.25, 60, 220, 100),
                    (0.5, 180, 60, 180), (0.75, 100, 0, 200),
                    (1.0, 0, 180, 60),
                ],
                waves: &[(0.15, 0.4, 0.0), (0.08, 0.2, 1.5)],
                weights: &[0.6, 0.4],
            },
            delay: Delay::Fixed(100),
        },
    },
    ThemeDef {
        name: "rain", category: "nature",
        kind: ThemeKind::Animated {
            behavior: Behavior::Particles {
                bg: (5, 5, 15),
                palette: &[(0.0, 20, 30, 60), (0.4, 40, 60, 120), (0.7, 80, 100, 180), (1.0, 160, 180, 255)],
                speed: 0.05, spawn_chance: 0.5, bright_chance: 0.15,
            },
            delay: Delay::Random(60, 100),
        },
    },

    // ── Vibes ───────────────────────────────────────────────────────────
    ThemeDef {
        name: "breathing", category: "vibes",
        kind: ThemeKind::Animated {
            behavior: Behavior::Breathe {
                palette: &[(0.0, 40, 10, 0), (1.0, 240, 90, 20)],
                speed: 0.4, power: 2,
            },
            delay: Delay::Fixed(80),
        },
    },
    ThemeDef {
        name: "romantic", category: "vibes",
        kind: ThemeKind::Animated {
            behavior: Behavior::Breathe {
                palette: &[(0.0, 60, 5, 15), (0.5, 160, 20, 50), (1.0, 200, 30, 60)],
                speed: 0.3, power: 2,
            },
            delay: Delay::Fixed(80),
        },
    },
    ThemeDef {
        name: "cozy", category: "vibes",
        kind: ThemeKind::Animated {
            behavior: Behavior::Breathe {
                palette: &[(0.0, 30, 15, 0), (1.0, 160, 90, 15)],
                speed: 0.2, power: 2,
            },
            delay: Delay::Fixed(100),
        },
    },
    ThemeDef {
        name: "cyberpunk", category: "vibes",
        kind: ThemeKind::Animated {
            behavior: Behavior::Drift {
                palette: &[
                    (0.0, 255, 0, 100), (0.15, 255, 0, 100),
                    (0.16, 0, 255, 255), (0.49, 0, 255, 255),
                    (0.50, 160, 0, 255), (0.82, 160, 0, 255),
                    (0.83, 255, 0, 100), (1.0, 255, 0, 100),
                ],
                speed: 0.08,
            },
            delay: Delay::Fixed(60),
        },
    },
    ThemeDef {
        name: "vaporwave", category: "vibes",
        kind: ThemeKind::Animated {
            behavior: Behavior::Drift {
                palette: &[
                    (0.0, 255, 150, 200), (0.33, 180, 130, 255),
                    (0.67, 100, 220, 220), (1.0, 255, 150, 200),
                ],
                speed: 0.04,
            },
            delay: Delay::Fixed(80),
        },
    },
    ThemeDef {
        name: "nightclub", category: "vibes",
        kind: ThemeKind::Animated {
            behavior: Behavior::Strobe {
                colors: &[
                    (255, 0, 0), (0, 255, 0), (0, 0, 255),
                    (255, 0, 255), (0, 255, 255), (255, 255, 0),
                ],
                cycle_speed: 8.0, flash_chance: 0.08,
            },
            delay: Delay::Fixed(50),
        },
    },

    // ── Functional ──────────────────────────────────────────────────────
    ThemeDef {
        name: "storm", category: "functional",
        kind: ThemeKind::Animated {
            behavior: Behavior::Flash {
                base_palette: &[(0.0, 0, 0, 30), (0.5, 10, 5, 50), (1.0, 10, 5, 70)],
                flash_palette: &[(0.3, 180, 180, 255), (0.7, 220, 220, 255), (1.0, 255, 255, 255)],
                decay: 0.85, flash_chance: 0.08, spread: (1, 2),
                base_wave_speed: 0.3, base_spatial_freq: 0.5, flash_threshold: 0.3,
            },
            delay: Delay::Random(50, 150),
        },
    },
    ThemeDef {
        name: "lightning", category: "functional",
        kind: ThemeKind::Animated {
            behavior: Behavior::Flash {
                base_palette: &[(0.0, 15, 5, 30), (0.5, 25, 10, 50), (1.0, 35, 15, 60)],
                flash_palette: &[(0.3, 200, 200, 255), (0.6, 240, 240, 255), (1.0, 255, 255, 255)],
                decay: 0.75, flash_chance: 0.06, spread: (2, 4),
                base_wave_speed: 0.2, base_spatial_freq: 0.3, flash_threshold: 0.25,
            },
            delay: Delay::Random(40, 120),
        },
    },
    ThemeDef {
        name: "thunderstorm", category: "functional",
        kind: ThemeKind::Animated {
            behavior: Behavior::Flash {
                base_palette: &[
                    (0.0, 5, 5, 20), (0.3, 10, 15, 50),
                    (0.6, 20, 30, 70), (0.8, 5, 10, 40), (1.0, 5, 5, 20),
                ],
                flash_palette: &[(0.3, 200, 200, 255), (0.7, 240, 240, 255), (1.0, 255, 255, 255)],
                decay: 0.80, flash_chance: 0.05, spread: (2, 4),
                base_wave_speed: 1.2, base_spatial_freq: 0.8, flash_threshold: 0.3,
            },
            delay: Delay::Random(60, 120),
        },
    },
    ThemeDef {
        name: "starfield", category: "functional",
        kind: ThemeKind::Animated {
            behavior: Behavior::Twinkle {
                bg: (2, 2, 8),
                colors: &[(255, 255, 255), (180, 200, 255), (200, 220, 255), (255, 240, 200)],
                on_chance: 0.06, fade_speed: 0.03,
            },
            delay: Delay::Fixed(80),
        },
    },
    ThemeDef {
        name: "pulse", category: "functional",
        kind: ThemeKind::Animated {
            behavior: Behavior::RadiatePulse {
                color: (0, 150, 255), speed: 0.6, width: 0.3,
            },
            delay: Delay::Fixed(50),
        },
    },
    ThemeDef {
        name: "rainbow", category: "functional",
        kind: ThemeKind::Animated {
            behavior: Behavior::HueRotate { speed: 0.1, saturation: 1.0, value: 1.0 },
            delay: Delay::Fixed(60),
        },
    },
    ThemeDef {
        name: "gradient-wave", category: "functional",
        kind: ThemeKind::Animated {
            behavior: Behavior::GradientWave {
                color_a: (0, 80, 255), color_b: (180, 0, 255), speed: 0.5,
            },
            delay: Delay::Fixed(60),
        },
    },
    ThemeDef {
        name: "sunrise", category: "functional",
        kind: ThemeKind::Animated {
            behavior: Behavior::Progression {
                palette: &[
                    (0.0, 60, 0, 0), (0.15, 180, 40, 0), (0.33, 255, 80, 0),
                    (0.50, 255, 160, 15), (0.66, 255, 200, 30),
                    (0.85, 255, 230, 120), (1.0, 255, 240, 180),
                ],
                duration_secs: 600.0, spatial_spread: 0.02,
            },
            delay: Delay::Fixed(500),
        },
    },

    // ── Seasonal ────────────────────────────────────────────────────────
    ThemeDef {
        name: "christmas", category: "seasonal",
        kind: ThemeKind::Animated {
            behavior: Behavior::Alternating {
                colors: &[(200, 10, 10), (10, 180, 20)],
                sparkle: (255, 255, 255), sparkle_chance: 0.1, shift_speed: 0.2,
            },
            delay: Delay::Fixed(100),
        },
    },
    ThemeDef {
        name: "halloween", category: "seasonal",
        kind: ThemeKind::Animated {
            behavior: Behavior::Alternating {
                colors: &[(255, 100, 0), (120, 0, 180)],
                sparkle: (255, 200, 50), sparkle_chance: 0.12, shift_speed: 0.15,
            },
            delay: Delay::Random(80, 150),
        },
    },
    ThemeDef {
        name: "snowfall", category: "seasonal",
        kind: ThemeKind::Animated {
            behavior: Behavior::Particles {
                bg: (5, 5, 20),
                palette: &[(0.0, 40, 50, 80), (0.4, 100, 120, 180), (0.7, 180, 200, 240), (1.0, 240, 245, 255)],
                speed: 0.02, spawn_chance: 0.3, bright_chance: 0.1,
            },
            delay: Delay::Random(120, 200),
        },
    },
];
