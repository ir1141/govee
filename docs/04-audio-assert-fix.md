# Stage 4: Replace assert with error return in audio capture

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace a `assert!(spec.is_valid())` in the audio capture background thread with a proper error return, preventing a silent thread death that leaves the analyzer returning stale state forever.

**Architecture:** Single-line change in `govee-lan/src/audio.rs`.

**Tech Stack:** Rust, anyhow

---

## Context

`capture_loop` in `govee-lan/src/audio.rs` uses `assert!(spec.is_valid())` to validate the PulseAudio sample spec. This function returns `Result`, but the assert bypasses error handling and panics directly. If it panics, the background thread dies silently — `AudioAnalyzer::new()` already returned `Ok` (the thread was spawned and the 200ms sleep ran), so the analyzer appears to work but returns `AudioState::default()` forever. The LEDs just stay dark with no error message.

---

### Task 1: Replace assert with bail

**Files:**
- Modify: `govee-lan/src/audio.rs`

- [ ] **Step 1: Replace the assert**

In `capture_loop` (around line 260), change:

```rust
assert!(spec.is_valid());
```

to:

```rust
if !spec.is_valid() {
    anyhow::bail!("Invalid PulseAudio sample spec (44100Hz mono f32)");
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p govee-lan`
Expected: Success.

- [ ] **Step 3: Commit**

```bash
git add govee-lan/src/audio.rs
git commit -m "fix(audio): replace assert with error return for invalid sample spec

A panic in the background capture thread would silently kill it,
leaving the analyzer returning stale default state forever. Now
returns a proper error that surfaces as 'Audio capture error: ...'."
```

---

## Verification

1. `cargo check -p govee-lan` — compiles
2. `cargo test` — all tests pass
3. `cargo clippy --all-targets --all-features` — no warnings
