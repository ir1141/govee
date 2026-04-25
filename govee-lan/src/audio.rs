//! Real-time audio analysis via PulseAudio monitor source capture.
//!
//! Performs 1024-sample FFT with Hanning windowing, decomposes into 6 frequency
//! bands, tracks RMS energy with adaptive gain, and provides beat detection.
//! Four visualization modes (energy, frequency, beat, drop) map analysis state
//! to per-segment LED colors.

use anyhow::Result;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use libpulse_binding as pulse;
use pulse::mainloop::standard::Mainloop;
use pulse::context::Context;
use pulse::stream::Stream;
use pulse::sample::{Format, Spec};
use pulse::stream::PeekResult;
use rustfft::{FftPlanner, num_complex::Complex};
use std::thread;
use std::time::Instant;

/// Visualization algorithm selection.
#[derive(Debug, Clone, Copy)]
pub enum VisMode {
    Energy,
    Frequency,
    Beat,
    Drop,
    Laser,
}

/// Color palette for the visualization.
#[derive(Debug, Clone, Copy)]
pub enum Palette {
    Fire,
    Ocean,
    Forest,
    Neon,
    Ice,
    Sunset,
    Rainbow,
}

/// Shared state between the capture thread and the main rendering loop.
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
    /// positive-only frame-to-frame bass change (spectral flux for bands 0+1)
    pub bass_flux: f64,
    /// positive-only frame-to-frame treble change (spectral flux for bands 4+5)
    pub treble_flux: f64,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            energy: 0.0,
            bands: [0.0; 6],
            beat: false,
            peak: 0.001,
            bass_flux: 0.0,
            treble_flux: 0.0,
        }
    }
}

/// Interpolate through palette anchor colors based on intensity (0.0-1.0).
pub fn palette_color(palette: Palette, intensity: f64) -> (u8, u8, u8) {
    let anchors: &[(u8, u8, u8)] = match palette {
        Palette::Fire => &[
            (0, 0, 0),
            (128, 0, 0),
            (255, 100, 0),
            (255, 180, 0),
            (255, 240, 180),
        ],
        Palette::Ocean => &[
            (0, 0, 0),
            (0, 0, 128),
            (0, 128, 128),
            (0, 220, 255),
            (255, 255, 255),
        ],
        Palette::Forest => &[
            (0, 0, 0),
            (0, 60, 20),
            (30, 140, 50),
            (80, 200, 80),
            (180, 255, 150),
        ],
        Palette::Neon => &[
            (40, 0, 60),
            (180, 0, 180),
            (255, 20, 147),
            (0, 100, 255),
            (0, 255, 255),
        ],
        Palette::Ice => &[
            (0, 0, 0),
            (20, 20, 80),
            (60, 100, 200),
            (150, 200, 255),
            (230, 240, 255),
        ],
        Palette::Sunset => &[
            (0, 0, 0),
            (128, 0, 40),
            (220, 60, 20),
            (255, 140, 50),
            (255, 200, 100),
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
    crate::colors::lerp_color_chain(anchors, intensity)
}

const SAMPLE_RATE: u32 = 44100;
const FFT_SIZE: usize = 1024;
const BUFFER_SIZE: usize = 1024;
/// Minimum 200ms between beat triggers to prevent double-counting.
const BEAT_COOLDOWN_MS: u128 = 200;
/// Energy must exceed 1.5× the recent average to register as a beat.
const BEAT_THRESHOLD: f64 = 1.5;
/// ~1 second of history at 44100 Hz / 1024-sample frames (~43 frames/sec).
const ENERGY_HISTORY: usize = 43;
/// Below this RMS the signal is treated as silence to avoid amplifying noise.
const NOISE_GATE: f64 = 5e-3;
/// Band EMA smoothing factor — 0.4 × raw + 0.6 × previous.
/// Reduces flicker in frequency mode without perceptible lag.
const BAND_SMOOTH: f64 = 0.4;
/// Flux must exceed this to trigger a drop flash.
/// 0.3 = normalized band jumped 30% of its range in one frame (~23ms).
const FLUX_TRIGGER: f64 = 0.3;

/// Frequency boundaries in Hz for 6 perceptual bands:
/// sub-bass, bass, low-mid, mid, upper-mid, brilliance.
const BAND_EDGES: [(f64, f64); 6] = [
    (20.0, 150.0),
    (150.0, 400.0),
    (400.0, 1000.0),
    (1000.0, 2500.0),
    (2500.0, 6000.0),
    (6000.0, 20000.0),
];

/// Asymmetric exponential filter — fast rise, slow decay.
/// Tracks a peak value that quickly jumps up to loud signals but slowly
/// decays, so dividing by it always yields a full 0.0-1.0 dynamic range.
struct ExpFilter {
    value: f64,
    alpha_rise: f64,
    alpha_decay: f64,
}

impl ExpFilter {
    fn new(initial: f64, alpha_rise: f64, alpha_decay: f64) -> Self {
        Self { value: initial, alpha_rise, alpha_decay }
    }

    fn update(&mut self, sample: f64) -> f64 {
        let alpha = if sample > self.value { self.alpha_rise } else { self.alpha_decay };
        self.value = alpha * sample + (1.0 - alpha) * self.value;
        // Never decay below noise gate — prevents normalizing silence to full range
        self.value = self.value.max(NOISE_GATE);
        self.value
    }
}

/// Spawns a background capture thread and provides thread-safe access to the
/// latest audio analysis state.
pub struct AudioAnalyzer {
    pub state: Arc<Mutex<AudioState>>,
    thread: Option<thread::JoinHandle<()>>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl AudioAnalyzer {
    pub fn new() -> Result<Self> {
        let state = Arc::new(Mutex::new(AudioState::default()));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        let state_clone = Arc::clone(&state);
        let running_clone = Arc::clone(&running);

        let thread = thread::spawn(move || {
            if let Err(e) = capture_loop(state_clone, running_clone) {
                eprintln!("Audio capture error: {e}");
            }
        });

        // Give PulseAudio a moment to connect
        thread::sleep(std::time::Duration::from_millis(200));

        Ok(Self {
            state,
            thread: Some(thread),
            running,
        })
    }

    /// Returns the latest analysis snapshot.
    ///
    /// Recovers gracefully from a poisoned mutex if the capture thread panicked.
    pub fn get_state(&self) -> AudioState {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Drop for AudioAnalyzer {
    fn drop(&mut self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn find_monitor_source(mainloop: &mut Mainloop, context: &Context) -> Result<String> {
    use pulse::operation::State;
    use std::cell::RefCell;
    use std::rc::Rc;

    let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let result_clone = Rc::clone(&result);

    let op = context.introspect().get_server_info(move |info| {
        if let Some(ref sink_name) = info.default_sink_name {
            *result_clone.borrow_mut() = Some(format!("{}.monitor", sink_name));
        }
    });

    loop {
        match op.get_state() {
            State::Done => break,
            State::Running => {
                mainloop.iterate(true);
            }
            State::Cancelled => anyhow::bail!("Server info query cancelled"),
        }
    }

    let borrowed = result.borrow().clone();
    borrowed.ok_or_else(|| anyhow::anyhow!("No default sink found"))
}

fn capture_loop(
    state: Arc<Mutex<AudioState>>,
    running: Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    let mut mainloop = Mainloop::new().ok_or_else(|| anyhow::anyhow!("Failed to create PulseAudio mainloop"))?;
    let mut context = Context::new(&mainloop, "govee-audio")
        .ok_or_else(|| anyhow::anyhow!("Failed to create PulseAudio context"))?;

    context.connect(None, pulse::context::FlagSet::NOFLAGS, None)
        .map_err(|e| anyhow::anyhow!("PA connect: {e}"))?;

    // Wait for context to be ready
    loop {
        mainloop.iterate(true);
        match context.get_state() {
            pulse::context::State::Ready => break,
            pulse::context::State::Failed | pulse::context::State::Terminated => {
                anyhow::bail!("PulseAudio connection failed");
            }
            _ => {}
        }
    }

    let monitor_source = find_monitor_source(&mut mainloop, &context)?;
    eprintln!("Capturing from: {monitor_source}");

    let spec = Spec {
        format: Format::FLOAT32NE,
        channels: 1,
        rate: SAMPLE_RATE,
    };
    if !spec.is_valid() {
        anyhow::bail!("Invalid PulseAudio sample spec (44100Hz mono f32)");
    }

    let mut stream = Stream::new(&mut context, "govee-capture", &spec, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to create PA stream"))?;

    // Request low-latency buffer: ~23ms fragments to minimize lag
    let target_bytes = BUFFER_SIZE as u32 * std::mem::size_of::<f32>() as u32;
    let buf_attr = pulse::def::BufferAttr {
        maxlength: u32::MAX,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: target_bytes,
    };
    stream.connect_record(
        Some(&monitor_source),
        Some(&buf_attr),
        pulse::stream::FlagSet::ADJUST_LATENCY,
    ).map_err(|e| anyhow::anyhow!("PA record connect: {e}"))?;

    // Wait for stream to be ready
    loop {
        mainloop.iterate(true);
        match stream.get_state() {
            pulse::stream::State::Ready => break,
            pulse::stream::State::Failed | pulse::stream::State::Terminated => {
                anyhow::bail!("PA stream failed");
            }
            _ => {}
        }
    }

    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut sample_buf: Vec<f32> = Vec::with_capacity(BUFFER_SIZE);
    let mut energy_history: VecDeque<f64> = VecDeque::with_capacity(ENERGY_HISTORY);
    let mut last_beat = Instant::now();

    // Adaptive gain: fast rise (0.99) catches transients, slow decay (0.02) releases in ~2.5s
    let mut rms_gain = ExpFilter::new(NOISE_GATE, 0.99, 0.02);
    let mut band_gains: [ExpFilter; 6] = std::array::from_fn(|_| ExpFilter::new(NOISE_GATE, 0.99, 0.02));
    let mut prev_bass: f64 = 0.0;
    let mut prev_treble: f64 = 0.0;
    let mut smoothed_bands = [0.0_f64; 6];

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        mainloop.iterate(true);

        // Drain ALL available fragments — don't block between them
        loop {
            match stream.peek() {
                Ok(PeekResult::Data(data)) => {
                    if let Ok(floats) = bytemuck::try_cast_slice::<u8, f32>(data) {
                        sample_buf.extend_from_slice(floats);
                    }
                    stream.discard().ok();
                }
                Ok(PeekResult::Hole(_)) => {
                    stream.discard().ok();
                }
                Ok(PeekResult::Empty) => break,
                Err(_) => break,
            }
        }

        // Process when we have enough samples
        if sample_buf.len() < BUFFER_SIZE {
            continue;
        }

        // Skip to the LATEST samples — discard stale data to minimize latency
        if sample_buf.len() > BUFFER_SIZE {
            let skip = sample_buf.len() - BUFFER_SIZE;
            sample_buf.drain(..skip);
        }

        let samples: Vec<f32> = sample_buf.drain(..BUFFER_SIZE).collect();

        // RMS energy
        let rms = (samples.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>()
            / samples.len() as f64)
            .sqrt();


        // Hanning window reduces spectral leakage before FFT:
        // w(n) = 0.5 × (1 − cos(2πn / (N−1)))
        let window_start = samples.len() - FFT_SIZE;
        let mut fft_input: Vec<Complex<f64>> = samples[window_start..]
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let window = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (FFT_SIZE - 1) as f64).cos());
                Complex::new(s as f64 * window, 0.0)
            })
            .collect();

        fft.process(&mut fft_input);

        // Frequency band analysis
        let bin_hz = SAMPLE_RATE as f64 / FFT_SIZE as f64;
        let mut bands = [0.0_f64; 6];
        for (band_idx, &(lo, hi)) in BAND_EDGES.iter().enumerate() {
            let bin_lo = (lo / bin_hz).ceil() as usize;
            let bin_hi = ((hi / bin_hz).floor() as usize).min(FFT_SIZE / 2);
            if bin_lo >= bin_hi {
                continue;
            }
            let sum: f64 = fft_input[bin_lo..bin_hi]
                .iter()
                .map(|c| c.norm())
                .sum();
            let avg = sum / (bin_hi - bin_lo) as f64;
            // Adaptive per-band normalization
            let peak = band_gains[band_idx].update(avg);
            bands[band_idx] = if peak > NOISE_GATE { (avg / peak).clamp(0.0, 1.0) } else { 0.0 };
        }

        // Spectral flux — positive-only frame-to-frame delta for onset detection
        let bass = bands[0].max(bands[1]);
        let treble = bands[4].max(bands[5]);
        let bass_flux = (bass - prev_bass).max(0.0);
        let treble_flux = (treble - prev_treble).max(0.0);
        prev_bass = bass;
        prev_treble = treble;

        // Per-band temporal smoothing to reduce flicker
        for i in 0..6 {
            smoothed_bands[i] = BAND_SMOOTH * bands[i] + (1.0 - BAND_SMOOTH) * smoothed_bands[i];
        }

        // Adaptive RMS normalization — track peak with fast rise / slow decay
        let rms_peak = rms_gain.update(rms);
        let energy = if rms < NOISE_GATE {
            0.0 // squelch: silence → LEDs off
        } else {
            (rms / rms_peak).clamp(0.0, 1.0)
        };

        // Beat detection — uses raw RMS (not normalized energy) so the
        // threshold comparison isn't compressed by adaptive gain
        energy_history.push_back(rms);
        if energy_history.len() > ENERGY_HISTORY {
            energy_history.pop_front();
        }
        let avg_rms = energy_history.iter().sum::<f64>() / energy_history.len() as f64;
        let beat = rms > NOISE_GATE
            && rms > avg_rms * BEAT_THRESHOLD
            && last_beat.elapsed().as_millis() > BEAT_COOLDOWN_MS;
        if beat {
            last_beat = Instant::now();
        }

        // Update shared state
        let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
        s.energy = energy;
        s.bands = smoothed_bands;
        s.beat = beat;
        s.peak = rms;
        s.bass_flux = bass_flux;
        s.treble_flux = treble_flux;
    }

    Ok(())
}

/// Map audio state to segment colors based on visualization mode
pub fn map_colors(
    audio: &AudioState,
    mode: VisMode,
    palette: Palette,
    n_seg: usize,
    t: f64,
    beat_hue: &mut f64,
    beat_decay: &mut f64,
) -> Vec<(u8, u8, u8)> {
    match mode {
        VisMode::Energy => map_energy(audio, palette, n_seg, t),
        VisMode::Frequency => map_frequency(audio, palette, n_seg),
        VisMode::Beat => map_beat(audio, palette, n_seg, beat_hue, beat_decay),
        VisMode::Drop => map_drop(audio, n_seg, beat_hue, beat_decay),
        VisMode::Laser => map_laser(audio, n_seg, t, beat_hue, beat_decay),
    }
}

/// DJ-style laser sweep: two narrow saturated beams scan across a black strip,
/// swap color on beats (rate-limited), and white-strobe on hard bass-flux spikes.
///
/// Reuses `last_swap_t` (was `beat_hue`) as the timestamp of the last color
/// change so swaps can be gated to a minimum interval, and `strobe_decay` as
/// the bass-flash envelope.
fn map_laser(
    audio: &AudioState,
    n_seg: usize,
    t: f64,
    last_swap_t: &mut f64,
    strobe_decay: &mut f64,
) -> Vec<(u8, u8, u8)> {
    // Beam color palette: classic green, hot magenta, ice cyan.
    const BEAMS: [(u8, u8, u8); 3] = [(0, 255, 30), (255, 0, 200), (0, 220, 255)];
    /// Minimum seconds between color swaps — keeps the look from flickering on every kick.
    const SWAP_COOLDOWN: f64 = 1.2;
    /// Stricter than FLUX_TRIGGER (0.3) so only real drops strobe, not every bass note.
    const LASER_STROBE_TRIGGER: f64 = 0.55;

    if audio.beat && (t - *last_swap_t) >= SWAP_COOLDOWN {
        *last_swap_t = t;
    }
    if audio.bass_flux > LASER_STROBE_TRIGGER {
        *strobe_decay = 1.0;
    } else {
        *strobe_decay *= 0.55;
    }

    // Color index advances each swap; golden-ratio multiplier ensures every
    // gated swap actually picks a different palette entry.
    let beam_color_idx = (*last_swap_t * 1.618).floor() as i64;
    let beam_color_idx = beam_color_idx.rem_euclid(BEAMS.len() as i64) as usize;

    // Mostly constant sweep speed; mild energy modulation so silence isn't static.
    let speed = 5.0 + audio.energy * 2.0;
    let span = n_seg as f64;
    // Triangle-wave bounce: 0 → span → 0 over period 2*span/speed
    let phase = (t * speed) % (2.0 * span);
    let pos_a = if phase < span { phase } else { 2.0 * span - phase };
    // Second beam runs counter-phase for an X-crossing pattern
    let phase_b = (phase + span) % (2.0 * span);
    let pos_b = if phase_b < span { phase_b } else { 2.0 * span - phase_b };

    let beam_color = BEAMS[beam_color_idx];
    // Counter-beam picks the next color so the two beams are always distinct
    let beam_color_b = BEAMS[(beam_color_idx + 1) % BEAMS.len()];

    let strobe = (*strobe_decay).powi(2);

    (0..n_seg)
        .map(|i| {
            let x = i as f64 + 0.5;
            // Beam intensity profile: 1.0 at center, 0.4 at ±1, 0.08 at ±2 — sharp falloff
            let intensity_at = |pos: f64| -> f64 {
                let d = (x - pos).abs();
                if d < 0.5 { 1.0 }
                else if d < 1.5 { 0.4 }
                else if d < 2.5 { 0.08 }
                else { 0.0 }
            };
            let ia = intensity_at(pos_a);
            let ib = intensity_at(pos_b);

            // Additive mix of the two beams, clamped per channel
            let r = (beam_color.0 as f64 * ia + beam_color_b.0 as f64 * ib).min(255.0);
            let g = (beam_color.1 as f64 * ia + beam_color_b.1 as f64 * ib).min(255.0);
            let b = (beam_color.2 as f64 * ia + beam_color_b.2 as f64 * ib).min(255.0);

            // Bass flash: flood the strip with the active beam color (never white)
            let r = (r + (beam_color.0 as f64 - r) * strobe).clamp(0.0, 255.0) as u8;
            let g = (g + (beam_color.1 as f64 - g) * strobe).clamp(0.0, 255.0) as u8;
            let b = (b + (beam_color.2 as f64 - b) * strobe).clamp(0.0, 255.0) as u8;
            (r, g, b)
        })
        .collect()
}

fn map_energy(audio: &AudioState, palette: Palette, n_seg: usize, t: f64) -> Vec<(u8, u8, u8)> {
    (0..n_seg)
        .map(|i| {
            // Per-segment variation scales with energy — silent = truly dark
            let offset = (t * 2.0 + i as f64 * 0.6).sin() * 0.08 * audio.energy;
            let intensity = (audio.energy + offset).clamp(0.0, 1.0);
            palette_color(palette, intensity)
        })
        .collect()
}

fn map_frequency(audio: &AudioState, palette: Palette, n_seg: usize) -> Vec<(u8, u8, u8)> {
    // Distribute 6 bands across N segments, weighting bass higher
    (0..n_seg)
        .map(|i| {
            // Map segment position to band index with bass bias
            // First 40% of segments cover bass (band 0), rest distributed evenly
            let pos = i as f64 / n_seg as f64;
            let band_idx = if pos < 0.4 {
                0
            } else {
                let remaining = (pos - 0.4) / 0.6;
                (1.0 + remaining * 4.99).min(5.0) as usize
            };
            palette_color(palette, audio.bands[band_idx])
        })
        .collect()
}

fn map_beat(
    audio: &AudioState,
    palette: Palette,
    n_seg: usize,
    beat_hue: &mut f64,
    beat_decay: &mut f64,
) -> Vec<(u8, u8, u8)> {
    if audio.beat {
        *beat_decay = 1.0;
        *beat_hue = (*beat_hue + 0.2) % 1.0;
    } else {
        // Exponential decay: 0.92 per frame ≈ 8-frame half-life (~130ms at 60fps)
        *beat_decay *= 0.92;
    }

    let base_intensity = 0.1 + audio.energy * 0.15;
    let flash_intensity = *beat_decay;
    let intensity = base_intensity + flash_intensity * (1.0 - base_intensity);

    (0..n_seg)
        .map(|_| palette_color(palette, intensity.clamp(0.0, 1.0)))
        .collect()
}

/// Drop mode: stays dark, flashes on bass onsets (deep red/purple) or treble onsets (cyan/white).
/// Uses spectral flux (frame-to-frame change) so sustained bass doesn't cause continuous flash.
fn map_drop(
    audio: &AudioState,
    n_seg: usize,
    bass_decay: &mut f64,
    treble_decay: &mut f64,
) -> Vec<(u8, u8, u8)> {
    if audio.bass_flux > FLUX_TRIGGER {
        *bass_decay = 1.0;
    } else {
        // Fast fade: 0.7 per frame ≈ 2-frame half-life — visible ~4-5 frames then gone
        *bass_decay *= 0.7;
    }

    if audio.treble_flux > FLUX_TRIGGER {
        *treble_decay = 1.0;
    } else {
        *treble_decay *= 0.65;
    }

    // Smooth but decisive — cubic curve so it snaps on, eases off
    let b = (*bass_decay).powi(2);
    let t = (*treble_decay).powi(2);

    (0..n_seg)
        .map(|_| {
            if b > t && b > 0.02 {
                // Bass drop: deep red → bright magenta
                let r = (80.0 + 175.0 * b) as u8;
                let g = 0;
                let b_ch = (40.0 * b) as u8;
                (r, g, b_ch)
            } else if t > 0.02 {
                // Treble hit: cyan → white
                let r = (180.0 * t) as u8;
                let g = (220.0 * t) as u8;
                let b_ch = (255.0 * t) as u8;
                (r, g, b_ch)
            } else {
                (0, 0, 0) // dark
            }
        })
        .collect()
}
