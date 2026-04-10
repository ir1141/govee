//! Wayland screen capture via the `wlr-screencopy-unstable-v1` protocol.
//!
//! [`ScreenCapturer`] maintains a persistent Wayland connection and reuses
//! shared-memory buffers across frames. [`CapturedFrame`] provides histogram-based
//! dominant-color extraction for ambilight-style LED output.

use anyhow::Result;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
use nix::sys::mman::{mmap, munmap, MapFlags, ProtFlags};
use std::ffi::CString;
use std::num::NonZeroUsize;
use std::os::fd::AsFd;
use std::os::unix::io::OwnedFd;
use std::sync::{Arc, Mutex};
use wayland_client::protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1, zwlr_screencopy_manager_v1,
};

/// A captured screen frame with raw pixel data and dimensions.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: wl_shm::Format,
    pub data: Vec<u8>,
}

/// Subsampling step — skip pixels for performance.
/// At 4K (3840x2160) with step=4, we sample ~500K pixels instead of ~8M.
const SAMPLE_STEP: u32 = 4;

struct SegmentAccum {
    histogram: [u32; 256],
    r_sum: [u64; 256],
    g_sum: [u64; 256],
    b_sum: [u64; 256],
    total: u32,
}

impl SegmentAccum {
    fn new() -> Self {
        Self {
            histogram: [0u32; 256],
            r_sum: [0u64; 256],
            g_sum: [0u64; 256],
            b_sum: [0u64; 256],
            total: 0,
        }
    }

    fn prominent_color(&self) -> (u8, u8, u8) {
        if self.total == 0 {
            return (0, 0, 0);
        }

        let threshold_count = self.total.div_ceil(5);
        let mut cumulative = 0u32;
        let mut cutoff: usize = 0;
        for bucket in (0..=255usize).rev() {
            cumulative += self.histogram[bucket];
            if cumulative >= threshold_count {
                cutoff = bucket;
                break;
            }
        }

        let mut r_total = 0u64;
        let mut g_total = 0u64;
        let mut b_total = 0u64;
        let mut count = 0u64;
        for bucket in cutoff..=255 {
            let h = self.histogram[bucket] as u64;
            r_total += self.r_sum[bucket];
            g_total += self.g_sum[bucket];
            b_total += self.b_sum[bucket];
            count += h;
        }

        if count == 0 {
            return (0, 0, 0);
        }
        (
            (r_total / count) as u8,
            (g_total / count) as u8,
            (b_total / count) as u8,
        )
    }
}

impl CapturedFrame {
    /// Extract colors by sampling full-height vertical columns, split into N segments.
    /// Single row-major pass over the frame buffer builds per-segment luminance
    /// histograms with bucketed RGB sums, then computes top-20% averages without
    /// re-reading the frame data.
    pub fn extract_segment_colors(&self, segments: usize) -> Vec<(u8, u8, u8)> {
        let segments = segments.max(1);
        let seg_w = self.width / segments as u32;

        if seg_w == 0 {
            return vec![(0, 0, 0); segments];
        }

        let mut accums: Vec<SegmentAccum> = (0..segments).map(|_| SegmentAccum::new()).collect();
        let bpp: u32 = 4;
        let last_seg = segments - 1;

        // Phase 1: single row-major scan, bucketing pixels into per-segment accumulators.
        let mut row = 0u32;
        while row < self.height {
            let row_offset = row * self.stride;
            let mut col = 0u32;
            while col < self.width {
                let offset = (row_offset + col * bpp) as usize;
                if offset + 3 < self.data.len() {
                    let seg_idx = ((col / seg_w) as usize).min(last_seg);

                    let b = self.data[offset];
                    let g = self.data[offset + 1];
                    let r = self.data[offset + 2];
                    // Rec. 601 luma: (0.299, 0.587, 0.114) × 256 = (77, 150, 29). >> 8 divides by 256.
                    let lum = ((77 * r as u32 + 150 * g as u32 + 29 * b as u32) >> 8) as usize;

                    let acc = &mut accums[seg_idx];
                    acc.histogram[lum] += 1;
                    acc.r_sum[lum] += r as u64;
                    acc.g_sum[lum] += g as u64;
                    acc.b_sum[lum] += b as u64;
                    acc.total += 1;
                }
                col += SAMPLE_STEP;
            }
            row += SAMPLE_STEP;
        }

        // Phase 2: compute prominent color per segment from accumulators only.
        accums.iter().map(SegmentAccum::prominent_color).collect()
    }
}

struct FrameState {
    width: u32,
    height: u32,
    stride: u32,
    format: wl_shm::Format,
    ready: bool,
    failed: bool,
    buffer_info_received: bool,
}

/// Persistent shared-memory buffer reused across captures to avoid per-frame allocation.
struct ShmBuffer {
    ptr: *mut u8,
    size: usize,
    fd: OwnedFd,
}

impl ShmBuffer {
    fn new(size: usize) -> Result<Self, ()> {
        if size == 0 {
            return Err(());
        }
        let name = CString::new("govee-screencopy").unwrap();
        let fd = memfd_create(&name, MemFdCreateFlag::MFD_CLOEXEC).map_err(|_| ())?;
        nix::unistd::ftruncate(&fd, size as i64).map_err(|_| ())?;

        // SAFETY: `fd` is a valid memfd we just created, `size` is non-zero (checked
        // by NonZeroUsize). MAP_SHARED on a memfd is well-defined. The pointer is used
        // only within ShmBuffer's lifetime and unmapped in Drop.
        let ptr = unsafe {
            mmap(
                None,
                NonZeroUsize::new(size).ok_or(())?,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                &fd,
                0,
            )
        }
        .map_err(|_| ())?;

        Ok(Self {
            ptr: ptr.as_ptr() as *mut u8,
            size,
            fd,
        })
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` was returned by mmap in `new()` and `self.size` is the
        // original mapping length. Called exactly once via Drop.
        unsafe {
            let _ = munmap(
                std::ptr::NonNull::new(self.ptr as *mut std::ffi::c_void)
                    .expect("mmap returned non-null"),
                self.size,
            );
        }
    }
}

struct WaylandState {
    shm: Option<wl_shm::WlShm>,
    screencopy_manager: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
    outputs: Vec<(wl_output::WlOutput, String)>,
    frame_state: Arc<Mutex<FrameState>>,
    buffer: Option<ShmBuffer>,
    wl_buffer: Option<wl_buffer::WlBuffer>,
}

impl WaylandState {
    fn new() -> Self {
        Self {
            shm: None,
            screencopy_manager: None,
            outputs: Vec::new(),
            frame_state: Arc::new(Mutex::new(FrameState {
                width: 0,
                height: 0,
                stride: 0,
                format: wl_shm::Format::Xrgb8888,
                ready: false,
                failed: false,
                buffer_info_received: false,
            })),
            buffer: None,
            wl_buffer: None,
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_shm" => {
                    state.shm =
                        Some(registry.bind::<wl_shm::WlShm, _, Self>(name, version.min(1), qh, ()));
                }
                "wl_output" => {
                    let output = registry.bind::<wl_output::WlOutput, _, Self>(
                        name,
                        version.min(4),
                        qh,
                        (),
                    );
                    state.outputs.push((output, String::new()));
                }
                "zwlr_screencopy_manager_v1" => {
                    state.screencopy_manager = Some(
                        registry.bind::<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1, _, Self>(
                            name,
                            version.min(3),
                            qh,
                            (),
                        ),
                    );
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_shm::WlShm, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _shm: &wl_shm::WlShm,
        _event: wl_shm::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_output::WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event {
            if let Some(entry) = state.outputs.iter_mut().find(|(o, _)| o == output) {
                entry.1 = name;
            }
        }
    }
}

impl Dispatch<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _manager: &zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
        _event: zwlr_screencopy_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        frame: &zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format: WEnum::Value(fmt),
                width,
                height,
                stride,
            } => {
                let mut fs = state.frame_state.lock().unwrap_or_else(|e| e.into_inner());
                if !fs.buffer_info_received
                    || fmt == wl_shm::Format::Xrgb8888
                    || fmt == wl_shm::Format::Argb8888
                {
                    fs.width = width;
                    fs.height = height;
                    fs.stride = stride;
                    fs.format = fmt;
                    fs.buffer_info_received = true;
                }
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => {
                let fs = state.frame_state.lock().unwrap_or_else(|e| e.into_inner());
                let size = (fs.stride as usize).checked_mul(fs.height as usize).unwrap_or(0);
                let format = fs.format;
                let width = fs.width as i32;
                let height = fs.height as i32;
                let stride = fs.stride as i32;
                drop(fs);

                if size == 0 {
                    state.frame_state.lock().unwrap_or_else(|e| e.into_inner()).failed = true;
                    return;
                }

                // Reallocate only if size changed
                let needs_new = match &state.buffer {
                    Some(buf) => buf.size != size,
                    None => true,
                };
                if needs_new {
                    state.buffer = None; // drop old buffer first
                    match ShmBuffer::new(size) {
                        Ok(buf) => state.buffer = Some(buf),
                        Err(()) => {
                            state.frame_state.lock().unwrap_or_else(|e| e.into_inner()).failed = true;
                            return;
                        }
                    }
                }

                let buf = state.buffer.as_ref().unwrap();

                // Create wl_shm_pool and wl_buffer from the persistent shm fd
                let shm = state.shm.as_ref().unwrap();
                let pool = shm.create_pool(buf.fd.as_fd(), size as i32, qh, ());
                let wl_buf = pool.create_buffer(0, width, height, stride, format, qh, ());
                pool.destroy();

                // Destroy previous wl_buffer before replacing
                if let Some(old) = state.wl_buffer.take() {
                    old.destroy();
                }
                frame.copy(&wl_buf);
                state.wl_buffer = Some(wl_buf);
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                state.frame_state.lock().unwrap_or_else(|e| e.into_inner()).ready = true;
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                state.frame_state.lock().unwrap_or_else(|e| e.into_inner()).failed = true;
            }
            _ => {}
        }
    }
}

delegate_noop!(WaylandState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(WaylandState: ignore wl_buffer::WlBuffer);

/// Persistent screen capturer that holds the Wayland connection open and reuses
/// shm buffers across frames.
pub struct ScreenCapturer {
    _conn: Connection,
    queue: EventQueue<WaylandState>,
    state: WaylandState,
}

impl ScreenCapturer {
    /// Connect to the Wayland compositor and bind screencopy globals.
    pub fn new() -> Result<Self> {
        let conn = Connection::connect_to_env()?;
        let display = conn.display();
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        let mut state = WaylandState::new();

        display.get_registry(&qh, ());
        queue.roundtrip(&mut state)?;
        queue.roundtrip(&mut state)?; // second roundtrip for output names

        if state.screencopy_manager.is_none() {
            anyhow::bail!("Compositor does not support wlr-screencopy-unstable-v1");
        }
        if state.shm.is_none() {
            anyhow::bail!("No wl_shm global found");
        }

        Ok(Self { _conn: conn, queue, state })
    }

    /// List available output names.
    pub fn outputs(&self) -> Vec<String> {
        self.state.outputs.iter().map(|(_, name)| name.clone()).collect()
    }

    /// Capture a frame from the specified output (or first output if None).
    pub fn capture(&mut self, output_name: Option<&str>) -> Result<CapturedFrame> {
        let output = if let Some(name) = output_name {
            self.state
                .outputs
                .iter()
                .find(|(_, n)| n == name)
                .map(|(o, _)| o.clone())
                .ok_or_else(|| anyhow::anyhow!("Output '{}' not found. Available: {:?}", name, self.outputs()))?
        } else {
            self.state
                .outputs
                .first()
                .map(|(o, _)| o.clone())
                .ok_or_else(|| anyhow::anyhow!("No outputs available"))?
        };

        // Reset frame state (buffer stays allocated)
        {
            let mut fs = self.state.frame_state.lock().unwrap_or_else(|e| e.into_inner());
            fs.ready = false;
            fs.failed = false;
            fs.buffer_info_received = false;
        }

        let qh = self.queue.handle();
        let manager = self.state.screencopy_manager.as_ref().unwrap();
        let screencopy_frame = manager.capture_output(0, &output, &qh, ());

        // Dispatch events until frame is ready, failed, or timed out
        loop {
            self.queue.flush()?;

            if let Some(guard) = self.queue.prepare_read() {
                let fd = guard.connection_fd();
                let mut poll_fds = [PollFd::new(fd, PollFlags::POLLIN)];
                match poll(&mut poll_fds, PollTimeout::from(2000u16)) {
                    Ok(0) => {
                        drop(guard);
                        screencopy_frame.destroy();
                        anyhow::bail!("Screen capture timed out");
                    }
                    Ok(_) => {
                        let _ = guard.read();
                    }
                    Err(e) => {
                        drop(guard);
                        screencopy_frame.destroy();
                        return Err(anyhow::anyhow!(e));
                    }
                }
            }

            self.queue.dispatch_pending(&mut self.state)?;

            let fs = self.state.frame_state.lock().unwrap_or_else(|e| e.into_inner());
            if fs.ready {
                break;
            }
            if fs.failed {
                screencopy_frame.destroy();
                anyhow::bail!("Screen capture failed");
            }
        }

        // Destroy the screencopy frame protocol object
        screencopy_frame.destroy();

        // Destroy the wl_buffer (compositor is done with it after Ready)
        if let Some(wl_buf) = self.state.wl_buffer.take() {
            wl_buf.destroy();
        }

        let fs = self.state.frame_state.lock().unwrap_or_else(|e| e.into_inner());
        let buf = self.state.buffer.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Screen capture completed but no buffer data received"))?;
        // SAFETY: buf.ptr and buf.size are from the same mmap allocation,
        // sized by ftruncate(size) + mmap(size). The compositor fills this
        // buffer before signaling Ready.
        let data = unsafe { std::slice::from_raw_parts(buf.ptr, buf.size) }.to_vec();

        Ok(CapturedFrame {
            width: fs.width,
            height: fs.height,
            stride: fs.stride,
            format: fs.format,
            data,
        })
    }
}
