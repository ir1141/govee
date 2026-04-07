# Stage 3: GUI fixes — blocking I/O and clippy warning

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix blocking `scan_devices` calls on the tokio runtime and clean up the clippy derivable_impls warning.

**Architecture:** Two small changes in govee-gui: wrap blocking discovery in `spawn_blocking`, and derive Default instead of manually implementing it.

**Tech Stack:** Rust, tokio, iced

---

## Context

`scan_devices` does blocking UDP I/O with a 2-second timeout. The GUI calls it via `Task::perform` which runs on tokio, blocking a runtime worker thread for up to 2 seconds every 10 seconds (the discovery tick interval). This can cause GUI stutter. Separately, clippy flags `GuiConfig`'s manual `Default` impl as derivable.

---

### Task 1: Move scan_devices off tokio runtime

**Files:**
- Modify: `govee-gui/src/app.rs`

- [ ] **Step 1: Wrap scan_devices in spawn_blocking in App::new()**

In `App::new()` (around line 139), change:

```rust
let init_task = Task::perform(
    async { govee_lan::scan_devices(Duration::from_secs(2)) },
    Message::DevicesDiscovered,
);
```

to:

```rust
let init_task = Task::perform(
    async {
        tokio::task::spawn_blocking(|| govee_lan::scan_devices(Duration::from_secs(2)))
            .await
            .unwrap_or_default()
    },
    Message::DevicesDiscovered,
);
```

- [ ] **Step 2: Apply the same fix to DiscoveryTick handler**

In the `Message::DiscoveryTick` handler (around line 227), change:

```rust
Message::DiscoveryTick => {
    return Task::perform(
        async { govee_lan::scan_devices(Duration::from_secs(2)) },
        Message::DevicesDiscovered,
    );
}
```

to:

```rust
Message::DiscoveryTick => {
    return Task::perform(
        async {
            tokio::task::spawn_blocking(|| govee_lan::scan_devices(Duration::from_secs(2)))
                .await
                .unwrap_or_default()
        },
        Message::DevicesDiscovered,
    );
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p govee-gui`
Expected: Success.

- [ ] **Step 4: Commit**

```bash
git add govee-gui/src/app.rs
git commit -m "fix(gui): move blocking scan_devices off tokio runtime

scan_devices does blocking UDP I/O with a 2-second timeout. Running
it directly in Task::perform blocks a tokio worker thread. Use
spawn_blocking to move it to the blocking thread pool."
```

---

### Task 2: Fix clippy derivable_impls warning

**Files:**
- Modify: `govee-gui/src/config.rs`

- [ ] **Step 1: Replace manual Default impl with derive**

Change line 11-12 from:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
```

to:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuiConfig {
```

Then delete the `impl Default for GuiConfig` block (lines 134-144):

```rust
// DELETE THIS BLOCK:
impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            controls: ControlsConfig::default(),
            screen: ScreenConfig::default(),
            audio: AudioConfig::default(),
            ambient: AmbientConfig::default(),
        }
    }
}
```

- [ ] **Step 2: Verify clippy is clean**

Run: `cargo clippy --all-targets`
Expected: No warnings.

- [ ] **Step 3: Commit**

```bash
git add govee-gui/src/config.rs
git commit -m "fix(gui): derive Default for GuiConfig per clippy suggestion"
```

---

## Verification

1. `cargo check -p govee-gui` — compiles
2. `cargo clippy --all-targets` — no warnings
3. `cargo test` — all tests pass
