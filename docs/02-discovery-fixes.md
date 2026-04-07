# Stage 2: Discovery fixes — dedup and error surfacing

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix two issues in device discovery: duplicate devices appearing when a device responds multiple times, and silent failures when the discovery port is already in use.

**Architecture:** Two small, independent changes to `scan_devices` in `govee-lan/src/discovery.rs`.

**Tech Stack:** Rust

---

## Context

`scan_devices` has two problems: (1) a device that sends multiple UDP responses during the scan window appears multiple times in the result vec, causing duplicate entries in the GUI sidebar, and (2) if port 4002 is already bound (e.g. another govee instance), the function silently returns an empty vec — the user just sees "No devices found" with no hint about what went wrong.

---

### Task 1: Deduplicate scan results by IP

**Files:**
- Modify: `govee-lan/src/discovery.rs`

- [ ] **Step 1: Add dedup check before pushing**

In `scan_devices`, inside the `Ok((n, _))` match arm (around line 60), change the push condition from:

```rust
if !info.ip.is_empty() && info.ip.parse::<Ipv4Addr>().is_ok() {
    devices.push(info);
}
```

to:

```rust
if !info.ip.is_empty()
    && info.ip.parse::<Ipv4Addr>().is_ok()
    && !devices.iter().any(|d| d.ip == info.ip)
{
    devices.push(info);
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p govee-lan`
Expected: Success.

- [ ] **Step 3: Commit**

```bash
git add govee-lan/src/discovery.rs
git commit -m "fix(discovery): deduplicate devices by IP in scan results

Devices that respond multiple times during the scan window no longer
appear as duplicates in the result list."
```

---

### Task 2: Surface discovery bind errors

**Files:**
- Modify: `govee-lan/src/discovery.rs`

- [ ] **Step 1: Log bind failure instead of silently returning empty**

In `scan_devices`, change the socket creation error handling (around line 31-34) from:

```rust
    Ok(s) => s,
    Err(_) => return devices,
};
```

to:

```rust
    Ok(s) => s,
    Err(e) => {
        eprintln!("govee: failed to bind discovery port {RESPONSE_PORT}: {e}");
        return devices;
    }
};
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p govee-lan`
Expected: Success.

- [ ] **Step 3: Commit**

```bash
git add govee-lan/src/discovery.rs
git commit -m "fix(discovery): log bind failure instead of silent empty return

If port 4002 is already in use (e.g. another govee instance), the
user now sees a diagnostic message instead of just 'No devices found'."
```

---

## Verification

1. `cargo check -p govee-lan` — compiles
2. `cargo test` — existing tests pass
3. Manual: run `govee scan` twice simultaneously — second instance should print the bind error
