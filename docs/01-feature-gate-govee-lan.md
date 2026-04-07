# Stage 1: Feature-gate wayland and audio in govee-lan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make wayland screen capture and PulseAudio audio analysis optional dependencies in govee-lan, so consumers like govee-gui that only need protocol + discovery don't pull in unnecessary system libraries.

**Architecture:** Add Cargo feature flags `screen` and `audio` (both on by default). Gate the `wayland` and `audio` modules behind `#[cfg(feature = ...)]`. Update govee-gui to opt out of defaults.

**Tech Stack:** Rust, Cargo features

---

## Context

The govee-lan library bundles protocol, discovery, wayland screen capture, and PulseAudio audio capture in one crate with no feature gates. The GUI only needs protocol + discovery (it spawns the CLI for continuous modes), but transitively links against libpulse and all wayland libraries. This increases compile time, binary size, and means govee-gui won't compile without libpulse-dev installed even though it never uses audio capture.

---

### Task 1: Add feature flags to govee-lan

**Files:**
- Modify: `govee-lan/Cargo.toml`
- Modify: `govee-lan/src/lib.rs`
- Modify: `govee-gui/Cargo.toml`

- [ ] **Step 1: Update govee-lan/Cargo.toml with feature flags**

Add a `[features]` section and mark the heavy dependencies as optional:

```toml
[features]
default = ["audio", "screen"]
audio = ["dep:libpulse-binding", "dep:rustfft", "dep:bytemuck"]
screen = ["dep:wayland-client", "dep:wayland-protocols", "dep:wayland-protocols-wlr", "dep:nix"]

[dependencies]
anyhow = "1.0.102"
base64 = "0.22"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
socket2 = { version = "0.5", features = ["all"] }
wayland-client = { version = "0.31", optional = true }
wayland-protocols = { version = "0.32", features = ["client"], optional = true }
wayland-protocols-wlr = { version = "0.3", features = ["client"], optional = true }
libpulse-binding = { version = "2", optional = true }
rustfft = { version = "6", optional = true }
bytemuck = { version = "1.25.0", optional = true }
nix = { version = "0.29", features = ["fs", "mman", "poll"], optional = true }
```

- [ ] **Step 2: Gate modules in govee-lan/src/lib.rs**

Replace the current lib.rs with:

```rust
pub mod protocol;
pub mod discovery;
pub mod colors;

#[cfg(feature = "screen")]
pub mod wayland;
#[cfg(feature = "audio")]
pub mod audio;

// Public API re-exports
pub use protocol::{
    send_turn, send_brightness, send_color, send_color_temp, send_command,
    razer_activate, razer_deactivate, send_segments,
    MULTICAST_GROUP, SCAN_PORT, RESPONSE_PORT, CONTROL_PORT,
};
pub use colors::{hex_to_rgb, color_distance, smooth, saturate_color, lerp_color_chain};
pub use discovery::{DeviceInfo, scan_devices, discover_device, resolve_ip};
```

- [ ] **Step 3: Update govee-gui/Cargo.toml to disable default features**

Change:
```toml
govee-lan = { path = "../govee-lan" }
```
to:
```toml
govee-lan = { path = "../govee-lan", default-features = false }
```

- [ ] **Step 4: Verify both crates compile**

Run: `cargo check -p govee-cli && cargo check -p govee-gui`
Expected: Both succeed.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --all-targets`
Expected: No new warnings from feature gating.

- [ ] **Step 6: Commit**

```bash
git add govee-lan/Cargo.toml govee-lan/src/lib.rs govee-gui/Cargo.toml
git commit -m "refactor(govee-lan): feature-gate wayland and audio modules

govee-gui only needs protocol + discovery, not screen capture or
audio analysis. Optional deps behind 'screen' and 'audio' features
reduce GUI compile time and remove unnecessary system lib deps."
```

---

## Verification

1. `cargo check -p govee-gui` compiles without libpulse/wayland headers in scope
2. `cargo check -p govee-cli` still compiles with all features
3. `cargo clippy --all-targets` — no warnings
