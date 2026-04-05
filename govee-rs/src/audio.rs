use std::sync::{Arc, Mutex};

/// Which visualization mode to use
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum VisMode {
    Energy,
    Frequency,
    Beat,
}

/// Color palette for visualization
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Palette {
    Fire,
    Ocean,
    Neon,
    Rainbow,
}

/// Shared state between capture thread and main loop
#[derive(Debug, Clone)]
pub struct AudioState {
    /// 0.0-1.0 normalized RMS energy
    pub energy: f64,
    /// 6 frequency bands: bass, low-mid, mid, upper-mid, presence, brilliance
    pub bands: [f64; 6],
    /// true on detected beat onset
    pub beat: bool,
    /// recent peak for auto-gain
    pub peak: f64,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            energy: 0.0,
            bands: [0.0; 6],
            beat: false,
            peak: 0.001,
        }
    }
}

/// Interpolate through palette anchor colors based on intensity (0.0-1.0)
pub fn palette_color(palette: Palette, intensity: f64) -> (u8, u8, u8) {
    let t = intensity.clamp(0.0, 1.0);
    let anchors: &[(u8, u8, u8)] = match palette {
        Palette::Fire => &[
            (0, 0, 0),
            (128, 0, 0),
            (255, 100, 0),
            (255, 220, 50),
            (255, 255, 255),
        ],
        Palette::Ocean => &[
            (0, 0, 0),
            (0, 0, 128),
            (0, 128, 128),
            (0, 220, 255),
            (255, 255, 255),
        ],
        Palette::Neon => &[
            (40, 0, 60),
            (180, 0, 180),
            (255, 20, 147),
            (0, 100, 255),
            (0, 255, 255),
        ],
        Palette::Rainbow => &[
            (255, 0, 0),
            (255, 165, 0),
            (255, 255, 0),
            (0, 255, 0),
            (0, 0, 255),
            (148, 0, 211),
        ],
    };
    let n = anchors.len() - 1;
    let pos = t * n as f64;
    let idx = (pos as usize).min(n - 1);
    let frac = pos - idx as f64;
    let (r1, g1, b1) = anchors[idx];
    let (r2, g2, b2) = anchors[idx + 1];
    (
        (r1 as f64 + (r2 as f64 - r1 as f64) * frac) as u8,
        (g1 as f64 + (g2 as f64 - g1 as f64) * frac) as u8,
        (b1 as f64 + (b2 as f64 - b1 as f64) * frac) as u8,
    )
}
