use anyhow::Result;
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

/// Which visualization mode to use
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum VisMode {
    Energy,
    Frequency,
    Beat,
    Drop,
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

const SAMPLE_RATE: u32 = 44100;
const FFT_SIZE: usize = 1024;
const BUFFER_SIZE: usize = 1024; // ~23ms window for responsive updates
const BEAT_COOLDOWN_MS: u128 = 200;
const BEAT_THRESHOLD: f64 = 1.5;
const ENERGY_HISTORY: usize = 43; // ~1 second at 44100/1024
const NOISE_GATE: f64 = 5e-3; // squelch: signal below this raw RMS is silence

/// Frequency band boundaries in Hz
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

    pub fn get_state(&self) -> AudioState {
        self.state.lock().unwrap().clone()
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
    assert!(spec.is_valid());

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
    let mut energy_history: Vec<f64> = Vec::with_capacity(ENERGY_HISTORY);
    let mut last_beat = Instant::now();

    // Adaptive gain: fast rise (0.99) catches transients, moderate decay (0.05) releases in ~1s
    let mut rms_gain = ExpFilter::new(NOISE_GATE, 0.99, 0.05);
    let mut band_gains: [ExpFilter; 6] = std::array::from_fn(|_| ExpFilter::new(NOISE_GATE, 0.99, 0.05));

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        mainloop.iterate(true);

        // Drain ALL available fragments — don't block between them
        loop {
            match stream.peek() {
                Ok(PeekResult::Data(data)) => {
                    let floats: &[f32] = bytemuck::cast_slice(data);
                    sample_buf.extend_from_slice(floats);
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


        // Hanning window + FFT on last FFT_SIZE samples
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

        // Adaptive RMS normalization — track peak with fast rise / slow decay
        let rms_peak = rms_gain.update(rms);
        let energy = if rms < NOISE_GATE {
            0.0 // squelch: silence → LEDs off
        } else {
            (rms / rms_peak).clamp(0.0, 1.0)
        };

        // Beat detection
        energy_history.push(energy);
        if energy_history.len() > ENERGY_HISTORY {
            energy_history.remove(0);
        }
        let avg_energy = energy_history.iter().sum::<f64>() / energy_history.len() as f64;
        let beat = energy > avg_energy * BEAT_THRESHOLD
            && last_beat.elapsed().as_millis() > BEAT_COOLDOWN_MS;
        if beat {
            last_beat = Instant::now();
        }

        // Update shared state
        let mut s = state.lock().unwrap();
        s.energy = energy;
        s.bands = bands;
        s.beat = beat;
        s.peak = rms;
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
    }
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
        *beat_decay *= 0.92; // exponential decay
    }

    let base_intensity = 0.1 + audio.energy * 0.15;
    let flash_intensity = *beat_decay;
    let intensity = base_intensity + flash_intensity * (1.0 - base_intensity);

    (0..n_seg)
        .map(|_| palette_color(palette, intensity.clamp(0.0, 1.0)))
        .collect()
}

/// Drop mode: stays dark, flashes on bass drops (deep red/purple) or treble hits (cyan/white).
/// beat_hue tracks bass decay, beat_decay tracks treble decay.
fn map_drop(
    audio: &AudioState,
    n_seg: usize,
    bass_decay: &mut f64,
    treble_decay: &mut f64,
) -> Vec<(u8, u8, u8)> {
    let bass = audio.bands[0].max(audio.bands[1]);
    let treble = audio.bands[4].max(audio.bands[5]);

    const TRIGGER: f64 = 0.65;

    if bass > TRIGGER {
        *bass_decay = 1.0;
    } else {
        *bass_decay *= 0.7; // fast fade — stays visible ~4-5 frames then gone
    }

    if treble > TRIGGER {
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
